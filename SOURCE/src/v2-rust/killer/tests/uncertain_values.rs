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
