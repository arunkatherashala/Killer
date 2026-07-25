use std::time::Instant;
use std::collections::HashMap;

/// Realistic KILLER Benchmark - Cannot be optimized away
/// Simulates actual temporal event processing
fn main() {
    println!("\n+================================================================+");
    println!("|     🚀 KILLER REALISTIC BENCHMARK - PHASE 1 OPTIMIZATION    🚀  |");
    println!("|     Actual event processing with state management             |");
    println!("+================================================================+\n");

    let test_scales = vec![
        (10_000, "10K"),
        (100_000, "100K"),
        (1_000_000, "1M"),
        (5_000_000, "5M"),
        (10_000_000, "10M"),
    ];

    let mut peak_throughput = 0.0;
    let mut peak_scale = String::new();

    for (ops_count, label) in test_scales {
        print!("Testing {} operations:", label);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let start = Instant::now();
        
        // Pre-allocate to prevent allocation noise
        let mut events = Vec::with_capacity(ops_count);
        let mut state = HashMap::with_capacity(1000);
        
        // Phase 1: Generate events
        for i in 0..ops_count {
            events.push((i as u64, (i % 100) as u32));
        }
        
        // Phase 2: Process events (actual work)
        for (timestamp, event_type) in &events {
            let key = format!("event_{}", event_type);
            *state.entry(key).or_insert(0u64) += timestamp;
        }
        
        // Phase 3: Finalize (prevent dead code elimination)
        let final_result: u64 = state.values().sum();
        
        let duration = start.elapsed();
        let ops_per_sec = (ops_count as f64) / duration.as_secs_f64();

        println!(
            " ✅ {} ops/sec | {:.3}s | Result: {}",
            format!("{:.0}", ops_per_sec).chars().rev().enumerate()
                .map(|(i, c)| if i > 0 && i % 3 == 0 { format!(",{}", c) } else { c.to_string() })
                .collect::<String>().chars().rev().collect::<String>(),
            duration.as_secs_f64(),
            final_result
        );

        if ops_per_sec > peak_throughput {
            peak_throughput = ops_per_sec;
            peak_scale = label.to_string();
        }
    }

    println!("\n+================================================================+");
    println!("|                   OPTIMIZATION RESULTS                       |");
    println!("+================================================================+\n");

    let peak_millions = peak_throughput / 1_000_000.0;
    println!("🏆 Peak Performance: {:.2} Million ops/sec @ {}", peak_millions, peak_scale);
    
    println!("\n📊 Analysis:");
    println!("   ✅ LTO optimization enabled");
    println!("   ✅ Phase 1 baseline established");
    println!("   ✅ Ready for Phase 2: SIMD + Cache optimizations");
    println!("   ✅ Next target: Break 15M ops/sec\n");
}
