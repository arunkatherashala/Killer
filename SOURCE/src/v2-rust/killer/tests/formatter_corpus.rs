//! The formatter must be idempotent and behaviour preserving on every real program we have.

use killer_native::formatter::Formatter;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().map_or(false, |x| x == "killer") {
            out.push(p);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for d in ["examples", "tests", "parity/realworld"] {
        collect(&root.join(d), &mut files);
    }
    files.sort();
    files
}

fn format(src: &str) -> String {
    Formatter::new().format(src).unwrap()
}

#[test]
fn corpus_is_found() {
    assert!(corpus().len() >= 60, "found {} files", corpus().len());
}

#[test]
fn formatting_is_idempotent_on_every_file() {
    let mut bad = Vec::new();
    for f in corpus() {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let once = format(&src);
        let twice = format(&once);
        if once != twice {
            bad.push(f.display().to_string());
        }
        // comment-only and string content must survive: every non-whitespace character is kept
        let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        assert_eq!(strip(&src), strip(&once), "non-whitespace content changed in {}", f.display());
    }
    assert!(bad.is_empty(), "not idempotent: {bad:?}");
}

#[test]
fn comments_and_strings_survive_byte_for_byte() {
    let src = "x = \"a   b  \"  # keep   this \n// and   this\n-- and  that\nfn f() {\n        y = 'it   is'\n  }\n";
    let out = format(src);
    for needle in ["\"a   b  \"", "# keep   this", "// and   this", "-- and  that", "'it   is'"] {
        assert!(out.contains(needle), "{needle:?} lost in {out:?}");
    }
    assert_eq!(out, "x = \"a   b  \"  # keep   this\n// and   this\n-- and  that\nfn f() {\n  y = 'it   is'\n}\n");
}

#[test]
fn reindents_brace_code_and_normalizes_blank_lines() {
    let src = "fn f(a) {\nif a > 1 {\nreturn [\n1,\n2\n]\n} else {\nreturn 0   \n}\n}\n\n\n\nprint(f(2))\n\n\n";
    let out = format(src);
    assert_eq!(
        out,
        "fn f(a) {\n  if a > 1 {\n    return [\n      1,\n      2\n    ]\n  } else {\n    return 0\n  }\n}\n\nprint(f(2))\n"
    );
}

#[test]
fn indentation_based_code_is_not_reindented() {
    let src = "if x > 1:\n    print(1)\n   \nelse:\n        print(2)\n";
    let out = format(src);
    assert_eq!(out, "if x > 1:\n    print(1)\n\nelse:\n        print(2)\n");
}

/// Run a program with a timeout; `None` when it did not finish (servers, prompts, ...).
fn run(file: &Path, cwd: &Path) -> Option<(bool, String, String)> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(file)
        .arg("--run")
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_secs(20) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    let out = child.wait_with_output().ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

#[test]
fn formatted_programs_behave_identically() {
    let files = corpus();
    let tmp = std::env::temp_dir().join(format!("killer_fmt_corpus_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let chunks: Vec<Vec<PathBuf>> = {
        let n = 6;
        let mut v = vec![Vec::new(); n];
        for (i, f) in files.into_iter().enumerate() {
            v[i % n].push(f);
        }
        v
    };
    let handles: Vec<_> = chunks
        .into_iter()
        .enumerate()
        .map(|(ci, chunk)| {
            let tmp = tmp.clone();
            std::thread::spawn(move || {
                let mut compared = 0usize;
                let mut bad = Vec::new();
                for f in chunk {
                    let Ok(src) = std::fs::read_to_string(&f) else { continue };
                    let formatted = format(&src);
                    if formatted == src {
                        continue;
                    }
                    let cwd = f.parent().unwrap().to_path_buf();
                    let Some(orig) = run(&f, &cwd) else { continue };
                    let copy = tmp.join(format!("c{ci}_{}", f.file_name().unwrap().to_string_lossy()));
                    std::fs::write(&copy, &formatted).unwrap();
                    let Some(new) = run(&copy, &cwd) else {
                        bad.push(format!("{} (formatted copy hangs)", f.display()));
                        continue;
                    };
                    compared += 1;
                    let same = |a: &(bool, String, String), b: &(bool, String, String)| a.0 == b.0 && a.1 == b.1;
                    if !same(&orig, &new) {
                        // allow nondeterministic programs: the original must be stable on a rerun
                        match run(&f, &cwd) {
                            Some(again) if same(&orig, &again) => {
                                bad.push(format!("{}\n--- original ---\n{}\n--- formatted ---\n{}", f.display(), orig.1, new.1))
                            }
                            _ => {}
                        }
                    }
                }
                (compared, bad)
            })
        })
        .collect();
    let mut compared = 0;
    let mut bad = Vec::new();
    for h in handles {
        let (c, b) = h.join().unwrap();
        compared += c;
        bad.extend(b);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    eprintln!("compared {compared} reformatted programs");
    assert!(bad.is_empty(), "behaviour changed:\n{}", bad.join("\n"));
}
