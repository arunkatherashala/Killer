import os, subprocess, tempfile, json

HERE = os.path.dirname(os.path.abspath(__file__))
KILLER = os.environ.get("KILLER_BIN") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "target", "debug", "killer" + (".exe" if os.name == "nt" else ""))


def run(code):
    d = tempfile.mkdtemp(prefix="kprobe_")
    f = os.path.join(d, "t.killer")
    open(f, "w", encoding="utf-8").write(code + "\n")
    try:
        r = subprocess.run([KILLER, "run", f], cwd=d, capture_output=True, text=True, timeout=30, stdin=subprocess.DEVNULL)
        return r.returncode, r.stdout.replace("\r\n", "\n").strip(), (r.stderr or "").replace("\r\n", "\n").strip()
    except subprocess.TimeoutExpired:
        return -1, "", "TIMEOUT"


P = [
    # re-checks of round-1 results that may have been my mistake
    ("recheck", "regex_test(text, pattern) order", 'println(regex_test("caat", "a+"), regex_replace("caat", "a+", "X"))', "true cXt"),
    ("recheck", "parse_csv shape", 'rows = parse_csv("a,b\\n1,2\\n")\nprintln(json_stringify(rows))', None),
    ("recheck", "type hint with Killer type name", 'fn f(x: number) {\n  return x\n}\nprintln(f(2))', "2"),
    ("recheck", "int(text, base)", 'println(int("ff", 16))', "255"),
    ("recheck", "round to 2 decimals via scaling", 'println(round(2.567 * 100) / 100)', "2.57"),
    ("recheck", "number to hex text", 'println(format("{:x}", 255))', None),
    ("recheck", "2 ** 100 via big()", 'println(big(2) ** 100)', "1267650600228229401496703205376"),
    # more everyday Python
    ("more", "list method call xs.append(1)", 'xs = []\nxs.append(1)\nprintln(len(xs))', "1"),
    ("more", "push / pop / insert / remove functions", 'xs = [1, 2]\npush(xs, 3)\nprintln(pop(xs), len(xs))', "3 2"),
    ("more", "dict.get with default", 'd = {"a": 1}\nprintln(get(d, "b", 7))', "7"),
    ("more", "dict keys / values / delete key", 'd = {"a": 1, "b": 2}\nprintln(keys(d), values(d))', "[a, b] [1, 2]"),
    ("more", "string % formatting", 'println("%d items" % 3)', None),
    ("more", "while / break / continue", 'i = 0\nwhile true {\n  i = i + 1\n  if i < 3 {\n    continue\n  }\n  break\n}\nprintln(i)', "3"),
    ("more", "nested functions and nonlocal rebinding", 'fn outer() {\n  x = 1\n  fn inner() {\n    x = 2\n  }\n  inner()\n  return x\n}\nprintln(outer())', None),
    ("more", "__str__ / toString used by print", 'class A {\n  fn toString() {\n    return "A!"\n  }\n}\nprintln(new A())', "A!"),
    ("more", "interactive input()", 'x = readline()\nprintln(x)', None),
    ("more", "list slicing assignment / del", 'xs = [1, 2, 3]\nxs[0] = 9\nprintln(xs)', "[9, 2, 3]"),
    ("more", "sorted descending", 'println(sorted([3, 1, 2], true))', "[3, 2, 1]"),
    ("more", "string isdigit / isalpha", 'println(isdigit("123"))', "true"),
    ("more", "enumerate with start / range with step", 'println(range(0, 10, 3))', "[0, 3, 6, 9]"),
    ("more", "multiline strings and escapes", 'println(len("""a\nb"""))', "3"),
    ("more", "list of lists copy semantics", 'a = [[1]]\nb = a\npush(b, [2])\nprintln(len(a))', "2"),
    ("more", "sleep", 'sleep_ms(5)\nprintln(1)', "1"),
    ("more", "exit code via exit()", 'exit(0)', None),
    ("more", "import a module by name (math-like stdlib)", 'import math\nprintln(1)', None),
    ("more", "pretty-printing dict/list", 'println({"a": [1, 2]})', None),
    ("more", "string comparison / sort strings", 'println(sorted(["b", "a", "c"]))', "[a, b, c]"),
    ("more", "integer to string and back", 'println(str(12) + "3", int("45") + 1)', "123 46"),
    ("more", "None/null handling and ?? default", 'x = null\nprintln(x ?? 5)', "5"),
    ("more", "float formatting of whole numbers", 'println(4 / 2, 7 / 2)', "2 3.5"),
]

out = []
for area, feat, code, exp in P:
    rc, so, se = run(code)
    first = (se.splitlines() or [""])[0][:120]
    status = "PASS" if (exp is None and rc == 0) or (exp is not None and rc == 0 and so == exp) else ("DIFF" if rc == 0 else "ERR")
    out.append((area, feat, status, so[:100], exp, first))
    print(f"{status:5} [{area}] {feat}\n      got: {so[:100]!r}  want: {exp!r}\n      {first}")
pass
