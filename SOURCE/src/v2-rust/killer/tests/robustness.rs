//! Robustness: the front end and the VM must never panic, hang or overflow the native stack, whatever
//! they are fed. Contains a deterministic fuzz harness (seeded xorshift, no external crates) and
//! regression tests for every crash or hang it found.
//!
//! The default fuzz test runs a few thousand cases in a few seconds. For a long run:
//!   KILLER_FUZZ_ITERS=200000 KILLER_FUZZ_SEED=7 cargo test --test robustness fuzz_long -- --ignored --nocapture
use killer_native::compiler::compile_killer_subset;
use killer_native::security::{CapabilityScopeGuard, CapabilitySet, ExecutionBudget};
use killer_native::vm::VirtualMachine;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

// ------------------------------------------------------------------------------------------------
// Execution harness
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    /// Compiled and ran to completion.
    Ok,
    /// Compile error or runtime error (a clean, reported failure).
    Err(String),
    /// A Rust panic escaped (always a bug).
    Panic(String),
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
    }
}

/// Compile and run `src` in this thread. Capabilities: no files / network / processes / ffi / llm.
fn execute(src: &str, budget: Option<ExecutionBudget>) -> Outcome {
    let caps = CapabilitySet { allow_native_jit: true, ..CapabilitySet::restricted() };
    let _guard = CapabilityScopeGuard::install(caps);
    let r = catch_unwind(AssertUnwindSafe(|| {
        let program = match compile_killer_subset(src) {
            Ok(p) => p,
            Err(e) => return Outcome::Err(format!("{e}")),
        };
        let mut vm = VirtualMachine::new();
        vm.set_execution_budget(budget);
        match vm.run(&program) {
            Ok(()) => Outcome::Ok,
            Err(e) => Outcome::Err(format!("{e}")),
        }
    }));
    match r {
        Ok(o) => o,
        Err(p) => Outcome::Panic(panic_text(p)),
    }
}

fn fuzz_budget() -> Option<ExecutionBudget> {
    Some(ExecutionBudget { max_instruction_steps: 400_000, max_wall_ms: 4_000, max_heap_bytes: 0 })
}

/// A long-lived worker thread with a big stack (like the `killer_super` binary's) so one thread
/// serves thousands of cases and a hang is detected by a timeout on the receiving side.
struct Worker {
    tx: Sender<(String, bool)>,
    rx: Receiver<Outcome>,
}

impl Worker {
    fn new() -> Worker {
        let (tx, job_rx) = channel::<(String, bool)>();
        let (out_tx, rx) = channel::<Outcome>();
        std::thread::Builder::new()
            .name("robustness-worker".into())
            .stack_size(512 * 1024 * 1024)
            .spawn(move || {
                while let Ok((src, budgeted)) = job_rx.recv() {
                    let o = execute(&src, if budgeted { fuzz_budget() } else { None });
                    if out_tx.send(o).is_err() {
                        break;
                    }
                }
            })
            .unwrap();
        Worker { tx, rx }
    }

    fn run(&self, src: &str, budgeted: bool, timeout: Duration) -> Option<Outcome> {
        self.tx.send((src.to_string(), budgeted)).unwrap();
        self.rx.recv_timeout(timeout).ok()
    }
}

/// Run one program on a fresh big-stack thread (no budget) and return its outcome; panics on a hang.
fn run_big(src: &str) -> Outcome {
    let w = Worker::new();
    w.run(src, false, Duration::from_secs(120)).unwrap_or_else(|| panic!("hang on:\n{}", &src[..src.len().min(400)]))
}

fn save_repro(tag: &str, src: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("killer_fuzz_repro_{}_{}.killer", tag, std::process::id()));
    let _ = std::fs::write(&path, src);
    path
}

// ------------------------------------------------------------------------------------------------
// Deterministic RNG and mutators
// ------------------------------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const TOKENS: &[&str] = &[
    "fn", "if", "else", "while", "for", "in", "return", "break", "continue", "match", "try", "catch", "finally",
    "throw", "class", "extends", "new", "this", "super", "yield", "let", "true", "false", "null", "and", "or",
    "not", "(", ")", "[", "]", "{", "}", ",", ";", ":", ".", "=>", "->", "=", "==", "!=", "<", ">", "<=", ">=",
    "+", "-", "*", "/", "%", "**", "&&", "||", "!", "?", "|", "&", "^", "~", "<<", ">>", "+=", "-=", "..", "...",
    "x", "y", "i", "n", "a", "b", "f", "g", "xs", "len", "push", "pop", "range", "map", "filter", "reduce", "str",
    "keys", "values", "sorted", "slice", "split", "join", "replace", "json_parse", "json_stringify", "println",
    "print", "0", "1", "2", "3", "10", "255", "1e9", "1e300", "-1", "0.5", "\"s\"", "\"\"", "\"abc\"", "'q'",
    "f\"v={x}\"", "\"{\"", "\"\\", "\"\\u{", "#", "//", "/*", "*/", "@", "$", "`", "\n", "\n", "\n", "\t", "\\",
    "__add__", "toString", "init", "set", "set_add", "chan_new", "mutex_new", "regex_match", "abs", "min", "max",
    "sum", "round", "floor", "int", "float", "type", "is_null", "assert", "assert_eq",
];

const NUMBERS: &[&str] = &[
    "0", "1", "-1", "2", "7", "100", "1000", "65536", "1e9", "1e12", "1e15", "1e300", "-1e300", "0.1", "9007199254740993",
    "4294967296", "18446744073709551616", "1e-9", "0.0",
];

/// Words that could touch the outside world; lines that mention them are never fuzzed.
const DANGEROUS: &[&str] = &[
    "exec", "system", "shell", "delete", "remove", "unlink", "write_file", "append_file", "http", "fetch", "download",
    "upload", "socket", "ffi", "python", "spawn", "kill", "rmdir", "mkdir", "rename", "copy_file", "ssh", "curl",
    "env", "listen", "serve", "connect", "llm", "ask", "chat", "think", "agent", "open", "save", "export", "dump",
    "file", "dir", "path", "read", "import", "include", "use ", "sleep", "input", "prompt", "jit", "native", "load",
    "mount", "tcp", "udp", "tmp", "temp", "pip", "cargo", "git ", "gpu", "cluster", "ghost", "kore", "novz",
];

fn is_safe_line(l: &str) -> bool {
    let low = l.to_lowercase();
    !DANGEROUS.iter().any(|d| low.contains(d))
}

fn collect_seed_dirs() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    vec![root.join("examples"), root.join("parity").join("realworld"), root.join("tests")]
}

fn load_seeds() -> Vec<String> {
    let mut seeds = Vec::new();
    for dir in collect_seed_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.extension().and_then(|e| e.to_str()) != Some("killer") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            if text.len() > 20_000 {
                continue;
            }
            // keep only the safe lines so no mutation can reach the file system or network
            let kept: Vec<&str> = text.lines().filter(|l| is_safe_line(l)).collect();
            if kept.len() >= 2 {
                seeds.push(kept.join("\n"));
            }
        }
    }
    seeds.push("fn fib(n) {\n  if n < 2 {\n    return n\n  }\n  return fib(n - 1) + fib(n - 2)\n}\nprintln(fib(10))\n".to_string());
    seeds.push("xs = [3, 1, 2]\nfor v in sorted(xs) {\n  println(v)\n}\nd = {\"a\": 1}\nprintln(d[\"a\"])\n".to_string());
    seeds
}

fn mutate(rng: &mut Rng, seed: &str) -> String {
    let mut s: Vec<char> = seed.chars().collect();
    let rounds = 1 + rng.below(4);
    for _ in 0..rounds {
        if s.is_empty() {
            s.extend("x = 1\n".chars());
        }
        match rng.below(10) {
            0 => {
                // delete a span
                let at = rng.below(s.len());
                let n = 1 + rng.below(12);
                let end = (at + n).min(s.len());
                s.drain(at..end);
            }
            1 => {
                // insert a token
                let at = rng.below(s.len() + 1);
                let t = *rng.pick(TOKENS);
                let ins: Vec<char> = format!(" {t} ").chars().collect();
                for (k, c) in ins.into_iter().enumerate() {
                    s.insert(at + k, c);
                }
            }
            2 => {
                // overwrite one char with punctuation
                let at = rng.below(s.len());
                s[at] = *rng.pick(&['(', ')', '{', '}', '[', ']', '"', '\'', '\\', ',', ';', '=', '.', '-', '\n', '0', '9', ' ']);
            }
            3 => {
                // truncate
                let at = rng.below(s.len());
                s.truncate(at);
            }
            4 => {
                // duplicate a span
                let at = rng.below(s.len());
                let n = 1 + rng.below(60);
                let end = (at + n).min(s.len());
                let span: Vec<char> = s[at..end].to_vec();
                let to = rng.below(s.len() + 1);
                for (k, c) in span.into_iter().enumerate() {
                    s.insert(to + k, c);
                }
            }
            5 => {
                // replace a number literal with an extreme one
                let digits: Vec<usize> = (0..s.len()).filter(|&i| s[i].is_ascii_digit()).collect();
                if !digits.is_empty() {
                    let i = digits[rng.below(digits.len())];
                    let mut a = i;
                    let mut b = i;
                    while a > 0 && (s[a - 1].is_ascii_digit() || s[a - 1] == '.') {
                        a -= 1;
                    }
                    while b + 1 < s.len() && (s[b + 1].is_ascii_digit() || s[b + 1] == '.') {
                        b += 1;
                    }
                    let n: Vec<char> = rng.pick(NUMBERS).chars().collect();
                    s.splice(a..=b, n);
                }
            }
            6 => {
                // swap two lines
                let text: String = s.iter().collect();
                let mut lines: Vec<&str> = text.split('\n').collect();
                if lines.len() > 1 {
                    let (i, j) = (rng.below(lines.len()), rng.below(lines.len()));
                    lines.swap(i, j);
                }
                s = lines.join("\n").chars().collect();
            }
            7 => {
                // wrap a line in a block / call
                let text: String = s.iter().collect();
                let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
                let i = rng.below(lines.len());
                lines[i] = match rng.below(4) {
                    0 => format!("while true {{ {} }}", lines[i]),
                    1 => format!("(({}))", lines[i]),
                    2 => format!("fn zz() {{ {} }}\nzz()", lines[i]),
                    _ => format!("try {{ {} }} catch e {{ println(e) }}", lines[i]),
                };
                s = lines.join("\n").chars().collect();
            }
            8 => {
                // delete a whole line
                let text: String = s.iter().collect();
                let mut lines: Vec<&str> = text.split('\n').collect();
                let i = rng.below(lines.len());
                lines.remove(i);
                s = lines.join("\n").chars().collect();
            }
            _ => {
                // splice in a chunk of another region (cross-over with itself)
                let at = rng.below(s.len());
                let ins: Vec<char> = "\n[[[[{{{{((((\n".chars().collect();
                for (k, c) in ins.into_iter().enumerate() {
                    s.insert(at + k, c);
                }
            }
        }
    }
    s.into_iter().collect()
}

fn token_soup(rng: &mut Rng) -> String {
    let n = 1 + rng.below(60);
    let mut out = String::new();
    for _ in 0..n {
        out.push_str(rng.pick(TOKENS));
        out.push(if rng.below(5) == 0 { '\n' } else { ' ' });
    }
    out
}

fn fuzz_iterations(default: usize) -> usize {
    std::env::var("KILLER_FUZZ_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn fuzz_seed(default: u64) -> u64 {
    std::env::var("KILLER_FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn run_fuzz(iters: usize, seed: u64) {
    let seeds = load_seeds();
    assert!(seeds.len() > 10, "expected real .killer seeds, found {}", seeds.len());
    let mut rng = Rng::new(seed);
    let worker = Worker::new();
    let mut worker = worker;
    let mut ran_ok = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for i in 0..iters {
        let src = if rng.below(6) == 0 {
            token_soup(&mut rng)
        } else {
            let s = rng.pick(&seeds).clone();
            mutate(&mut rng, &s)
        };
        match worker.run(&src, true, Duration::from_secs(30)) {
            None => {
                let p = save_repro("hang", &src);
                failures.push(format!("case {i}: HANG (repro {})", p.display()));
                worker = Worker::new(); // the stuck thread is abandoned
            }
            Some(Outcome::Panic(m)) => {
                let p = save_repro("panic", &src);
                failures.push(format!("case {i}: PANIC {m} (repro {})", p.display()));
            }
            Some(Outcome::Ok) => ran_ok += 1,
            Some(Outcome::Err(_)) => {}
        }
        if failures.len() >= 5 {
            break;
        }
    }
    assert!(failures.is_empty(), "fuzzing found problems (seed {seed}):\n{}", failures.join("\n"));
    assert!(ran_ok > 0, "no fuzz case ran to completion; the harness is not exercising the VM");
}

#[test]
fn fuzz_default() {
    run_fuzz(fuzz_iterations(1500), fuzz_seed(0xC0FFEE));
}

#[test]
#[ignore = "long run: KILLER_FUZZ_ITERS=200000 cargo test --test robustness fuzz_long -- --ignored"]
fn fuzz_long() {
    run_fuzz(fuzz_iterations(100_000), fuzz_seed(1));
}

// ------------------------------------------------------------------------------------------------
// Stress inputs: hostile shapes of valid-looking programs
// ------------------------------------------------------------------------------------------------

fn assert_clean(name: &str, src: &str) {
    let w = Worker::new();
    match w.run(src, true, Duration::from_secs(90)) {
        None => {
            let p = save_repro(name, src);
            panic!("{name}: hang (repro {})", p.display());
        }
        Some(Outcome::Panic(m)) => {
            let p = save_repro(name, src);
            panic!("{name}: panic {m} (repro {})", p.display());
        }
        Some(_) => {}
    }
}

#[test]
fn deeply_nested_parentheses() {
    for n in [50, 500, 5_000, 50_000] {
        assert_clean("parens", &format!("x = {}1{}\nprintln(x)\n", "(".repeat(n), ")".repeat(n)));
    }
}

#[test]
fn deeply_nested_brackets() {
    for n in [50, 500, 5_000, 50_000] {
        assert_clean("brackets", &format!("x = {}1{}\nprintln(len(x))\n", "[".repeat(n), "]".repeat(n)));
        assert_clean("unclosed", &format!("x = {}\n", "[".repeat(n)));
    }
}

#[test]
fn deeply_nested_blocks_and_lambdas() {
    for n in [50, 500, 5_000, 30_000] {
        assert_clean("blocks", &format!("x = 0\n{}x = 1\n{}\n", "if true {\n".repeat(n), "}\n".repeat(n)));
        assert_clean("whiles", &format!("x = 0\n{}x = 1\n{}\n", "while x < 1 {\n".repeat(n), "}\n".repeat(n)));
        assert_clean("lambdas", &format!("f = {}1{}\n", "fn() { return ".repeat(n), " }".repeat(n)));
        assert_clean("match", &format!("x = 1\n{}println(1)\n{}\n", "match x {\n 1 => {\n".repeat(n), "}\n}\n".repeat(n)));
        assert_clean("dicts", &format!("x = {}1{}\n", "{\"a\": ".repeat(n), "}".repeat(n)));
        assert_clean("calls", &format!("x = {}1{}\n", "abs(".repeat(n), ")".repeat(n)));
        assert_clean("unary", &format!("x = {}1\n", "-".repeat(n)));
        assert_clean("not", &format!("x = {}true\n", "!".repeat(n)));
    }
}

#[test]
fn huge_literals_and_long_expressions() {
    let n = 100_000;
    let items: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    assert_clean("array100k", &format!("x = [{}]\nprintln(len(x))\n", items.join(", ")));
    let sum: Vec<String> = (0..n).map(|_| "1".to_string()).collect();
    assert_clean("sum100k", &format!("x = {}\nprintln(x)\n", sum.join(" + ")));
    assert_clean("longstring", &format!("x = \"{}\"\nprintln(len(x))\n", "a".repeat(2_000_000)));
    let stmts: Vec<String> = (0..20_000).map(|i| format!("v{} = {}", i % 50, i)).collect();
    assert_clean("stmts20k", &format!("{}\nprintln(v1)\n", stmts.join("\n")));
    let params: Vec<String> = (0..2000).map(|i| format!("p{i}")).collect();
    assert_clean("params", &format!("fn f({}) {{\n return p0\n}}\nprintln(1)\n", params.join(", ")));
}

#[test]
fn unclosed_and_garbage_input() {
    for src in [
        "\"abc", "'abc", "f\"{", "f\"{x", "/* never closed", "fn", "fn (", "class", "class A {", "match", "match x {",
        "x = ", "x = [", "x = {", "if", "if true {", "try {", "for", "for i in", "for i in range(", "}}}}", ")))))",
        "\\", "\u{0}", "\u{feff}\u{feff}", "x = \"\\u{110000}\"", "x = \"\\u{zz}\"", "@", "$$$", "`", "1e", "1e+", "0x", "0xzz",
        "1..", "x.", "x?.", "x[", "x[1", "-", "!", "yield", "throw", "return", "import", "import \"", "import \"nope_nope\"",
        "x = 1 2 3", "fn f( {", "fn f() => ", "a ? b :", "a ?? ", "😀 = 1", "x = \"😀\" * 3\nprintln(x)",
    ] {
        assert_clean("garbage", src);
    }
}

// ------------------------------------------------------------------------------------------------
// Regression tests for specific crashes and hangs
// ------------------------------------------------------------------------------------------------

fn expect_err(src: &str, needle: &str) {
    match run_big(src) {
        Outcome::Err(m) => assert!(m.contains(needle), "expected error containing {needle:?}, got {m:?}"),
        other => panic!("expected an error containing {needle:?}, got {other:?}"),
    }
}

fn expect_ok(src: &str) {
    match run_big(src) {
        Outcome::Ok => {}
        other => panic!("expected success, got {other:?}"),
    }
}

#[test]
fn unbounded_recursion_is_a_catchable_error() {
    expect_err("fn f(n) {\n  return f(n + 1)\n}\nf(0)\n", "stack overflow: recursion too deep");
    expect_ok(
        "fn f(n) {\n  return f(n + 1)\n}\ntry {\n  f(0)\n} catch e {\n  println(\"caught: \" + e)\n}\nprintln(\"after\")\n",
    );
}

#[test]
fn deep_but_finite_recursion_still_works() {
    expect_ok(
        "fn d(n) {\n  if n == 0 {\n    return 0\n  }\n  return 1 + d(n - 1)\n}\nassert(d(5000) == 5000)\nassert(d(10000) == 10000)\n",
    );
}

#[test]
fn recursion_through_callbacks_operators_and_tostring() {
    expect_ok(
        "fn f(n) {\n  return map([1], fn(x) {\n    return f(n + 1)\n  })\n}\ntry {\n  f(0)\n} catch e {\n  println(e)\n}\n",
    );
    expect_ok(
        "class V {\n  fn init(x) {\n    this.x = x\n  }\n  fn __add__(o) {\n    return this + o\n  }\n}\na = new V(1)\ntry {\n  b = a + a\n} catch e {\n  println(e)\n}\n",
    );
    expect_ok(
        "class A {\n  fn init() {\n    this.v = 1\n  }\n  fn toString() {\n    return \"a\" + str(this)\n  }\n}\na = new A()\ntry {\n  println(str(a))\n} catch e {\n  println(e)\n}\n",
    );
}

#[test]
fn recursive_generators_end_with_an_error() {
    expect_ok(
        "fn g(n) {\n  yield n\n  for v in g(n + 1) {\n    yield v\n  }\n}\ntry {\n  c = 0\n  for v in g(0) {\n    c = c + 1\n  }\n} catch e {\n  println(e)\n}\n",
    );
}

#[test]
fn giant_allocations_are_refused() {
    expect_err("x = \"x\" * 1000000000\n", "too large");
    expect_err("x = [0] * 1000000000\n", "too large");
    expect_err("x = range(1000000000000)\n", "range is too large");
    expect_err("for i in range(1000000000000) {\n  break\n}\n", "range is too large");
    expect_err("x = range(0, 9223372036854775807, 1)\n", "range is too large");
    expect_err("s = \"a\"\nfor i in range(40) {\n  s = s + s\n}\n", "string is too large");
    expect_err("s = \"a\"\nfor i in range(40) {\n  s = s * 2\n}\n", "too large");
    expect_err("a = [1]\nfor i in range(40) {\n  a = a + a\n}\n", "too large");
}

#[test]
fn range_edge_cases_do_not_overflow() {
    expect_ok("x = range(9007199254740990, 9007199254740992)\nassert(len(x) == 2)\n");
    expect_ok("x = range(0, 10, 3)\nassert(len(x) == 4)\ny = range(10, 0, -3)\nassert(len(y) == 4)\nassert(len(range(5, 5)) == 0)\n");
    expect_ok("x = range(-9223372036854775807, 9223372036854775807, 4611686018427387904)\nassert(len(x) == 4)\n");
}

#[test]
fn self_referencing_containers_do_not_overflow_the_stack() {
    expect_ok("a = []\npush(a, a)\nprintln(a)\ns = str(a)\nassert(len(s) > 0)\n");
    expect_ok("d = {}\nd[\"self\"] = d\nprintln(d)\nprintln(str(d))\n");
    expect_err("d = {}\nd[\"self\"] = d\nprintln(json_stringify(d))\n", "nested too deeply");
}

#[test]
fn exponentially_shared_structures_stay_bounded() {
    // 2^40 leaves if expanded: printing and serialising must stop with a bounded result
    expect_err("x = []\nfor i in range(40) {\n  x = [x, x]\n}\njson_stringify(x)\n", "too");
    expect_ok("x = []\nfor i in range(40) {\n  x = [x, x]\n}\ns = str(x)\nassert(len(s) > 0)\n");
}

#[test]
fn json_parse_of_hostile_input_is_an_error() {
    expect_ok(&format!(
        "try {{\n  v = json_parse(\"{}{}\")\n  println(\"parsed\")\n}} catch e {{\n  println(e)\n}}\n",
        "[".repeat(200_000),
        "]".repeat(200_000)
    ));
    expect_ok("try {\n  json_parse(\"{\\\"a\\\": \")\n} catch e {\n  println(e)\n}\ntry {\n  json_parse(\"[1,,2]\")\n} catch e {\n  println(e)\n}\n");
}

#[test]
fn catastrophic_regex_terminates() {
    expect_ok(&format!(
        "s = \"{}!\"\ntry {{\n  println(regex_match(\"(a+)+$\", s))\n}} catch e {{\n  println(e)\n}}\n",
        "a".repeat(64)
    ));
    expect_ok(&format!(
        "s = \"{}\"\ntry {{\n  println(regex_match(\"(a|aa)+$\", s + \"b\"))\n  println(regex_match(\"(.*)*x\", s))\n}} catch e {{\n  println(e)\n}}\n",
        "a".repeat(64)
    ));
}

#[test]
fn string_building_loops_are_bounded() {
    expect_ok("s = \"\"\nfor i in range(200000) {\n  s = s + \"ab\"\n}\nassert(len(s) == 400000)\n");
    expect_ok("parts = []\nfor i in range(100000) {\n  push(parts, str(i))\n}\ns = join(parts, \",\")\nassert(len(s) > 100000)\n");
}

/// Fuzz seed 1 case 7191: a comparison chain with an unterminated string was rewritten to text that
/// split as a chain again, growing forever. It must now fail to compile, promptly.
#[test]
fn comparison_chain_with_unterminated_string_does_not_hang() {
    for src in [
        "data = \"region,product,uniprintln(\"== revenue by region ==\")\n",
        "x = \"a(\"== b ==\")\n",
        "x = a == b == \"c\n",
        "x = (a < b < c\n",
    ] {
        let (tx, rx) = std::sync::mpsc::channel();
        let owned = src.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(killer_native::compiler::compile_killer_subset(&owned).is_ok());
        });
        let ok = rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap_or_else(|_| panic!("compiler hung on {src:?}"));
        assert!(!ok, "{src:?} should be a compile error");
    }
    // a real chain still works
    assert!(killer_native::compiler::compile_killer_subset("a = 1\nb = 2\nc = 3\nprintln(a < b < c)\nprintln(a == b == c)\n").is_ok());
}

/// Fuzz seed 1 case 11928: unbounded recursion that reads a global at every level got quadratically
/// slower (name lookup walks the scope chain) and outlived its wall-clock budget, which was only
/// checked on loop back-edges. The budget must now stop it.
#[test]
fn wall_budget_stops_deep_recursion_that_reads_globals() {
    let src = "g = 5\nfn rec(n) {\n  x = g\n  return 1 + rec(n + 1)\n}\nprintln(rec(0))\n";
    let (tx, rx) = std::sync::mpsc::channel();
    let owned = src.to_string();
    std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(move || {
        let caps = CapabilitySet { allow_native_jit: true, ..CapabilitySet::restricted() };
        let _g = CapabilityScopeGuard::install(caps);
        let p = compile_killer_subset(&owned).unwrap();
        let mut vm = VirtualMachine::new();
        vm.set_execution_budget(Some(ExecutionBudget { max_instruction_steps: 0, max_wall_ms: 1_000, max_heap_bytes: 0 }));
        let _ = tx.send(vm.run(&p).map_err(|e| e.to_string()));
    }).unwrap();
    let r = rx.recv_timeout(Duration::from_secs(60)).expect("budgeted recursion did not stop");
    let msg = r.expect_err("must hit a limit");
    assert!(msg.contains("time limit") || msg.contains("recursion too deep"), "{msg}");
}

// ------------------------------------------------------------------------------------------------
// GC over deep or cyclic dict structures
// ------------------------------------------------------------------------------------------------

#[test]
fn gc_does_not_go_quadratic_on_a_deep_tree_held_by_every_frame() {
    // a left-deep tree of 30000 dicts walked by a recursive function (one frame holds each node)
    // while arrays are allocated, so collections run in the middle: the mark phase used to walk the
    // shared tree again for every frame (minutes); it must be linear.
    let src = r#"
fn walk(node) {
  if node == null {
    return 0
  }
  junk = [1, 2]
  more = [3]
  return walk(node["l"]) + 1
}
x = null
i = 0
while i < 30000 {
  x = {"l": x}
  i = i + 1
}
total = 0
rep = 0
while rep < 4 {
  total = total + walk(x)
  rep = rep + 1
}
assert(total == 120000)
"#;
    let t = std::time::Instant::now();
    expect_ok(src);
    assert!(t.elapsed() < Duration::from_secs(60), "took {:?}", t.elapsed());
}

#[test]
fn gc_terminates_with_cyclic_dicts_and_objects_and_closures() {
    expect_ok(
        r#"
d = {}
d["self"] = d
class N {
  fn init() {
    this.me = this
  }
}
n = new N()
fn mk() {
  c = 0
  fn inc() {
    c = c + 1
    return c
  }
  return inc
}
f = mk()
f()
i = 0
while i < 200000 {
  a = [i]
  i = i + 1
}
assert(f() == 2)
"#,
    );
}

// ------------------------------------------------------------------------------------------------
// Compile time stays near-linear for huge but legal inputs
// ------------------------------------------------------------------------------------------------

fn expect_ok_within(label: &str, src: &str, secs: u64) {
    let t = std::time::Instant::now();
    expect_ok(src);
    assert!(t.elapsed() < Duration::from_secs(secs), "{label}: took {:?}", t.elapsed());
}

#[test]
fn twenty_thousand_statements_compile_quickly() {
    let mut src = String::new();
    for i in 0..20000 {
        src.push_str(&format!("x{} = {} + 1\n", i % 50, i));
    }
    src.push_str("assert(x1 == 19952)\n");
    expect_ok_within("20000 statements", &src, 60);
}

#[test]
fn long_expressions_within_the_limit_compile_quickly() {
    // 900 terms: under the 1000-operator limit, so it must compile (and fast)
    let plus = vec!["1"; 900].join(" + ");
    expect_ok_within("900-term sum", &format!("x = {plus}\nassert(x == 900)\n"), 30);
    let logical = vec!["true"; 900].join(" and ");
    expect_ok_within("900-term and chain", &format!("x = {logical}\nassert(x)\n"), 30);
    // beyond the limit: a clear error, quickly, for every spelling
    for join in [" + ", " and ", " or ", " == ", " ?? "] {
        let t = std::time::Instant::now();
        expect_err(&format!("x = {}\n", vec!["1"; 30000].join(join)), "expression is too long");
        assert!(t.elapsed() < Duration::from_secs(30), "{join}: {:?}", t.elapsed());
    }
}

#[test]
fn very_long_else_if_chains_and_function_bodies_compile_linearly() {
    let mut src = String::from("x = 77\nr = 0\nif x == 0 {\n  r = 0\n}");
    for i in 1..20000 {
        src.push_str(&format!(" else if x == {i} {{\n  r = {i}\n}}"));
    }
    src.push_str("\nassert(r == 77)\n");
    expect_ok_within("20000 else-if branches", &src, 60);
    // 3000 operators spread over the lines of one function body are not one expression
    let mut f = String::from("fn f(a) {\n");
    for i in 0..3000 {
        f.push_str(&format!("  a = a + {i}\n"));
    }
    f.push_str("  return a\n}\nassert(f(0) == 4498500)\n");
    expect_ok_within("3000-line function", &f, 60);
    // many functions and long literals
    let mut g = String::new();
    for i in 0..5000 {
        g.push_str(&format!("fn g{i}(a) {{\n  return a + {i}\n}}\n"));
    }
    g.push_str(&format!("xs = [{}]\nassert(len(xs) == 20000)\nassert(g4999(1) == 5000)\n", (0..20000).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")));
    expect_ok_within("5000 functions + 20000-element literal", &g, 60);
}

#[test]
fn endless_postfix_chains_and_many_classes_and_lambdas_are_fast() {
    for chain in [".a".repeat(30000), "[0]".repeat(30000), ".upper()".repeat(30000)] {
        let t = std::time::Instant::now();
        expect_err(&format!("d = [1]\nx = d{chain}\n"), "expression is too long");
        assert!(t.elapsed() < Duration::from_secs(30), "{:?}", t.elapsed());
    }
    // a realistic chain still works
    expect_ok("s = \"ab\"\nx = s.upper().lower().upper().lower().upper()\nassert(x == \"AB\")\n");
    let mut classes = String::new();
    for i in 0..3000 {
        classes.push_str(&format!("class C{i} {{\n  fn init() {{\n    this.a = {i}\n  }}\n}}\n"));
    }
    classes.push_str("o = new C2999()\nassert(o.a == 2999)\n");
    expect_ok_within("3000 classes", &classes, 60);
    let mut lambdas = String::new();
    for i in 0..4000 {
        lambdas.push_str(&format!("f{i} = fn(a) {{ return a + {i} }}\n"));
    }
    lambdas.push_str("assert(f3999(1) == 4000)\nassert(f0(1) == 1)\n");
    expect_ok_within("4000 lambdas", &lambdas, 60);
    // a literal spread over 20000 lines, and a call with 20000 argument lines
    let mut lit = String::from("x = [\n");
    for i in 0..20000 {
        lit.push_str(&format!("  {i},\n"));
    }
    lit.push_str("  0\n]\nassert(len(x) == 20001)\n");
    expect_ok_within("20000-line array literal", &lit, 60);
    let mut call = String::from("fn f(*a) {\n  return len(a)\n}\nn = f(\n");
    for i in 0..20000 {
        call.push_str(&format!("  {i},\n"));
    }
    call.push_str("  0\n)\nassert(n == 20001)\n");
    expect_ok_within("20000-line call", &call, 60);
}

// ------------------------------------------------------------------------------------------------
// Deeply nested user data: every walk over a value must be iterative or depth-limited
// ------------------------------------------------------------------------------------------------

/// Run `src` on a thread with a deliberately small native stack (8 MB, like an ordinary main
/// thread); a recursive walk over a 100,000-deep value overflows it and kills the whole process.
fn run_small_stack(src: &str) -> Outcome {
    let (tx, rx) = channel::<Outcome>();
    let src = src.to_string();
    std::thread::Builder::new()
        .name("small-stack".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let _ = tx.send(execute(&src, None));
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(300)).unwrap_or_else(|_| panic!("hang on a deep structure"))
}

fn deep_build(kind: &str, depth: usize) -> String {
    match kind {
        "list" => format!("a = []\nb = []\ni = 0\nwhile i < {depth} {{\n  a = [a]\n  b = [b]\n  i = i + 1\n}}\n"),
        "dict" => format!("a = {{}}\nb = {{}}\ni = 0\nwhile i < {depth} {{\n  a = {{\"k\": a}}\n  b = {{\"k\": b}}\n  i = i + 1\n}}\n"),
        _ => format!(
            "class N {{\n  fn init(next) {{\n    this.next = next\n  }}\n}}\na = null\nb = null\ni = 0\nwhile i < {depth} {{\n  a = new N(a)\n  b = new N(b)\n  i = i + 1\n}}\n"
        ),
    }
}

#[test]
fn deep_values_never_overflow_the_native_stack() {
    // every operation either works or raises a catchable Killer error; the process must survive
    let ops = [
        "r = a == b\nassert(r)",
        "r = a != b\nassert(not r)",
        "r = [a] == [b]\nassert(r)",
        "r = a in [b]\nassert(r)",
        "r = contains([b], a)\nassert(r)",
        "s = str(a)\nassert(len(s) > 0)",
        "s = \"\" + str(b)\nassert(len(s) > 0)",
        "println(\"\")",
        "try { s = json_stringify(a) } catch e { assert(contains(str(e), \"nested too deeply\")) }",
        "h = spawn_thread(a)",
        "s = set()\nset_add(s, 1)",
        "x = sorted([[a], [b]])",
        "x = copy([a, b])",
        "x = [a, b, a, b]\nx = null",
        // enough allocation to run the collector with the deep values alive
        "k = 0\nwhile k < 120000 {\n  t = [k]\n  k = k + 1\n}",
        "a = null\nb = null",
    ];
    for kind in ["list", "dict", "obj"] {
        let mut src = deep_build(kind, 100_000);
        for op in ops {
            if kind != "list" && op.contains("sorted") {
                continue;
            }
            if kind == "obj" && op.contains("copy(") {
                continue;
            }
            src.push_str(&format!("try {{\n{op}\n}} catch err {{\n  println(\"caught\", str(err)[0:60])\n}}\n"));
        }
        src.push_str("println(\"survived\")\n");
        match run_small_stack(&src) {
            Outcome::Ok => {}
            other => panic!("{kind}: {other:?}"),
        }
    }
}

#[test]
fn cyclic_values_compare_without_hanging() {
    expect_ok("a = []\npush(a, a)\nb = []\npush(b, b)\nassert(a == b)\nassert(a == a)\nx = [1]\npush(x, a)\ny = [1]\npush(y, b)\nassert(x == y)\n");
    expect_ok("d = {}\nd[\"self\"] = d\ne = {}\ne[\"self\"] = e\nassert(d == e)\nassert(sorted([a_row(), a_row()]) != null)\nfn a_row() {\n  r = [1]\n  push(r, r)\n  return r\n}\n");
    // exponentially shared structure: compared in linear time
    expect_ok("x = []\ny = []\ni = 0\nwhile i < 60 {\n  x = [x, x]\n  y = [y, y]\n  i = i + 1\n}\nassert(x == y)\n");
}
