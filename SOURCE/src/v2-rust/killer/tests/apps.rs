//! Real-application trial: the five example apps under examples/apps/ run through the `killer`
//! driver (`killer test`, `killer run`), plus regression tests for every bug the trial found
//! (see examples/apps/FRICTION.md).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn norm(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n")
}

fn killer(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_killer")).current_dir(cwd).args(args).output().unwrap()
}

fn killer_pkg(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_killer_pkg")).current_dir(cwd).args(args).output().unwrap()
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
        "1000000000000000000000\n-10000000000000000000000000\n1180591620717411300000\n0.0000001\n2501\n0.06\n4\n123456789012345680"
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
