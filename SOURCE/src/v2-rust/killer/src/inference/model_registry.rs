// inference/model_registry.rs — Killer Model Registry
//
// Stores GGUF models in ~/.killer/models/ — same idea as Ollama (~/.ollama/models)
// but lighter.  No server, no daemon — just a known directory the CLI searches.
//
// Resolution order for model names:
//   1. Exact file path (absolute or relative) — if it ends in .gguf or exists as-is
//   2. Current working directory — <name>.gguf
//   3. ~/.killer/models/<name>.gguf
//   4. ~/.killer/models/<name>  (no extension)
//   5. Fuzzy match: any .gguf in ~/.killer/models/ whose name contains <name>
//
// Usage from CLI:
//   killer-native --chat qwen2.5          → resolves to ~/.killer/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
//   killer-native --chat ./my.gguf        → uses ./my.gguf directly
//   killer-native --model-list            → lists installed models
//   killer-native --model-install <file>  → copies a .gguf into ~/.killer/models/

use std::path::{Path, PathBuf};
use std::fs;

// --- Registry directory -------------------------------------------------------

/// Returns `~/.killer/models/`, creating it if it doesn't exist.
pub fn killer_models_dir() -> PathBuf {
    let home = home_dir();
    let dir  = home.join(".killer").join("models");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Platform-safe home directory.
fn home_dir() -> PathBuf {
    // Try HOME (Unix) or USERPROFILE (Windows)
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

// --- Path resolution ----------------------------------------------------------

/// Resolve a model name/path to an actual file path that exists.
///
/// Accepts:
/// - Full absolute path:     `C:\models\qwen.gguf`
/// - Relative path:          `./qwen.gguf` or `qwen.gguf`
/// - Short name:             `qwen2.5`  → searches ~/.killer/models/
/// - Partial filename:       `tiny`     → fuzzy matches tinyllama*.gguf
///
/// Returns `Ok(resolved_path_string)` or `Err(helpful_message)`.
pub fn resolve_model_path(name: &str) -> Result<String, String> {
    let p = Path::new(name);

    // 1. Absolute path — use as-is
    if p.is_absolute() {
        if p.exists() {
            return Ok(name.to_string());
        }
        return Err(format!(
            "Model not found: {}\nCheck the path is correct.", name
        ));
    }

    // 2. Exact relative path (includes ./ prefix or direct filename with extension)
    if p.exists() {
        return Ok(name.to_string());
    }

    // 3. Try adding .gguf to relative path
    let with_ext = format!("{}.gguf", name);
    if Path::new(&with_ext).exists() {
        return Ok(with_ext);
    }

    // 4. Search ~/.killer/models/ for exact filename
    let models_dir = killer_models_dir();
    let exact = models_dir.join(name);
    if exact.exists() {
        return Ok(exact.to_string_lossy().to_string());
    }
    let exact_gguf = models_dir.join(format!("{}.gguf", name));
    if exact_gguf.exists() {
        return Ok(exact_gguf.to_string_lossy().to_string());
    }

    // 5. Fuzzy match: find any .gguf whose stem contains the name (case-insensitive)
    let name_lower = name.to_lowercase();
    if let Ok(entries) = fs::read_dir(&models_dir) {
        let mut matches: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map(|e| e == "gguf").unwrap_or(false)
                && p.file_name()
                   .and_then(|n| n.to_str())
                   .map(|n| n.to_lowercase().contains(&name_lower))
                   .unwrap_or(false)
            })
            .collect();

        matches.sort(); // deterministic: pick alphabetically first

        if matches.len() == 1 {
            return Ok(matches[0].to_string_lossy().to_string());
        }
        if matches.len() > 1 {
            let names: Vec<String> = matches.iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .collect();
            return Err(format!(
                "Ambiguous model name '{}' — multiple matches in ~/.killer/models/:\n  {}\nUse a more specific name.",
                name, names.join("\n  ")
            ));
        }
    }

    // Nothing found — helpful error with install instructions
    Err(format!(
        "Model '{}' not found.\n\
         \n\
         Searched:\n\
           • Current directory: {}.gguf\n\
           • Model registry:    {}\n\
         \n\
         To install a model:\n\
           1. Download a .gguf from https://huggingface.co/TheBloke\n\
           2. Run: killer-native --model-install <path-to-file.gguf>\n\
         \n\
         Recommended models:\n\
           qwen2.5-0.5b-instruct-q4_k_m.gguf   ~469 MB  (fast)\n\
           tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf ~638 MB  (stronger)",
        name,
        name,
        models_dir.display()
    ))
}

/// Default GGUF for KhLM Ask and **AI System** (neural advisor + coordinator synthesis) when no path is supplied.
///
/// Prefer a **reasoning** `.gguf` (e.g. DeepSeek-R1–style) for synthesis quality.
///
/// 1. `KILLER_KHLM_GGUF` — short name or path, resolved via [`resolve_model_path`].
/// 2. Else the first `.gguf` under `~/.killer/models/` (sorted paths, deterministic).
pub fn pick_default_gguf_for_khlm() -> Option<String> {
    if let Ok(name) = std::env::var("KILLER_KHLM_GGUF") {
        let name = name.trim();
        if !name.is_empty() {
            if let Ok(path) = resolve_model_path(name) {
                return Some(path);
            }
        }
    }
    let models_dir = killer_models_dir();
    let mut paths: Vec<PathBuf> = match fs::read_dir(&models_dir) {
        Ok(e) => e
            .filter_map(|ent| ent.ok())
            .map(|ent| ent.path())
            .filter(|p| p.extension().map(|ex| ex == "gguf").unwrap_or(false))
            .collect(),
        Err(_) => return None,
    };
    if paths.is_empty() {
        return None;
    }
    paths.sort();
    paths
        .into_iter()
        .next()
        .map(|p| p.to_string_lossy().to_string())
}

// --- Model management ---------------------------------------------------------

/// Install a model: copy a .gguf file into ~/.killer/models/.
pub fn install_model(src_path: &str) -> Result<String, String> {
    let src = Path::new(src_path);
    if !src.exists() {
        return Err(format!("File not found: {}", src_path));
    }
    if src.extension().map(|e| e != "gguf").unwrap_or(true) {
        return Err(format!("'{}' is not a .gguf file.", src_path));
    }

    let models_dir = killer_models_dir();
    let filename   = src.file_name().ok_or("Invalid filename")?;
    let dest       = models_dir.join(filename);

    if dest.exists() {
        return Ok(format!("Already installed: {}", dest.display()));
    }

    fs::copy(src, &dest)
        .map_err(|e| format!("Failed to install: {}", e))?;

    let size_mb = fs::metadata(&dest)
        .map(|m| m.len() / (1024 * 1024))
        .unwrap_or(0);

    Ok(format!("Installed: {}  ({} MB)\nLocation: {}",
        filename.to_string_lossy(), size_mb, dest.display()))
}

/// List all installed models in ~/.killer/models/.
pub fn list_models() -> String {
    let models_dir = killer_models_dir();

    let entries: Vec<_> = match fs::read_dir(&models_dir) {
        Err(_) => return format!("No models installed yet.\nRegistry: {}\n\nInstall with: killer-native --model-install <file.gguf>", models_dir.display()),
        Ok(e)  => e.filter_map(|e| e.ok()).collect(),
    };

    let mut models: Vec<(String, u64)> = entries.iter()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "gguf").unwrap_or(false))
        .map(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let size = fs::metadata(&p).map(|m| m.len() / (1024 * 1024)).unwrap_or(0);
            (name, size)
        })
        .collect();

    if models.is_empty() {
        return format!(
            "No models installed yet.\nRegistry: {}\n\nInstall with: killer-native --model-install <file.gguf>",
            models_dir.display()
        );
    }

    models.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = format!("Installed models ({})\n", models_dir.display());
    out.push_str(&"-".repeat(60));
    out.push('\n');
    for (name, mb) in &models {
        // Extract short alias hint (strip q4_k_m, Q8_0 etc. suffixes)
        let alias = short_alias(name);
        out.push_str(&format!("  {:50} {:>6} MB   alias: {}\n", name, mb, alias));
    }
    out.push_str(&"-".repeat(60));
    out.push_str(&format!("\n{} model(s) installed  |  Use short alias with --chat / --model\n", models.len()));
    out
}

/// Derive a short alias from a long model filename.
/// e.g. "qwen2.5-0.5b-instruct-q4_k_m.gguf" → "qwen2.5"
fn short_alias(filename: &str) -> String {
    let stem = filename.trim_end_matches(".gguf");
    // Remove quantization suffix (q4_k_m, Q8_0, q4_0, etc.)
    let re_parts: Vec<&str> = stem.split('-').collect();
    let meaningful: Vec<&str> = re_parts.iter()
        .take_while(|p| !p.to_lowercase().starts_with('q') || p.len() < 3)
        .copied()
        .collect();
    if meaningful.is_empty() { stem.to_string() }
    else { meaningful.join("-") }
}

// --- Move models from current dir to registry --------------------------------

/// Scan the current directory for any .gguf files and offer to move them
/// to ~/.killer/models/.  Returns a summary of what was moved.
pub fn migrate_local_models() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let models_dir = killer_models_dir();
    let mut moved = Vec::new();
    let mut errors = Vec::new();

    let entries = match fs::read_dir(&cwd) {
        Ok(e)  => e,
        Err(e) => return format!("Cannot read current directory: {}", e),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e == "gguf").unwrap_or(false) {
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let dest = models_dir.join(&filename);
            if dest.exists() {
                moved.push(format!("  {} (already in registry, skipped)", filename));
                continue;
            }
            match fs::rename(&path, &dest) {
                Ok(_)  => moved.push(format!("  {} → {}", filename, dest.display())),
                Err(_) => {
                    // rename fails across drives — fall back to copy+delete
                    match fs::copy(&path, &dest) {
                        Ok(_) => {
                            let _ = fs::remove_file(&path);
                            moved.push(format!("  {} → {}", filename, dest.display()));
                        }
                        Err(e) => errors.push(format!("  {} FAILED: {}", filename, e)),
                    }
                }
            }
        }
    }

    let mut out = String::new();
    if moved.is_empty() && errors.is_empty() {
        out.push_str("No .gguf files found in current directory.");
    } else {
        if !moved.is_empty() {
            out.push_str(&format!("Moved {} model(s) to {}:\n", moved.len(), models_dir.display()));
            out.push_str(&moved.join("\n"));
        }
        if !errors.is_empty() {
            out.push_str("\nErrors:\n");
            out.push_str(&errors.join("\n"));
        }
    }
    out.push_str(&format!("\n\nUse 'killer-native --model-list' to see installed models."));
    out
}
