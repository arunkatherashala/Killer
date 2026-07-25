//! Core runtime timings: compile + VM + parse → AST compile.
//! Run: `cargo bench --bench vm_runtime` (uses `profile.bench` ≈ release)
//!
//! For HTML charts on CI, enable optional `criterion` dev-dependency and switch to Criterion harness.

use std::time::Instant;

fn time_iter(name: &str, iterations: u32, mut f: impl FnMut()) {
    let t0 = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = t0.elapsed();
    let per_us = elapsed.as_micros() as f64 / f64::from(iterations);
    println!(
        "{name}: {iterations} iters in {elapsed:?}  ({per_us:.2} µs/iter)",
        name = name,
        iterations = iterations,
        elapsed = elapsed,
        per_us = per_us
    );
}

fn main() {
    let src_trivial = "print(1)\n";
    time_iter("compile_default_trivial", 500, || {
        let _ = killer_native::compile_killer_default(src_trivial).unwrap();
    });

    let src_loop = "x = 0\ni = 0\nwhile i < 500 {\nx = x + 1\ni = i + 1\n}\nprint(x)\n";
    let program = killer_native::compile_killer_default(src_loop).unwrap();
    time_iter("vm_run_counting_loop_500", 200, || {
        let mut vm = killer_native::VirtualMachine::new();
        let _ = vm.run(&program);
    });

    let src_ast = "kfn f() {\nprint(1)\n}\nf()\n";
    time_iter("parse_compile_ast_small", 300, || {
        let stmts = killer_native::parse_killer_program(src_ast).unwrap();
        let _ = killer_native::compile_killer_ast(&stmts).unwrap();
    });
}
