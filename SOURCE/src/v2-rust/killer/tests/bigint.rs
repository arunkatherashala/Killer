//! Arbitrary-precision integers: literals, operators, builtins, errors, JSON, and a few hundred
//! deterministic random cases whose expected output was computed with Python
//! (tests/data/gen_bigint_cases.py regenerates tests/data/bigint_*).
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_big_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn random_cases_match_python() {
    let program = include_str!("data/bigint_program.killer");
    let expected = include_str!("data/bigint_expected.txt").replace("\r\n", "\n");
    let got = run(program);
    let (g, e): (Vec<&str>, Vec<&str>) = (got.lines().collect(), expected.trim().lines().collect());
    for (i, (a, b)) in g.iter().zip(e.iter()).enumerate() {
        assert_eq!(a, b, "output line {} differs", i + 1);
    }
    assert_eq!(g.len(), e.len(), "line count (first lines of output: {:?})", &g[..g.len().min(3)]);
}

#[test]
fn literals_and_basic_arithmetic() {
    let src = "println(123n)\nprintln(0xFFn)\nprintln(123456789012345678901234567890n + 1n)\nprintln(2n ** 128)\nprintln(2n ** 10)\nprintln(10n - 11n)\nprintln(12n * 12n)\n";
    assert_eq!(run(src), "123\n255\n123456789012345678901234567891\n340282366920938463463374607431768211456\n1024\n-1\n144");
}

#[test]
fn factorial_and_fibonacci() {
    let src = "println(factorial(50))\nfa = 0n\nfb = 1n\nfor i in range(500) {\n  t = fa + fb\n  fa = fb\n  fb = t\n}\nprintln(fa)\n";
    assert_eq!(
        run(src),
        "30414093201713378043612608166064768844377641568960512000000000000\n139423224561697880139724382870407283950070256587697307264108962948325571622863290691557658876222521294125"
    );
}

#[test]
fn mixed_with_numbers_is_exact_when_integral() {
    let src = r#"
b = 2n ** 64
println(b + 1)
println(b * 2)
println(b - 18446744073709551616)
println(b == 18446744073709551616)
println(b > 1.5)
println(b < 1.5)
println(5n == 5)
println(5n == 5.5)
println(5n < 5.5)
println(1 + 2n)
println(0.5 + 2n)
println(7n / 2)
println(8n / 2)
println(9n % 4)
println(2 ** 70n)
"#;
    assert_eq!(
        run(src),
        "18446744073709551617\n36893488147419103232\n0\ntrue\ntrue\nfalse\ntrue\nfalse\ntrue\n3\n2.5\n3.5\n4\n1\n1180591620717411303424"
    );
}

#[test]
fn whole_number_results_beyond_2_pow_53_are_promoted() {
    // a plain integer literal above 2^53 is read as a float (9007199254740993 is not representable), but
    // arithmetic that would lose precision is done exactly: `2 ** 70` is a bigint, and so is `+ - *`
    // on whole numbers whose result passes 2^53. Fractions and identity operations stay plain numbers.
    let src = "println(9007199254740993 + 0)
println(type(2 ** 70))
println(type(2n ** 70))
println(2 ** 70 == 2n ** 70)
println(type(0.5 * 4), type(-9007199254740993), type(2 ** 40))
";
    assert_eq!(run(src), "9007199254740992
bigint
bigint
true
number number number");
}

#[test]
fn floor_semantics_for_division_and_modulo() {
    let src = "println(divmod(7n, 2n))\nprintln(divmod(0n - 7n, 2n))\nprintln(divmod(7n, 0n - 2n))\nprintln(idiv(0n - 7n, 2n))\nprintln((0n - 7n) % 3n)\nprintln(7n % (0n - 3n))\nprintln(divmod(7, 2))\nprintln(idiv(-7, 2))\n";
    assert_eq!(run(src), "[3, 1]\n[-4, 1]\n[-4, -1]\n-4\n2\n-2\n[3, 1]\n-4");
}

#[test]
fn rsa_toy_with_powmod() {
    let src = "p = 1000000007\nq = 998244353\nn = big(p) * q\ne = 65537\nmsg = 123456789012345\nc = powmod(msg, e, n)\nprintln(c)\nprintln(powmod(4, 13, 497))\nprintln(powmod(big(2), big(100), big(1000000007)))\n";
    assert_eq!(run(src), "481649520542750598\n445\n976371285");
}

#[test]
fn truthiness_big_conversions_and_helpers() {
    let src = "if 0n {\n  println(\"zero true\")\n} else {\n  println(\"zero false\")\n}\nif 3n {\n  println(\"three true\")\n}\nprintln(big(\"-42\"))\nprintln(big(7.0))\nprintln(abs(0n - 5n))\nprintln(isqrt(10n ** 30))\nprintln(gcd(12n, 18n))\nprintln(bit_length(2n ** 100))\nprintln(str(2n ** 70) + \"!\")\n";
    assert_eq!(run(src), "zero false\nthree true\n-42\n7\n5\n1000000000000000\n6\n101\n1180591620717411303424!");
}

#[test]
fn json_uses_plain_number_text() {
    let src = "println(json_stringify({\"a\": 2n ** 70}))\nprintln(json_stringify([1, 2n ** 64]))\n";
    assert_eq!(run(src), "{\"a\":1180591620717411303424}\n[1,18446744073709551616]");
}

#[test]
fn errors_are_clear() {
    let out = run("println(10n ** 10 ** 10)\n");
    assert!(out.contains("exponent too large"), "{out}");
    let out = run("println(1n / 0n)\n");
    assert!(out.contains("Division by zero"), "{out}");
    let out = run("println(1n % 0n)\n");
    assert!(out.contains("Modulo by zero"), "{out}");
    let out = run("println(big(2.5))\n");
    assert!(out.contains("expected an integer"), "{out}");
    let out = run("println(big(\"12x\"))\n");
    assert!(out.contains("invalid integer"), "{out}");
    let out = run("println(1n + [1])\n");
    assert!(out.contains("bigint"), "{out}");
    let out = run("try {\n  println(1n / 0n)\n} catch e {\n  println(\"caught\")\n}\n");
    assert_eq!(out, "caught");
}

#[test]
fn bigint_in_loops_functions_and_collections() {
    let src = r#"
fn power_sum(n) {
  total = 0n
  for i in range(n) {
    total = total + big(i) ** 20
  }
  return total
}
println(power_sum(10))
xs = [3n, 1n, 2n ** 70, 2n]
println(sorted(xs))
println(xs[2] > xs[0])
i = 0n
while i < 3n {
  i = i + 1
}
println(i)
println(len([1n, 2n]))
"#;
    assert_eq!(run(src), "13394131858832552133\n[1, 2, 3, 1180591620717411303424]\ntrue\n3\n2");
}

#[test]
fn a_user_function_named_like_a_new_builtin_still_wins() {
    let src = "fn main() {\n  println(factorial(5))\n}\nfn factorial(n) {\n  return 99\n}\nmain()\n";
    assert_eq!(run(src), "99");
}
