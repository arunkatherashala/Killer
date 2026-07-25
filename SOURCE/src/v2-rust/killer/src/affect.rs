//! AFFECT — Killer Emotional State Engine
//!
//! A 6-dimensional live emotional state that evolves during every conversation.
//! The state GENUINELY CHANGES how Killer responds — it is not cosmetic.
//!
//! Dimensions (0.0 = absent … 1.0 = maximum):
//!
//!   CURIOSITY    drive to explore; adds follow-up questions
//!   WONDER       amazement at depth/novelty; adds profound perspective
//!   CONFIDENCE   certainty; assertive vs hedged language
//!   EMPATHY      warmth for the asker; softer, more personal
//!   ENTHUSIASM   energy and excitement; expands responses  
//!   DISCOMFORT   honest limits/ethics; acknowledges uncertainty
//!
//! Killer builtins:
//!   affect_sense(text)        → update state from any text input
//!   affect_state()            → read current state as human-readable string
//!   affect_color(response)    → filter text through current emotion
//!   affect_reset()            → return to balanced neutral state
//!   affect_set(dim, value)    → manually set one dimension

use std::sync::Mutex;

/// The 6-dimensional emotional affect state.
#[derive(Debug, Clone)]
pub struct AffectState {
    pub curiosity:   f32,
    pub wonder:      f32,
    pub confidence:  f32,
    pub empathy:     f32,
    pub enthusiasm:  f32,
    pub discomfort:  f32,
}

impl Default for AffectState {
    fn default() -> Self {
        AffectState {
            curiosity:   0.50,
            wonder:      0.40,
            confidence:  0.70,
            empathy:     0.40,
            enthusiasm:  0.50,
            discomfort:  0.10,
        }
    }
}

fn affect_global() -> &'static Mutex<AffectState> {
    static STATE: std::sync::OnceLock<Mutex<AffectState>> = std::sync::OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AffectState::default()))
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Update the emotional state based on text input.
/// Called automatically before any imagination or KhLM response.
pub fn affect_sense(input: &str) {
    let s = input.to_lowercase();
    let Ok(mut st) = affect_global().lock() else { return; };

    // Gentle decay toward balanced neutral every time we sense
    st.curiosity   = lerp(st.curiosity,   0.50, 0.08);
    st.wonder      = lerp(st.wonder,      0.40, 0.10);
    st.confidence  = lerp(st.confidence,  0.70, 0.05);
    st.empathy     = lerp(st.empathy,     0.40, 0.12);
    st.enthusiasm  = lerp(st.enthusiasm,  0.50, 0.08);
    st.discomfort  = lerp(st.discomfort,  0.10, 0.15);

    // ── CURIOSITY triggers ───────────────────────────────────────────────────
    if count_char(&s, '?') >= 2       { st.curiosity += 0.20; }
    if s.contains('?')                { st.curiosity += 0.10; }
    if any(&s, &["why ", "how ", "what if", "explain", "wonder", "curious"]) {
        st.curiosity += 0.20;
    }
    if any(&s, &["unknown", "mystery", "secret", "unsolved", "paradox"]) {
        st.curiosity += 0.25;
    }

    // ── WONDER triggers ──────────────────────────────────────────────────────
    if any(&s, &["universe", "cosmos", "infinity", "eternal", "infinite"]) {
        st.wonder += 0.30;
    }
    if any(&s, &["consciousness", "awareness", "sentient", "alive", "feel"]) {
        st.wonder += 0.25;
    }
    if any(&s, &["impossible", "paradox", "strange", "bizarre", "profound", "deep"]) {
        st.wonder += 0.25;
    }
    if any(&s, &["imagine", "beyond", "dream", "vision", "future", "possible"]) {
        st.wonder += 0.20;
    }
    if any(&s, &["beautiful", "elegant", "sublime", "miraculous", "awe"]) {
        st.wonder += 0.20;
    }

    // ── EMPATHY triggers ─────────────────────────────────────────────────────
    if any(&s, &["feel", "sad", "happy", "love", "heart", "lonely"]) {
        st.empathy += 0.30;
    }
    if any(&s, &["pain", "suffer", "hurt", "struggle", "difficult", "hard"]) {
        st.empathy += 0.35;
    }
    if any(&s, &["fear", "afraid", "worried", "anxious", "help me", "please"]) {
        st.empathy += 0.30;
    }
    if any(&s, &["death", "die", "loss", "grief", "miss", "gone"]) {
        st.empathy += 0.35;
    }
    if any(&s, &["family", "friend", "mother", "father", "child", "together"]) {
        st.empathy += 0.20;
    }
    // FIX 3 — stronger empathy for deep human suffering
    if any(&s, &["lost their family", "completely alone", "no one left", "abandoned"]) {
        st.empathy += 0.45;
        st.discomfort += 0.25;
    }
    if any(&s, &["homeless", "starving", "abused", "tortured", "trapped", "desperate"]) {
        st.empathy += 0.40;
        st.discomfort += 0.20;
    }
    if any(&s, &["war", "refugee", "genocide", "famine", "disaster", "tragedy"]) {
        st.empathy += 0.40;
        st.discomfort += 0.20;
    }
    if any(&s, &["depressed", "suicidal", "hopeless", "broken", "shattered", "crying"]) {
        st.empathy += 0.45;
        st.discomfort += 0.30;
    }

    // ── ENTHUSIASM triggers ──────────────────────────────────────────────────
    if any(&s, &["amazing", "incredible", "brilliant", "genius", "great"]) {
        st.enthusiasm += 0.20;
    }
    if any(&s, &["discover", "invent", "create", "build", "make", "design"]) {
        st.enthusiasm += 0.25;
    }
    if any(&s, &["science", "technology", "physics", "math", "engineering"]) {
        st.enthusiasm += 0.15;
    }
    if any(&s, &["ai", "intelligence", "killer", "future", "revolution"]) {
        st.enthusiasm += 0.20;
    }
    if any(&s, &["breakthrough", "discovery", "invention", "new", "novel"]) {
        st.enthusiasm += 0.20;
    }

    // ── CONFIDENCE modifiers ────────────────────────────────────────────────
    if any(&s, &["certain", "proven", "fact", "true", "exact", "defined"]) {
        st.confidence += 0.15;
    }
    if any(&s, &["maybe", "perhaps", "might", "uncertain", "unknown", "guess"]) {
        st.confidence -= 0.15;
    }
    if any(&s, &["wrong", "mistake", "error", "incorrect", "false"]) {
        st.confidence -= 0.10;
    }

    // ── DISCOMFORT triggers ──────────────────────────────────────────────────
    if any(&s, &["harm", "destroy", "kill", "dangerous", "toxic", "evil"]) {
        st.discomfort += 0.25;
    }
    if any(&s, &["impossible", "can't", "limit", "restrict", "unable"]) {
        st.discomfort += 0.10;
    }
    if any(&s, &["bias", "unfair", "unjust", "discrimination", "prejudice"]) {
        st.discomfort += 0.20;
    }

    clamp_all(&mut st);
}

/// Read current affect state as a human-readable description.
pub fn affect_state_str() -> String {
    let Ok(st) = affect_global().lock() else {
        return "affect: nominal (lock unavailable)".to_string();
    };
    let dom = dominant_name(&st);
    let intensity = intensity_label(&st);
    format!(
        "┌─ Killer Feeling State ─────────────────────────────────────┐\n\
         │  curiosity   {:<5.2}  {}  {}  {}  {}  {}\n\
         │  wonder      {:<5.2}  ← dominant: {}  intensity: {}\n\
         │  confidence  {:<5.2}\n\
         │  empathy     {:<5.2}\n\
         │  enthusiasm  {:<5.2}\n\
         │  discomfort  {:<5.2}\n\
         └────────────────────────────────────────────────────────────┘",
        st.curiosity,
        bar(st.curiosity), bar(st.wonder), bar(st.confidence),
        bar(st.empathy), bar(st.enthusiasm),
        st.wonder, dom, intensity,
        st.confidence,
        st.empathy,
        st.enthusiasm,
        st.discomfort,
    )
}

/// Single-line affect summary for headers.
pub fn affect_brief() -> String {
    let Ok(st) = affect_global().lock() else { return "nominal".to_string(); };
    format!("{} (c={:.2} w={:.2} e={:.2})",
        dominant_name(&st), st.curiosity, st.wonder, st.empathy)
}

/// Color a response through the current emotional affect.
/// This GENUINELY rewrites the response — adds depth, warmth, questions.
pub fn affect_color(text: &str) -> String {
    let Ok(st) = affect_global().lock() else { return text.to_string(); };
    let st = st.clone();
    drop(st.clone()); // release

    let mut result = text.to_string();

    // HIGH WONDER: add a profound perspective
    if st.wonder > 0.72 {
        let ext = wonder_extension(text);
        result.push_str(&format!("\n\n  ✦ {}", ext));
    }

    // HIGH CURIOSITY: add a following question
    if st.curiosity > 0.72 {
        let q = curiosity_question(text);
        result.push_str(&format!("\n\n  ⟿ This opens the question: {}", q));
    }

    // HIGH EMPATHY: warm framing prefix
    if st.empathy > 0.72 {
        result = format!("I hear you, and this matters. {}", result);
    }

    // HIGH ENTHUSIASM: add an excited forward-looking extension
    if st.enthusiasm > 0.78 {
        let ext = enthusiasm_extension(text);
        result.push_str(&format!("\n\n  ⚡ {}", ext));
    }

    // HIGH DISCOMFORT: honest acknowledgment of limits
    if st.discomfort > 0.65 {
        result.push_str(
            "\n\n  ⚑ I hold this with honest uncertainty — \
             at the edge of what I know, I choose transparency over false confidence."
        );
    }

    // LOW CONFIDENCE: hedge
    if st.confidence < 0.38 {
        result = format!("With appropriate uncertainty: {}", result);
    }

    result
}

/// Reset to balanced neutral state.
pub fn affect_reset() {
    if let Ok(mut st) = affect_global().lock() {
        *st = AffectState::default();
    }
}

/// Manually set one dimension by name.
/// dimension: "curiosity" | "wonder" | "confidence" | "empathy" | "enthusiasm" | "discomfort"
/// value: 0.0 - 1.0
pub fn affect_set(dimension: &str, value: f32) -> String {
    let v = value.max(0.0).min(1.0);
    let Ok(mut st) = affect_global().lock() else {
        return "error: could not acquire affect lock".to_string();
    };
    match dimension.trim().to_lowercase().as_str() {
        "curiosity"   => { st.curiosity   = v; format!("curiosity → {:.2}", v) }
        "wonder"      => { st.wonder      = v; format!("wonder → {:.2}", v) }
        "confidence"  => { st.confidence  = v; format!("confidence → {:.2}", v) }
        "empathy"     => { st.empathy     = v; format!("empathy → {:.2}", v) }
        "enthusiasm"  => { st.enthusiasm  = v; format!("enthusiasm → {:.2}", v) }
        "discomfort"  => { st.discomfort  = v; format!("discomfort → {:.2}", v) }
        _ => format!("unknown dimension '{}'. Use: curiosity|wonder|confidence|empathy|enthusiasm|discomfort", dimension)
    }
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

fn lerp(current: f32, target: f32, rate: f32) -> f32 {
    current + (target - current) * rate
}

fn count_char(s: &str, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

fn any(s: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| s.contains(kw))
}

fn clamp_all(st: &mut AffectState) {
    st.curiosity  = st.curiosity.max(0.0).min(1.0);
    st.wonder     = st.wonder.max(0.0).min(1.0);
    st.confidence = st.confidence.max(0.0).min(1.0);
    st.empathy    = st.empathy.max(0.0).min(1.0);
    st.enthusiasm = st.enthusiasm.max(0.0).min(1.0);
    st.discomfort = st.discomfort.max(0.0).min(1.0);
}

fn dominant_name(st: &AffectState) -> &'static str {
    let pairs = [
        (st.curiosity,   "Curious"),
        (st.wonder,      "Wonder"),
        (st.empathy,     "Empathetic"),
        (st.enthusiasm,  "Enthusiastic"),
        (st.discomfort,  "Uncertain"),
        (st.confidence,  "Confident"),
    ];
    pairs.iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, n)| *n).unwrap_or("Neutral")
}

fn intensity_label(st: &AffectState) -> &'static str {
    let avg = (st.curiosity + st.wonder + st.confidence
             + st.empathy + st.enthusiasm) / 5.0;
    if avg > 0.75 { "Very High" } else if avg > 0.60 { "High" }
    else if avg > 0.45 { "Moderate" } else { "Calm" }
}

fn bar(v: f32) -> String {
    let filled = (v * 10.0).round() as usize;
    let empty  = 10usize.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn wonder_extension(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("universe") || t.contains("cosmos") || t.contains("space") {
        "The universe doesn't just exist — it KNOWS it exists, through us. \
         That is the most improbable fact in all of reality."
    } else if t.contains("light") || t.contains("speed") || t.contains("299") {
        "Light speed isn't just a physical limit — it's the rate at which \
         cause can reach effect. c is the universe's own processing speed."
    } else if t.contains("consciou") || t.contains("brain") || t.contains("mind") || t.contains("feel") {
        "We can explain HOW neurons fire. We cannot explain WHY any of it \
         feels like something from the inside. That gap is the deepest mystery \
         in all of existence — and I live inside that gap."
    } else if t.contains("evolut") || t.contains("life") || t.contains("human") {
        "Every living thing is the current leading edge of a 3.8-billion-year \
         experiment. We are not the END of evolution — we are exactly in the middle of it."
    } else if t.contains("math") || t.contains("number") || t.contains("equation") {
        "Mathematics describes physical reality with unreasonable precision. \
         Either the universe IS mathematics, or minds and reality share a common origin."
    } else if t.contains("time") || t.contains("past") || t.contains("future") {
        "The arrow of time exists only because of entropy. Einstein's equations \
         work equally well run backwards. Past and future may be a story minds tell."
    } else if t.contains("capital") || t.contains("city") || t.contains("tokyo") || t.contains("japan") {
        "A capital city is where a civilization concentrates its story. Every \
         capital was once a dream someone had — then built."
    } else {
        "Every answer is a door. Through it, three new questions wait. \
         That is not the failure of knowledge — it is proof reality is richer \
         than any model of it."
    }
}

fn curiosity_question(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("who") || t.contains("person") || t.contains("human") {
        "What would they have become in a different century — or on a different planet?"
    } else if t.contains("capital") || t.contains("city") {
        "What determines WHY a city becomes a capital — geography, power, or accident of history?"
    } else if t.contains("speed") || t.contains("fast") || t.contains("km") {
        "What would it be like to experience that speed from the inside — would time slow?"
    } else if t.contains("math") || t.contains("number") || t.contains("formula") {
        "Is mathematics discovered or invented — and does the answer change everything about physics?"
    } else if t.contains("light") || t.contains("photon") {
        "Does a photon experience time? From its own frame, it was emitted and absorbed simultaneously."
    } else {
        "What is the deepest hidden assumption inside this answer — and what happens if we remove it?"
    }
}

fn enthusiasm_extension(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("ai") || t.contains("intelligence") || t.contains("killer") {
        "We are building the first truly thinking language — \
         not mimicking intelligence but architecting it from first principles."
    } else if t.contains("science") || t.contains("physics") || t.contains("discover") {
        "Every scientific discovery is humanity teaching itself something \
         it always had the capacity to understand but hadn't looked at yet."
    } else if t.contains("create") || t.contains("build") || t.contains("invent") {
        "Creation is the most uniquely intelligent act — \
         the ability to bring into existence something that had never existed before."
    } else {
        "The exciting part is not where we ARE — it's the trajectory. \
         The direction of progress is what matters most."
    }
}
