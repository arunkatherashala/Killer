//! Shared helpers for the hostile-input tests (run the real `killer` binary with a watchdog).
#![allow(dead_code)]
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Outcome {
    pub timed_out: bool,
    pub code: Option<i32>,
    pub text: String,
}

/// Run `killer <args..>` in `cwd`, kill it after `limit`.
pub fn run_killer(args: &[&std::ffi::OsStr], cwd: &std::path::Path, limit: Duration) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer"))
        .args(args)
        .current_dir(cwd)
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
    Outcome { timed_out, code, text }
}

/// Why this outcome counts as a crash, if it does.
pub fn crash_reason(o: &Outcome) -> Option<String> {
    if o.timed_out {
        return Some("hung (watchdog timeout)".to_string());
    }
    for marker in ["panicked at", "has overflowed its stack", "memory allocation of", "RUST_BACKTRACE"] {
        if o.text.contains(marker) {
            let line = o.text.lines().find(|l| l.contains(marker)).unwrap_or(marker);
            return Some(line.chars().take(200).collect());
        }
    }
    match o.code {
        Some(0) | Some(1) | Some(2) => None,
        other => Some(format!("abnormal exit {:?}", other)),
    }
}

pub fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("killer_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}
