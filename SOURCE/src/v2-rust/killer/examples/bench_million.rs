/// Benchmark: How many ghost capsule runs per second can we achieve in-process?
use killer_native::ghost_vm::{self, Capsule, NullHost, RunStatus, OP_PUSH, OP_ADD, OP_DUP, OP_MUL, OP_HALT, OP_LOAD};
use std::time::Instant;

fn main() {
    // Child code: load RAM[0], dup, mul, halt  → computes X²
    let child_code = vec![OP_LOAD, 0, 0, OP_DUP, OP_MUL, OP_HALT];

    // Warm up
    let mut c = Capsule::with_ram_and_fuel(64, 100);
    c.code = child_code.clone();
    c.ram[0..4].copy_from_slice(&7i32.to_le_bytes());
    let mut h = NullHost;
    let _ = ghost_vm::run(&mut c, &mut h, Some(100));

    // Benchmark: 1 million capsule runs
    let n = 1_000_000;
    let t0 = Instant::now();
    for i in 0..n {
        let mut cap = Capsule::with_ram_and_fuel(64, 100);
        cap.code = child_code.clone();
        cap.ram[0..4].copy_from_slice(&(i as i32).to_le_bytes());
        let mut host = NullHost;
        let _ = ghost_vm::run(&mut cap, &mut host, Some(100));
    }
    let elapsed = t0.elapsed();
    let per_sec = n as f64 / elapsed.as_secs_f64();

    println!("╔══════════════════════════════════════════╗");
    println!("║  GHOST VM THROUGHPUT BENCHMARK           ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("  Capsule runs:  {}", n);
    println!("  Time:          {:.2}s", elapsed.as_secs_f64());
    println!("  Throughput:    {:.0} capsules/sec", per_sec);
    println!("  Per capsule:   {:.1}µs", elapsed.as_micros() as f64 / n as f64);
    println!();

    // Now benchmark with reuse (reset PC instead of allocating new capsule)
    let t1 = Instant::now();
    let mut reuse = Capsule::with_ram_and_fuel(64, 100);
    reuse.code = child_code.clone();
    for i in 0..n {
        reuse.pc = 0;
        reuse.stack.clear();
        reuse.ram[0..4].copy_from_slice(&(i as i32).to_le_bytes());
        let mut host = NullHost;
        let _ = ghost_vm::run(&mut reuse, &mut host, Some(100));
    }
    let elapsed2 = t1.elapsed();
    let per_sec2 = n as f64 / elapsed2.as_secs_f64();

    println!("  [REUSE MODE - reset PC, no alloc]");
    println!("  Throughput:    {:.0} capsules/sec", per_sec2);
    println!("  Per capsule:   {:.1}µs", elapsed2.as_micros() as f64 / n as f64);
    println!("  Speedup:       {:.1}x vs fresh alloc", per_sec2 / per_sec);
    println!();

    // Evolution sim: 1000 generations × 1000 population
    let pop = 1000;
    let gens = 1000;
    let t2 = Instant::now();
    let mut best_score: i64 = 0;
    let mut best_input: i32 = 0;
    let mut rng: u32 = 12345;
    for _gen in 0..gens {
        for _ in 0..pop {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let input = ((rng >> 16) & 0x7FFF) as i32;
            reuse.pc = 0;
            reuse.stack.clear();
            reuse.ram[0..4].copy_from_slice(&input.to_le_bytes());
            let mut host = NullHost;
            let _ = ghost_vm::run(&mut reuse, &mut host, Some(100));
            let score = reuse.stack.last().copied().unwrap_or(0);
            if score > best_score {
                best_score = score;
                best_input = input;
            }
        }
    }
    let elapsed3 = t2.elapsed();
    let total_evals = pop * gens;
    println!("  [EVOLUTION SIM: {} gens × {} pop = {} evals]", gens, pop, total_evals);
    println!("  Time:          {:.2}s", elapsed3.as_secs_f64());
    println!("  Throughput:    {:.0} evals/sec", total_evals as f64 / elapsed3.as_secs_f64());
    println!("  Best:          input={}, score={}", best_input, best_score);
    println!();

    // Project: how long for 1M agents evolving 24/7?
    let rate = total_evals as f64 / elapsed3.as_secs_f64();
    let agents_1m = 1_000_000.0;
    let evals_per_day = rate * 86400.0;
    let evals_per_year = evals_per_day * 365.0;
    println!("  [PROJECTION at {:.0} evals/sec]", rate);
    println!("  Per minute:    {:.0} evals", rate * 60.0);
    println!("  Per hour:      {:.0} evals", rate * 3600.0);
    println!("  Per day:       {:.2}B evals", evals_per_day / 1e9);
    println!("  Per year:      {:.1}T evals", evals_per_year / 1e12);
    println!("  1M agents × 1 eval/min = {:.1} evals/sec (need {:.1}x cores)", 
        agents_1m / 60.0, (agents_1m / 60.0) / rate);
}
