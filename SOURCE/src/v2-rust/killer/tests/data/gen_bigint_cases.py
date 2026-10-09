import sys; sys.set_int_max_str_digits(0)
#!/usr/bin/env python3
"""Generate tests/data/bigint_program.killer and tests/data/bigint_expected.txt.

The Killer program evaluates deterministic pseudo-random bigint cases; the expected output is
computed here with Python's own integers (floor division / modulo semantics are identical).
Re-run:  python tests/data/gen_bigint_cases.py
"""
import math
import os
import random

random.seed(20260508)
here = os.path.dirname(os.path.abspath(__file__))


def rand_int(digits):
    n = random.randint(10 ** (digits - 1), 10 ** digits - 1) if digits > 1 else random.randint(0, 9)
    return -n if random.random() < 0.4 else n


def structured():
    """Values near word and power boundaries, where carries and quotient estimates are fragile."""
    k = random.choice([31, 32, 33, 63, 64, 65, 96, 128, 192, 256, 1024])
    base = random.choice([2 ** k, 10 ** (k // 3 + 1), 2 ** k - 1, 2 ** (32 * (k // 32 + 1)) - 1])
    v = base + random.choice([-2, -1, 0, 1, 2])
    return -v if random.random() < 0.3 else v


sizes = [1, 2, 5, 9, 10, 19, 20, 30, 60, 100, 200, 400, 700]
program = []
expected = []


def emit(expr, value):
    program.append("println(%s)" % expr)
    if isinstance(value, bool):
        expected.append("true" if value else "false")
    elif isinstance(value, list):
        expected.append("[" + ", ".join(str(v) for v in value) + "]")
    else:
        expected.append(str(value))


def lit(v):
    return 'big("%d")' % v


case_no = 0
for i in range(220):
    if i % 5 == 0:
        a, b = structured(), structured()
    elif i % 5 == 1:
        # build a divisible / near-divisible pair so exact division and remainders are exercised
        b = rand_int(random.choice(sizes[:11]))
        q = rand_int(random.choice(sizes[:11]))
        r = random.choice([0, 0, 1, -1, 7])
        a = b * q + r
    else:
        a = rand_int(random.choice(sizes))
        b = rand_int(random.choice(sizes))
    if b == 0:
        b = 3
    program.append("a = %s" % lit(a))
    program.append("b = %s" % lit(b))
    emit("a + b", a + b)
    emit("a - b", a - b)
    emit("a * b", a * b)
    emit("idiv(a, b)", a // b)
    emit("a % b", a % b)
    emit("divmod(a, b)", [a // b, a % b])
    emit("a < b", a < b)
    emit("a == b", a == b)
    emit("a >= b", a >= b)
    emit("gcd(a, b)", math.gcd(a, b))
    emit("-a", -a)
    if i % 4 == 0:
        emit("isqrt(abs(a))", math.isqrt(abs(a)))
    if i % 6 == 0:
        e = random.randint(0, 25)
        small = rand_int(random.choice([1, 3, 10, 30]))
        program.append("s = %s" % lit(small))
        emit("s ** %d" % e, small ** e)
    if i % 8 == 0:
        m = abs(rand_int(random.choice([2, 5, 10, 30]))) + 2
        e = abs(rand_int(random.choice([1, 3, 10, 20])))
        base = rand_int(random.choice([1, 5, 20, 40]))
        emit("powmod(%s, %s, %s)" % (lit(base), lit(e), lit(m)), pow(base, e, m))
    if i % 7 == 0:
        emit("str(a) == \"%d\"" % a, True)

# fixed spot checks that are cheap to state exactly
emit("factorial(50)", math.factorial(50))
emit("factorial(100)", math.factorial(100))
emit("2n ** 128", 2 ** 128)
emit("big(2) ** 521 - 1", 2 ** 521 - 1)
emit("idiv(0n - 7n, 2n)", -7 // 2)
emit("(0n - 7n) % 3n", -7 % 3)
emit("7n % (0n - 3n)", 7 % -3)
emit("0xFFFFFFFFFFFFFFFFFFn", 0xFFFFFFFFFFFFFFFFFF)
emit("0xDEADBEEFDEADBEEFn * 0x10n", 0xDEADBEEFDEADBEEF * 16)
emit("123456789012345678901234567890n * 987654321098765432109876543210n", 123456789012345678901234567890 * 987654321098765432109876543210)

fa, fb = 0, 1
for _ in range(500):
    fa, fb = fb, fa + fb
program.append("fa = 0n")
program.append("fb = 1n")
program.append("for i in range(500) {")
program.append("  t = fa + fb")
program.append("  fa = fb")
program.append("  fb = t")
program.append("}")
emit("fa", fa)

# RSA toy
p, q, e = 1000000007, 998244353, 65537
n = p * q
phi = (p - 1) * (q - 1)
d = pow(e, -1, phi)
msg = 123456789012345
c = pow(msg, e, n)
program.append("n = %dn" % n)
emit("powmod(%d, %d, n)" % (msg, e), c)
emit("powmod(%dn, %dn, n)" % (c, d), msg)

with open(os.path.join(here, "bigint_program.killer"), "w", newline="\n") as f:
    f.write("\n".join(program) + "\n")
with open(os.path.join(here, "bigint_expected.txt"), "w", newline="\n") as f:
    f.write("\n".join(expected) + "\n")
print(len(expected), "expected lines;", len(program), "program lines")
