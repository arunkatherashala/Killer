// Pure Rust implementation matching Killer's f64 type system

fn main() {
    println!("Starting Killer Speed Test...");
    println!("================================");
    
    // Test 1: Loop (100,000 iterations)
    println!("Test 1: Loop (100,000 iterations)");
    let mut count = 0.0f64;
    while count < 100000.0 {
        count += 1.0;
    }
    println!("Result: Complete");
    
    // Test 2: Arithmetic (50,000 ops)
    println!("Test 2: Arithmetic (50,000 ops)");
    let mut result = 0.0f64;
    let mut i = 0.0f64;
    while i < 50000.0 {
        result = ((result + i) * 2.0) / 2.0;
        i += 1.0;
    }
    println!("Result: {}", if result.fract() == 0.0 { result as i64 } else { result as i64 });
    
    // Test 3: Array (10,000 elements)
    println!("Test 3: Array (10,000 elements)");
    let mut arr: Vec<f64> = Vec::new();
    let mut j = 0.0f64;
    while j < 10000.0 {
        arr.push(j);
        j += 1.0;
    }
    println!("Result: Array length = {}", arr.len());
    
    // Test 4: Recursion (fibonacci(20))
    println!("Test 4: Recursion (fibonacci(20))");
    let fib_val = fib(20.0);
    println!("Result: {}", fib_val as i64);
    
    // Test 5: String Concatenation
    println!("Test 5: String Concatenation");
    let mut s = String::new();
    let mut k = 0.0f64;
    while k < 1000.0 {
        s.push('a');
        k += 1.0;
    }
    println!("Result: String length = {}", s.len());
    
    // Test 6: Nested Loops
    println!("Test 6: Nested Loops");
    let mut sum_val = 0.0f64;
    let mut x = 0.0f64;
    while x < 100.0 {
        let mut y = 0.0f64;
        while y < 100.0 {
            sum_val += 1.0;
            y += 1.0;
        }
        x += 1.0;
    }
    println!("Result: Total iterations = {}", sum_val as i64);
    
    println!("================================");
    println!("Speed Test Complete!");
}

fn fib(n: f64) -> f64 {
    if n <= 1.0 {
        return n;
    }
    fib(n - 1.0) + fib(n - 2.0)
}
