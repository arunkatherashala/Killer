//! Differential checks for the line-based compiler passes: each snippet has an expected output worked out by hand
//! (the comments say how). They cover single evaluation of index and call expressions in compound assignments, `??`,
//! the ternary and short-circuit operators, type-annotation false positives, and lines where quotes, comments and
//! brackets in strings could confuse the string-based splitters (single quotes, `//` and `--` inside strings,
//! brackets inside keys, triple-quoted text, unicode).

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_cc_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    text.replace("\r\n", "\n").trim().to_string()
}

fn lines(src: &str) -> Vec<String> {
    run(src).lines().map(|l| l.to_string()).collect()
}

/// Shared prelude: `nx()` returns 1, 2, 3, ... and records every call in `log`, so any double evaluation of an
/// expression that contains it shows up in `cnt[0]` / `log`.
const PRELUDE: &str = r#"cnt = [0]
log = []
fn nx() {
  cnt[0] += 1
  push(log, cnt[0])
  return cnt[0]
}
"#;

#[test]
fn index_and_call_expressions_in_compound_assignments_run_once() {
    let src = format!(
        r#"{PRELUDE}arr = [0, 0, 0, 0]
arr[nx()] += 7
arr[nx()] += 7
println(arr)
println(log)
d = {{"a": 1, "b": 2}}
fn key() {{
  push(log, "key")
  return "a"
}}
d[key()] += 10
d[key()] *= 3
println(d)
println(len(log))
o = {{"n": 1}}
o.n += nx()
println(o.n)
u = [1, 2, 3]
u[nx() - 3] += 100
println(u)
println(cnt[0])
"#
    );
    // arr: nx()=1 then nx()=2, each adds 7 once -> [0, 7, 7, 0], log [1, 2]
    // d["a"]: 1 + 10 = 11, then * 3 = 33; key() ran twice, so len(log) = 2 + 2 = 4
    // o.n: nx() = 3 -> 1 + 3 = 4
    // u[nx() - 3]: nx() = 4 -> index 1 -> 2 + 100
    let out = lines(&src);
    assert_eq!(&out[..6], ["[0, 7, 7, 0]", "[1, 2]", "{a: 33, b: 2}", "4", "4", "[1, 102, 3]"], "{out:?}");
    assert_eq!(out[6], "4", "u[...] called nx() once: {out:?}");
}

#[test]
fn null_coalescing_ternary_and_short_circuit_evaluate_only_what_they_need() {
    let src = format!(
        r#"{PRELUDE}v = null
v = v ?? nx()
println(v)
w = [null, 5]
w[0] = w[0] ?? nx()
w[1] = w[1] ?? nx()
println(w)
println(cnt[0])
t = nx() > 100 ? nx() : nx()
println(t)
println(cnt[0])
e = (nx() > 0) and (nx() > 0) or (nx() > 0)
println(e)
println(cnt[0])
p = false and nx()
q = true or nx()
println(p)
println(q)
println(cnt[0])
r = true ? nx() : nx()
println(r)
println(cnt[0])
"#
    );
    // v: nx() = 1.  w[0]: nx() = 2, w[1] keeps 5 (no call).  Counter 2.
    // t: condition nx() = 3 (not > 100), else branch nx() = 4 only -> t = 4, counter 4
    // e: 5 > 0 and 6 > 0 are both true, `or` is skipped -> true, counter 6
    // p/q: neither right operand runs, counter stays 6
    // r: only the true branch runs -> 7, counter 7
    assert_eq!(
        lines(&src),
        ["1", "[2, 5]", "2", "4", "4", "true", "6", "false", "true", "6", "7", "7"]
    );
}

#[test]
fn strings_comments_and_brackets_do_not_confuse_the_line_splitters() {
    let src = r#"s = "a//b"
println(s)
s2 = "x -- y" // comment with "quote
println(s2)
s3 = "br[ack" + "et)"
println(s3)
s4 = 'say "hi" // not a comment'
println(s4)
b = 'é' + "→"
b += 'ü'
println(b)
c = [1, 2, 3]
c[1] += 5 -- trailing dash comment with 'quote
println(c)
e = {"k": 'v//w', "m": "a--b"}
println(e["k"] ?? 'x')
g = 7
g -= 2 // minus 'tick
println(g)
n = 1
doc = """line one
n += 5
x = y ?? z
// not a comment
fn fake() {
}
"""
println(doc)
println(n)
"#;
    assert_eq!(
        lines(src),
        ["a//b", "x -- y", "br[acket)", "say \"hi\" // not a comment", "é→ü", "[1, 7, 3]", "v//w", "5", "line one", "n += 5", "x = y ?? z", "// not a comment", "fn fake() {", "}", "", "1"]
    );
}

#[test]
fn brackets_inside_string_keys_in_assignment_targets() {
    // d["a]b"] used to fail with "unsupported Killer subset statement": the `]` inside the key closed the target early
    let src = r#"d = {"a]b": 1, 'c[d': 10, "e)f": 100}
d["a]b"] += 1
d["a]b"] *= 4
d["a]b"] -= 3
d["a]b"]++
d['c[d'] += 5
d["e)f"] -= 1
println(d["a]b"])
println(d['c[d'])
println(d["e)f"])
nested = {"x[": {"y]": 1}}
nested["x["]["y]"] += 2
println(nested)
"#;
    // a]b: 1 + 1 = 2, * 4 = 8, - 3 = 5, ++ = 6;  c[d: 15;  e)f: 99;  nested: {x[: {y]: 3}}
    assert_eq!(lines(src), ["6", "15", "99", "{x[: {y]: 3}}"]);
}

#[test]
fn lambdas_ignore_single_quoted_strings_that_look_like_code() {
    // the lambda lifter masked only double-quoted strings, so these were rewritten or failed to parse
    let src = r#"s = 'x => y'
println(s)
t = 'fn(x) { return x }'
println(t)
f = x => '}' + str(x)
println(f(1))
println(filter(['a(', 'b'], q => q != 'a('))
u = '=>' + ' ' + '(a) => a'
println(u)
println(map([1, 2], x => '[' + str(x) + ']'))
g = x => x * 2 // it's doubling
println(g(2))
"#;
    assert_eq!(lines(src), ["x => y", "fn(x) { return x }", "}1", "[b]", "=> (a) => a", "[[1], [2]]", "4"]);
}

#[test]
fn typed_code_has_no_false_positives() {
    let src = r#"fn f(c: bool) -> number {
  return c ? 1 : 2
}
println(f(1 > 0))
fn g(s: string) -> string {
  return upper(s)
}
r: string = g("x") + g("y")
println(r)
n: number = null ?? 5
println(n)
s: string = 1 > 2 ? "a" : "b"
println(s)
x: string = "a: number = 5"
println(x)
fn h(a: number, b: number) -> number {
  return a - b // comment with "x: string = 5"
}
println(h(5, 2))
fn k(a: number) -> string {
  t = "returns number: 5"
  return t
}
println(k(1))
xs: array = [1, 2]
xs[0] += 1
xs = xs + [3]
println(xs)
i: number = 0
i += 1
i++
println(i)
fn untyped(a) {
  return a
}
m: string = untyped(1)
println(m)
ok: bool = 1 < 2 and 2 < 3
println(not ok)
"#;
    assert_eq!(lines(src), ["1", "XY", "5", "b", "a: number = 5", "3", "returns number: 5", "[2, 2, 3]", "2", "1", "false"]);
}

#[test]
fn typed_code_still_reports_real_errors() {
    assert!(run("x: number = 1\nx = \"now a string\"\nprintln(x)").contains("type error"));
    assert!(run("x: number = \"5\" + 0").contains("type error"));
}
