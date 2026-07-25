//! Offline-safe tests for `khlm_generate_code` templates and empty input.
//! Run: `cargo test --test code_generation_tests`
//!
//! Note: `cargo test` builds the library with `cfg(test)`, so the expert/web
//! fallback for unmatched templates is **disabled** — unknown prompts return the
//! local “connect an LLM” stub (deterministic).

use killer_native::khlm_polyglot::khlm_generate_code;

#[test]
fn codegen_fizzbuzz_includes_pattern() {
    let s = khlm_generate_code("fizzbuzz in python");
    let l = s.to_lowercase();
    assert!(
        l.contains("fizz") && (l.contains("```") || l.contains("python")),
        "expected fizzbuzz template, got: {}",
        &s[..s.len().min(400)]
    );
}

#[test]
fn codegen_binary_search_has_fence_and_logic() {
    let s = khlm_generate_code("iterative binary search in rust");
    let l = s.to_lowercase();
    assert!(s.contains("```"), "code fence: {}", &s[..s.len().min(200)]);
    assert!(
        l.contains("rust") && (l.contains("binary") || l.contains("search")),
        "got: {}",
        &s[..s.len().min(400)]
    );
}

#[test]
fn codegen_hello_world_python() {
    let s = khlm_generate_code("hello world in python");
    let l = s.to_lowercase();
    assert!(l.contains("hello") && l.contains("```"), "{}", &s[..s.len().min(300)]);
}

#[test]
fn codegen_empty_prompt_is_helpful() {
    let s = khlm_generate_code("   ");
    let l = s.to_lowercase();
    assert!(
        l.contains("describe") || l.contains("empty") || l.contains("build"),
        "got: {}",
        &s[..s.len().min(200)]
    );
}

#[test]
fn codegen_unknown_in_test_build_uses_stub() {
    let s = khlm_generate_code("__no_builtin_template_match_xyz999__ obscure language foobar sort");
    assert!(
        s.contains("LLM") || s.contains("code") || s.contains("Kala") || s.contains("killer"),
        "stub should guide user, got: {}",
        &s[..s.len().min(350)]
    );
}

#[test]
fn codegen_python_ml_agent_request_returns_agent_loop() {
    let s = khlm_generate_code("write a python ml agent program");
    let l = s.to_lowercase();
    assert!(
        l.contains("fake_llm_plan") || l.contains("run_agent"),
        "expected ML-agent offline template, got: {}",
        &s[..s.len().min(500)]
    );
    assert!(s.contains("```"), "code fence: {}", &s[..s.len().min(200)]);
}

#[test]
fn codegen_project_phrase_routes_templates_or_stub() {
    // "project" may use offline generic stub under tests; still must return guidance.
    let s = khlm_generate_code("create a hello world project in python");
    assert!(!s.is_empty() && s.len() > 20, "{}", &s[..s.len().min(200)]);
}
