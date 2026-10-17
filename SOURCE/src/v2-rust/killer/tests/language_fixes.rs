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
