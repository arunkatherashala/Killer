#include <stdio.h>
#include <string.h>

double fib(double n) {
    if (n <= 1.0) {
        return n;
    }
    return fib(n - 1.0) + fib(n - 2.0);
}

int main() {
    printf("Starting Killer Speed Test...\n");
    printf("================================\n");
    
    // Test 1: Loop (100,000 iterations)
    printf("Test 1: Loop (100,000 iterations)\n");
    double count = 0.0;
    while (count < 100000.0) {
        count += 1.0;
    }
    printf("Result: Complete\n");
    
    // Test 2: Arithmetic (50,000 ops)
    printf("Test 2: Arithmetic (50,000 ops)\n");
    double result = 0.0;
    double i = 0.0;
    while (i < 50000.0) {
        result = ((result + i) * 2.0) / 2.0;
        i += 1.0;
    }
    printf("Result: %lld\n", (long long)result);
    
    // Test 3: Array (10,000 elements)
    printf("Test 3: Array (10,000 elements)\n");
    double arr[10000];
    double j = 0.0;
    int idx = 0;
    while (j < 10000.0) {
        arr[idx++] = j;
        j += 1.0;
    }
    printf("Result: Array length = %d\n", idx);
    
    // Test 4: Recursion (fibonacci(20))
    printf("Test 4: Recursion (fibonacci(20))\n");
    double fib_val = fib(20.0);
    printf("Result: %lld\n", (long long)fib_val);
    
    // Test 5: String Concatenation
    printf("Test 5: String Concatenation\n");
    char s[1001] = "";
    double k = 0.0;
    int slen = 0;
    while (k < 1000.0) {
        s[slen++] = 'a';
        k += 1.0;
    }
    s[slen] = '\0';
    printf("Result: String length = %d\n", slen);
    
    // Test 6: Nested Loops
    printf("Test 6: Nested Loops\n");
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
    printf("Result: Total iterations = %lld\n", (long long)sum_val);
    
    printf("================================\n");
    printf("Speed Test Complete!\n");
    
    return 0;
}
