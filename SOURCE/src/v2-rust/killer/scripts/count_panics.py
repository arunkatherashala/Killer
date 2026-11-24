#!/usr/bin/env python3
"""Count panic sites (.unwrap(), .expect(, panic!, unreachable!, todo!, unimplemented!)
in NON-test Rust code.

Comments, string/char literals, #[cfg(test)] / #[test] items, tests/, benches/ and
archive/ are ignored.

usage:
  python scripts/count_panics.py                  # per-file table for src/
  python scripts/count_panics.py --json           # machine readable
  python scripts/count_panics.py --sites FILE     # print every site in FILE
  python scripts/count_panics.py --locks          # lock().unwrap() style sites (poison cascades)
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.environ.get("KILLER_AUDIT_SRC") or os.path.join(ROOT, "src")
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

PATTERNS = {
    "unwrap": re.compile(r"\.unwrap\(\)"),
    "expect": re.compile(r"\.expect\("),
    "panic": re.compile(r"\bpanic!"),
    "unreachable": re.compile(r"\bunreachable!"),
    "todo": re.compile(r"\btodo!"),
    "unimplemented": re.compile(r"\bunimplemented!"),
}
LOCK_RE = re.compile(r"\.(lock|read|write)\(\)\s*\.unwrap\(\)")


def sanitize(src):
    """Blank out comments and string/char literals, keeping newlines and offsets."""
    out = []
    i = 0
    n = len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif c == "/" and nxt == "*":
            depth = 1
            j = i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append("".join(ch if ch in ("\r", "\n") else " " for ch in src[i:j]))
            i = j
        elif c == "r" and re.match(r'r#*"', src[i:i + 12]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            m = re.match(r'r(#*)"', src[i:])
            hashes = m.group(1)
            end = src.find('"' + hashes, i + len(m.group(0)))
            j = n if end < 0 else end + 1 + len(hashes)
            out.append("".join(ch if ch in ("\r", "\n") else " " for ch in src[i:j]))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            out.append("".join(ch if ch in ("\r", "\n") else " " for ch in src[i:j]))
            i = j
        elif c == "'":
            m = re.match(r"'(\\.[^']*|[^\\'])'", src[i:i + 12])
            if m:
                out.append(" " * len(m.group(0)))
                i += len(m.group(0))
            else:
                out.append(c)  # lifetime
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def strip_test_items(text):
    """Blank out items annotated #[cfg(test)] or #[test]."""
    res = list(text)
    for m in re.finditer(r"#\[\s*(cfg\(\s*test\s*\)|test)\s*\]", text):
        start = m.start()
        if res[start] == " ":
            continue
        i = m.end()
        n = len(text)
        depth_paren = 0
        while i < n:
            ch = text[i]
            if ch in "([":
                depth_paren += 1
            elif ch in ")]":
                depth_paren -= 1
            elif ch == ";" and depth_paren <= 0:
                i += 1
                break
            elif ch == "{" and depth_paren <= 0:
                depth = 1
                i += 1
                while i < n and depth:
                    if text[i] == "{":
                        depth += 1
                    elif text[i] == "}":
                        depth -= 1
                    i += 1
                break
            i += 1
        for k in range(start, min(i, n)):
            if res[k] != "\n":
                res[k] = " "
    return "".join(res)


def prod_text(path):
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        raw = f.read()
    return strip_test_items(sanitize(raw)), raw


def rust_files(base):
    for dp, dns, fns in os.walk(base):
        dns[:] = [d for d in dns if d not in ("tests", "benches", "archive", "target")]
        for fn in fns:
            if fn.endswith(".rs"):
                yield os.path.join(dp, fn)


def count_file(path):
    text, _ = prod_text(path)
    return {k: len(p.findall(text)) for k, p in PATTERNS.items()}


def main(argv):
    if "--sites" in argv:
        path = argv[argv.index("--sites") + 1]
        text, raw = prod_text(path)
        raw_lines = raw.split("\n")
        for ln, line in enumerate(text.split("\n"), 1):
            if any(p.search(line) for p in PATTERNS.values()):
                print("%s:%d: %s" % (path, ln, raw_lines[ln - 1].strip()))
        return 0
    if "--locks" in argv:
        for path in sorted(rust_files(SRC)):
            text, raw = prod_text(path)
            raw_lines = raw.split("\n")
            for ln, line in enumerate(text.split("\n"), 1):
                if LOCK_RE.search(line):
                    print("%s:%d: %s" % (os.path.relpath(path, ROOT), ln, raw_lines[ln - 1].strip()))
        return 0
    table = {}
    for path in sorted(rust_files(SRC)):
        c = count_file(path)
        c["total"] = sum(c.values())
        if c["total"]:
            table[os.path.relpath(path, ROOT).replace(os.sep, "/")] = c
    total = sum(c["total"] for c in table.values())
    if "--json" in argv:
        print(json.dumps({"total": total, "files": table}, indent=1, sort_keys=True))
        return 0
    for f, c in sorted(table.items(), key=lambda kv: -kv[1]["total"]):
        print("%5d  %s" % (c["total"], f))
    print("%5d  TOTAL (non-test sites)" % total)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
