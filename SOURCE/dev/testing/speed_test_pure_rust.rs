// Pure Rust implementation of the speed test
// This is the baseline for maximum performance

fn main() {
    println!("Starting Killer Speed Test...");
    println!("================================");
    
    // Test 1: Loop (100,000 iterations)
    println!("Test 1: Loop (100,000 iterations)");
    let mut count = 0u64;
    while count < 100000 {
        count += 1;
    }
    println!("Result: Complete");
    
    // Test 2: Arithmetic (50,000 ops)
    println!("Test 2: Arithmetic (50,000 ops)");
    let mut result = 0u64;
    let mut i = 0u64;
    while i < 50000 {
        result = ((result + i) * 2) / 2;
        i += 1;
    }
    println!("Result: {}", result);
    
    // Test 3: Array (10,000 elements)
    println!("Test 3: Array (10,000 elements)");
    let mut arr: Vec<u64> = Vec::new();
    let mut j = 0u64;
    while j < 10000 {
        arr.push(j);
        j += 1;
    }
    println!("Result: Array length = {}", arr.len());
    
    // Test 4: Recursion (fibonacci(20))
    println!("Test 4: Recursion (fibonacci(20))");
    let fib_val = fib(20);
    println!("Result: {}", fib_val);
    
    // Test 5: String Concatenation
    println!("Test 5: String Concatenation");
    let mut s = String::new();
    let mut k = 0u64;
    while k < 1000 {
        s.push('a');
        k += 1;
    }
    println!("Result: String length = {}", s.len());
    
    // Test 6: Nested Loops
    println!("Test 6: Nested Loops");
    let mut sum_val = 0u64;
    let mut x = 0u64;
    while x < 100 {
        let mut y = 0u64;
        while y < 100 {
            sum_val += 1;
            y += 1;
        }
        x += 1;
    }
    println!("Result: Total iterations = {}", sum_val);
    
    println!("================================");
    println!("Speed Test Complete!");
}

fn fib(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}
