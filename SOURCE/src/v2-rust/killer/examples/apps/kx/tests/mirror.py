# Python mirror of programs/numbers.kx: prints the same lines with native Python integers.
import math, sys

def fact(n):
    return math.factorial(n)

def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

def idiv(a, b):
    return a // b  # floor division, like kx

print(fact(50))
print(fact(100))
print(fib(100))
print(2 ** 200)
print(math.gcd(fact(20), 2 ** 70))
print(pow(7, 1000, 1000000007))
print(math.isqrt(10 ** 40 + 12345))
print(idiv(-7, 2))
print(-7 % 3)
print(idiv(7, -2))
print(7 % -2)
print(len(str(fact(100))))
print(2 ** (3 ** 2))
print(-(2 ** 2))
print((1 + 2) * 3 - idiv(4, 2))
print(10 ** 30 + 1 - 10 ** 30)
