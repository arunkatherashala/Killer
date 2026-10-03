//! Real-world programs (parity/realworld/*.killer) run end to end and compared with their reviewed
//! expected output, plus a regression test for every bug those programs exposed.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("killer_rw_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_file(path: &std::path::Path, cwd: &std::path::Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(path)
        .arg("--run")
        .current_dir(cwd)
        .output()
        .unwrap();
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

fn run(src: &str) -> String {
    let dir = scratch_dir();
    let path = dir.join("main.killer");
    std::fs::write(&path, src).unwrap();
    let out = run_file(&path, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// Run parity/realworld/<name>.killer in a scratch directory and compare with <name>.expected.
fn program(name: &str) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parity").join("realworld");
    let dir = scratch_dir();
    let actual = run_file(&base.join(format!("{name}.killer")), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let expected = std::fs::read_to_string(base.join(format!("{name}.expected")))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(actual, expected.trim(), "output of {name} changed");
}

#[test]
fn assigning_through_a_field_and_an_index() {
    // `this.cells[i] = x` was rejected as an invalid assignment target
    let src = "class B {\n  fn init() {\n    this.cells = [0, 0, 0]\n    this.pos = {\"x\": 1}\n  }\n  fn set(i, v) {\n    this.cells[i] = v\n    this.pos.x = v\n  }\n}\nb = new B()\nb.set(1, 7)\nprintln(b.cells)\nprintln(b.pos)\nb.cells[2] = 9\nb.pos.y = 5\nprintln(b.cells)\nprintln(b.pos)\nm = {\"rows\": [[1, 2], [3, 4]]}\nm.rows[1][0] = 30\nprintln(m)\n";
    assert_eq!(run(src), "[0, 7, 0]\n{x: 7}\n[0, 7, 9]\n{x: 7, y: 5}\n{rows: [[1, 2], [30, 4]]}");
}

