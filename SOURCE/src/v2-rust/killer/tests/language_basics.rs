//! Regression tests for core-language behaviour found by the parity audit (parity/).
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_lb_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn negative_literal_before_a_function_does_not_break_jumps() {
    // `-5` compiles to `0 - 5`; folding it used to leave every later jump pointing at the wrong place
    let src = "temp = -5\nfn f(a) {\n  return a + 1\n}\nprintln(f(temp))\nif temp < 0 {\n  println(\"neg\")\n}\n";
    assert_eq!(run(src), "-4\nneg");
}

#[test]
fn negative_literals_in_loops_and_nested_functions() {
    let src = "fn g(a) {\n  return a * -2\n}\nfn h(a) {\n  t = 0\n  for i in range(a) {\n    t = t + g(i) - 1\n  }\n  return t\n}\nprintln(h(4))\n";
    assert_eq!(run(src), "-16");
}

#[test]
fn negative_array_indices_count_from_the_end() {
    let src = "a = [10, 20, 30]\nprintln(a[-1])\nprintln(a[-3])\nprintln(a[-4])\nprintln(a[3])\na[-1] = 99\nprintln(a)\nfn last(xs) {\n  return xs[-1]\n}\nprintln(last([1, 2, 3]))\n";
    assert_eq!(run(src), "30\n10\nnull\nnull\n[10, 20, 99]\n3");
}

#[test]
fn negative_index_in_a_hot_function_matches_the_interpreter() {
    // called often enough to be compiled natively; out-of-range reads fall back and still agree
    let src = "fn last(xs) {\n  return xs[-1]\n}\nfn main() {\n  s = 0\n  for i in range(200) {\n    s = s + last([i, i + 1, i + 2])\n  }\n  return s\n}\nprintln(main())\n";
    assert_eq!(run(src), "20300");
}
