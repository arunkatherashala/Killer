//! Hostile-argument torture test: every pure builtin is called through the real `killer`
//! binary with huge / negative / NaN-ish / empty / odd values.  A call may fail with a
//! Killer error (caught by try/catch), but the process must never panic, overflow its stack,
//! abort on a huge allocation or hang.
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const VALUES: &[&str] = &[
    "0",
    "-1",
    "1e300",
    "-1e300",
    "9223372036854775807",
    "0.5",
    "1e308 * 1e308",
    "\"\"",
    "\"abc\"",
    "\"\u{e9}\u{4e2d}\"",
    "[]",
    "[1, 2, 3]",
    "{}",
    "null",
    "true",
];

/// (builtin name, maximum number of arguments to try)
const FUNCS: &[(&str, usize)] = &[
    ("len", 1), ("push", 2), ("pop", 1), ("contains", 2), ("index_of", 2), ("get", 3),
    ("setdefault", 3), ("copy", 1), ("reversed", 1), ("range", 3), ("type_of", 1),
    ("to_string", 1), ("to_int", 1), ("to_float", 1), ("keys", 1), ("values", 1),
    ("sort", 1), ("reverse", 1), ("join", 2), ("split", 2), ("trim", 1), ("replace", 3),
    ("upper", 1), ("lower", 1), ("starts_with", 2), ("ends_with", 2), ("substr", 3),
    ("format", 3), ("parse_int", 2), ("parse_float", 1), ("abs", 1), ("floor", 1),
    ("ceil", 1), ("round", 2), ("sqrt", 1), ("pow", 2), ("sin", 1), ("cos", 1), ("tan", 1),
    ("bit_and", 2), ("bit_or", 2), ("bit_xor", 2), ("bit_shl", 2), ("bit_shr", 2),
    ("min", 2), ("max", 2), ("concat", 2), ("insert", 3), ("hash_map_new", 1),
    ("hash_map_insert", 3), ("hash_map_get", 2), ("hash_map_remove", 2), ("hash_map_size", 1),
    ("trit_and", 2), ("trit_or", 2), ("trit_not", 1), ("trit_add", 2), ("trit_mul", 2),
    ("trit_to_int", 1), ("trit_from_int", 1), ("int_to_trit", 1), ("trit_to_str", 1),
    ("trit_word", 2), ("trit_word_to_int", 1), ("fuzzy_and", 2), ("fuzzy_or", 2),
    ("fuzzy_not", 1), ("fuzzy_threshold", 2), ("fuzzy_combine", 2), ("signal_create", 3),
    ("signal_value", 1), ("signal_confidence", 1), ("signal_and", 2), ("signal_or", 2),
    ("qubit_create", 2), ("qubit_hadamard", 1), ("qubit_measure", 1), ("qubit_entangle", 2),
    ("tryte_create", 2), ("tryte_from_int", 1), ("tryte_to_int", 1), ("tryte_to_str", 1),
    ("tryte_get", 2), ("tryte_set", 3), ("tryte_add", 2), ("str", 1), ("int", 1),
    ("substring", 3), ("indexOf", 2), ("repeat", 2), ("charAt", 2), ("charCodeAt", 2),
    ("slice", 3), ("includes", 2), ("sorted", 1), ("sum", 1), ("enumerate", 2), ("all", 1),
    ("any", 1), ("zip", 2), ("parse_json", 1), ("json_stringify", 1), ("json_pretty", 2),
    ("parse_csv", 2), ("to_csv", 2), ("to_yaml", 2), ("parse_datetime", 1),
    ("format_datetime", 2), ("hex", 1), ("bin", 1), ("oct", 1), ("dict_merge", 2),
    ("ord", 1), ("chr", 1), ("pad_left", 3), ("pad_right", 3), ("strip", 1), ("lstrip", 1),
    ("rstrip", 1), ("startsWith", 2), ("endsWith", 2), ("base64_encode", 1),
    ("base64_decode", 1), ("hmac_sha256", 2), ("float", 1), ("bool", 1), ("log", 2),
    ("log2", 1), ("log10", 1), ("exp", 1), ("cbrt", 1), ("trunc", 1), ("sign", 1),
    ("atan2", 2), ("hypot", 2), ("asin", 1), ("acos", 1), ("atan", 1), ("sinh", 1),
    ("cosh", 1), ("tanh", 1), ("degrees", 1), ("radians", 1), ("big", 1), ("factorial", 1),
    ("divmod", 2), ("idiv", 2), ("powmod", 3), ("isqrt", 1), ("bit_length", 1), ("gcd", 2),
    ("lcm", 2), ("mean", 1), ("median", 1), ("variance", 1), ("stdev", 1),
    ("pvariance", 1), ("pstdev", 1), ("instanceof", 2), ("dict_from_pairs", 1),
    ("uncertain", 2), ("unc_value", 1), ("unc_lo", 1), ("unc_hi", 1), ("gauss", 2),
    ("prob_gt", 2), ("prob_lt", 2),
];

fn tuples(arity: usize) -> Vec<Vec<&'static str>> {
    let mut out: Vec<Vec<&'static str>> = vec![vec![]];
    for _ in 0..arity {
        let mut next = Vec::new();
        for t in &out {
            for v in VALUES {
                let mut n = t.clone();
                n.push(v);
                next.push(n);
            }
        }
        out = next;
    }
    out
}

/// Run a script file with the real binary; returns (timed_out, exit_code, combined_output).
fn run_script(path: &std::path::Path, limit: Duration) -> (bool, Option<i32>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer"))
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn killer");
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t_out = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = so.read_to_end(&mut b);
        b
    });
    let t_err = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = se.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    let mut timed_out = false;
    let code = loop {
        match child.try_wait().expect("wait") {
            Some(st) => break st.code(),
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    let mut text = String::from_utf8_lossy(&t_out.join().unwrap()).into_owned();
    text.push_str(&String::from_utf8_lossy(&t_err.join().unwrap()));
    (timed_out, code, text)
}

fn crashed(timed_out: bool, code: Option<i32>, text: &str) -> Option<String> {
    if timed_out {
        return Some("hung (timeout)".to_string());
    }
    for marker in ["panicked at", "has overflowed its stack", "memory allocation of", "RUST_BACKTRACE"] {
        if text.contains(marker) {
            let line = text.lines().find(|l| l.contains(marker)).unwrap_or(marker);
            return Some(line.chars().take(200).collect());
        }
    }
    match code {
        Some(0) | Some(1) | Some(2) => None,
        other => Some(format!("abnormal exit {:?}", other)),
    }
}

#[test]
fn hostile_arguments_never_crash_a_builtin() {
    let dir = std::env::temp_dir().join(format!("killer_hostile_builtins_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let only = std::env::var("KILLER_TORTURE_ONLY").ok();
    let mut failures: Vec<String> = Vec::new();
    for (name, max_arity) in FUNCS {
        if let Some(o) = &only {
            if o != name {
                continue;
            }
        }
        let mut src = String::new();
        for arity in 0..=*max_arity {
            for t in tuples(arity) {
                src.push_str(&format!("try {{\n  let r = {}({})\n}} catch e {{\n}}\n", name, t.join(", ")));
            }
        }
        src.push_str("println(\"done\")\n");
        let file = dir.join(format!("{name}.killer"));
        std::fs::write(&file, src).unwrap();
        let (to, code, text) = run_script(&file, Duration::from_secs(90));
        if let Some(why) = crashed(to, code, &text) {
            failures.push(format!("{name}: {why}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "builtins crashed on hostile input:\n{}", failures.join("\n"));
}
