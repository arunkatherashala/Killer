/// Killer Language REPL — interactive read-eval-print loop
///
/// Features:
///   • Persistent variables across lines (repl_mode)
///   • Multi-line input (auto-detects open braces, backslash continuation)
///   • Expression result printing  (=> value)
///   • .jit   — show JIT compilation stats
///   • .vars  — list all live variables
///   • .gc    — show GC stats
///   • .history — command history
///   • .clear — clear screen
///   • .help  — help
use std::io::{self, Write};
use killer_native::compiler::compile_killer_subset;
use killer_native::vm::VirtualMachine;

fn main() {
    println!("Killer v2.1  REPL  —  type .help for commands, .exit to quit");
    println!("─────────────────────────────────────────────────────────────");

    let stdin = io::stdin();
    let mut vm = VirtualMachine::default();
    vm.repl_mode = true;   // preserve variables between lines

    let mut history: Vec<String> = Vec::new();

    loop {
        print!("killer> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let line = input.trim_end_matches('\n').trim_end_matches('\r').to_string();

        // ── REPL commands ─────────────────────────────────────────────────
        match line.trim() {
            ".exit" | ".quit" | "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }

            ".help" => {
                println!("REPL commands:");
                println!("  .exit / .quit  — exit");
                println!("  .vars          — list all live variables");
                println!("  .jit           — show JIT compilation stats");
                println!("  .gc            — show GC stats");
                println!("  .history       — command history");
                println!("  .clear         — clear screen");
                println!();
                println!("Language quick-ref:");
                println!("  x = 42                    — assign variable");
                println!("  println(x)                — print value");
                println!("  fn add(a, b) {{ a + b }}   — define function");
                println!("  for i in range(10) {{ … }} — loop");
                println!("  arr = [1, 2, 3]           — array");
                println!("  x = http_get(url)         — HTTP request");
                println!("  x = readFile(\"f.txt\")    — read file");
                println!("  believe v = 72 ± 3        — uncertain value");
                continue;
            }

            ".vars" => {
                let vars = vm.repl_vars();
                if vars.is_empty() {
                    println!("(no variables defined)");
                } else {
                    println!("─── variables ───");
                    for (k, v) in &vars {
                        // Truncate long values for display
                        let display = if v.len() > 80 { format!("{}…", &v[..80]) } else { v.clone() };
                        println!("  {} = {}", k, display);
                    }
                }
                continue;
            }

            ".jit" => {
                let (compiled, threshold) = vm.jit_stats();
                println!("─── JIT stats ───");
                println!("  Compiled loops : {}", compiled);
                println!("  Hot threshold  : {} iterations", threshold);
                println!("  Status         : {}", if compiled > 0 { "active — native x86-64 running" } else { "warming up (no loops hit threshold yet)" });
                continue;
            }

            ".gc" => {
                let (tracked, allocs) = killer_native::gc::gc_stats();
                println!("─── GC stats ───");
                println!("  Tracked arrays : {}", tracked);
                println!("  Total allocs   : {}", allocs);
                println!("  Interval       : every {} allocs", killer_native::gc::GC_INTERVAL);
                continue;
            }

            ".history" => {
                if history.is_empty() {
                    println!("(no history)");
                } else {
                    for (i, h) in history.iter().enumerate() {
                        println!("  {:3}: {}", i + 1, h);
                    }
                }
                continue;
            }

            ".clear" => {
                print!("\x1b[2J\x1b[H");
                io::stdout().flush().ok();
                continue;
            }

            "" => continue,
            _ => {}
        }

        // ── Multi-line input (open braces / backslash continuation) ──────
        let mut full_input = line.clone();
        let mut open_braces = count_open_braces(&full_input);

        while open_braces > 0 || full_input.trim_end().ends_with('\\') {
            print!("  ... ");
            io::stdout().flush().ok();
            let mut more = String::new();
            if stdin.read_line(&mut more).is_err() { break; }
            let more_trimmed = more.trim_end_matches('\n').trim_end_matches('\r');
            // Strip trailing backslash continuation
            if full_input.trim_end().ends_with('\\') {
                let end = full_input.trim_end_matches('\\').len();
                full_input.truncate(end);
            }
            full_input.push('\n');
            full_input.push_str(more_trimmed);
            open_braces += count_open_braces(more_trimmed);
            if more_trimmed.trim().is_empty() && open_braces <= 0 { break; }
        }

        history.push(full_input.clone());

        // ── Compile and run ───────────────────────────────────────────────
        match compile_killer_subset(&full_input) {
            Ok(program) => {
                match vm.run(&program) {
                    Ok(()) => {
                        // Print top-of-stack result for expressions
                        if let Some(val) = vm.stack.last() {
                            let s = format!("{}", val);
                            if !s.is_empty() && s != "null" {
                                println!("=> {}", s);
                            }
                        }
                    }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            Err(e) => eprintln!("syntax error: {}", e),
        }
    }
}

fn count_open_braces(s: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = '\0';
    for ch in s.chars() {
        match ch {
            '"' if prev != '\\' => in_str = !in_str,
            '{' if !in_str => depth += 1,
            '}' if !in_str => depth -= 1,
            '#' if !in_str => break, // comment
            _ => {}
        }
        prev = ch;
    }
    depth
}
