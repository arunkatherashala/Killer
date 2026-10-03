//! End-to-end checks that compile and runtime errors name the user's original file and line,
//! however much the source was rewritten (lambdas lifted, match/switch/loops lowered, imports
//! inlined) before it was compiled.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new() -> Case {
        let dir = std::env::temp_dir().join(format!(
            "killer_error_lines_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Case { dir }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    /// Run `name` through the binary; returns (success, stdout, stderr).
    fn run(&self, name: &str) -> (bool, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
            .arg(self.dir.join(name))
            .arg("--run")
            .current_dir(&self.dir)
            .output()
            .expect("run killer_super");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        )
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Run a script that must fail; returns the error text (stderr).
fn fail(script: &str) -> String {
    let c = Case::new();
    c.write("main.killer", script);
    let (ok, _out, err) = c.run("main.killer");
    assert!(!ok, "script should have failed:\n{script}");
    err
}

/// The failing line must be reported as `main.killer:LINE` (not just any line).
fn assert_runtime_line(err: &str, line: usize) {
    let want = format!("main.killer:{line}");
    let at = err.find("main.killer:").map(|i| &err[i..]);
    assert!(
        err.contains(&want) && at.map_or(false, |s| !s[want.len()..].starts_with(|c: char| c.is_ascii_digit())),
        "expected {want} in:\n{err}"
    );
}

#[test]
fn plain_script_runtime_error() {
    let err = fail("x = 1\nprintln(x)\n\n// a comment\ny = undefined_name + 1\n");
    assert!(err.contains("Undefined variable"), "{err}");
    assert_runtime_line(&err, 5);
}

#[test]
fn division_by_zero_inside_a_function() {
    let err = fail("fn f(a) {\n  b = 2\n  return a / 0\n}\nprintln(\"hi\")\nr = f(3)\n");
    assert!(err.contains("Division by zero"), "{err}");
    assert_runtime_line(&err, 3);
}

#[test]
fn uncaught_throw_reports_its_line() {
    let err = fail("println(\"start\")\n\nfn check(v) {\n  if v > 1 {\n    throw \"too big\"\n  }\n}\ncheck(5)\n");
    assert!(err.contains("too big"), "{err}");
    assert_runtime_line(&err, 5);
}

#[test]
fn lambdas_do_not_shift_later_lines() {
    let script = "\
double = (a) => a * 2
inc = fn(n) {
  return n + 1
}
println(double(inc(1)))
nums = map([1, 2, 3], x => x * 10)
println(nums)
bad = nums[0] / 0
";
    let err = fail(script);
    assert!(err.contains("Division by zero"), "{err}");
    assert_runtime_line(&err, 8);
}

#[test]
fn error_inside_a_lambda_body_points_at_the_lambda() {
    let script = "\
x = 1
div = (a) => 10 / a
println(\"before\")
println(div(0))
";
    let err = fail(script);
    assert!(err.contains("Division by zero"), "{err}");
    assert_runtime_line(&err, 2);
}

#[test]
fn error_in_function_called_from_a_lambda() {
    let script = "\
fn risky(n) {
  return 100 / n
}

println(\"go\")
run = (v) => risky(v)
println(run(0))
";
    let err = fail(script);
    assert!(err.contains("Division by zero"), "{err}");
    assert_runtime_line(&err, 2);
}

#[test]
fn error_in_function_called_through_a_callback_builtin() {
    let script = "\
fn risky(n) {
  return 100 / n
}
vals = map([1, 0], v => risky(v))
println(vals)
";
    let err = fail(script);
    assert!(err.contains("Division by zero"), "{err}");
    assert_runtime_line(&err, 2);
}

#[test]
fn match_statement() {
    let script = "\
v = 2
match v {
  1 => println(\"one\")
  2 => println(10 / 0)
  _ => println(\"other\")
}
println(\"after\")
";
    let err = fail(script);
    assert_runtime_line(&err, 4);
    // and an error after the match
    let err = fail("v = 2\nmatch v {\n  1 => println(\"one\")\n  _ => println(\"other\")\n}\nz = undefined_after\n");
    assert_runtime_line(&err, 6);
}

#[test]
fn switch_statement() {
    let script = "\
k = 3
switch k {
  case 1:
    println(\"one\")
  case 3:
    println(\"three\")
    println(1 / 0)
  default:
    println(\"d\")
}
println(\"done\")
";
    let err = fail(script);
    assert_runtime_line(&err, 7);
    let err = fail("k = 3\nswitch k {\n  case 3:\n    println(\"three\")\n  default:\n    println(\"d\")\n}\nq = no_such_var\n");
    assert_runtime_line(&err, 8);
}

#[test]
fn do_while_loop() {
    let script = "\
n = 0
do {
  n = n + 1
  println(n)
  r = 5 / (n - 2)
} while n < 4
";
    let err = fail(script);
    assert_runtime_line(&err, 5);
    let err = fail("n = 0\ndo {\n  n = n + 1\n} while n < 3\nz = missing_name\n");
    assert_runtime_line(&err, 5);
}

#[test]
fn c_style_for_loop() {
    let script = "\
total = 0
for (i = 0; i < 5; i++) {
  total = total + i
  w = 10 / (3 - i)
}
";
    let err = fail(script);
    assert_runtime_line(&err, 4);
    let err = fail("for (i = 0; i < 2; i++) {\n  println(i)\n}\nz = missing_name\n");
    assert_runtime_line(&err, 4);
}

#[test]
fn destructuring_for_loop() {
    let script = "\
pairs = [[1, 2], [3, 0]]
for a, b in pairs {
  println(a)
  q = a / b
}
";
    let err = fail(script);
    assert_runtime_line(&err, 4);
    let err = fail("pairs = [[1, 2]]\nfor a, b in pairs {\n  println(a + b)\n}\nz = missing_name\n");
    assert_runtime_line(&err, 5);
}

#[test]
fn error_after_several_rewritten_constructs() {
    let script = "\
sq = (x) => x * x
v = 2
match v {
  1 => println(\"one\")
  2 => println(sq(v))
  _ => println(\"other\")
}
for (i = 0; i < 2; i++) {
  println(i)
}
n = 0
do {
  n = n + 1
} while n < 3
for a, b in [[1, 2]] {
  println(a + b)
}
s = \"\"\"multi
line
string\"\"\"
a, b = 1, 2
println(s)
final = missing_after_all
";
    let err = fail(script);
    assert_runtime_line(&err, 23);
}

#[test]
fn imported_file_errors_name_their_own_file_and_line() {
    let c = Case::new();
    c.write("lib.killer", "// helper\n\nfn helper(n) {\n  x = 1\n  return 10 / (n - 2)\n}\n");
    c.write(
        "main.killer",
        "import \"lib.killer\"\nprintln(\"loaded\")\nsq = (x) => x * x\nprintln(helper(sq(1) + 1))\n",
    );
    let (ok, _out, err) = c.run("main.killer");
    assert!(!ok);
    assert!(err.contains("Division by zero"), "{err}");
    assert!(err.contains("lib.killer:5"), "{err}");
    assert!(!err.contains("main.killer"), "{err}");
}

#[test]
fn error_in_main_after_an_import_uses_the_main_file_line() {
    let c = Case::new();
    c.write("lib.killer", "fn a() {\n  return 1\n}\nfn b() {\n  return 2\n}\n");
    c.write("main.killer", "import \"lib.killer\"\nprintln(a() + b())\nz = missing_name\n");
    let (ok, _out, err) = c.run("main.killer");
    assert!(!ok);
    assert!(err.contains("main.killer:3"), "{err}");
}

#[test]
fn compile_error_in_an_imported_file() {
    let c = Case::new();
    c.write("lib.killer", "x = 1\nfn broken( {\n  return 1\n}\n");
    c.write("main.killer", "import \"lib.killer\"\nprintln(1)\n");
    let (ok, _out, err) = c.run("main.killer");
    assert!(!ok);
    assert!(err.contains("Line 2 of lib.killer"), "{err}");
}

#[test]
fn compile_error_line_after_rewritten_constructs() {
    let script = "\
f = (a) => a + 1
match 1 {
  1 => println(f(1))
  _ => println(0)
}
for (i = 0; i < 1; i++) {
  println(i)
}
break
";
    let err = fail(script);
    assert!(err.contains("Line 9:"), "{err}");
}

#[test]
fn type_errors_use_original_lines() {
    let c = Case::new();
    c.write("lib.killer", "fn ok() {\n  return 1\n}\n");
    c.write(
        "main.killer",
        "import \"lib.killer\"\nfn add(a: number, b: number) -> number {\n  return a + b\n}\nprintln(add(1, \"x\"))\n",
    );
    let (ok, _out, err) = c.run("main.killer");
    assert!(!ok);
    assert!(err.contains("line 5"), "{err}");
}

#[test]
fn double_slash_is_explained_instead_of_a_confusing_signature_error() {
    let err = fail("println(1)\nx = (7 // 2) + 1\n");
    assert!(err.contains("Line 2:"), "{err}");
    assert!(err.contains("// starts a comment in Killer; use floor(a / b)"), "{err}");
    assert!(!err.contains("unclosed"), "{err}");
}

#[test]
fn else_if_condition_has_its_own_line() {
    let script = "\
v = 5
if v == 1 {
  println(\"one\")
} else if 10 / (v - 5) > 1 {
  println(\"big\")
} else {
  println(\"small\")
}
";
    let err = fail(script);
    assert_runtime_line(&err, 4);
}

#[test]
fn compile_errors_are_remapped_in_process() {
    let err = killer_native::compile_killer_subset("f = (a) => a\nmatch 1 {\n  1 => f(1)\n  _ => f(2)\n}\ncontinue\n").unwrap_err();
    assert!(err.to_string().contains("Line 6:"), "{err}");
}

#[test]
fn program_line_table_maps_instructions_to_original_lines() {
    let program = killer_native::compile_killer_subset("f = (a) => a + 1\n\nx = f(1)\ny = x * 2\n").unwrap();
    let table = &program.line_table;
    assert!(!table.entries.is_empty());
    let lines: Vec<u32> = (0..program.instructions.len()).filter_map(|ip| table.lookup(ip).map(|(_, l)| l)).collect();
    // the lifted lambda comes from line 1, then lines 3 and 4 of the script
    assert!(lines.contains(&1) && lines.contains(&3) && lines.contains(&4), "{lines:?}");
}

#[test]
fn caught_errors_keep_their_plain_message() {
    let c = Case::new();
    c.write(
        "main.killer",
        "try {\n  x = 1 / 0\n} catch e {\n  println(\"caught: \" + e)\n}\nprintln(\"ok\")\n",
    );
    let (ok, out, err) = c.run("main.killer");
    assert!(ok, "{err}");
    assert!(out.contains("caught: Division by zero"), "{out}");
    assert!(!out.contains("main.killer"), "{out}");
}
