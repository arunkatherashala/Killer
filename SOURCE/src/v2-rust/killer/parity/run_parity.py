"""Run the language-parity probes against the real `killer_super` binary.

    python parity/run_parity.py            # prints a summary, writes parity/results.json
    python parity/run_parity.py --md       # also writes parity/LANGUAGE_PARITY.md

A probe PASSES only if stdout matches the expected text exactly. Expected "ERROR" means the
program must fail (parse/runtime error). Nothing here is mocked: every probe is executed.
"""

import json
import os
import subprocess
import sys
import tempfile
from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from probes import P  # noqa: E402

CRATE = os.path.dirname(HERE)
BIN = os.path.join(CRATE, "target", "release", "killer_super.exe")
if not os.path.exists(BIN):
    BIN = os.path.join(CRATE, "target", "release", "killer_super")
BIN = os.environ.get("KILLER_BIN", BIN)  # e.g. target/debug/killer_super.exe for quick checks


def norm(s):
    return "\n".join(line.rstrip() for line in s.replace("\r\n", "\n").strip().split("\n"))


def run_one(item):
    category, name, code, expected, files = item
    with tempfile.TemporaryDirectory() as d:
        for fname, content in files.items():
            with open(os.path.join(d, fname), "w", encoding="utf-8") as f:
                f.write(content)
        main = os.path.join(d, "main.killer")
        with open(main, "w", encoding="utf-8") as f:
            f.write(code + "\n")
        try:
            r = subprocess.run([BIN, main, "--run"], cwd=d, capture_output=True, timeout=25)
            out = r.stdout.decode("utf-8", "replace")
            err = r.stderr.decode("utf-8", "replace")
            rc = r.returncode
        except subprocess.TimeoutExpired:
            out, err, rc = "", "TIMEOUT", -1
    combined = norm(out + "\n" + err)
    got = norm(out)
    failed = ("ERROR" in combined) or rc != 0
    if expected == "ERROR":
        status = "PASS" if failed else "FAIL"
        detail = "" if failed else "expected an error but it ran: " + got[:60]
    elif norm(expected) == got and not failed:
        status, detail = "PASS", ""
    elif failed:
        status = "ERROR"
        first = [ln for ln in combined.split("\n") if "ERROR" in ln or "TIMEOUT" in ln]
        detail = (first[0] if first else combined)[:150]
    else:
        status = "WRONG"
        detail = "got %r, expected %r" % (got[:70], norm(expected)[:70])
    return {"category": category, "name": name, "status": status, "detail": detail}


def main():
    if not os.path.exists(BIN):
        sys.exit("build first: cargo build --release --bin killer_super")
    with ThreadPoolExecutor(max_workers=8) as ex:
        results = list(ex.map(run_one, P))
    with open(os.path.join(HERE, "results.json"), "w", encoding="utf-8") as f:
        json.dump(results, f, indent=1, ensure_ascii=False)

    by_cat = OrderedDict()
    for r in results:
        by_cat.setdefault(r["category"], []).append(r)
    total = len(results)
    passed = sum(1 for r in results if r["status"] == "PASS")
    print("LANGUAGE PARITY: %d/%d probes pass (%.0f%%)\n" % (passed, total, 100.0 * passed / total))
    for cat, rs in by_cat.items():
        p = sum(1 for r in rs if r["status"] == "PASS")
        print("  %-16s %3d/%-3d" % (cat, p, len(rs)))
    if "--md" in sys.argv:
        write_md(results, by_cat, passed, total)
    return 0


def write_md(results, by_cat, passed, total):
    lines = []
    w = lines.append
    w("# Killer language parity (generated)\n")
    w("Every probe below was executed against the real `killer_super` binary. A probe passes only if the")
    w("output matches what the equivalent Python/JS/Java program prints. Regenerate with")
    w("`python parity/run_parity.py --md`. Source of truth: `parity/probes.py`.\n")
    w("**Overall: %d of %d probes pass (%.0f%%).**\n" % (passed, total, 100.0 * passed / total))
    w("| Area | Pass | Total |")
    w("|------|-----:|------:|")
    for cat, rs in by_cat.items():
        w("| %s | %d | %d |" % (cat, sum(1 for r in rs if r["status"] == "PASS"), len(rs)))
    w("\nStatus legend: PASS = identical output; WRONG = ran but printed something different; "
      "ERROR = the program failed to parse or run.\n")
    for cat, rs in by_cat.items():
        w("## %s\n" % cat)
        w("| Probe | Status | Detail |")
        w("|-------|--------|--------|")
        for r in rs:
            d = r["detail"].replace("|", "\\|")
            w("| %s | %s | %s |" % (r["name"], r["status"], d))
        w("")
    with open(os.path.join(HERE, "LANGUAGE_PARITY.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print("\nwrote parity/LANGUAGE_PARITY.md")


if __name__ == "__main__":
    sys.exit(main())
