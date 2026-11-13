//! Language features added to close the gaps in the Python comparison (docs/PYTHON_COMPARISON.md, "Not in Killer"):
//! spread, set literals, `enum`, dict destructuring, `abstract` / `interface` / `implements`, labeled `break` /
//! `continue`, `const`, unicode names, and the embedded `Fraction` / `Complex` library.
//!
//! Each table row is (script, expected output); an expected output of `ERR` means the script must fail.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> (bool, String, String) {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_langadd_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let s = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.success(), s(&out.stdout).trim_end_matches('\n').to_string(), s(&out.stderr))
}

fn check(cases: &[(&str, &str)]) {
    for (src, want) in cases {
        let (ok, stdout, stderr) = run(src);
        if *want == "ERR" {
            assert!(!ok, "expected an error from:\n{src}\n(got stdout {stdout:?})");
        } else {
            assert!(ok, "script failed: {stderr}\n{src}");
            assert_eq!(&stdout, want, "script:\n{src}");
        }
    }
}

const SPREAD_AND_SETS: &[(&str, &str)] = &[
    (r#"a = [1, 2]
b = [3]
println([...a, 9, ...b])"#, r#"[1, 2, 9, 3]"#),
    (r#"a = [1, 2]
println([0, ...a])
println([...a])
println(len([...a, ...a, ...a]))"#, r#"[0, 1, 2]
[1, 2]
6"#),
    (r#"a = [1]
b = [...a, 2]
push(b, 3)
println(a, b)"#, r#"[1] [1, 2, 3]"#),
    (r#"d = {"a": 1, "b": 2}
e = {...d, "b": 9, "c": 3}
println(e["a"], e["b"], e["c"], len(e))"#, r#"1 9 3 3"#),
    (r#"d = {"a": 1}
x = {"z": 0, ...d}
println(json_stringify(x))"#, r#"{"z":0,"a":1}"#),
    (r#"d = {"a": 1}
d2 = {...d}
d2["a"] = 5
println(d["a"], d2["a"])"#, r#"1 5"#),
    (r#"s = {1, 2, 3, 2}
println(len(s), set_has(s, 2), set_has(s, 9))"#, r#"3 true false"#),
    (r#"s = {"a", "b"}
println(len(s))"#, r#"2"#),
    (r#"e = {}
println(len(e))
f = {"k": 1}
println(len(f))"#, r#"0
1"#),
    (r#"println(len({x * 2 for x in range(4)}))"#, r#"4"#),
    (r#"println([...5])"#, r#"ERR"#),
    (r#"println({...[1]})"#, r#"ERR"#),
];
const ENUM_DESTRUCTURING_ABSTRACT: &[(&str, &str)] = &[
    (r#"enum Color { RED, GREEN, BLUE }
println(Color.RED, Color.GREEN, Color.BLUE)"#, r#"0 1 2"#),
    (r#"enum Level {
  LOW = 10,
  MID,
  HIGH = 50, # comment
  MAX
}
println(Level.LOW, Level.MID, Level.HIGH, Level.MAX)"#, r#"10 11 50 51"#),
    (r#"enum Mode { A = "a", B = "b" }
println(Mode.A + Mode.B)"#, r#"ab"#),
    (r#"enum E { X, Y }
fn f(v) {
  if v == E.Y {
    return "y"
  }
  return "x"
}
println(f(E.X), f(E.Y))"#, r#"x y"#),
    (r#"enum E { A, A }
println(1)"#, r#"ERR"#),
    (r#"enum E { A = "a", B }
println(1)"#, r#"ERR"#),
    (r#"d = {"a": 1, "b": 2, "c": 3}
{a, c} = d
println(a, c)"#, r#"1 3"#),
    (r#"d = {"name": "Ann", "age": 5}
{name: n, age: years} = d
println(n, years)"#, r#"Ann 5"#),
    (r#"{x, missing} = {"x": 1}
println(x, missing)"#, r#"1 null"#),
    (r#"fn f() {
  return {"p": 7, "q": 8}
}
{p, q} = f()
println(p + q)"#, r#"15"#),
    (r#"fn g(d) {
  {a, b} = d
  return a + b
}
println(g({"a": 1, "b": 2}))"#, r#"3"#),
    (r#"x = {"k": 1}
println(x["k"])"#, r#"1"#),
    (r#"class Shape {
  abstract fn area()
  fn describe() {
    return "area " + str(this.area())
  }
}
class Sq extends Shape {
  fn init(s) {
    this.s = s
  }
  fn area() {
    return this.s * this.s
  }
}
println((new Sq(3)).describe())"#, r#"area 9"#),
    (r#"class Shape {
  abstract fn area()
}
class Bad extends Shape {
}
try {
  (new Bad()).area()
} catch e {
  println(contains(e, "abstract"))
}"#, r#"true"#),
    (r#"abstract class Base {
  fn hi() {
    return "hi"
  }
}
println((new Base()).hi())"#, r#"hi"#),
    (r#"interface Greeter {
  fn greet(name)
  fn polite(name) {
    return "dear " + name
  }
}
class En implements Greeter {
  fn greet(name) {
    return "hello " + name
  }
}
e = new En()
println(e.greet("a"), e.polite("b"))"#, r#"hello a dear b"#),
    (r#"interface I {
  fn m()
}
class C implements I {
}
try {
  (new C()).m()
} catch e {
  println(contains(e, "interface I"))
}"#, r#"true"#),
    (r#"class Base {
  fn who() {
    return "base"
  }
}
interface Tag {
  fn tag()
}
class K extends Base implements Tag {
  fn tag() {
    return "k"
  }
}
k = new K()
println(k.who(), k.tag())"#, r#"base k"#),
    (r#"enum = 3
println(enum + 1)"#, r#"4"#),
    (r#"abstract = 2
interface = 3
println(abstract + interface)"#, r#"5"#),
];
const LABELED_LOOPS: &[(&str, &str)] = &[
    (r#"outer: for i in range(3) {
  for j in range(3) {
    if j == 1 {
      break outer
    }
    println(i, j)
  }
}
println("done")"#, r#"0 0
done"#),
    (r#"outer: for i in range(3) {
  for j in range(3) {
    if j == 1 {
      continue outer
    }
    println(i, j)
  }
}
println("done")"#, r#"0 0
1 0
2 0
done"#),
    (r#"n = 0
outer: for i in range(3) {
  for j in range(3) {
    for k in range(3) {
      if k == 1 {
        break outer
      }
      n += 1
    }
  }
}
println(n)"#, r#"1"#),
    (r#"n = 0
outer: for i in range(3) {
  for j in range(3) {
    for k in range(3) {
      if k == 1 {
        continue outer
      }
      n += 1
    }
    n += 100
  }
  n += 1000
}
println(n)"#, r#"3"#),
    (r#"i = 0
outer: while i < 5 {
  i += 1
  for j in range(5) {
    if j == 2 {
      continue outer
    }
    if i == 4 {
      break outer
    }
  }
  println("never")
}
println(i)"#, r#"4"#),
    (r#"outer: for i in range(2) {
  println("i", i)
  if i == 0 {
    continue outer
  }
  break outer
}
println("end")"#, r#"i 0
i 1
end"#),
    (r#"total = 0
for a in range(3) {
  row: for b in range(3) {
    for c in range(3) {
      if c == 1 {
        continue row
      }
      total += 1
    }
  }
}
println(total)"#, r#"9"#),
    (r#"fn find(grid, want) {
  found = null
  search: for r in range(len(grid)) {
    for c in range(len(grid[r])) {
      if grid[r][c] == want {
        found = [r, c]
        break search
      }
    }
  }
  return found
}
println(find([[1, 2], [3, 4]], 3), find([[1]], 9))"#, r#"[1, 0] null"#),
    (r#"a: for i in range(2) {
  b: for j in range(2) {
    for k in range(2) {
      if k == 0 {
        continue a
      }
    }
    println("no")
  }
  println("no")
}
println("ok")"#, r#"ok"#),
    (r#"outer: for i in range(3) {
  println(i)
}
println("x")"#, r#"0
1
2
x"#),
    (r#"d = {
  "a": 1,
  "b": 2
}
println(d["b"])"#, r#"2"#),
    (r#"x = 1
sw = x
switch sw {
  case 1: println("one")
  default: println("other")
}"#, r#"one"#),
];
const CONSTANTS: &[(&str, &str)] = &[
    (r#"const PI = 3
println(PI * 2)"#, r#"6"#),
    (r#"const NAME = "x"
const LIMIT = 10 + 5
println(NAME, LIMIT)"#, r#"x 15"#),
    (r#"const X = 1
X = 2
println(X)"#, r#"ERR"#),
    (r#"const X = 1
X += 1
println(X)"#, r#"ERR"#),
    (r#"const X = 1
X++
println(X)"#, r#"ERR"#),
    (r#"const X = 1
if true {
  X = 5
}
println(X)"#, r#"ERR"#),
    (r#"const X = 1
const X = 2
println(X)"#, r#"ERR"#),
    (r#"const X
println(1)"#, r#"ERR"#),
    (r#"const LIST = [1, 2]
push(LIST, 3)
println(len(LIST))"#, r#"3"#),
    (r#"const D = {"a": 1}
D["a"] = 5
println(D["a"])"#, r#"5"#),
    (r#"const X = 1
println(X == 1, X != 2)"#, r#"true true"#),
    (r#"const MAX = 10
fn f() {
  MAX = 5
  return MAX
}
println(f(), MAX)"#, r#"5 10"#),
    (r#"const MAX = 10
fn f() {
  global MAX
  MAX = 5
}
f()
println(MAX)"#, r#"ERR"#),
    (r#"fn f() {
  const K = 3
  return K * 2
}
println(f())
K = 9
println(K)"#, r#"6
9"#),
    (r#"fn f() {
  const K = 3
  K = 4
  return K
}
println(f())"#, r#"ERR"#),
    (r#"fn f() {
  const K = 3
  return K
}
fn g() {
  const K = 4
  return K
}
println(f(), g())"#, r#"3 4"#),
    (r#"const = 3
println(const + 1)"#, r#"4"#),
    (r#"const A = 1
for i in range(2) {
  x = A + i
  println(x)
}"#, r#"1
2"#),
    (r#"const A = 1
B = A
B = 2
println(A, B)"#, r#"1 2"#),
    (r#"const total = 0
for total in range(2) {
  println(total)
}"#, r#"0
1"#),
];
const UNICODE_NAMES: &[(&str, &str)] = &[
    (r#"naïve = "日本語"
println(len(naïve), naïve)"#, r#"3 日本語"#),
    (r#"π = 3
println(π * 2)"#, r#"6"#),
    (r#"fn größe(x) {
  return x * 2
}
println(größe(4))"#, r#"8"#),
    (r#"class Café {
  fn init() {
    this.preis = 3
  }
}
c = new Café()
println(c.preis)"#, r#"3"#),
    (r#"名前 = "x"
println(名前)"#, r#"x"#),
    (r#"x = "ünï"
println(upper(x), len(x))"#, r#"ÜNÏ 3"#),
    (r#"println("→ arrow", "é" + "à")"#, r#"→ arrow éà"#),
    (r#"# café comment
println(1) # naïve"#, r#"1"#),
    (r#"d = {"clé": 1}
println(d["clé"])"#, r#"1"#),
    (r#"for 数 in range(2) {
  println(数)
}"#, r#"0
1"#),
    (r#"a_b1 = 2
_x = 3
println(a_b1 + _x)"#, r#"5"#),
];
const FRACTIONS_AND_COMPLEX: &[(&str, &str)] = &[
    (r#"println(Fraction(1, 3) + Fraction(1, 6))"#, r#"1/2"#),
    (r#"println(Fraction(2, 4), Fraction(-6, 4), Fraction(6, -4), Fraction(4, 2))"#, r#"1/2 -3/2 -3/2 2"#),
    (r#"a = Fraction(3, 4)
println(a * 2, 2 * a, a - 1, 1 - a, a / 3, 3 / a)"#, r#"3/2 3/2 -1/4 1/4 1/4 4"#),
    (r#"println(Fraction("3/4") + Fraction("1/4"), Fraction("5"))"#, r#"1 5"#),
    (r#"println(Fraction(1, 2) == Fraction(2, 4), Fraction(1, 3) < Fraction(1, 2), Fraction(1, 2) > Fraction(1, 3))"#, r#"true true true"#),
    (r#"println(Fraction(2, 3) ** 3, Fraction(2, 3) ** -2, -Fraction(1, 2))"#, r#"8/27 9/4 -1/2"#),
    (r#"x = Fraction(0, 1)
for i in range(10) {
  x = x + Fraction(1, 10)
}
println(x, x == 1, x.to_float())"#, r#"1 true 1"#),
    (r#"println(Fraction(1, 3).num, Fraction(1, 3).den)"#, r#"1 3"#),
    (r#"println(Fraction(1, 0))"#, r#"ERR"#),
    (r#"println(Fraction(0.5, 1))"#, r#"ERR"#),
    (r#"println(Fraction(1, 2) / Fraction(0, 1))"#, r#"ERR"#),
    (r#"a = Complex(1, 2)
b = Complex(3, -1)
println(a + b, a - b, a * b)"#, r#"4+1i -2+3i 5+5i"#),
    (r#"println(Complex(3, 4).abs(), Complex(3, 4).conj(), -Complex(1, 2))"#, r#"5 3-4i -1-2i"#),
    (r#"println(Complex(1, 1) == Complex(1, 1), 2 * Complex(1, 2), Complex(1, 2) + 1)"#, r#"true 2+4i 2+2i"#),
    (r#"println(Complex(1, 2) / Complex(1, 2), Complex(4, 2) / 2)"#, r#"1+0i 2+1i"#),
    (r#"println(Complex(2))"#, r#"2+0i"#),
    (r#"fn Fraction(a) {
  return a + 1
}
println(Fraction(1))"#, r#"2"#),
    (r#"class Complex {
  fn init() {
    this.v = 7
  }
}
println((new Complex()).v)"#, r#"7"#),
    (r#"x = "Fraction(1, 2)"
println(x)"#, r#"Fraction(1, 2)"#),
    (r#"# Complex(1, 2)
println(1)"#, r#"1"#),
];
const LIST_PATTERNS: &[(&str, &str)] = &[
    (r#"fn d(x) {
  match x {
    [] => { return "empty" }
    [a] => { return "one " + str(a) }
    [1, b] => { return "s1 " + str(b) }
    [h, ...t] if len(t) > 2 => { return "long " + str(len(t)) }
    [h, ...t] => { return "list " + str(h) + str(t) }
    _ => { return "other" }
  }
}
println(d([]), d([7]), d([1, 2]), d([5, 6]), d([1, 2, 3, 4]), d(3))"#, r#"empty one 7 s1 2 list 5[6] long 3 other"#),
    (r#"match [1, [2, 3]] {
  [a, [b, c]] => { println(a + b + c) }
}"#, r#"6"#),
    (r#"match [1, 2] {
  [_, y] => { println(y) }
}"#, r#"2"#),
    (r#"match "ab" {
  [a] => { println("list") }
  _ => { println("not a list") }
}"#, r#"not a list"#),
];
const LABELED_DO: &[(&str, &str)] = &[
    (r#"n = 0
outer: do {
  n = n + 1
  for i in range(3) {
    if i == 1 { continue outer }
    if n == 3 { break outer }
  }
} while n < 10
println(n)"#, r#"3"#),
    (r#"k = 0
do {
  k++
} while k < 3
println(k)"#, r#"3"#),
];
const FILE_ITERATION: &[(&str, &str)] = &[
    (r#"f = open("it.txt", "w")
f.write("a\nb\nc")
f.close()
for line in open("it.txt") {
  println(trim(line))
}"#, r#"a
b
c"#),
    (r#"f = open("it2.txt", "w")
f.write("x\ny\n")
f.close()
n = 0
with open("it2.txt") as g {
  for line in g { n = n + 1 }
}
println(n)"#, r#"2"#),
];

/// `[a, b]`, `[h, ...t]`, `[]`, literals, `_` and guards in `match`.
#[test]
fn list_patterns_in_match() {
    check(LIST_PATTERNS);
}

/// `label: do { } while cond` with `break label` / `continue label`.
#[test]
fn labeled_do_while() {
    check(LABELED_DO);
}

/// `for line in open(path)` and over a `with` file.
#[test]
fn for_loop_over_a_file() {
    check(FILE_ITERATION);
}

/// `[...a, 3]`, `{...d, k: v}` (`dict_merge`) and `{1, 2, 3}`.
#[test]
fn spread_and_set_literals() {
    check(SPREAD_AND_SETS);
}

/// `enum`, `{a, b: c} = d`, `abstract fn`, `interface`, `implements`.
#[test]
fn enums_dict_destructuring_abstract_methods_and_interfaces() {
    check(ENUM_DESTRUCTURING_ABSTRACT);
}

/// `label: for` / `while` with `break label` and `continue label` at several depths.
#[test]
fn labeled_break_and_continue() {
    check(LABELED_LOOPS);
}

/// `const NAME = value`: reassignment, `+=`, `++` and redeclaration are errors; contents can change.
#[test]
fn const_declarations_cannot_be_reassigned() {
    check(CONSTANTS);
}

/// Non-ASCII letters in variable, function, class and key names.
#[test]
fn unicode_identifiers_and_text() {
    check(UNICODE_NAMES);
}

/// `Fraction` and `Complex`, loaded only when a script uses them.
#[test]
fn fractions_and_complex_numbers() {
    check(FRACTIONS_AND_COMPLEX);
}

#[test]
fn error_lines_are_correct_after_labels_enums_destructuring_and_the_number_library() {
    let src = "enum E { A, B }\n{p, q} = {\"p\": 1, \"q\": 2}\nouter: for i in range(2) {\n  for j in range(2) {\n    continue outer\n  }\n}\nf = Fraction(1, 2)\nboom = missing_function(p)\n";
    let (ok, _, stderr) = run(src);
    assert!(!ok);
    assert!(stderr.contains("Line 9"), "stderr was: {stderr}");
}

#[test]
fn a_constant_error_names_the_constant_and_the_right_line() {
    let (ok, _, stderr) = run("x = 1\nconst LIMIT = 10\ny = 2\nLIMIT = 11\n");
    assert!(!ok);
    assert!(stderr.contains("LIMIT") && stderr.contains("Line 4"), "stderr was: {stderr}");
}
