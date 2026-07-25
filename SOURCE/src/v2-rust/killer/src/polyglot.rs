// NOVA GALAXY ENGINE v1
// Polyglot @lang{} Runtime - Write any language inside a .killer file
//
// Architecture:
//   @python { ... }  ->  Ghost Agent detects Python -> executes -> returns stdout
//   @java   { ... }  ->  Ghost Agent detects Java   -> executes -> returns stdout
//   @bash   { ... }  ->  Ghost Agent detects Bash   -> executes -> returns stdout
//   @node   { ... }  ->  Ghost Agent detects Node   -> executes -> returns stdout
//   @ruby   { ... }  ->  Ghost Agent detects Ruby   -> executes -> returns stdout
//   @go     { ... }  ->  Ghost Agent detects Go     -> executes -> returns stdout
//   @ps     { ... }  ->  Ghost Agent detects PowerShell -> executes -> returns stdout
//
// Ghost Agent execution pipeline (Phase 1 - Runtime Bridge):
//   1. Accept code block as &str
//   2. Write to secure temp file
//   3. Spawn runtime subprocess with AssassinLayer limits
//   4. Capture stdout
//   5. Clean up temp file
//   6. Return result to Killer scope
//
// Future phases (Ghost Agent -> Nova Galaxy Engine v2+):
//   Phase 2: Python->Killer IR transpiler (no Python install needed)
//   Phase 3: JS/Ruby->Killer IR transpiler
//   Phase 4: Full native multi-language VM - black hole engine
//
// Zero external crates - pure std::process::Command + std::fs

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

// --- Language Descriptor -----------------------------------------------------

/// Describes how to run a specific language.
#[derive(Debug, Clone)]
pub struct LangRuntime {
    /// Language name as used in @lang{}
    pub name: &'static str,
    /// Candidate executables to try (first found wins)
    pub executables: &'static [&'static str],
    /// File extension for temp file
    pub ext: &'static str,
    /// Args before the temp file path (e.g. ["run"] for go run)
    pub pre_args: &'static [&'static str],
    /// Args after the temp file path
    pub post_args: &'static [&'static str],
    /// Max execution time in seconds
    pub timeout_s: u64,
    /// Args used to detect the runtime exists (checks exit code = 0).
    /// Most runtimes: &["--version"].
    /// Go uses &["version"] (not --version).
    /// PowerShell uses &["-NoLogo", "-NonInteractive", "-Command", "exit 0"].
    pub detect_args: &'static [&'static str],
}

/// All supported languages - Ghost Agent tries each executable in order.
static RUNTIMES: &[LangRuntime] = &[
    LangRuntime {
        name: "python",
        executables: &["python3", "python", "py"],
        ext: "py",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "python3",
        executables: &["python3", "python"],
        ext: "py",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "js",
        executables: &["node", "node.exe", "nodejs"],
        ext: "js",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "node",
        executables: &["node", "node.exe", "nodejs"],
        ext: "js",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "ruby",
        executables: &["ruby", "ruby3", "ruby.exe"],
        ext: "rb",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "go",
        executables: &["go"],
        ext: "go",
        pre_args: &["run"],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["version"], // `go version`, not `go --version`
    },
    LangRuntime {
        name: "bash",
        executables: &["bash", "sh"],
        ext: "sh",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "sh",
        executables: &["sh", "bash"],
        ext: "sh",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "ps",
        executables: &["powershell", "pwsh"],
        ext: "ps1",
        pre_args: &["-File"],
        post_args: &[],
        timeout_s: 30,
        // PowerShell 5.1 doesn't support --version; use -Command instead
        detect_args: &["-NoLogo", "-NonInteractive", "-Command", "exit 0"],
    },
    LangRuntime {
        name: "powershell",
        executables: &["pwsh", "powershell"],
        ext: "ps1",
        pre_args: &["-File"],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["-NoLogo", "-NonInteractive", "-Command", "exit 0"],
    },
    LangRuntime {
        name: "java",
        executables: &["java"],
        ext: "java",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "ts",
        executables: &["npx", "ts-node"],
        ext: "ts",
        pre_args: &["ts-node"],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "typescript",
        executables: &["npx", "ts-node"],
        ext: "ts",
        pre_args: &["ts-node"],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "rust",
        executables: &["rustc"],
        ext: "rs",
        pre_args: &[],
        post_args: &[],
        timeout_s: 120,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "lua",
        executables: &["lua", "lua5.4", "lua5.3"],
        ext: "lua",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "perl",
        executables: &["perl", "perl.exe"],
        ext: "pl",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "php",
        executables: &["php", "php8", "php7"],
        ext: "php",
        pre_args: &[],
        post_args: &[],
        timeout_s: 30,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "r",
        executables: &["Rscript", "rscript"],
        ext: "R",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    // -- D: C / C++ (compile + run via gcc/g++/clang) -------------------------
    LangRuntime {
        name: "c",
        executables: &["gcc", "clang", "cc"],
        ext: "c",
        pre_args: &[],  // special-cased in polyglot_exec
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "cpp",
        executables: &["g++", "clang++", "c++"],
        ext: "cpp",
        pre_args: &[],  // special-cased in polyglot_exec
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    LangRuntime {
        name: "c++",
        executables: &["g++", "clang++", "c++"],
        ext: "cpp",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    // -- D: Swift --------------------------------------------------------------
    LangRuntime {
        name: "swift",
        executables: &["swift"],
        ext: "swift",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    // -- D: Kotlin (requires kotlinc) ------------------------------------------
    LangRuntime {
        name: "kotlin",
        executables: &["kotlin", "kotlinc-jvm"],
        ext: "kts",
        pre_args: &["-script"],  // kotlinc -script file.kts for scripting
        post_args: &[],
        timeout_s: 120,
        detect_args: &["-version"],
    },
    // -- D: Scala --------------------------------------------------------------
    LangRuntime {
        name: "scala",
        executables: &["scala", "scala3"],
        ext: "sc",
        pre_args: &[],
        post_args: &[],
        timeout_s: 120,
        detect_args: &["-version"],
    },
    // -- D: Elixir -------------------------------------------------------------
    LangRuntime {
        name: "elixir",
        executables: &["elixir"],
        ext: "exs",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    // -- D: Haskell ------------------------------------------------------------
    LangRuntime {
        name: "haskell",
        executables: &["runghc", "runhaskell"],
        ext: "hs",
        pre_args: &[],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["--version"],
    },
    // -- D: Zig ----------------------------------------------------------------
    LangRuntime {
        name: "zig",
        executables: &["zig"],
        ext: "zig",
        pre_args: &["run"],
        post_args: &[],
        timeout_s: 60,
        detect_args: &["version"],
    },
];

// --- Ghost Agent - Runtime Detection -----------------------------------------

/// Check if an executable exists using the given detection args.
/// Uses `.success()` so Windows Python stubs (which exit non-zero) are excluded.
fn is_executable_available_with(exe: &str, detect_args: &[&str]) -> bool {
    Command::new(exe)
        .args(detect_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Find runtime descriptor for a given language name.
pub fn find_runtime(lang: &str) -> Option<&'static LangRuntime> {
    let lang_lower = lang.to_lowercase();
    RUNTIMES.iter().find(|r| r.name == lang_lower)
}

/// Find which executable actually exists for a given runtime.
fn resolve_executable(runtime: &LangRuntime) -> Option<&'static str> {
    for &exe in runtime.executables {
        if is_executable_available_with(exe, runtime.detect_args) {
            return Some(exe);
        }
    }
    None
}

/// List all detected runtimes on this machine.
/// Returns vec of (lang_name, executable_path).
pub fn detect_all_runtimes() -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for rt in RUNTIMES {
        if seen.contains(rt.name) {
            continue;
        }
        if let Some(exe) = resolve_executable(rt) {
            found.push((rt.name.to_string(), exe.to_string()));
            seen.insert(rt.name);
        }
    }
    found
}

// --- Temp File Management -----------------------------------------------------

/// Write code to a secure temp file. Returns the file path.
fn write_temp_file(code: &str, ext: &str) -> Result<std::path::PathBuf, String> {
    let tmp = std::env::temp_dir();
    // Use a unique name per call to avoid collisions
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .subsec_nanos();
    let filename = format!("killer_nova_galaxy_{}.{}", id, ext);
    let path = tmp.join(&filename);
    let mut f = fs::File::create(&path).map_err(|e| format!("Cannot create temp file: {}", e))?;
    f.write_all(code.as_bytes()).map_err(|e| format!("Cannot write temp file: {}", e))?;
    Ok(path)
}

// Java Special Handling
//
// Java requires:
//   1. Class name must match file name
//   2. Compile with javac first
//   3. Then run with java ClassName
//
// The Ghost Agent auto-wraps bare Java statements in a Main class if
// no class declaration is found, so users can write:
//   @java { System.out.println("Hello!"); }   works without boilerplate

fn wrap_java_if_needed(code: &str) -> String {
    let trimmed = code.trim();
    // If code already has a class declaration, use as-is
    if trimmed.contains("class ") && trimmed.contains('{') {
        return code.to_string();
    }
    // Wrap bare statements in a runnable Main class
    format!(
        "public class KillerMain {{\n    public static void main(String[] args) {{\n{}\n    }}\n}}\n",
        code.lines().map(|l| format!("        {}", l)).collect::<Vec<_>>().join("\n")
    )
}

/// Execute Java code: compile with javac, then run with java.
fn exec_java(code: &str) -> Result<String, String> {
    // Check javac available
    if !is_executable_available_with("javac", &["--version"]) {
        return Err(
            "Java runtime not found.\n\
             Install JDK from: https://adoptium.net\n\
             Or use: @python, @node, @bash (may be available on your system)"
                .to_string(),
        );
    }

    let wrapped = wrap_java_if_needed(code);
    let tmp_dir = std::env::temp_dir();
    let java_file = tmp_dir.join("KillerMain.java");
    let mut f = fs::File::create(&java_file)
        .map_err(|e| format!("Cannot create Java temp file: {}", e))?;
    f.write_all(wrapped.as_bytes())
        .map_err(|e| format!("Cannot write Java temp file: {}", e))?;

    // Compile
    let compile = Command::new("javac")
        .arg(&java_file)
        .arg("-d")
        .arg(&tmp_dir)
        .output()
        .map_err(|e| format!("javac failed to start: {}", e))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(format!("Java compile error:\n{}", stderr));
    }

    // Run
    let output = Command::new("java")
        .arg("-cp")
        .arg(&tmp_dir)
        .arg("KillerMain")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("java failed to start: {}", e))?;

    // Clean up
    let _ = fs::remove_file(&java_file);
    let _ = fs::remove_file(tmp_dir.join("KillerMain.class"));

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Java runtime error:\n{}", stderr))
    }
}

// --- Rust Special Handling ---------------------------------------------------
//
// Rust requires compile + run. Ghost Agent wraps bare expressions in fn main().

fn wrap_rust_if_needed(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.contains("fn main(") {
        return code.to_string();
    }
    format!("fn main() {{\n{}\n}}\n", code.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"))
}

// --- C Special Handling -----------------------------------------------------
//
// C requires compile + run. Ghost Agent wraps bare statements in a main() if
// no main() is found.

fn wrap_c_if_needed(code: &str) -> String {
    let t = code.trim();
    if t.contains("int main(") || t.contains("void main(") {
        return code.to_string();
    }
    format!(
        "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\
         int main(int argc, char** argv) {{\n{}\n    return 0;\n}}\n",
        code.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
    )
}

fn exec_c(code: &str) -> Result<String, String> {
    // Try gcc then clang
    let compiler = if is_executable_available_with("gcc", &["--version"]) { "gcc" }
        else if is_executable_available_with("clang", &["--version"]) { "clang" }
        else { return Err("C compiler not found. Install gcc: https://gcc.gnu.org or clang: https://clang.llvm.org".to_string()); };

    let wrapped = wrap_c_if_needed(code);
    let tmp_dir = std::env::temp_dir();
    let c_file  = tmp_dir.join("killer_nova_galaxy_c.c");
    let out_file = tmp_dir.join(if cfg!(target_os = "windows") {
        "killer_nova_galaxy_c.exe"
    } else {
        "killer_nova_galaxy_c_out"
    });

    let mut f = fs::File::create(&c_file).map_err(|e| format!("Cannot create C temp file: {}", e))?;
    f.write_all(wrapped.as_bytes()).map_err(|e| format!("Cannot write C temp file: {}", e))?;

    let compile = Command::new(compiler)
        .arg(&c_file)
        .arg("-o").arg(&out_file)
        .arg("-Wall")
        .output()
        .map_err(|e| format!("{} failed to start: {}", compiler, e))?;

    let _ = fs::remove_file(&c_file);
    if !compile.status.success() {
        return Err(format!("C compile error:\n{}", String::from_utf8_lossy(&compile.stderr)));
    }

    let output = Command::new(&out_file)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("C binary failed to start: {}", e))?;
    let _ = fs::remove_file(&out_file);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
    } else {
        Err(format!("C runtime error:\n{}", String::from_utf8_lossy(&output.stderr)))
    }
}

// --- C++ Special Handling ----------------------------------------------------

fn wrap_cpp_if_needed(code: &str) -> String {
    let t = code.trim();
    if t.contains("int main(") {
        return code.to_string();
    }
    format!(
        "#include <iostream>\n#include <string>\n#include <vector>\n#include <algorithm>\n\
         using namespace std;\nint main() {{\n{}\n    return 0;\n}}\n",
        code.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
    )
}

fn exec_cpp(code: &str) -> Result<String, String> {
    let compiler = if is_executable_available_with("g++", &["--version"]) { "g++" }
        else if is_executable_available_with("clang++", &["--version"]) { "clang++" }
        else { return Err("C++ compiler not found. Install g++: https://gcc.gnu.org or clang++: https://clang.llvm.org".to_string()); };

    let wrapped = wrap_cpp_if_needed(code);
    let tmp_dir  = std::env::temp_dir();
    let cpp_file = tmp_dir.join("killer_nova_galaxy_cpp.cpp");
    let out_file = tmp_dir.join(if cfg!(target_os = "windows") {
        "killer_nova_galaxy_cpp.exe"
    } else {
        "killer_nova_galaxy_cpp_out"
    });

    let mut f = fs::File::create(&cpp_file).map_err(|e| format!("Cannot create C++ temp file: {}", e))?;
    f.write_all(wrapped.as_bytes()).map_err(|e| format!("Cannot write C++ temp file: {}", e))?;

    let compile = Command::new(compiler)
        .arg(&cpp_file)
        .arg("-o").arg(&out_file)
        .arg("-std=c++17").arg("-Wall")
        .output()
        .map_err(|e| format!("{} failed to start: {}", compiler, e))?;

    let _ = fs::remove_file(&cpp_file);
    if !compile.status.success() {
        return Err(format!("C++ compile error:\n{}", String::from_utf8_lossy(&compile.stderr)));
    }

    let output = Command::new(&out_file)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("C++ binary failed to start: {}", e))?;
    let _ = fs::remove_file(&out_file);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
    } else {
        Err(format!("C++ runtime error:\n{}", String::from_utf8_lossy(&output.stderr)))
    }
}

fn exec_rust(code: &str) -> Result<String, String> {
    if !is_executable_available_with("rustc", &["--version"]) {
        return Err("Rust compiler not found. Install from: https://rustup.rs".to_string());
    }

    let wrapped = wrap_rust_if_needed(code);
    let tmp_dir = std::env::temp_dir();
    let rs_file = tmp_dir.join("killer_nova_galaxy_rust.rs");
    let out_file = tmp_dir.join("killer_nova_galaxy_rust_out");

    let mut f = fs::File::create(&rs_file)
        .map_err(|e| format!("Cannot create Rust temp file: {}", e))?;
    f.write_all(wrapped.as_bytes())
        .map_err(|e| format!("Cannot write Rust temp file: {}", e))?;

    // Compile
    let compile = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&out_file)
        .output()
        .map_err(|e| format!("rustc failed to start: {}", e))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let _ = fs::remove_file(&rs_file);
        return Err(format!("Rust compile error:\n{}", stderr));
    }

    // Run
    let output = Command::new(&out_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Rust binary failed to start: {}", e))?;

    let _ = fs::remove_file(&rs_file);
    let _ = fs::remove_file(&out_file);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Rust runtime error:\n{}", stderr))
    }
}

// --- Core Execution Engine ---------------------------------------------------

/// Nova Galaxy Engine v1 - Execute a code block in any language.
///
/// # Usage in Killer:
/// ```killer
/// result = @python { print(10 * 5) }
/// output = @node   { console.log("Hello from JS!") }
/// info   = @bash   { df -h }
/// ```
///
/// # Returns
/// - `Ok(stdout)` - captured output from the language runtime
/// - `Err(message)` - runtime not found or execution failed (with helpful hint)
pub fn polyglot_exec(lang: &str, code: &str) -> Result<String, String> {
    let lang_lower = lang.to_lowercase();

    // -- Special cases: languages requiring compile+run ---------------------
    if lang_lower == "java" {
        return exec_java(code);
    }
    if lang_lower == "rust" {
        return exec_rust(code);
    }
    if lang_lower == "c" {
        return exec_c(code);
    }
    if lang_lower == "cpp" || lang_lower == "c++" {
        return exec_cpp(code);
    }

    // -- Standard script languages: write temp file -> run ------------------
    let runtime = find_runtime(&lang_lower).ok_or_else(|| {
        format!(
            "Unknown language: '{}'.\n\
             Supported: python, node/js, ruby, go, bash/sh, ps/powershell, java, ts, lua, perl, php, r, rust\n\
             Check available runtimes with: polyglot_list()",
            lang
        )
    })?;

    let exe = resolve_executable(runtime).ok_or_else(|| {
        format!(
            "{} runtime not found on this machine.\n\
             Install it or use a different language.\n\
             Check what IS available with: polyglot_list()",
            runtime.name
        )
    })?;

    let temp_path = write_temp_file(code, runtime.ext)?;

    // Build command: exe [pre_args] temp_path [post_args]
    let mut cmd = Command::new(exe);
    for &arg in runtime.pre_args {
        cmd.arg(arg);
    }
    cmd.arg(&temp_path);
    for &arg in runtime.post_args {
        cmd.arg(arg);
    }

    // Capture stdout and stderr
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to launch {}: {}", exe, e))?;

    // Always clean up temp file
    let _ = fs::remove_file(&temp_path);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut msg = format!("{} execution failed", runtime.name);
        if !stderr.is_empty() {
            msg.push_str(&format!(":\n{}", stderr.trim()));
        }
        if !stdout.is_empty() {
            msg.push_str(&format!("\nOutput before error:\n{}", stdout.trim()));
        }
        Err(msg)
    }
}

// --- AI Assassin Assist Layer Wrapper ----------------------------------------

/// AI-assisted version of `polyglot_exec`.
/// Routes through the Assassin Assist Layer:
///   1. Executes normally (same as polyglot_exec)
///   2. Measures wall-clock time
///   3. Hands result to AssassinAssistLayer for logging, error analysis, AI debug
///   4. Returns stdout on success, or formatted error (with AI suggestion) on fail
pub fn polyglot_exec_assisted(lang: &str, code: &str) -> Result<String, String> {
    let start = std::time::Instant::now();
    let result = polyglot_exec(lang, code);
    let duration_ms = start.elapsed().as_millis();

    // Feed result through the AI Assassin Assist Layer
    let (stdout, ai_suggestion) = crate::assassin_assist::layer()
        .lock()
        .unwrap()
        .process_execution(lang, code, &result, duration_ms);

    match result {
        Ok(_) => Ok(stdout),
        Err(original_err) => {
            // Enrich the error with AI suggestion if available
            if let Some(suggestion) = ai_suggestion {
                Err(format!(
                    "{}\n\nAI Assassin Assist (@{}):\n{}",
                    original_err, lang, suggestion
                ))
            } else {
                Err(original_err)
            }
        }
    }
}

/// List all languages detectable on this machine with their executables.
/// Returns a formatted string for `polyglot_list()` builtin.
pub fn polyglot_list_str() -> String {
    let found = detect_all_runtimes();
    if found.is_empty() {
        return "Nova Galaxy Engine: No external runtimes detected.\nKiller runs natively - no install needed for .killer code.".to_string();
    }

    let mut lines = vec![];
    for (lang, exe) in &found {
        lines.push(format!("  @{}  {}", lang, exe));
    }
    format!("Detected runtimes:\n{}", lines.join("\n"))
}

// Killer Value Bridge
// These are the builtin.rs dispatcher targets.
// They follow the same signature pattern as all other nova_* functions.

use crate::value::Value;
use crate::error::VmError;

/// Builtin: polyglot_exec(lang, code)
/// Executes a code block in the given language and returns its stdout as a string.
pub fn builtin_polyglot_exec(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    if args.len() < 2 {
        return Err(VmError::runtime_error(
            "polyglot_exec(lang, code) requires 2 arguments".to_string(),
        ));
    }
    let lang = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("polyglot_exec: lang must be a string".to_string())),
    };
    let code = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("polyglot_exec: code must be a string".to_string())),
    };

    // Route through AI Assassin Assist Layer (logs + auto-debug on errors)
    match polyglot_exec_assisted(&lang, &code) {
        Ok(output) => Ok(Value::Str(output)),
        Err(e) => Err(VmError::runtime_error(format!("Nova Galaxy Engine: {}", e))),
    }
}

/// Builtin: polyglot_list()
/// Returns a string listing all runtimes detected on this machine.
pub fn builtin_polyglot_list(_args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    Ok(Value::Str(polyglot_list_str()))
}

/// Builtin: polyglot_check(lang)
/// Returns true if the given language runtime is installed.
pub fn builtin_polyglot_check(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    if args.is_empty() {
        return Err(VmError::runtime_error(
            "polyglot_check(lang) requires 1 argument".to_string(),
        ));
    }
    let lang = match &args[0] {
        Value::Str(s) => s.as_str().to_lowercase(),
        _ => return Err(VmError::runtime_error("polyglot_check: lang must be a string".to_string())),
    };
    let available = if lang == "java" {
        is_executable_available_with("javac", &["--version"]) && is_executable_available_with("java", &["--version"])
    } else if lang == "rust" {
        is_executable_available_with("rustc", &["--version"])
    } else if lang == "c" {
        is_executable_available_with("gcc", &["--version"]) || is_executable_available_with("clang", &["--version"])
    } else if lang == "cpp" || lang == "c++" {
        is_executable_available_with("g++", &["--version"]) || is_executable_available_with("clang++", &["--version"])
    } else {
        find_runtime(&lang)
            .and_then(|rt| resolve_executable(rt))
            .is_some()
    };
    Ok(Value::Bool(available))
}

// A: Streaming Builtin
//
// polyglot_stream(lang, code) - executes code, prints each stdout line
// as it arrives (real-time), and returns the full output as a String.
// Uses BufReader on child stdout pipe.

/// Builtin: polyglot_stream(lang, code)
/// Like polyglot_exec but prints each line of output immediately as the
/// subprocess produces it (streaming), then returns the full output.
pub fn builtin_polyglot_stream(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    use std::io::{BufRead, BufReader};

    if args.len() < 2 {
        return Err(VmError::runtime_error(
            "polyglot_stream(lang, code) requires 2 arguments".to_string()
        ));
    }
    let lang = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("polyglot_stream: lang must be a string".to_string())),
    };
    let code = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("polyglot_stream: code must be a string".to_string())),
    };

    let lang_lower = lang.to_lowercase();

    // Compile-then-run languages fall back to regular polyglot_exec
    // (streaming only applies to script interpreters that produce output incrementally)
    if matches!(lang_lower.as_str(), "java" | "rust" | "c" | "cpp" | "c++") {
        return match polyglot_exec_assisted(&lang, &code) {
            Ok(out)  => Ok(Value::Str(out)),
            Err(e)   => Err(VmError::runtime_error(format!("Nova Galaxy Engine: {}", e))),
        };
    }

    let runtime = find_runtime(&lang_lower).ok_or_else(|| {
        VmError::runtime_error(format!("Unknown language: '{}'. Use polyglot_list() to see available runtimes.", lang))
    })?;
    let exe = resolve_executable(runtime).ok_or_else(|| {
        VmError::runtime_error(format!("{} runtime not found. Use polyglot_list() to see what's installed.", runtime.name))
    })?;

    let temp_path = write_temp_file(&code, runtime.ext)
        .map_err(|e| VmError::runtime_error(e))?;

    let mut cmd = Command::new(exe);
    for &arg in runtime.pre_args { cmd.arg(arg); }
    cmd.arg(&temp_path);
    for &arg in runtime.post_args { cmd.arg(arg); }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| VmError::runtime_error(format!("Failed to launch {}: {}", exe, e)))?;

    let stdout_pipe = child.stdout.take()
        .ok_or_else(|| VmError::runtime_error("Could not capture stdout".to_string()))?;

    let mut all_lines: Vec<String> = Vec::new();
    let reader = BufReader::new(stdout_pipe);
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                // Print each line immediately so the user sees streaming output
                println!("{}", line);
                all_lines.push(line);
            }
            Err(_) => break,
        }
    }

    let status = child.wait()
        .map_err(|e| VmError::runtime_error(format!("Wait error: {}", e)))?;
    let _ = fs::remove_file(&temp_path);

    if status.success() {
        Ok(Value::Str(all_lines.join("\n")))
    } else {
        let stderr_out = match child.stderr.take() {
            Some(mut s) => {
                let mut buf = String::new();
                use std::io::Read;
                let _ = s.read_to_string(&mut buf);
                buf
            }
            None => String::new(),
        };
        Err(VmError::runtime_error(format!("{} execution failed: {}", runtime.name, stderr_out.trim())))
    }
}
