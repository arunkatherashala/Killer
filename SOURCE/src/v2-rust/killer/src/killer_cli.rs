//! **killer_cli** — CLI argument parser: flags, options, subcommands, help generation.
//!
//! Replaces the "zero CLI arg parsing" gap. Inspired by Go's `cobra` and Python's `argparse`.

use std::collections::HashMap;
use std::fmt;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// Boolean flag, e.g. `--verbose`
    Flag,
    /// String value, e.g. `--output file.txt`
    Str,
    /// Integer value, e.g. `--port 8080`
    Int,
    /// Float value, e.g. `--threshold 0.5`
    Float,
    /// Positional argument
    Positional,
}

#[derive(Debug, Clone)]
pub struct ArgDef {
    pub name: String,
    pub short: Option<char>,
    pub arg_type: ArgType,
    pub required: bool,
    pub default: Option<String>,
    pub help: String,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedArgs {
    pub command: Option<String>,
    pub flags: HashMap<String, bool>,
    pub values: HashMap<String, String>,
    pub positional: Vec<String>,
    pub errors: Vec<String>,
}

impl ParsedArgs {
    pub fn get_flag(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }

    pub fn get_str(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(|s| s.as_str())
    }

    pub fn get_int(&self, name: &str) -> Option<i64> {
        self.values.get(name).and_then(|s| s.parse().ok())
    }

    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.values.get(name).and_then(|s| s.parse().ok())
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Command definition
// ══════════════════════════════════════════════════════════════════════════════

/// A CLI command or subcommand.
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub args: Vec<ArgDef>,
    pub subcommands: Vec<Command>,
    pub version: Option<String>,
}

impl Command {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            args: Vec::new(),
            subcommands: Vec::new(),
            version: None,
        }
    }

    pub fn version(mut self, v: &str) -> Self { self.version = Some(v.to_string()); self }

    /// Add a boolean flag.
    pub fn flag(mut self, name: &str, short: Option<char>, help: &str) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short,
            arg_type: ArgType::Flag,
            required: false,
            default: None,
            help: help.to_string(),
            choices: Vec::new(),
        });
        self
    }

    /// Add a string option.
    pub fn option(mut self, name: &str, short: Option<char>, help: &str, required: bool) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short,
            arg_type: ArgType::Str,
            required,
            default: None,
            help: help.to_string(),
            choices: Vec::new(),
        });
        self
    }

    /// Add a string option with default.
    pub fn option_default(mut self, name: &str, short: Option<char>, help: &str, default: &str) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short,
            arg_type: ArgType::Str,
            required: false,
            default: Some(default.to_string()),
            help: help.to_string(),
            choices: Vec::new(),
        });
        self
    }

    /// Add an integer option.
    pub fn int_option(mut self, name: &str, short: Option<char>, help: &str, default: Option<i64>) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short,
            arg_type: ArgType::Int,
            required: default.is_none(),
            default: default.map(|d| d.to_string()),
            help: help.to_string(),
            choices: Vec::new(),
        });
        self
    }

    /// Add a positional argument.
    pub fn positional(mut self, name: &str, help: &str, required: bool) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short: None,
            arg_type: ArgType::Positional,
            required,
            default: None,
            help: help.to_string(),
            choices: Vec::new(),
        });
        self
    }

    /// Add a string option with choices.
    pub fn option_choices(mut self, name: &str, short: Option<char>, help: &str, choices: &[&str]) -> Self {
        self.args.push(ArgDef {
            name: name.to_string(),
            short,
            arg_type: ArgType::Str,
            required: false,
            default: None,
            help: help.to_string(),
            choices: choices.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add a subcommand.
    pub fn subcommand(mut self, cmd: Command) -> Self {
        self.subcommands.push(cmd);
        self
    }

    /// Parse command-line arguments.
    pub fn parse(&self, args: &[String]) -> ParsedArgs {
        let mut result = ParsedArgs {
            command: None,
            flags: HashMap::new(),
            values: HashMap::new(),
            positional: Vec::new(),
            errors: Vec::new(),
        };

        // Apply defaults
        for arg in &self.args {
            if arg.arg_type == ArgType::Flag {
                result.flags.insert(arg.name.clone(), false);
            }
            if let Some(ref def) = arg.default {
                result.values.insert(arg.name.clone(), def.clone());
            }
        }

        let mut i = 0;
        let mut positional_idx = 0;
        let positional_defs: Vec<&ArgDef> = self.args.iter()
            .filter(|a| a.arg_type == ArgType::Positional)
            .collect();

        while i < args.len() {
            let arg = &args[i];

            // Check for subcommand
            if !arg.starts_with('-') && result.command.is_none() {
                if let Some(sub) = self.subcommands.iter().find(|s| s.name == *arg) {
                    result.command = Some(sub.name.clone());
                    // Parse remaining args with subcommand definition
                    let sub_result = sub.parse(&args[i + 1..]);
                    result.flags.extend(sub_result.flags);
                    result.values.extend(sub_result.values);
                    result.positional.extend(sub_result.positional);
                    result.errors.extend(sub_result.errors);
                    return result;
                }
            }

            if arg == "--help" || arg == "-h" {
                result.flags.insert("help".to_string(), true);
                i += 1;
                continue;
            }

            if arg == "--version" || arg == "-V" {
                result.flags.insert("version".to_string(), true);
                i += 1;
                continue;
            }

            if arg.starts_with("--") {
                let name = &arg[2..];
                // Check for --name=value
                if let Some(eq_pos) = name.find('=') {
                    let key = &name[..eq_pos];
                    let val = &name[eq_pos + 1..];
                    if let Some(def) = self.find_arg(key) {
                        self.validate_and_set(&def, val, &mut result);
                    } else {
                        result.errors.push(format!("unknown option: --{}", key));
                    }
                    i += 1;
                    continue;
                }
                if let Some(def) = self.find_arg(name) {
                    if def.arg_type == ArgType::Flag {
                        result.flags.insert(name.to_string(), true);
                    } else if i + 1 < args.len() {
                        i += 1;
                        self.validate_and_set(&def, &args[i], &mut result);
                    } else {
                        result.errors.push(format!("--{} requires a value", name));
                    }
                } else {
                    result.errors.push(format!("unknown option: --{}", name));
                }
            } else if arg.starts_with('-') && arg.len() == 2 {
                let ch = arg.chars().nth(1).unwrap();
                if let Some(def) = self.find_short(ch) {
                    if def.arg_type == ArgType::Flag {
                        result.flags.insert(def.name.clone(), true);
                    } else if i + 1 < args.len() {
                        i += 1;
                        self.validate_and_set(&def, &args[i], &mut result);
                    } else {
                        result.errors.push(format!("-{} requires a value", ch));
                    }
                } else {
                    result.errors.push(format!("unknown option: -{}", ch));
                }
            } else {
                // Positional argument
                if positional_idx < positional_defs.len() {
                    result.values.insert(positional_defs[positional_idx].name.clone(), arg.clone());
                    positional_idx += 1;
                }
                result.positional.push(arg.clone());
            }
            i += 1;
        }

        // Check required args
        for arg_def in &self.args {
            if arg_def.required && !result.values.contains_key(&arg_def.name) {
                if arg_def.arg_type == ArgType::Flag { continue; }
                result.errors.push(format!("missing required argument: {}", arg_def.name));
            }
        }

        result
    }

    fn find_arg(&self, name: &str) -> Option<ArgDef> {
        self.args.iter().find(|a| a.name == name).cloned()
    }

    fn find_short(&self, ch: char) -> Option<ArgDef> {
        self.args.iter().find(|a| a.short == Some(ch)).cloned()
    }

    fn validate_and_set(&self, def: &ArgDef, value: &str, result: &mut ParsedArgs) {
        if !def.choices.is_empty() && !def.choices.iter().any(|c| c == value) {
            result.errors.push(format!("--{}: invalid choice '{}' (choose from {:?})", def.name, value, def.choices));
            return;
        }
        match def.arg_type {
            ArgType::Int => {
                if value.parse::<i64>().is_err() {
                    result.errors.push(format!("--{}: expected integer, got '{}'", def.name, value));
                    return;
                }
            }
            ArgType::Float => {
                if value.parse::<f64>().is_err() {
                    result.errors.push(format!("--{}: expected number, got '{}'", def.name, value));
                    return;
                }
            }
            _ => {}
        }
        result.values.insert(def.name.clone(), value.to_string());
    }

    /// Generate help text.
    pub fn help(&self) -> String {
        let mut s = String::new();
        if let Some(ref v) = self.version {
            s.push_str(&format!("{} v{}\n", self.name, v));
        }
        s.push_str(&format!("{}\n\n", self.description));
        s.push_str(&format!("USAGE:\n  {} [OPTIONS]", self.name));
        let positionals: Vec<&ArgDef> = self.args.iter().filter(|a| a.arg_type == ArgType::Positional).collect();
        for p in &positionals {
            if p.required { s.push_str(&format!(" <{}>", p.name)); }
            else { s.push_str(&format!(" [{}]", p.name)); }
        }
        if !self.subcommands.is_empty() {
            s.push_str(" [COMMAND]");
        }
        s.push('\n');

        if !self.subcommands.is_empty() {
            s.push_str("\nCOMMANDS:\n");
            for sub in &self.subcommands {
                s.push_str(&format!("  {:<20} {}\n", sub.name, sub.description));
            }
        }

        let flags: Vec<&ArgDef> = self.args.iter().filter(|a| a.arg_type == ArgType::Flag).collect();
        let options: Vec<&ArgDef> = self.args.iter().filter(|a| !matches!(a.arg_type, ArgType::Flag | ArgType::Positional)).collect();

        if !flags.is_empty() || !options.is_empty() {
            s.push_str("\nOPTIONS:\n");
            for f in &flags {
                let short = f.short.map(|c| format!("-{}, ", c)).unwrap_or_default();
                s.push_str(&format!("  {}--{:<18} {}\n", short, f.name, f.help));
            }
            for o in &options {
                let short = o.short.map(|c| format!("-{}, ", c)).unwrap_or_default();
                let mut line = format!("  {}--{:<18} {}", short, format!("{} <{}>", o.name, o.name), o.help);
                if let Some(ref def) = o.default {
                    line.push_str(&format!(" [default: {}]", def));
                }
                if o.required { line.push_str(" (required)"); }
                s.push_str(&line);
                s.push('\n');
            }
        }

        if !positionals.is_empty() {
            s.push_str("\nARGS:\n");
            for p in &positionals {
                let req = if p.required { " (required)" } else { "" };
                s.push_str(&format!("  {:<20} {}{}\n", format!("<{}>", p.name), p.help, req));
            }
        }

        s
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.help())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cli() -> Command {
        Command::new("killer", "The Killer programming language")
            .version("2.1.0")
            .flag("verbose", Some('v'), "Enable verbose output")
            .flag("debug", Some('d'), "Enable debug mode")
            .option("output", Some('o'), "Output file path", false)
            .int_option("port", Some('p'), "Port number", Some(8080))
            .option_choices("format", Some('f'), "Output format", &["json", "csv", "text"])
            .positional("file", "Input .killer file", true)
            .subcommand(
                Command::new("test", "Run tests")
                    .flag("parallel", None, "Run tests in parallel")
                    .option("filter", None, "Test name filter", false)
            )
            .subcommand(
                Command::new("build", "Build project")
                    .flag("release", None, "Optimized build")
            )
    }

    #[test]
    fn parse_flags_and_options() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["--verbose", "-o", "out.txt", "main.killer"]
            .into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert!(parsed.get_flag("verbose"));
        assert_eq!(parsed.get_str("output"), Some("out.txt"));
        assert_eq!(parsed.positional[0], "main.killer");
        assert!(!parsed.has_errors());
    }

    #[test]
    fn parse_default_values() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["main.killer"].into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert_eq!(parsed.get_int("port"), Some(8080)); // default
    }

    #[test]
    fn parse_subcommand() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["test", "--parallel", "--filter", "unit_"]
            .into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert_eq!(parsed.command.as_deref(), Some("test"));
        assert!(parsed.get_flag("parallel"));
        assert_eq!(parsed.get_str("filter"), Some("unit_"));
    }

    #[test]
    fn parse_invalid_choice() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["--format", "xml", "main.killer"]
            .into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert!(parsed.has_errors());
        assert!(parsed.errors[0].contains("invalid choice"));
    }

    #[test]
    fn parse_equals_syntax() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["--output=result.txt", "main.killer"]
            .into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert_eq!(parsed.get_str("output"), Some("result.txt"));
    }

    #[test]
    fn parse_missing_required() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["--verbose"].into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert!(parsed.has_errors());
        assert!(parsed.errors.iter().any(|e| e.contains("missing required")));
    }

    #[test]
    fn help_output_contains_sections() {
        let cli = sample_cli();
        let help = cli.help();
        assert!(help.contains("USAGE:"));
        assert!(help.contains("OPTIONS:"));
        assert!(help.contains("COMMANDS:"));
        assert!(help.contains("ARGS:"));
        assert!(help.contains("2.1.0"));
    }

    #[test]
    fn parse_help_flag() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["--help"].into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert!(parsed.get_flag("help"));
    }

    #[test]
    fn parse_short_flags() {
        let cli = sample_cli();
        let args: Vec<String> = vec!["-v", "-d", "main.killer"]
            .into_iter().map(String::from).collect();
        let parsed = cli.parse(&args);
        assert!(parsed.get_flag("verbose"));
        assert!(parsed.get_flag("debug"));
    }
}
