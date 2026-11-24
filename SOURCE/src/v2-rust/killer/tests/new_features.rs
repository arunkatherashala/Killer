//! Tests that need more than one process or a piped stdin: the step debugger and the browser-window helper
//! `gui_show` (run without opening a browser).
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn script(src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("killer_nf_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    file
}

fn lf(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n")
}

const PROGRAM: &str = "fn add(a, b) {\n  c = a + b\n  return c\n}\nx = 1\ny = add(x, 2)\nprintln(y)\nprintln(\"done\")\n";

fn debug(input: &str, extra: &[&str]) -> (bool, String, String) {
    let file = script(PROGRAM);
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer"))
        .arg("debug")
        .arg(&file)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    (out.status.success(), lf(&out.stdout), lf(&out.stderr))
}

#[test]
fn debugger_steps_into_a_call_and_shows_the_call_stack_and_locals() {
    let (ok, out, err) = debug("s\ns\ns\nbt\nlocals\nc\n", &[]);
    assert!(ok, "{err}");
    assert!(out.contains("c = a + b"), "stopped inside add: {out}");
    assert!(out.contains("called from") && out.contains(":6"), "call stack: {out}");
    assert!(out.contains("arg0 = 1") && out.contains("arg1 = 2"), "locals: {out}");
    assert!(out.contains("3\n") && out.contains("done"), "program output: {out}");
}

#[test]
fn debugger_stops_at_a_breakpoint_and_can_quit() {
    let (ok, out, err) = debug("c\nq\n", &["--break", "3"]);
    assert!(!ok);
    assert!(out.contains("[breakpoint]") && out.contains(":3:"), "{out}");
    assert!(err.contains("stopped by the user"), "{err}");
    assert!(!out.contains("done"), "the program must not continue after quit: {out}");
}

#[test]
fn debugger_next_steps_over_a_call() {
    let (ok, out, err) = debug("n\nn\nn\nc\n", &[]);
    assert!(ok, "{err}");
    assert!(!out.contains("c = a + b"), "next must not enter add: {out}");
    assert!(out.contains("done"));
}

#[test]
fn gui_show_serves_a_page_and_handles_events_then_quits() {
    let src = r#"fn client(port) {
  base = "http://127.0.0.1:" + str(port)
  a = http_get(base + "/", {}, 10)
  b = http_request({"method": "POST", "url": base + "/event/add", "body": 5, "timeout": 10})
  c = http_request({"method": "POST", "url": base + "/event/none", "body": "", "timeout": 10})
  d = http_request({"method": "POST", "url": base + "/quit", "body": "", "timeout": 10})
  return [contains(a, "<h1>hi</h1>") and contains(a, "function send"), b["body"], c["status"], d["status"]]
}
port = free_port()
h = async_spawn(client, port)
n = gui_show("Demo", "<h1>hi</h1>", {"add": fn(d) { return d * 2 }}, {"open": false, "port": port, "max_seconds": 30})
r = async_await(h)
println(r[0], r[1], r[2], r[3], n)
"#;
    let file = script(src);
    // the page is served on 127.0.0.1: the SSRF guard blocks loopback unless it is allowed explicitly
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).env("KILLER_ALLOW_PRIVATE_NET", "1").output().unwrap();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    assert!(out.status.success(), "{}", lf(&out.stderr));
    assert_eq!(lf(&out.stdout).trim(), "true {\"result\":10} 404 200 4");
}

#[test]
fn gui_open_refuses_bad_targets() {
    let file = script("gui_open(\"ftp://example.com/x\")\n");
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    assert!(!out.status.success());
    assert!(lf(&out.stderr).contains("not an http(s) address"), "{}", lf(&out.stderr));
}
