#!/usr/bin/env python3
"""Generate tests/data/modulo_cases.txt for tests/modulo.rs.

Deterministic pseudo-random signed integer and float cases; expected values come from Python's own
`%` (floor modulo) and from `math.fmod` / truncation for `rem`. Re-run:
    python tests/data/gen_modulo_cases.py
Lines:
    I a b mod rem        integer case (all exact in f64)
    F a b mod rem        float case (Python repr, round-trips)
    B a b mod rem        big integer case (beyond 2^53)
    L name value         loop checksum (tests/modulo.rs knows the loop bodies)
"""
import math
import os
import random

random.seed(20261017)
here = os.path.dirname(os.path.abspath(__file__))
out = []


def trunc_rem_int(a, b):
    r = abs(a) % abs(b)
    return -r if a < 0 else r


def nz(f):
    while True:
        v = f()
        if v != 0:
            return v


# 320 integer cases
ints = []
for lo, hi, n in [(-20, 20, 100), (-1000000, 1000000, 100), (-(2 ** 52), 2 ** 52, 60)]:
    for _ in range(n):
        ints.append((random.randint(lo, hi), nz(lambda: random.randint(lo // 3 if lo < -50 else -9, hi // 3 if hi > 50 else 9))))
ints += [(-7, 3), (7, -3), (-7, -3), (7, 3), (0, 5), (0, -5), (-6, 3), (6, -3), (-1, 1000), (1, -1000),
         (2 ** 53 - 1, -7), (-(2 ** 53 - 1), 7), (-5, 5), (5, -5), (-1, -1), (-2 ** 31, 10), (2 ** 31, -10)]
for a, b in ints:
    out.append(f"I {a} {b} {a % b} {trunc_rem_int(a, b)}")

# 260 float cases
floats = []
for _ in range(100):
    floats.append((round(random.uniform(-100, 100), 3), nz(lambda: round(random.uniform(-10, 10), 3))))
for _ in range(100):
    floats.append((random.uniform(-1e6, 1e6), nz(lambda: random.uniform(-1e3, 1e3))))
for _ in range(40):
    floats.append((random.choice([-1, 1]) * random.randint(0, 40) * 0.5, random.choice([-1, 1]) * random.choice([0.5, 1.5, 2.0, 0.25])))
floats += [(5.5, 2.0), (-5.5, 2.0), (5.5, -2.0), (-5.5, -2.0), (0.3, 0.1), (-0.3, 0.1), (1e15, 7.5), (-1e15, 7.5),
           (1e-9, 3.0), (-1e-9, 3.0), (1e-9, -3.0), (-6.0, 3.0), (6.0, -3.0), (-0.0, 5.0), (0.0, -5.0), (7.0, 0.1)]
for a, b in floats:
    out.append(f"F {a!r} {b!r} {a % b + 0.0!r} {math.fmod(a, b) + 0.0!r}")

# 40 bigint cases
for _ in range(40):
    d = random.choice([20, 30, 60, 100])
    a = random.randint(10 ** (d - 1), 10 ** d)
    b = random.randint(10 ** (d // 2), 10 ** (d // 2 + 3))
    a *= random.choice([-1, 1])
    b *= random.choice([-1, 1])
    out.append(f"B {a} {b} {a % b} {trunc_rem_int(a, b)}")

# loop checksums (bodies in tests/modulo.rs)
out.append(f"L range7 {sum(i % 7 for i in range(-5000, 5000))}")
out.append(f"L while7 {sum(i % 7 for i in range(-5000, 5000))}")
out.append(f"L whileneg7 {sum(i % -7 for i in range(-5000, 5000))}")
out.append(f"L float {sum((i * 0.5) % 2.5 for i in range(-6000, 6000))!r}")
out.append(f"L fnloop {sum(((i - 2500) % 11) * (i % 3 + 1) for i in range(5000))}")
out.append(f"L fncalls {sum((i - 1000) % 7 + (i - 1000) % -7 for i in range(2000))}")

with open(os.path.join(here, "modulo_cases.txt"), "w", newline="\n") as f:
    f.write("\n".join(out) + "\n")
print(len(out), "cases")
