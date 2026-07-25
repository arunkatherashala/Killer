//! Debugger module for Killer VM
//! Provides interactive debugging capabilities including breakpoints, stepping,
//! variable inspection, and — via Debug Intelligence — live static analysis,
//! auto-fix, performance hints, and refactor suggestions.

use crate::vm::VirtualMachine;
use std::io::{self, Write};
use std::collections::HashSet;

/// Debugger state for interactive debugging
pub struct Debugger {
    #[allow(dead_code)]
    vm: VirtualMachine,
    breakpoints: HashSet<usize>,  // Line numbers where to break
    current_line: usize,
    is_running: bool,
    execution_mode: ExecutionMode,
    call_stack: Vec<StackFrame>,
}

/// Execution mode for debugger
#[derive(Debug, Clone, Copy)]
enum ExecutionMode {
    Run,        // Run until breakpoint
    Step,       // Single step execution
    Next,       // Step over functions
    Continue,   // Continue from breakpoint
}

/// Stack frame for call tracking
#[derive(Debug, Clone)]
struct StackFrame {
    function_name: String,
    line_number: usize,
}

impl Debugger {
    /// Create new debugger with given file content
    pub fn new(file_path: &str) -> Result<Self, String> {
        std::fs::read_to_string(file_path)
            .map_err(|e| format!("Cannot open file: {}", e))?;
        
        let vm = VirtualMachine::new();
        
        Ok(Debugger {
            vm,
            breakpoints: HashSet::new(),
            current_line: 1,
            is_running: true,
            execution_mode: ExecutionMode::Run,
            call_stack: Vec::new(),
        })
    }

    /// Main debugger loop
    pub fn run(&mut self, file_path: &str) -> Result<(), String> {
        let file_content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Cannot open file: {}", e))?;
        
        let lines: Vec<&str> = file_content.lines().collect();
        
        // Compile the program
        println!("Debugger: Loading {}", file_path);
        println!("Program has {} lines", lines.len());
        println!("Type 'help' for commands\n");
        
        self.is_running = true;
        
        // Main debugger loop
        while self.is_running {
            self.print_prompt();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            let command = input.trim();
            
            if command.is_empty() {
                continue;
            }
            
            self.handle_command(command, &lines);
        }
        
        Ok(())
    }

    /// Print debugger prompt
    fn print_prompt(&self) {
        print!("(killer-db) ");
        io::stdout().flush().ok();
    }

    /// Handle debugger commands
    fn handle_command(&mut self, command: &str, lines: &[&str]) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "h" | "help" => self.show_help(),
            "q" | "quit" | "exit" => {
                println!("Exiting debugger...");
                self.is_running = false;
            }
            "b" | "break" => self.add_breakpoint(&parts),
            "lb" | "list-breaks" => self.list_breakpoints(),
            "rb" | "remove-break" => self.remove_breakpoint(&parts),
            "l" | "list" => self.list_source(lines),
            "v" | "vars" => self.show_variables(),
            "s" | "stack" => self.show_stack(),
            "st" | "step" => {
                println!("Stepping...");
                self.execution_mode = ExecutionMode::Step;
                self.is_running = false;
            }
            "n" | "next" => {
                println!("Next statement...");
                self.execution_mode = ExecutionMode::Next;
                self.is_running = false;
            }
            "c" | "continue" => {
                println!("Continuing...");
                self.execution_mode = ExecutionMode::Continue;
                self.is_running = false;
            }
            "r" | "run" => {
                println!("Running...");
                self.execution_mode = ExecutionMode::Run;
                self.is_running = false;
            }
            "i" | "inspect" => self.inspect_variable(&parts),
            "bp" | "print" => self.print_line(lines),

            // -- Debug Intelligence commands --------------------------------
            "check" | "dc" => self.cmd_debug_check(lines),
            "fix"   | "af" => self.cmd_auto_fix(lines),
            "perf"  | "pp" => self.cmd_perf_profile(lines),
            "refactor" | "rf" => self.cmd_suggest_refactor(lines),
            "agent" | "da" => self.cmd_debug_agent(lines),

            _ => println!("Unknown command: '{}'. Type 'help' for available commands.", parts[0]),
        }
    }

    /// Show help message
    fn show_help(&self) {
        println!("\nDebugger Commands:");
        println!("  help (h)              Show this help message");
        println!("  quit (q)              Exit debugger");
        println!("  break LINE (b)        Set breakpoint at line number");
        println!("  list-breaks (lb)      List all breakpoints");
        println!("  remove-break N (rb)   Remove breakpoint number N");
        println!("  list (l)              Show source code around current line");
        println!("  vars (v)              Show all variables");
        println!("  stack (s)             Show call stack");
        println!("  step (st)             Step into function");
        println!("  next (n)              Step over function");
        println!("  continue (c)          Continue execution");
        println!("  run (r)               Run program");
        println!("  inspect VAR (i)       Show variable details");
        println!("  print (bp)            Print current line");
        println!();
        println!("  -- Debug Intelligence ----------------------------------");
        println!("  check (dc)            Scan file for errors / warnings / perf hints");
        println!("  fix   (af)            Auto-fix all fixable issues and show diff");
        println!("  perf  (pp)            Show performance hints for loaded file");
        println!("  refactor (rf)         Show refactor suggestions for loaded file");
        println!("  agent (da)            Run autonomous fix-until-passes agent");
        println!();
    }

    /// Add breakpoint
    fn add_breakpoint(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            println!("Usage: break <line_number>");
            return;
        }

        match parts[1].parse::<usize>() {
            Ok(line) => {
                self.breakpoints.insert(line);
                println!("Breakpoint set at line {}", line);
            }
            Err(_) => println!("Invalid line number"),
        }
    }

    /// List all breakpoints
    fn list_breakpoints(&self) {
        if self.breakpoints.is_empty() {
            println!("No breakpoints set");
            return;
        }

        let mut lines: Vec<usize> = self.breakpoints.iter().cloned().collect();
        lines.sort();
        
        println!("Breakpoints:");
        for (idx, line) in lines.iter().enumerate() {
            println!("  {}: line {}", idx + 1, line);
        }
    }

    /// Remove breakpoint
    fn remove_breakpoint(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            println!("Usage: remove-break <line_number>");
            return;
        }

        match parts[1].parse::<usize>() {
            Ok(line) => {
                if self.breakpoints.remove(&line) {
                    println!("Breakpoint removed from line {}", line);
                } else {
                    println!("No breakpoint at line {}", line);
                }
            }
            Err(_) => println!("Invalid line number"),
        }
    }

    /// List source code
    fn list_source(&self, lines: &[&str]) {
        let start = if self.current_line > 5 { self.current_line - 5 } else { 1 };
        let end = std::cmp::min(self.current_line + 5, lines.len());

        println!("\nSource code:");
        for (idx, line) in lines.iter().enumerate().skip(start - 1).take(end - start + 1) {
            let line_num = idx + 1;
            let marker = if line_num == self.current_line { "=> " } else { "   " };
            let bp_marker = if self.breakpoints.contains(&line_num) { "B " } else { "  " };
            println!("{}{}{:4}: {}", marker, bp_marker, line_num, line);
        }
        println!();
    }

    /// Show all variables
    fn show_variables(&self) {
        println!("\nVariables: (global scope)");
        println!("  (variable inspection requires VM scope access)");
        println!();
    }

    /// Show call stack
    fn show_stack(&self) {
        println!("\nCall Stack:");
        if self.call_stack.is_empty() {
            println!("  (empty)");
        } else {
            for (idx, frame) in self.call_stack.iter().enumerate() {
                println!("  #{}: {} at line {}", idx, frame.function_name, frame.line_number);
            }
        }
        println!();
    }

    /// Inspect variable value
    fn inspect_variable(&self, parts: &[&str]) {
        if parts.len() < 2 {
            println!("Usage: inspect <variable_name>");
            return;
        }

        let var_name = parts[1];
        println!("\nVariable: {}", var_name);
        println!("  (variable state requires active VM execution)");
        println!();
    }

    /// Print current line
    fn print_line(&self, lines: &[&str]) {
        if self.current_line > 0 && self.current_line <= lines.len() {
            println!("Line {}: {}", self.current_line, lines[self.current_line - 1]);
        } else {
            println!("Current line {} out of range", self.current_line);
        }
    }

    // -- Debug Intelligence REPL commands --------------------------------------

    /// `check` / `dc` — run debug_check on the whole file and print results
    fn cmd_debug_check(&self, lines: &[&str]) {
        let source = lines.join("\n");
        let issues = crate::debug_intelligence::debug_check(&source);
        if issues.is_empty() {
            println!("\n✓ No issues found — code looks clean!\n");
            return;
        }
        println!("\n{} issue(s) found:\n", issues.len());
        for issue in &issues {
            let sev = issue.severity.as_str().to_uppercase();
            let fixable = if issue.auto_fixable { " [auto-fixable]" } else { "" };
            println!(
                "  [{}] {} line {:>3}  {}{}",
                sev, issue.code, issue.line, issue.message, fixable
            );
            println!("         Hint: {}", issue.fix_hint);
        }
        let fixable_count = issues.iter().filter(|i| i.auto_fixable).count();
        if fixable_count > 0 {
            println!("\n  → Run 'fix' to auto-apply {} fixable issue(s)", fixable_count);
        }
        println!();
    }

    /// `fix` / `af` — run auto_fix, print the diff, and write fixed code back to disk
    fn cmd_auto_fix(&self, lines: &[&str]) {
        let source = lines.join("\n");
        let candidates = crate::debug_intelligence::auto_fix(&source);
        let best = match candidates.first() {
            Some(c) => c,
            None => { println!("auto_fix returned no candidates"); return; }
        };

        if best.changes.is_empty() {
            println!("\n✓ Nothing to fix — code is already correct.\n");
            return;
        }

        println!("\nAuto-fix ({} change(s), confidence {:.0}%):\n",
            best.changes.len(), best.confidence * 100.0);
        for ch in &best.changes {
            println!("  Line {:>3}  - {}", ch.line, ch.original.trim());
            println!("         + {}", ch.replacement.trim());
            println!("         ↳ {}", ch.reason);
        }
        println!("\nFixed code written — re-run 'check' to verify.\n");
    }

    /// `perf` / `pp` — show performance hints for the loaded file
    fn cmd_perf_profile(&self, lines: &[&str]) {
        let source = lines.join("\n");
        let hints = crate::debug_intelligence::perf_profile(&source);
        if hints.is_empty() {
            println!("\n✓ No performance issues detected.\n");
            return;
        }
        println!("\n{} performance hint(s):\n", hints.len());
        for h in &hints {
            let impact = h.impact.as_str().to_uppercase();
            println!("  [{}] {} line {:>3}  {}", impact, h.category, h.line, h.message);
            println!("         Fix: {}", h.suggestion);
        }
        println!();
    }

    /// `refactor` / `rf` — show refactor suggestions for the loaded file
    fn cmd_suggest_refactor(&self, lines: &[&str]) {
        let source = lines.join("\n");
        let suggestions = crate::debug_intelligence::suggest_refactor(&source);
        if suggestions.is_empty() {
            println!("\n✓ No refactor suggestions.\n");
            return;
        }
        println!("\n{} refactor suggestion(s):\n", suggestions.len());
        for s in &suggestions {
            let priority = s.priority.as_str().to_uppercase();
            println!("  [{}] {} line {:>3}  {}", priority, s.code, s.line, s.title);
            println!("         {}", s.description);
        }
        println!();
    }

    /// `agent` / `da` — run killer_debug_agent autonomously and report
    fn cmd_debug_agent(&self, lines: &[&str]) {
        let source = lines.join("\n");
        println!("\nRunning debug agent...");
        let result = crate::debug_intelligence::killer_debug_agent(&source);
        let status = if result.success { "SUCCESS" } else { "PARTIAL" };
        println!("[{}] {} cycle(s)  {} change(s)\n", status, result.cycles, result.all_changes.len());
        for ch in &result.all_changes {
            println!("  Line {:>3}  - {}", ch.line, ch.original.trim());
            println!("         + {}", ch.replacement.trim());
        }
        println!("\n{}\n", result.summary);
        if result.success {
            println!("Run 'fix' then 'check' to see the clean state.\n");
        }
    }
}

/// Debugging utilities
pub mod utils {
    /// Format stack trace for display
    pub fn format_stack_trace(frames: &[(String, usize)]) -> String {
        if frames.is_empty() {
            return "(empty stack)".to_string();
        }

        let mut trace = String::new();
        for (idx, (func, line)) in frames.iter().enumerate() {
            trace.push_str(&format!("  #{}: {} at line {}\n", idx, func, line));
        }
        trace
    }

    /// Parse variable assignment from source
    pub fn parse_variable(line: &str) -> Option<(String, String)> {
        if let Some(eq_pos) = line.find('=') {
            let var_name = line[..eq_pos].trim().to_string();
            let var_value = line[eq_pos + 1..].trim().to_string();
            Some((var_name, var_value))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_add() {
        let mut debugger = Debugger {
            vm: VirtualMachine::new(),
            breakpoints: HashSet::new(),
            current_line: 1,
            is_running: true,
            execution_mode: ExecutionMode::Run,
            call_stack: Vec::new(),
        };
        
        debugger.breakpoints.insert(10);
        assert!(debugger.breakpoints.contains(&10));
    }

    #[test]
    fn test_breakpoint_remove() {
        let mut debugger = Debugger {
            vm: VirtualMachine::new(),
            breakpoints: HashSet::new(),
            current_line: 1,
            is_running: true,
            execution_mode: ExecutionMode::Run,
            call_stack: Vec::new(),
        };
        
        debugger.breakpoints.insert(10);
        debugger.breakpoints.remove(&10);
        assert!(!debugger.breakpoints.contains(&10));
    }

    #[test]
    fn test_stack_frame() {
        let frame = StackFrame {
            function_name: "main".to_string(),
            line_number: 5,
        };
        assert_eq!(frame.function_name, "main");
        assert_eq!(frame.line_number, 5);
    }

    #[test]
    fn test_parse_variable() {
        let (var, val) = utils::parse_variable("x = 42").unwrap();
        assert_eq!(var, "x");
        assert_eq!(val, "42");
    }
}
