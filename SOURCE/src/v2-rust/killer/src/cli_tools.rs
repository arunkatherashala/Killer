//! Command implementations behind the `killer` driver (`src/bin/killer.rs`): `run`, `fmt`,
//! `lint`, `repl`. Each takes the arguments after the subcommand and returns the exit code, so
//! they can be tested in-process. (`test` lives in [`crate::testfw`], `lsp` in
//! [`crate::lsp_core`].)

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Expand files and directories (recursively, `*.killer`) into a sorted file list.
pub fn expand_paths(args: &[String]) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') && name != "node_modules" {
                    walk(&p, out);
                }
            } else if p.extension().map_or(false, |e| e == "killer") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for a in args {
        let p = PathBuf::from(a);
        if p.is_dir() {
            walk(&p, &mut out);
        } else {
            out.push(p);
        }
    }
    out
}

/// `killer run <file>`: compile and run a program.
/// Resolve `import` relative to the script's folder and name the script in error messages
/// (what `killer_super` does before compiling).
pub fn prepare_script(path: &Path) {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    crate::imports::set_base_dir(dir);
    crate::imports::set_main_file(path.display().to_string());
}

pub fn cmd_run(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("usage: killer run <file.killer>");
        return 2;
    };
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("killer: cannot read {path}: {e}");
            return 1;
        }
    };
    prepare_script(Path::new(path));
    match crate::run_killer_source(&source) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

/// `killer fmt [--check] <files or dirs...>`.
pub fn cmd_fmt(args: &[String]) -> i32 {
    let mut check = false;
    let mut targets: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "--check" => check = true,
            "-h" | "--help" => {
                println!("usage: killer fmt [--check] <files or dirs...>\n\nRe-indents brace code (2 spaces), strips trailing whitespace and collapses blank lines.\nWith --check nothing is written; exit code 1 means some file would change.");
                return 0;
            }
            other => targets.push(other.to_string()),
        }
    }
    if targets.is_empty() {
        eprintln!("usage: killer fmt [--check] <files or dirs...>");
        return 2;
    }
    let mut changed = 0usize;
    let mut failed = false;
    let files = expand_paths(&targets);
    for f in &files {
        let source = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("killer fmt: cannot read {}: {e}", f.display());
                failed = true;
                continue;
            }
        };
        let formatted = match crate::formatter::Formatter::new().format(&source) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("killer fmt: {}: {e}", f.display());
                failed = true;
                continue;
            }
        };
        if formatted != source {
            changed += 1;
            if check {
                println!("would reformat {}", f.display());
            } else if let Err(e) = std::fs::write(f, &formatted) {
                eprintln!("killer fmt: cannot write {}: {e}", f.display());
                failed = true;
            } else {
                println!("formatted {}", f.display());
            }
        }
    }
    if check {
        if changed > 0 {
            println!("{changed} of {} file(s) need formatting", files.len());
        }
        return if changed > 0 || failed { 1 } else { 0 };
    }
    if failed { 1 } else { 0 }
}

/// `killer lint [--deny-warnings] [--allow RULE] [--rules] <files or dirs...>`.
pub fn cmd_lint(args: &[String]) -> i32 {
    use crate::linter::{lint_killer, LintSeverity, LINT_RULES};
    let mut deny_warnings = false;
    let mut allow: Vec<String> = Vec::new();
    let mut targets: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--deny-warnings" => deny_warnings = true,
            "--allow" => {
                i += 1;
                if let Some(r) = args.get(i) {
                    allow.push(r.clone());
                }
            }
            "--rules" => {
                for (id, sev, doc) in LINT_RULES {
                    println!("{id:24} {sev:5}  {doc}");
                }
                return 0;
            }
            "-h" | "--help" => {
                println!("usage: killer lint [--deny-warnings] [--allow RULE] [--rules] <files or dirs...>\n\nSuppress a finding with `# lint:ignore RULE` on or above the line, or `# lint:ignore-file RULE`.");
                return 0;
            }
            other => targets.push(other.to_string()),
        }
        i += 1;
    }
    if targets.is_empty() {
        eprintln!("usage: killer lint [--deny-warnings] [--allow RULE] <files or dirs...>");
        return 2;
    }
    let (mut errors, mut warnings) = (0usize, 0usize);
    for f in expand_paths(&targets) {
        let source = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("killer lint: cannot read {}: {e}", f.display());
                errors += 1;
                continue;
            }
        };
        for v in lint_killer(&source) {
            if allow.contains(&v.rule) {
                continue;
            }
            match v.severity {
                LintSeverity::Error => errors += 1,
                _ => warnings += 1,
            }
            println!(
                "{}:{}:{}: {} [{}] {}",
                f.display(),
                v.line.unwrap_or(0),
                v.column.unwrap_or(0),
                match v.severity {
                    LintSeverity::Error => "error",
                    LintSeverity::Warning => "warning",
                    LintSeverity::Info => "info",
                },
                v.rule,
                v.message
            );
        }
    }
    println!("{errors} error(s), {warnings} warning(s)");
    if errors > 0 || (deny_warnings && warnings > 0) { 1 } else { 0 }
}

/// `killer repl`: variables persist between lines; a line ending in `{` continues until braces balance.
pub fn cmd_repl(_args: &[String]) -> i32 {
    println!("Killer REPL - type .exit to quit");
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut vm = crate::vm::VirtualMachine::default();
    vm.repl_mode = true;
    loop {
        print!("killer> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let mut input = line.trim_end().to_string();
        if matches!(input.trim(), ".exit" | ".quit" | "exit" | "quit") {
            break;
        }
        if input.trim().is_empty() {
            continue;
        }
        let balance = |s: &str| {
            crate::formatter::mask_code(s).chars().fold(0i64, |d, c| match c {
                '{' | '[' | '(' => d + 1,
                '}' | ']' | ')' => d - 1,
                _ => d,
            })
        };
        let mut depth = balance(&input);
        while depth > 0 {
            print!("  ...> ");
            std::io::stdout().flush().ok();
            let mut more = String::new();
            if reader.read_line(&mut more).unwrap_or(0) == 0 {
                break;
            }
            depth += balance(&more);
            input.push('\n');
            input.push_str(more.trim_end());
        }
        match crate::compiler::compile_killer_subset(&input) {
            Ok(program) => {
                if let Err(e) = vm.run(&program) {
                    eprintln!("Error: {e}");
                }
            }
            Err(e) => eprintln!("Error: {e}"),
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, text: &str) -> String {
        let dir = std::env::temp_dir().join(format!("killer_cli_tools_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p.display().to_string()
    }

    #[test]
    fn fmt_check_and_write() {
        let f = tmp("fmt1.killer", "fn f() {\nreturn 1   \n}\n");
        assert_eq!(cmd_fmt(&["--check".into(), f.clone()]), 1);
        assert_eq!(cmd_fmt(&[f.clone()]), 0);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "fn f() {\n  return 1\n}\n");
        assert_eq!(cmd_fmt(&["--check".into(), f]), 0);
    }

    #[test]
    fn lint_exit_codes() {
        let bad = tmp("lint_bad.killer", "prnt(1)\n");
        let warn = tmp("lint_warn.killer", "fn f() {\n  unused = 1\n  return 2\n}\nprint(f())\n");
        let ok = tmp("lint_ok.killer", "print(1)\n");
        assert_eq!(cmd_lint(&[bad]), 1);
        assert_eq!(cmd_lint(&[warn.clone()]), 0);
        assert_eq!(cmd_lint(&["--deny-warnings".into(), warn]), 1);
        assert_eq!(cmd_lint(&[ok]), 0);
    }
}
