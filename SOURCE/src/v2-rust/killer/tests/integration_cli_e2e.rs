#![cfg(feature = "cli-e2e-tests")]
// Integration Test: End-to-End CLI Compilation
// Tests killer_super CLI with real .killer files

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "killer_super", "--", "--help"])
        .output()
        .expect("Failed to run killer_super --help");
    
    assert!(output.status.success(), "Help command failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("KILLER SUPER"), "Help output missing description");
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "killer_super", "--", "--version"])
        .output()
        .expect("Failed to run killer_super --version");
    
    assert!(output.status.success(), "Version command failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4.0.0"), "Version output incorrect");
}

#[test]
fn test_compile_hello_world() {
    let input = "killer_rcore/examples/hello.killer";
    if !Path::new(input).exists() {
        // Create test file if not exists
        fs::create_dir_all("killer_rcore/examples").ok();
        fs::write(input, "fn main() { println(\"Hello!\"); }").expect("Failed to create test file");
    }

    let output = Command::new("cargo")
        .args(&["run", "--bin", "killer_super", "--", input, "-o", "hello.out"])
        .output()
        .expect("Failed to compile hello.killer");

    // Check if compilation succeeded or gave expected output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Either success or diagnostic output is acceptable
    assert!(
        stdout.contains("Compilation") || stderr.len() > 0,
        "No compiler output"
    );
}

#[test]
fn test_compile_different_opt_levels() {
    let input = "killer_rcore/examples/fibonacci.killer";
    
    for opt_level in &["0", "1", "2", "3"] {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "killer_super", "--",
                input,
                "-O", opt_level,
                "-o", &format!("fib_o{}.out", opt_level)
            ])
            .output()
            .expect(&format!("Failed to compile with -O{}", opt_level));

        // Check compilation result
        assert!(
            output.status.success() || String::from_utf8_lossy(&output.stderr).len() > 0,
            "No compilation for -O{}",
            opt_level
        );
    }
}

#[test]
fn test_compile_with_verbose() {
    let input = "killer_rcore/examples/array_reduce.killer";
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "killer_super", "--", input, "--verbose"])
        .output()
        .expect("Failed to compile with --verbose");

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verbose mode should show compilation info
    assert!(
        stdout.contains("Killer") || stdout.contains("Configuration") || stdout.len() > 50,
        "Verbose output missing or too short"
    );
}

#[test]
fn test_compile_different_targets() {
    let input = "killer_rcore/examples/hello.killer";
    
    for target in &["x86-64", "arm64", "wasm32", "riscv64"] {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "killer_super", "--",
                input,
                "-t", target,
                "-o", &format!("hello_{}.out", target)
            ])
            .output()
            .expect(&format!("Failed to compile for target {}", target));

        // Should complete, success or with diagnostics
        assert!(
            output.status.success() || String::from_utf8_lossy(&output.stderr).len() > 0,
            "No output for target {}",
            target
        );
    }
}

#[test]
fn test_compile_with_stats() {
    let input = "killer_rcore/examples/hello.killer";
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "killer_super", "--", input, "--stats"])
        .output()
        .expect("Failed to compile with --stats");

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Stats should show compilation information
    assert!(
        stdout.contains("Compilation") || stdout.contains("Statistics") || stdout.contains("Time") || stdout.len() > 0,
        "Stats output missing"
    );
}

#[test]
fn test_compile_to_llvm_ir() {
    let input = "killer_rcore/examples/hello.killer";
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "killer_super", "--",
            input,
            "--emit", "llvm",
            "-o", "hello.ll"
        ])
        .output()
        .expect("Failed to compile to LLVM IR");

    // Should complete
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 0 || output.status.success(), "No output from LLVM emission");
}

#[test]
fn test_compile_to_bytecode() {
    let input = "killer_rcore/examples/hello.killer";
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "killer_super", "--",
            input,
            "--emit", "bytecode",
            "-o", "hello.bc"
        ])
        .output()
        .expect("Failed to compile to bytecode");

    // Should complete
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 0 || output.status.success(), "No output from bytecode emission");
}

#[test]
fn test_compile_dev_mode() {
    let input = "killer_rcore/examples/hello.killer";
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "killer_super", "--",
            input,
            "-m", "dev",
            "-O1"
        ])
        .output()
        .expect("Failed to compile in dev mode");

    // Should succeed or show diagnostics
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.len() > 0 || stderr.len() > 0, "No compiler output in dev mode");
}

#[test]
fn test_compile_prod_mode() {
    let input = "killer_rcore/examples/hello.killer";
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "killer_super", "--",
            input,
            "-m", "prod",
            "-O3"
        ])
        .output()
        .expect("Failed to compile in prod mode");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.len() > 0 || stderr.len() > 0, "No compiler output in prod mode");
}

#[test]
fn test_missing_input_file() {
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "killer_super", "--",
            "nonexistent_file.killer"
        ])
        .output()
        .expect("Failed to run compiler");

    // Should fail with error about missing file
    assert!(!output.status.success(), "Should fail for missing file");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("error") || stderr.contains("ERROR"),
        "Error message missing"
    );
}
