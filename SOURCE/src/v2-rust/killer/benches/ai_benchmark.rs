//! Lightweight AI-shaped timing loops (no network, no crates.io deps).
//! Run: `cargo bench --bench ai_benchmark`

use std::time::{Duration, Instant};

fn burn_cpu_until(target: Duration) {
    let start = Instant::now();
    while start.elapsed() < target {
        std::hint::black_box(vec![0u64; 256]);
    }
}

fn time_iter(name: &str, iterations: u32, mut f: impl FnMut()) {
    let t0 = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = t0.elapsed();
    let per_ms = elapsed.as_secs_f64() * 1000.0 / f64::from(iterations);
    println!(
        "{name}: {iterations} iters in {elapsed:?}  ({per_ms:.3} ms/iter)",
        name = name,
        iterations = iterations,
        elapsed = elapsed,
        per_ms = per_ms
    );
}

fn main() {
    time_iter("ai_shape_generate_2ms", 20, || {
        burn_cpu_until(Duration::from_millis(2));
    });
    time_iter("ai_shape_embed_2ms", 20, || {
        burn_cpu_until(Duration::from_millis(2));
    });
}
