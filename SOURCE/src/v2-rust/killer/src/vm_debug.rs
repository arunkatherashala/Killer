//! Step debugger: `killer debug script.killer [--break N]...`.
//!
//! The VM (with `profiling` on, which also keeps native JIT paths off so every line is seen) calls
//! [`before_instruction`] ahead of each instruction. When the source line changes, the debugger decides whether to
//! stop (single step, step over, step out, or a breakpoint) and then reads commands from standard input:
//!
//! ```text
//! s / step        run to the next line (into calls)      n / next    run to the next line in this function
//! f / finish      run until the current function returns   c / continue  run to the next breakpoint
//! b N             breakpoint at line N                    d N         delete it        bl   list breakpoints
//! l [N]           show source around line N               bt          call stack (return lines)
//! locals          local slot values of this frame         globals     global values (functions, constants)
//! p NAME          print one global                        q / quit    stop the program
//! ```
//!
//! Limits: local variables are shown by slot number (the bytecode does not keep their names), and top-level
//! variables are slots of the main frame, so they appear under `locals` while stopped at the top level.

use crate::bytecode::Program;
use crate::error::VmError;
use crate::vm::VirtualMachine;
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, Write};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Step,
    Next(usize),
    Finish(usize),
    Continue,
}

pub struct DebugSession {
    breakpoints: BTreeSet<u32>,
    mode: Mode,
    last: Option<(String, u32, usize)>,
    sources: HashMap<String, Vec<String>>,
    input: Box<dyn BufRead>,
    script: String,
}

impl DebugSession {
    pub fn new(breakpoints: Vec<u32>, input: Box<dyn BufRead>, script: String) -> Self {
        DebugSession { breakpoints: breakpoints.into_iter().collect(), mode: Mode::Step, last: None, sources: HashMap::new(), input, script }
    }

    fn line_text(&mut self, file: &str, line: u32) -> String {
        let real = if file == "<script>" { self.script.clone() } else { file.to_string() };
        let lines = self.sources.entry(file.to_string()).or_insert_with(|| {
            std::fs::read_to_string(&real).map(|t| t.lines().map(|l| l.to_string()).collect()).unwrap_or_default()
        });
        lines.get((line as usize).wrapping_sub(1)).cloned().unwrap_or_default()
    }

    fn list(&mut self, file: &str, around: u32) {
        let from = around.saturating_sub(3).max(1);
        for l in from..from + 7 {
            let text = self.line_text(file, l);
            if text.is_empty() && l > around + 3 {
                break;
            }
            println!("{} {:>4}  {}", if l == around { "->" } else { "  " }, l, text);
        }
    }
}

/// Called by the VM before every instruction while a debug session is attached.
pub(crate) fn before_instruction(vm: &mut VirtualMachine, program: &Program) -> Result<(), VmError> {
    let mut session = match vm.debugger.take() {
        Some(s) => s,
        None => return Ok(()),
    };
    let result = step(vm, program, &mut session);
    vm.debugger = Some(session);
    result
}

fn step(vm: &mut VirtualMachine, program: &Program, s: &mut DebugSession) -> Result<(), VmError> {
    let (file, line) = match program.line_table.lookup(vm.ip) {
        Some((f, l)) => (f.to_string(), l),
        None => return Ok(()),
    };
    if file.starts_with("<std>") {
        return Ok(()); // embedded libraries (<std>/...) are not stepped through
    }
    let depth = vm.call_stack_depth();
    let new_location = match &s.last {
        Some((f, l, d)) => *f != file || *l != line || *d != depth,
        None => true,
    };
    if !new_location {
        return Ok(());
    }
    s.last = Some((file.clone(), line, depth));
    let by_mode = match s.mode {
        Mode::Step => true,
        Mode::Next(d) => depth <= d,
        Mode::Finish(d) => depth < d,
        Mode::Continue => false,
    };
    let hit = s.breakpoints.contains(&line);
    if !(by_mode || hit) {
        return Ok(());
    }
    println!("{}{}:{}: {}", if hit { "[breakpoint] " } else { "" }, file, line, s.line_text(&file, line).trim());
    loop {
        print!("(kdb) ");
        let _ = std::io::stdout().flush();
        let mut cmd = String::new();
        if s.input.read_line(&mut cmd).unwrap_or(0) == 0 {
            return Err(VmError::runtime_error("debugger: input closed, program stopped".to_string()));
        }
        let cmd = cmd.trim().to_string();
        let (word, arg) = match cmd.split_once(' ') {
            Some((w, a)) => (w, a.trim()),
            None => (cmd.as_str(), ""),
        };
        match word {
            "s" | "step" => {
                s.mode = Mode::Step;
                return Ok(());
            }
            "n" | "next" => {
                s.mode = Mode::Next(depth);
                return Ok(());
            }
            "f" | "finish" => {
                s.mode = Mode::Finish(depth);
                return Ok(());
            }
            "c" | "continue" => {
                s.mode = Mode::Continue;
                return Ok(());
            }
            "q" | "quit" => return Err(VmError::runtime_error("debugger: program stopped by the user".to_string())),
            "b" | "break" => match arg.parse::<u32>() {
                Ok(n) if n >= 1 => {
                    s.breakpoints.insert(n);
                    println!("breakpoint at line {}", n);
                }
                _ => println!("usage: b LINE"),
            },
            "d" | "delete" => match arg.parse::<u32>() {
                Ok(n) => println!("{}", if s.breakpoints.remove(&n) { "deleted" } else { "no breakpoint there" }),
                _ => println!("usage: d LINE"),
            },
            "bl" => println!("breakpoints: {:?}", s.breakpoints),
            "l" | "list" => {
                let at = arg.parse::<u32>().unwrap_or(line);
                s.list(&file, at);
            }
            "bt" | "where" => {
                println!("#0 {}:{}", file, line);
                for (i, ret) in vm.call_stack.iter().rev().enumerate() {
                    match program.line_table.lookup(ret.saturating_sub(1)) {
                        Some((f, l)) => println!("#{} called from {}:{}", i + 1, f, l),
                        None => println!("#{} called from instruction {}", i + 1, ret),
                    }
                }
            }
            "locals" => {
                // parameters are the first slots of a call frame: name them from the declaration
                let nparams = if vm.call_stack_depth() > 0 && program.param_slots {
                    std::fs::read_to_string(if file == "<script>" { s.script.as_str() } else { file.as_str() }).map(|src| crate::dap::params_at_line(&src, line as usize).len()).unwrap_or(0)
                } else {
                    0
                };
                let vals = vm.debug_locals(nparams);
                if vals.is_empty() {
                    println!("(no local slots)");
                }
                for (name, v) in vals.iter() {
                    println!("{} = {}", name, v);
                }
            }
            "globals" => {
                let mut g: Vec<_> = vm.get_globals().into_iter().filter(|(k, _)| !matches!(k.as_str(), "E" | "INF" | "PI" | "TAU")).collect();
                g.sort_by(|a, b| a.0.cmp(&b.0));
                for (k, v) in g {
                    println!("{} = {}", k, v);
                }
            }
            "p" | "print" => match vm.get_globals().get(arg) {
                Some(v) => println!("{} = {}", arg, v),
                None => println!("no global named {} (use `locals` for slots)", arg),
            },
            "" => {}
            other => println!("unknown command {} (s n f c b d bl l bt locals globals p q)", other),
        }
    }
}

/// `killer debug script.killer [--break N]...`
pub fn cmd_debug(args: &[String]) -> i32 {
    let mut file = None;
    let mut breaks = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--break" | "-b" => {
                i += 1;
                match args.get(i).and_then(|x| x.parse::<u32>().ok()) {
                    Some(n) => breaks.push(n),
                    None => {
                        eprintln!("killer debug: --break needs a line number");
                        return 2;
                    }
                }
            }
            x if x.starts_with('-') => {
                eprintln!("killer debug: unknown option {}", x);
                return 2;
            }
            x => file = Some(x.to_string()),
        }
        i += 1;
    }
    let path = match file {
        Some(f) => f,
        None => {
            eprintln!("usage: killer debug <file.killer> [--break LINE]...");
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
    let script = path.clone();
    let handle = std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(move || -> Result<(), VmError> {
        let program = crate::compiler::compile_killer_default(&source)?;
        let mut vm = VirtualMachine::new();
        vm.attach_debugger(DebugSession::new(breaks, Box::new(std::io::BufReader::new(std::io::stdin())), script));
        vm.run(&program)
    });
    match handle.map(|h| h.join()) {
        Ok(Ok(Ok(()))) => 0,
        Ok(Ok(Err(e))) => {
            eprintln!("Error: {}", e);
            1
        }
        _ => {
            eprintln!("killer debug: the program crashed");
            1
        }
    }
}
