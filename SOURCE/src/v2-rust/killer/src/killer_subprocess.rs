//! **killer_subprocess** — process spawning, piping, output capture.
//!
//! Fills the "no subprocess/exec API" gap. Inspired by Python's `subprocess` and Go's `os/exec`.

use std::collections::HashMap;
use std::process::{Command as StdCommand, Stdio};
use std::time::Instant;

// ══════════════════════════════════════════════════════════════════════════════
// Process result
// ══════════════════════════════════════════════════════════════════════════════

/// Result of running a subprocess.
#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: f64,
    pub success: bool,
}

impl ProcessResult {
    /// Check if stdout contains a substring.
    pub fn stdout_contains(&self, s: &str) -> bool { self.stdout.contains(s) }

    /// Get stdout lines.
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Trim stdout.
    pub fn stdout_trimmed(&self) -> &str { self.stdout.trim() }
}

// ══════════════════════════════════════════════════════════════════════════════
// ProcessBuilder — fluent API for constructing subprocess calls
// ══════════════════════════════════════════════════════════════════════════════

/// Builder for spawning subprocesses.
#[derive(Debug, Clone)]
pub struct ProcessBuilder {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    cwd: Option<String>,
    stdin_data: Option<String>,
    timeout_ms: Option<u64>,
    inherit_env: bool,
}

impl ProcessBuilder {
    pub fn new(program: &str) -> Self {
        Self {
            program: program.to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            stdin_data: None,
            timeout_ms: None,
            inherit_env: true,
        }
    }

    /// Add a single argument.
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add multiple arguments.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, dir: &str) -> Self {
        self.cwd = Some(dir.to_string());
        self
    }

    /// Provide stdin data.
    pub fn stdin(mut self, data: &str) -> Self {
        self.stdin_data = Some(data.to_string());
        self
    }

    /// Set a timeout in milliseconds.
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Don't inherit parent environment.
    pub fn clear_env(mut self) -> Self {
        self.inherit_env = false;
        self
    }

    /// Execute the subprocess and capture output.
    pub fn run(&self) -> Result<ProcessResult, String> {
        let start = Instant::now();
        let mut cmd = StdCommand::new(&self.program);
        cmd.args(&self.args);

        if !self.inherit_env {
            cmd.env_clear();
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        if let Some(ref dir) = self.cwd {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if self.stdin_data.is_some() {
            cmd.stdin(Stdio::piped());
        }

        let mut child = cmd.spawn().map_err(|e| format!("failed to spawn '{}': {}", self.program, e))?;

        // Write stdin if provided
        if let Some(ref data) = self.stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(data.as_bytes());
            }
        }

        let output = child.wait_with_output().map_err(|e| format!("process error: {}", e))?;
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(ProcessResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
            success: output.status.success(),
        })
    }

    /// Execute and return stdout only (trimmed). Errors on non-zero exit.
    pub fn output(&self) -> Result<String, String> {
        let result = self.run()?;
        if result.success {
            Ok(result.stdout.trim().to_string())
        } else {
            Err(format!("process exited with code {}: {}", result.exit_code, result.stderr.trim()))
        }
    }

    /// Execute and return exit code only.
    pub fn status(&self) -> Result<i32, String> {
        let result = self.run()?;
        Ok(result.exit_code)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Convenience functions
// ══════════════════════════════════════════════════════════════════════════════

/// Run a command and capture output (shorthand).
pub fn run(program: &str, args: &[&str]) -> Result<ProcessResult, String> {
    ProcessBuilder::new(program).args(args).run()
}

/// Run a shell command (cmd /C on Windows, sh -c on Unix).
pub fn shell(command: &str) -> Result<ProcessResult, String> {
    if cfg!(windows) {
        ProcessBuilder::new("cmd").args(&["/C", command]).run()
    } else {
        ProcessBuilder::new("sh").args(&["-c", command]).run()
    }
}

/// Run a command and return stdout (trimmed).
pub fn capture(program: &str, args: &[&str]) -> Result<String, String> {
    ProcessBuilder::new(program).args(args).output()
}

/// Check if a program exists in PATH.
pub fn which(program: &str) -> Option<String> {
    let result = if cfg!(windows) {
        ProcessBuilder::new("where").arg(program).output()
    } else {
        ProcessBuilder::new("which").arg(program).output()
    };
    result.ok().map(|s| s.lines().next().unwrap_or(&s).to_string())
}

// ══════════════════════════════════════════════════════════════════════════════
// Pipeline — chain processes (A | B | C)
// ══════════════════════════════════════════════════════════════════════════════

/// A pipeline of commands piped together.
#[derive(Debug)]
pub struct Pipeline {
    stages: Vec<ProcessBuilder>,
}

impl Pipeline {
    pub fn new() -> Self { Self { stages: Vec::new() } }

    /// Add a stage to the pipeline.
    pub fn pipe(mut self, builder: ProcessBuilder) -> Self {
        self.stages.push(builder);
        self
    }

    /// Execute the pipeline. Each stage's stdout feeds the next stage's stdin.
    pub fn run(self) -> Result<ProcessResult, String> {
        if self.stages.is_empty() {
            return Err("empty pipeline".to_string());
        }

        let mut input: Option<String> = None;
        let mut last_result = None;

        for mut stage in self.stages {
            if let Some(data) = input {
                stage.stdin_data = Some(data);
            }
            let result = stage.run()?;
            input = Some(result.stdout.clone());
            last_result = Some(result);
        }

        last_result.ok_or_else(|| "pipeline produced no result".to_string())
    }
}

impl Default for Pipeline {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_builder_echo() {
        // Windows: use cmd /C echo
        let result = shell("echo hello").unwrap();
        assert!(result.success);
        assert!(result.stdout.trim().contains("hello"));
    }

    #[test]
    fn process_builder_with_env() {
        let result = if cfg!(windows) {
            ProcessBuilder::new("cmd")
                .args(&["/C", "echo %KILLER_TEST%"])
                .env("KILLER_TEST", "world")
                .run()
        } else {
            ProcessBuilder::new("sh")
                .args(&["-c", "echo $KILLER_TEST"])
                .env("KILLER_TEST", "world")
                .run()
        };
        assert!(result.is_ok());
    }

    #[test]
    fn capture_output() {
        let out = capture("cmd", &["/C", "echo test_output"]);
        assert!(out.is_ok());
        assert!(out.unwrap().contains("test_output"));
    }

    #[test]
    fn which_finds_cmd() {
        // cmd.exe exists on Windows
        if cfg!(windows) {
            let result = which("cmd");
            assert!(result.is_some());
        }
    }

    #[test]
    fn process_result_helpers() {
        let r = ProcessResult {
            exit_code: 0,
            stdout: "line1\nline2\nline3\n".into(),
            stderr: String::new(),
            duration_ms: 10.0,
            success: true,
        };
        assert!(r.stdout_contains("line2"));
        assert_eq!(r.stdout_lines().len(), 3);
    }

    #[test]
    fn failed_process() {
        // Run a command that doesn't exist
        let result = run("nonexistent_binary_12345", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_construct() {
        let pipe = Pipeline::new()
            .pipe(ProcessBuilder::new("echo").arg("hello"))
            .pipe(ProcessBuilder::new("cat"));
        assert_eq!(pipe.stages.len(), 2);
    }
}
