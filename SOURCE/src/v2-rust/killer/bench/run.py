"""Killer-vs-Python benchmark runner for non-numeric (non-JIT) workloads.

    python bench/run.py                 # all benchmarks, 5 runs each, median wall time
    python bench/run.py -n 3 str_ dict  # only benchmarks whose name contains str_ or dict
    KILLER_BIN=path/to/killer_super.exe python bench/run.py

For every NAME there is bench/NAME.killer and bench/NAME.py. Both must print identical stdout,
otherwise the row is flagged MISMATCH. Wall time includes process start-up for both sides.
"""

import argparse
import os
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)


def default_bin():
    for name in ("killer_super.exe", "killer_super"):
        p = os.path.join(CRATE, "target", "release", name)
        if os.path.exists(p):
            return p
    return "killer_super"


def run(cmd, cwd):
    t = time.perf_counter()
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, timeout=300)
    dt = time.perf_counter() - t
    out = r.stdout.decode("utf-8", "replace").replace("\r\n", "\n").strip()
    return dt, out, r.returncode, r.stderr.decode("utf-8", "replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("filters", nargs="*")
    ap.add_argument("-n", "--runs", type=int, default=5)
    ap.add_argument("--bin", default=os.environ.get("KILLER_BIN") or default_bin())
    args = ap.parse_args()

    names = sorted(f[:-7] for f in os.listdir(HERE) if f.endswith(".killer"))
    if args.filters:
        names = [n for n in names if any(f in n for f in args.filters)]

    rows = []
    for name in names:
        kfile = os.path.join(HERE, name + ".killer")
        pfile = os.path.join(HERE, name + ".py")
        kt, pt = [], []
        kout = pout = None
        status = "ok"
        for _ in range(args.runs):
            dt, out, rc, err = run([args.bin, kfile, "--run"], HERE)
            if rc != 0 or "ERROR" in err:
                status = "KILLER-FAIL: " + (err.strip().splitlines() or ["rc=%d" % rc])[0][:70]
            kt.append(dt)
            kout = out
            dt, out, rc, err = run([sys.executable, pfile], HERE)
            if rc != 0:
                status = "PY-FAIL"
            pt.append(dt)
            pout = out
        if status == "ok" and kout != pout:
            status = "MISMATCH"
        km, pm = statistics.median(kt), statistics.median(pt)
        rows.append((name, km, pm, km / pm, status))
        print("  done %-18s killer %.3fs python %.3fs %s" % (name, km, pm, status), file=sys.stderr)

    print()
    print("| Benchmark | Killer (s) | Python (s) | Killer/Python | Verdict |")
    print("|-----------|-----------:|-----------:|--------------:|---------|")
    for name, km, pm, ratio, status in sorted(rows, key=lambda r: -r[3]):
        if status != "ok":
            verdict = status
        elif ratio < 0.95:
            verdict = "Killer %.1fx faster" % (1 / ratio)
        elif ratio > 1.05:
            verdict = "Killer %.1fx slower" % ratio
        else:
            verdict = "level"
        print("| %s | %.3f | %.3f | %.2f | %s |" % (name, km, pm, ratio, verdict))
    geo = statistics.geometric_mean([r[3] for r in rows]) if rows else 0
    print()
    print("Geometric mean Killer/Python: %.2f (lower is better; >1 means Killer slower)" % geo)
    return 1 if any(r[4] != "ok" for r in rows) else 0


if __name__ == "__main__":
    sys.exit(main())
