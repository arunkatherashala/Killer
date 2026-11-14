//! Safety properties of `eval(code[, vars])` / `exec(code[, vars])`: the evaluated text gets the caller's
//! capabilities (never more), cannot see the caller's variables, is bounded in depth and time, and its errors do not
//! reveal host paths.
use killer_native::builtin::BuiltinFunctions;
use killer_native::security::{CapabilityScopeGuard, CapabilitySet};
use killer_native::value::{SharedDict, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn ev(name: &str, code: &str) -> Result<Value, String> {
    BuiltinFunctions::call(name, &[s(code)]).map_err(|e| format!("{e}"))
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("killer_evalsafe_{}_{}", std::process::id(), tag));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn fwd(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Run on a thread with a large stack, like the interpreter's own main thread does (nested VMs are deep).
fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

fn is_denied(r: &Result<Value, String>) -> bool {
    matches!(r, Err(m) if m.contains("apability denied") || m.contains("apability"))
}

#[test]
fn eval_under_restricted_capabilities_cannot_read_write_or_spawn() {
    let d = scratch("caps");
    let secret = d.join("secret.txt");
    std::fs::write(&secret, "TOP-SECRET-CANARY").unwrap();
    let out = d.join("written.txt");
    let _ = std::fs::remove_file(&out);
    let (sp, op) = (fwd(&secret), fwd(&out));
    let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
    for call in ["eval", "exec"] {
        let r = ev(call, &format!("readFile(\"{}\")", sp));
        assert!(is_denied(&r), "{call} read: {r:?}");
        let r = ev(call, &format!("writeFile(\"{}\", \"x\")", op));
        assert!(is_denied(&r), "{call} write: {r:?}");
        let r = ev(call, "env_set(\"KILLER_EVAL_ESCAPE\", \"1\")");
        assert!(is_denied(&r), "{call} env_set: {r:?}");
        let r = ev(call, "http_get(\"http://127.0.0.1:9/\")");
        assert!(is_denied(&r), "{call} network: {r:?}");
        let r = ev(call, "polyglot_exec(\"python\", \"print(1)\")");
        assert!(is_denied(&r), "{call} process: {r:?}");
    }
    // nested: eval inside eval does not regain anything
    let r = ev("eval", &format!("eval('readFile(\"{}\")')", sp));
    assert!(is_denied(&r), "nested eval read: {r:?}");
    assert!(!out.exists());
    assert!(std::env::var("KILLER_EVAL_ESCAPE").is_err());
}

#[test]
fn eval_with_trusted_capabilities_still_works() {
    let d = scratch("trusted");
    let f = d.join("t.txt");
    std::fs::write(&f, "hello").unwrap();
    let _g = CapabilityScopeGuard::install(CapabilitySet::trusted_local());
    let r = ev("eval", &format!("readFile(\"{}\")", fwd(&f))).unwrap();
    assert_eq!(format!("{r}"), "hello");
}

#[test]
fn infinite_loop_inside_eval_is_stopped() {
    big(|| {
        let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
        let t = Instant::now();
        let r = ev("exec", "i = 0\nwhile true {\n  i = i + 1\n}\n");
        assert!(r.is_err(), "an endless loop must end with an error, got {r:?}");
        let m = r.unwrap_err();
        assert!(m.contains("limit exceeded"), "{m}");
        assert!(t.elapsed() < Duration::from_secs(60), "took {:?}", t.elapsed());
    });
}

#[test]
fn deep_recursion_inside_eval_is_an_error_not_a_crash() {
    big(|| {
        let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
        let r = ev("exec", "fn f(n) { return f(n + 1) }\nf(0)\n");
        assert!(r.is_err(), "{r:?}");
    });
}

#[test]
fn eval_nesting_is_limited_to_eight_levels() {
    big(|| {
        // wrap `1` in n nested eval("...") calls
        fn nest(n: usize) -> String {
            let mut code = "1".to_string();
            for _ in 0..n {
                let q = code.replace('\\', "\\\\").replace('"', "\\\"");
                code = format!("eval(\"{}\")", q);
            }
            code
        }
        // the outermost call is level 1; 8 levels of eval are allowed, 9 are not
        let ok = ev("eval", &nest(7)).map(|v| format!("{v}"));
        assert_eq!(ok, Ok("1".to_string()));
        let bad = ev("eval", &nest(8));
        assert!(matches!(&bad, Err(m) if m.contains("nested more than 8 levels")), "{bad:?}");
        // the depth counter is restored afterwards
        assert_eq!(ev("eval", "2 + 2").map(|v| format!("{v}")), Ok("4".to_string()));
    });
}

#[test]
fn eval_cannot_see_or_change_callers_variables() {
    // run a real script: the caller has `secret`, the evaluated text must not see it and cannot assign to it
    let program = killer_native::compiler::compile_killer_default(
        "secret = 41\n__a = eval(\"secret\")\n",
    );
    let program = program.expect("compile");
    let mut vm = killer_native::vm::VirtualMachine::new();
    let r = vm.run(&program);
    let msg = format!("{:?}", r);
    assert!(r.is_err() && (msg.contains("secret") || msg.contains("ndefined")), "evaluated text saw the caller's variable: {msg}");

    let program = killer_native::compiler::compile_killer_default("secret = 41\n__a = exec(\"secret = 99\")\n__b = secret\n").expect("compile");
    let mut vm = killer_native::vm::VirtualMachine::new();
    vm.run(&program).expect("run");
    let g = vm.get_globals();
    assert_eq!(format!("{}", g.get("__b").unwrap()), "41", "exec changed the caller's variable");
}

#[test]
fn only_the_passed_variables_are_visible() {
    let vars = Value::Dict(SharedDict::new(HashMap::from([("x".to_string(), Value::Integer(5))])));
    let r = BuiltinFunctions::call("eval", &[s("x * 2"), vars.clone()]).unwrap();
    assert_eq!(format!("{r}"), "10");
    let r = BuiltinFunctions::call("eval", &[s("y"), vars]);
    assert!(r.is_err());
    // functions and handles cannot be smuggled in through vars
    let bad = Value::Dict(SharedDict::new(HashMap::from([("1bad name".to_string(), Value::Integer(1))])));
    assert!(BuiltinFunctions::call("eval", &[s("1"), bad]).is_err());
}

#[test]
fn errors_do_not_leak_host_paths() {
    let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default().replace('\\', "/").to_lowercase();
    for code in ["1 +", "undefined_name_xyz", "1 / 0", "readFile(\"nope.txt\")", "[1,2][9]", "\"a\" - 1", "fn ("] {
        for call in ["eval", "exec"] {
            let m = match ev(call, code) {
                Ok(v) => format!("{v}"),
                Err(e) => e,
            };
            let low = m.replace('\\', "/").to_lowercase();
            assert!(!low.contains(".rs:") && !low.contains("/src/") && !low.contains("c:/") && !low.contains("/users/"), "{call}({code:?}) leaked a path: {m}");
            assert!(home.is_empty() || !low.contains(&home), "{call}({code:?}) leaked the home directory: {m}");
            assert!(!low.contains("panicked"), "{m}");
        }
    }
}

#[test]
fn the_docs_carry_a_warning_about_untrusted_text() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs");
    let lang = std::fs::read_to_string(root.join("LANGUAGE.md")).unwrap();
    let i = lang.find("**`eval` and `exec`.**").expect("eval section");
    let section = &lang[i..(i + 2500).min(lang.len())];
    assert!(section.to_lowercase().contains("warning") && section.contains("untrusted"), "no warning near the eval docs");
    let feat = std::fs::read_to_string(root.join("FEATURES.md")).unwrap();
    let row = feat.lines().find(|l| l.contains("`eval` / `exec`")).expect("FEATURES row");
    assert!(row.contains("untrusted"), "FEATURES row lacks the warning: {row}");
}
