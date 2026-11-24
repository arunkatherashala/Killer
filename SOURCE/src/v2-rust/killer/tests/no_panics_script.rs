//! Hostile SCRIPTS (source text) through the real binary: the front end and VM must answer
//! with a Killer error, never a panic, stack overflow, giant allocation or hang.
mod common;
use common::*;
use std::time::Duration;

fn cases() -> Vec<(&'static str, Vec<u8>)> {
    let rep = |s: &str, n: usize| s.repeat(n);
    vec![
        ("empty", b"".to_vec()),
        ("whitespace_only", b"   \n\t\n  ".to_vec()),
        ("nul_bytes", vec![0u8; 64]),
        ("bom_only", vec![0xEF, 0xBB, 0xBF]),
        ("invalid_utf8", vec![b'p', b'r', b'i', b'n', b't', b'(', b'"', 0xFF, 0xFE, 0xC0, 0x80, b'"', b')']),
        ("unterminated_string", b"print(\"abc".to_vec()),
        ("lone_quote", b"\"".to_vec()),
        ("unterminated_block_comment", b"/* never closed".to_vec()),
        ("trailing_backslash", b"print(\"abc\\".to_vec()),
        ("bad_escape_u", b"print(\"\\u{ZZZZ}\")".to_vec()),
        ("bad_escape_u_big", b"print(\"\\u{110000}\")".to_vec()),
        ("bad_escape_x", b"print(\"\\x\")".to_vec()),
        ("huge_exponent_literal", b"print(1e999999)\nprint(-1e999999)\nprint(1e-999999)".to_vec()),
        ("huge_int_literal", format!("print({})", rep("9", 5000)).into_bytes()),
        ("bare_hex", b"print(0x)".to_vec()),
        ("bad_hex", b"print(0xZZ)".to_vec()),
        ("double_dot", b"print(1..2)".to_vec()),
        ("deep_parens", format!("print({}1{})", rep("(", 200_000), rep(")", 200_000)).into_bytes()),
        ("deep_array_literal", format!("let a = {}1{}\nprint(len(a))", rep("[", 100_000), rep("]", 100_000)).into_bytes()),
        ("deep_dict_literal", format!("let a = {}1{}", rep("{\"a\":", 50_000), rep("}", 50_000)).into_bytes()),
        ("deep_blocks", format!("{}print(1){}", rep("if true { ", 50_000), rep(" }", 50_000)).into_bytes()),
        ("deep_calls", format!("print({}1{})", rep("abs(", 100_000), rep(")", 100_000)).into_bytes()),
        ("long_add_chain", format!("print(1{})", rep("+1", 500_000)).into_bytes()),
        ("long_unary_chain", format!("print({}1)", rep("-", 200_000)).into_bytes()),
        ("long_not_chain", format!("print({}true)", rep("!", 200_000)).into_bytes()),
        ("unbounded_recursion", b"fn f(n) { return f(n + 1) }\nf(0)".to_vec()),
        ("mutual_recursion", b"fn a(n) { return b(n) }\nfn b(n) { return a(n) }\na(0)".to_vec()),
        ("recursive_closure", b"let f = fn(x) { return f(x) }\nf(1)".to_vec()),
        ("huge_string_literal", format!("let s = \"{}\"\nprint(len(s))", rep("a", 20_000_000)).into_bytes()),
        ("long_identifier", format!("let {} = 1", rep("a", 2_000_000)).into_bytes()),
        ("many_args", format!("print({}0)", rep("1, ", 200_000)).into_bytes()),
        ("self_referencing_array", b"let a = [1]\npush(a, a)\nprint(a)".to_vec()),
        ("self_referencing_dict", b"let d = {}\nd[\"me\"] = d\nprint(d)\nprint(json_stringify(d))".to_vec()),
        ("string_doubling", b"let s = \"ab\"\nfor i in range(0, 60) { s = s + s }\nprint(len(s))".to_vec()),
        ("array_doubling", b"let a = [1]\nfor i in range(0, 60) { a = concat(a, a) }\nprint(len(a))".to_vec()),
        ("int_overflow_arith", b"print(9223372036854775807 + 1)\nprint(-9223372036854775807 - 10)\nprint(9223372036854775807 * 9223372036854775807)".to_vec()),
        ("div_mod_zero", b"print(1 / 0)\nprint(1 % 0)\nprint(0 / 0)".to_vec()),
        ("shift_extremes", b"print(bit_shl(1, 1e300))\nprint(bit_shr(1, -5))\nprint(bit_shl(1, 64))".to_vec()),
        ("index_extremes", b"let a = [1,2,3]\nprint(a[1e300])\nprint(a[-1e300])\nprint(a[0.5])".to_vec()),
        ("slice_extremes", b"let a = [1,2,3]\nprint(a[1e300:2e300])\nprint(slice(a, -1e300, 1e300))\nprint(\"abc\"[5:1])".to_vec()),
        ("negative_range", b"for i in range(5, 0, -1) { }\nfor i in range(0, 5, 0) { break }".to_vec()),
        ("repeat_huge", b"print(len(repeat(\"ab\", 1e300)))".to_vec()),
        ("repeat_negative", b"print(repeat(\"ab\", -5))".to_vec()),
        ("format_braces", b"print(format(\"{} {} {}\", 1))\nprint(format(\"{\", 1))\nprint(format(\"}}{{\", 1))".to_vec()),
        ("interpolation_garbage", b"let x = 1\nprint(\"${x\")\nprint(\"${\")\nprint(\"${}\")\nprint(\"${${x}}\")".to_vec()),
        ("list_comp_garbage", b"print([x for x in])\nprint([for x in [1]])".to_vec()),
        ("import_empty", b"import ''".to_vec()),
        ("import_traversal", b"import '../../../../../../etc/passwd'".to_vec()),
        ("match_no_arms", b"match 1 { }".to_vec()),
        ("lone_operators", b"+\n*\n=\n===\n&&\n||\n??\n?.\n".to_vec()),
        ("unicode_identifiers", "let \u{1F600} = 1\nlet \u{4e2d}\u{6587} = 2\nprint(\u{4e2d}\u{6587})".as_bytes().to_vec()),
        ("control_chars", b"print(\"\x01\x02\x1b[2J\")\n\x0c\x0b".to_vec()),
        ("crlf_mixed", b"let a = 1\r\nprint(a)\rprint(2)\n".to_vec()),
    ]
}

#[test]
fn hostile_scripts_never_crash_the_binary() {
    let dir = scratch_dir("hostile_scripts");
    let only = std::env::var("KILLER_TORTURE_ONLY").ok();
    let mut failures = Vec::new();
    for (name, src) in cases() {
        if let Some(o) = &only {
            if o != name {
                continue;
            }
        }
        let file = dir.join(format!("{name}.killer"));
        std::fs::write(&file, &src).unwrap();
        let out = run_killer(&[file.as_os_str()], &dir, Duration::from_secs(90));
        if let Some(why) = crash_reason(&out) {
            failures.push(format!("{name}: {why}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "scripts crashed the binary:\n{}", failures.join("\n"));
}
