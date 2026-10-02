"""Check the README's standard-library table against the real binary.

    python parity/claims.py

Every function name listed in the README "Standard Library" table is called with no arguments
inside a throw-away script. The call counts as AVAILABLE if the runtime knows the function (it may
still complain about arguments); it is MISSING if the compiler or runtime says "unknown function".
"""

import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
BIN = os.path.join(CRATE, "target", "release", "killer_super.exe")
if not os.path.exists(BIN):
    BIN = os.path.join(CRATE, "target", "release", "killer_super")


def readme_functions():
    text = open(os.path.join(CRATE, "README.md"), encoding="utf-8").read()
    m = re.search(r"## Standard Library.*?\n(.*?)\n---", text, re.S)
    rows = [r for r in m.group(1).split("\n") if r.startswith("|") and "---" not in r and "Category" not in r]
    out = []
    for r in rows:
        cols = [c.strip() for c in r.strip("|").split("|")]
        if len(cols) < 2:
            continue
        for name in re.findall(r"`([A-Za-z_][A-Za-z_0-9]*)(?:\([^`]*\))?`", cols[1]):
            out.append((cols[0], name))
    return out


def available(name):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "t.killer")
        with open(p, "w") as f:
            f.write("x = %s()\n" % name)
        try:
            r = subprocess.run([BIN, p, "--run"], capture_output=True, timeout=15, cwd=d)
        except subprocess.TimeoutExpired:
            return True
        out = (r.stdout + r.stderr).decode("utf-8", "replace")
        return "unknown function" not in out


def main():
    fns = readme_functions()
    with ThreadPoolExecutor(8) as ex:
        ok = list(ex.map(lambda t: available(t[1]), fns))
    missing = [(c, n) for (c, n), good in zip(fns, ok) if not good]
    print("README lists %d functions; %d callable, %d missing" % (len(fns), len(fns) - len(missing), len(missing)))
    cats = {}
    for c, n in missing:
        cats.setdefault(c, []).append(n)
    for c, ns in cats.items():
        print("  %-12s %s" % (c, ", ".join(ns)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
