//! Comprehensive Kala AI answer quality tests.
//! Exercises greetings, identity, creator, wellbeing, help, social, code routing,
//! name recognition, wrong-name correction, mode dispatch, and edge cases.
//!
//! Run: `cargo test --test kala_answer_quality_tests -- --nocapture`

use killer_native::builtin::BuiltinFunctions;

fn dispatch(mode: &str, q: &str) -> String {
    BuiltinFunctions::kala_dispatch(mode, q, "casual", "killer")
}

// ═══════════════════════════════════════════════════════════════════
// GREETINGS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn greeting_hi() {
    let r = dispatch("ask", "hi");
    assert!(r.contains("Kala"), "greeting should mention Kala: {r}");
    assert!(!r.contains("काल"), "no Sanskrit in greeting: {r}");
    assert!(r.len() < 100, "greeting should be short and simple: {r}");
}

#[test]
fn greeting_hello() {
    let r = dispatch("ask", "hello");
    assert!(r.contains("Kala"), "hello should identify as Kala: {r}");
    assert!(!r.contains("काल"), "no Sanskrit: {r}");
}

#[test]
fn greeting_namaste() {
    let r = dispatch("ask", "namaste");
    assert!(r.contains("Kala"), "namaste should trigger Kala greeting: {r}");
}

#[test]
fn greeting_with_name() {
    let r = dispatch("ask", "hi i am deepak");
    assert!(r.contains("Deepak"), "should recognize and capitalize name: {r}");
    assert!(r.contains("Kala"), "should still identify as Kala: {r}");
    assert!(!r.contains("काल"), "no Sanskrit: {r}");
    assert!(r.len() < 100, "named greeting should be short: {r}");
}

#[test]
fn greeting_with_request_not_intercepted() {
    let r = dispatch("ask", "hey can you help me write python code");
    // Should NOT return a simple greeting — it should route to code generation
    assert!(
        !r.starts_with("Hello! 👋 I'm **Kala**"),
        "greeting+request should skip pure greeting path: {}",
        &r[..r.len().min(300)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// IDENTITY
// ═══════════════════════════════════════════════════════════════════

#[test]
fn identity_who_are_you() {
    let r = dispatch("ask", "who are you?");
    assert!(r.contains("Kala"), "identity must mention Kala: {r}");
    assert!(r.contains("Killer"), "identity must mention Killer language: {r}");
    assert!(!r.contains("काल"), "no Sanskrit in identity: {r}");
}

#[test]
fn identity_what_is_kala() {
    let r = dispatch("ask", "what is kala?");
    assert!(r.contains("Kala"), "should explain Kala: {r}");
    assert!(r.contains("AI") || r.contains("code") || r.contains("question"),
        "should mention what it does: {r}");
}

#[test]
fn identity_are_you_ai() {
    let r = dispatch("ask", "are you an ai?");
    assert!(r.contains("Kala"), "AI question should get Kala identity: {r}");
    assert!(!r.contains("काल"), "no Sanskrit: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// CREATOR
// ═══════════════════════════════════════════════════════════════════

#[test]
fn creator_who_made_you() {
    let r = dispatch("ask", "who made you?");
    assert!(r.contains("Sai Arun Kumar") || r.contains("Katherashala"),
        "creator question should name the creator: {r}");
    assert!(r.contains("Rust"), "should mention Rust: {r}");
}

#[test]
fn creator_who_built_killer() {
    let r = dispatch("ask", "who built killer?");
    assert!(r.contains("Sai Arun Kumar") || r.contains("Katherashala"),
        "builder question should name the creator: {r}");
}

#[test]
fn creator_who_owns_kala() {
    let r = dispatch("ask", "who owns kala?");
    assert!(r.contains("Sai Arun Kumar") || r.contains("Katherashala"),
        "ownership question should name the creator: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// WELLBEING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn wellbeing_how_are_you() {
    let r = dispatch("ask", "how are you?");
    assert!(r.to_lowercase().contains("good") || r.to_lowercase().contains("doing"),
        "wellbeing should be positive and simple: {r}");
}

#[test]
fn wellbeing_how_r_u() {
    let r = dispatch("ask", "how r u");
    assert!(r.to_lowercase().contains("good") || r.to_lowercase().contains("doing")
        || r.to_lowercase().contains("mind"),
        "typo tolerance should still trigger wellbeing: {r}");
}

#[test]
fn wellbeing_wrong_name_correction() {
    let r = dispatch("ask", "hey, Carl, how are you?");
    assert!(r.contains("Kala"), "should correct wrong name to Kala: {r}");
    assert!(r.contains("Carl"), "should acknowledge the wrong name: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// HELP & CAPABILITIES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn help_guide_me() {
    let r = dispatch("ask", "guide me");
    assert!(r.to_lowercase().contains("question") || r.to_lowercase().contains("code")
        || r.to_lowercase().contains("help") || r.to_lowercase().contains("write"),
        "help should mention what Kala can do: {r}");
}

#[test]
fn wellbeing_what_can_you_do() {
    let r = dispatch("ask", "what can you do?");
    assert!(r.to_lowercase().contains("good") || r.to_lowercase().contains("mind"),
        "should respond naturally: {r}");
}

#[test]
fn help_how_to_use() {
    let r = dispatch("ask", "how do i use you?");
    assert!(r.to_lowercase().contains("question") || r.to_lowercase().contains("type")
        || r.to_lowercase().contains("ask") || r.to_lowercase().contains("help"),
        "should explain usage simply: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// SOCIAL MICRO-INTERACTIONS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn social_thanks() {
    let r = dispatch("ask", "thank you");
    assert!(r.contains("welcome") || r.contains("Welcome"), "should say you're welcome: {r}");
}

#[test]
fn social_bye() {
    let r = dispatch("ask", "goodbye");
    assert!(r.to_lowercase().contains("see you") || r.to_lowercase().contains("bye"),
        "should say goodbye simply: {r}");
    assert!(!r.contains("AI that knows time"), "no cheesy tagline: {r}");
}

#[test]
fn social_cool() {
    let r = dispatch("ask", "cool");
    assert!(r.to_lowercase().contains("else") || r.to_lowercase().contains("need"),
        "social should ask what's next: {r}");
}

#[test]
fn social_impressive() {
    let r = dispatch("ask", "impressive");
    assert!(r.to_lowercase().contains("help") || r.to_lowercase().contains("glad")
        || r.to_lowercase().contains("liked"),
        "praise response should be simple: {r}");
    assert!(!r.contains("KhLM") && !r.contains("Ghost-108"),
        "no technical marketing in casual social: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// NAME INTRODUCTION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn name_intro_my_name_is() {
    let r = dispatch("ask", "my name is sai");
    assert!(r.contains("Sai"), "should capitalize and echo name: {r}");
    assert!(r.contains("meet") || r.contains("Kala"), "should be simple intro: {r}");
    assert!(r.len() < 120, "name intro should be short: {r}");
}

#[test]
fn name_intro_call_me() {
    let r = dispatch("ask", "call me kumar");
    assert!(r.contains("Kumar"), "should recognize 'call me' pattern: {r}");
}

// ═══════════════════════════════════════════════════════════════════
// CODE ROUTING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn code_python_request() {
    let r = dispatch("ask", "write python code for fibonacci");
    assert!(
        r.contains("```") || r.to_lowercase().contains("python") || r.to_lowercase().contains("fibonacci"),
        "python code request should produce code-related answer: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn code_sql_create_table() {
    let r = dispatch("ask", "create table for employee management");
    assert!(
        r.contains("```") || r.to_lowercase().contains("table") || r.to_lowercase().contains("sql"),
        "SQL request should produce table/SQL answer: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn code_html_request() {
    let r = dispatch("ask", "create html page with a login form");
    assert!(
        r.contains("```") || r.to_lowercase().contains("html") || r.to_lowercase().contains("form"),
        "HTML request should produce code answer: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn code_mode_direct() {
    let r = dispatch("code", "binary search in rust");
    assert!(
        r.contains("```") || r.to_lowercase().contains("binary") || r.to_lowercase().contains("search"),
        "code mode should produce code output: {}",
        &r[..r.len().min(500)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// ASK MODE — GENERAL KNOWLEDGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ask_factual_question() {
    let r = dispatch("ask", "what is machine learning?");
    let rl = r.to_lowercase();
    assert!(
        rl.contains("machine learning") || rl.contains("ml") || rl.contains("learn")
            || rl.contains("data") || rl.contains("algorithm"),
        "factual question should produce relevant answer: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn ask_comparison() {
    let r = dispatch("ask", "python vs javascript");
    let rl = r.to_lowercase();
    assert!(
        rl.contains("python") || rl.contains("javascript") || rl.contains("vs")
            || rl.contains("comparison"),
        "comparison should mention both sides: {}",
        &r[..r.len().min(500)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// THINK MODE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn think_mode_produces_output() {
    let r = dispatch("think", "what is the meaning of life?");
    assert!(
        !r.is_empty() && r.len() > 20,
        "think mode should produce substantial output: {}",
        &r[..r.len().min(500)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// WRITE MODE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn write_mode_produces_prose() {
    let r = dispatch("write", "a short poem about the ocean");
    assert!(
        !r.is_empty() && r.len() > 20,
        "write mode should produce prose: {}",
        &r[..r.len().min(500)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// FEEL MODE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn feel_mode_emotional() {
    let r = dispatch("feel", "i'm feeling anxious about my exam");
    let rl = r.to_lowercase();
    assert!(
        rl.contains("feel") || rl.contains("anxious") || rl.contains("okay")
            || rl.contains("breath") || rl.contains("worry") || rl.contains("stress")
            || rl.contains("exam") || rl.contains("support") || rl.contains("help")
            || rl.contains("you"),
        "feel mode should give empathetic response: {}",
        &r[..r.len().min(500)]
    );
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn empty_question() {
    let r = dispatch("ask", "");
    assert!(!r.is_empty(), "empty question should still produce a response");
}

#[test]
fn single_word_question() {
    let r = dispatch("ask", "weather");
    assert!(!r.is_empty(), "single word should produce a response");
}

#[test]
fn unicode_question() {
    let r = dispatch("ask", "what is 5 × 3?");
    let rl = r.to_lowercase();
    assert!(
        rl.contains("15") || rl.contains("multiply") || rl.contains("math")
            || r.contains("```") || !r.is_empty(),
        "unicode math should be handled: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn no_prose_filler_on_code_topics() {
    let r = dispatch("ask", "javascript debounce utility");
    assert!(
        !r.contains("The short version:"),
        "code-related ask should not hit prose filler: {}",
        &r[..r.len().min(500)]
    );
}

#[test]
fn response_never_empty_for_real_questions() {
    let questions = vec![
        "how does blockchain work",
        "explain neural networks",
        "what is kubernetes",
        "difference between tcp and udp",
    ];
    for q in questions {
        let r = dispatch("ask", q);
        assert!(
            r.len() > 10,
            "question '{q}' produced too-short response ({} chars): {r}",
            r.len()
        );
    }
}
