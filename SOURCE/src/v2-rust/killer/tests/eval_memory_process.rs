//! Process-level checks of the eval memory cap: the environment variable `KILLER_EVAL_MAX_MEMORY`, and a spot check
//! that the host's peak memory stays bounded while evaluations try to allocate far more than the cap.
//! (Own test binary so that changing the environment cannot disturb other tests.)
use killer_native::builtin::BuiltinFunctions;
use killer_native::value::{SharedDict, Value};
use std::collections::HashMap;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn ev(code: &str, mb: Option<i64>) -> Result<Value, String> {
    let mut args = vec![s(code)];
    if let Some(mb) = mb {
        args.push(Value::Null);
        args.push(Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Integer(mb))]))));
    }
    BuiltinFunctions::call("eval", &args).map_err(|e| format!("{e}"))
}

fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

#[test]
fn the_environment_variable_sets_the_cap_and_opts_override_it() {
    let _l = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    big(|| {
        std::env::set_var("KILLER_EVAL_MAX_MEMORY", "8");
        let r = ev("len('a' * 20000000)", None);
        assert!(matches!(&r, Err(m) if m.contains("memory limit exceeded (8 MB)")), "{r:?}");
        // an explicit option wins over the variable
        let r = ev("len('a' * 20000000)", Some(64));
        assert_eq!(r.map(|v| format!("{v}")), Ok("20000000".to_string()));
        // bad values are errors, not silently ignored
        std::env::set_var("KILLER_EVAL_MAX_MEMORY", "99999");
        let r = ev("1", None);
        assert!(matches!(&r, Err(m) if m.contains("from 1 to 2048")), "{r:?}");
        std::env::set_var("KILLER_EVAL_MAX_MEMORY", "lots");
        assert!(ev("1", None).is_err());
        std::env::remove_var("KILLER_EVAL_MAX_MEMORY");
        assert_eq!(ev("1", None).map(|v| format!("{v}")), Ok("1".to_string()));
    });
}

/// Peak resident memory of this process in bytes, where the platform offers it.
fn peak_rss() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let st = std::fs::read_to_string("/proc/self/status").ok()?;
        let l = st.lines().find(|l| l.starts_with("VmHWM:"))?;
        let kb: u64 = l.split_whitespace().nth(1)?.parse().ok()?;
        return Some(kb * 1024);
    }
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Counters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set: usize,
            working_set: usize,
            rest: [usize; 6],
        }
        extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(p: isize, c: *mut Counters, cb: u32) -> i32;
        }
        let mut c = Counters { cb: std::mem::size_of::<Counters>() as u32, page_fault_count: 0, peak_working_set: 0, working_set: 0, rest: [0; 6] };
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
        return if ok != 0 { Some(c.peak_working_set as u64) } else { None };
    }
    #[allow(unreachable_code)]
    None
}

#[test]
fn host_memory_stays_bounded_while_evaluations_exceed_the_cap() {
    let _l = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    big(|| {
        let before = peak_rss();
        for _ in 0..4 {
            assert!(ev("x = [1, 2, 3]\nwhile true {\n  x = x + x\n}\n", Some(64)).is_err());
            assert!(ev("len('a' * 200000000)", Some(64)).is_err());
            assert!(ev("len(range(30000000))", Some(64)).is_err());
        }
        if let (Some(b), Some(a)) = (before, peak_rss()) {
            // each run is allowed 64 MB (plus a growth step and the VM itself); with the cap broken the process
            // would reach several GB (a 200 MB string, a 30M-element list, an endless doubling list)
            let growth = a.saturating_sub(b);
            assert!(growth < 400 * 1024 * 1024, "peak memory grew by {} MB", growth / (1024 * 1024));
            eprintln!("peak memory growth: {} MB (cap 64 MB)", growth / (1024 * 1024));
        }
    });
}
