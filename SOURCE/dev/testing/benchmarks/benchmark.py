import time

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

start = time.time()

# Test 1: Loop (100,000 iterations)
count = 0
for _ in range(100000):
    count += 1

# Test 2: Arithmetic (50,000 operations)
sum_val = 0
for i in range(50000):
    sum_val += i

# Test 3: Array (10,000 elements)
arr = []
for i in range(10000):
    arr.append(i)

# Test 4: Recursion (fibonacci(20))
fib_result = fibonacci(20)

# Test 5: String Concatenation (1,000 times)
s = ""
for _ in range(1000):
    s += "a"

# Test 6: Nested Loops (100x100)
nested_count = 0
for _ in range(100):
    for _ in range(100):
        nested_count += 1

elapsed = time.time() - start

print("Python Benchmark Results:")
print(f"Test 1: Loop count = {count}")
print(f"Test 2: Arithmetic sum = {sum_val}")
print(f"Test 3: Array size = {len(arr)}")
print(f"Test 4: Fibonacci(20) = {fib_result}")
print(f"Test 5: String length = {len(s)}")
print(f"Test 6: Nested loops = {nested_count}")
print(f"\nTotal Execution Time: {elapsed * 1000:.1f} ms")
