use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_unc_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).trim().replace("\r\n", "\n")
}

const AB: &str = "believe a = 10 \u{b1} 1\nbelieve b = 20 \u{b1} 2\n";

#[test]
fn arithmetic_propagates_guaranteed_margins() {
    let src = format!("{}println(a + b)\nprintln(b - a)\nprintln(a * 2)\nprintln(a + 5)\n", AB);
    assert_eq!(run(&src), "30 \u{b1} 3\n10 \u{b1} 3\n20 \u{b1} 2\n15 \u{b1} 1");
}

#[test]
fn comparisons_are_three_valued() {
    let src = format!("{}println(a > 5)\nprintln(a < 5)\nprintln(a > 10)\nprintln(a < b)\nprintln(a == 10)\n", AB);
    assert_eq!(run(&src), "T_POS\nT_NEG\nT_ZERO\nT_POS\nT_ZERO");
}

#[test]
fn if_takes_a_branch_only_when_the_comparison_is_certain() {
    let src = format!("{}if a > 5 {{\n  println(\"yes\")\n}}\nif a > 10 {{\n  println(\"no\")\n}}\nif a < 5 {{\n  println(\"never\")\n}}\nprintln(\"done\")\n", AB);
    assert_eq!(run(&src), "yes\ndone");
}

#[test]
fn accessors_and_constructor() {
    let src = "x = uncertain(3, 0.5)\nprintln(x)\nprintln(unc_value(x))\nprintln(unc_margin(x))\nprintln(unc_lo(x))\nprintln(unc_hi(x))\nprintln(unc_hi(7))\n";
    assert_eq!(run(src), "3 \u{b1} 0.5\n3\n0.5\n2.5\n3.5\n7");
}

#[test]
fn division_and_sqrt_domain_errors_are_reported() {
    let div = format!("{}believe z = 1 \u{b1} 2\nprintln(a / z)\n", AB);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_killer_super"));
    drop(out);
    assert_eq!(run(&div), ""); // error goes to stderr, nothing on stdout
    let ok = format!("{}println(unc_value(b / a))\nprintln(abs(a - b))\n", AB);
    assert_eq!(run(&ok), "2\n10 \u{b1} 3");
}

#[test]
fn results_stay_correct_in_loops_and_functions() {
    // accumulate measurements: margins add up, comparison stays honest
    let src = "fn total(n) {\n  t = uncertain(0, 0)\n  for i in range(n) {\n    t = t + uncertain(10, 0.5)\n  }\n  return t\n}\nr = total(4)\nprintln(r)\nprintln(r > 37)\nprintln(r > 39)\n";
    assert_eq!(run(src), "40 \u{b1} 2\nT_POS\nT_ZERO");
}

fn trit_name(t: i8) -> &'static str {
    match t {
        1 => "T_POS",
        0 => "T_ZERO",
        _ => "T_NEG",
    }
}

#[test]
fn logical_operators_follow_kleene_three_valued_logic() {
    // T = certain, U = undecidable, F = impossible
    let mut src = String::from("believe a = 10 \u{b1} 1\nT = a > 5\nU = a > 10\nF = a < 5\n");
    let vals = [("T", 1i8), ("U", 0i8), ("F", -1i8)];
    let mut expected: Vec<String> = Vec::new();
    for (na, va) in vals {
        for (nb, vb) in vals {
            src.push_str(&format!("println({} && {})\nprintln({} || {})\n", na, nb, na, nb));
            expected.push(trit_name(va.min(vb)).to_string());
            expected.push(trit_name(va.max(vb)).to_string());
        }
    }
    for (na, va) in vals {
        src.push_str(&format!("println(!{})\n", na));
        expected.push(trit_name(-va).to_string());
    }
    assert_eq!(run(&src), expected.join("\n"));
}

#[test]
fn trits_mix_with_plain_booleans() {
    let src = "believe a = 10 \u{b1} 1\nU = a > 10\nprintln(true && U)\nprintln(false && U)\nprintln(true || U)\nprintln(false || U)\n";
    assert_eq!(run(src), "T_ZERO\nT_NEG\nT_POS\nT_ZERO");
}

#[test]
fn conditions_only_run_when_certain() {
    let src = "believe a = 10 \u{b1} 1\nbelieve b = 20 \u{b1} 2\nif a > 5 && b > 15 {\n  println(\"both certain\")\n}\nif a > 5 && b > 19 {\n  println(\"not printed\")\n}\nif !(a > 10) {\n  println(\"not printed either\")\n}\nif a > 5 || b > 19 {\n  println(\"one certain is enough\")\n}\nprintln(\"done\")\n";
    assert_eq!(run(src), "both certain\none certain is enough\ndone");
}

#[test]
fn plain_boolean_logic_is_unchanged() {
    let src = "println(true && false)\nprintln(true || false)\nprintln(!true)\nprintln(1 < 2 && 2 < 3)\n";
    assert_eq!(run(src), "false\ntrue\nfalse\ntrue");
}

#[test]
fn gauss_values_combine_in_quadrature() {
    let src = "a = gauss(10, 3)\nb = gauss(20, 4)\nprintln(a + b)\nprintln(b - a)\nprintln(a * 2)\nprintln(unc_value(a + b))\nprintln(unc_sigma(a + b))\n";
    assert_eq!(run(src), "30 \u{b1} 5\u{3c3}\n10 \u{b1} 5\u{3c3}\n20 \u{b1} 6\u{3c3}\n30\n5");
}

#[test]
fn gauss_comparisons_need_about_95_percent_confidence() {
    // sigma of a - b is 5: a < b is 2 sigma (decided), a > 5 is only 1.67 sigma (undecided)
    let src = "a = gauss(10, 3)\nb = gauss(20, 4)\nprintln(a < b)\nprintln(a > b)\nprintln(a > 5)\nprintln(a > 0)\n";
    assert_eq!(run(src), "T_POS\nT_NEG\nT_ZERO\nT_POS");
}

#[test]
fn gauss_probabilities_and_conditions() {
    let src = "a = gauss(10, 3)\nb = gauss(20, 4)\np = prob_gt(b, a)\nprintln(p > 0.9772)\nprintln(p < 0.9773)\nprintln(prob_lt(b, a) < 0.0228)\nif a > 0 {\n  println(\"positive\")\n}\nif a > 5 {\n  println(\"not printed\")\n}\n";
    assert_eq!(run(src), "true\ntrue\ntrue\npositive");
}

#[test]
fn gauss_accumulates_over_a_loop() {
    // nine independent measurements: sigma grows as sqrt(n) = 3, not n = 9
    let src = "total = gauss(0, 0)\nfor i in range(9) {\n  total = total + gauss(10, 1)\n}\nprintln(unc_value(total))\nprintln(unc_sigma(total) > 2.999)\nprintln(unc_sigma(total) < 3.001)\n";
    assert_eq!(run(src), "90\ntrue\ntrue");
}

#[test]
fn interval_and_gauss_cannot_be_mixed() {
    let src = "believe a = 10 \u{b1} 1\nb = gauss(10, 1)\nprintln(\"before\")\nprintln(a + b)\nprintln(\"after\")\n";
    assert_eq!(run(src), "before"); // the mixed expression is an error, nothing after it runs
}

#[test]
fn gauss_works_in_fused_slot_comparisons_and_kleene_logic() {
    let src = "fn f() {\n  x = gauss(5, 1)\n  y = x > 0\n  z = x > 5\n  println(y && z)\n  println(y || z)\n  println(!z)\n}\nf()\n";
    assert_eq!(run(src), "T_ZERO\nT_POS\nT_ZERO");
}

#[test]
fn trit_constants_are_plain_names() {
    let src = "x = T_POS\nprintln(x)\nprintln(x == T_POS)\nprintln(T_ZERO)\nbelieve a = 10 \u{b1} 1\nt = a > 10\nprintln(t == T_ZERO)\nfn f(T_ZERO) {\n  return T_ZERO\n}\nprintln(f(7))\n";
    assert_eq!(run(src), "T_POS\ntrue\nT_ZERO\ntrue\n7");
}
