#!/usr/bin/env python3
"""Run every fenced ```killer example in the docs and compare its output.

    KILLER_BIN=target/debug/killer_super python docs/check_examples.py [file.md ...]

Convention inside a ```killer block:
  * Each line `# expect: TEXT` is one expected line of stdout, in order.
    (A `# expect:` line with nothing after the colon expects an empty line.)
  * `# expect-error: TEXT` expects a non-zero exit and TEXT somewhere in stderr+stdout.
  * A block with neither must simply run and exit 0.
  * ```killer skip  : fenced block is shown in the docs but not executed.
  * `# file: name.killer` on the first line starts a helper file written next to the main
    program; the helper block is not executed itself (see docs/LANGUAGE.md, imports).
Trailing whitespace and CRLF are ignored. Exit status is the number of failing blocks (max 1).
"""

import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)


def default_bin():
    for mode in ("debug", "release"):
        for name in ("killer_super.exe", "killer_super"):
            p = os.path.join(CRATE, "target", mode, name)
            if os.path.exists(p):
                return p
    return "killer_super"


BIN = os.environ.get("KILLER_BIN") or default_bin()
if os.path.exists(BIN):
    BIN = os.path.abspath(BIN)  # examples run in a temp dir
FENCE = re.compile(r"^```killer(?:\s+(\w+))?\s*$")


def blocks(path):
    """Yield (start_line, flag, source) for each killer block."""
    with open(path, encoding="utf-8") as f:
        lines = f.read().replace("\r\n", "\n").split("\n")
    i = 0
    while i < len(lines):
        m = FENCE.match(lines[i])
        if m:
            start = i + 1
            body = []
            i += 1
            while i < len(lines) and not lines[i].startswith("```"):
                body.append(lines[i])
                i += 1
            yield start, m.group(1), "\n".join(body) + "\n"
        i += 1


def norm(s):
    return "\n".join(l.rstrip() for l in s.replace("\r\n", "\n").strip().split("\n"))


def check_file(path):
    fails = 0
    total = 0
    pending = {}  # helper files collected from `# file:` blocks, per document
    items = list(blocks(path))
    for line, flag, src in items:
        first = src.split("\n", 1)[0]
        m = re.match(r"#\s*file:\s*(\S+)", first)
        if m:
            pending[m.group(1)] = src
    for line, flag, src in items:
        first = src.split("\n", 1)[0]
        if re.match(r"#\s*file:\s*(\S+)", first):
            continue
        if flag == "skip":
            continue
        total += 1
        expect = []
        expect_err = None
        for l in src.split("\n"):
            m = re.match(r"\s*#\s*expect:(?: ?)(.*)$", l)
            if m:
                expect.append(m.group(1))
            m = re.match(r"\s*#\s*expect-error:\s*(.*)$", l)
            if m:
                expect_err = m.group(1).strip()
        with tempfile.TemporaryDirectory() as d:
            for name, text in pending.items():
                with open(os.path.join(d, name), "w", encoding="utf-8", newline="\n") as f:
                    f.write(text)
            main = os.path.join(d, "main.killer")
            with open(main, "w", encoding="utf-8", newline="\n") as f:
                f.write(src)
            try:
                r = subprocess.run([BIN, main, "--run"], cwd=d, capture_output=True, timeout=60)
            except subprocess.TimeoutExpired:
                print("FAIL %s:%d timeout" % (path, line))
                fails += 1
                continue
            out = r.stdout.decode("utf-8", "replace")
            err = r.stderr.decode("utf-8", "replace")
        if expect_err is not None:
            ok = (r.returncode != 0 or "ERROR" in err or "ERROR" in out) and expect_err in (err + out)
            if not ok:
                print("FAIL %s:%d expected error containing %r\n  rc=%d out=%r err=%r" % (path, line, expect_err, r.returncode, out[-200:], err[-200:]))
                fails += 1
            continue
        bad = r.returncode != 0 or "ERROR" in err
        want = "\n".join(expect)
        if bad or (expect and norm(out) != norm(want)):
            print("FAIL %s:%d rc=%d" % (path, line, r.returncode))
            if expect:
                print("  expected: %r" % norm(want))
            print("  got:      %r" % norm(out))
            if err.strip():
                print("  stderr:   %r" % err.strip()[:300])
            fails += 1
    return total, fails


def main():
    files = sys.argv[1:] or [os.path.join(HERE, "LANGUAGE.md"), os.path.join(HERE, "TUTORIAL.md")]
    total = fails = 0
    for p in files:
        t, f = check_file(p)
        print("%s: %d examples, %d failed" % (os.path.relpath(p, CRATE), t, f))
        total += t
        fails += f
    print("TOTAL: %d examples, %d failed (bin: %s)" % (total, fails, BIN))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
