//! Builtins whose result can be far larger than their arguments (a separator joined between many
//! items, a long replacement, splitting into characters, overlapping chunks, encodings, ...) must
//! check the result size BEFORE they build it: against the fixed limits (256 MB strings, 2^25
//! elements per list, 1 GB byte buffers) and against an active `eval` memory cap.
//!
//! The cap tests measure the peak number of bytes the calling thread had allocated while the
//! builtin ran. Without a pre-check the builtin builds the whole result first and the cap only
//! notices afterwards (peak far above the cap); with it the request fails up front.
use killer_native::builtin::BuiltinFunctions;
use killer_native::memcap::CountingAlloc;
use killer_native::value::{SharedDict, Value};
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::collections::HashMap;

mod common;

/// The library's counting allocator (so the eval cap works) plus a per-thread peak tracker.
struct PeakAlloc;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static LIVE: Cell<i64> = const { Cell::new(0) };
    static PEAK: Cell<i64> = const { Cell::new(0) };
}

#[inline]
fn bump(delta: i64) {
    let _ = TRACK.try_with(|t| {
        if t.get() {
            let _ = LIVE.try_with(|l| {
                let v = l.get() + delta;
                l.set(v);
                let _ = PEAK.try_with(|p| {
                    if v > p.get() {
                        p.set(v)
                    }
                });
            });
        }
    });
}

unsafe impl GlobalAlloc for PeakAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = CountingAlloc.alloc(layout);
        if !p.is_null() {
            bump(layout.size() as i64);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = CountingAlloc.alloc_zeroed(layout);
        if !p.is_null() {
            bump(layout.size() as i64);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CountingAlloc.dealloc(ptr, layout);
        bump(-(layout.size() as i64));
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = CountingAlloc.realloc(ptr, layout, new_size);
        if !p.is_null() {
            bump(new_size as i64 - layout.size() as i64);
        }
        p
    }
}

#[global_allocator]
static ALLOC: PeakAlloc = PeakAlloc;

const MB: i64 = 1024 * 1024;

fn opts(mb: i64) -> Value {
    Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Integer(mb))])))
}

/// Evaluate `code` with a memory cap of `mb` MB; (result, peak MB allocated on this thread meanwhile).
fn measured(code: &str, mb: Option<i64>) -> (Result<String, String>, i64) {
    let mut args = vec![Value::Str(code.to_string()), Value::Null];
    if let Some(mb) = mb {
        args.push(opts(mb));
    }
    LIVE.with(|l| l.set(0));
    PEAK.with(|p| p.set(0));
    TRACK.with(|t| t.set(true));
    let r = BuiltinFunctions::call("eval", &args).map(|v| format!("{v}")).map_err(|e| format!("{e}"));
    TRACK.with(|t| t.set(false));
    (r, PEAK.with(|p| p.get()) / MB)
}

/// Run on a thread with a large stack, like the interpreter's own main thread.
fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

/// `code` must fail with the memory-limit error of a `cap` MB eval without first allocating far
/// more than the cap (`input_mb`: what the code legitimately holds before the builtin runs).
fn refused_up_front(code: &'static str, cap: i64, input_mb: i64) {
    big(move || {
        let (r, peak) = measured(code, Some(cap));
        assert!(
            matches!(&r, Err(m) if m.contains("memory limit exceeded") || m.contains("too large")),
            "{code}: expected a size error, got {r:?}"
        );
        assert!(peak <= cap + input_mb + 8, "{code}: allocated {peak} MB under a {cap} MB cap before failing");
    });
}

/// `code` must fail with a "too large" error without a cap (the fixed limits).
fn too_large(code: &'static str) {
    big(move || {
        let (r, peak) = measured(code, Some(2048));
        assert!(matches!(&r, Err(m) if m.contains("too large")), "{code}: expected a too-large error, got {r:?}");
        assert!(peak < 200, "{code}: allocated {peak} MB before failing");
    });
}

// ---------------------------------------------------------------- strings

#[test]
fn join_with_a_huge_separator_is_checked_first() {
    refused_up_front("len(join(range(80), 'x' * 1000000))", 16, 1);
    refused_up_front("len(range(80).join('x' * 1000000))", 16, 1);
}

#[test]
fn join_over_the_string_limit_is_refused() {
    too_large("len(join(range(300), 'x' * 1000000))");
}

#[test]
fn replace_with_a_longer_text_is_checked_first() {
    refused_up_front("len(replace('a' * 8000, 'a', 'x' * 10000))", 16, 1);
    refused_up_front("len(('a' * 8000).replace('a', 'x' * 10000))", 16, 1);
    refused_up_front("len(replace('a' * 8000, '', 'x' * 10000))", 16, 1);
}

#[test]
fn replace_over_the_string_limit_is_refused() {
    too_large("len(replace('a' * 30000, 'a', 'x' * 10000))");
}

#[test]
fn regex_replace_with_a_longer_text_is_checked_first() {
    refused_up_front("len(regex_replace('a' * 8000, 'a', 'x' * 10000))", 16, 1);
}

#[test]
fn hex_and_base64_encodings_are_checked_first() {
    refused_up_front("len(hex_encode('a' * 20000000))", 32, 20);
    refused_up_front("len(b64_encode('a' * 30000000))", 36, 30);
    refused_up_front("len(base64_encode('a' * 30000000))", 36, 30);
}

#[test]
fn bytes_concat_is_checked_first() {
    refused_up_front("len(bytes_concat(bytes_new(25000000), bytes_new(25000000)))", 64, 50);
}

// ---------------------------------------------------------------- lists

#[test]
fn split_into_many_pieces_is_checked_while_building() {
    refused_up_front("len(split('a' * 2000000, ''))", 16, 2);
    refused_up_front("len(('a' * 2000000).split(''))", 16, 2);
    refused_up_front("len(split('a,' * 2000000, ','))", 16, 4);
    refused_up_front("len(split('a ' * 2000000))", 16, 4);
}

#[test]
fn splitlines_list_and_regex_find_all_are_checked_while_building() {
    refused_up_front("len(splitlines('\\n' * 3000000))", 16, 3);
    refused_up_front("len(list('a' * 3000000))", 16, 3);
    refused_up_front("len(regex_find_all('a' * 3000000, 'a'))", 16, 15);
}

#[test]
fn overlapping_chunks_are_checked_while_building() {
    // ~100k windows of ~1000 characters each from a 100 KB text: about 100 MB of output
    refused_up_front("len(chunk_text('a ' * 50000, 1000, 999))", 16, 1);
}

#[test]
fn modest_results_still_work() {
    big(|| {
        for (code, want) in [
            ("len(join(range(10), '-'))", "19"),
            ("len(replace('abc' * 1000, 'b', 'xyz'))", "5000"),
            ("len(split('a' * 1000, ''))", "1000"),
            ("len(split('a b c'))", "3"),
            ("len(splitlines('a\\nb\\nc'))", "3"),
            ("len(list('hello'))", "5"),
            ("len(regex_find_all('a1b22c333', '[0-9]+'))", "3"),
            ("regex_replace('a1b2', '[0-9]', '#')", "a#b#"),
            ("len(chunk_text('one two three four five six', 10, 3))", "4"),
            ("hex_encode('hi')", "6869"),
            ("len(bytes_concat(bytes_new(3), bytes_new(4)))", "7"),
        ] {
            let (r, _) = measured(code, Some(64));
            assert_eq!(r.as_deref(), Ok(want), "{code}");
        }
    });
}

// ---------------------------------------------------------------- recursion

#[test]
fn deeply_nested_math_eval_is_an_error_not_a_stack_overflow() {
    let dir = common::scratch_dir("size_caps_math");
    let script = dir.join("deep.killer");
    std::fs::write(
        &script,
        "try {\n  math_eval(\"(\" * 1000000 + \"1\" + \")\" * 1000000)\n  println(\"no error\")\n} catch e {\n  println(\"caught\")\n}\ntry {\n  math_eval(\"-\" * 1000000 + \"1\")\n} catch e {\n  println(\"caught\")\n}\ntry {\n  math_eval(\"2\" + \"^2\" * 1000000)\n} catch e {\n  println(\"caught\")\n}\nprintln(math_eval(\"((1+2)*3)^2\"))\n",
    )
    .unwrap();
    let o = common::run_killer(&[script.as_os_str()], &dir, std::time::Duration::from_secs(120));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(common::crash_reason(&o).is_none(), "crashed: {:?}\n{}", common::crash_reason(&o), o.text);
    assert_eq!(o.text.matches("caught").count(), 3, "{}", o.text);
    assert!(o.text.contains("81"), "{}", o.text);
}
