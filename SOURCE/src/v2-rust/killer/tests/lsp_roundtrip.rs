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
        let mut child = Command::new(env!("CARGO_BIN_EXE_killer_lsp"))
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
