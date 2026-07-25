use std::time::Instant;

fn fibonacci(n: i32) -> i64 {
    if n <= 1 {
        n as i64
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() {
    let start = Instant::now();

    // Test 1: Loop (100,000 iterations)
    let mut count = 0;
    for _ in 0..100000 {
        count += 1;
    }

    // Test 2: Arithmetic (50,000 operations)
    let mut sum = 0i64;
    for i in 0..50000 {
        sum += i as i64;
    }

    // Test 3: Array (10,000 elements)
    let mut arr = Vec::new();
    for i in 0..10000 {
        arr.push(i);
    }

    // Test 4: Recursion (fibonacci(20))
    let fib_result = fibonacci(20);

    // Test 5: String Concatenation (1,000 times)
    let mut s = String::new();
    for _ in 0..1000 {
        s.push('a');
    }

    // Test 6: Nested Loops (100x100)
    let mut nested_count = 0;
    for _ in 0..100 {
        for _ in 0..100 {
            nested_count += 1;
        }
    }

    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();

    println!("Rust Benchmark Results:");
    println!("Test 1: Loop count = {}", count);
    println!("Test 2: Arithmetic sum = {}", sum);
    println!("Test 3: Array size = {}", arr.len());
    println!("Test 4: Fibonacci(20) = {}", fib_result);
    println!("Test 5: String length = {}", s.len());
    println!("Test 6: Nested loops = {}", nested_count);
    println!("\nTotal Execution Time: {} ms", ms);
}
