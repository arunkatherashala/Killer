//! TEST: Kala AI general knowledge, personality, and conversational ability.
//! Tests that Kala can discuss sports, entertainment, science, philosophy,
//! history, fun facts, and more — like a real conversational AI.

use killer_native::builtin::BuiltinFunctions;

fn ask(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("ask", q, "casual", "killer")
}

fn assert_knows(q: &str, must_contain: &[&str]) {
    let r = ask(q);
    assert!(r.len() > 50, "Too short for '{}': {} chars\n{}", q, r.len(), &r[..r.len().min(200)]);
    for word in must_contain {
        assert!(
            r.to_lowercase().contains(&word.to_lowercase()),
            "Missing '{}' in answer for: '{}'\nGot: {}", word, q, &r[..r.len().min(400)]
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SPORTS
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_cricket()    { assert_knows("what is cricket", &["bat", "ball", "11"]); }
#[test] fn knows_football()   { assert_knows("what is football", &["goal", "FIFA"]); }
#[test] fn knows_basketball() { assert_knows("what is basketball", &["NBA", "hoop"]); }
#[test] fn knows_tennis()     { assert_knows("what is tennis", &["Grand Slam", "Wimbledon"]); }
#[test] fn knows_f1()         { assert_knows("what is formula 1", &["racing", "Grand Prix"]); }
#[test] fn knows_olympics()   { assert_knows("what are the olympic games", &["sport", "athletes"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// ENTERTAINMENT & MOVIES
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_marvel()    { assert_knows("what is the marvel cinematic universe", &["MCU", "Iron Man"]); }
#[test] fn knows_anime()     { assert_knows("what is anime", &["Japanese", "Naruto"]); }
#[test] fn knows_bollywood() { assert_knows("what is bollywood", &["Hindi", "Mumbai"]); }
#[test] fn knows_netflix()   { assert_knows("what is netflix", &["streaming", "subscribers"]); }
#[test] fn knows_best_movies() { assert_knows("best movies of all time", &["Shawshank", "Godfather"]); }
#[test] fn knows_music()     { assert_knows("explain the music industry and music genres", &["Jazz", "Rock", "Pop"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// SCIENCE
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_photosynthesis() { assert_knows("what is photosynthesis", &["light", "CO", "glucose"]); }
#[test] fn knows_dna()       { assert_knows("what is DNA", &["genetic", "double helix"]); }
#[test] fn knows_evolution()  { assert_knows("what is evolution", &["Darwin", "natural selection"]); }
#[test] fn knows_gravity()    { assert_knows("what is gravity", &["Newton", "force"]); }
#[test] fn knows_quantum()    { assert_knows("what is quantum physics", &["superposition", "wave"]); }
#[test] fn knows_relativity() { assert_knows("what is the theory of relativity by einstein", &["relativity", "spacetime"]); }
#[test] fn knows_cell()       { assert_knows("what is a cell in biology", &["nucleus", "mitochondria"]); }
#[test] fn knows_periodic_table() { assert_knows("what is the periodic table", &["element", "Mendeleev"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// SPACE
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_black_holes() { assert_knows("what is a black hole", &["gravity", "light"]); }
#[test] fn knows_mars()       { assert_knows("tell me about mars", &["Red Planet", "SpaceX"]); }
#[test] fn knows_solar_system() { assert_knows("what is the solar system", &["planet", "Sun"]); }
#[test] fn knows_nasa()       { assert_knows("what is NASA", &["space", "Apollo"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// HISTORY
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_ww2()        { assert_knows("what is world war 2", &["1939", "Nazi"]); }
#[test] fn knows_ww1()        { assert_knows("what is world war 1", &["1914", "trench"]); }
#[test] fn knows_egypt()      { assert_knows("tell me about ancient egypt", &["pyramid", "pharaoh"]); }
#[test] fn knows_renaissance() { assert_knows("what is the renaissance", &["Leonardo", "Michelangelo"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// PHILOSOPHY
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_philosophy()    { assert_knows("what is philosophy", &["wisdom", "Socrates"]); }
#[test] fn knows_stoicism()      { assert_knows("what is stoicism", &["Marcus Aurelius", "control"]); }
#[test] fn knows_existentialism() { assert_knows("what is existentialism", &["Sartre", "freedom"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// ECONOMICS & BUSINESS
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_inflation()  { assert_knows("what is inflation", &["price", "purchasing power"]); }
#[test] fn knows_crypto()     { assert_knows("what is cryptocurrency", &["Bitcoin", "blockchain"]); }
#[test] fn knows_stock_market() { assert_knows("what is the stock market", &["NYSE", "shares"]); }
#[test] fn knows_startup()    { assert_knows("what is a startup and how to build one", &["MVP", "funding"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// PEOPLE
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_elon_musk()    { assert_knows("who is elon musk", &["Tesla", "SpaceX"]); }
#[test] fn knows_steve_jobs()   { assert_knows("who is steve jobs", &["Apple", "iPhone"]); }
#[test] fn knows_einstein()     { assert_knows("who is albert einstein", &["relativity", "Nobel"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// FUN & PERSONALITY
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn tells_jokes() {
    let r = ask("tell me a joke");
    assert!(r.contains("joke") || r.contains("bug") || r.contains("SQL") || r.contains("binary"),
        "Should tell jokes: {}", &r[..r.len().min(300)]);
}

#[test] fn gives_fun_facts() {
    let r = ask("tell me a fun fact");
    assert!(r.len() > 100, "Fun facts should be detailed: {}", r.len());
    assert!(r.contains("fact") || r.contains("octop") || r.contains("honey") || r.contains("Venus"),
        "Should have fun facts: {}", &r[..r.len().min(300)]);
}

#[test] fn gives_motivation() {
    let r = ask("give me some motivational quotes to inspire me");
    assert!(r.len() > 100 && (r.contains("Steve Jobs") || r.contains("Edison") || r.contains("Darwin")
        || r.contains("Einstein") || r.contains("motivat") || r.contains("inspir")),
        "Should contain motivational content: {}", &r[..r.len().min(300)]);
}

#[test] fn recommends_books() {
    let r = ask("what are the best must read books to recommend");
    assert!(r.len() > 100 && (r.contains("Atomic Habits") || r.contains("Sapiens") || r.contains("1984")
        || r.contains("Alchemist") || r.contains("book")),
        "Should recommend books: {}", &r[..r.len().min(300)]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// OPINION / DISCUSSION TOPICS
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn discusses_best_language() {
    let r = ask("what is the best programming language to learn");
    assert!(r.contains("Python") || r.contains("JavaScript"),
        "Should discuss language recommendations: {}", &r[..r.len().min(300)]);
}

#[test] fn discusses_meaning_of_life() {
    let r = ask("what is the meaning of life");
    assert!(r.len() > 100, "Should give thoughtful answer on meaning of life");
    assert!(r.contains("meaning") || r.contains("purpose") || r.contains("42"),
        "Should address the question: {}", &r[..r.len().min(300)]);
}

#[test] fn knows_climate_change() {
    let r = ask("what is climate change");
    assert!(r.contains("greenhouse") || r.contains("CO") || r.contains("temperature"),
        "Should know about climate change: {}", &r[..r.len().min(300)]);
}

#[test] fn knows_agi() {
    let r = ask("what is artificial general intelligence");
    assert!(r.contains("AGI") || r.contains("human level") || r.contains("general"),
        "Should know about AGI: {}", &r[..r.len().min(300)]);
}

#[test] fn knows_meditation() {
    let r = ask("what is meditation");
    assert!(r.contains("mindful") || r.contains("breath") || r.contains("focus"),
        "Should know about meditation: {}", &r[..r.len().min(300)]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// FOOD & TRAVEL
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_sushi()  { assert_knows("what is sushi", &["Japanese", "rice"]); }
#[test] fn knows_coffee() { assert_knows("what is coffee", &["caffeine", "bean"]); }
#[test] fn knows_pizza()  { assert_knows("what is pizza", &["Naples", "Italy"]); }
#[test] fn knows_japan()  { assert_knows("tell me about japan", &["Tokyo", "anime"]); }
#[test] fn knows_india()  { assert_knows("tell me about india", &["Delhi", "Hindi"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// PSYCHOLOGY
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn knows_psychology() { assert_knows("what is psychology", &["mind", "behavior"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// SMART FALLBACK — doesn't just say "I don't know"
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn smart_fallback_health() {
    let r = ask("how to improve my health and fitness");
    assert!(r.len() > 100, "Health topic should get a thoughtful response");
    assert!(r.to_lowercase().contains("exercise") || r.to_lowercase().contains("sleep") || r.to_lowercase().contains("health"),
        "Should discuss health: {}", &r[..r.len().min(300)]);
}

#[test] fn smart_fallback_career() {
    let r = ask("career advice for beginners");
    assert!(r.len() > 100, "Career topic should get useful response");
}

#[test] fn smart_fallback_learning() {
    let r = ask("how to study effectively and improve productivity");
    assert!(r.len() > 100, "Learning topic should get useful response");
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXISTING KNOWLEDGE STILL WORKS
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn still_knows_python()     { assert_knows("what is python", &["Python", "Guido"]); }
#[test] fn still_knows_rust()       { assert_knows("what is rust language", &["Rust", "memory safety"]); }
#[test] fn still_knows_ai()         { assert_knows("what is artificial intelligence", &["AI", "Machine Learning"]); }
#[test] fn still_knows_blockchain() { assert_knows("what is blockchain", &["distributed"]); }
