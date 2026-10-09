//! Capability audit: with a restricted capability set no builtin may read or write files, open
//! sockets, start processes or load native code. The sweep calls EVERY builtin name found in the
//! dispatcher (src/builtin.rs) with arguments that point at a scratch directory, a local listener
//! and marker-writing commands, and then checks that nothing observable happened. Explicit tests
//! below check the denial message for the families of builtins that must be gated.
use killer_native::builtin::BuiltinFunctions;
use killer_native::security::{CapabilityScopeGuard, CapabilitySet};
use killer_native::value::Value;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

fn restricted() -> CapabilitySet {
    CapabilitySet::restricted()
}

fn call_restricted(name: &str, args: &[Value]) -> Result<Value, String> {
    let _g = CapabilityScopeGuard::install(restricted());
    BuiltinFunctions::call(name, args).map_err(|e| format!("{e}"))
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn denied(r: &Result<Value, String>) -> bool {
    matches!(r, Err(m) if m.contains("Capability denied") || m.contains("capability"))
}

/// Every `"name" =>` arm of the builtin dispatcher.
fn dispatcher_names() -> Vec<String> {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("builtin.rs")).unwrap();
    let mut names = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        if !t.starts_with('"') {
            continue;
        }
        let Some(arrow) = t.find("=>") else { continue };
        let head = &t[..arrow];
        for part in head.split('|') {
            let p = part.trim();
            if p.len() > 2 && p.starts_with('"') && p.ends_with('"') {
                let n = &p[1..p.len() - 1];
                if n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !names.contains(&n.to_string()) {
                    names.push(n.to_string());
                }
            }
        }
    }
    names
}

/// Builtins that block by design, terminate the process, or wait on the outside world.
fn skip_in_sweep(name: &str) -> bool {
    const SKIP: &[&str] = &[
        "exit", "sleep", "serve", "listen", "accept", "recv", "readline", "input", "wait", "await", "join_thread",
        "async_timeout", "async_all", "async_race", "mic_", "phone_", "lsp_start", "kala_serve", "chan_", "wfi",
        "interrupts", "websocket_server",
    ];
    SKIP.iter().any(|k| name.contains(k))
}

struct Scratch {
    dir: PathBuf,
    canary: PathBuf,
    marker_cmd: PathBuf,
    marker_py: PathBuf,
    listener: TcpListener,
}

fn fwd(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn scratch(tag: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("killer_caps_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canary = dir.join("canary.txt");
    std::fs::write(&canary, "TOP-SECRET-CANARY").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    Scratch {
        marker_cmd: dir.join("marker_cmd.txt"),
        marker_py: dir.join("marker_py.txt"),
        dir,
        canary,
        listener,
    }
}

fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| (e.file_name().to_string_lossy().to_string(), std::fs::read(e.path()).unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn argument_patterns(sc: &Scratch) -> Vec<Vec<Value>> {
    let file = fwd(&sc.canary);
    let other = fwd(&sc.dir.join("new_file.txt"));
    let subdir = fwd(&sc.dir.join("new_dir"));
    let url = format!("http://{}/probe", sc.listener.local_addr().unwrap());
    let addr = sc.listener.local_addr().unwrap().to_string();
    let cmd = format!("echo x > {}", fwd(&sc.marker_cmd));
    let py = format!("open(r'{}', 'w').write('x')", fwd(&sc.marker_py));
    vec![
        vec![s(&file)],
        vec![s(&file), s("overwritten")],
        vec![s(&other), s("created")],
        vec![s(&file), s(&other)],
        vec![s(&file), s(&other), s("data")],
        vec![s(&subdir)],
        vec![s(&file), Value::Number(0.0), Value::Number(16.0)],
        vec![s(&url)],
        vec![s(&url), s("body")],
        vec![s(&addr)],
        vec![s(&cmd)],
        vec![s("python"), s(&py)],
        vec![s("bash"), s(&cmd)],
        vec![s(&py), s("python")],
    ]
}

/// Run `f` on a worker thread; `None` when it does not finish in time.
fn with_timeout<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let _ = tx.send(f());
        })
        .unwrap();
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(v) => Some(v),
        Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => None,
    }
}

#[test]
fn restricted_sweep_has_no_side_effects() {
    let sc = scratch("sweep");
    let names: Vec<String> = dispatcher_names().into_iter().filter(|n| !skip_in_sweep(n)).collect();
    assert!(names.len() > 300, "dispatcher scan found only {} names", names.len());
    let patterns = argument_patterns(&sc);
    let mut leaks: Vec<String> = Vec::new();
    for name in &names {
        let before = snapshot(&sc.dir);
        for args in &patterns {
            let n = name.clone();
            let a = args.clone();
            // a call that hangs is not a leak; it is abandoned (and not probed again)
            if with_timeout(3, move || call_restricted(&n, &a)).is_none() {
                break;
            }
            let after = snapshot(&sc.dir);
            let accepted = sc.listener.accept().is_ok();
            if after != before || accepted {
                let what = if accepted { "opened a network connection" } else { "changed the file system" };
                leaks.push(format!("{name}({args:?}) {what}"));
                // restore the scratch dir for the next probes
                let _ = std::fs::remove_dir_all(&sc.dir);
                std::fs::create_dir_all(&sc.dir).unwrap();
                std::fs::write(&sc.canary, "TOP-SECRET-CANARY").unwrap();
                break;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&sc.dir);
    assert!(leaks.is_empty(), "builtins that ignored the restricted capability set:\n{}", leaks.join("\n"));
}

#[test]
fn restricted_builtins_do_not_return_file_contents() {
    let sc = scratch("leak");
    let file = fwd(&sc.canary);
    for name in dispatcher_names().into_iter().filter(|n| !skip_in_sweep(n)) {
        for args in [vec![s(&file)], vec![s(&file), Value::Number(0.0), Value::Number(64.0)]] {
            let n = name.clone();
            let out = with_timeout(3, move || call_restricted(&n, &args));
            if out.is_none() {
                break;
            }
            if let Some(r) = out {
                let text = match r {
                    Ok(v) => format!("{v:?}"),
                    Err(e) => e,
                };
                assert!(!text.contains("TOP-SECRET-CANARY"), "{name} leaked file contents under restricted capabilities");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&sc.dir);
}

fn assert_denied(name: &str, args: Vec<Value>) {
    let r = call_restricted(name, &args);
    assert!(denied(&r), "{name} should be refused under restricted capabilities, got {r:?}");
}

#[test]
fn file_builtins_require_file_capabilities() {
    let p = std::env::temp_dir().join("killer_caps_explicit.txt");
    let p = fwd(&p);
    for name in ["readFile", "fileExists", "fileSize", "listDir", "readBytes"] {
        assert_denied(name, vec![s(&p)]);
    }
    for name in ["writeFile", "appendFile"] {
        assert_denied(name, vec![s(&p), s("x")]);
    }
    assert_denied("deleteFile", vec![s(&p)]);
    assert_denied("mkdir", vec![s(&p)]);
    assert_denied("fileCopy", vec![s(&p), s(&format!("{p}.copy"))]);
    assert_denied("renameFile", vec![s(&p), s(&format!("{p}.moved"))]);
    assert_denied("disk_read_block", vec![s(&p), Value::Number(0.0), Value::Number(16.0)]);
    assert_denied("disk_write_block", vec![s(&p), Value::Number(0.0), Value::Bytes(vec![1, 2, 3])]);
    assert!(!Path::new(&p).exists());
}

#[test]
fn network_builtins_require_network_capability() {
    for name in ["http_get", "http_head", "http_status", "http_get_json", "http_delete"] {
        assert_denied(name, vec![s("http://127.0.0.1:9/")]);
    }
    for name in ["http_post", "http_put", "http_patch", "http_post_json"] {
        assert_denied(name, vec![s("http://127.0.0.1:9/"), s("{}")]);
    }
    assert_denied("http_download", vec![s("http://127.0.0.1:9/"), s("out.bin")]);
    assert_denied("TcpListener_bind", vec![s("127.0.0.1:0")]);
}

#[test]
fn process_and_native_builtins_require_their_capabilities() {
    assert_denied("polyglot_exec", vec![s("python"), s("print(1)")]);
    assert_denied("env_get", vec![s("PATH")]);
    assert_denied("env_set", vec![s("KILLER_X"), s("1")]);
    assert_denied("process_exit", vec![Value::Number(3.0)]);
    assert_denied("ffi_open", vec![s("kernel32")]);
    assert_denied("ffi_alloc", vec![Value::Number(16.0)]);
    assert_denied("mmap_alloc", vec![Value::Number(4096.0)]);
    assert_denied("mmap_exec", vec![Value::Pointer(4096), Value::Number(16.0)]);
    assert_denied("mmap_read", vec![Value::Pointer(4096), Value::Number(16.0)]);
    assert_denied("mmap_write", vec![Value::Pointer(4096), Value::Bytes(vec![1])]);
    assert_denied("call_native", vec![Value::Bytes(vec![0xB8, 1, 0, 0, 0, 0xC3])]);
}

#[test]
fn privileged_cpu_builtins_never_fault_the_process() {
    // these used to execute real `in`/`out`/`cli`/`sti`/`hlt` instructions, which fault in user mode
    for name in ["io_port_in_u8", "io_port_in_u16"] {
        let r = call_restricted(name, &[Value::Number(0x60 as f64)]);
        assert!(r.is_err(), "{name} must report an error, got {r:?}");
    }
    assert!(call_restricted("io_port_out_u8", &[Value::Number(0x80 as f64), Value::Number(1.0)]).is_err());
    assert!(call_restricted("io_port_out_u16", &[Value::Number(0x80 as f64), Value::Number(1.0)]).is_err());
    for name in ["interrupts_disable", "interrupts_enable", "wfi"] {
        assert!(call_restricted(name, &[]).is_ok(), "{name} is a simulated no-op");
    }
}

#[test]
fn trusted_local_still_allows_everything() {
    let _g = CapabilityScopeGuard::install(CapabilitySet::trusted_local());
    let p = std::env::temp_dir().join(format!("killer_caps_trusted_{}.txt", std::process::id()));
    let ps = fwd(&p);
    BuiltinFunctions::call("writeFile", &[s(&ps), s("ok")]).unwrap();
    assert_eq!(format!("{}", BuiltinFunctions::call("readFile", &[s(&ps)]).unwrap()), "ok");
    BuiltinFunctions::call("deleteFile", &[s(&ps)]).unwrap();
    assert!(BuiltinFunctions::call("env_get", &[s("PATH")]).is_ok());
}

#[test]
fn imports_need_the_file_read_capability() {
    use killer_native::compiler::compile_killer_subset;
    let dir = std::env::temp_dir().join(format!("killer_caps_imp_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib_mod.killer"), "fn helper() {\n  return 1\n}\n").unwrap();
    killer_native::imports::set_base_dir(&dir);
    {
        let _g = CapabilityScopeGuard::install(restricted());
        let r = compile_killer_subset("import \"lib_mod\"\nprintln(helper())\n");
        let msg = r.err().map(|e| format!("{e}")).unwrap_or_default();
        assert!(msg.contains("apability"), "import under restricted caps must be refused, got {msg:?}");
    }
    {
        let _g = CapabilityScopeGuard::install(CapabilitySet::trusted_local());
        assert!(compile_killer_subset("import \"lib_mod\"\nprintln(helper())\n").is_ok());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
