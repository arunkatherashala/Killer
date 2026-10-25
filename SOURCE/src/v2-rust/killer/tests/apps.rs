//! Real-application trial: the five example apps under examples/apps/ run through the `killer`
//! driver (`killer test`, `killer run`), plus regression tests for every bug the trial found
//! (see examples/apps/FRICTION.md).
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Longest any single spawned process may run before the harness kills it (a hung server or client
/// must fail one test in minutes, not hang the whole CI job for hours).
const PROCESS_LIMIT_SECS: u64 = 240;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn norm(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n")
}

/// Wait for `child` (whose stdout/stderr are piped) for at most `secs` seconds, reading both pipes on
/// threads so a chatty child cannot block on a full pipe. On timeout the child is killed and a marker
/// line is appended to stderr so the failing assertion shows what happened.
fn finish(mut child: std::process::Child, secs: u64) -> Output {
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let read_all = |p: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut v = Vec::new();
            if let Some(mut p) = p {
                let _ = p.read_to_end(&mut v);
            }
            v
        })
    };
    let t_out = read_all(out_pipe.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let t_err = read_all(err_pipe.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().unwrap() {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().unwrap();
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    };
    let stdout = t_out.join().unwrap();
    let mut stderr = t_err.join().unwrap();
    if timed_out {
        stderr.extend_from_slice(format!("\n[test harness: process killed after {secs} s]\n").as_bytes());
    }
    Output { status, stdout, stderr }
}

/// Run a command to completion with captured output, no stdin and a time limit.
fn run_limited(mut cmd: Command, secs: u64) -> Output {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    finish(cmd.spawn().unwrap(), secs)
}

fn killer(cwd: &Path, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_killer"));
    c.current_dir(cwd).args(args);
    run_limited(c, PROCESS_LIMIT_SECS)
}

fn killer_pkg(cwd: &Path, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_killer_pkg"));
    c.current_dir(cwd).args(args);
    run_limited(c, PROCESS_LIMIT_SECS)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("killer_apps_{}_{}_{}", tag, std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let f = self.0.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, text).unwrap();
        f
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app_dir(name: &str) -> PathBuf {
    crate_dir().join("examples/apps").join(name)
}

/// Run `killer test` inside an app and return (success, stdout).
fn app_tests(name: &str) -> (bool, String) {
    let out = killer(&app_dir(name), &["test"]);
    (out.status.success(), norm(&out.stdout) + &norm(&out.stderr))
}

// ---------------------------------------------------------------------------------------------
// regression tests for bugs found by the trial
// ---------------------------------------------------------------------------------------------

#[test]
fn fstring_escapes_and_literal_braces() {
    let t = Tmp::new("fstr");
    let f = t.write(
        "a.killer",
        r#"x = 5
println(f"a\nb {x}")
println(f"t\tq \"{x}\" \\")
println(f"{{lit}} {x}")
println(k"k\n{x}")
"#,
    );
    let out = killer(&t.0, &["run", f.to_str().unwrap()]);
    assert_eq!(norm(&out.stdout), "a\nb 5\nt\tq \"5\" \\\n{lit} 5\nk\n5\n", "{}", norm(&out.stderr));
}

#[test]
fn killer_run_resolves_imports_relative_to_the_script() {
    let t = Tmp::new("imp");
    t.write("src/helper.killer", "fn helper_twice(x) {\n  return x * 2\n}\n");
    t.write("src/main.killer", "import \"helper.killer\"\nprintln(helper_twice(21))\n");
    let elsewhere = Tmp::new("cwd");
    let out = killer(&elsewhere.0, &["run", t.0.join("src/main.killer").to_str().unwrap()]);
    assert_eq!(norm(&out.stdout).trim(), "42", "{}", norm(&out.stderr));
}

#[test]
fn killer_test_resolves_imports_relative_to_the_test_file() {
    let t = Tmp::new("timp");
    t.write("src/lib.killer", "fn lib_inc(x) {\n  return x + 1\n}\n");
    t.write("tests/test_lib.killer", "import \"../src/lib.killer\"\ntest \"inc\" {\n  assert_eq(lib_inc(1), 2)\n}\n");
    let out = killer(&t.0, &["test"]);
    assert!(out.status.success(), "{}{}", norm(&out.stdout), norm(&out.stderr));
    assert!(norm(&out.stdout).contains("ok"), "{}", norm(&out.stdout));
}

#[test]
fn scaffolded_project_tests_are_discovered_by_killer_test() {
    let t = Tmp::new("scaf");
    assert!(killer_pkg(&t.0, &["new", "demo"]).status.success());
    let out = killer(&t.0.join("demo"), &["test"]);
    let text = norm(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("arithmetic works"), "{text}");
}

#[test]
fn killer_test_finds_test_blocks_in_any_file_of_a_tests_folder() {
    let t = Tmp::new("tdir");
    t.write("tests/checks.killer", "test \"one\" {\n  assert_eq(1, 1)\n}\n");
    t.write("tests/helper.killer", "fn helper() {\n  return 1\n}\n");
    let out = killer(&t.0, &["test"]);
    let text = norm(&out.stdout);
    assert!(text.contains("checks.killer::one"), "{text}");
    assert!(!text.contains("helper.killer"), "{text}");
}

#[test]
fn eprintln_writes_to_stderr_only() {
    let t = Tmp::new("eprint");
    let f = t.write("e.killer", "println(\"out\")\neprintln(\"bad\", 1, [2])\neprint(\"x\")\n");
    let out = killer(&t.0, &["run", f.to_str().unwrap()]);
    assert_eq!(norm(&out.stdout).trim(), "out");
    assert_eq!(norm(&out.stderr).trim(), "bad 1 [2]\nx");
}

fn run_src(src: &str) -> String {
    let t = Tmp::new("src");
    let f = t.write("p.killer", src);
    let out = killer(&t.0, &["run", f.to_str().unwrap()]);
    format!("{}{}", norm(&out.stdout), norm(&out.stderr)).trim().to_string()
}

#[test]
fn bigints_work_as_indexes_counts_and_builtin_arguments() {
    let src = r#"xs = [10, 20, 30]
i = 1n
println(xs[i])
xs[i] = 21
println(xs)
println(xs[0n:2n])
println("ab" * 2n)
println(range(3n))
println(int(5n))
println(chr(65n))
println(max(1n, 2n ** 70))
println(min([5n, 2, 9]))
println(sum([1n, 2n ** 70]))
println(floor(7n))
println(float(2n ** 70))
fn f() {
  ys = [1, 2]
  ys[1n] = 7
  return ys
}
println(f())
"#;
    assert_eq!(
        run_src(src),
        "20\n[10, 21, 30]\n[10, 21]\nabab\n[0, 1, 2]\n5\nA\n1180591620717411303424\n2\n1180591620717411303425\n7\n1180591620717411300000\n[1, 7]"
    );
}

#[test]
fn large_floats_print_exactly_and_exponent_literals_parse() {
    let src = "println(1e21)\nprintln(-1e25)\nprintln(2.0 ** 70)\nprintln(1e-7)\nprintln(2.5E+3 + 1)\nprintln(3e-2 * 2)\ne = 5\nprintln(e-1)\nprintln(123456789012345678.0)\n";
    assert_eq!(
        run_src(src),
        // 2.0 ** 70 is a whole-number power, so it is computed exactly (a plain `2 ** 70` prints the same)
        "1000000000000000000000\n-10000000000000000000000000\n1180591620717411303424\n0.0000001\n2501\n0.06\n4\n123456789012345680"
    );
}

#[test]
fn deeply_nested_containers_are_freed_without_overflowing_the_stack() {
    let src = r#"x = {"v": 0}
i = 0
while i < 100000 {
  x = {"l": x, "i": i}
  i = i + 1
}
x = null
ys = []
j = 0
while j < 100000 {
  ys = [ys]
  j = j + 1
}
println("same", ys == ys)
ys = null
class N {
  fn init(next) {
    this.next = next
  }
}
n = null
k = 0
while k < 50000 {
  n = new N(n)
  k = k + 1
}
n = null
println("freed")
"#;
    assert_eq!(run_src(src), "same true\nfreed");
}

// ---------------------------------------------------------------------------------------------
// app 5: kx, an expression language with exact integers
// ---------------------------------------------------------------------------------------------

fn run_app_main(app: &str, args: &[&str]) -> Output {
    let dir = app_dir(app);
    let mut a = vec!["run", "src/main.killer"];
    a.extend_from_slice(args);
    killer(&dir, &a)
}

fn python() -> Option<String> {
    for name in ["python", "python3"] {
        if let Ok(o) = Command::new(name).arg("--version").output() {
            if o.status.success() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[test]
fn kx_unit_tests_pass() {
    let (ok, text) = app_tests("kx");
    assert!(ok, "{text}");
    assert!(text.contains("0 failed"), "{text}");
}

#[test]
fn kx_numbers_match_the_python_mirror() {
    let out = run_app_main("kx", &["programs/numbers.kx"]);
    let got = norm(&out.stdout);
    let expected = norm(&std::fs::read(app_dir("kx").join("programs/numbers.expected")).unwrap());
    assert_eq!(got, expected, "{}", norm(&out.stderr));
    if let Some(py) = python() {
        let live = Command::new(py).arg("-I").arg(app_dir("kx").join("tests/mirror.py")).output().unwrap();
        assert_eq!(got, norm(&live.stdout));
    }
}

#[test]
fn kx_closures_program_and_error_exit_codes() {
    let out = run_app_main("kx", &["programs/closures.kx"]);
    let expected = norm(&std::fs::read(app_dir("kx").join("programs/closures.expected")).unwrap());
    assert_eq!(norm(&out.stdout), expected);
    let out = run_app_main("kx", &["programs/errors.kx"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(norm(&out.stderr).contains("SyntaxError at 1:16: unexpected ')'"), "{}", norm(&out.stderr));
    assert!(norm(&out.stderr).contains("               ^"), "caret line: {}", norm(&out.stderr));
    let out = run_app_main("kx", &["programs/runtime_error.kx"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(norm(&out.stderr).contains("RuntimeError at 3:3: undefined variable 'undefined_name'"));
    let out = run_app_main("kx", &["no_such_file.kx"]);
    assert_eq!(out.status.code(), Some(2));
    let out = run_app_main("kx", &[]);
    assert_eq!(out.status.code(), Some(2));
    let out = run_app_main("kx", &["-e", "2 ^ 100"]);
    assert_eq!(norm(&out.stdout).trim(), "=> 1267650600228229401496703205376");
}

#[test]
fn csv_parse_and_write_follow_rfc4180_and_keep_column_order() {
    let src = r#"r = parse_csv("zeta,alpha,mid\n1,2,3\n")
println(keys(r[0]))
println(len(parse_csv("a,b\n\"multi\nline\",2\n")))
println(parse_csv("a,b\n\"multi\nline\",2\n")[0]["a"] == "multi\nline")
println(parse_csv("a,b\r\n1,2\r\n\r\n3,4"))
println(parse_csv(""))
stdout_write(to_csv([["a","b"],["x,y","q\"r"],[1,2.5,null]]))
stdout_write(to_csv([{"z": 1, "a": "x,y"}, {"z": 2, "a": 2n ** 70, "new": true}]))
println(parse_csv("a;b\n1;2", ";"))
println(parse_csv("a,b\n\" x \", y ")[0]["a"] == " x ")
try {
  parse_csv("a\n\"open")
} catch e {
  println(e)
}
"#;
    assert_eq!(
        run_src(src),
        "[zeta, alpha, mid]\n1\ntrue\n[{a: 1, b: 2}, {a: 3, b: 4}]\n[]\na,b\n\"x,y\",\"q\"\"r\"\n1,2.5,\nz,a,new\n1,\"x,y\",\n2,1180591620717411303424,true\n[{a: 1, b: 2}]\ntrue\nparse_csv(): unterminated quoted field starting on line 2"
    );
}

#[test]
fn print_accepts_several_arguments_and_stdout_write_has_no_newline() {
    let src = "print(\"a\", 1, [2])\nprint(\"b\")\nstdout_write(\"x\")\nstdout_write(\"y\", 3)\nstdout_flush()\nprintln(\"\")\nprintln(\"z\")\n";
    assert_eq!(run_src(src), "a 1 [2]\nb\nxy 3\nz");
}

// ---------------------------------------------------------------------------------------------
// app 3: csvtool
// ---------------------------------------------------------------------------------------------

#[test]
fn csvtool_unit_tests_pass() {
    let (ok, text) = app_tests("csvtool");
    assert!(ok, "{text}");
    assert!(text.contains("0 failed"), "{text}");
}

#[test]
fn csvtool_summary_table_and_json_match_the_python_mirror() {
    let out = run_app_main("csvtool", &["summary", "data/sales.csv", "--group-by", "region", "--value", "amount", "--top", "3"]);
    let text = norm(&out.stdout);
    assert!(out.status.success(), "{}", norm(&out.stderr));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "data/sales.csv: 240 rows, 6 groups by region, total amount 117351.23");
    assert_eq!(lines[1], "region | count |      sum |    avg |   min |    max");
    assert_eq!(lines[2], "-------+-------+----------+--------+-------+-------");
    assert_eq!(lines[3], "Zürich |    44 | 22510.86 | 511.61 | 19.66 | 983.59");
    assert_eq!(lines.len(), 6);
    let t = Tmp::new("csvjson");
    let report = t.0.join("report.json");
    let out = run_app_main("csvtool", &["summary", "data/sales.csv", "--group-by", "region", "--value", "amount", "--json", report.to_str().unwrap()]);
    assert!(out.status.success(), "{}", norm(&out.stderr));
    assert!(std::fs::read_to_string(&report).unwrap().contains("\"group\": \"Zürich\""));
    if let Some(py) = python() {
        let live = Command::new(py)
            .arg("-I")
            .arg(app_dir("csvtool").join("tests/mirror.py"))
            .args(["data/sales.csv", "region", "amount", report.to_str().unwrap()])
            .current_dir(app_dir("csvtool"))
            .output()
            .unwrap();
        assert_eq!(norm(&live.stdout).trim(), "MATCH", "{}", norm(&live.stderr));
    }
}

#[test]
fn csvtool_exit_codes_and_messages() {
    let code = |args: &[&str]| {
        let o = run_app_main("csvtool", args);
        (o.status.code(), norm(&o.stderr))
    };
    assert_eq!(code(&["describe", "data/sales.csv"]).0, Some(0));
    assert_eq!(code(&["help"]).0, Some(0));
    assert_eq!(code(&[]).0, Some(2));
    assert_eq!(code(&["bogus", "x"]).0, Some(2));
    assert_eq!(code(&["summary", "data/sales.csv"]).0, Some(2));
    assert_eq!(code(&["summary", "data/sales.csv", "--bogus"]).0, Some(2));
    let (c, e) = code(&["describe", "no_such.csv"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("cannot read no_such.csv"), "{e}");
    let (c, e) = code(&["summary", "data/sales.csv", "--group-by", "nope", "--value", "amount"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("group-by column 'nope' not found (columns: id, region, product, units, amount)"), "{e}");
    let (c, e) = code(&["summary", "data/bad.csv", "--group-by", "region", "--value", "amount"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("line 3: value 'abc' in column 'amount' is not a number"), "{e}");
    let (c, e) = code(&["describe", "data/empty.csv"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("is empty"), "{e}");
    assert_eq!(code(&["summary", "data/bad.csv", "--group-by", "region", "--value", "amount", "--skip-bad"]).0, Some(0));
    // nothing but errors goes to stderr; stdout stays clean for data
    let o = run_app_main("csvtool", &["describe", "no_such.csv"]);
    assert!(norm(&o.stdout).is_empty());
}

#[test]
fn csvtool_handles_a_large_file() {
    let t = Tmp::new("csvbig");
    let mut text = String::from("id,region,amount\n");
    let mut sums = [0i64; 4];
    for i in 0..6000i64 {
        let r = (i * 7 + 3) % 4;
        let cents = (i * 37) % 10000;
        sums[r as usize] += cents;
        text.push_str(&format!("{},{},{}.{:02}\n", i, ["A", "B", "C", "Zü"][r as usize], cents / 100, cents % 100));
    }
    let f = t.write("big.csv", &text);
    let o = run_app_main("csvtool", &["summary", f.to_str().unwrap(), "--group-by", "region", "--value", "amount"]);
    let out = norm(&o.stdout);
    assert!(o.status.success(), "{}", norm(&o.stderr));
    let total: i64 = sums.iter().sum();
    assert!(out.contains(&format!("6,000 rows, 4 groups by region, total amount {}.{:02}", total / 100, total % 100)), "{out}");
}

#[test]
fn sorted_with_list_keys_json_pretty_of_values_and_csv_columns() {
    let src = r#"people = [{"n": "b", "a": 2}, {"n": "a", "a": 2}, {"n": "c", "a": 1}]
byage = sorted(people, fn(p) { return [0 - p["a"], p["n"]] })
for p in byage {
  println(p["n"])
}
println(sorted([3n, 1, 2n ** 70, 2], fn(x) { return x }))
println(sorted(["b", "a", "c"], fn(x) { return [x == "a", x] }))
println(json_pretty({"a": [1, 2], "b": {"c": null}}))
println(json_pretty("{\"x\":1}"))
println(csv_columns("id,name\n"))
println(csv_columns(""))
"#;
    assert_eq!(
        run_src(src),
        "a\nb\nc\n[1, 2, 3, 1180591620717411303424]\n[b, c, a]\n{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {\n    \"c\": null\n  }\n}\n{\n  \"x\": 1\n}\n[id, name]\n[]"
    );
}

#[test]
fn operator_overloading_reflected_neg_mod_and_derived_comparisons() {
    let src = r#"class R {
  fn init(v) {
    this.v = v
  }
  fn __rmul__(k) {
    return new R(k * this.v)
  }
  fn __radd__(k) {
    return new R(k + this.v)
  }
  fn __rsub__(k) {
    return new R(k - this.v)
  }
  fn __mod__(k) {
    return new R(this.v % k)
  }
  fn __lt__(o) {
    return this.v < o.v
  }
  fn toString() {
    return "R" + str(this.v)
  }
}
class N {
  fn init(v) {
    this.v = v
  }
  fn __neg__() {
    return new N(0 - this.v)
  }
  fn toString() {
    return "N" + str(this.v)
  }
}
r = new R(5)
println(3 * r)
println(3 + r)
println(10 - r)
println(r % 3)
println(r > new R(2), r >= new R(5), r <= new R(5), r < new R(1), r <= new R(4))
println(-new N(4))
fn is_five(o) {
  return o == 5
}
println(is_five(r), r == 5, r != 5)
"#;
    assert_eq!(run_src(src), "R15\nR8\nR5\nR2\ntrue true true false false\nN-4\nfalse false true");
}

#[test]
fn generators_can_be_given_to_list_sum_sorted_max_join_and_friends() {
    let src = r#"fn g(n) {
  i = 0
  while i < n {
    yield i * 2
    i = i + 1
  }
}
println(list(g(4)))
println(sum(g(4)))
println(sorted(g(3)))
println(max(g(5)))
println(join(g(3), "-"))
println(list("héy"))
println(list({"a": 1, "b": 2}))
x = g(3)
next(x)
println(list(x))
println(any(g(3)))
println(list(map(g(3), fn(v) { return v + 1 })))
println(list([1, 2]))
"#;
    assert_eq!(run_src(src), "[0, 2, 4, 6]\n12\n[0, 2, 4]\n8\n0-2-4\n[h, é, y]\n[a, b]\n[2, 4]\ntrue\n[1, 3, 5]\n[1, 2]");
}

// ---------------------------------------------------------------------------------------------
// app 4: dungeon (text adventure engine)
// ---------------------------------------------------------------------------------------------

#[test]
fn dungeon_unit_tests_pass() {
    let (ok, text) = app_tests("dungeon");
    assert!(ok, "{text}");
    assert!(text.contains("0 failed"), "{text}");
}

#[test]
fn dungeon_walkthrough_transcript_is_stable() {
    let out = run_app_main("dungeon", &["play", "data/walkthrough.txt", "--seed", "1"]);
    let expected = norm(&std::fs::read(app_dir("dungeon").join("data/walkthrough.expected")).unwrap());
    assert_eq!(norm(&out.stdout), expected, "{}", norm(&out.stderr));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn dungeon_fights_match_the_python_mirror_for_many_seeds() {
    let Some(py) = python() else { return };
    for seed in ["1", "7", "42", "12345", "-5", "0", "2147483647", "99999999999"] {
        let k = run_app_main("dungeon", &["fights", seed]);
        let p = Command::new(&py).arg("-I").arg(app_dir("dungeon").join("tests/mirror.py")).arg(seed).output().unwrap();
        assert_eq!(norm(&k.stdout), norm(&p.stdout), "seed {seed}");
    }
}

#[test]
fn dungeon_save_and_resume_continue_exactly_like_an_uninterrupted_game() {
    let t = Tmp::new("save");
    let save = t.0.join("mid.json");
    let full = norm(&run_app_main("dungeon", &["play", "data/walkthrough.txt", "--seed", "1"]).stdout);
    let first = run_app_main("dungeon", &["play", "data/part1.txt", "--seed", "1", "--save", save.to_str().unwrap()]);
    assert!(first.status.success(), "{}", norm(&first.stderr));
    let text = std::fs::read_to_string(&save).unwrap();
    assert!(text.contains("\"state\": \"combat\"") && text.contains("\"version\": 1"), "{text}");
    let second = run_app_main("dungeon", &["resume", save.to_str().unwrap(), "data/part2.txt"]);
    let second = norm(&second.stdout);
    // the fight lines after the save point are identical in both runs
    for line in second.lines().filter(|l| l.starts_with("Round ") || l.starts_with("You take the crown")) {
        assert!(full.contains(line), "resumed line not in the full run: {line}");
    }
    assert!(second.ends_with("state: won, turns: 30, hp: 4, kills: 3\n"), "{second}");
}

#[test]
fn dungeon_exit_codes() {
    let t = Tmp::new("dexit");
    let code = |args: &[&str]| run_app_main("dungeon", args).status.code();
    assert_eq!(code(&[]), Some(2));
    assert_eq!(code(&["bogus"]), Some(2));
    assert_eq!(code(&["play"]), Some(2));
    assert_eq!(code(&["play", "no_such_script.txt"]), Some(1));
    assert_eq!(code(&["resume", "no_such_save.json", "data/part2.txt"]), Some(1));
    let corrupt = t.write("corrupt.json", "{\"version\": 1, bad");
    let out = run_app_main("dungeon", &["resume", corrupt.to_str().unwrap(), "data/part2.txt"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(norm(&out.stderr).contains("invalid JSON"), "{}", norm(&out.stderr));
    // a hero with 1 hp resumes a fight with the dragon: dying exits with code 3
    let out = run_app_main("dungeon", &["resume", "data/dying_save.json", "data/one_attack.txt"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(norm(&out.stdout).contains("You die."), "{}", norm(&out.stdout));
    let empty = t.write("empty.txt", "");
    assert_eq!(code(&["play", empty.to_str().unwrap()]), Some(0));
}

#[test]
fn a_return_inside_try_does_not_leave_a_handler_that_catches_later_errors() {
    let src = r#"fn ident(x) {
  return x
}
fn parse(s) {
  try {
    return {"ok": true, "doc": ident(s)}
  } catch e {
    return {"ok": false, "caught": "STALE " + str(e)}
  }
}
class S {
  fn save() {
    throw "disk full"
  }
  fn create() {
    this.save()
    return 1
  }
}
r = parse(1)
println(r["ok"])
s = new S()
try {
  r2 = s.create()
  println("wrong:", r2)
} catch e {
  println("caught by the right handler:", e)
}
r3 = parse(2)
println(r3["ok"])
"#;
    assert_eq!(run_src(src), "true\ncaught by the right handler: disk full\ntrue");
    // and without any try around the failing call the error is reported, not swallowed
    let src2 = "fn p(s) {\n  try {\n    return s\n  } catch e {\n    return 0\n  }\n}\nfn deep() {\n  throw \"real error\"\n}\nfn mid() {\n  deep()\n  return 1\n}\np(1)\nx = mid()\nprintln(\"unreachable\", x)\n";
    let out = run_src(src2);
    assert!(out.contains("real error") && !out.contains("unreachable"), "{out}");
}

#[test]
fn a_bare_return_yields_null_and_names_starting_with_return_are_variables() {
    let src = r#"class T {
  fn init(p) {
    this.p = p
  }
  fn persist(x) {
    if this.p == null {
      return
    }
    println("persisting", x)
  }
  fn commit(x) {
    this.persist(x)
    this.v = x
  }
}
t = new T(null)
t.commit(1)
println(t.v)
fn f() {
  return
}
println(f())
y = f()
println(y == null)
t2 = new T("x")
t2.commit(2)
returned = 5
return_code = 7
fn g() {
  returns = 3
  return_value = returns + 1
  return return_value
}
println(returned, return_code, g())
"#;
    assert_eq!(run_src(src), "1\nnull\ntrue\npersisting 2\n5 7 4");
}

// ---------------------------------------------------------------------------------------------
// app 1: todo REST service (server and client are separate processes)
// ---------------------------------------------------------------------------------------------

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn start_todo_server(port: u16, data: &Path, max_requests: u32) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_killer"))
        .current_dir(app_dir("todo"))
        .args(["run", "src/main.killer", "--port", &port.to_string(), "--data", data.to_str().unwrap(), "--max-requests", &max_requests.to_string()])
        .stdin(Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn todo_unit_tests_pass() {
    let (ok, text) = app_tests("todo");
    assert!(ok, "{text}");
    assert!(text.contains("0 failed"), "{text}");
}

#[test]
fn todo_client_script_against_the_server_matches_the_golden_transcript_and_persists() {
    let t = Tmp::new("todo");
    let data = t.0.join("todos.json");
    let port = free_port();
    let server = start_todo_server(port, &data, 22);
    let client = killer(&app_dir("todo"), &["run", "src/client.killer", &port.to_string()]);
    let out = finish(server, PROCESS_LIMIT_SECS);
    assert!(client.status.success(), "{}", norm(&client.stderr));
    let expected = norm(&std::fs::read(app_dir("todo").join("tests/client.expected")).unwrap());
    assert_eq!(norm(&client.stdout), expected);
    let server_out = norm(&out.stdout);
    assert!(server_out.contains("served 22 requests"), "{server_out}{}", norm(&out.stderr));
    // the file holds the two surviving todos and the id counter
    let saved = std::fs::read_to_string(&data).unwrap();
    assert!(saved.contains("\"next_id\": 4") && saved.contains("Write the final report") && !saved.contains("Buy milk"), "{saved}");
    assert!(!t.0.join("todos.json.tmp").exists());
    // restart on the same file: the data is still there and ids keep counting
    let port2 = free_port();
    let server2 = start_todo_server(port2, &data, 2);
    let probe = |method: &str, path: &str, body: Option<&str>| {
        let src = format!(
            "r = http_request({{\"method\": \"{}\", \"url\": \"http://127.0.0.1:{}{}\", \"timeout\": 10, \"retries\": 6{}}})\nprintln(r[\"status\"], r[\"body\"])\n",
            method,
            port2,
            path,
            body.map(|b| format!(", \"body\": {:?}", b)).unwrap_or_default()
        );
        let f = t.write("probe.killer", &src);
        norm(&killer(&t.0, &["run", f.to_str().unwrap()]).stdout)
    };
    assert_eq!(probe("GET", "/health", None).trim(), "200 {\"status\":\"ok\",\"todos\":2}");
    let created = probe("POST", "/todos", Some("{\"title\": \"after restart\"}"));
    assert!(created.starts_with("201 {\"id\":4,\"title\":\"after restart\""), "{created}");
    let out2 = finish(server2, PROCESS_LIMIT_SECS);
    assert!(norm(&out2.stdout).contains("(2 todos loaded)"), "{}", norm(&out2.stdout));
}

#[test]
fn todo_server_startup_errors_and_storage_failures() {
    let t = Tmp::new("todo2");
    let run = |args: &[&str]| {
        let mut a = vec!["run", "src/main.killer"];
        a.extend_from_slice(args);
        killer(&app_dir("todo"), &a)
    };
    let o = run(&["--port", "abc"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(norm(&o.stderr).contains("--port needs a whole number"), "{}", norm(&o.stderr));
    let o = run(&["--port", "70000"]);
    assert_eq!(o.status.code(), Some(2));
    let corrupt = t.write("bad.json", "{broken");
    let o = run(&["--port", "1", "--data", corrupt.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(norm(&o.stderr).contains("todo: invalid JSON"), "{}", norm(&o.stderr));
    // an unwritable data file: the POST gets a JSON 500 and nothing is half-stored
    let port = free_port();
    let bad_path = t.0.join("no_such_dir").join("todos.json");
    // 3 requests: a readiness probe (retried until the server is listening, so a slow start on a
    // loaded machine cannot make the unretried POST below fail), the POST, and the final health check
    let server = start_todo_server(port, &bad_path, 3);
    let src = format!(
        "w = http_request({{\"url\": \"http://127.0.0.1:{p}/health\", \"timeout\": 10, \"retries\": 8}})\na = http_request({{\"method\": \"POST\", \"url\": \"http://127.0.0.1:{p}/todos\", \"body\": \"{{\\\"title\\\": \\\"x\\\"}}\", \"timeout\": 10, \"retries\": 0}})\nprintln(a[\"status\"], contains(a[\"body\"], \"internal error\"))\nb = http_request({{\"url\": \"http://127.0.0.1:{p}/health\", \"timeout\": 10, \"retries\": 6}})\nprintln(b[\"body\"])\n",
        p = port
    );
    let f = t.write("probe2.killer", &src);
    let out = norm(&killer(&t.0, &["run", f.to_str().unwrap()]).stdout);
    let s = finish(server, PROCESS_LIMIT_SECS);
    assert_eq!(out.trim(), "500 true\n{\"status\":\"ok\",\"todos\":0}", "server: {}{}", norm(&s.stdout), norm(&s.stderr));
}

#[test]
fn todo_date_validation_agrees_with_python_datetime() {
    let Some(py) = python() else { return };
    let t = Tmp::new("dates");
    let cases = t.0.join("cases.tsv");
    let g = Command::new(py).arg("-I").arg(app_dir("todo").join("tests/dates.py")).arg(&cases).output().unwrap();
    assert!(g.status.success(), "{}", norm(&g.stderr));
    let out = killer(&app_dir("todo"), &["run", "tests/check_dates.killer", cases.to_str().unwrap()]);
    let text = norm(&out.stdout);
    assert!(text.trim_end().ends_with("mismatches 0"), "{text}{}", norm(&out.stderr));
}

#[test]
fn isdir_and_isfile_tell_folders_from_files() {
    let t = Tmp::new("isdir");
    t.write("sub/a.txt", "x");
    let src = format!(
        "d = \"{}\"\nprintln(isDir(d), isFile(d), isDir(d + \"/sub\"), isFile(d + \"/sub/a.txt\"), isDir(d + \"/sub/a.txt\"), isDir(d + \"/nope\"), isFile(d + \"/nope\"))\n",
        t.0.to_str().unwrap().replace('\\', "/")
    );
    assert_eq!(run_src(&src), "true false true true false false false");
}

// ---------------------------------------------------------------------------------------------
// app 2: rag (document Q&A)
// ---------------------------------------------------------------------------------------------

fn rag(args: &[&str]) -> Output {
    run_app_main("rag", args)
}

#[test]
fn rag_unit_tests_pass() {
    let _ = std::fs::create_dir_all(app_dir("rag").join("build"));
    let (ok, text) = app_tests("rag");
    assert!(ok, "{text}");
    assert!(text.contains("0 failed"), "{text}");
}

#[test]
fn rag_ingest_ask_and_persistence_through_the_cli() {
    let t = Tmp::new("rag");
    let index = t.0.join("idx").join("docs");
    let o = rag(&["ingest", "data/docs", "--index", index.to_str().unwrap()]);
    let out = norm(&o.stdout);
    assert!(o.status.success(), "{}", norm(&o.stderr));
    assert!(out.starts_with("indexed 8 files into 27 chunks"), "{out}");
    assert!(out.contains("skipped empty.txt (empty)"), "{out}");
    // a new process loads the saved index
    let o = rag(&["ask", "How do I reset my password?", "--index", index.to_str().unwrap(), "--k", "2"]);
    let out = norm(&o.stdout);
    assert!(out.starts_with("(stub, no OPENAI_API_KEY) Resetting your password"), "{out}");
    assert!(out.contains("password-reset.txt#0"), "{out}");
    assert_eq!(out.lines().filter(|l| l.starts_with("  [")).count(), 2, "{out}");
    let o = rag(&["ask", "quantum chromodynamics lattice", "--index", index.to_str().unwrap()]);
    assert_eq!(norm(&o.stdout).trim(), "I do not know: nothing in the documents matches.");
    // unicode question and answer
    let o = rag(&["ask", "München office hours", "--index", index.to_str().unwrap(), "--k", "1"]);
    assert!(norm(&o.stdout).contains("Königsstraße"), "{}", norm(&o.stdout));
}

#[test]
fn rag_ranking_matches_the_python_mirror() {
    let Some(py) = python() else { return };
    let t = Tmp::new("ragmirror");
    let index = t.0.join("docs");
    assert!(rag(&["ingest", "data/docs", "--index", index.to_str().unwrap()]).status.success());
    let o = rag(&["dump", "when is my invoice generated", "can I delete my data", "How do I reset my password?", "München", "quantum", "--index", index.to_str().unwrap()]);
    assert!(o.status.success(), "{}", norm(&o.stderr));
    let dump = t.write("dump.json", &norm(&o.stdout));
    let m = Command::new(py).arg("-I").arg(app_dir("rag").join("tests/mirror.py")).arg(&dump).output().unwrap();
    assert_eq!(norm(&m.stdout).trim(), "MATCH", "{}", norm(&m.stderr));
}

#[test]
fn rag_exit_codes() {
    let t = Tmp::new("ragexit");
    let idx = t.0.join("none");
    let idx = idx.to_str().unwrap();
    assert_eq!(rag(&[]).status.code(), Some(2));
    assert_eq!(rag(&["bogus"]).status.code(), Some(2));
    assert_eq!(rag(&["ingest", "data/docs"]).status.code(), Some(2));
    assert_eq!(rag(&["ask", "q"]).status.code(), Some(2));
    let o = rag(&["ask", "q", "--index", idx]);
    assert_eq!(o.status.code(), Some(1));
    assert!(norm(&o.stderr).contains("no index at"), "{}", norm(&o.stderr));
    let o = rag(&["ingest", "data/no_such_folder", "--index", idx]);
    assert_eq!(o.status.code(), Some(1));
    assert!(norm(&o.stderr).contains("not a folder"), "{}", norm(&o.stderr));
    assert!(rag(&["ingest", "data/docs", "--index", idx]).status.success());
    assert_eq!(rag(&["ask", "reset", "--index", idx, "--k", "x"]).status.code(), Some(2));
    assert_eq!(rag(&["ask", "", "--index", idx]).status.code(), Some(2));
    assert_eq!(rag(&["serve", "--index", idx, "--port", "99x"]).status.code(), Some(2));
}

#[test]
fn rag_service_answers_http_requests() {
    let t = Tmp::new("ragsrv");
    let index = t.0.join("docs");
    assert!(rag(&["ingest", "data/docs", "--index", index.to_str().unwrap()]).status.success());
    let port = free_port();
    let server = Command::new(env!("CARGO_BIN_EXE_killer"))
        .current_dir(app_dir("rag"))
        .args(["run", "src/main.killer", "serve", "--index", index.to_str().unwrap(), "--port", &port.to_string(), "--max-requests", "5"])
        .stdin(Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let src = format!(
        r#"fn call(method, path, body) {{
  o = {{"method": method, "url": "http://127.0.0.1:{port}" + path, "timeout": 20, "retries": 6}}
  if body != null {{
    o["body"] = body
  }}
  return http_request(o)
}}
a = call("POST", "/ask", {{"question": "how long does express delivery take", "k": 2}})
d = json_parse(a["body"])
println(a["status"], d["sources"][0]["source"], len(d["sources"]), contains(d["answer"], "Orders ship"))
b = call("POST", "/ask", "{{oops")
println(b["status"])
c = call("POST", "/ask", {{"question": ""}})
println(c["status"], json_parse(c["body"])["error"])
e = call("GET", "/health", null)
println(e["status"], json_parse(e["body"])["chunks"])
f = call("GET", "/nope", null)
println(f["status"])
"#
    );
    let f = t.write("client.killer", &src);
    let out = norm(&killer(&t.0, &["run", f.to_str().unwrap()]).stdout);
    let s = finish(server, PROCESS_LIMIT_SECS);
    assert_eq!(
        out.trim(),
        "200 shipping.txt 2 true\n400\n422 question must be a non-empty string\n200 27\n404",
        "server: {}{}",
        norm(&s.stdout),
        norm(&s.stderr)
    );
    assert!(norm(&s.stdout).contains("served 5 requests"));
}

#[test]
fn nested_callbacks_inside_a_serve_handler_do_not_overflow_the_driver_stack() {
    let src = r#"fn inner(x) {
  return sorted([3, 1, 2], fn(v) { return v })
}
fn handler(req) {
  return {"a": map([1, 2], fn(x) { return inner(x) })}
}
port = free_port()
fn clients(port) {
  return http_request({"url": "http://127.0.0.1:" + str(port) + "/x", "timeout": 20, "retries": 6})
}
h = async_spawn(clients, port)
n = serve(port, handler, {"max_requests": 1})
r = async_await(h)
println(r["status"], r["body"])
"#;
    assert_eq!(run_src(src), "200 {\"a\":[[1,2,3],[1,2,3]]}");
}

#[test]
fn fmt_indents_a_dict_argument_one_level_and_lint_knows_the_library_builtins() {
    let t = Tmp::new("fmtlint");
    let f = t.write(
        "a.killer",
        "fn f(out) {\n  push(out, {\n    \"a\": 1,\n    \"b\": [\n      1,\n      2\n    ]\n  })\n  return out\n}\nprintln(f([]))\nprintln(mock_embed(\"x\", 4))\nprintln(vec_new(4))\nprintln(isDir(\".\"))\nprintln(eprintln(\"\"))\nprintln(csv_columns(\"a\"))\nprintln(list([1]))\n",
    );
    let out = killer(&t.0, &["fmt", "--check", f.to_str().unwrap()]);
    assert!(out.status.success(), "fmt wants to change it: {}{}", norm(&out.stdout), norm(&out.stderr));
    let out = killer(&t.0, &["lint", f.to_str().unwrap()]);
    let text = norm(&out.stdout);
    assert!(!text.contains("undefined-function"), "{text}");
}
