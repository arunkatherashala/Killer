//! `@lang { ... }` polyglot blocks: the block text reaches the foreign runtime verbatim, limits,
//! environment scrub, runtime resolution, error codes, policy, audit log, CLI and variable passing.
//!
//! Every test that needs a foreign runtime first checks that the runtime really runs a one-line
//! block on this machine and prints a `skip:` line otherwise (CI has python3 and bash; java, ruby,
//! node and go may be missing). Each test runs the `killer` binary as a child process with its own
//! environment, so tests never change the environment of this test process.
use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn unique(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("killer_pgt_{}_{}_{}", tag, std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)))
}

struct Out {
    stdout: String,
    stderr: String,
    code: i32,
}

impl Out {
    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

fn norm(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n").trim().to_string()
}

/// Run `src` with `killer run`, with extra environment variables.
fn run_env(src: &str, env: &[(&str, &str)]) -> Out {
    let path = unique("src").with_extension("killer");
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer"));
    cmd.arg("run").arg(&path);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_file(&path);
    Out { stdout: norm(&out.stdout), stderr: norm(&out.stderr), code: out.status.code().unwrap_or(-1) }
}

/// Like [`run_env`] but kills the run after `limit` (the test then fails instead of hanging) and
/// lets the caller pick the working directory. Returns the output and the elapsed time.
fn run_watch(src: &str, env: &[(&str, &str)], cwd: Option<&std::path::Path>, limit: std::time::Duration) -> (Out, std::time::Duration) {
    let path = unique("src").with_extension("killer");
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer"));
    cmd.arg("run").arg(&path).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let start = std::time::Instant::now();
    let mut child = cmd.spawn().unwrap();
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t1 = std::thread::spawn(move || { let mut v = Vec::new(); let _ = std::io::Read::read_to_end(&mut so, &mut v); v });
    let t2 = std::thread::spawn(move || { let mut v = Vec::new(); let _ = std::io::Read::read_to_end(&mut se, &mut v); v });
    let mut code = -2;
    while start.elapsed() < limit {
        if let Ok(Some(st)) = child.try_wait() {
            code = st.code().unwrap_or(-1);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if code == -2 {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&path);
        panic!("killer run did not finish within {:?}", limit);
    }
    let elapsed = start.elapsed();
    let _ = std::fs::remove_file(&path);
    let (o, e) = (t1.join().unwrap(), t2.join().unwrap());
    (Out { stdout: norm(&o), stderr: norm(&e), code }, elapsed)
}

fn run(src: &str) -> Out {
    run_env(src, &[])
}

/// Does `lang` run a trivial block here? Cached per language.
fn works(lang: &str) -> bool {
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().unwrap().get(lang) {
        return *v;
    }
    let probe = match lang {
        "python" => "print('ok')",
        "bash" | "sh" => "echo ok",
        "ruby" => "puts 'ok'",
        "node" => "console.log('ok')",
        "perl" => "print \"ok\\n\";",
        "ps" => "Write-Output 'ok'",
        "java" => "System.out.println(\"ok\");",
        _ => "",
    };
    let src = format!("println(polyglot_exec(\"{}\", \"{}\"))\n", lang, probe.replace('\\', "\\\\").replace('"', "\\\""));
    let ok = run(&src).stdout.lines().any(|l| l.trim() == "ok");
    cache.lock().unwrap().insert(lang.to_string(), ok);
    ok
}

macro_rules! need {
    ($lang:expr) => {
        if !works($lang) {
            eprintln!("skip: the {} runtime is not available here", $lang);
            return;
        }
    };
}

// --- Item 1: the block body is taken verbatim from the source ----------------------------------

#[test]
fn py_indented_for_loop() {
    need!("python");
    let src = "x = @python {\n    for i in range(3):\n        if i % 2 == 0:\n            print(\"even\", i)\n        else:\n            print(\"odd\", i)\n}\nprintln(x)\n";
    let o = run(src);
    assert_eq!(o.stdout, "even 0\nodd 1\neven 2", "{}", o.all());
}

#[test]
fn py_floor_div_and_comment_chars() {
    need!("python");
    let o = run("y = @python { print(10 // 3) }\nprintln(y)\nz = @python { print('a--b') }\nprintln(z)\n");
    assert_eq!(o.stdout, "3\na--b", "{}", o.all());
}

#[test]
fn bash_double_dash_and_single_quotes() {
    need!("bash");
    let o = run("z = @bash { echo a--b }\nprintln(z)\nw = @bash {\n    V=1\n    echo '$V literal // x'\n}\nprintln(w)\n");
    assert_eq!(o.stdout, "a--b\n$V literal // x", "{}", o.all());
}

#[test]
fn bash_if_fi_block_is_byte_identical() {
    need!("bash");
    // the script prints its own file: it must equal the dedented block, with no injected braces
    let src = "s = @bash {\n    if [ 1 -eq 1 ]; then\n      echo yes\n    fi\n    cat \"$0\"\n}\nprintln(s)\n";
    let o = run(src);
    assert_eq!(o.stdout, "yes\nif [ 1 -eq 1 ]; then\n  echo yes\nfi\ncat \"$0\"", "{}", o.all());
}

#[test]
fn block_in_expression_position() {
    need!("python");
    let o = run("println(@python { print(5) })\nfn f() {\n    return @python { print(6) }\n}\nprintln(f())\nprintln(\"n=\" + @python { print(7) })\n");
    assert_eq!(o.stdout, "5\n6\nn=7", "{}", o.all());
}

#[test]
fn braces_in_strings_and_dict_literal() {
    need!("python");
    let o = run("d = @python {\n    d = {\"a\": 1, \"b\": {\"c\": 2}}\n    print(d[\"b\"][\"c\"], \"}{\")\n}\nprintln(d)\n");
    assert_eq!(o.stdout, "2 }{", "{}", o.all());
}

#[test]
fn empty_block_returns_empty_string() {
    need!("python");
    let o = run("e = @python { }\nprintln(len(e))\n");
    assert_eq!(o.stdout, "0", "{}", o.all());
}

#[test]
fn error_line_numbers_after_a_block_are_unchanged() {
    need!("python");
    let o = run("x = @python {\n    print(1)\n    print(2)\n}\nprintln(x)\nprintln(undefined_name_zz)\n");
    assert!(o.all().contains("6"), "error should name line 6: {}", o.all());
    assert!(o.stdout.starts_with("1\n2"), "{}", o.all());
}

#[test]
fn statement_form_prints_the_output() {
    need!("python");
    let o = run("@python {\n    print(\"hi\")\n}\nprintln(\"after\")\n");
    assert_eq!(o.stdout, "hi\nafter", "{}", o.all());
}

#[test]
fn at_sign_inside_killer_strings_and_comments_is_not_a_block() {
    let o = run("s = \"mail@python { not code }\"\nprintln(s)\n# @python { print(1) }\nprintln(2)\n");
    assert_eq!(o.stdout, "mail@python { not code }\n2", "{}", o.all());
}

// --- Item 2: timeouts, output cap, process-tree kill ---------------------------------------------

const WATCH: std::time::Duration = std::time::Duration::from_secs(120);

#[test]
fn timeout_kills_python_and_is_catchable() {
    need!("python");
    let src = "try {
    x = @python {
        import time
        time.sleep(60)
    }
    println(\"no\")
} catch e {
    println(\"caught: \" + str(e))
}
println(\"after\")
";
    let (o, took) = run_watch(src, &[("KILLER_POLYGLOT_TIMEOUT", "2")], None, WATCH);
    assert!(o.stdout.contains("caught") && o.stdout.to_lowercase().contains("timed out"), "{}", o.all());
    assert!(o.stdout.ends_with("after"), "{}", o.all());
    assert!(took < std::time::Duration::from_secs(40), "took {:?}", took);
}

#[test]
fn bash_sleep_timeout_kills_the_process_tree() {
    need!("bash");
    // `sleep` is a child of bash: unless the whole tree dies it keeps the output pipe open
    let src = "try {
    x = @bash {
        sleep 30
        echo late
    }
} catch e {
    println(\"caught: \" + str(e))
}
";
    let (o, took) = run_watch(src, &[("KILLER_POLYGLOT_TIMEOUT", "2")], None, WATCH);
    assert!(o.stdout.to_lowercase().contains("timed out"), "{}", o.all());
    assert!(took < std::time::Duration::from_secs(25), "took {:?}", took);
}

#[test]
fn output_cap_stops_a_flood() {
    need!("python");
    let src = "try {
    x = @python { print('x' * 50000000) }
    println(len(x))
} catch e {
    println(\"caught: \" + str(e))
}
";
    let (o, _) = run_watch(src, &[("KILLER_POLYGLOT_MAX_OUTPUT", "1048576")], None, WATCH);
    assert!(o.stdout.contains("output limit exceeded"), "{}", &o.all()[..o.all().len().min(400)]);
}

#[test]
fn stream_with_lots_of_stderr_does_not_deadlock() {
    need!("python");
    let src = "s = polyglot_stream(\"python\", \"import sys; sys.stderr.write(chr(101) * 1000000); print(chr(100) + chr(111) + chr(110) + chr(101))\")\nprintln(\"got \" + s)\n";
    let (o, _) = run_watch(src, &[], None, WATCH);
    assert!(o.stdout.ends_with("got done"), "{}", &o.all()[..o.all().len().min(400)]);
}

// --- Item 3: environment scrub, private working directory, temp cleanup -------------------------

#[test]
fn env_is_scrubbed_and_pass_through_is_explicit() {
    need!("bash");
    let src = "e = @bash { env }\nprintln(e)\n";
    let env = [("SECRET_TEST_PG", "leak1"), ("KILLER_KHLM_LLM_API_KEY", "leak2")];
    let (o, _) = run_watch(src, &env, None, WATCH);
    assert!(!o.stdout.contains("leak1") && !o.stdout.contains("leak2"), "{}", o.all());
    assert!(o.stdout.contains("PATH="), "{}", o.all());
    let env = [("SECRET_TEST_PG", "leak1"), ("KILLER_POLYGLOT_ENV_PASS", "SECRET_TEST_PG")];
    let (o, _) = run_watch(src, &env, None, WATCH);
    assert!(o.stdout.contains("SECRET_TEST_PG=leak1"), "{}", o.all());
}

#[test]
fn python_unicode_round_trips() {
    need!("python");
    let (o, _) = run_watch("u = @python { print('h\u{e9}llo \u{65e5}\u{672c} \u{2713}') }\nprintln(u)\n", &[], None, WATCH);
    assert_eq!(o.stdout, "h\u{e9}llo \u{65e5}\u{672c} \u{2713}", "{}", o.all());
}

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let d = unique(tag);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn temp_files_are_removed_after_success_failure_and_timeout() {
    need!("python");
    let tmp = fresh_dir("tmp");
    let t = tmp.to_string_lossy().to_string();
    let env = [("TEMP", t.as_str()), ("TMP", t.as_str()), ("TMPDIR", t.as_str()), ("KILLER_POLYGLOT_TIMEOUT", "20")];
    let src = "a = @python { print(1) }\ntry {\n    b = @python { raise SystemExit(4) }\n} catch e {\n    println(\"x\")\n}\ntry {\n    c = @python {\n        import time\n        time.sleep(300)\n    }\n} catch e {\n    println(\"y\")\n}\nprintln(a)\n";
    let (o, _) = run_watch(src, &env, None, WATCH);
    assert_eq!(o.stdout, "x\ny\n1", "{}", o.all());
    let left: Vec<_> = std::fs::read_dir(&tmp).unwrap().flatten().map(|e| e.file_name()).filter(|n| n.to_string_lossy().starts_with("killer_")).collect();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(left.is_empty(), "left behind: {:?}", left);
}

#[test]
fn block_runs_in_a_private_working_directory() {
    need!("python");
    let cwd = fresh_dir("cwd");
    let src = "d = @python {\n    import os\n    print(os.getcwd())\n    print(os.environ.get('KILLER_CWD', ''))\n}\nprintln(d)\n";
    let (o, _) = run_watch(src, &[], Some(&cwd), WATCH);
    let lines: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", o.all());
    let canon = |p: &str| std::fs::canonicalize(p).map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    assert_ne!(canon(lines[0]), canon(&cwd.to_string_lossy()), "{}", o.all());
    assert_eq!(canon(lines[1]), canon(&cwd.to_string_lossy()), "{}", o.all());
    let _ = std::fs::remove_dir_all(&cwd);
}

#[cfg(unix)]
#[test]
fn script_file_is_owner_only() {
    need!("bash");
    let (o, _) = run_watch("m = @bash { ls -l \"$0\" }\nprintln(m)\n", &[], None, WATCH);
    assert!(o.stdout.starts_with("-rw-------"), "{}", o.all());
}


// --- Item 4: absolute runtime resolution and overrides -------------------------------------------

#[test]
fn runtime_override_must_be_absolute_and_exist() {
    let (o, _) = run_watch("println(polyglot_exec(\"python\", \"print(1)\"))\n", &[("KILLER_POLYGLOT_PYTHON", "python")], None, WATCH);
    assert!(o.all().contains("KILLER_POLYGLOT_PYTHON") && o.all().contains("absolute"), "{}", o.all());
    let missing = std::env::temp_dir().join("killer_pgt_no_such_python_zz.exe");
    let m = missing.to_string_lossy().to_string();
    let (o, _) = run_watch("println(polyglot_exec(\"python\", \"print(1)\"))\n", &[("KILLER_PYTHON", m.as_str())], None, WATCH);
    assert!(o.all().contains("POLYGLOT_E_RUNTIME_MISSING"), "{}", o.all());
}

#[test]
fn polyglot_list_shows_absolute_paths() {
    need!("python");
    let (o, _) = run_watch("println(polyglot_list())\n", &[], None, WATCH);
    let line = o.stdout.lines().find(|l| l.trim_start().starts_with("@python")).unwrap_or("").to_string();
    let path = line.split_whitespace().nth(1).unwrap_or("");
    assert!(std::path::Path::new(path).is_absolute(), "{}", o.all());
}

#[cfg(unix)]
#[test]
fn planted_runtime_in_a_relative_path_entry_is_not_run() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir("plant");
    let marker = dir.join("PLANTED_RAN");
    for name in ["python3", "python", "py"] {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\ntouch '{}'\necho planted\n", marker.display())).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(".:{}", std::env::var("PATH").unwrap_or_default());
    let (o, _) = run_watch("try {\n    println(polyglot_exec(\"python\", \"print(1)\"))\n} catch e {\n    println(\"err\")\n}\n", &[("PATH", path.as_str())], Some(&dir), WATCH);
    assert!(!marker.exists() && !o.stdout.contains("planted"), "{}", o.all());
    let _ = std::fs::remove_dir_all(&dir);
}


// --- Item 5: error taxonomy with exit codes ------------------------------------------------------

#[test]
fn nonzero_exit_names_the_exit_code() {
    need!("bash");
    let (o, _) = run_watch("try {\n    x = @bash { exit 3 }\n} catch e {\n    println(str(e))\n}\n", &[], None, WATCH);
    assert!(o.stdout.contains("POLYGLOT_E_EXIT") && o.stdout.contains("exit code 3"), "{}", o.all());
}

#[test]
fn unknown_language_has_a_stable_code() {
    let (o, _) = run_watch("try {\n    polyglot_exec(\"cobol\", \"x\")\n} catch e {\n    println(str(e))\n}\n", &[], None, WATCH);
    assert!(o.stdout.contains("POLYGLOT_E_UNKNOWN_LANG") && o.stdout.contains("Unknown language"), "{}", o.all());
}

#[test]
fn polyglot_run_returns_stdout_stderr_and_code() {
    need!("python");
    let src = "r = polyglot_run(\"python\", \"import sys\\nsys.stderr.write('warn1\\\\n')\\nprint(7)\")\nprintln(r[\"stdout\"])\nprintln(r[\"stderr\"])\nprintln(r[\"code\"])\nq = polyglot_run(\"python\", \"raise SystemExit(5)\")\nprintln(q[\"code\"])\n";
    let (o, _) = run_watch(src, &[], None, WATCH);
    assert_eq!(o.stdout, "7\nwarn1\n0\n5", "{}", o.all());
}

#[test]
fn stderr_snippet_in_errors_is_capped() {
    need!("python");
    let src = "try {\n    x = @python {\n        import sys\n        sys.stderr.write('E' * 20000)\n        sys.exit(2)\n    }\n} catch e {\n    println(len(str(e)))\n}\n";
    let (o, _) = run_watch(src, &[], None, WATCH);
    let n: usize = o.stdout.trim().parse().unwrap_or(usize::MAX);
    assert!(n < 4096, "{}", &o.all()[..o.all().len().min(300)]);
}

// --- Item 6: the AI assist layer is opt-in ------------------------------------------------------

#[test]
fn errors_carry_no_ai_boilerplate() {
    need!("bash");
    let src = "try {\n    x = @bash { exit 3 }\n} catch e {\n    println(str(e))\n}\n";
    for env in [&[][..], &[("KILLER_POLYGLOT_AI", "1")][..]] {
        let (o, _) = run_watch(src, env, None, WATCH);
        assert!(o.stdout.contains("exit code 3") && !o.stdout.contains("AI Assassin Assist"), "{}", o.all());
    }
}

#[test]
fn missing_runtime_error_is_short() {
    let missing = std::env::temp_dir().join("killer_pgt_no_such_node_zz.exe");
    let m = missing.to_string_lossy().to_string();
    let src = "try {\n    x = @node { console.log(1) }\n} catch e {\n    println(str(e))\n}\n";
    let (o, _) = run_watch(src, &[("KILLER_POLYGLOT_NODE", m.as_str())], None, WATCH);
    assert!(o.stdout.starts_with("POLYGLOT_E_RUNTIME_MISSING") && !o.stdout.contains("AI Assassin"), "{}", o.all());
}

// --- Item 7: language policy -------------------------------------------------------------------

#[test]
fn allow_list_admits_only_the_listed_languages() {
    need!("python");
    let src = "println(@python { print(1) })\ntry {\n    x = @bash { echo 2 }\n    println(x)\n} catch e {\n    println(str(e))\n}\n";
    let (o, _) = run_watch(src, &[("KILLER_POLYGLOT_ALLOW", "python")], None, WATCH);
    let lines: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(lines.first().copied(), Some("1"), "{}", o.all());
    assert!(lines.get(1).map_or(false, |l| l.starts_with("POLYGLOT_E_POLICY_DENIED")), "{}", o.all());
    assert!(!o.stderr.contains("warning"), "an explicit allow list silences the warning: {}", o.all());
}

#[test]
fn allow_none_and_strict_profile_deny() {
    let src = "try {\n    x = @python { print(1) }\n    println(x)\n} catch e {\n    println(str(e))\n}\n";
    for env in [&[("KILLER_POLYGLOT_ALLOW", "none")][..], &[("KILLER_PROFILE", "strict")][..], &[("KILLER_PROFILE", "prod")][..]] {
        let (o, _) = run_watch(src, env, None, WATCH);
        assert!(o.stdout.starts_with("POLYGLOT_E_POLICY_DENIED"), "{:?}: {}", env, o.all());
    }
}

#[test]
fn strict_profile_with_an_allow_list_runs_and_aliases_match() {
    need!("python");
    let src = "println(@python3 { print(1) })\nprintln(@python { print(2) })\n";
    let (o, _) = run_watch(src, &[("KILLER_PROFILE", "strict"), ("KILLER_POLYGLOT_ALLOW", "python3")], None, WATCH);
    assert_eq!(o.stdout, "1\n2", "{}", o.all());
    // `js` in the list admits `@node` (the runtime may be missing here: only the policy is checked)
    let (o, _) = run_watch("try {\n    x = @node { console.log(1) }\n} catch e {\n    println(str(e))\n}\n", &[("KILLER_PROFILE", "strict"), ("KILLER_POLYGLOT_ALLOW", "js")], None, WATCH);
    assert!(!o.all().contains("POLYGLOT_E_POLICY_DENIED"), "{}", o.all());
}

#[test]
fn default_policy_warns_once_per_process() {
    need!("python");
    let (o, _) = run_watch("a = @python { print(1) }\nb = @python { print(2) }\nprintln(a + b)\n", &[], None, WATCH);
    assert_eq!(o.stdout, "12", "{}", o.all());
    assert_eq!(o.stderr.matches("warning").count(), 1, "{}", o.all());
}

// --- Item 8: audit log -------------------------------------------------------------------------

fn audit_lines(src: &str, extra: &[(&str, &str)]) -> (Out, Vec<String>) {
    let log = unique("audit").with_extension("jsonl");
    let l = log.to_string_lossy().to_string();
    let mut env: Vec<(&str, &str)> = vec![("KILLER_AUDIT_LOG", l.as_str())];
    env.extend_from_slice(extra);
    let (o, _) = run_watch(src, &env, None, WATCH);
    let lines = std::fs::read_to_string(&log).unwrap_or_default().lines().map(str::to_string).collect();
    let _ = std::fs::remove_file(&log);
    (o, lines)
}

#[test]
fn audit_log_has_one_json_line_per_run_without_the_source() {
    need!("python");
    let (o, lines) = audit_lines("x = @python { print(41 + 1) }\nprintln(x)\n", &[]);
    assert_eq!(o.stdout, "42", "{}", o.all());
    assert_eq!(lines.len(), 1, "{:?}", lines);
    let l = &lines[0];
    assert!(l.starts_with('{') && l.ends_with('}'), "{l}");
    for key in ["\"ts\":", "\"lang\":\"python\"", "\"exe\":", "\"code_sha256\":\"", "\"code_len\":", "\"duration_ms\":", "\"exit_code\":0", "\"outcome\":\"ok\""] {
        assert!(l.contains(key), "{key} missing: {l}");
    }
    assert!(!l.contains("41 + 1"), "the source must not be logged: {l}");
    let (_, lines) = audit_lines("x = @python { print(41 + 1) }\n", &[("KILLER_AUDIT_CODE", "1")]);
    assert!(lines.len() == 1 && lines[0].contains("41 + 1"), "{:?}", lines);
}

#[test]
fn audit_log_records_stream_denials_and_timeouts() {
    need!("python");
    let src = "s = polyglot_stream(\"python\", \"print(1)\")\ntry {\n    y = @bash { echo 1 }\n} catch e {\n    println(\"denied\")\n}\ntry {\n    z = @python {\n        import time\n        time.sleep(60)\n    }\n} catch e {\n    println(\"timeout\")\n}\n";
    let (o, lines) = audit_lines(src, &[("KILLER_POLYGLOT_ALLOW", "python"), ("KILLER_POLYGLOT_TIMEOUT", "3")]);
    assert_eq!(lines.len(), 3, "{:?} {}", lines, o.all());
    assert!(lines[0].contains("\"outcome\":\"ok\""), "{:?}", lines);
    assert!(lines[1].contains("\"lang\":\"bash\"") && lines[1].contains("POLYGLOT_E_POLICY_DENIED"), "{:?}", lines);
    assert!(lines[2].contains("POLYGLOT_E_TIMEOUT"), "{:?}", lines);
}
