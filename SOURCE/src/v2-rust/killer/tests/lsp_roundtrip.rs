//! Round trip against the real `killer_lsp` binary over stdio.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Client {
    fn start() -> Client {
        Client::spawn(Command::new(env!("CARGO_BIN_EXE_killer_lsp")))
    }

    /// The same server through the unified driver: `killer lsp`.
    fn start_driver() -> Client {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer"));
        cmd.arg("lsp");
        Client::spawn(cmd)
    }

    fn spawn(mut cmd: Command) -> Client {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn killer_lsp");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Client { child, stdin, out }
    }

    fn send(&mut self, body: &str) {
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read(&mut self) -> String {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            assert!(self.out.read_line(&mut line).unwrap() > 0, "server closed stdout");
            let t = line.trim();
            if t.is_empty() {
                break;
            }
            if let Some(v) = t.strip_prefix("Content-Length:") {
                len = v.trim().parse().unwrap();
            }
        }
        let mut buf = vec![0u8; len];
        self.out.read_exact(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

const URI: &str = "file:///tmp/broken.killer";

#[test]
fn initialize_diagnostics_completion_definition_shutdown() {
    let mut c = Client::start();

    c.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#);
    let init = c.read();
    assert!(init.contains("\"completionProvider\""), "{init}");
    assert!(init.contains("\"definitionProvider\":true"), "{init}");
    c.send(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);

    // line 3 (index 2) is broken
    let broken = "x = 1\\nfn add(a, b) {\\n  return a +\\n}\\ny = add(1, 2)\\n";
    c.send(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{URI}","languageId":"killer","version":1,"text":"{broken}"}}}}}}"#
    ));
    let diag = c.read();
    assert!(diag.contains("textDocument/publishDiagnostics"), "{diag}");
    assert!(diag.contains("\"line\":2"), "diagnostic should be on line index 2: {diag}");
    assert!(diag.contains("unsupported expression"), "{diag}");

    // fix the file with a full-text change: diagnostics clear
    let fixed = "x = 1\\nfn add(a, b) {\\n  return a + b\\n}\\ny = add(1, 2)\\n";
    c.send(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{URI}","version":2}},"contentChanges":[{{"text":"{fixed}"}}]}}}}"#
    ));
    let diag = c.read();
    assert!(diag.contains("\"diagnostics\":[]"), "{diag}");

    c.send(&format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{URI}"}},"position":{{"line":4,"character":0}}}}}}"#
    ));
    let comp = c.read();
    for label in ["\"add\"", "\"while\"", "\"print\"", "\"json_parse\""] {
        assert!(comp.contains(label), "completion lacks {label}");
    }

    // go to definition of `add` on line 4: `y = add(1, 2)`
    c.send(&format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{URI}"}},"position":{{"line":4,"character":5}}}}}}"#
    ));
    let def = c.read();
    assert!(def.contains("\"line\":1"), "definition should be line index 1: {def}");

    c.send(&format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{URI}"}},"position":{{"line":4,"character":5}}}}}}"#
    ));
    assert!(c.read().contains("fn add(a, b)"));

    c.send(&format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"textDocument/documentSymbol","params":{{"textDocument":{{"uri":"{URI}"}}}}}}"#
    ));
    let syms = c.read();
    assert!(syms.contains("\"add\"") && syms.contains("\"y\""), "{syms}");

    c.send(r#"{"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}"#);
    assert!(c.read().contains("\"result\":null"));
    c.send(r#"{"jsonrpc":"2.0","method":"exit"}"#);
    let status = c.child.wait().unwrap();
    assert!(status.success(), "exit code after clean shutdown must be 0");
}

fn req(id: i64, method: &str, uri: &str, line: usize, ch: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{ch}}}}}}}"#
    )
}

#[test]
fn killer_lsp_subcommand_hover_builtins_and_definitions_of_classes_and_variables() {
    let mut c = Client::start_driver();
    c.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#);
    let init = c.read();
    assert!(init.contains("\"hoverProvider\""), "{init}");
    c.send(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);

    let uri = "file:///tmp/defs.killer";
    // 0: class Animal {  1: fn speak() {  2: return 1  3: }  4: }  5: counter = 10
    // 6: pet = new Animal()  7: print(counter)  8: n = len("abc")
    let text = "class Animal {\\n  fn speak() {\\n    return 1\\n  }\\n}\\ncounter = 10\\npet = new Animal()\\nprint(counter)\\nn = len([1, 2])\\n";
    c.send(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"killer","version":1,"text":"{text}"}}}}}}"#
    ));
    let diag = c.read();
    assert!(diag.contains("\"diagnostics\":[]"), "valid file must have no diagnostics: {diag}");

    // class: `Animal` in `pet = new Animal()` -> line 0
    c.send(&req(2, "textDocument/definition", uri, 6, 12));
    let d = c.read();
    assert!(d.contains("\"line\":0"), "{d}");
    // variable: `counter` in print(counter) -> line 5
    c.send(&req(3, "textDocument/definition", uri, 7, 8));
    let d = c.read();
    assert!(d.contains("\"line\":5"), "{d}");
    // hover on a builtin gives its documentation / signature
    c.send(&req(4, "textDocument/hover", uri, 8, 5));
    let h = c.read();
    assert!(h.contains("len"), "{h}");
    assert!(h.contains("\"value\""), "{h}");
    // hover on a class
    c.send(&req(5, "textDocument/hover", uri, 6, 12));
    assert!(c.read().contains("class Animal"));
    // completion offers file symbols, keywords and builtins
    c.send(&req(6, "textDocument/completion", uri, 8, 0));
    let comp = c.read();
    for l in ["\"Animal\"", "\"counter\"", "\"pet\"", "\"class\"", "\"len\""] {
        assert!(comp.contains(l), "completion lacks {l}: {comp}");
    }
    c.send(r#"{"jsonrpc":"2.0","id":7,"method":"shutdown","params":null}"#);
    assert!(c.read().contains("\"result\":null"));
    c.send(r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert!(c.child.wait().unwrap().success());
}
