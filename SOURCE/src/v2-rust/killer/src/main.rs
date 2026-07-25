use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::process;

use killer_native::error::VmError;
use killer_native::security::{SecurityConfig, read_file_safe};
use killer_native::inference::{resolve_model_path, list_models, install_model, migrate_local_models};

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn run_cli() -> Result<(), VmError> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        None | Some("--repl") => run_repl(),

        Some("--supernova") => killer_native::supernova::run(),

        Some("--version") | Some("-v") => {
            println!("Killer Language Runtime {} + Supernova Engine (native)",
                     killer_native::version::VERSION);
            println!("Min compatible: {}",
                     killer_native::version::MIN_COMPATIBLE_VERSION);
            Ok(())
        }

        Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }

        // -- Model registry ---------------------------------------------------
        Some("--model-list") | Some("--models") => {
            println!("{}", list_models());
            Ok(())
        }

        Some("--model-install") => {
            let src = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --model-install <path-to-file.gguf>"
                ))?;
            install_model(src)
                .map(|msg| println!("{}", msg))
                .map_err(|e| VmError::runtime_error(e))
        }

        Some("--model-migrate") => {
            println!("{}", migrate_local_models());
            Ok(())
        }

        // -- Native LLM inference ---------------------------------------------
        // Accepts: full path, relative path, short name, or fuzzy match
        Some("--model") => {
            let name = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --model <model> \"prompt\"\n  <model> can be a path or short name (e.g. qwen2.5)"
                ))?;
            let model_path = resolve_model_path(name).map_err(|e| VmError::runtime_error(e))?;
            let prompt = args.get(3).map(|s| s.as_str()).unwrap_or("Hello! What can you do?");

            killer_native::inference::killer_ask(&model_path, prompt, 512)
                .map(|answer| println!("{}", answer))
                .map_err(|e| VmError::runtime_error(e))
        }

        // -- Chat mode (auto chat template + generate) ------------------------
        Some("--chat") => {
            let name = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --chat <model> \"question\"\n  <model> can be a path or short name (e.g. qwen2.5)"
                ))?;
            let model_path = resolve_model_path(name).map_err(|e| VmError::runtime_error(e))?;
            let question = args.get(3).map(|s| s.as_str()).unwrap_or("Hello!");

            killer_native::inference::killer_chat_auto(&model_path, question, None, 512)
                .map(|_answer| ())  // answer already streamed token-by-token to stderr
                .map_err(|e| VmError::runtime_error(e))
        }

        // -- Model info (no weight loading) -----------------------------------
        Some("--model-info") => {
            let name = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --model-info <model>"
                ))?;
            let model_path = resolve_model_path(name).map_err(|e| VmError::runtime_error(e))?;

            killer_native::inference::killer_model_info(&model_path)
                .map(|info| println!("{}", info))
                .map_err(|e| VmError::runtime_error(e))
        }

        // -- RLM reasoning mode -----------------------------------------------
        Some("--think") => {
            let name = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --think <model> \"question\" [max-tokens]"
                ))?;
            let model_path = resolve_model_path(name).map_err(|e| VmError::runtime_error(e))?;
            let question = args.get(3).map(|s| s.as_str())
                .unwrap_or("Explain step by step: what is 17 × 23?");
            let max_tokens: usize = args.get(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(2048);

            killer_native::inference::killer_think(&model_path, question, max_tokens)
                .map(|resp| println!("{}", resp.display()))
                .map_err(|e| VmError::runtime_error(e))
        }

        // -- Embed text → vector ----------------------------------------------
        Some("--embed") => {
            let name = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --embed <model> \"text\""
                ))?;
            let model_path = resolve_model_path(name).map_err(|e| VmError::runtime_error(e))?;
            let text = args.get(3).map(|s| s.as_str()).unwrap_or("Hello world");

            killer_native::inference::killer_embed(&model_path, text)
                .map(|vec| {
                    println!("Dims: {}", vec.len());
                    let preview: Vec<String> = vec.iter().take(8).map(|v| format!("{:.4}", v)).collect();
                    println!("Vector (first 8): [{}{}]",
                             preview.join(", "),
                             if vec.len() > 8 { ", ..." } else { "" });
                })
                .map_err(|e| VmError::runtime_error(e))
        }

        // -- Format -----------------------------------------------------------
        Some("--format") => {
            let path = args.get(2)
                .ok_or_else(|| VmError::runtime_error(
                    "Usage: killer-native --format <file.killer>".to_string()
                ))?;
            let security = SecurityConfig::default();
            let source = read_file_safe(path, &security)?;
            let mut fmt = killer_native::formatter::Formatter::new();
            match fmt.format(&source) {
                Ok(formatted) => {
                    std::fs::write(path, &formatted)
                        .map_err(|e| VmError::runtime_error(format!("write error: {e}")))?;
                    println!("Formatted {path}");
                    Ok(())
                }
                Err(e) => Err(VmError::runtime_error(e)),
            }
        }

        // -- Test runner ------------------------------------------------------
        Some("--test") => {
            let pattern = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            killer_native::test_runner::run_test_suite(pattern)
        }

        // -- Re-run on file change (poll; Ctrl+C to exit) ---------------------
        Some("--watch") => {
            let path = args.get(2).ok_or_else(|| {
                VmError::runtime_error("Usage: killer-native --watch <file.killer>")
            })?;
            run_watch_loop(path)
        }

        Some(path) => {
            let security = SecurityConfig::default();
            let source = read_file_safe(path, &security)?;
            killer_native::run_killer_source(&source)
        }
    }
}

fn run_repl() -> Result<(), VmError> {
    println!("Killer REPL v4.0 \u{2014} type 'exit' to quit");
    println!();

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut definitions = String::new();

    loop {
        print!("killer> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            "exit" | "quit" => break,
            "help" => {
                print_repl_help();
                continue;
            }
            _ => {}
        }

        let mut input = line.clone();
        let mut brace_depth = brace_balance(&input);
        // Multi-line: continue only when the line ends with '{' and braces are unbalanced.
        if input.trim_end().ends_with('{') && brace_depth > 0 {
            while brace_depth > 0 {
                print!("  ...> ");
                io::stdout().flush().ok();

                let mut cont = String::new();
                if reader.read_line(&mut cont).unwrap_or(0) == 0 {
                    break;
                }
                brace_depth += brace_balance(&cont);
                input.push_str(&cont);
            }
        }

        let trimmed_input = input.trim();
        let is_def = trimmed_input.starts_with("kfn ")
            || trimmed_input.starts_with("fn ")
            || trimmed_input.starts_with("class ");

        let full_source = if definitions.is_empty() {
            trimmed_input.to_string()
        } else {
            format!("{}\n{}", definitions, trimmed_input)
        };

        match killer_native::run_killer_source(&full_source) {
            Ok(()) => {
                if is_def {
                    definitions.push_str(trimmed_input);
                    definitions.push('\n');
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}

fn brace_balance(s: &str) -> i32 {
    let mut depth: i32 = 0;
    for ch in s.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth
}

fn print_repl_help() {
    println!("Killer REPL Commands:");
    println!("  exit, quit    Exit the REPL");
    println!("  help          Show this help");
    println!();
    println!("Enter any Killer expression or statement to evaluate it.");
    println!("Function and class definitions are remembered across inputs.");
    println!("Multi-line input: if a line ends with '{{', more lines are read until braces balance.");
}

fn content_fingerprint(source: &str) -> u64 {
    let mut h = DefaultHasher::new();
    source.hash(&mut h);
    h.finish()
}

/// Re-run `path` whenever its contents change (polling). Blocks until Ctrl+C.
fn run_watch_loop(path: &str) -> Result<(), VmError> {
    let security = SecurityConfig::default();
    let mut last_fp: Option<u64> = None;
    loop {
        let source = read_file_safe(path, &security)?;
        let fp = content_fingerprint(&source);
        if last_fp != Some(fp) {
            last_fp = Some(fp);
            println!("\n--- {} ---\n", path);
            if let Err(e) = killer_native::run_killer_source(&source) {
                eprintln!("Error: {e}");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(900));
    }
}

fn print_help() {
    println!("Killer Native Runtime v2.1 + Supernova Engine");
    println!();
    println!("Usage:");
    println!("  killer-native                          Launch interactive REPL");
    println!("  killer-native --repl                   Launch interactive REPL (explicit)");
    println!("  killer-native --supernova              Launch embedded Supernova engine");
    println!("  killer-native <program.killer>         Run a Killer source file");
    println!("  killer-native --watch <file.killer>    Re-run when the file changes (poll)");
    println!("  killer-native --format <file.killer>    Format a source file in-place");
    println!("  killer-native --test [pattern]          Run .killer test files matching pattern");
    println!("  killer-native --version                Show version information");
    println!("  killer-native --help                   Show this help");
    println!();
    println!("Model Registry (~/.killer/models/):");
    println!("  killer-native --model-list                        List installed models");
    println!("  killer-native --model-install <file.gguf>         Install a model");
    println!("  killer-native --model-migrate                     Move .gguf files from current dir");
    println!();
    println!("Native LLM Inference  (<model> = path, filename, or short name):");
    println!("  killer-native --chat       <model> \"question\"           Chat (auto template)");
    println!("  killer-native --model      <model> \"prompt\"             Raw prompt generation");
    println!("  killer-native --model-info <model>                      Show model metadata");
    println!("  killer-native --think      <model> \"question\" [tokens]  RLM reasoning (DeepSeek-R1, QwQ)");
    println!("  killer-native --embed      <model> \"text\"               Embed text → vector");
    println!();
    println!("Examples:");
    println!("  killer-native sample_programs/hello.killer");
    println!("  killer-native --watch sample_programs/hello.killer");
    println!("  killer-native --model-install qwen2.5-0.5b-instruct-q4_k_m.gguf");
    println!("  killer-native --chat qwen2.5 \"What is 2+2?\"      # short name lookup");
    println!("  killer-native --chat tiny    \"Hello!\"             # fuzzy match tinyllama");
    println!("  killer-native --model-list");
}
