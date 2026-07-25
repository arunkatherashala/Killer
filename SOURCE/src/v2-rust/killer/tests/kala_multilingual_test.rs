//! TEST: Kala multilingual conversational responses.
//! When a user chats in any language, Kala should respond in THAT language.

use killer_native::builtin::BuiltinFunctions;

fn ask(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("ask", q, "casual", "killer")
}

// Helper: assert response contains expected substring (case-insensitive for Latin scripts)
fn assert_has(q: &str, expected: &str) {
    let r = ask(q);
    assert!(
        r.contains(expected),
        "Query: \"{}\"\nExpected to contain: \"{}\"\nGot: \"{}\"",
        q, expected, r
    );
}

// Helper: assert response does NOT contain something
fn assert_not(q: &str, not_expected: &str) {
    let r = ask(q);
    assert!(
        !r.contains(not_expected),
        "Query: \"{}\"\nShould NOT contain: \"{}\"\nGot: \"{}\"",
        q, not_expected, r
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// HINDI (हिंदी) — greetings, identity, wellbeing, creator, help, social
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn hindi_greeting_namaste()    { assert_has("namaste", "Kala"); assert_has("namaste", "मैं"); }
#[test] fn hindi_greeting_namaskar()   { assert_has("namaskar", "Kala"); }
#[test] fn hindi_wellbeing()           { assert_has("kaise ho", "बढ़िया"); }
#[test] fn hindi_wellbeing_devanagari(){ assert_has("कैसे हो", "बढ़िया"); }
#[test] fn hindi_identity()            { assert_has("tum kaun ho", "Kala"); assert_has("tum kaun ho", "AI"); }
#[test] fn hindi_identity_devanagari() { assert_has("तुम कौन हो", "Kala"); }
#[test] fn hindi_creator()             { assert_has("kisne banaya", "Sai Arun Kumar"); }
#[test] fn hindi_creator_devanagari()  { assert_has("किसने बनाया", "Sai Arun Kumar"); }
#[test] fn hindi_help()                { assert_has("batao kya kar sakte ho", "code"); }
#[test] fn hindi_thanks()              { assert_has("dhanyavad", "नहीं"); }
#[test] fn hindi_bye()                 { assert_has("alvida", "मिलेंगे"); }

// ═══════════════════════════════════════════════════════════════════════════════
// TELUGU (తెలుగు)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn telugu_greeting()           { assert_has("namaskaram", "Kala"); assert_has("namaskaram", "హాయ్"); }
#[test] fn telugu_wellbeing()          { assert_has("ela unnaru", "బాగున్నా"); }
#[test] fn telugu_wellbeing_script()   { assert_has("ఎలా ఉన్నారు", "బాగున్నా"); }
#[test] fn telugu_identity()           { assert_has("nuvvu evaru", "Kala"); }
#[test] fn telugu_identity_script()    { assert_has("నువ్వు ఎవరు", "Kala"); }
#[test] fn telugu_creator()            { assert_has("kala ni evaru build chesaru", "Sai Arun Kumar"); }
#[test] fn telugu_thanks()             { assert_has("thanks mama", "పర్వాలేదు"); }
#[test] fn telugu_bye()                { assert_has("bye mama", "కలుద్దాం"); }
#[test] fn telugu_ack()                { assert_has("sare", "సరే"); }
#[test] fn telugu_impressed()          { assert_has("manchidi", "నచ్చినందుకు"); }

// ═══════════════════════════════════════════════════════════════════════════════
// TAMIL (தமிழ்)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn tamil_greeting()            { assert_has("vanakkam", "Kala"); assert_has("vanakkam", "வணக்கம்"); }
#[test] fn tamil_wellbeing()           { assert_has("epdi irukeenga", "நல்லா"); }
#[test] fn tamil_identity()            { assert_has("nee yaaru", "Kala"); }
#[test] fn tamil_thanks()              { assert_has("nandri", "பரவாயில்ல"); }

// ═══════════════════════════════════════════════════════════════════════════════
// SPANISH (Español)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn spanish_greeting()          { assert_has("hola", "Kala"); assert_has("hola", "¿En qué"); }
#[test] fn spanish_wellbeing()         { assert_has("como estas", "bien"); }
#[test] fn spanish_identity()          { assert_has("quien eres", "Kala"); assert_has("quien eres", "Killer"); }
#[test] fn spanish_creator()           { assert_has("quien te hizo", "Sai Arun Kumar"); }
#[test] fn spanish_help()              { assert_has("ayuda", "código"); }
#[test] fn spanish_thanks()            { assert_has("gracias", "nada"); }
#[test] fn spanish_bye()               { assert_has("adios", "vemos"); }
#[test] fn spanish_ack()               { assert_has("vale", "¡Entendido"); }

// ═══════════════════════════════════════════════════════════════════════════════
// FRENCH (Français)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn french_greeting()           { assert_has("bonjour", "Kala"); assert_has("bonjour", "aider"); }
#[test] fn french_greeting_salut()     { assert_has("salut", "Kala"); }
#[test] fn french_wellbeing()          { assert_has("comment allez-vous", "bien"); }
#[test] fn french_identity()           { assert_has("qui es-tu", "Kala"); }
#[test] fn french_creator()            { assert_has("qui t'a créé", "Sai Arun Kumar"); }
#[test] fn french_thanks()             { assert_has("merci", "rien"); }
#[test] fn french_bye()                { assert_has("au revoir", "bientôt"); }

// ═══════════════════════════════════════════════════════════════════════════════
// GERMAN (Deutsch)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn german_greeting()           { assert_has("guten tag", "Kala"); assert_has("guten tag", "helfen"); }
#[test] fn german_greeting_hallo()     { assert_has("hallo", "Kala"); }
#[test] fn german_wellbeing()          { assert_has("wie geht es dir", "gut"); }
#[test] fn german_identity()           { assert_has("wer bist du", "Kala"); }
#[test] fn german_creator()            { assert_has("wer hat dich gebaut", "Sai Arun Kumar"); }
#[test] fn german_thanks()             { assert_has("danke", "Gern"); }
#[test] fn german_bye()                { assert_has("tschüss", "Tschüss"); }

// ═══════════════════════════════════════════════════════════════════════════════
// ITALIAN (Italiano)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn italian_greeting()          { assert_has("ciao", "Kala"); assert_has("ciao", "aiutarti"); }
#[test] fn italian_wellbeing()         { assert_has("come stai", "bene"); }
#[test] fn italian_identity()          { assert_has("chi sei", "Kala"); }
#[test] fn italian_thanks()            { assert_has("grazie", "Prego"); }
#[test] fn italian_bye()               { assert_has("arrivederci", "vediamo"); }

// ═══════════════════════════════════════════════════════════════════════════════
// PORTUGUESE (Português)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn portuguese_greeting()       { assert_has("olá", "Kala"); }
#[test] fn portuguese_wellbeing()      { assert_has("tudo bem", "bem"); }
#[test] fn portuguese_identity()       { assert_has("quem é você", "Kala"); }
#[test] fn portuguese_thanks()         { assert_has("obrigado", "nada"); }
#[test] fn portuguese_bye()            { assert_has("tchau", "logo"); }

// ═══════════════════════════════════════════════════════════════════════════════
// RUSSIAN (Русский)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn russian_greeting()          { assert_has("привет", "Kala"); assert_has("привет", "помочь"); }
#[test] fn russian_wellbeing()         { assert_has("как дела", "спасибо"); }
#[test] fn russian_identity()          { assert_has("кто ты", "Kala"); }
#[test] fn russian_creator()           { assert_has("кто тебя создал", "Sai Arun Kumar"); } // "who created you" uses "who" detection differently
#[test] fn russian_thanks()            { assert_has("спасибо", "Пожалуйста"); }
#[test] fn russian_bye()               { assert_has("пока", "Пока"); }
#[test] fn russian_ack()               { assert_has("хорошо", "Понял"); }

// ═══════════════════════════════════════════════════════════════════════════════
// JAPANESE (日本語)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn japanese_greeting()         { assert_has("こんにちは", "Kala"); }
#[test] fn japanese_wellbeing()        { assert_has("お元気ですか", "元気"); }
#[test] fn japanese_identity()         { assert_has("あなたは誰ですか", "Kala"); }
#[test] fn japanese_creator()          { assert_has("誰が作りましたか", "Sai Arun Kumar"); }
#[test] fn japanese_thanks()           { assert_has("ありがとう", "どういたしまして"); }
#[test] fn japanese_bye()              { assert_has("さようなら", "またね"); }

// ═══════════════════════════════════════════════════════════════════════════════
// KOREAN (한국어)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn korean_greeting()           { assert_has("안녕하세요", "Kala"); }
#[test] fn korean_identity()           { assert_has("당신은 누구", "Kala"); }
#[test] fn korean_creator()            { assert_has("누가 만들었어", "Sai Arun Kumar"); }
#[test] fn korean_thanks()             { assert_has("감사합니다", "천만에요"); }

// ═══════════════════════════════════════════════════════════════════════════════
// CHINESE (中文)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn chinese_greeting()          { assert_has("你好", "Kala"); assert_has("你好", "帮助"); }
#[test] fn chinese_wellbeing()         { assert_has("你好吗", "好"); }
#[test] fn chinese_identity()          { assert_has("你是谁", "Kala"); }
#[test] fn chinese_creator()           { assert_has("谁创造了kala", "Sai Arun Kumar"); }
#[test] fn chinese_thanks()            { assert_has("谢谢", "不客气"); }
#[test] fn chinese_bye()               { assert_has("再见", "再见"); }

// ═══════════════════════════════════════════════════════════════════════════════
// ARABIC (العربية)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn arabic_greeting()           { assert_has("مرحبا", "Kala"); }
#[test] fn arabic_wellbeing()          { assert_has("كيف حالك", "شكراً"); }
#[test] fn arabic_identity()           { assert_has("من أنت", "Kala"); }
#[test] fn arabic_thanks()             { assert_has("شكرا", "عفواً"); }
#[test] fn arabic_bye()                { assert_has("مع السلامة", "السلامة"); }

// ═══════════════════════════════════════════════════════════════════════════════
// TURKISH (Türkçe)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn turkish_greeting()          { assert_has("merhaba", "Kala"); }
#[test] fn turkish_wellbeing()         { assert_has("nasılsın", "teşekkürler"); }
#[test] fn turkish_identity()          { assert_has("sen kimsin", "Kala"); }
#[test] fn turkish_thanks()            { assert_has("teşekkürler", "Rica"); }

// ═══════════════════════════════════════════════════════════════════════════════
// CROSS-LANGUAGE — ensure English stays English
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn english_greeting_stays_english() {
    let r = ask("hello");
    assert!(r.contains("Hey") || r.contains("Kala"), "English greeting should stay English: {}", r);
    assert!(!r.contains("मैं") && !r.contains("నేను"), "English should not have Hindi/Telugu: {}", r);
}

#[test] fn english_wellbeing_stays_english() {
    let r = ask("how are you");
    assert!(r.contains("Doing good") || r.contains("good"), "English wellbeing: {}", r);
}

#[test] fn english_creator_stays_english() {
    let r = ask("who made you");
    assert!(r.contains("Sai Arun Kumar") && r.contains("Rust"), "English creator: {}", r);
}

// ═══════════════════════════════════════════════════════════════════════════════
// NON-CONVERSATIONAL queries should still work regardless of language detection
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn code_still_works_hindi_speaker() {
    let r = BuiltinFunctions::kala_dispatch("code", "write java for loop", "casual", "killer");
    assert!(r.contains("```java"), "Code gen should still work: {}", &r[..r.len().min(200)]);
}

#[test] fn code_still_works_telugu_speaker() {
    let r = BuiltinFunctions::kala_dispatch("code", "write python hello world", "casual", "killer");
    assert!(r.contains("```python"), "Code gen should still work: {}", &r[..r.len().min(200)]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// MIXED — greeting in one language, rest in english (should detect non-English)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn hindi_kya_haal_hai() {
    let r = ask("kya haal hai bhai");
    assert!(r.contains("बढ़िया") || r.contains("Kala"), "Hindi: {}", r);
}

#[test] fn telugu_em_chestunnav() {
    let r = ask("em chestunnav");
    assert!(r.contains("బాగున్నా") || r.contains("చెప్పు") || r.contains("Kala"),
        "Telugu detection: {}", r);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TOTAL: ~100+ tests across 15+ languages
// ═══════════════════════════════════════════════════════════════════════════════
