"""Per-file inventory of the Killer crate: is each source file compiled, tested, or a stub?

    python parity/inventory.py     # writes parity/FILE_INVENTORY.csv and prints a summary

For every .rs file under src/ it records:
  lines       - physical lines
  status      - COMPILED  (reachable through `mod` declarations from lib.rs or a bin entry point)
                DEAD      (never declared as a module anywhere: the compiler never sees it)
                BIN       (src/bin/*.rs, each is its own binary)
  tests       - number of #[test] functions inside the file
  stub_marks  - occurrences of simulation/placeholder markers (simulated, placeholder, TODO,
                unimplemented!, todo!, "not yet implemented", stub, mock). A high count relative to
                size suggests the file fakes its behaviour.
"""

import csv
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
SRC = os.path.join(CRATE, "src")

STUB = re.compile(r"simulat|placeholder|\bTODO\b|unimplemented!|todo!|not yet implemented|\bstub\b|\bmock", re.I)
TEST = re.compile(r"#\[test\]")
MOD = re.compile(r"^\s*(?:pub(?:\([a-z]+\))?\s+)?mod\s+(\w+)\s*;", re.M)
PATHMOD = re.compile(r'#\[path\s*=\s*"([^"]+)"\s*\]\s*(?:pub\s+)?mod\s+(\w+)\s*;')


def read(p):
    with open(p, encoding="utf-8", errors="ignore") as f:
        return f.read()


def declared_children(path):
    """Files directly pulled in by the module file at `path` (handles foo.rs/foo/ and mod.rs)."""
    text = read(path)
    base = os.path.dirname(path)
    stem = os.path.splitext(os.path.basename(path))[0]
    child_dir = base if os.path.basename(path) in ("mod.rs", "lib.rs", "main.rs") else os.path.join(base, stem)
    out = []
    consumed = set()
    for rel, name in PATHMOD.findall(text):
        out.append(os.path.normpath(os.path.join(base, rel)))
        consumed.add(name)
    for name in MOD.findall(text):
        if name in consumed:
            continue
        for cand in (os.path.join(child_dir, name + ".rs"), os.path.join(child_dir, name, "mod.rs")):
            if os.path.exists(cand):
                out.append(os.path.normpath(cand))
                break
    return out


def compiled_set():
    seen = set()
    stack = [os.path.join(SRC, "lib.rs")]
    # binaries are separate roots; their own `mod` declarations count too
    bin_dir = os.path.join(SRC, "bin")
    if os.path.isdir(bin_dir):
        stack += [os.path.join(bin_dir, f) for f in os.listdir(bin_dir) if f.endswith(".rs")]
    # inline `#[path]` includes from any compiled file are followed through declared_children
    while stack:
        p = os.path.normpath(stack.pop())
        if p in seen or not os.path.exists(p):
            continue
        seen.add(p)
        stack.extend(declared_children(p))
    return seen


def main():
    compiled = compiled_set()
    rows = []
    for root, _dirs, files in os.walk(SRC):
        for f in files:
            if not f.endswith(".rs"):
                continue
            p = os.path.normpath(os.path.join(root, f))
            text = read(p)
            rel = os.path.relpath(p, CRATE).replace(os.sep, "/")
            in_bin = rel.startswith("src/bin/")
            status = "BIN" if in_bin else ("COMPILED" if p in compiled else "DEAD")
            rows.append({
                "path": rel,
                "lines": text.count("\n") + 1,
                "status": status,
                "tests": len(TEST.findall(text)),
                "stub_marks": len(STUB.findall(text)),
            })
    rows.sort(key=lambda r: (r["status"], -r["lines"]))
    with open(os.path.join(HERE, "FILE_INVENTORY.csv"), "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=["path", "lines", "status", "tests", "stub_marks"])
        w.writeheader()
        w.writerows(rows)

    def agg(status):
        rs = [r for r in rows if r["status"] == status]
        return len(rs), sum(r["lines"] for r in rs), sum(r["tests"] for r in rs)

    print("source files under src/: %d  (%d lines)" % (len(rows), sum(r["lines"] for r in rows)))
    for s in ("COMPILED", "BIN", "DEAD"):
        n, l, t = agg(s)
        print("  %-9s %4d files  %7d lines  %5d tests" % (s, n, l, t))
    comp = [r for r in rows if r["status"] == "COMPILED"]
    untested = [r for r in comp if r["tests"] == 0 and r["lines"] >= 200]
    print("\ncompiled files >= 200 lines with NO inline tests: %d (%d lines)" % (len(untested), sum(r["lines"] for r in untested)))
    suspicious = sorted([r for r in comp if r["stub_marks"] >= 5], key=lambda r: -r["stub_marks"])[:15]
    print("\ncompiled files with the most simulation/placeholder markers:")
    for r in suspicious:
        print("  %4d marks  %5d lines  %s" % (r["stub_marks"], r["lines"], r["path"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
