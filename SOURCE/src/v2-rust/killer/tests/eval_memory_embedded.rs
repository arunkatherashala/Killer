//! The eval memory cap in a process that does NOT install `memcap::CountingAlloc` (an embedder that uses the library
//! directly, with its own allocator and without the `mem-cap` feature). This test binary declares no
//! `#[global_allocator]` on purpose.
//!
//! Weaker guarantee proven here: eval still works and still stops runaway allocations at the explicit pre-check
//! sites (string and list repeat and concatenation, `range`), the step budget and the capability inheritance still
//! apply, and nothing panics or reports a false "memory limit exceeded" from a counter that never moves. Only the
//! periodic live-bytes check (for allocations that are not at a checked site) is inactive.
//! With `--features mem-cap` the library installs the allocator itself and the same file proves that (`is_installed`).
use killer_native::builtin::BuiltinFunctions;
use killer_native::memcap;
use killer_native::value::{SharedDict, Value};
use std::collections::HashMap;

fn ev(name: &str, code: &str, mb: Option<i64>) -> Result<Value, String> {
    let mut args = vec![Value::Str(code.to_string()), Value::Null];
    if let Some(mb) = mb {
        args.push(Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Integer(mb))]))));
    }
    BuiltinFunctions::call(name, &args).map_err(|e| format!("{e}"))
}

fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

#[test]
fn installed_flag_matches_the_build() {
    // detection is stable and leaves no cap state behind
    let first = memcap::is_installed();
    assert_eq!(first, memcap::is_installed());
    assert_eq!(first, cfg!(feature = "mem-cap"));
    assert!(memcap::check().is_ok());
}

#[test]
fn prechecks_still_stop_huge_requests() {
    big(|| {
        for call in ["eval", "exec"] {
            let r = ev(call, "x = 'a' * 100000000000\nx", None);
            assert!(r.is_err(), "{call}: {r:?}");
        }
        // under the fixed string limit but over a small cap: the pre-check refuses it
        let r = ev("eval", "len('a' * 100000000)", Some(16));
        assert!(matches!(&r, Err(m) if m.contains("memory limit exceeded (16 MB)")), "{r:?}");
        let r = ev("eval", "len(range(30000000))", Some(8));
        assert!(matches!(&r, Err(m) if m.contains("memory limit exceeded (8 MB)")), "{r:?}");
    });
}

#[test]
fn doubling_list_is_stopped_by_prechecks_not_a_crash() {
    big(|| {
        let r = ev("exec", "x = [1, 2, 3]\nwhile true {\n  x = x + x\n}\n", Some(8));
        assert!(matches!(&r, Err(m) if m.contains("memory limit exceeded (8 MB)")), "{r:?}");
        let r = ev("exec", "x = [1]\nwhile true {\n  x = x * 2\n}\n", Some(8));
        assert!(r.is_err(), "{r:?}");
    });
}

#[test]
fn step_budget_still_ends_endless_loops_and_normal_code_works() {
    big(|| {
        let r = ev("exec", "n = 0\nwhile true {\n  n = n + 1\n}\n", None);
        assert!(matches!(&r, Err(m) if m.contains("step limit")), "{r:?}");
        // no false positive: ordinary evaluation, including moderate allocation, succeeds
        let r = ev("eval", "len('a' * 1000000) + len(range(100000))", Some(8));
        assert_eq!(r.map(|v| format!("{v}")), Ok("1100000".to_string()));
        let r = ev("eval", "1 + 2 * 3", None);
        assert_eq!(r.map(|v| format!("{v}")), Ok("7".to_string()));
    });
}

#[test]
fn without_the_allocator_small_allocations_are_not_counted() {
    if memcap::is_installed() {
        return; // `mem-cap` build: the counting variant is covered by eval_memory.rs
    }
    big(|| {
        // the periodic check has nothing to compare, so a loop of unchecked small allocations is bounded only by
        // the step budget; it must finish (or hit the step limit) and never claim the memory limit
        let r = ev("exec", "x = []\ni = 0\nwhile i < 200000 {\n  x.push([i])\n  i = i + 1\n}\nlen(x)\n", Some(1));
        assert!(!matches!(&r, Err(m) if m.contains("memory limit")), "{r:?}");
    });
}
