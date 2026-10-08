"""Validate the VS Code extension files: JSON syntax, grammar regexes, keyword coverage."""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FILES = [
    "package.json",
    "language-configuration.json",
    "syntaxes/killer.tmLanguage.json",
    "snippets/killer.json",
]
KEYWORDS = ("fn class extends static new this super if else elif while for in of do match switch "
            "case default try catch finally throw return break continue yield import global let "
            "async await spawn").split()


def load(rel):
    with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
        return json.load(f)


def walk(node, out):
    if isinstance(node, dict):
        for k, v in node.items():
            if k in ("match", "begin", "end") and isinstance(v, str):
                out.append(v)
            walk(v, out)
    elif isinstance(node, list):
        for v in node:
            walk(v, out)


def main():
    errors = []
    docs = {}
    for rel in FILES:
        try:
            docs[rel] = load(rel)
        except Exception as e:
            errors.append(f"{rel}: {e}")
    if errors:
        print("\n".join(errors))
        return 1
    pkg = docs["package.json"]
    if pkg["contributes"]["languages"][0]["id"] != "killer":
        errors.append("language id must be killer")
    for g in pkg["contributes"]["grammars"]:
        if not os.path.exists(os.path.join(ROOT, g["path"])):
            errors.append("missing grammar " + g["path"])
    if not os.path.exists(os.path.join(ROOT, pkg["main"])):
        errors.append("missing main " + pkg["main"])
    grammar = docs["syntaxes/killer.tmLanguage.json"]
    pats = []
    walk(grammar, pats)
    for p in pats:
        try:
            re.compile(p)
        except re.error as e:
            errors.append(f"grammar regex {p!r}: {e}")
    text = json.dumps(grammar)
    for kw in KEYWORDS:
        if not re.search(r"[(|]" + kw + r"[|)]", text):
            errors.append(f"grammar misses keyword {kw}")
    for c in ("#.*$", "//.*$", "--"):
        if c not in grammar["repository"]["comments"]["patterns"][0]["match"] + json.dumps(grammar["repository"]["comments"]):
            errors.append(f"grammar misses comment {c}")
    cfg = docs["language-configuration.json"]
    if not (cfg["brackets"] and cfg["autoClosingPairs"] and "indentationRules" in cfg):
        errors.append("language configuration incomplete")
    for rule in cfg["indentationRules"].values():
        try:
            re.compile(rule)
        except re.error as e:
            errors.append(f"indentation regex {rule!r}: {e}")
    for name, s in docs["snippets/killer.json"].items():
        if "prefix" not in s or "body" not in s:
            errors.append(f"snippet {name} malformed")
    if errors:
        print("\n".join(errors))
        return 1
    print("extension files OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
