//! Regression tests for language bugs and gaps fixed in batch F3a.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_lf_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn zero_bigint_is_falsy_everywhere() {
    let src = "z = 0n\nw = 5n\nprintln(bool(z))\nprintln(bool(w))\nprintln(not z)\nprintln(z && 1)\nprintln(z || 7)\nprintln(1 if z else 2)\nif z { println(\"t\") } else { println(\"f\") }\nprintln(len(filter([0n, 1n, 2n], fn(v) { return v })))\nprintln(any([0n, 0]))\nprintln(all([1n, 0n]))\nprintln(Boolean(z))\n";
    assert_eq!(run(src), "false\ntrue\ntrue\nfalse\ntrue\n2\nf\n2\nfalse\nfalse\nfalse");
}

#[test]
fn bigint_mixes_with_numbers_in_builtins_and_containers() {
    let src = r#"x = 5n
b = 12345678901234567890n
println(int(x))
println(type(int(b)))
println(int("12345678901234567890"))
println(type(int("12345678901234567890")))
println(int("-12345678901234567890"))
println(int("42"))
println(float(x))
println(abs(0n - b))
println(min(x, 3))
println(max([x, b, 2]))
println(sum([x, b]))
println(round(x))
println(sorted([b, x, 3, 2.5]))
println(range(x))
println(chr(65n))
println(5 in [x])
println(x in [1, 5])
println(x in [5.0])
println([7, 5].index_of(x))
println(contains([1, 5], x))
println(x in {"5": 1})
println({x: "v"}[5])
println(x == 5.0)
println(json_stringify([b]))
println(type(json_parse("[12345678901234567890]")[0]))
println(json_parse("[12345678901234567890, 7, 1.5]"))
println(int(true))
"#;
    assert_eq!(
        run(src),
        "5\nbigint\n12345678901234567890\nbigint\n-12345678901234567890\n42\n5\n12345678901234567890\n3\n12345678901234567890\n12345678901234567895\n5\n[2.5, 3, 5, 12345678901234567890]\n[0, 1, 2, 3, 4]\nA\ntrue\ntrue\ntrue\n1\ntrue\ntrue\nv\ntrue\n[12345678901234567890]\nbigint\n[12345678901234567890, 7, 1.5]\n1"
    );
}

#[test]
fn one_line_function_and_class_definitions() {
    let src = "fn f() { return 1 }\nfn g(x) { return x + 1 }\nprintln(f(), g(2))\nclass A { fn m() { return 7 } }\nprintln((new A()).m())\nclass B {\n  fn n() { return 8 }\n  fn o(a) { return a }\n}\nb = new B()\nprintln(b.n(), b.o(3))\nxs = filter([1, 2, 3], fn(v) { return v > 1 })\nprintln(xs)\n";
    assert_eq!(run(src), "1 3\n7\n8 3\n[2, 3]");
}

#[test]
fn dict_literal_after_in_in_headers() {
    let src = "for k, v in {\"a\": 1, \"b\": 2} {\n  println(k, v)\n}\nif \"a\" in {\"a\": 1} {\n  println(\"yes\")\n}\nfor x in [1, 2] { println(x) }\nfor a, b in [[1, 2], [3, 4]] { println(a + b) }\nif \"z\" in {\"a\": 1} { println(\"no\") } else { println(\"absent\") }\n";
    assert_eq!(run(src), "a 1\nb 2\nyes\n1\n2\n3\n7\nabsent");
}

#[test]
fn every_library_builtin_is_a_first_class_value() {
    let src = r#"f = vec_new
s = f(3)
vec_add(s, "a", [1, 0, 0])
vl = vec_len
println(vl(s))
mk = mock_embed
println(len(mk("hello", 4)))
println(len(map(["x", "y"], fn(t) { return mk(t, 3) })))
ch = chunk_text
println(len(ch("one two three four five six", 10, 2)))
ct = count_tokens
println(map(["hello world", "a"], ct))
n = norm
println(n([3, 4]))
println(map([[3, 4], [0, 2]], norm))
d = dot
println(d([1, 2], [3, 4]))
cs = cosine
println(cs([1, 0], [1, 0]))
k = top_k
println(k([0.1, 0.9], 1))
fp = free_port
println(fp() > 0)
li = list
println(li(range(3)))
bp = build_rag_prompt
println(len(bp("q", ["c"])) > 0)
cb = chunk_by_sentence
println(cb("Hi there. Yo.", 20))
isd = isDir
println(isd("."))
println(filter(["a", ".", "zzz_not_there"], isDir))
println(map(["abc", "de"], base64_encode))
println(map([4, 9], sqrt))
pl = pad_left
println(pl("a", 3) + "|" + pl("a", 3, "-"))
g = gcd
println(g(12, 18))
log_set_level("off")
l = log_info
l("hidden")
l("hidden", {"k": 1})
println("done")
"#;
    assert_eq!(
        run(src),
        "1
4
2
4
[4, 1]
5
[5, 2]
11
1
[1]
true
[0, 1, 2]
true
[Hi there. Yo.]
true
[.]
[YWJj, ZGU=]
[2, 3]
  a|--a
6
done"
    );
}

#[test]
fn dunder_methods_len_getitem_setitem_contains_iter_call_pow_neg_mod_bool() {
    let src = r#"class Bag {
  fn init() {
    this.items = [10, 20, 30]
  }
  fn __len__() { return len(this.items) }
  fn __getitem__(i) { return this.items[i] }
  fn __setitem__(i, v) { this.items[i] = v }
  fn __contains__(x) { return x in this.items }
  fn __iter__() { return this.items }
  fn __call__(k) { return k * len(this.items) }
  fn __pow__(e) { return len(this.items) ** e }
  fn __neg__() { return "negated" }
  fn __mod__(m) { return len(this.items) % m }
  fn __bool__() { return len(this.items) > 3 }
}
b = new Bag()
b[1] = 25
println(len(b), b[1], 25 in b, 99 in b)
for x in b { println(x) }
println(b(2), b ** 2, -b, b % 2, bool(b))
if not b { println("small bag") }
"#;
    assert_eq!(run(src), "3 25 true false
10
25
30
6 9 negated 1 false
small bag");
}

#[test]
fn dunder_iteration_call_and_truthiness_protocols() {
    let src = r#"class L {
  fn init() { this.xs = [4, 5, 6] }
  fn __iter__() { return this.xs }
}
for x in new L() { println(x) }
class Cnt {
  fn init(n) {
    this.i = 0
    this.n = n
  }
  fn __iter__() { return this }
  fn __next__() {
    if this.i >= this.n {
      throw "StopIteration"
    }
    this.i = this.i + 1
    return this.i * 10
  }
}
for v in new Cnt(3) { println(v) }
println([v + 1 for v in new Cnt(2)])
fn gen3() {
  yield 1
  yield 2
}
class G {
  fn __iter__() { return gen3() }
}
for g in new G() { println(g) }
class Seq {
  fn __len__() { return 3 }
  fn __getitem__(i) { return i * i }
}
for s in new Seq() { println(s) }
class Adder {
  fn init(k) { this.k = k }
  fn __call__(x) { return x + this.k }
}
add5 = new Adder(5)
println(add5(1))
println(map([1, 2], add5))
class Plain {
  fn init() { this.a = 1 }
}
p = new Plain()
p.a = 3
println(p.a)
class Emp {
  fn __len__() { return 0 }
}
e = new Emp()
if e { println("t") } else { println("empty falsy") }
println(7 in new Bag2())
class Bag2 {
  fn __contains__(x) { return x == 7 }
}
"#;
    assert_eq!(run(src), "4
5
6
10
20
30
[11, 21]
1
2
0
1
4
6
[6, 7]
3
empty falsy
true");
}

#[test]
fn objects_without_the_dunder_method_give_clear_errors() {
    let cases = [
        ("println(len(new P()))", "does not define __len__"),
        ("println(new P()[1])", "does not define __getitem__"),
        ("q = new P()
q[1] = 2", "does not define __setitem__"),
        ("println(1 in new P())", "does not define __contains__"),
        ("for z in new P() { println(z) }", "define __iter__"),
        ("println(new P()(1))", "does not define __call__"),
        ("println(new P() ** 2)", "does not define __pow__"),
        ("println(-new P())", "does not define __neg__"),
        ("println(new P() % 2)", "does not define __mod__"),
    ];
    for (body, want) in cases {
        let out = run(&format!("class P {{
  fn init() {{ this.a = 1 }}
}}
{}
", body));
        assert!(out.contains(want) && out.contains("class P"), "{body}: {out}");
    }
    // plain field assignment and reads are untouched
    assert_eq!(run("class P {
  fn init() { this.a = 1 }
}
p = new P()
p.a = 3
p[\"a\"] = 4
println(p.a, p[\"a\"])
"), "4 4");
}
