//! Regression: Ask mode must not emit prose templates for code questions or shopping spam.
//! Run: `cargo test --test kala_ask_routing_tests`

use killer_native::builtin::BuiltinFunctions;

#[test]
fn ask_threejs_gesture_not_prose_casual() {
    let r = BuiltinFunctions::kala_dispatch(
        "ask",
        "how are write a threee.js face gester code",
        "essay",
        "killer",
    );
    assert!(
        !r.contains("The short version:") && !r.contains("cross-cutting quality"),
        "expected code/template answer, not prose_casual filler:\n{}",
        &r[..r.len().min(800)]
    );
    assert!(
        r.contains("```") || r.to_lowercase().contains("three") || r.to_lowercase().contains("webgl"),
        "expected Three/WebGL-related reply:\n{}",
        &r[..r.len().min(800)]
    );
}

#[test]
fn ask_write_code_short_prompt_avoids_prose_template() {
    let r = BuiltinFunctions::kala_dispatch("ask", "write code", "essay", "killer");
    assert!(
        !r.contains("The short version:"),
        "short code prompts must not hit creative prose:\n{}",
        &r[..r.len().min(600)]
    );
    assert!(
        !r.to_lowercase().contains("amazon.com") || r.contains("```"),
        "should not be e‑commerce spam as primary answer:\n{}",
        &r[..r.len().min(600)]
    );
}

#[test]
fn write_mode_javascript_snippet_not_prose_casual() {
    let r = BuiltinFunctions::kala_dispatch(
        "write",
        "javascript debounce function 200ms",
        "casual",
        "killer",
    );
    assert!(
        !r.contains("The short version:") && !r.contains("cross-cutting quality"),
        "write+casual must not use offline prose_casual for code topics:\n{}",
        &r[..r.len().min(900)]
    );
    assert!(
        r.contains("```"),
        "expected a fenced code block:\n{}",
        &r[..r.len().min(900)]
    );
}
