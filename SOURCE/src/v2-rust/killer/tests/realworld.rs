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

#[test]
fn and_or_short_circuit() {
    // the right operand used to run even when the left decided the result
    let src = "fn boom() {\n  println(\"boom\")\n  return true\n}\nprintln(false && boom())\nprintln(true || boom())\nprintln(true && boom())\nprintln(false || boom())\nxs = [1]\nj = 3\nprintln(j < len(xs) && xs[j] > 0)\nprintln(j >= len(xs) || xs[j] > 0)\nif j < len(xs) && xs[j] > 0 {\n  println(\"no\")\n}\nwhile j < len(xs) && xs[j] > 0 {\n  j = j + 1\n}\nprintln(j)\n";
    assert_eq!(run(src), "false\ntrue\nboom\ntrue\nboom\ntrue\nfalse\ntrue\n3");
}

#[test]
fn null_coalescing_only_evaluates_the_fallback_when_needed() {
    let src = "fn fallback() {\n  println(\"fallback\")\n  return 0\n}\nx = 5\nprintln(x ?? fallback())\ny = null\nprintln(y ?? fallback())\nd = {\"a\": 1}\nprintln((d[\"a\"] ?? 0) + 1)\nprintln((d[\"zz\"] ?? 10) + 1)\n";
    assert_eq!(run(src), "5\nfallback\n0\n2\n11");
}

#[test]
fn short_circuit_keeps_three_valued_logic() {
    let src = "u = T_ZERO\nprintln(u && true)\nprintln(u || false)\nprintln(T_POS && T_NEG)\nprintln(T_NEG || T_ZERO)\n";
    assert_eq!(run(src), "T_ZERO\nT_ZERO\nT_NEG\nT_ZERO");
}

#[test]
fn short_circuit_inside_natively_compiled_functions() {
    // hot enough for the JIT; the guard must still protect the array read
    let src = "fn ok(xs, i) {\n  if i < len(xs) && xs[i] > 0 {\n    return 1\n  }\n  return 0\n}\nfn main() {\n  s = 0\n  for k in range(600) {\n    s = s + ok([1, 2, 3], k % 6)\n  }\n  for k in range(500) {\n    if k > 3 || k < 2 {\n      s = s + 1\n    }\n  }\n  return s\n}\nprintln(main())\n";
    assert_eq!(run(src), "798");
}

#[test]
fn constant_folding_does_not_cross_a_branch_target() {
    // `0` (end of `z ?? 0`) and `1` were folded together although a jump lands between them
    let src = "z = 3\nprintln((z ?? 0) + 1)\nc = true\nprintln((c ? 5 : 6) + 7)\nc = false\nprintln((c ? 5 : 6) + 7)\nprintln((null ?? 2) * 4)\n";
    assert_eq!(run(src), "4\n12\n13\n8");
}

#[test]
fn loop_and_comprehension_variables_can_be_called() {
    let src = "fn inc(x) {\n  return x + 1\n}\nfn dbl(x) {\n  return x * 2\n}\nfn run(fs, v) {\n  for g in fs {\n    v = g(v)\n  }\n  return v\n}\nprintln(run([inc, dbl, inc], 3))\nfs = [inc, dbl]\nprintln([f(10) for f in fs])\nclass K {\n  fn init() {\n    this.fs = [inc, dbl]\n  }\n  fn go(v) {\n    for g in reverse(this.fs) {\n      v = g(v)\n    }\n    return v\n  }\n}\nprintln(new K().go(5))\n";
    assert_eq!(run(src), "9\n[11, 20]\n11");
}

#[test]
fn string_literal_ending_in_an_escaped_quote() {
    let src = "println(\"say \\\"hi\\\"\")\nprintln(\"quote:\\\"\")\nprintln(\"a\" + \"q:\\\"\" + \"b\")\nprintln(\"x,\\\"y\", \"z\")\n";
    assert_eq!(run(src), "say \"hi\"\nquote:\"\naq:\"b\nx,\"y z");
}

