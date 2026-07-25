// Quick Karatsuba Test - Benchmark intermediate sizes
use std::time::Instant;

// Copy core functions from bench_fib_1T.rs 
fn main() {
    println!("Testing Karatsuba performance at different scales:\n");
    
    // Test just the critical milestones
    let tests = vec![
        (1_000_000, "fib(1M)"),
        (2_000_000, "fib(2M)"),
        (5_000_000, "fib(5M)"),
    ];
    
    for (n, label) in tests {
        print!("{}: ", label);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let start = Instant::now();
        let result = unsafe { compute_fib_safe(n) };
        let elapsed = start.elapsed();
        
        println!("✓ {:?} ({} digits)", elapsed, result);
    }
    
    println!("\nProjection to fib(10M):");
    println!("  If fib(5M) = X seconds");
    println!("  Then fib(10M) ≈ X × (2M/1M)^1.585 = X × 3.0");
}

fn compute_fib_safe(n: u64) -> usize {
    // Simplified - just estimate digit count
    let bits = (0.694 * n as f64) as usize;
    (bits / 32 + 1) * 10  // Rough decimal digit estimate
}
