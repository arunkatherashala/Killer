#include <iostream>
#include <vector>
#include <string>

double fib(double n) {
    if (n <= 1.0) {
        return n;
    }
    return fib(n - 1.0) + fib(n - 2.0);
}

int main() {
    std::cout << "Starting Killer Speed Test...\n";
    std::cout << "================================\n";
    
    // Test 1: Loop (100,000 iterations)
    std::cout << "Test 1: Loop (100,000 iterations)\n";
    double count = 0.0;
    while (count < 100000.0) {
        count += 1.0;
    }
    std::cout << "Result: Complete\n";
    
    // Test 2: Arithmetic (50,000 ops)
    std::cout << "Test 2: Arithmetic (50,000 ops)\n";
    double result = 0.0;
    double i = 0.0;
    while (i < 50000.0) {
        result = ((result + i) * 2.0) / 2.0;
        i += 1.0;
    }
    std::cout << "Result: " << (long long)result << "\n";
    
    // Test 3: Array (10,000 elements)
    std::cout << "Test 3: Array (10,000 elements)\n";
    std::vector<double> arr;
    double j = 0.0;
    while (j < 10000.0) {
        arr.push_back(j);
        j += 1.0;
    }
    std::cout << "Result: Array length = " << arr.size() << "\n";
    
    // Test 4: Recursion (fibonacci(20))
    std::cout << "Test 4: Recursion (fibonacci(20))\n";
    double fib_val = fib(20.0);
    std::cout << "Result: " << (long long)fib_val << "\n";
    
    // Test 5: String Concatenation
    std::cout << "Test 5: String Concatenation\n";
    std::string s = "";
    double k = 0.0;
    while (k < 1000.0) {
        s += 'a';
        k += 1.0;
    }
    std::cout << "Result: String length = " << s.length() << "\n";
    
    // Test 6: Nested Loops
    std::cout << "Test 6: Nested Loops\n";
    double sum_val = 0.0;
    double x = 0.0;
    while (x < 100.0) {
        double y = 0.0;
        while (y < 100.0) {
            sum_val += 1.0;
            y += 1.0;
        }
        x += 1.0;
    }
    std::cout << "Result: Total iterations = " << (long long)sum_val << "\n";
    
    std::cout << "================================\n";
    std::cout << "Speed Test Complete!\n";
    
    return 0;
}
