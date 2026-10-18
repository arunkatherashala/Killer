// killer_rcore/src/jit/compiler.rs
// Rust compiler integration for JIT code generation
// Week 3 part 3

use std::process::Command;
use std::fs;
use std::path::PathBuf;
use std::io::Write;

/// Compiles generated Rust code to native binaries
pub struct RustCompiler {
    /// Path to rustc executable
    rustc_path: String,
    
    /// Enable optimizations (-O)
    optimize: bool,
    
    /// Compilation target (e.g., "x86_64-pc-windows-gnu")
    target: Option<String>,
}

/// Result of compilation attempt
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub binary_path: Option<PathBuf>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

impl RustCompiler {
    /// Create new compiler with default settings
    pub fn new() -> Self {
        RustCompiler {
            rustc_path: "rustc".to_string(),
            optimize: true,
            target: None,
        }
    }
    
    /// Create with custom rustc path
    pub fn with_rustc_path(path: String) -> Self {
        RustCompiler {
            rustc_path: path,
            optimize: true,
            target: None,
        }
    }
    
    /// Set optimization level
    pub fn set_optimize(&mut self, optimize: bool) {
        self.optimize = optimize;
    }
    
    /// Set compilation target
    pub fn set_target(&mut self, target: String) {
        self.target = Some(target);
    }
    
    /// Compile Rust source code to native binary
    pub fn compile(&self, rust_source: &str, output_name: &str) -> CompileResult {
        // Step 1: Write source to temporary file
        let temp_rs = match write_temp_file(rust_source, "killer_jit_", ".rs") {
            Ok(path) => path,
            Err(e) => {
                return CompileResult {
                    success: false,
                    binary_path: None,
                    error: Some(format!("Failed to write source file: {}", e)),
                    warnings: vec![],
                };
            }
        };
        
        // Step 2: Build compilation command
        let mut cmd = Command::new(&self.rustc_path);
        
        // Input file
        cmd.arg(&temp_rs);
        
        // Output name
        let out_path = PathBuf::from(format!("{}.so", output_name));
        cmd.arg("-o").arg(&out_path);
        
        // Make it a shared library
        cmd.arg("--crate-type").arg("cdylib");
        
        // Optimization
        if self.optimize {
            cmd.arg("-O"); // Release mode optimizations
        }
        
        // Target (if specified)
        if let Some(ref target) = self.target {
            cmd.arg("--target").arg(target);
        }
        
        // Step 3: Run compilation
        let output = cmd.output();
        
        let result = match output {
            Ok(out) => {
                if out.status.success() {
                    // Extract warnings from stderr
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let warnings = stderr
                        .lines()
                        .filter(|l| l.contains("warning:"))
                        .map(|l| l.to_string())
                        .collect();
                    
                    CompileResult {
                        success: true,
                        binary_path: if out_path.exists() { Some(out_path) } else { None },
                        error: None,
                        warnings,
                    }
                } else {
                    // Compilation failed
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let error = extract_rustc_error(&stderr);
                    
                    CompileResult {
                        success: false,
                        binary_path: None,
                        error: Some(error),
                        warnings: vec![],
                    }
                }
            }
            Err(e) => {
                CompileResult {
                    success: false,
                    binary_path: None,
                    error: Some(format!("Failed to invoke rustc: {}", e)),
                    warnings: vec![],
                }
            }
        };
        
        // Step 4: Cleanup temp file
        let _ = fs::remove_file(&temp_rs);
        
        result
    }
    
    /// Compile to library (.so/.dll)
    pub fn compile_to_lib(
        &self,
        rust_source: &str,
        lib_name: &str,
    ) -> CompileResult {
        self.compile(rust_source, &format!("{}_lib", lib_name))
    }
    
    /// Verify rustc is available
    pub fn verify_available() -> bool {
        Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
    
    /// Get rustc version
    pub fn version() -> Option<String> {
        let output = Command::new("rustc")
            .arg("--version")
            .output()
            .ok()?;
        
        String::from_utf8(output.stdout).ok()
    }
}

impl Default for RustCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Write content to a temporary file
fn write_temp_file(content: &str, prefix: &str, suffix: &str) -> std::io::Result<String> {
    // The name must be unique per call: the clock alone is not (two threads can read the same
    // nanosecond value on a coarse clock such as Windows', and would then overwrite each other's
    // source), so add the process id and a process-wide counter, and never overwrite a file.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp_dir = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for _ in 0..16 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let filename = format!("{}_{}_{}_{}{}", prefix, std::process::id(), nanos, n, suffix);
        let path = temp_dir.join(&filename);
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(content.as_bytes())?;
                file.flush()?;
                return Ok(path.to_string_lossy().to_string());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "could not pick an unused temp file name"))
}

/// Extract relevant error message from rustc stderr
fn extract_rustc_error(stderr: &str) -> String {
    // Look for the first "error:" line
    for line in stderr.lines() {
        if line.contains("error:") {
            return line.to_string();
        }
    }
    
    // If no specific error found, return first few lines
    let first_lines: Vec<_> = stderr.lines().take(3).collect();
    first_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compiler_creation() {
        let compiler = RustCompiler::new();
        assert_eq!(compiler.rustc_path, "rustc");
        assert!(compiler.optimize);
    }
    
    #[test]
    fn test_compiler_with_custom_path() {
        let compiler = RustCompiler::with_rustc_path("/usr/bin/rustc".to_string());
        assert_eq!(compiler.rustc_path, "/usr/bin/rustc");
    }
    
    #[test]
    fn test_set_optimize() {
        let mut compiler = RustCompiler::new();
        compiler.set_optimize(false);
        assert!(!compiler.optimize);
    }
    
    #[test]
    fn temp_files_are_unique_and_keep_their_own_content_under_contention() {
        let handles: Vec<_> = (0..8)
            .map(|t| {
                std::thread::spawn(move || {
                    (0..60)
                        .map(|i| {
                            let text = format!("// thread {} item {}\n", t, i);
                            let path = write_temp_file(&text, "killer_jit_test_", ".rs").expect("temp file");
                            (path, text)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut seen = std::collections::HashSet::new();
        for h in handles {
            for (path, text) in h.join().unwrap() {
                assert!(seen.insert(path.clone()), "two calls returned the same file: {}", path);
                assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "file {} was overwritten", path);
                let _ = std::fs::remove_file(&path);
            }
        }
        assert_eq!(seen.len(), 8 * 60);
    }

    #[test]
    fn test_set_target() {
        let mut compiler = RustCompiler::new();
        compiler.set_target("x86_64-unknown-linux-gnu".to_string());
        assert_eq!(compiler.target, Some("x86_64-unknown-linux-gnu".to_string()));
    }
    
    #[test]
    fn test_compile_simple_valid_rust() {
        // Only run if rustc is available
        if !RustCompiler::verify_available() {
            eprintln!("Skipping test: rustc not available");
            return;
        }
        
        let compiler = RustCompiler::new();
        let simple_rs = r#"
#[no_mangle]
pub extern "C" fn add_two(x: i64) -> i64 {
    x + 2
}
"#;
        
        let result = compiler.compile(simple_rs, "test_simple");
        
        // Should succeed
        assert!(result.success, "Compilation failed: {:?}", result.error);
        assert!(result.binary_path.is_some());
    }
    
    #[test]
    fn test_compile_invalid_rust() {
        if !RustCompiler::verify_available() {
            eprintln!("Skipping test: rustc not available");
            return;
        }
        
        let compiler = RustCompiler::new();
        let invalid_rs = r#"
pub extern "C" fn bad_code() {
    this is not valid rust code!
}
"#;
        
        let result = compiler.compile(invalid_rs, "test_invalid");
        
        // Should fail
        assert!(!result.success);
        assert!(result.error.is_some());
    }
    
    #[test]
    fn test_rustc_available() {
        // This might fail in offline environments, which is OK
        let available = RustCompiler::verify_available();
        // We don't assert, just check it doesn't panic
        let _ = available;
    }
    
    #[test]
    fn test_error_extraction() {
        let stderr = r#"error[E0425]: cannot find value `x` in this scope
  --> src/lib.rs:2:5
   |
2 |     x + 1
   | ^ not found in this scope"#;
        
        let error = extract_rustc_error(stderr);
        assert!(error.contains("error"));
        assert!(error.contains("E0425"));
    }
    
    #[test]
    fn test_compile_result_success() {
        let result = CompileResult {
            success: true,
            binary_path: Some(PathBuf::from("/tmp/test.so")),
            error: None,
            warnings: vec![],
        };
        
        assert!(result.success);
        assert!(result.error.is_none());
    }
    
    #[test]
    fn test_compile_result_failure() {
        let result = CompileResult {
            success: false,
            binary_path: None,
            error: Some("Compilation failed".to_string()),
            warnings: vec![],
        };
        
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
