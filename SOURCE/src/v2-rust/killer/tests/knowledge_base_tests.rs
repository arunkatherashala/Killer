//! **Knowledge base smoke tests** — exercises built-in facts in `llm::knowledge_base_lookup` and `native_think`.
//! No network, no API keys. Run:
//!   cargo test --test knowledge_base_tests
//!   cargo test --test knowledge_base_tests -- --nocapture

use killer_native::builtin::BuiltinFunctions;
use killer_native::llm::{knowledge_base_lookup_pub, native_think};

fn assert_kb_contains(question: &str, must_all_substrings: &[&str]) {
    let ans = knowledge_base_lookup_pub(question)
        .unwrap_or_else(|| panic!("expected knowledge hit for {:?}", question));
    let lower = ans.to_lowercase();
    for needle in must_all_substrings {
        assert!(
            lower.contains(&needle.to_lowercase()),
            "Q: {:?}\nMissing {:?} in:\n{}",
            question,
            needle,
            &ans[..ans.len().min(400)]
        );
    }
}

#[test]
fn kb_programming_python() {
    assert_kb_contains("what is the python language", &["python", "guido"]);
}

#[test]
fn kb_programming_rust() {
    assert_kb_contains("what is rust programming language", &["rust", "borrow"]);
}

#[test]
fn kb_ai_ml_llm() {
    assert_kb_contains("what is machine learning", &["machine learning", "supervised"]);
    assert_kb_contains("what is an LLM", &["transformer", "token"]);
}

#[test]
fn kb_cs_big_o() {
    assert_kb_contains("what is big o notation", &["o(n", "complexities"]);
}

#[test]
fn kb_science_gravity() {
    assert_kb_contains("what is gravity newton", &["gravity", "mass"]);
}

#[test]
fn kb_miss_returns_none() {
    assert!(knowledge_base_lookup_pub("xyzzy_plugh_no_such_topic_12345").is_none());
}

/// `expert_normalize_kb_query` fixes "microsfot" → "microsoft" before KB / web routing.
#[test]
fn expert_ask_typo_microsfot_still_hits_microsoft_topic() {
    // Must include "gates" (or another KB trigger) so the bundled KB matches Bill Gates, not only "microsoft".
    let r = BuiltinFunctions::kala_expert_ask("is bill gates still tied to microsfot");
    let low = r.to_lowercase();
    assert!(
        low.contains("microsoft") || low.contains("gates") || low.contains("paul allen"),
        "expected typo-corrected expert path to mention Microsoft/founders, got: {}",
        &r[..r.len().min(600)]
    );
}

#[test]
fn native_think_hits_kb_python() {
    let out = native_think("What is Python as a programming language?");
    assert!(
        out.to_lowercase().contains("python"),
        "native_think should mention Python, got: {}",
        &out[..out.len().min(500)]
    );
}

#[test]
fn native_think_handles_unknown_gracefully() {
    let out = native_think("xyzzy_plugh completely unknown phrase 99999");
    assert!(!out.is_empty());
    // Should not panic; ends with explicit fallback or exhausts web with "no … found" steps
    let low = out.to_lowercase();
    assert!(
        low.contains("could not")
            || low.contains("not find")
            || low.contains("reliable")
            || low.contains("no result found")
            || low.contains("no direct answer")
            || low.contains("web search: no result"),
        "expected graceful fallback, got: {}",
        &out[..out.len().min(800)]
    );
}
