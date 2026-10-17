//! `%` is floor modulo (Python): the result has the sign of the divisor. `rem(a, b)` is the
//! truncated remainder (sign of the dividend) and `mod(a, b)` is an alias of `%`.
//!
//! The 500+ expected values in tests/data/modulo_cases.txt are computed by Python
//! (tests/data/gen_modulo_cases.py) and checked in every execution mode: interpreted, constant
//! folded, native function JIT, loops over fused slot instructions, and the `rem`/`mod` builtins.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_mod_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

struct Case {
    kind: char,
    a: String,
    b: String,
    modulo: String,
    rem: String,
}

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/modulo_cases.txt");

fn cases() -> Vec<Case> {
    let text = std::fs::read_to_string(DATA).unwrap();
    text.lines()
        .filter(|l| !l.starts_with('L'))
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            Case { kind: p[0].chars().next().unwrap(), a: p[1].into(), b: p[2].into(), modulo: p[3].into(), rem: p[4].into() }
        })
        .collect()
}

fn loop_expected(name: &str) -> String {
    let text = std::fs::read_to_string(DATA).unwrap();
    text.lines().find_map(|l| l.strip_prefix(&format!("L {} ", name))).unwrap().to_string()
}

/// A Killer literal for the case operand: negative numbers are parenthesised, bigints get an `n`.
fn lit(kind: char, s: &str) -> String {
    match kind {
        'B' => {
            if let Some(d) = s.strip_prefix('-') {
                format!("(0n - {}n)", d)
            } else {
                format!("{}n", s)
            }
        }
        'F' => {
            let v: f64 = s.parse().unwrap();
            let t = format!("{}", v); // plain decimal, round-trips, never an exponent
            let t = if t.contains('.') { t } else { format!("{}.0", t) };
            if v < 0.0 { format!("({})", t) } else { t }
        }
        _ => {
            if s.starts_with('-') { format!("({})", s) } else { s.to_string() }
        }
    }
}

fn same(kind: char, got: &str, want: &str, ctx: &str) {
    if kind == 'B' {
        assert_eq!(got, want, "{}", ctx);
    } else {
        let g: f64 = got.parse().unwrap_or_else(|_| panic!("{}: unparsable output {:?}", ctx, got));
        let w: f64 = want.parse().unwrap();
        assert!(g == w, "{}: got {} want {}", ctx, got, want);
    }
}

fn check_lines(out: &str, cs: &[&Case], pick: impl Fn(&Case) -> &String, what: &str) {
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), cs.len(), "{}: wrong line count, output starts {:?}", what, &out[..out.len().min(300)]);
    for (c, got) in cs.iter().zip(lines) {
        same(c.kind, got, pick(c), &format!("{}: {} % {}", what, c.a, c.b));
    }
}

fn numeric(cs: &[Case]) -> Vec<&Case> {
    cs.iter().filter(|c| c.kind != 'B').collect()
}

fn pair_list(nums: &[&Case]) -> String {
    // one line: a multi-line top-level list literal is not supported by the statement parser
    let items: Vec<String> = nums.iter().map(|c| format!("[{}, {}]", lit(c.kind, &c.a), lit(c.kind, &c.b))).collect();
    format!("cases = [{}]
", items.join(", "))
}

#[test]
fn data_file_has_enough_cases() {
    let cs = cases();
    assert!(cs.len() >= 500, "{} cases", cs.len());
    assert!(cs.iter().any(|c| c.kind == 'I') && cs.iter().any(|c| c.kind == 'F') && cs.iter().any(|c| c.kind == 'B'));
}

#[test]
fn interpreted_operator_matches_python() {
    let cs = cases();
    let nums = numeric(&cs);
    let src = format!("{}for c in cases {{\n  println(c[0] % c[1])\n}}\n", pair_list(&nums));
    check_lines(&run(&src), &nums, |c| &c.modulo, "interpreted");
}

#[test]
fn constant_folded_operator_matches_python() {
    let cs = cases();
    let nums = numeric(&cs);
    let mut src = String::new();
    for c in &nums {
        src.push_str(&format!("println({} % {})\n", lit(c.kind, &c.a), lit(c.kind, &c.b)));
    }
    check_lines(&run(&src), &nums, |c| &c.modulo, "folded");
    // the same folded expression inside a function body
    let mut src = String::from("fn f() {\n");
    for c in &nums {
        src.push_str(&format!("  println({} % {})\n", lit(c.kind, &c.a), lit(c.kind, &c.b)));
    }
    src.push_str("}\nf()\n");
    check_lines(&run(&src), &nums, |c| &c.modulo, "folded in fn");
}

#[test]
fn compound_assignment_matches_python() {
    let cs = cases();
    let nums = numeric(&cs);
    let mut src = String::new();
    for c in &nums {
        src.push_str(&format!("x = {}\nx %= {}\nprintln(x)\n", lit(c.kind, &c.a), lit(c.kind, &c.b)));
    }
    check_lines(&run(&src), &nums, |c| &c.modulo, "%=");
}

#[test]
fn native_function_jit_matches_python() {
    let cs = cases();
    let nums = numeric(&cs);
    // 100 warm-up calls compile `m` natively; every case then runs through the compiled code
    let src = format!(
        "fn m(a, b) {{\n  return a % b\n}}\nw = 0\nfor i in range(100) {{\n  w = w + m(i + 1, 7)\n}}\n{}for c in cases {{\n  println(m(c[0], c[1]))\n}}\n",
        pair_list(&nums)
    );
    check_lines(&run(&src), &nums, |c| &c.modulo, "function JIT");
}

#[test]
fn builtins_mod_and_rem_match_python() {
    let cs = cases();
    let picks: [(&str, fn(&Case) -> &String); 2] = [("mod", |c| &c.modulo), ("rem", |c| &c.rem)];
    for (name, pick) in picks {
        for kinds in [&['I', 'F'][..], &['B'][..]] {
            let sel: Vec<&Case> = cs.iter().filter(|c| kinds.contains(&c.kind)).collect();
            let mut src = String::new();
            for c in &sel {
                src.push_str(&format!("println({}({}, {}))\n", name, lit(c.kind, &c.a), lit(c.kind, &c.b)));
            }
            check_lines(&run(&src), &sel, pick, name);
        }
    }
}

#[test]
fn bigint_operator_matches_python() {
    let cs = cases();
    let sel: Vec<&Case> = cs.iter().filter(|c| c.kind == 'B').collect();
    let mut src = String::new();
    for c in &sel {
        src.push_str(&format!("println({} % {})\n", lit('B', &c.a), lit('B', &c.b)));
    }
    check_lines(&run(&src), &sel, |c| &c.modulo, "bigint %");
}

#[test]
fn fused_slot_loops_and_hot_functions_match_python() {
    let src = "s = 0\nfor i in range(-5000, 5000) {\n  s = s + i % 7\n}\nprintln(s)\n\
               s = 0\ni = 0 - 5000\nwhile i < 5000 {\n  s = s + i % 7\n  i = i + 1\n}\nprintln(s)\n\
               s = 0\ni = 0 - 5000\nwhile i < 5000 {\n  s = s + i % -7\n  i = i + 1\n}\nprintln(s)\n\
               s = 0\nfor i in range(-6000, 6000) {\n  s = s + (i * 0.5) % 2.5\n}\nprintln(s)\n\
               fn loopsum(n) {\n  s = 0\n  i = 0\n  while i < n {\n    s = s + ((i - 2500) % 11) * (i % 3 + 1)\n    i = i + 1\n  }\n  return s\n}\nprintln(loopsum(5000))\n\
               fn m(a, b) {\n  return a % b\n}\nfn main() {\n  s = 0\n  for i in range(2000) {\n    s = s + m(i - 1000, 7) + m(i - 1000, -7)\n  }\n  return s\n}\nprintln(main())\n";
    let out = run(src);
    let lines: Vec<&str> = out.lines().collect();
    let names = ["range7", "while7", "whileneg7", "float", "fnloop", "fncalls"];
    assert_eq!(lines.len(), names.len(), "{}", out);
    for (n, got) in names.iter().zip(lines) {
        same('F', got, &loop_expected(n), n);
    }
}

#[test]
fn documented_examples() {
    let src = "println(-7 % 3)\nprintln(7 % -3)\nprintln(-7 % -3)\nprintln(5.5 % 2)\nprintln(-5.5 % 2)\n\
               println(rem(-7, 3))\nprintln(rem(7, -3))\nprintln(rem(-5.5, 2))\nprintln(mod(-7, 3))\nprintln(divmod(-7, 3))\n";
    assert_eq!(run(src), "2\n-2\n-1\n1.5\n0.5\n-1\n1\n-1.5\n2\n[-3, 2]");
}

#[test]
fn zero_results_print_as_plain_zero() {
    // floor modulo of an exact multiple is +0, never -0
    assert_eq!(run("println(-6 % 3)\nprintln(6 % -3)\nprintln(-6.0 % 3.0)\nprintln(rem(-6, 3))\n"), "0\n0\n0\n0");
}

#[test]
fn division_by_zero_still_errors_for_all_three() {
    let src = "try { println(5 % 0) } catch e { println(e) }\ntry { println(rem(5, 0)) } catch e { println(e) }\n\
               try { println(mod(5, 0)) } catch e { println(e) }\ntry { println(rem(5n, 0n)) } catch e { println(e) }\n\
               try { println(5n % 0n) } catch e { println(e) }\n";
    assert_eq!(run(src), "Modulo by zero\nModulo by zero\nModulo by zero\nModulo by zero\nModulo by zero");
}

#[test]
fn infinities_and_nan_follow_python() {
    let src = "inf = 1e999\nprintln(5 % inf)\nprintln(-5 % inf)\nprintln(5 % -inf)\nprintln(-5 % -inf)\nprintln(inf % 3)\n\
               println(rem(5, inf))\nprintln(rem(-5, inf))\nprintln(rem(inf, 3))\nprintln(mod(-5, inf))\n";
    assert_eq!(run(src), "5\ninf\n-inf\n-5\nNaN\n5\n-5\nNaN\ninf");
}

#[test]
fn script_defined_mod_and_rem_win_over_the_builtins() {
    let src = "fn mod(a, b) {\n  return \"mine\"\n}\nfn rem(a, b) {\n  return \"also mine\"\n}\nprintln(mod(7, 3))\nprintln(rem(7, 3))\nprintln(7 % 3)\n";
    assert_eq!(run(src), "mine\nalso mine\n1");
}

#[test]
fn builtins_accept_mixed_bigint_and_number_arguments() {
    assert_eq!(run("println(rem(-7n, 3))\nprintln(mod(-7, 3n))\nprintln(rem(7n, -3n))\nprintln(type(mod(-7n, 3n)))\n"), "-1\n2\n1\nbigint");
}
