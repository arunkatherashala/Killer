//! ULTIMATE Interactive Chat Test for Kala AI
//! Simulates REAL conversations people have with AI chatbots.
//! Goal: Every response must be natural, helpful, and NEVER empty or generic.

use killer_native::builtin::BuiltinFunctions;

fn ask(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("ask", q, "casual", "killer")
}
fn code(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("code", q, "casual", "killer")
}
fn think(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("think", q, "casual", "killer")
}
fn write_mode(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("write", q, "casual", "killer")
}
fn feel(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("feel", q, "casual", "killer")
}

fn not_empty(r: &str, context: &str) {
    assert!(r.trim().len() > 5, "EMPTY/TOO SHORT for: \"{}\"\nGot: {:?}", context, r);
}
fn no_stub(r: &str, context: &str) {
    assert!(
        !r.contains("Connect an LLM") && !r.contains("ready to write your function"),
        "GENERIC STUB for: \"{}\"\nGot: {}",
        context, &r[..r.len().min(200)]
    );
}
fn is_natural(r: &str, context: &str) {
    // Should NOT have Sanskrit/branded stuff
    assert!(!r.contains("काल"), "Contains Sanskrit for: \"{}\"", context);
    assert!(!r.contains("Time and Fate"), "Contains branded text for: \"{}\"", context);
}

// ═══════════════════════════════════════════════════════════════════════
// 1. FIRST CONTACT — How people start chatting
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_01_just_hi() {
    let r = ask("hi");
    not_empty(&r, "hi");
    is_natural(&r, "hi");
    assert!(r.to_lowercase().contains("kala"), "Should introduce itself on 'hi'");
}

#[test] fn chat_02_hello() {
    let r = ask("hello");
    not_empty(&r, "hello");
    assert!(r.to_lowercase().contains("kala") || r.to_lowercase().contains("help"),
        "Should be welcoming: {}", r);
}

#[test] fn chat_03_hey_whats_up() {
    let r = ask("hey");
    not_empty(&r, "hey");
    is_natural(&r, "hey");
}

#[test] fn chat_04_sup() {
    let r = ask("sup");
    not_empty(&r, "sup");
}

#[test] fn chat_05_yo() {
    let r = ask("yo");
    not_empty(&r, "yo");
}

#[test] fn chat_06_good_morning() {
    let r = ask("good morning");
    not_empty(&r, "good morning");
}

#[test] fn chat_07_namaste() {
    let r = ask("namaste");
    not_empty(&r, "namaste");
}

#[test] fn chat_08_hola() {
    let r = ask("hola");
    not_empty(&r, "hola");
}

// ═══════════════════════════════════════════════════════════════════════
// 2. NAME INTRODUCTION — How people introduce themselves
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_09_hi_i_am_deepak() {
    let r = ask("hi i am deepak");
    not_empty(&r, "hi i am deepak");
    assert!(r.contains("Deepak"), "Should use the name 'Deepak': {}", r);
    is_natural(&r, "hi i am deepak");
}

#[test] fn chat_10_my_name_is_priya() {
    let r = ask("my name is priya");
    not_empty(&r, "my name is priya");
    assert!(r.contains("Priya"), "Should use 'Priya': {}", r);
}

#[test] fn chat_11_call_me_arun() {
    let r = ask("call me arun");
    not_empty(&r, "call me arun");
    assert!(r.contains("Arun"), "Should use 'Arun': {}", r);
}

#[test] fn chat_12_im_sai() {
    let r = ask("i'm sai");
    not_empty(&r, "i'm sai");
    assert!(r.contains("Sai"), "Should use 'Sai': {}", r);
}

// ═══════════════════════════════════════════════════════════════════════
// 3. IDENTITY — Who are you questions
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_13_who_are_you() {
    let r = ask("who are you?");
    not_empty(&r, "who are you");
    assert!(r.to_lowercase().contains("kala"), "Should say 'Kala': {}", r);
    assert!(r.to_lowercase().contains("killer"), "Should mention Killer lang: {}", r);
    is_natural(&r, "who are you");
}

#[test] fn chat_14_what_is_kala() {
    let r = ask("what is kala?");
    not_empty(&r, "what is kala");
    assert!(r.to_lowercase().contains("ai") || r.to_lowercase().contains("killer"),
        "Should explain itself: {}", r);
}

#[test] fn chat_15_are_you_ai() {
    let r = ask("are you an ai?");
    not_empty(&r, "are you an ai");
}

#[test] fn chat_16_are_you_human() {
    let r = ask("are you human?");
    not_empty(&r, "are you human");
}

// ═══════════════════════════════════════════════════════════════════════
// 4. CREATOR QUESTIONS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_17_who_made_you() {
    let r = ask("who made you?");
    not_empty(&r, "who made you");
    assert!(r.contains("Sai") || r.contains("Katherashala"), "Should name creator: {}", r);
}

#[test] fn chat_18_who_built_killer() {
    let r = ask("who built killer?");
    not_empty(&r, "who built killer");
    assert!(r.contains("Sai") || r.contains("Katherashala"), "Should name creator: {}", r);
}

#[test] fn chat_19_who_owns_kala() {
    let r = ask("who owns kala?");
    not_empty(&r, "who owns kala");
}

// ═══════════════════════════════════════════════════════════════════════
// 5. WELLBEING — How are you variations
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_20_how_are_you() {
    let r = ask("how are you?");
    not_empty(&r, "how are you");
    is_natural(&r, "how are you");
    assert!(r.len() < 200, "Should be short and natural, not an essay: {}", r);
}

#[test] fn chat_21_how_r_u() {
    let r = ask("how r u");
    not_empty(&r, "how r u");
}

#[test] fn chat_22_how_you_doing() {
    let r = ask("how you doing?");
    not_empty(&r, "how you doing");
}

#[test] fn chat_23_wrong_name_correction() {
    let r = ask("hey carl, how are you?");
    not_empty(&r, "wrong name carl");
    assert!(r.to_lowercase().contains("kala"), "Should correct to Kala: {}", r);
}

// ═══════════════════════════════════════════════════════════════════════
// 6. HELP & CAPABILITIES
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_24_what_can_you_do() {
    let r = ask("what can you do?");
    not_empty(&r, "what can you do");
}

#[test] fn chat_25_help_me() {
    let r = ask("help me");
    not_empty(&r, "help me");
}

#[test] fn chat_26_guide_me() {
    let r = ask("guide me");
    not_empty(&r, "guide me");
}

#[test] fn chat_27_im_new() {
    let r = ask("i'm new here");
    not_empty(&r, "i'm new");
}

// ═══════════════════════════════════════════════════════════════════════
// 7. SOCIAL MICRO-INTERACTIONS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_28_thanks() {
    let r = ask("thanks");
    not_empty(&r, "thanks");
    assert!(r.to_lowercase().contains("welcome") || r.to_lowercase().contains("else"),
        "Should acknowledge thanks: {}", r);
}

#[test] fn chat_29_thank_you() {
    let r = ask("thank you so much");
    not_empty(&r, "thank you so much");
}

#[test] fn chat_30_bye() {
    let r = ask("bye");
    not_empty(&r, "bye");
    assert!(r.to_lowercase().contains("see you") || r.to_lowercase().contains("bye") 
        || r.to_lowercase().contains("come back"),
        "Should say goodbye: {}", r);
}

#[test] fn chat_31_cool() {
    let r = ask("cool");
    not_empty(&r, "cool");
}

#[test] fn chat_32_wow() {
    let r = ask("wow");
    not_empty(&r, "wow");
}

#[test] fn chat_33_ok() {
    let r = ask("ok");
    not_empty(&r, "ok");
}

#[test] fn chat_34_got_it() {
    let r = ask("got it");
    not_empty(&r, "got it");
}

#[test] fn chat_35_impressive() {
    let r = ask("impressive");
    not_empty(&r, "impressive");
}

// ═══════════════════════════════════════════════════════════════════════
// 8. CODE REQUESTS THROUGH CHAT (ask mode, not code mode)
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_36_write_java_loop() {
    let r = ask("write java for loop program");
    not_empty(&r, "java for loop");
    no_stub(&r, "java for loop");
    assert!(r.contains("```"), "Should have code block: {}", &r[..r.len().min(100)]);
}

#[test] fn chat_37_python_hello_world() {
    let r = ask("write python hello world");
    not_empty(&r, "python hello world");
    no_stub(&r, "python hello world");
    assert!(r.contains("```"), "Should have code block");
}

#[test] fn chat_38_can_you_write_code() {
    let r = ask("can you write a python program for fibonacci");
    not_empty(&r, "can you write fibonacci");
    no_stub(&r, "fibonacci");
}

#[test] fn chat_39_how_to_write() {
    let r = ask("how to write for loop in java");
    not_empty(&r, "how to for loop java");
    no_stub(&r, "how to for loop java");
}

// ═══════════════════════════════════════════════════════════════════════
// 9. GREETING + REQUEST COMBO (should NOT just greet, should do the task)
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_40_hi_write_code() {
    let r = ask("hi can you write a python calculator");
    // Should NOT just respond with a greeting — should write code
    no_stub(&r, "hi can you write calculator");
    assert!(r.contains("```") || r.to_lowercase().contains("calc"),
        "Should generate code, not just greet: {}", &r[..r.len().min(200)]);
}

#[test] fn chat_41_hello_help_me_write() {
    let r = ask("hello can you help me write a java program");
    no_stub(&r, "hello help me write java");
}

#[test] fn chat_42_hey_generate() {
    let r = ask("hey generate python fibonacci code");
    no_stub(&r, "hey generate fibonacci");
}

// ═══════════════════════════════════════════════════════════════════════
// 10. THINK MODE — Deep reasoning
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_43_think_comparison() {
    let r = think("which is faster, quicksort or mergesort?");
    not_empty(&r, "think: quicksort vs mergesort");
    assert!(r.len() > 50, "Think mode should give detailed answer: {}", r);
}

#[test] fn chat_44_think_explain() {
    let r = think("explain how recursion works");
    not_empty(&r, "think: recursion");
}

// ═══════════════════════════════════════════════════════════════════════
// 11. WRITE MODE — Prose generation
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_45_write_essay() {
    let r = write_mode("write an essay about artificial intelligence");
    not_empty(&r, "write: AI essay");
    assert!(r.len() > 100, "Write mode should produce substantial text: len={}", r.len());
}

#[test] fn chat_46_write_poem() {
    let r = write_mode("write a poem about coding");
    not_empty(&r, "write: coding poem");
}

// ═══════════════════════════════════════════════════════════════════════
// 12. FEEL MODE — Emotional support
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_47_feel_sad() {
    let r = feel("i'm feeling sad today");
    not_empty(&r, "feel: sad");
    is_natural(&r, "feel: sad");
}

#[test] fn chat_48_feel_happy() {
    let r = feel("i'm so happy right now!");
    not_empty(&r, "feel: happy");
}

#[test] fn chat_49_feel_stressed() {
    let r = feel("i'm really stressed about my exams");
    not_empty(&r, "feel: stressed");
}

// ═══════════════════════════════════════════════════════════════════════
// 13. CODE MODE — Direct code requests
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_50_code_java_class() {
    let r = code("write java class program");
    no_stub(&r, "code: java class");
    assert!(r.contains("```java"), "Should have java code block");
}

#[test] fn chat_51_code_python_api() {
    let r = code("write python api request");
    no_stub(&r, "code: python api");
    assert!(r.contains("```python"), "Should have python code block");
}

#[test] fn chat_52_code_rust_enum() {
    let r = code("write rust enum example");
    no_stub(&r, "code: rust enum");
    assert!(r.contains("```rust"), "Should have rust code block");
}

// ═══════════════════════════════════════════════════════════════════════
// 14. EDGE CASES — Weird inputs
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_53_empty() {
    let r = ask("");
    // Should handle gracefully
    not_empty(&r, "empty input");
}

#[test] fn chat_54_single_char() {
    let r = ask("a");
    not_empty(&r, "single char 'a'");
}

#[test] fn chat_55_just_question_mark() {
    let r = ask("?");
    not_empty(&r, "just '?'");
}

#[test] fn chat_56_unicode_emoji() {
    let r = ask("hello 😊");
    not_empty(&r, "hello with emoji");
}

#[test] fn chat_57_all_caps() {
    let r = ask("HELLO HOW ARE YOU");
    not_empty(&r, "ALL CAPS greeting");
}

#[test] fn chat_58_lots_of_spaces() {
    let r = ask("  hello   kala   ");
    not_empty(&r, "extra spaces");
}

#[test] fn chat_59_mixed_case() {
    let r = ask("HeLLo KaLa");
    not_empty(&r, "mixed case");
}

// ═══════════════════════════════════════════════════════════════════════
// 15. TYPO TOLERANCE
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_60_hellow() {
    let r = ask("hellow");
    // Not a recognized greeting, but should still respond
    not_empty(&r, "hellow (typo)");
}

#[test] fn chat_61_thnks() {
    let r = ask("thx");
    not_empty(&r, "thx");
}

// ═══════════════════════════════════════════════════════════════════════
// 16. MULTI-LANGUAGE GREETINGS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_62_bonjour() {
    let r = ask("bonjour");
    not_empty(&r, "bonjour");
}

#[test] fn chat_63_ciao() {
    let r = ask("ciao");
    not_empty(&r, "ciao");
}

#[test] fn chat_64_vanakkam() {
    let r = ask("vanakkam");
    not_empty(&r, "vanakkam");
}

// ═══════════════════════════════════════════════════════════════════════
// 17. NATURAL CONVERSATION FLOW (the way real people talk)
// ═══════════════════════════════════════════════════════════════════════
#[test] fn chat_65_casual_intro() {
    let r = ask("hey i'm new to programming, can you help?");
    not_empty(&r, "casual intro");
    // Should not just be a greeting — should offer help
}

#[test] fn chat_66_explain_concept() {
    let r = ask("what is object oriented programming?");
    not_empty(&r, "what is OOP");
    assert!(r.len() > 50, "Should give a real explanation");
}

#[test] fn chat_67_compare() {
    let r = ask("python vs java which is better?");
    not_empty(&r, "python vs java");
}

#[test] fn chat_68_opinion() {
    let r = ask("what do you think about rust?");
    not_empty(&r, "opinion on rust");
}

// ═══════════════════════════════════════════════════════════════════════
// 18. RESPONSE QUALITY CHECKS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn quality_01_no_sanskrit_in_greeting() {
    let r = ask("hi i am raj");
    is_natural(&r, "greeting with name");
    assert!(!r.contains("काल"), "No Sanskrit");
    assert!(!r.contains("Sanskrit"), "No Sanskrit mention");
}

#[test] fn quality_02_no_bullet_lists_in_greeting() {
    let r = ask("hello");
    assert!(!r.contains("- **"), "No bullet lists in greeting: {}", r);
    assert!(!r.contains("• "), "No bullet points in greeting: {}", r);
}

#[test] fn quality_03_short_greeting() {
    let r = ask("hi");
    assert!(r.len() < 150, "Greeting should be short ({} chars): {}", r.len(), r);
}

#[test] fn quality_04_short_wellbeing() {
    let r = ask("how are you?");
    assert!(r.len() < 150, "Wellbeing should be short ({} chars): {}", r.len(), r);
}

#[test] fn quality_05_short_thanks() {
    let r = ask("thanks");
    assert!(r.len() < 100, "Thanks should be short ({} chars): {}", r.len(), r);
}

#[test] fn quality_06_short_bye() {
    let r = ask("bye");
    assert!(r.len() < 100, "Bye should be short ({} chars): {}", r.len(), r);
}

#[test] fn quality_07_code_has_code_block() {
    let r = code("write java for loop");
    assert!(r.contains("```java"), "Code mode must have java block: {}", &r[..r.len().min(100)]);
}

#[test] fn quality_08_identity_concise() {
    let r = ask("who are you?");
    assert!(r.len() < 300, "Identity should be concise ({} chars): {}", r.len(), r);
    is_natural(&r, "identity");
}

#[test] fn quality_09_creator_concise() {
    let r = ask("who made you?");
    assert!(r.len() < 200, "Creator should be concise ({} chars): {}", r.len(), r);
}

#[test] fn quality_10_help_is_helpful() {
    let r = ask("help me");
    assert!(r.to_lowercase().contains("code") || r.to_lowercase().contains("question")
        || r.to_lowercase().contains("help") || r.to_lowercase().contains("ask"),
        "Help response should mention capabilities: {}", r);
}

// ═══════════════════════════════════════════════════════════════════════
// 19. CONSECUTIVE INTERACTIONS (simulating a chat session)
// ═══════════════════════════════════════════════════════════════════════
#[test] fn session_flow() {
    // Step 1: User says hi
    let r1 = ask("hi");
    not_empty(&r1, "session: hi");

    // Step 2: User asks for code
    let r2 = code("write python for loop");
    no_stub(&r2, "session: python for loop");
    assert!(r2.contains("```python"), "Should have python code");

    // Step 3: User says thanks
    let r3 = ask("thanks");
    not_empty(&r3, "session: thanks");

    // Step 4: User asks another code
    let r4 = code("write java hello world");
    no_stub(&r4, "session: java hello world");
    assert!(r4.contains("```java"), "Should have java code");

    // Step 5: User says bye
    let r5 = ask("bye");
    not_empty(&r5, "session: bye");
}

// ═══════════════════════════════════════════════════════════════════════
// 20. RAPID-FIRE RANDOM QUESTIONS (stress test)
// ═══════════════════════════════════════════════════════════════════════
#[test] fn rapid_fire() {
    let questions = vec![
        ("ask", "what is machine learning?"),
        ("code", "write python fibonacci"),
        ("ask", "how does the internet work?"),
        ("code", "write java calculator program"),
        ("ask", "what is an API?"),
        ("code", "write python list comprehension"),
        ("ask", "explain databases"),
        ("code", "write java for loop"),
        ("ask", "what is cloud computing?"),
        ("code", "write rust hello world"),
    ];
    for (mode, q) in &questions {
        let r = BuiltinFunctions::kala_dispatch(mode, q, "casual", "killer");
        not_empty(&r, q);
        if *mode == "code" {
            no_stub(&r, q);
            assert!(r.contains("```"), "Code should have block for: {}", q);
        }
    }
}
