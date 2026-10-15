"""Writes date test cases for check_dates.killer: one `text<TAB>0|1` per line, where 1 means a
real calendar date in YYYY-MM-DD form according to Python's datetime.

usage: python dates.py OUT.tsv
"""
import datetime
import random
import sys

random.seed(11)
cases = []
for y in (1, 1900, 1999, 2000, 2023, 2024, 2100, 2400, 9999):
    for m in range(0, 14):
        for d in (0, 1, 28, 29, 30, 31, 32):
            cases.append(f"{y:04d}-{m:02d}-{d:02d}")
for _ in range(3000):
    cases.append(f"{random.randint(1, 9999):04d}-{random.randint(0, 13):02d}-{random.randint(0, 32):02d}")
cases += ["", "2024-1-01", "2024-01-1", "24-01-01", "2024/01/01", "2024-01-01 ", " 2024-01-01", "abcd-ef-gh", "2024-01", "2024-01-01-01", "２０２４-01-01", "2024-02-29T00:00"]


def ok(s):
    if len(s) != 10 or s[4] != "-" or s[7] != "-":
        return 0
    if not all(c in "0123456789" for c in s[:4] + s[5:7] + s[8:]):
        return 0
    try:
        datetime.date(int(s[:4]), int(s[5:7]), int(s[8:]))
        return 1
    except ValueError:
        return 0


with open(sys.argv[1], "w", encoding="utf-8", newline="") as f:
    for c in cases:
        f.write(f"{c}\t{ok(c)}\n")
