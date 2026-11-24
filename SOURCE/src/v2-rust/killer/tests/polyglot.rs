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
