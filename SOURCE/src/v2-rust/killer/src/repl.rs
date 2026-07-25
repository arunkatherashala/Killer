/// REPL (Read-Eval-Print Loop) for Killer VM
/// Interactive shell for testing Killer code in real-time

use std::io::{self, BufRead, Write};
use std::collections::HashMap;
use crate::{run_killer_source, VirtualMachine, VmError};

pub struct Repl {
    #[allow(dead_code)]
    vm: VirtualMachine,
    global_scope: String,
    history: Vec<String>,
    variables: HashMap<String, VarInfo>,  // Track variable names and types
}

/// Variable information for better inspection
#[derive(Debug, Clone)]
struct VarInfo {
    name: String,
    line_defined: usize,
    last_value: String,
}

impl Repl {
    pub fn new() -> Self {
        Repl {
            vm: VirtualMachine::new(),
            global_scope: String::new(),
            history: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), VmError> {
        self.print_welcome();
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut line_buffer = String::new();
        let mut input_buffer = String::new();

        loop {
            // Print prompt
            print!("killer-vm> ");
            io::stdout().flush().ok();

            // Read line
            line_buffer.clear();
            if reader.read_line(&mut line_buffer).is_err() {
                break;
            }

            let trimmed = line_buffer.trim();

            // Handle empty input
            if trimmed.is_empty() {
                continue;
            }

            // Handle special commands
            if trimmed.starts_with(':') {
                if self.handle_command(trimmed)? {
                    break; // :exit was called
                }
                continue;
            }

            // Accumulate input for multi-line statements
            input_buffer.push_str(trimmed);

            // Check if statement is complete
            if self.is_complete_statement(&input_buffer) {
                // Add to history
                self.history.push(input_buffer.clone());

                // Execute
                match self.execute(&input_buffer) {
                    Ok(output) => {
                        if !output.is_empty() {
                            println!("=> {}", output);
                        } else {
                            println!("✓ OK");  // Indicate successful execution
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Error: {}", e);
                    }
                }

                input_buffer.clear();
            } else {
                // Continue reading
                input_buffer.push('\n');
                print!("     > ");
                io::stdout().flush().ok();
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, cmd: &str) -> Result<bool, VmError> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            ":exit" | ":quit" | ":q" => {
                println!("Goodbye!");
                Ok(true)
            }
            ":help" => {
                self.print_help();
                Ok(false)
            }
            ":clear" => {
                self.global_scope.clear();
                self.variables.clear();
                println!("✓ Cleared all variables and state");
                Ok(false)
            }
            ":history" => {
                if self.history.is_empty() {
                    println!("(no history)");
                } else {
                    println!("\nCommand History:");
                    for (i, cmd) in self.history.iter().enumerate() {
                        println!("  {:3}. {}", i + 1, cmd);
                    }
                    println!();
                }
                Ok(false)
            }
            ":vars" => {
                self.show_variables();
                Ok(false)
            }
            ":inspect" => {
                if parts.len() < 2 {
                    println!("Usage: :inspect <variable_name>");
                } else {
                    self.inspect_variable(parts[1]);
                }
                Ok(false)
            }
            ":stack" => {
                println!("Stack depth: {} (variable scope tracking)", self.variables.len());
                Ok(false)
            }
            _ => {
                eprintln!("Unknown command '{}'. Type :help for available commands.", parts[0]);
                Ok(false)
            }
        }
    }

    fn show_variables(&self) {
        if self.variables.is_empty() {
            println!("(no variables defined)");
        } else {
            println!("\nGlobal Variables ({} defined):", self.variables.len());
            let mut vars: Vec<_> = self.variables.values().collect();
            vars.sort_by(|a, b| a.line_defined.cmp(&b.line_defined));
            for var in vars {
                println!("  {} = {} (defined at line {})", var.name, var.last_value, var.line_defined);
            }
            println!();
        }
    }

    fn inspect_variable(&self, name: &str) {
        if let Some(var) = self.variables.get(name) {
            println!("\n+- Variable: {}", name);
            println!("+- Value: {}", var.last_value);
            println!("+- Defined at line: {}", var.line_defined);
            println!("+- Active: yes");
        } else {
            println!("✗ Variable '{}' not found in scope", name);
        }
    }

    fn execute(&mut self, code: &str) -> Result<String, VmError> {
        // Extract and track variable assignments
        self.extract_variable_info(code);

        // Add to global scope
        self.global_scope.push_str("\n");
        self.global_scope.push_str(code);

        // Try to parse and execute combined source
        let source = self.global_scope.clone();

        // Compile and run
        match compile_and_run(&source) {
            Ok(_result) => {
                // Return empty - execution side effects (like print) handle output
                Ok(String::new())
            }
            Err(e) => {
                // If it fails, remove the last addition to global scope
                self.global_scope = source.trim_end_matches(code).to_string();
                Err(e)
            }
        }
    }

    fn extract_variable_info(&mut self, code: &str) {
        // Simple pattern matching for variable assignments
        // Handles: x = 5, name = "hello", arr = [1,2,3], etc.
        let trimmed = code.trim();
        
        // Look for simple assignments: identifier = expression
        if let Some(eq_pos) = trimmed.find('=') {
            if !trimmed[..eq_pos].contains('=') && !trimmed[..eq_pos].contains('!') {
                let var_name = trimmed[..eq_pos].trim().to_string();
                let var_value = trimmed[eq_pos + 1..].trim().to_string();
                
                // Validate variable name (simple check)
                if !var_name.is_empty() && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    let line_num = self.global_scope.lines().count();
                    self.variables.insert(
                        var_name.clone(),
                        VarInfo {
                            name: var_name,
                            line_defined: line_num,
                            last_value: var_value,
                        },
                    );
                }
            }
        }
    }

    fn is_complete_statement(&self, code: &str) -> bool {
        // Simple heuristic: check for unmatched braces/parens
        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();
        let open_parens = code.matches('(').count();
        let close_parens = code.matches(')').count();
        let open_brackets = code.matches('[').count();
        let close_brackets = code.matches(']').count();

        open_braces == close_braces
            && open_parens == close_parens
            && open_brackets == close_brackets
            && !code.trim().ends_with('\\')
    }

    fn print_welcome(&self) {
        println!("+===============================================+");
        println!("|  Killer VM Interactive REPL v2.1 (Enhanced)   |");
        println!("|  Type :help for commands, :exit to quit       |");
        println!("+===============================================+");
        println!();
    }

    fn print_help(&self) {
        println!("\n+-------------------------------------------------+");
        println!("|  Killer VM REPL - Available Commands            |");
        println!("+-------------------------------------------------+\n");
        
        println!("Core Commands:");
        println!("  :help          Show this help message");
        println!("  :exit          Exit REPL (:quit, :q)");
        println!("  :clear         Clear all variables and reset state");
        println!("");
        
        println!("Variable Inspection:");
        println!("  :vars          Show all defined variables");
        println!("  :inspect VAR   Show detailed variable information");
        println!("  :stack         Show variable scope depth");
        println!("");
        
        println!("Debugging:");
        println!("  :history       Show recent command history");
        println!("");
        
        println!("Examples:");
        println!("  killer-vm> x = 42");
        println!("  killer-vm> y = x * 2");
        println!("  killer-vm> :vars");
        println!("  killer-vm> :inspect y");
        println!("  killer-vm> print([1, 2, 3].map(i => i * 2))");
        println!();
    }
}

/// Compile and run code, returning the last expression result
fn compile_and_run(source: &str) -> Result<String, VmError> {
    // Use the main runner which handles all compilation
    run_killer_source(source)?;
    
    // Return empty string (REPL doesn't show stack, just side effects like print)
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_statement() {
        let repl = Repl::new();
        assert!(repl.is_complete_statement("x = 5"));
        assert!(repl.is_complete_statement("fn f() { return 1; }"));
        assert!(!repl.is_complete_statement("fn f() { return 1;"));
        assert!(!repl.is_complete_statement("[1, 2,"));
    }
}
