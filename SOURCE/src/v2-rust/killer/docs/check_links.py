#!/usr/bin/env python3
"""Check relative links in the Markdown documentation.

    python docs/check_links.py [file.md ...]

Default files: the crate README.md and CHANGELOG.md, docs/*.md, and the repository-root README.md and
CHANGELOG.md. For every inline link `[text](target)` outside fenced code blocks and inline code:
  * http(s):, mailto: and other URL schemes are skipped (no network access);
  * the path part must exist, relative to the file that contains the link (a leading `/` is relative to
    the repository root);
  * a `#fragment` pointing into a Markdown file must match a heading of that file (GitHub-style slug).
Exit status 1 if any link is broken. No third-party packages.
"""

import glob
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)


def repo_root():
    d = CRATE
    while True:
        if os.path.isdir(os.path.join(d, ".git")) or os.path.isfile(os.path.join(d, ".git")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return CRATE
        d = parent


ROOT = repo_root()
LINK = re.compile(r"(?<!\!)\[(?:[^\[\]]|\[[^\]]*\])*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
IMAGE = re.compile(r"!\[[^\]]*\]\(([^)\s]+)")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read().replace("\r\n", "\n")


def strip_code(text):
    """Blank out fenced code blocks and inline code spans (keeps line numbers)."""
    out = []
    fence = None
    for line in text.split("\n"):
        m = re.match(r"^\s*(```+|~~~+)", line)
        if fence is None and m:
            fence = m.group(1)[0] * 3
            out.append("")
            continue
        if fence is not None:
            if line.strip().startswith(fence):
                fence = None
            out.append("")
            continue
        out.append(re.sub(r"`[^`]*`", "``", line))
    return "\n".join(out)


def slug(heading):
    h = heading.strip().lower()
    h = re.sub(r"`", "", h)
    h = re.sub(r"[^\w\- ]", "", h, flags=re.UNICODE)
    return h.replace(" ", "-")


_anchor_cache = {}


def anchors(path):
    if path not in _anchor_cache:
        found = set()
        counts = {}
        for line in strip_code(read(path)).split("\n"):
            m = re.match(r"^#{1,6}\s+(.*?)\s*#*\s*$", line)
            if m:
                s = slug(m.group(1))
                n = counts.get(s, 0)
                counts[s] = n + 1
                found.add(s if n == 0 else "%s-%d" % (s, n))
        _anchor_cache[path] = found
    return _anchor_cache[path]


def check_file(path):
    bad = 0
    total = 0
    base = os.path.dirname(path)
    text = strip_code(read(path))
    for lineno, line in enumerate(text.split("\n"), 1):
        targets = LINK.findall(line) + IMAGE.findall(line)
        for target in targets:
            target = target.strip("<>")
            if not target or SCHEME.match(target):
                continue
            total += 1
            part, _, frag = target.partition("#")
            if part == "":
                dest = path
            elif part.startswith("/"):
                dest = os.path.join(ROOT, part.lstrip("/"))
            else:
                dest = os.path.normpath(os.path.join(base, part.split("?")[0]))
            if not os.path.exists(dest):
                print("BROKEN %s:%d -> %s (no such file)" % (os.path.relpath(path, ROOT), lineno, target))
                bad += 1
                continue
            if frag and dest.lower().endswith(".md") and os.path.isfile(dest):
                if frag.lower() not in anchors(dest):
                    print("BROKEN %s:%d -> %s (no heading #%s)" % (os.path.relpath(path, ROOT), lineno, target, frag))
                    bad += 1
    return total, bad


def default_files():
    files = [os.path.join(CRATE, n) for n in ("README.md", "CHANGELOG.md", "DOCS.md")]
    files += sorted(glob.glob(os.path.join(HERE, "*.md")))
    for name in ("README.md", "CHANGELOG.md"):
        p = os.path.join(ROOT, name)
        if os.path.isfile(p) and p not in files:
            files.append(p)
    return [f for f in files if os.path.isfile(f)]


def main():
    files = [os.path.abspath(a) for a in sys.argv[1:]] or default_files()
    total = bad = 0
    for f in files:
        t, b = check_file(f)
        total += t
        bad += b
    print("checked %d relative links in %d files: %d broken" % (total, len(files), bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
