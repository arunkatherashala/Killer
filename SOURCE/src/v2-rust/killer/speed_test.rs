// Pure Rust Speed Test - Core Rust Performance
// This is a baseline to compare Killer V1/V2 against native Rust

use std::time::Instant;

fn main() {
    println!("Starting Pure Rust Speed Test...");
    println!("================================");
    
    let start = Instant::now();
    
    // Test 1: Loop (100K iterations)
    println!("Test 1: Loop (100,000 iterations)");
    let mut count = 0;
    while count < 100_000 {
        count += 1;
    }
    println!("Result: Complete");
    
    // Test 2: Arithmetic (50K operations)
    println!("Test 2: Arithmetic (50,000 ops)");
    let mut result = 0.0;
    let mut i = 0;
    while i < 50_000 {
        result = ((result + i as f64) * 2.0) / 2.0;
        i += 1;
    }
    println!("Result: {}", result);
    
    // Test 3: Vector/Array (10K elements)
    println!("Test 3: Array (10,000 elements)");
    let mut arr = Vec::new();
    let mut j = 0;
    while j < 10_000 {
        arr.push(j);
        j += 1;
    }
    println!("Result: Array length = {}", arr.len());
    
    // Test 4: Recursion (fibonacci)
    println!("Test 4: Recursion (fibonacci(20))");
    fn fib(n: u32) -> u64 {
        if n <= 1 {
            n as u64
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }
    let fib_val = fib(20);
    println!("Result: {}", fib_val);
    
    // Test 5: String Concatenation
    println!("Test 5: String Concatenation");
    let mut s = String::new();
    let mut k = 0;
    while k < 1_000 {
        s.push('a');
        k += 1;
    }
    println!("Result: String length = {}", s.len());
    
    // Test 6: Nested Loops
    println!("Test 6: Nested Loops");
    let mut sum_val = 0;
    let mut x = 0;
    while x < 100 {
        let mut y = 0;
        while y < 100 {
            sum_val += 1;
            y += 1;
        }
        x += 1;
    }
    println!("Result: Total iterations = {}", sum_val);
    
    let elapsed = start.elapsed();
    println!("================================");
    println!("Speed Test Complete!");
    println!("⏱️  Pure Rust Execution Time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
}
