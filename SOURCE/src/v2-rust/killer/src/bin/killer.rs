//! `killer`: the driver for the Killer toolchain.
//!
//! ```text
//! killer run <file>             run a program
//! killer fmt [--check] <files>  format source files
//! killer lint <files>           lint source files
//! killer profile <file>         run under the profiler (report, --html flame graph)
//! killer test [filter]          run tests
//! killer repl                   interactive prompt
//! killer lsp                    language server over stdio
//! killer --version
//! ```
//!
//! To add a subcommand, add one row to [`COMMANDS`]: `(name, one-line help, handler)`. A handler
//! takes the arguments after the subcommand name and returns the process exit code.

use killer_native::{cli_tools, lsp_core, testfw};

type Handler = fn(&[String]) -> i32;

/// The subcommand table: the single place to extend the driver.
const COMMANDS: &[(&str, &str, Handler)] = &[
    ("run", "run a program: killer run <file.killer>", cli_tools::cmd_run),
    ("fmt", "format files: killer fmt [--check] <files or dirs>", cli_tools::cmd_fmt),
    ("profile", "profile a program: killer profile <file.killer> [--html out.html] [--top N] [--lines N]", killer_native::profile_cli::cmd_profile),
    ("lint", "lint files: killer lint [--deny-warnings] <files or dirs>", cli_tools::cmd_lint),
    ("test", "run tests: killer test [--junit FILE] [filter] [paths]", testfw::cmd_test),
    ("repl", "interactive prompt", cli_tools::cmd_repl),
    ("lsp", "language server over stdio", |_| lsp_core::serve_stdio()),
];

fn version_line() -> String {
    format!("killer {}", env!("CARGO_PKG_VERSION"))
}

fn print_help() {
    println!("{}\n\nUSAGE:\n    killer <command> [args]\n    killer <file.killer>          same as `killer run`\n\nCOMMANDS:", version_line());
    for (name, help, _) in COMMANDS {
        println!("    {name:<6} {help}");
    }
    println!("\nOPTIONS:\n    -V, --version   print the version\n    -h, --help      print this help");
}

/// Dispatch `args` (without the program name); returns the exit code.
fn dispatch(args: &[String]) -> i32 {
    match args.first().map(|s| s.as_str()) {
        None | Some("-h") | Some("--help") | Some("help") => {
            print_help();
            0
        }
        Some("-V") | Some("--version") | Some("version") => {
            println!("{}", version_line());
            0
        }
        Some(cmd) => {
            if let Some((_, _, handler)) = COMMANDS.iter().find(|(n, _, _)| *n == cmd) {
                handler(&args[1..])
            } else if cmd.ends_with(".killer") {
                cli_tools::cmd_run(args)
            } else {
                eprintln!("killer: unknown command `{cmd}`\n");
                print_help();
                2
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(dispatch(&args));
}
