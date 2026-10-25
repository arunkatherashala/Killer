import os, subprocess, sys, tempfile, json

HERE = os.path.dirname(os.path.abspath(__file__))
KILLER = os.environ.get("KILLER_BIN") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "target", "debug", "killer" + (".exe" if os.name == "nt" else ""))

# (area, feature, killer code, expected stdout)
P = []
def p(area, feature, code, expected):
    P.append((area, feature, code, expected))

# ---- numbers
p("numbers", "integer floor division (Python //)", 'println(floor(7 / 2))\nprintln(idiv(-7, 2))', "3\n-4")
p("numbers", "2 ** 100 gives the exact big integer", 'println(2 ** 100)', "1267650600228229401496703205376")
p("numbers", "big integer with n suffix", 'println(2n ** 100)', "1267650600228229401496703205376")
p("numbers", "factorial(25) exact", 'println(factorial(25))', "15511210043330985984000000")
p("numbers", "float arithmetic prints like Python", 'println(0.1 + 0.2)', "0.30000000000000004")
p("numbers", "floor modulo for negatives", 'println(-7 % 3)', "2")
p("numbers", "integer arithmetic past 2**53 is exact", 'println(9007199254740992 + 1, 2 ** 62 * 4)', "9007199254740993 18446744073709551616")
p("numbers", "round / abs / min / max / sum", 'println(round(2.567, 2), abs(-3), min(4, 2, 9), max([1, 5, 3]), sum([1, 2, 3]))', "2.57 3 2 5 6")
p("numbers", "math: sqrt, pow, floor, ceil, log", 'println(sqrt(16), pow(2, 10), floor(2.7), ceil(2.1), log2(8))', "4 1024 2 3 3")
p("numbers", "hex / bin / int parse base", 'println(hex(255), int("ff", 16))', "0xff 255")
# ---- strings
p("strings", "slicing, negative index, repeat", 's = "hello"\nprintln(s[1:3], s[-1], "ab" * 3)', "el o ababab")
p("strings", "f-strings", 'x = 4\nprintln(f"x={x} next={x + 1}")', "x=4 next=5")
p("strings", "split / join / replace / trim", 'println(join(split("a,b,c", ","), "-"), replace("a-b", "-", "+"), trim("  x "))', "a-b-c a+b x")
p("strings", "method call syntax s.upper()", 'println("abc".upper())', "ABC")
p("strings", "unicode length counts characters", 'println(len("héllo"), len("日本語"))', "5 3")
p("strings", "format with placeholders", 'println(format("{} and {}", 1, "two"))', "1 and two")
p("strings", "startswith / endswith / find", 'println(starts_with("hello", "he"), ends_with("hello", "lo"), index_of("hello", "l"))', "true true 2")
# ---- collections
p("collections", "list comprehension with condition", 'println([x * x for x in range(6) if x % 2 == 0])', "[0, 4, 16]")
p("collections", "dict comprehension", 'd = {k: k * 2 for k in [1, 2, 3]}\nprintln(d[2], len(d))', "4 3")
p("collections", "set operations", 's = set_from_array([1, 2, 3])\nt = set_from_array([2, 3, 4])\nprintln(len(set_intersection(s, t)), len(set_union(s, t)))', "2 4")
p("collections", "sorted with key function", 'println(sorted(["bb", "a", "ccc"], len))', "[a, bb, ccc]")
p("collections", "tuple unpacking", 'a, b = 1, 2\na, b = b, a\nprintln(a, b)', "2 1")
p("collections", "enumerate and zip", 'for i, x in enumerate(["a", "b"]) {\n  println(i, x)\n}\nprintln(zip([1, 2], ["x", "y"]))', "0 a\n1 b\n[[1, x], [2, y]]")
p("collections", "dict iteration with items", 'd = {"a": 1, "b": 2}\nfor k, v in d {\n  println(k, v)\n}', "a 1\nb 2")
p("collections", "any / all / map / filter / reduce", 'println(any([0, 1]), all([1, 1]), map([1, 2], str), filter([1, 2, 3, 4], x => x > 2), reduce([1, 2, 3], (a, b) => a + b, 0))', "true true [1, 2] [3, 4] 6")
p("collections", "negative step slice / reverse list", 'println(reverse([1, 2, 3]))', "[3, 2, 1]")
p("collections", "named tuple / dataclass-like record", 'class P {\n  fn init(x, y) {\n    this.x = x\n    this.y = y\n  }\n}\np = new P(1, 2)\nprintln(p.x + p.y)', "3")
# ---- control / functions
p("functions", "default and keyword arguments", 'fn f(a, b = 2) {\n  return a + b\n}\nprintln(f(1), f(1, b = 5))', "3 6")
p("functions", "variadic *args", 'fn s(...xs) {\n  return sum(xs)\n}\nprintln(s(1, 2, 3))', "6")
p("functions", "closures share a variable", 'fn counter() {\n  n = 0\n  return fn() {\n    n = n + 1\n    return n\n  }\n}\nc = counter()\nc()\nprintln(c())', "2")
p("functions", "generators with yield", 'fn g() {\n  yield 1\n  yield 2\n}\nfor x in g() {\n  println(x)\n}', "1\n2")
p("functions", "decorators", 'fn deco(f) {\n  return fn(x) {\n    return f(x) + 1\n  }\n}\n@deco\nfn inc(x) {\n  return x * 2\n}\nprintln(inc(3))', "7")
p("functions", "recursion depth 5000", 'fn d(n) {\n  if n == 0 {\n    return 0\n  }\n  return 1 + d(n - 1)\n}\nprintln(d(5000))', "5000")
p("functions", "match / switch statement", 'x = 2\nmatch x {\n  1 => println("one")\n  2 => println("two")\n  _ => println("other")\n}', "two")
p("functions", "ternary expression", 'println(1 if 2 > 1 else 0)', "1")
p("functions", "walrus-free chained comparison", 'println(1 < 2 < 3, 1 < 3 < 2)', "true false")
# ---- classes / errors
p("oop", "inheritance and super", 'class A {\n  fn hi() {\n    return "a"\n  }\n}\nclass B extends A {\n  fn hi() {\n    return super.hi() + "b"\n  }\n}\nprintln((new B()).hi())', "ab")
p("oop", "operator overloading __add__", 'class V {\n  fn init(x) {\n    this.x = x\n  }\n  fn __add__(o) {\n    return new V(this.x + o.x)\n  }\n}\nprintln((new V(1) + new V(2)).x)', "3")
p("oop", "static / class methods", 'class M {\n  static fn twice(x) {\n    return x * 2\n  }\n}\nprintln(M.twice(4))', "8")
p("oop", "properties / getters", 'class T {\n  fn init() {\n    this._v = 3\n  }\n  get v() {\n    return this._v\n  }\n}\nprintln((new T()).v)', "3")
p("oop", "isinstance / type()", 'println(type(1), type("a"), type([1]))', "number string array")
p("errors", "try / catch / finally", 'try {\n  throw "boom"\n} catch e {\n  println("caught", e)\n} finally {\n  println("done")\n}', "caught boom\ndone")
p("errors", "division by zero is catchable", 'try {\n  x = 1 / 0\n} catch e {\n  println("caught")\n}', "caught")
p("errors", "custom exception class", 'class MyErr {\n  fn init(m) {\n    this.m = m\n  }\n}\ntry {\n  throw new MyErr("x")\n} catch e {\n  println(e.m)\n}', "x")
p("errors", "unassigned variable is an error", 'fn f() {\n  if false {\n    x = 1\n  }\n  return x\n}\ntry {\n  println(f())\n} catch e {\n  println("error")\n}', "error")
p("errors", "with statement / context manager", 'with open("x") as f {\n  println(1)\n}', "__SKIP_IF_ERR__")
p("errors", "assert statement", 'try {\n  assert(1 == 2)\n} catch e {\n  println("assert failed")\n}', "assert failed")
# ---- standard library
p("stdlib", "regex find / replace", 'println(regex_test("caat", "a+"), regex_replace("caat", "a+", "X"))', "true cXt")
p("stdlib", "JSON round trip", 'd = json_parse("{\\"a\\": [1, 2, {\\"b\\": null}]}")\nprintln(json_stringify(d))', '{"a":[1,2,{"b":null}]}')
p("stdlib", "date and time", 'println(len(date_now()) > 0, time_ms() > 0)', "true true")
p("stdlib", "random numbers", 'r = random()\nprintln(r >= 0 and r < 1)', "true")
p("stdlib", "sha256 hash", 'println(sha256("abc"))', "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
p("stdlib", "base64", 'println(base64_encode("hi"), base64_decode("aGk="))', "aGk= hi")
p("stdlib", "read and write a file", 'file_write("probe_tmp.txt", "hello")\nprintln(file_read("probe_tmp.txt"), file_exists("probe_tmp.txt"))\nfile_delete("probe_tmp.txt")', "hello true")
p("stdlib", "command-line arguments", 'println(type(args()))', "array")
p("stdlib", "environment variables", 'println(env("PATH") != null)', "true")
p("stdlib", "CSV parse", 'rows = parse_csv("a,b\\n1,2\\n")\nprintln(len(rows))', "1")
p("stdlib", "compression (gzip/zlib)", 'c = compress("aaaaaaaaaaaaaaaa", "rle")\nprintln(decompress(c, "rle"))', "aaaaaaaaaaaaaaaa")
p("stdlib", "UUID generation", 'println(len(uuid()))', "36")
p("stdlib", "sqlite / SQL database", 'db = sqlite_open(":memory:")\nprintln(1)', "__SKIP_IF_ERR__")
p("stdlib", "run an external command", 'println(len(polyglot_exec("bash", "echo hi")) > 0)', "true")
# ---- concurrency / network
p("concurrency", "async / await", 'async fn f() {\n  return 5\n}\nprintln(await f())', "5")
p("concurrency", "threads with channels", 'ch = chan_new()\nchan_send(ch, 9)\nprintln(chan_recv(ch))', "9")
p("network", "HTTP client request function exists", 'println(type(http_request))', "__ANY__")
p("network", "HTTP server (serve/route)", 'println(type(serve))', "__ANY__")
# ---- language features around the edges
p("syntax", "multiple assignment from function", 'fn f() {\n  return [1, 2]\n}\na, b = f()\nprintln(a + b)', "3")
p("syntax", "string multiplication / membership", 'println("a" in "cat", 3 in [1, 2, 3], "k" in {"k": 1})', "true true true")
p("syntax", "lambda", 'f = x => x + 1\nprintln(f(2))', "3")
p("syntax", "type hints are accepted", 'fn f(x: int) -> int {\n  return x\n}\nprintln(f(2))', "2")
p("syntax", "global statement", 'n = 1\nfn f() {\n  global n\n  n = 5\n}\nf()\nprintln(n)', "5")
p("syntax", "comments with #", '# comment\nprintln(1) # trailing', "1")
p("syntax", "line continuation / multi-line expressions", 'x = (1 +\n  2)\nprintln(x)', "3")
p("syntax", "bytes / bytearray", 'b = bytes_from_str("hi")\nprintln(bytes_len(b))', "2")
p("syntax", "bit operations", 'println(5 & 3, 5 | 3, 5 ^ 3, 1 << 4)', "1 7 6 16")
p("syntax", "import another file", 'import "nope_not_there.killer"\nprintln(1)', "__EXPECT_ERR__")


def run(code):
    d = tempfile.mkdtemp(prefix="kprobe_")
    f = os.path.join(d, "t.killer")
    open(f, "w", encoding="utf-8").write(code + "\n")
    try:
        r = subprocess.run([KILLER, "run", f], cwd=d, capture_output=True, text=True, timeout=30, stdin=subprocess.DEVNULL)
        return r.returncode, r.stdout.replace("\r\n", "\n").strip(), (r.stderr or "").replace("\r\n", "\n").strip()
    except subprocess.TimeoutExpired:
        return -1, "", "TIMEOUT"


results = []
for area, feat, code, exp in P:
    rc, out, err = run(code)
    first_err = (err.splitlines() or [""])[0][:110]
    if exp == "__EXPECT_ERR__":
        status = "PASS" if rc != 0 else "DIFF"
    elif exp == "__SKIP_IF_ERR__":
        status = "PASS" if rc == 0 else "MISSING"
    elif exp == "__ANY__":
        status = "PASS" if rc == 0 else "MISSING"
    elif rc == 0 and out == exp:
        status = "PASS"
    elif rc != 0:
        status = "MISSING" if any(k in (err + out).lower() for k in ("undefined", "unknown", "not defined", "expected", "parse", "unexpected", "no such", "not found", "syntax")) else "ERROR"
    else:
        status = "DIFF"
    results.append(dict(area=area, feature=feat, status=status, got=out[:140], want=exp[:140], err=first_err))

pass
from collections import Counter
print(Counter(r["status"] for r in results), "of", len(results))
for r in results:
    if r["status"] != "PASS":
        print(f'{r["status"]:7} [{r["area"]}] {r["feature"]}\n        got : {r["got"]!r}\n        want: {r["want"]!r}\n        err : {r["err"]}')
