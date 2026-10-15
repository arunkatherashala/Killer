//! `killer profile script.killer [--html out.html] [--top N] [--lines N]`: run a script under the
//! VM profiler and print the report (functions by self time, hot lines), optionally writing a
//! self-contained flame-graph HTML page. Also used by `killer_super --profile`.

use crate::error::VmError;

pub struct ProfileOpts {
    pub html: Option<String>,
    pub top: usize,
    pub lines: usize,
    pub title: String,
    /// script path shown in line reports instead of `<script>`
    pub file: Option<String>,
}

impl Default for ProfileOpts {
    fn default() -> Self {
        ProfileOpts { html: None, top: 30, lines: 15, title: "Killer profile".to_string(), file: None }
    }
}

/// Run `source` with profiling on and print the report to stdout after the script's own output.
/// The script's error (if any) is returned after the report has been printed.
pub fn run_profiled(source: &str, opts: &ProfileOpts) -> Result<(), VmError> {
    let program = crate::compiler::compile_killer_default(source)?;
    let mut vm = crate::vm::VirtualMachine::new();
    vm.start_profiling(&program);
    let result = vm.run(&program);
    let prof = vm.stop_profiling();
    if let Some(mut p) = prof {
        if let Some(f) = &opts.file {
            p.rename_file(f);
        }
        println!();
        print!("{}", p.render_text(opts.top, opts.lines));
        if let Some(path) = &opts.html {
            match std::fs::write(path, p.render_html(&opts.title)) {
                Ok(()) => println!("\nflame graph written to {}", path),
                Err(e) => eprintln!("killer profile: cannot write {}: {}", path, e),
            }
        }
    }
    result
}

/// Parse the arguments of `killer profile`: `(file, options)`.
pub fn parse_args(args: &[String]) -> Result<(String, ProfileOpts), String> {
    let mut file = None;
    let mut opts = ProfileOpts::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut value = |name: &str| -> Result<String, String> {
            i += 1;
            args.get(i).cloned().ok_or_else(|| format!("{} needs a value", name))
        };
        match a {
            "--html" => opts.html = Some(value("--html")?),
            "--top" => opts.top = value("--top")?.parse().map_err(|_| "--top needs a number".to_string())?,
            "--lines" => opts.lines = value("--lines")?.parse().map_err(|_| "--lines needs a number".to_string())?,
            x if x.starts_with('-') => return Err(format!("unknown option {}", x)),
            x => {
                if file.is_some() {
                    return Err("only one script can be profiled".to_string());
                }
                file = Some(x.to_string());
            }
        }
        i += 1;
    }
    let file = file.ok_or_else(|| "usage: killer profile <file.killer> [--html out.html] [--top N] [--lines N]".to_string())?;
    opts.file = Some(file.clone());
    opts.title = format!("Killer profile: {}", file);
    Ok((file, opts))
}

pub fn cmd_profile(args: &[String]) -> i32 {
    let (path, opts) = match parse_args(args) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("killer profile: {}", e);
            return 2;
        }
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("killer: cannot read {}: {}", path, e);
            return 1;
        }
    };
    crate::imports::set_main_file(path.clone());
    // deep recursion in the profiled script needs the same large stack the compiler binary uses
    let handle = std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(move || run_profiled(&source, &opts));
    match handle.map(|h| h.join()) {
        Ok(Ok(Ok(()))) => 0,
        Ok(Ok(Err(e))) => {
            eprintln!("Error: {}", e);
            1
        }
        _ => {
            eprintln!("killer profile: the profiled program crashed");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_options() {
        let (f, o) = parse_args(&s(&["a.killer", "--html", "o.html", "--top", "5"])).unwrap();
        assert_eq!(f, "a.killer");
        assert_eq!(o.html.as_deref(), Some("o.html"));
        assert_eq!(o.top, 5);
        assert!(parse_args(&s(&[])).is_err());
        assert!(parse_args(&s(&["a", "b"])).is_err());
        assert!(parse_args(&s(&["a", "--nope"])).is_err());
        assert!(parse_args(&s(&["a", "--top"])).is_err());
    }
}
