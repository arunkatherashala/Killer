//! `x += e` inside one-line blocks, and `x--` (a silent comment) becoming a compile error.

use std::process::Command;

fn run(name: &str, source: &str) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!("killer_olb_{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), text.trim().to_string())
}

#[test]
fn compound_assignment_in_one_line_blocks() {
    let src = r#"total = 0
for s in [1, 2, 3] { total += s }
println(total)
n = 0
while n < 3 { n++ }
println(n)
m = 0
for s in [1, 2, 3] { m += s; m *= 2 }
println(m)
a = 0
if a == 0 { a += 5 } else { a -= 1 }
println(a)
b = 1
if b == 0 { b += 5 } else { b -= 1 }
println(b)
c = 0
for i in range(3) { if i > 0 { c += i } else { c -= 1 } }
println(c)
d = {"a": 1}
for k in [1, 2] { d["a"] += k }
println(d["a"])
e = {"b": 2}
println(e["b"])
"#;
    let (ok, out) = run("compound", src);
    assert!(ok, "{out}");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["6", "3", "22", "5", "0", "2", "4", "2"]);
}

#[test]
fn decrement_with_double_dash_is_an_error_not_a_silent_comment() {
    let (ok, out) = run("dec1", "i = 3\nwhile i > 0 {\n  i--\n}\nprintln(i)\n");
    assert!(!ok, "{out}");
    assert!(out.contains("Line 3") && out.contains("-= 1"), "{out}");
    let (ok, out) = run("dec2", "xs = [1, 2]\ni = 1\nxs[i]--;\n");
    assert!(!ok && out.contains("Line 3"), "{out}");
}

#[test]
fn double_dash_comments_still_work() {
    let src = "x = 5 -- a trailing comment\n-- a full-line comment\ny = x -- 1 more\ns = \"a--\"\nprintln(x, y, s)\nz = 4\nz -= 1\nprintln(z)\n";
    let (ok, out) = run("comments", src);
    assert!(ok, "{out}");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["5 5 a--", "3"]);
}
