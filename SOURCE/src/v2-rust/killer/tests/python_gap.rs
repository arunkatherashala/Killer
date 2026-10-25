//! Features added to close gaps found by the Python comparison (docs/PYTHON_COMPARISON.md):
//! automatic big integers, `round(x, digits)`, `int(text, base)`, `hex` / `bin` / `oct`, `uuid`,
//! format specs and printf-style `%` formatting.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Run a script through the `killer` driver; returns (exit ok, stdout without the final newline, stderr).
fn run(src: &str) -> (bool, String, String) {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_pygap_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let s = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.success(), s(&out.stdout).trim_end_matches('\n').to_string(), s(&out.stderr))
}

fn out(src: &str) -> String {
    let (ok, stdout, stderr) = run(src);
    assert!(ok, "script failed: {stderr}\n{src}");
    stdout
}

fn fails(src: &str, needle: &str) {
    let (ok, _, stderr) = run(src);
    assert!(!ok, "expected an error from: {src}");
    assert!(stderr.contains(needle), "stderr was: {stderr}");
}

// ---- automatic big integers ----------------------------------------------------------------------

#[test]
fn whole_number_results_that_leave_the_exact_float_range_become_big_integers() {
    assert_eq!(out("println(2 ** 100)"), "1267650600228229401496703205376");
    assert_eq!(out("println(10 ** 20, 10 ** 20 / 10 ** 18, 7 ** 30)"), "100000000000000000000 100 22539340290692258087863249");
    assert_eq!(out("println(2 ** 53 + 1, 2 ** 53 - 1, 2 ** 62 * 4)"), "9007199254740993 9007199254740991 18446744073709551616");
    assert_eq!(out("println(99999999 * 99999999)"), "9999999800000001");
}

#[test]
fn loops_and_variables_promote_too() {
    assert_eq!(out("f = 1\ni = 1\nwhile i < 26 {\n  f = f * i\n  i = i + 1\n}\nprintln(f)"), "15511210043330985984000000");
    assert_eq!(
        out("a = 0\nb = 1\ni = 0\nwhile i < 100 {\n  t = a + b\n  a = b\n  b = t\n  i = i + 1\n}\nprintln(a)"),
        "354224848179261915075"
    );
    assert_eq!(out("x = 4611686018427387904\nprintln(x * 2, x * 4, x + x + x)"), "9223372036854775808 18446744073709551616 13835058055282163712");
}

#[test]
fn native_compiled_functions_and_hot_loops_fall_back_to_exact_arithmetic() {
    // dbl() is called enough times to be compiled natively, then asked for a result beyond 2^53
    let f = "fn dbl(n) {\n  r = 1\n  i = 0\n  while i < n {\n    r = r * 2\n    i = i + 1\n  }\n  return r\n}\nk = 0\nwhile k < 200 {\n  dbl(10)\n  k = k + 1\n}\nprintln(dbl(100), dbl(10))";
    assert_eq!(out(f), "1267650600228229401496703205376 1024");
    let s = "fn s(n) {\n  t = 0\n  i = 0\n  while i < n {\n    t = t + 4000000000000\n    i = i + 1\n  }\n  return t\n}\nk = 0\nwhile k < 100 {\n  s(10)\n  k = k + 1\n}\nprintln(s(5000))";
    assert_eq!(out(s), "20000000000000000");
    // a hot top-level accumulator loop whose total passes 2^53
    assert_eq!(out("i = 0\ns = 0\nwhile i < 3000 {\n  s += 5000000000000\n  i += 1\n}\nprintln(s)"), "15000000000000000");
    assert_eq!(out("i = 0\ns = 0\nwhile i < 100000 {\n  s += 3\n  i += 1\n}\nprintln(s)"), "300000");
}

#[test]
fn ordinary_float_arithmetic_is_unchanged() {
    assert_eq!(
        out("println(0.1 + 0.2, 1.5 * 4, 7 / 2, 2 ** 0.5, 2 ** -2, 10 ** 15)"),
        "0.30000000000000004 6 3.5 1.4142135623730951 0.25 1000000000000000"
    );
    assert_eq!(out("println(1e308 * 10, 1e15 + 0.5, 3 * 1e18)"), "inf 1000000000000000.5 3000000000000000000");
    assert_eq!(out("println(2 ** 10, 3 * 4 + 5, 1000000 * 1000000)"), "1024 17 1000000000000");
}

#[test]
fn negation_and_identity_operations_do_not_promote() {
    // -x, x * 1, x + 0 lose nothing, so a big plain number stays a plain number (range() needs numbers)
    assert_eq!(
        out("x = -9223372036854775807
y = 5000000000000000000
println(type(x), type(-y), type(y * 1), type(y + 0), type(0 - y))"),
        "number number number number number"
    );
    assert_eq!(out("println(len(range(-9223372036854775807, 9223372036854775807, 4611686018427387904)))"), "4");
}

#[test]
fn promoted_values_compare_sort_and_serialise() {
    assert_eq!(
        out("big = 2 ** 100\nprintln(big > 2 ** 99, big == 2 ** 100, big % 7, big / 2 ** 98, type(big))"),
        "true true 2 4 bigint"
    );
    assert_eq!(out("println(sorted([2 ** 100, 5, 2 ** 60]))"), "[5, 1152921504606846976, 1267650600228229401496703205376]");
    assert_eq!(out("println(json_stringify([2 ** 70]))"), "[1180591620717411303424]");
}

// ---- round / int / hex / uuid --------------------------------------------------------------------

#[test]
fn round_with_digits() {
    assert_eq!(out("println(round(2.567, 2), round(2.5, 0), round(1234.5678, -2), round(-2.567, 1), round(7))"), "2.57 3 1200 -2.6 7");
    fails("println(round(1.5, 1.5))", "whole number");
}

#[test]
fn int_with_a_base_and_hex_bin_oct() {
    assert_eq!(out("println(int(\"ff\", 16), int(\"-101\", 2), int(\"0x1f\", 16), int(\"0b11\", 0), int(\"z\", 36), int(\" 12 \", 10))"), "255 -5 31 3 35 12");
    assert_eq!(out("println(hex(255), hex(-255), bin(5), oct(8), hex(0))"), "0xff -0xff 0b101 0o10 0x0");
    fails("println(int(\"zz\", 10))", "cannot convert");
    fails("println(hex(1.5))", "whole number");
}

#[test]
fn uuid_is_a_random_version_4_identifier() {
    assert_eq!(out("u = uuid()\nprintln(len(u), u[8], u[13], u[14], u != uuid())"), "36 - - 4 true");
    let variant = out("u = uuid()\nprintln(u[19])");
    assert!(["8", "9", "a", "b"].contains(&variant.as_str()), "variant digit {variant}");
}

// ---- format specs and % formatting ---------------------------------------------------------------

#[test]
fn format_specs() {
    assert_eq!(
        out("println(format(\"{:>8.2f}|{:<6}|{:^7}|{:05d}|{:+d}|{:x}|{:#X}|{:b}\", 3.14159, \"ab\", \"mid\", 42, 5, 255, 255, 5))"),
        "    3.14|ab    |  mid  |00042|+5|ff|0XFF|101"
    );
    assert_eq!(
        out("println(format(\"{:,}|{:,.2f}|{:e}|{:.1%}|{:*^9}|{:8s}|\", 1234567, 9876.543, 1500, 0.256, \"hi\", \"x\"))"),
        "1,234,567|9,876.54|1.500000e+03|25.6%|***hi****|x       |"
    );
    assert_eq!(out("println(format(\"{0}-{1}-{0}\", \"a\", \"b\"))"), "a-b-a");
    fails("println(format(\"{:q}\", 1))", "unknown format type");
    fails("println(format(\"{:d}\", 1.5))", "whole number");
    fails("println(format(\"{:9999999d}\", 1))", "too large");
}

#[test]
fn percent_formatting() {
    assert_eq!(out("println(\"%d items\" % 3)"), "3 items");
    assert_eq!(out("println(\"%s is %5.1f%% done, id %04d, hex %x\" % [\"job\", 42.567, 7, 255])"), "job is  42.6% done, id 0007, hex ff");
    assert_eq!(out("println(\"%-6s|%6s|%+d|%c%c\" % [\"ab\", \"cd\", 3, \"x\", 65])"), "ab    |    cd|+3|xA");
    assert_eq!(out("println(\"100%%\" % [])"), "100%");
    // numeric % is untouched
    assert_eq!(out("println(5 % 3, -7 % 3, 7.5 % 2)"), "2 2 1.5");
    fails("println(\"%d\" % [1, 2])", "not all arguments");
    fails("println(\"%d %d\" % 1)", "not enough arguments");
    fails("println(\"%y\" % 1)", "unsupported");
}

// ---- with statement ------------------------------------------------------------------------------

const RES: &str = "class R {
  fn init(n) {
    this.n = n
  }
  fn __enter__() {
    println(\"open \" + this.n)
    return this
  }
  fn __exit__(err) {
    println(\"close \" + this.n + \" \" + str(err))
  }
}
";

#[test]
fn with_runs_enter_body_and_exit() {
    let src = format!("{RES}with new R(\"a\") as r {{
  println(\"body \" + r.n)
}}
println(\"after\")
");
    assert_eq!(out(&src), "open a
body a
close a null
after");
    // without `as`
    let src = format!("{RES}with new R(\"b\") {{
  println(\"body\")
}}
");
    assert_eq!(out(&src), "open b
body
close b null");
}

#[test]
fn with_exits_on_errors_return_break_and_continue() {
    let src = format!("{RES}try {{
  with new R(\"c\") as r {{
    throw \"boom\"
  }}
}} catch e {{
  println(\"caught \" + e)
}}
");
    assert_eq!(out(&src), "open c
close c boom
caught boom");
    let src = format!("{RES}fn f() {{
  with new R(\"d\") as r {{
    return 5
  }}
  return 0
}}
println(f())
");
    assert_eq!(out(&src), "open d
close d null
5");
    let src = format!("{RES}for i in range(3) {{
  with new R(\"e\" + str(i)) as r {{
    if i == 0 {{
      continue
    }}
    if i == 1 {{
      break
    }}
  }}
}}
");
    assert_eq!(out(&src), "open e0
close e0 null
open e1
close e1 null");
}

#[test]
fn nested_with_closes_in_reverse_order() {
    let src = format!("{RES}with new R(\"x\") as a {{
  with new R(\"y\") as b {{
    println(\"inner\")
  }}
}}
");
    assert_eq!(out(&src), "open x
open y
inner
close y null
close x null");
}

#[test]
fn with_errors_point_at_the_original_line() {
    let src = format!("{RES}with new R(\"z\") as r {{
  x = 1
  y = missing_function(x)
}}
");
    let (ok, _, stderr) = run(&src);
    assert!(!ok);
    // the failing call is on line 15 of the script (RES is 12 lines, then `with`, `x = 1`, the call), not
    // on a line of the lowered try/catch/finally text
    assert!(stderr.contains("Line 15"), "stderr was: {stderr}");
}

#[test]
fn an_identifier_called_with_is_not_a_statement() {
    assert_eq!(out("with = 5
println(with + 1)"), "6");
}

// ---- property getters and setters ----------------------------------------------------------------

const TEMP: &str = "class Temp {
  fn init(c) {
    this._c = c
  }
  get celsius() {
    return this._c
  }
  set celsius(v) {
    if v < -273 {
      throw \"below absolute zero\"
    }
    this._c = v
  }
  get fahrenheit() {
    return this._c * 9 / 5 + 32
  }
}
";

#[test]
fn getters_and_setters_work_like_python_properties() {
    let src = format!("{TEMP}t = new Temp(100)\nprintln(t.celsius, t.fahrenheit)\nt.celsius = 0\nprintln(t.celsius, t.fahrenheit)\nprintln(t._c)\n");
    assert_eq!(out(&src), "100 212\n0 32\n0");
}

#[test]
fn a_setter_can_reject_a_value() {
    let src = format!("{TEMP}t = new Temp(1)\ntry {{\n  t.celsius = -300\n}} catch e {{\n  println(\"rejected: \" + e)\n}}\nprintln(t.celsius)\n");
    assert_eq!(out(&src), "rejected: below absolute zero\n1");
}

#[test]
fn properties_are_inherited_and_methods_can_use_them() {
    let src = format!("{TEMP}class Sensor extends Temp {{\n  fn describe() {{\n    return \"it is \" + str(this.celsius) + \" C\"\n  }}\n}}\ns = new Sensor(21)\nprintln(s.describe())\n");
    assert_eq!(out(&src), "it is 21 C");
}

#[test]
fn a_real_field_wins_over_a_getter_and_plain_objects_are_unaffected() {
    let src = "class A {\n  fn init() {\n    this.x = 1\n  }\n  get x() {\n    return 99\n  }\n  get y() {\n    return 2\n  }\n}\na = new A()\nprintln(a.x, a.y)\nclass P {\n  fn init(v) {\n    this.v = v\n  }\n}\np = new P(5)\np.v = p.v + 1\nprintln(p.v, p.missing)\n";
    assert_eq!(out(src), "1 2\n6 null");
}

#[test]
fn get_and_set_stay_usable_as_names() {
    assert_eq!(out("d = {\"a\": 1}\nprintln(get(d, \"b\", 7))\nget = 3\nset = 4\nprintln(get + set)"), "7\n7");
}
