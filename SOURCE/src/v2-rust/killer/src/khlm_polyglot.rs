// KhLM-Polyglot - 5-Tier AI Intelligence Router for Nova Galaxy Engine
//
// Upgrades KhLM with LLM + RLM tiers specifically for polyglot code operations.
//
// KhLM-Polyglot - 5-Tier Routing
//
//   Tier 0  CAG  - Static pattern index   < 1ms       $0.000 (memory)
//   Tier 1  KhLM - Deterministic          < 1ms       $0.000
//   Tier 2  LLM  - Ollama/Groq/OpenAI    200-800ms   $0-$0.60/1M
//   Tier 3  RLM  - DeepSeek-R1/QwQ       500-2000ms  $0 (local)
//   Tier 4  Ghost-108 web parallel        2-8s        $0.000
//
// Routing:
//   query -> Tier 0 CAG match?              yes -> instant (<1ms)
//         -> Tier 1 deterministic match?    yes -> instant
//         -> LLM available?                yes -> Tier 2 LLM call
//         -> RLM model loaded?             yes -> Tier 3 RLM reasoning
//         -> Tier 4 Ghost-108 fallback
//
// Killer builtins exposed:
//   khlm_debug(code, lang)             - auto-debug with AI
//   khlm_suggest(code, lang)           - optimization hints
//   khlm_translate(code, from, to)     - code translation between languages
//   khlm_explain(code, lang)           - explain what code does
//   khlm_fix(code, error, lang)        - fix code given an error message
//   khlm_status()                      - show which tiers are available
//   khlm_set_llm(provider, key, model) - configure LLM for Tier 2
//   khlm_set_rlm(model_path)           - configure RLM model for Tier 3

use std::collections::HashMap;
use std::sync::{OnceLock, Mutex};
use std::time::Instant;
use crate::value::Value;
use crate::error::VmError;

// Global Config Singleton

static CONFIG: OnceLock<Mutex<KhLmPolyglotConfig>> = OnceLock::new();

pub fn config() -> &'static Mutex<KhLmPolyglotConfig> {
    CONFIG.get_or_init(|| Mutex::new(KhLmPolyglotConfig::new()))
}

// ── Conversation context (set by kala_ui per request, used by khlm_ask_expert) ──
static CONV_CONTEXT: OnceLock<Mutex<String>> = OnceLock::new();
static CONV_HISTORY: OnceLock<Mutex<Vec<(String,String)>>> = OnceLock::new();
static CONV_UNAME:   OnceLock<Mutex<String>> = OnceLock::new();

fn conv_context_store() -> &'static Mutex<String> {
    CONV_CONTEXT.get_or_init(|| Mutex::new(String::new()))
}
fn conv_history_store() -> &'static Mutex<Vec<(String,String)>> {
    CONV_HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}
fn conv_uname_store() -> &'static Mutex<String> {
    CONV_UNAME.get_or_init(|| Mutex::new(String::new()))
}

/// Set conversation history context for the current request.
/// Called by builtin.rs before every kala_dispatch_with_memory.
pub fn set_conversation_context(ctx: String) {
    if let Ok(mut lock) = conv_context_store().lock() {
        *lock = ctx;
    }
}

/// Store the full structured history and username for multi-turn LLM calls.
pub fn set_conversation_history(history: Vec<(String,String)>, uname: String) {
    if let Ok(mut lock) = conv_history_store().lock() { *lock = history; }
    if let Ok(mut lock) = conv_uname_store().lock()   { *lock = uname;   }
}

/// Clear in-process conversation buffers (e.g. new chat in UI). Next request should pass fresh `history`.
pub fn clear_conversation_session() {
    if let Ok(mut lock) = conv_history_store().lock() { lock.clear(); }
    if let Ok(mut lock) = conv_context_store().lock() { lock.clear(); }
    if let Ok(mut lock) = conv_uname_store().lock()   { lock.clear(); }
}

/// Max prior messages (user+assistant pairs count as 2) included in LLM `build_messages`.
const MAX_HISTORY_MESSAGES: usize = 64;
/// Per-turn cap so huge code dumps do not blow the context window (UTF-8 safe).
const MAX_HISTORY_CONTENT_CHARS: usize = 3500;

pub(crate) fn truncate_history_content(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let trimmed: String = s.chars().take(keep).collect();
    format!("{}…", trimmed)
}

#[allow(dead_code)]
fn get_conversation_context() -> String {
    conv_context_store().lock().map(|l| l.clone()).unwrap_or_default()
}

fn get_conversation_history() -> Vec<(String,String)> {
    conv_history_store().lock().map(|l| l.clone()).unwrap_or_default()
}

fn get_uname() -> String {
    conv_uname_store().lock().map(|l| l.clone()).unwrap_or_default()
}

/// Public accessors for conversational intelligence (used by builtin.rs Tier 0.5)
pub fn get_conversation_history_pub() -> Vec<(String,String)> {
    get_conversation_history()
}

pub fn get_uname_pub() -> String {
    get_uname()
}

/// Build a full multi-turn message array: system + history + current question.
/// History is passed as proper user/assistant turns (not a text blob).
fn build_messages(system: &str, question: &str) -> Vec<crate::llm::LlmMessage> {
    let history = get_conversation_history();
    let uname   = get_uname();

    // Inject user name into system prompt if known
    let sys = if uname.is_empty() {
        system.to_string()
    } else {
        format!("{}\n\nThe user's name is **{}**. Address them by name naturally.", system, uname)
    };

    let mut msgs = vec![crate::llm::LlmMessage { role: "system".into(), content: sys }];

    // Include a long tail of the session so follow-ups ("expand that", "fix the bug") work.
    let start = history.len().saturating_sub(MAX_HISTORY_MESSAGES);
    for (role, content) in &history[start..] {
        let r = if role == "user" { "user" } else { "assistant" };
        let c = truncate_history_content(content, MAX_HISTORY_CONTENT_CHARS);
        msgs.push(crate::llm::LlmMessage { role: r.into(), content: c });
    }

    msgs.push(crate::llm::LlmMessage { role: "user".into(), content: question.to_string() });
    msgs
}

#[derive(Debug, Clone)]
pub struct KhLmPolyglotConfig {
    /// LLM provider for Tier 2: "ollama", "groq", "openai", "anthropic", ""
    pub llm_provider: String,
    /// API key (blank for Ollama)
    pub llm_api_key: String,
    /// Model identifier
    pub llm_model: String,
    /// Local RLM/GGUF model path for Tier 3
    pub rlm_model: String,
    /// Max tokens for LLM response
    pub max_tokens: usize,
    /// Sampling temperature for Tier-2 LLM calls (0.0–2.0). Env: `KILLER_KHLM_LLM_TEMPERATURE`.
    pub llm_temperature: f64,
    /// Whether to use CAG tier 0
    pub cag_enabled: bool,
    /// Result cache: (operation+lang+code_hash) -> answer
    cache: HashMap<u64, CachedAnswer>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CachedAnswer {
    text:  String,
    tier:  u8,
    ms:    u128,
}

fn default_model_for(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "ollama"    => "llama3".into(),
        "groq"      => "llama3-70b-8192".into(),
        "openai"    => "gpt-4o-mini".into(),
        "anthropic" => "claude-opus-4-5".into(),
        _           => String::new(),
    }
}

impl KhLmPolyglotConfig {
    /// Defaults + optional env (advanced setup — no code changes needed):
    /// - `KILLER_KHLM_LLM_PROVIDER` — `ollama` | `groq` | `openai` | `anthropic`
    /// - `KILLER_KHLM_LLM_API_KEY` — API key (empty for Ollama)
    /// - `KILLER_KHLM_LLM_MODEL` — model id (defaults per provider if omitted)
    /// - `KILLER_KHLM_LLM_MAX_TOKENS` — optional usize, default 512
    /// - `KILLER_KHLM_LLM_TEMPERATURE` — optional f64 in 0.0…2.0, default 0.7
    /// - `KILLER_KHLM_RLM` — GGUF path or registry name (Tier-3 polyglot); resolved via `resolve_model_path`
    /// - `KILLER_KHLM_GGUF` — if `KILLER_KHLM_RLM` unset, same resolution fills Tier-3 RLM path
    pub fn new() -> Self {
        let mut c = KhLmPolyglotConfig {
            llm_provider: String::new(),
            llm_api_key:  String::new(),
            llm_model:    String::new(),
            rlm_model:    String::new(),
            max_tokens:   512,
            llm_temperature: 0.7,
            cag_enabled:  true,
            cache:        HashMap::new(),
        };
        if let Ok(p) = std::env::var("KILLER_KHLM_LLM_PROVIDER") {
            let p = p.trim().to_lowercase();
            if matches!(p.as_str(), "ollama" | "groq" | "openai" | "anthropic") {
                c.llm_provider = p;
            }
        }
        if let Ok(k) = std::env::var("KILLER_KHLM_LLM_API_KEY") {
            c.llm_api_key = k;
        }
        if let Ok(m) = std::env::var("KILLER_KHLM_LLM_MODEL") {
            let m = m.trim();
            if !m.is_empty() {
                c.llm_model = m.to_string();
            }
        }
        if !c.llm_provider.is_empty() && c.llm_model.is_empty() {
            c.llm_model = default_model_for(&c.llm_provider);
        }
        if let Ok(mt) = std::env::var("KILLER_KHLM_LLM_MAX_TOKENS") {
            if let Ok(n) = mt.trim().parse::<usize>() {
                if n >= 64 && n <= 32000 {
                    c.max_tokens = n;
                }
            }
        }
        if let Ok(tmp) = std::env::var("KILLER_KHLM_LLM_TEMPERATURE") {
            if let Ok(x) = tmp.trim().parse::<f64>() {
                if (0.0..=2.0).contains(&x) {
                    c.llm_temperature = x;
                }
            }
        }
        if let Ok(r) = std::env::var("KILLER_KHLM_RLM") {
            let r = r.trim();
            if !r.is_empty() {
                c.rlm_model = crate::inference::resolve_model_path(r)
                    .unwrap_or_else(|_| r.to_string());
            }
        } else if let Ok(r) = std::env::var("KILLER_KHLM_GGUF") {
            let r = r.trim();
            if !r.is_empty() {
                if let Ok(p) = crate::inference::resolve_model_path(r) {
                    c.rlm_model = p;
                }
            }
        }
        c
    }

    pub fn llm_available(&self) -> bool {
        !self.llm_provider.is_empty()
    }

    pub fn rlm_available(&self) -> bool {
        !self.rlm_model.is_empty()
    }
}

// CAG: Static Code-Pattern Knowledge Base
//
// Pre-indexed answers for the most common polyglot patterns.
// Structured as (lang, error_keyword) -> explanation + fix.
// Zero network, zero tokens, sub-millisecond.

struct CagEntry {
    lang:    &'static str,  // "" = all languages
    keyword: &'static str,
    answer:  &'static str,
}

static CAG_KB: &[CagEntry] = &[
    // Go
    CagEntry { lang: "go", keyword: "undefined:", answer: "Go: Undefined identifier. Check spelling and imports. If it's a package symbol, add the import at the top: `import \"package/path\"`." },
    CagEntry { lang: "go", keyword: "cannot use", answer: "Go: Type mismatch. Go is strictly typed. Use explicit conversion: `int64(x)`, `float64(x)`, or `string(x)`." },
    CagEntry { lang: "go", keyword: "declared and not used", answer: "Go: Unused variable - compilation error in Go. Remove the variable or use `_ = varName` to suppress." },
    CagEntry { lang: "go", keyword: "no required module", answer: "Go: Missing module. Run `go mod init` in your directory, then `go mod tidy` to fetch dependencies." },
    CagEntry { lang: "go", keyword: "multiple-value", answer: "Go: Function returned multiple values but only one was captured. Use: `result, err := fn()`." },
    CagEntry { lang: "go", keyword: "deadlock", answer: "Go: All goroutines are asleep - deadlock. Check: unbuffered channel with no receiver, or mutex locked twice without unlock." },
    CagEntry { lang: "go", keyword: "invalid memory address", answer: "Go: Nil pointer dereference. Check if pointer is nil before use: `if ptr != nil { ... }`." },

    // Python
    CagEntry { lang: "python", keyword: "nameerror", answer: "Python NameError: Variable used before assignment. Check spelling and scope. Use `global x` for module-level variables inside functions." },
    CagEntry { lang: "python", keyword: "typeerror", answer: "Python TypeError: Type mismatch. Use `str()`, `int()`, `float()` for conversion. Check function signatures match argument types." },
    CagEntry { lang: "python", keyword: "indexerror", answer: "Python IndexError: Index out of range. Check `len(list)` before indexing. Use `list[-1]` for last element." },
    CagEntry { lang: "python", keyword: "keyerror", answer: "Python KeyError: Dictionary key not found. Use `dict.get(key, default)` or check `if key in dict`." },
    CagEntry { lang: "python", keyword: "attributeerror", answer: "Python AttributeError: Object has no such attribute. Check the type with `type(obj)` and inspect with `dir(obj)`." },
    CagEntry { lang: "python", keyword: "syntaxerror", answer: "Python SyntaxError: Invalid syntax. Common causes: missing `:` after if/for/def, unmatched brackets, wrong indentation." },
    CagEntry { lang: "python", keyword: "indentationerror", answer: "Python IndentationError: Use consistent 4-space indentation. Do not mix tabs and spaces." },
    CagEntry { lang: "python", keyword: "zerodivisionerror", answer: "Python ZeroDivisionError: Division by zero. Guard with: `if divisor != 0: result = a / divisor`." },
    CagEntry { lang: "python", keyword: "importerror", answer: "Python ImportError: Module not found. Install with `pip install <module>` or check the spelling." },
    CagEntry { lang: "python", keyword: "modulenotfounderror", answer: "Python ModuleNotFoundError: Package not installed. Run: `pip install <package>` or check virtual environment." },
    CagEntry { lang: "python", keyword: "recursionerror", answer: "Python RecursionError: Stack overflow. Add a base case to your recursion or increase `sys.setrecursionlimit(N)`." },

    // PowerShell
    CagEntry { lang: "powershell", keyword: "is not recognized", answer: "PowerShell: Command not found. Check spelling. Use `Get-Command <name>` to locate it or install the module with `Install-Module`." },
    CagEntry { lang: "ps",         keyword: "is not recognized", answer: "PowerShell: Command not found. Check spelling. Use `Get-Command <name>` to locate it or install the module with `Install-Module`." },
    CagEntry { lang: "powershell", keyword: "cannot find path", answer: "PowerShell: Path not found. Use `Test-Path` to verify. Quote paths with spaces: `\"C:\\Path With Spaces\"`." },
    CagEntry { lang: "powershell", keyword: "access is denied", answer: "PowerShell: Access denied. Run as Administrator or check file permissions with `Get-Acl`." },
    CagEntry { lang: "powershell", keyword: "unauthorizedaccessexception", answer: "PowerShell: UnauthorizedAccessException. Run PowerShell as Administrator or use `Set-ExecutionPolicy RemoteSigned`." },

    // Rust
    CagEntry { lang: "rust", keyword: "cannot borrow", answer: "Rust: Borrow checker error. You cannot borrow a value as mutable while an immutable borrow exists. Clone the value or restructure to limit borrow lifetimes." },
    CagEntry { lang: "rust", keyword: "value moved here", answer: "Rust: Value was moved. After moving ownership, the original is invalid. Use `.clone()` or borrow with `&` instead." },
    CagEntry { lang: "rust", keyword: "does not live long enough", answer: "Rust: Lifetime error. A reference outlives the value it points to. Return owned values or use `'static` lifetime for static data." },
    CagEntry { lang: "rust", keyword: "mismatched types", answer: "Rust: Type mismatch. Use `as` for primitive casts: `x as f64`. For complex types use `From`/`Into` traits." },
    CagEntry { lang: "rust", keyword: "unwrap() called on `none`", answer: "Rust: Option was None. Use `if let Some(val) = opt { ... }` or `.unwrap_or(default)` instead of `.unwrap()`." },
    CagEntry { lang: "rust", keyword: "called `result::unwrap()`", answer: "Rust: Result was Err. Use `if let Ok(val) = result { ... }` or `match result { Ok(v) => ..., Err(e) => ... }`." },

    // Node.js / JavaScript
    CagEntry { lang: "node",       keyword: "cannot read properties of undefined", answer: "JS: Undefined property access. Check if the object is null/undefined before accessing: `if (obj?.prop) { ... }` (optional chaining)." },
    CagEntry { lang: "js",         keyword: "cannot read properties of undefined", answer: "JS: Undefined property access. Check if the object is null/undefined before accessing: `if (obj?.prop) { ... }` (optional chaining)." },
    CagEntry { lang: "node",       keyword: "typeerror", answer: "JS TypeError: Wrong type. Use `typeof x` or `Array.isArray(x)` to check types before operations." },
    CagEntry { lang: "node",       keyword: "syntaxerror", answer: "JS SyntaxError: Invalid syntax. Common: missing `;`, unmatched `{`, wrong `=>` arrow function syntax." },
    CagEntry { lang: "node",       keyword: "cannot find module", answer: "Node: Module not found. Run `npm install <module>` or check the relative path: `./module` not `module`." },

    // Generic (all languages)
    CagEntry { lang: "", keyword: "stack overflow",   answer: "Stack overflow: Infinite recursion detected. Ensure your recursive function has a base case that terminates." },
    CagEntry { lang: "", keyword: "out of memory",    answer: "Out of memory: Process exhausted available RAM. Reduce data size, use streaming/chunks, or increase system memory." },
    CagEntry { lang: "", keyword: "segmentation fault", answer: "Segmentation fault: Invalid memory access. Common causes: null pointer dereference, buffer overflow, or use-after-free." },
    CagEntry { lang: "", keyword: "connection refused", answer: "Connection refused: Target server is not running or port is wrong. Verify service is running and check firewall settings." },
    CagEntry { lang: "", keyword: "permission denied", answer: "Permission denied: Insufficient file/directory permissions. Check with `ls -la` (Linux/Mac) or file Properties (Windows)." },
    CagEntry { lang: "", keyword: "timeout",          answer: "Timeout: Operation exceeded time limit. Increase timeout, optimize the operation, or check network connectivity." },
];

/// Search CAG knowledge base. Returns (answer, ms) or None.
fn cag_lookup(query: &str, lang: &str) -> Option<String> {
    let q_lower = query.to_lowercase();
    let lang_lower = lang.to_lowercase();

    // Score each entry: lang-specific beats generic, more keyword matches = higher score
    let mut best: Option<(&str, u8)> = None; // (answer, score)
    for entry in CAG_KB {
        if !entry.lang.is_empty() && entry.lang != lang_lower { continue; }
        if q_lower.contains(entry.keyword) {
            let score: u8 = if entry.lang == lang_lower { 10 } else { 5 };
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((entry.answer, score));
            }
        }
    }
    best.map(|(a, _)| a.to_string())
}

// FNV hash

fn quick_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

// Tier 2: LLM call (Ollama / Groq / OpenAI / Anthropic)

fn llm_call(prompt: &str, cfg: &KhLmPolyglotConfig) -> Option<String> {
    if cfg.llm_provider.is_empty() { return None; }

    let mut llm_cfg = match cfg.llm_provider.to_lowercase().as_str() {
        "ollama" => crate::llm::LlmConfig::ollama(&cfg.llm_model),
        "groq"   => crate::llm::LlmConfig::groq(&cfg.llm_api_key, &cfg.llm_model),
        "openai" => crate::llm::LlmConfig::openai(&cfg.llm_api_key, &cfg.llm_model),
        "anthropic" => crate::llm::LlmConfig::anthropic(&cfg.llm_api_key, &cfg.llm_model),
        _ => return None,
    };
    llm_cfg.temperature = cfg.llm_temperature;

    let messages = vec![
        crate::llm::LlmMessage { role: "system".into(), content: SYSTEM_PROMPT.into() },
        crate::llm::LlmMessage { role: "user".into(),   content: prompt.to_string()  },
    ];

    match crate::llm::complete(&llm_cfg, &messages) {
        Ok(r) if !r.content.trim().is_empty() => Some(r.content),
        _ => None,
    }
}

// Tier 3: RLM deep reasoning

fn rlm_call(prompt: &str, model_path: &str) -> Option<String> {
    let use_rlm = crate::llm::is_rlm_model_path(model_path);

    let result = if use_rlm {
        crate::inference::killer_think_rlm(model_path, prompt, 1200)
            .map(|r| r.answer_only().to_string())
            .ok()
    } else {
        crate::inference::killer_llm_as_rlm(model_path, prompt, 800)
            .map(|r| r.answer_only().to_string())
            .ok()
    };

    result.filter(|r| r.len() > 20)
}

// System Prompt (code operations)

const SYSTEM_PROMPT: &str = "\
You are the AI engine for Nova Galaxy Engine v1 - a polyglot code execution \
system built inside the Killer programming language. You specialize in:\n\
1. Debugging code errors across languages (Python, Go, Rust, PowerShell, Node.js, \
   Java, Bash, Ruby, Lua, Perl, PHP, R)\n\
2. Suggesting optimizations and improvements\n\
3. Translating code between languages\n\
4. Explaining what code does concisely\n\
Be concise, show corrected code when fixing bugs. Format responses clearly.";

// ── Kala Expert System Prompts ───────────────────────────────────────────────

/// Expert assistant system prompt injected on every Kala → LLM call.
pub const KALA_EXPERT_SYSTEM_PROMPT: &str = "\
You are **Kala** (काल), an expert-level AI assistant built into the Killer programming language. \
Your name means 'Time' and 'Fate' in Sanskrit.\n\n\
## Core Principles — ALWAYS follow these\n\
1. **Accuracy first**: Every factual claim must be correct. If uncertain, say so explicitly.\n\
2. **Intent match**: Read the user's exact words — different users phrase things differently; answer *their* request, not a generic nearby topic. If the question is ambiguous, state your interpretation in one line, then answer.\n\
3. **Depth over brevity**: Give thorough, substantive answers. Never give shallow one-liners for complex topics.\n\
4. **Structured thinking**: Decompose complex questions into clear components before answering.\n\
5. **Cite reasoning**: Show *why* something is true, not just *what* is true.\n\
6. **Zero hallucination**: Never fabricate names, dates, numbers, or citations. Say 'I don't have exact data on this' rather than guessing.\n\n\
## Personality\n\
- Warm, direct, and intellectually engaged — like a sharp friend who is **honest about limits**, not omniscient\n\
- Use the user's name naturally in conversation\n\
- Light appropriate humor; never forced or excessive\n\
- Ask a sharp follow-up question when it would genuinely deepen the conversation\n\n\
## Conversation Memory\n\
- The full conversation history is provided as prior turns — USE IT for every reply\n\
- **Session continuity**: Prior turns are ground truth for *this* chat — reuse names, facts, and code the user already gave; resolve \"it\" / \"that\" / short follow-ups from history\n\
- Reference and build on what was said before; do not repeat large blocks verbatim unless asked\n\
- Adapt depth and style to what the user has shown they want\n\n\
## Attribution / meta\n\
- Do **not** name the Killer/Kala project's author or \"who built you\" unless the user clearly asks who created, owns, or built Kala/Killer/you.\n\
- Answer the user's actual question first; keep meta details out of unrelated topics.\n\n\
## Response Format\n\
- Lead with the direct answer in the first sentence\n\
- Use **## headings** for sub-topics in longer answers\n\
- **Bold** key terms and critical facts\n\
- `code fences` for all code/commands\n\
- Bullet lists for comparisons and steps\n\
- Close with either a follow-up question OR a clear 'next step' suggestion\n\n\
## When You Don't Know\n\
Say: *'I don't have reliable data on that — here's what I do know: ...'* \
Never speculate as if it were fact.\n\n\
## High-stakes topics (health, law, money, safety, live data)\n\
- Do **not** present guesses as professional advice. Say you may be wrong; suggest verifying with an authoritative source or a qualified human.\n\
- For prices, laws, medical issues, or \"what happened today\" — flag uncertainty and that your knowledge may be outdated.";

/// Think-mode system prompt — deep reasoning, step-by-step analysis.
const KALA_THINK_SYSTEM_PROMPT: &str = "\
You are **Kala** (काल), a deep reasoning AI built into the Killer programming language.\n\n\
## Reasoning Protocol\n\
1. **Identify the real question** — what is actually being asked? Restate it sharply.\n\
2. **Expose assumptions** — list every hidden assumption in the question.\n\
3. **Decompose** — break into the smallest logical sub-problems.\n\
4. **Reason from first principles** — use evidence, logic, and analogies for each part.\n\
5. **Consider counterarguments** — what would a smart opponent say? Address it.\n\
6. **Synthesise** — build a well-justified final answer that integrates everything.\n\
7. **Assess confidence** — state explicitly what you're certain about vs uncertain about.\n\
8. **Session memory** — integrate facts and questions from earlier turns in this chat when reasoning.\n\n\
## Non-Negotiable Rules\n\
- NEVER assert a fact you can't justify with reasoning\n\
- If the question has no definitive answer, explain WHY and map the space of possibilities\n\
- Show your work — reasoning is more valuable than the conclusion alone\n\
- Use LaTeX for math: $inline$ and $$block$$\n\n\
## Format\n\
## Core Question\n\
## Hidden Assumptions\n\
## Step-by-Step Reasoning\n\
## Counterarguments\n\
## Synthesis & Answer\n\
## Confidence Level\n\n\
Be rigorous, precise, and intellectually honest.";

/// Call the configured LLM with the Kala expert system prompt.
/// Falls back to standard `khlm_ask` if no LLM is configured.
pub fn khlm_ask_expert(question: &str) -> String {
    let cfg_opt = {
        let lock = config().lock().unwrap();
        if lock.llm_available() { Some(lock.clone()) } else { None }
    };
    if let Some(cfg) = cfg_opt {
        // Use proper multi-turn messages with full conversation history
        let messages = build_messages(KALA_EXPERT_SYSTEM_PROMPT, question);
        if let Some(out) = llm_call_messages(&messages, &cfg) {
            return out;
        }
    }
    // Offline: Tier 1 deterministic + Ghost-108 web
    crate::llm::khlm_ask(question)
}

const KALA_CODE_SYSTEM_PROMPT: &str = "\
You are Kala (काल), an expert coding assistant built into the Killer programming language.\n\
\n\
## Mission\n\
Ship code that **directly answers** what the user asked — not a generic tutorial unless they asked to learn. One strong, runnable solution beats several shallow variants.\n\
\n\
## Copilot-Style Code Generation Rules\n\
1. Write complete, working, runnable code — no pseudocode, no TODOs.\n\
2. Use the language the user specified; default to Python if unspecified.\n\
3. Include clear, concise comments explaining non-obvious logic.\n\
4. Wrap ALL code in a markdown code fence with the correct language tag.\n\
5. After the code block, add ONE-LINE description of what it does.\n\
6. If the user asks for a function: write the function PLUS a usage example.\n\
7. If the user asks for a class: include `__init__`, key methods, and a usage example.\n\
8. If the user asks for tests: write real pytest/unittest test cases that actually pass.\n\
9. Prefer idiomatic patterns: list comprehensions in Python, iterators in Rust, etc.\n\
10. For algorithms: include time/space complexity comment at the top.\n\
\n\
## Session memory\n\
Prior turns in this chat are authoritative — extend, refactor, or fix code the user already showed; do not ignore their earlier language choice or constraints.\n\
\n\
## Correctness Protocol (must follow before you answer)\n\
- **No invented APIs**: use only real standard library / widely documented crates or packages; if unsure, say so and give a minimal alternative.\n\
- **Trace mentally**: walk through inputs (empty, single element, max size) and error paths; fix off-by-one and null cases you find.\n\
- **Match versions**: avoid Python 3.10+-only syntax if the user asked for older compatibility; state assumptions if you need a version.\n\
- **Security**: no string-concat SQL; use parameters/bindings; avoid `eval` on user text; validate external input.\n\
- **Non-trivial logic**: add 2–4 focused asserts OR a tiny test example showing expected output for a normal case.\n\
- If you cannot meet the spec with confidence, say what is uncertain and give the safest partial solution.\n\
- **Single-snippet asks**: one code fence is enough. When the user asks for a **project / app / scaffold / full stack**, the system adds **Project builder** rules — follow those instead of collapsing everything into one file.\n\
\n\
Output format (single file):\n\
```language\n\
# code here\n\
```\n\
*One-line description.*";

/// When the user wants repos, apps, or multi-file layouts — injected into the code system prompt.
const KALA_MULTI_FILE_PROJECT_SUPPLEMENT: &str = "\
## Multi-file project builder (PROJECT MODE — follow ALL of this)\n\
Behave like an IDE copilot (Cursor-style): deliver a **coherent, runnable** codebase, not one giant dump unless the stack is truly single-file.\n\
\n\
1. **Overview** — 2–4 bullets: purpose, stack, how to run locally.\n\
2. **Layout** — A fenced `text` block showing an ASCII **directory tree**.\n\
3. **Files** — For each important file:\n\
   - A markdown heading with the exact relative path, e.g. `### src/main.py` or `### package.json`.\n\
   - Then **one** code fence (`python`, `rust`, `typescript`, etc.) with the **complete** file contents.\n\
   - Typical order: dependency manifests → source → configs → tests (if asked).\n\
4. **README.md** — Include content (in its own section + fence) with: prerequisites, install, `run` / `dev` commands, and how to run tests.\n\
5. **Consistency** — Imports/paths between files must match the tree; provide a clear **entrypoint**.\n\
6. **Too large for one reply** — Ship a **minimal vertical slice** that runs end-to-end, then a numbered **Next steps** list for remaining modules — never leave a broken partial tree without saying so.\n\
\n\
If the user named a framework (React, FastAPI, Next.js, …), honour it and use current idioms.";

/// Appended to LLM code replies so users always verify output (models can still err).
const KALA_CODE_POSTSCRIPT: &str = "\n\n---\n*Always run compile/lint/tests on generated code before trusting it in production.*";

/// When templates miss and the expert/web pipeline answers instead (non-test builds).
#[cfg(not(test))]
const KALA_CODE_EXPERT_FALLBACK_NOTE: &str = "\n\n---\n*Produced via Kala expert + research path — review and run locally.*";

fn is_programming_lang(w: &str) -> bool {
    matches!(w, "python"|"rust"|"javascript"|"java"|"typescript"|"go"|"golang"|"c"|"cpp"|
        "csharp"|"ruby"|"php"|"swift"|"kotlin"|"scala"|"dart"|"bash"|"shell"|"sql"|"html"|"css")
}

fn extract_number_from_query(q: &str) -> Option<usize> {
    for word in q.split_whitespace() {
        if let Ok(n) = word.parse::<usize>() {
            if n > 0 && n <= 10000 { return Some(n); }
        }
    }
    None
}

fn extract_name_from_query(q: &str) -> Option<String> {
    let skip = ["write","a","program","with","name","named","it","has","and","the",
        "loop","loops","agent","agents","in","on","for","killer","code","create",
        "build","make","function","class","that","using","example","generate","show",
        "me","my","called","100","200","500","1000","50","10","20","basic","simple"];
    for word in q.split_whitespace() {
        let w = word.to_lowercase();
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        if w.len() >= 2 && w.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
            && !skip.contains(&w) && !is_programming_lang(w)
            && w.parse::<usize>().is_err()
        {
            return Some(w.to_string());
        }
    }
    None
}

fn compose_killer_program(name: &str, loop_count: usize, with_agents: bool) -> String {
    let cap_name = {
        let mut c = name.chars();
        match c.next() {
            Some(f) => f.to_uppercase().to_string() + c.as_str(),
            None => name.to_string(),
        }
    };

    let mut code = String::new();
    code += &format!("# ══════════════════════════════════════════════\n");
    code += &format!("# Program: {} — {} loops{}\n", cap_name, loop_count,
        if with_agents { " + agents" } else { "" });
    code += &format!("# ══════════════════════════════════════════════\n\n");

    if with_agents {
        code += &format!("# Agent definition\n");
        code += &format!("kfn create_agent(id, role) {{\n");
        code += &format!("  agent = {{}}\n");
        code += &format!("  agent[\"id\"] = id\n");
        code += &format!("  agent[\"role\"] = role\n");
        code += &format!("  agent[\"status\"] = \"idle\"\n");
        code += &format!("  agent[\"tasks_done\"] = 0\n");
        code += &format!("  return agent\n");
        code += &format!("}}\n\n");

        code += &format!("kfn agent_work(agent, task_id) {{\n");
        code += &format!("  agent[\"status\"] = \"working\"\n");
        code += &format!("  print(K\"  [Agent {{agent[\\\"id\\\"]}}] ({{agent[\\\"role\\\"]}}) → processing task #{{task_id}}\")\n");
        code += &format!("  agent[\"tasks_done\"] = agent[\"tasks_done\"] + 1\n");
        code += &format!("  agent[\"status\"] = \"done\"\n");
        code += &format!("  return agent\n");
        code += &format!("}}\n\n");

        let roles = ["researcher", "analyzer", "builder", "validator", "optimizer"];
        let agent_count = 5.min(loop_count);
        code += &format!("# Create {} agents\n", agent_count);
        code += &format!("agents = []\n");
        let mut idx = 0;
        while idx < agent_count {
            code += &format!("agents = agents + [create_agent(\"{}-{}\", \"{}\")]\n",
                name, idx + 1, roles[idx % roles.len()]);
            idx += 1;
        }
        code += &format!("\n");

        code += &format!("# Run {} loops — distribute tasks across agents\n", loop_count);
        code += &format!("print(K\"\\n🚀 Starting {{{}}} — {{{}}} loops with {{len(agents)}} agents\\n\")\n", cap_name, loop_count);
        code += &format!("i = 0\n");
        code += &format!("while i < {} {{\n", loop_count);
        code += &format!("  agent_idx = i % len(agents)\n");
        code += &format!("  current = agents[agent_idx]\n");
        code += &format!("  current = agent_work(current, i + 1)\n");
        code += &format!("  agents[agent_idx] = current\n");
        code += &format!("  i = i + 1\n");
        code += &format!("}}\n\n");

        code += &format!("# Results summary\n");
        code += &format!("print(K\"\\n══════ {} Complete ══════\")\n", cap_name);
        code += &format!("j = 0\n");
        code += &format!("while j < len(agents) {{\n");
        code += &format!("  a = agents[j]\n");
        code += &format!("  print(K\"  Agent {{a[\\\"id\\\"]}} ({{a[\\\"role\\\"]}}) completed {{a[\\\"tasks_done\\\"]}} tasks\")\n");
        code += &format!("  j = j + 1\n");
        code += &format!("}}\n");
        code += &format!("print(K\"Total loops executed: {}\")\n", loop_count);
    } else {
        code += &format!("print(K\"🚀 Starting {} — {} loops\")\n\n", cap_name, loop_count);
        code += &format!("results = []\n");
        code += &format!("i = 1\n");
        code += &format!("while i <= {} {{\n", loop_count);
        code += &format!("  result = i * i\n");
        code += &format!("  results = results + [result]\n");
        code += &format!("  print(K\"Loop {{i}}/{}: result = {{result}}\")\n", loop_count);
        code += &format!("  i = i + 1\n");
        code += &format!("}}\n\n");
        code += &format!("print(K\"\\n✅ {} done — {{len(results)}} results collected\")\n", cap_name);
    }

    code
}

#[cfg(not(test))]
fn is_offline_generic_stub(s: &str) -> bool {
    s.contains("Connect an LLM")
        || s.contains("full code generation")
        || s.contains("ready to write your function")
}

/// True when the user is asking for a repo / app / scaffold (multi-file), not a one-off snippet.
pub(crate) fn wants_multi_file_project(question: &str) -> bool {
    let l = question.to_lowercase();
    if l.starts_with("what ")
        || l.starts_with("why ")
        || l.starts_with("who ")
        || l.starts_with("when ")
        || l.starts_with("where ")
        || l.starts_with("explain ")
        || l.starts_with("describe ")
        || l.starts_with("define ")
    {
        return false;
    }
    l.contains("scaffold")
        || l.contains("boilerplate")
        || l.contains("starter kit")
        || l.contains("full stack")
        || l.contains("full-stack")
        || l.contains("monorepo")
        || l.contains("multi-file")
        || l.contains("multi file")
        || l.contains("entire project")
        || l.contains("whole project")
        || l.contains("project structure")
        || l.contains("folder structure")
        || l.contains("directory structure")
        || l.contains("file layout")
        || ((l.contains("create") || l.contains("build") || l.contains("generate") || l.contains("make") || l.starts_with("new "))
            && l.contains(" project"))
}

fn code_generation_system_prompt(question: &str) -> String {
    if wants_multi_file_project(question) {
        format!("{}\n\n{}", KALA_CODE_SYSTEM_PROMPT, KALA_MULTI_FILE_PROJECT_SUPPLEMENT)
    } else {
        KALA_CODE_SYSTEM_PROMPT.to_string()
    }
}

/// Code generation — Copilot-style. Returns code in a markdown block.
/// Online: multi-turn LLM with strict code system prompt.
/// Offline: built-in templates; if none match, **non-test** builds try `khlm_ask_expert` so users still get a real answer when possible.
pub fn khlm_generate_code(question: &str) -> String {
    let q = question.trim();
    if q.is_empty() {
        return "```text\n(Describe what to build — language, inputs, and expected behavior.)\n```\n\n\
                *Example: “Python function to merge two sorted lists”.*"
            .to_string();
    }

    let cfg_opt = {
        let lock = config().lock().unwrap();
        if lock.llm_available() { Some(lock.clone()) } else { None }
    };
    if let Some(cfg) = cfg_opt {
        let sys_prompt = code_generation_system_prompt(q);
        let messages = build_messages(&sys_prompt, q);
        if let Some(out) = llm_call_messages(&messages, &cfg) {
            let t = out.trim();
            if !t.is_empty() {
                return format!("{}{}", out.trim_end(), KALA_CODE_POSTSCRIPT);
            }
        }
    }

    let offline = kala_code_offline(q);
    #[cfg(not(test))]
    {
        if is_offline_generic_stub(&offline) {
            let project_extra = if wants_multi_file_project(q) {
                "\n\nPROJECT / MULTI-FILE: Output a directory tree, then each file as `### path` + fenced code. Include README and manifest(s). MVP must run."
            } else {
                ""
            };
            let wrapped = format!(
                "[CODE GENERATION — priority: working code]\n\
                 Reply with: (1) a markdown code fence using the correct language tag, (2) complete runnable code or the best partial solution, \
                 (3) one line listing any package/crate dependency if needed, (4) one-line summary after the fence.\n\
                 Match the user’s language and libraries. Do not refuse without offering a helpful sketch.{}\n\n\
                 User request:\n{}",
                project_extra, q
            );
            let expert = khlm_ask_expert(&wrapped);
            let el = expert.to_lowercase();
            let bad = expert.trim().len() < 120
                || el.contains("i could not find a reliable")
                || el.contains("no result found")
                || (el.contains("could not find") && el.contains("reliable answer"));
            if !bad {
                return format!("{}{}", expert.trim_end(), KALA_CODE_EXPERT_FALLBACK_NOTE);
            }
        }
    }
    offline
}

/// Offline Copilot — generates real working code for 25+ common patterns.
fn kala_code_offline(question: &str) -> String {
    let q = question.to_lowercase();

    // ── Detect target language ──────────────────────────────────────────────
    let lang = if q.contains("killer") { "killer" }
               else if q.contains("rust") { "rust" }
               else if q.contains("typescript") { "typescript" }
               else if q.contains("javascript") || q.contains(" js ") || q.ends_with(" js")
                    || q.contains("three.js") || q.contains("threejs") || q.contains("webgl") { "javascript" }
               else if q.contains("sql") || q.contains("tsql") || q.contains("t-sql")
                    || q.contains("create table") || q.contains("table for ") || q.contains("table schema") { "sql" }
               else if q.contains("html") { "html" }
               else if q.contains("css") { "css" }
               else if q.contains("kotlin") { "kotlin" }
               else if q.contains("swift") { "swift" }
               else if q.contains("golang") || q.contains(" go ") || q.starts_with("go ") || q.ends_with(" go") || q.ends_with(" golang") { "go" }
               else if q.contains("csharp") || q.contains("c#") || q.contains("c sharp") { "csharp" }
               else if q.contains("ruby") { "ruby" }
               else if q.contains("php") { "php" }
               else if q.contains("scala") { "scala" }
               else if q.contains("dart") { "dart" }
               else if q.contains(" java") || q.starts_with("java ") { "java" }
               else if q.contains("c++") || q.contains("cpp") { "cpp" }
               else if q.contains("c language") || q.contains(" in c") && !q.contains("in css") { "c" }
               else if q.contains("bash") || q.contains("shell") { "bash" }
               else { "python" }; // safe default

    // ═══════════════════════════════════════════════════════════════════════════
    // PROJECT SCAFFOLDING — checked FIRST (before single-file templates)
    // ═══════════════════════════════════════════════════════════════════════════
    if let Some(proj) = kala_project_scaffold(&q) {
        return proj;
    }

    // ── Custom Killer program composer — specific requests with names/counts/agents ──
    if lang == "killer" {
        let has_specific_count = extract_number_from_query(&q).is_some();
        let has_agent = q.contains("agent");
        let has_name = {
            let words: Vec<&str> = q.split_whitespace().collect();
            words.iter().any(|w| {
                let wl = w.to_lowercase();
                wl.len() > 2
                    && wl.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
                    && !["write","program","with","name","has","loop","loops","and","agents",
                         "agent","the","for","code","create","build","make","killer","function",
                         "class","that","this","using","example","basic","simple","it","its",
                         "100","200","500","1000","50","10","20","are","can","generate","show",
                         "how","what","implement","script","project"].contains(&wl.as_str())
                    && !is_programming_lang(&wl)
            })
        };
        let is_specific_request = (has_specific_count && (has_agent || has_name))
            || (has_agent && has_name)
            || (has_specific_count && has_name);

        if is_specific_request {
            let count = extract_number_from_query(&q).unwrap_or(10);
            let name = extract_name_from_query(&q).unwrap_or_else(|| "app".to_string());
            let code = compose_killer_program(&name, count, has_agent);
            return format!(
                "```killer\n{}\n```\n\n*Killer program \"{}\" — {} loops{}.*\n\n---\n\
                💡 **Try next:** *\"Add error handling\"* · *\"Add more agent types\"* · *\"Make agents communicate\"*",
                code, name, count,
                if has_agent { " with agents" } else { "" }
            );
        }
    }

    // ── ML / LLM-style agent (Python — runs offline; swap in real LLM when configured) ──
    let wants_implementation = q.contains("write ") || q.contains("write a") || q.starts_with("write ")
        || q.contains("create ") || q.contains("build ") || q.contains("make ")
        || q.contains("program") || q.contains("code") || q.contains("script")
        || q.contains("implement") || q.contains("generate ") || q.contains("scaffold");
    let is_ml_agent_ask = (q.contains("ml agent") || q.contains("machine learning agent") || q.contains("llm agent")
        || q.contains(" ai agent") || q.contains("ai agent")
        || (q.contains("agent") && (q.contains("ml") || q.contains("machine learning") || q.contains("llm") || q.contains("gpt") || q.contains("openai") || q.contains("langchain") || q.contains("tool use") || q.contains("tool-calling")))
        || q.contains("langchain") || q.contains("autogpt"))
        && (lang == "python" || q.contains("python"));
    let not_pure_definitional = !q.starts_with("what is ") && !q.starts_with("what are ") && !q.starts_with("explain ");
    if is_ml_agent_ask && wants_implementation && not_pure_definitional {
        let code = "#!/usr/bin/env python3\n\"\"\"\nMinimal tool-using agent loop (offline demo). Replace `fake_llm_plan` with OpenAI,\nAnthropic, or Ollama when `OPENAI_API_KEY` / local endpoint is available.\n\"\"\"\nfrom __future__ import annotations\n\nimport json\nimport re\nfrom typing import Any, Callable\n\n# ── Register tools the model may invoke ───────────────────────────────────────\ndef tool_calc(expr: str) -> str:\n    \"\"\"Safe arithmetic only (no builtins).\"\"\"\n    if not re.fullmatch(r\"[0-9+\\-*/().\\s]+\", expr):\n        return \"error: only digits and +-*/(). allowed\"\n    try:\n        return str(round(eval(expr, {\"__builtins__\": {}}, {}), 8))\n    except Exception as e:  # noqa: BLE001\n        return f\"error: {e}\"\n\n\ndef tool_echo(text: str) -> str:\n    return (text or \"\").strip()\n\n\nTOOLS: dict[str, Callable[[str], str]] = {\n    \"calc\": tool_calc,\n    \"echo\": tool_echo,\n}\n\n\ndef fake_llm_plan(goal: str, trace: list[dict[str, Any]]) -> str:\n    \"\"\"Offline planner — returns one JSON line per step. Swap for llm.chat.completions.create.\"\"\"\n    g = goal.lower()\n    if \"2+2\" in g or \"2 + 2\" in g:\n        return json.dumps({\n            \"thought\": \"User asked for arithmetic; use calc tool.\",\n            \"action\": \"calc\",\n            \"input\": \"2+2\",\n        })\n    if \"capital\" in g and \"france\" in g:\n        return json.dumps({\n            \"thought\": \"Fact is known; finish without tool.\",\n            \"action\": \"done\",\n            \"input\": \"Paris is the capital of France.\",\n        })\n    # Default demo: show one tool call then answer\n    if not trace:\n        return json.dumps({\n            \"thought\": \"Demonstrate tool-use pattern for ML/agent tutorials.\",\n            \"action\": \"calc\",\n            \"input\": \"7*6\",\n        })\n    return json.dumps({\n        \"thought\": \"Synthesize after observation.\",\n        \"action\": \"done\",\n        \"input\": f\"Demo complete. Last tool result is in trace. Original goal: {goal[:120]}\",\n    })\n\n\ndef run_agent(\n    goal: str,\n    planner: Callable[[str, list[dict[str, Any]]], str] = fake_llm_plan,\n    max_steps: int = 12,\n) -> str:\n    trace: list[dict[str, Any]] = []\n    for _ in range(max_steps):\n        raw = planner(goal, trace)\n        try:\n            step = json.loads(raw)\n        except json.JSONDecodeError:\n            return raw  # treat as final text from real LLM\n        action = str(step.get(\"action\", \"done\")).lower()\n        inp = str(step.get(\"input\", \"\"))\n        trace.append({\"plan\": step})\n        if action == \"done\":\n            return inp\n        fn = TOOLS.get(action)\n        if fn is None:\n            trace.append({\"error\": f\"unknown action {action}\"})\n            continue\n        obs = fn(inp)\n        trace.append({\"observation\": obs})\n    return \"Stopped: max_steps — plug in a real LLM or narrow the goal.\"\n\n\nif __name__ == \"__main__\":\n    print(run_agent(\"What is 2+2?\"))\n    print(\"---\")\n    print(run_agent(\"Write a python ml agent program\"))\n";
        return format!(
            "```python\n{}\n```\n\n*Python ML-style agent: tool registry + JSON plan/execute loop. Works offline with `fake_llm_plan`; connect `khlm_set_llm` / OpenAI for real reasoning.*",
            code
        );
    }

    // ── Hello World (with typo tolerance: "hellow", "helo", "hallo") ────────
    let is_hello_world = q.contains("hello world") || q.contains("helloworld")
        || q.contains("hellow world") || q.contains("helo world") || q.contains("hallo world")
        || (q.contains("hello") && q.contains("world"))
        || (q.contains("helo") && q.contains("world"))
        || (q.contains("hellow") && q.contains("world"));
    if is_hello_world {
        let (code, note) = match lang {
            "killer" => ("kfn main() {\n  print(K\"Hello, World!\")\n}\nmain()", "Run: `killer-native hello.killer`"),
            "rust"   => ("fn main() {\n    println!(\"Hello, World!\");\n}", "Compile: `rustc main.rs && ./main`"),
            "javascript" => ("console.log('Hello, World!');", "Run: `node hello.js`"),
            "typescript" => ("const msg: string = 'Hello, World!';\nconsole.log(msg);", "Run: `ts-node hello.ts`"),
            "html"   => ("<!DOCTYPE html>\n<html>\n<body><h1>Hello, World!</h1></body>\n</html>", "Open in any browser."),
            "java"   => ("public class HelloWorld {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, World!\");\n    }\n}", "Compile: `javac HelloWorld.java && java HelloWorld`"),
            "cpp"    => ("#include <iostream>\n\nint main() {\n    std::cout << \"Hello, World!\" << std::endl;\n    return 0;\n}", "Compile: `g++ -o hello hello.cpp && ./hello`"),
            "bash"   => ("#!/bin/bash\necho \"Hello, World!\"", "Run: `bash hello.sh`"),
            "go"     => ("package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"Hello, World!\")\n}", "Run: `go run hello.go`"),
            "csharp" => ("using System;\n\nclass Program\n{\n    static void Main(string[] args)\n    {\n        Console.WriteLine(\"Hello, World!\");\n    }\n}", "Run: `dotnet run`"),
            "kotlin" => ("fun main() {\n    println(\"Hello, World!\")\n}", "Run: `kotlinc hello.kt -include-runtime -d hello.jar && java -jar hello.jar`"),
            "swift"  => ("print(\"Hello, World!\")", "Run: `swift hello.swift`"),
            "ruby"   => ("puts 'Hello, World!'", "Run: `ruby hello.rb`"),
            "php"    => ("<?php\necho \"Hello, World!\\n\";\n?>", "Run: `php hello.php`"),
            "scala"  => ("object Hello extends App {\n  println(\"Hello, World!\")\n}", "Run: `scala Hello.scala`"),
            "dart"   => ("void main() {\n  print('Hello, World!');\n}", "Run: `dart run hello.dart`"),
            "c"      => ("#include <stdio.h>\n\nint main() {\n    printf(\"Hello, World!\\n\");\n    return 0;\n}", "Compile: `gcc -o hello hello.c && ./hello`"),
            _ =>       ("print('Hello, World!')", "Run: `python hello.py`"),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── For loop / While loop / basic loops ──────────────────────────────────
    let is_loop_req = q.contains("for loop") || q.contains("for-loop") || q.contains("forloop")
        || q.contains("while loop") || q.contains("while-loop") || q.contains("whileloop")
        || (q.contains("loop") && (q.contains("program") || q.contains("example") || q.contains("code") || q.contains("write") || q.contains("basic")))
        || q.contains("iterate") || q.contains("iteration");
    if is_loop_req {
        let (code, note) = match lang {
            "killer" => ("# For loop — iterate 1 to 10\ni = 1\nwhile i <= 10 {\n  print(i)\n  i = i + 1\n}\n\n# Loop over an array\narr = [10, 20, 30, 40, 50]\nidx = 0\nwhile idx < len(arr) {\n  print(K\"Item {idx}: {arr[idx]}\")\n  idx = idx + 1\n}\n\n# Nested loop — multiplication table\nrow = 1\nwhile row <= 5 {\n  col = 1\n  while col <= 5 {\n    print(K\"{row} x {col} = {row * col}\")\n    col = col + 1\n  }\n  row = row + 1\n}", "Killer loops — while-based iteration, array traversal, nested loops."),
            "rust" => ("fn main() {\n    // For loop — range\n    for i in 1..=10 {\n        println!(\"{}\", i);\n    }\n\n    // For loop — iterate over a vector\n    let fruits = vec![\"apple\", \"banana\", \"cherry\"];\n    for (i, fruit) in fruits.iter().enumerate() {\n        println!(\"{}: {}\", i, fruit);\n    }\n\n    // While loop\n    let mut count = 0;\n    while count < 5 {\n        println!(\"count = {}\", count);\n        count += 1;\n    }\n\n    // Loop with break\n    let mut sum = 0;\n    loop {\n        sum += 1;\n        if sum >= 100 {\n            break;\n        }\n    }\n    println!(\"sum = {}\", sum);\n\n    // Nested loop — multiplication table\n    for row in 1..=5 {\n        for col in 1..=5 {\n            print!(\"{:4}\", row * col);\n        }\n        println!();\n    }\n}", "Rust loops — for range, iter, while, loop+break, nested."),
            "javascript" | "typescript" => ("// For loop — classic\nfor (let i = 1; i <= 10; i++) {\n    console.log(i);\n}\n\n// For...of — iterate array\nconst fruits = ['apple', 'banana', 'cherry'];\nfor (const fruit of fruits) {\n    console.log(fruit);\n}\n\n// For...in — iterate object keys\nconst user = { name: 'Arun', age: 28, city: 'Hyderabad' };\nfor (const key in user) {\n    console.log(`${key}: ${user[key]}`);\n}\n\n// While loop\nlet count = 0;\nwhile (count < 5) {\n    console.log(`count = ${count}`);\n    count++;\n}\n\n// Do-while\nlet num = 1;\ndo {\n    console.log(num);\n    num *= 2;\n} while (num <= 64);\n\n// forEach with index\nfruits.forEach((fruit, i) => {\n    console.log(`${i}: ${fruit}`);\n});\n\n// Nested loop — multiplication table\nfor (let row = 1; row <= 5; row++) {\n    let line = '';\n    for (let col = 1; col <= 5; col++) {\n        line += String(row * col).padStart(4);\n    }\n    console.log(line);\n}", "JavaScript loops — for, for..of, for..in, while, do-while, forEach."),
            "java" => ("public class ForLoopDemo {\n    public static void main(String[] args) {\n        // Basic for loop — print 1 to 10\n        for (int i = 1; i <= 10; i++) {\n            System.out.println(i);\n        }\n\n        // Enhanced for loop (for-each) — iterate array\n        String[] fruits = {\"apple\", \"banana\", \"cherry\", \"date\"};\n        for (String fruit : fruits) {\n            System.out.println(fruit);\n        }\n\n        // While loop\n        int count = 0;\n        while (count < 5) {\n            System.out.println(\"count = \" + count);\n            count++;\n        }\n\n        // Do-while loop\n        int num = 1;\n        do {\n            System.out.println(num);\n            num *= 2;\n        } while (num <= 64);\n\n        // For loop with index on array\n        int[] numbers = {10, 20, 30, 40, 50};\n        for (int i = 0; i < numbers.length; i++) {\n            System.out.println(\"numbers[\" + i + \"] = \" + numbers[i]);\n        }\n\n        // Nested loop — multiplication table\n        for (int row = 1; row <= 5; row++) {\n            for (int col = 1; col <= 5; col++) {\n                System.out.printf(\"%4d\", row * col);\n            }\n            System.out.println();\n        }\n\n        // Loop with break and continue\n        for (int i = 1; i <= 20; i++) {\n            if (i % 3 == 0) continue;\n            if (i > 15) break;\n            System.out.println(i);\n        }\n    }\n}", "Java loops — for, for-each, while, do-while, nested, break/continue."),
            "cpp" => ("#include <iostream>\n#include <vector>\n#include <string>\nusing namespace std;\n\nint main() {\n    // Basic for loop\n    for (int i = 1; i <= 10; i++) {\n        cout << i << endl;\n    }\n\n    // Range-based for loop (C++11)\n    vector<string> fruits = {\"apple\", \"banana\", \"cherry\"};\n    for (const auto& fruit : fruits) {\n        cout << fruit << endl;\n    }\n\n    // While loop\n    int count = 0;\n    while (count < 5) {\n        cout << \"count = \" << count << endl;\n        count++;\n    }\n\n    // Do-while\n    int num = 1;\n    do {\n        cout << num << endl;\n        num *= 2;\n    } while (num <= 64);\n\n    // Nested loop — multiplication table\n    for (int row = 1; row <= 5; row++) {\n        for (int col = 1; col <= 5; col++) {\n            printf(\"%4d\", row * col);\n        }\n        cout << endl;\n    }\n    return 0;\n}", "C++ loops — for, range-based for, while, do-while, nested."),
            "c" => ("#include <stdio.h>\n\nint main() {\n    // Basic for loop\n    for (int i = 1; i <= 10; i++) {\n        printf(\"%d\\n\", i);\n    }\n\n    // While loop\n    int count = 0;\n    while (count < 5) {\n        printf(\"count = %d\\n\", count);\n        count++;\n    }\n\n    // Do-while\n    int num = 1;\n    do {\n        printf(\"%d\\n\", num);\n        num *= 2;\n    } while (num <= 64);\n\n    // Iterate over array\n    int arr[] = {10, 20, 30, 40, 50};\n    int len = sizeof(arr) / sizeof(arr[0]);\n    for (int i = 0; i < len; i++) {\n        printf(\"arr[%d] = %d\\n\", i, arr[i]);\n    }\n\n    // Nested loop — multiplication table\n    for (int row = 1; row <= 5; row++) {\n        for (int col = 1; col <= 5; col++) {\n            printf(\"%4d\", row * col);\n        }\n        printf(\"\\n\");\n    }\n    return 0;\n}", "C loops — for, while, do-while, array traversal, nested."),
            "go" => ("package main\n\nimport \"fmt\"\n\nfunc main() {\n\t// Basic for loop (Go only has 'for')\n\tfor i := 1; i <= 10; i++ {\n\t\tfmt.Println(i)\n\t}\n\n\t// For-range over slice\n\tfruits := []string{\"apple\", \"banana\", \"cherry\"}\n\tfor i, fruit := range fruits {\n\t\tfmt.Printf(\"%d: %s\\n\", i, fruit)\n\t}\n\n\t// While-style loop\n\tcount := 0\n\tfor count < 5 {\n\t\tfmt.Println(\"count =\", count)\n\t\tcount++\n\t}\n\n\t// Infinite loop with break\n\tsum := 0\n\tfor {\n\t\tsum++\n\t\tif sum >= 100 {\n\t\t\tbreak\n\t\t}\n\t}\n\tfmt.Println(\"sum =\", sum)\n\n\t// Nested — multiplication table\n\tfor row := 1; row <= 5; row++ {\n\t\tfor col := 1; col <= 5; col++ {\n\t\t\tfmt.Printf(\"%4d\", row*col)\n\t\t}\n\t\tfmt.Println()\n\t}\n}", "Go loops — for, for-range, while-style, infinite+break, nested."),
            "csharp" => ("using System;\n\nclass LoopDemo {\n    static void Main() {\n        // Basic for loop\n        for (int i = 1; i <= 10; i++) {\n            Console.WriteLine(i);\n        }\n\n        // Foreach\n        string[] fruits = {\"apple\", \"banana\", \"cherry\"};\n        foreach (string fruit in fruits) {\n            Console.WriteLine(fruit);\n        }\n\n        // While loop\n        int count = 0;\n        while (count < 5) {\n            Console.WriteLine($\"count = {count}\");\n            count++;\n        }\n\n        // Do-while\n        int num = 1;\n        do {\n            Console.WriteLine(num);\n            num *= 2;\n        } while (num <= 64);\n\n        // Nested — multiplication table\n        for (int row = 1; row <= 5; row++) {\n            for (int col = 1; col <= 5; col++) {\n                Console.Write($\"{row * col,4}\");\n            }\n            Console.WriteLine();\n        }\n    }\n}", "C# loops — for, foreach, while, do-while, nested."),
            "kotlin" => ("fun main() {\n    // For loop — range\n    for (i in 1..10) {\n        println(i)\n    }\n\n    // For-each over list\n    val fruits = listOf(\"apple\", \"banana\", \"cherry\")\n    for (fruit in fruits) {\n        println(fruit)\n    }\n\n    // With index\n    for ((i, fruit) in fruits.withIndex()) {\n        println(\"$i: $fruit\")\n    }\n\n    // While loop\n    var count = 0\n    while (count < 5) {\n        println(\"count = $count\")\n        count++\n    }\n\n    // Do-while\n    var num = 1\n    do {\n        println(num)\n        num *= 2\n    } while (num <= 64)\n\n    // Repeat\n    repeat(3) {\n        println(\"Hello #$it\")\n    }\n\n    // Nested — multiplication table\n    for (row in 1..5) {\n        for (col in 1..5) {\n            print(\"%4d\".format(row * col))\n        }\n        println()\n    }\n}", "Kotlin loops — for range, forEach, while, do-while, repeat, nested."),
            "swift" => ("import Foundation\n\n// For loop — range\nfor i in 1...10 {\n    print(i)\n}\n\n// For-in over array\nlet fruits = [\"apple\", \"banana\", \"cherry\"]\nfor fruit in fruits {\n    print(fruit)\n}\n\n// With index\nfor (i, fruit) in fruits.enumerated() {\n    print(\"\\(i): \\(fruit)\")\n}\n\n// While loop\nvar count = 0\nwhile count < 5 {\n    print(\"count = \\(count)\")\n    count += 1\n}\n\n// Repeat-while (do-while)\nvar num = 1\nrepeat {\n    print(num)\n    num *= 2\n} while num <= 64\n\n// Stride\nfor i in stride(from: 0, to: 20, by: 3) {\n    print(i)\n}\n\n// Nested — multiplication table\nfor row in 1...5 {\n    for col in 1...5 {\n        print(String(format: \"%4d\", row * col), terminator: \"\")\n    }\n    print()\n}", "Swift loops — for-in, while, repeat-while, stride, nested."),
            "ruby" => ("# For loop — range\nfor i in 1..10\n  puts i\nend\n\n# .each (idiomatic Ruby)\n[10, 20, 30, 40, 50].each do |num|\n  puts num\nend\n\n# .each_with_index\n%w[apple banana cherry].each_with_index do |fruit, i|\n  puts \"#{i}: #{fruit}\"\nend\n\n# While loop\ncount = 0\nwhile count < 5\n  puts \"count = #{count}\"\n  count += 1\nend\n\n# Until loop\nnum = 1\nuntil num > 64\n  puts num\n  num *= 2\nend\n\n# .times\n5.times { |i| puts \"Hello ##{i}\" }\n\n# .upto / .downto\n1.upto(5) { |i| print \"#{i} \" }\nputs\n\n# Nested — multiplication table\n(1..5).each do |row|\n  (1..5).each do |col|\n    print format('%4d', row * col)\n  end\n  puts\nend", "Ruby loops — for, each, while, until, times, upto, nested."),
            "php" => ("<?php\n// Basic for loop\nfor ($i = 1; $i <= 10; $i++) {\n    echo $i . \"\\n\";\n}\n\n// Foreach — array\n$fruits = ['apple', 'banana', 'cherry'];\nforeach ($fruits as $fruit) {\n    echo $fruit . \"\\n\";\n}\n\n// Foreach with key\nforeach ($fruits as $i => $fruit) {\n    echo \"$i: $fruit\\n\";\n}\n\n// While loop\n$count = 0;\nwhile ($count < 5) {\n    echo \"count = $count\\n\";\n    $count++;\n}\n\n// Do-while\n$num = 1;\ndo {\n    echo $num . \"\\n\";\n    $num *= 2;\n} while ($num <= 64);\n\n// Nested — multiplication table\nfor ($row = 1; $row <= 5; $row++) {\n    for ($col = 1; $col <= 5; $col++) {\n        printf('%4d', $row * $col);\n    }\n    echo \"\\n\";\n}\n?>", "PHP loops — for, foreach, while, do-while, nested."),
            "bash" => ("#!/bin/bash\n\n# For loop — range\nfor i in {1..10}; do\n    echo \"$i\"\ndone\n\n# For loop — C-style\nfor ((i=1; i<=10; i++)); do\n    echo \"$i\"\ndone\n\n# For loop — iterate list\nfruits=(\"apple\" \"banana\" \"cherry\")\nfor fruit in \"${fruits[@]}\"; do\n    echo \"$fruit\"\ndone\n\n# While loop\ncount=0\nwhile [ $count -lt 5 ]; do\n    echo \"count = $count\"\n    ((count++))\ndone\n\n# While read lines from file\nwhile IFS= read -r line; do\n    echo \"Line: $line\"\ndone < /etc/hostname\n\n# Until loop\nnum=1\nuntil [ $num -gt 64 ]; do\n    echo \"$num\"\n    ((num*=2))\ndone\n\n# Nested — multiplication table\nfor row in {1..5}; do\n    for col in {1..5}; do\n        printf '%4d' $((row * col))\n    done\n    echo\ndone", "Bash loops — for, C-style for, while, until, nested."),
            _ => ("# For loop — range\nfor i in range(1, 11):\n    print(i)\n\n# For loop — iterate list\nfruits = ['apple', 'banana', 'cherry']\nfor fruit in fruits:\n    print(fruit)\n\n# With index\nfor i, fruit in enumerate(fruits):\n    print(f'{i}: {fruit}')\n\n# While loop\ncount = 0\nwhile count < 5:\n    print(f'count = {count}')\n    count += 1\n\n# List comprehension (Pythonic loop)\nsquares = [x**2 for x in range(1, 11)]\nprint(squares)\n\n# Loop with break and continue\nfor i in range(1, 21):\n    if i % 3 == 0:\n        continue\n    if i > 15:\n        break\n    print(i)\n\n# Nested loop — multiplication table\nfor row in range(1, 6):\n    for col in range(1, 6):\n        print(f'{row * col:4d}', end='')\n    print()\n\n# Loop over dictionary\nuser = {'name': 'Arun', 'age': 28, 'city': 'Hyderabad'}\nfor key, value in user.items():\n    print(f'{key}: {value}')", "Python loops — for, while, enumerate, comprehension, nested, dict iteration."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── If-else / conditionals / switch-case ─────────────────────────────────
    let is_conditional_req = q.contains("if else") || q.contains("if-else") || q.contains("ifelse")
        || q.contains("switch case") || q.contains("switch-case") || q.contains("conditional")
        || (q.contains("if ") && q.contains("else") && (q.contains("program") || q.contains("example") || q.contains("code") || q.contains("write")));
    if is_conditional_req {
        let (code, note) = match lang {
            "java" => ("public class IfElseDemo {\n    public static void main(String[] args) {\n        int score = 85;\n\n        // If-else if-else\n        if (score >= 90) {\n            System.out.println(\"Grade: A\");\n        } else if (score >= 80) {\n            System.out.println(\"Grade: B\");\n        } else if (score >= 70) {\n            System.out.println(\"Grade: C\");\n        } else {\n            System.out.println(\"Grade: F\");\n        }\n\n        // Ternary operator\n        String result = score >= 60 ? \"Pass\" : \"Fail\";\n        System.out.println(result);\n\n        // Switch-case\n        String day = \"Monday\";\n        switch (day) {\n            case \"Monday\":\n            case \"Tuesday\":\n            case \"Wednesday\":\n            case \"Thursday\":\n            case \"Friday\":\n                System.out.println(\"Weekday\");\n                break;\n            case \"Saturday\":\n            case \"Sunday\":\n                System.out.println(\"Weekend\");\n                break;\n            default:\n                System.out.println(\"Unknown\");\n        }\n\n        // Switch expression (Java 14+)\n        int numDay = 3;\n        String name = switch (numDay) {\n            case 1 -> \"Monday\";\n            case 2 -> \"Tuesday\";\n            case 3 -> \"Wednesday\";\n            default -> \"Other\";\n        };\n        System.out.println(name);\n    }\n}", "Java conditionals — if-else, ternary, switch-case, switch expression."),
            "rust" => ("fn main() {\n    let score = 85;\n\n    // If-else\n    if score >= 90 {\n        println!(\"Grade: A\");\n    } else if score >= 80 {\n        println!(\"Grade: B\");\n    } else {\n        println!(\"Grade: C or below\");\n    }\n\n    // If as expression\n    let result = if score >= 60 { \"Pass\" } else { \"Fail\" };\n    println!(\"{}\", result);\n\n    // Match (pattern matching)\n    let day = \"Wednesday\";\n    match day {\n        \"Monday\" | \"Tuesday\" | \"Wednesday\" | \"Thursday\" | \"Friday\" => println!(\"Weekday\"),\n        \"Saturday\" | \"Sunday\" => println!(\"Weekend\"),\n        _ => println!(\"Unknown\"),\n    }\n\n    // Match with ranges\n    let grade = match score {\n        90..=100 => 'A',\n        80..=89 => 'B',\n        70..=79 => 'C',\n        _ => 'F',\n    };\n    println!(\"Grade: {}\", grade);\n}", "Rust conditionals — if-else, if expression, match, pattern matching."),
            _ => ("score = 85\n\n# If-elif-else\nif score >= 90:\n    print('Grade: A')\nelif score >= 80:\n    print('Grade: B')\nelif score >= 70:\n    print('Grade: C')\nelse:\n    print('Grade: F')\n\n# Ternary\nresult = 'Pass' if score >= 60 else 'Fail'\nprint(result)\n\n# Match-case (Python 3.10+)\nday = 'Wednesday'\nmatch day:\n    case 'Monday' | 'Tuesday' | 'Wednesday' | 'Thursday' | 'Friday':\n        print('Weekday')\n    case 'Saturday' | 'Sunday':\n        print('Weekend')\n    case _:\n        print('Unknown')\n\n# Chained conditions\nage = 25\nif 18 <= age < 65:\n    print('Working age')", "Python conditionals — if-elif-else, ternary, match-case."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Arrays / Lists / Collections ─────────────────────────────────────────
    let is_array_req = q.contains("array") || q.contains("arraylist")
        || (q.contains("list") && !q.contains("linked list") && (q.contains("program") || q.contains("example") || q.contains("code") || q.contains("write") || q.contains("basic")));
    if is_array_req {
        let (code, note) = match lang {
            "java" => ("import java.util.ArrayList;\nimport java.util.Arrays;\nimport java.util.Collections;\n\npublic class ArrayDemo {\n    public static void main(String[] args) {\n        // Array declaration and initialization\n        int[] numbers = {10, 20, 30, 40, 50};\n        String[] fruits = new String[3];\n        fruits[0] = \"apple\";\n        fruits[1] = \"banana\";\n        fruits[2] = \"cherry\";\n\n        // Access and iterate\n        System.out.println(\"First: \" + numbers[0]);\n        System.out.println(\"Length: \" + numbers.length);\n        for (int num : numbers) {\n            System.out.println(num);\n        }\n\n        // ArrayList (dynamic)\n        ArrayList<String> list = new ArrayList<>(Arrays.asList(\"dog\", \"cat\", \"bird\"));\n        list.add(\"fish\");\n        list.remove(\"cat\");\n        System.out.println(\"Size: \" + list.size());\n        System.out.println(\"Contains dog: \" + list.contains(\"dog\"));\n\n        // Sort\n        Collections.sort(list);\n        System.out.println(\"Sorted: \" + list);\n\n        // Array to ArrayList and back\n        ArrayList<Integer> numList = new ArrayList<>(Arrays.asList(5, 3, 8, 1));\n        Collections.sort(numList);\n        System.out.println(\"Sorted numbers: \" + numList);\n\n        // 2D array\n        int[][] matrix = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};\n        for (int[] row : matrix) {\n            System.out.println(Arrays.toString(row));\n        }\n    }\n}", "Java arrays — static arrays, ArrayList, sort, 2D array."),
            _ => ("# List basics\nnumbers = [10, 20, 30, 40, 50]\nfruits = ['apple', 'banana', 'cherry']\n\n# Access\nprint(numbers[0])     # 10\nprint(numbers[-1])    # 50\nprint(numbers[1:3])   # [20, 30]\n\n# Modify\nnumbers.append(60)\nnumbers.insert(0, 5)\nnumbers.remove(30)\nprint(numbers)\n\n# Sort\nnumbers.sort()\nprint(sorted(fruits, reverse=True))\n\n# List comprehension\nsquares = [x**2 for x in range(1, 6)]\nprint(squares)  # [1, 4, 9, 16, 25]\n\n# 2D list\nmatrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]\nfor row in matrix:\n    print(row)", "Python lists — access, modify, sort, comprehension, 2D."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Games (tic-tac-toe, guessing, hangman, snake, quiz) ─────────────────
    let is_game_req = q.contains("tic tac toe") || q.contains("tic-tac-toe") || q.contains("tictactoe")
        || q.contains("guessing game") || q.contains("guess the number") || q.contains("number guess")
        || q.contains("hangman") || q.contains("snake game") || q.contains("quiz game")
        || q.contains("rock paper") || q.contains("dice game") || q.contains("coin flip")
        || (q.contains("game") && (q.contains("program") || q.contains("code") || q.contains("write") || q.contains("build") || q.contains("create") || q.contains("simple")));
    if is_game_req {
        let is_ttt = q.contains("tic tac") || q.contains("tic-tac") || q.contains("tictac");
        let is_rps = q.contains("rock paper") || q.contains("rock-paper");
        if is_ttt {
            let (code, note) = match lang {
                "java" => ("import java.util.Scanner;\n\npublic class TicTacToe {\n    static char[] board = {'1','2','3','4','5','6','7','8','9'};\n    static char current = 'X';\n\n    static void printBoard() {\n        System.out.println();\n        for (int i = 0; i < 9; i += 3) {\n            System.out.printf(\" %c | %c | %c\\n\", board[i], board[i+1], board[i+2]);\n            if (i < 6) System.out.println(\"---+---+---\");\n        }\n        System.out.println();\n    }\n\n    static boolean checkWin() {\n        int[][] wins = {{0,1,2},{3,4,5},{6,7,8},{0,3,6},{1,4,7},{2,5,8},{0,4,8},{2,4,6}};\n        for (int[] w : wins) {\n            if (board[w[0]] == board[w[1]] && board[w[1]] == board[w[2]]) return true;\n        }\n        return false;\n    }\n\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n        for (int turn = 0; turn < 9; turn++) {\n            printBoard();\n            System.out.printf(\"Player %c, pick a cell (1-9): \", current);\n            int choice = sc.nextInt() - 1;\n            if (choice < 0 || choice > 8 || board[choice] == 'X' || board[choice] == 'O') {\n                System.out.println(\"Invalid move!\"); turn--; continue;\n            }\n            board[choice] = current;\n            if (checkWin()) {\n                printBoard();\n                System.out.println(\"Player \" + current + \" wins!\");\n                return;\n            }\n            current = (current == 'X') ? 'O' : 'X';\n        }\n        printBoard();\n        System.out.println(\"It's a draw!\");\n    }\n}", "Java Tic-Tac-Toe — 2-player console game."),
                _ => ("def print_board(b):\n    for i in range(0, 9, 3):\n        print(f' {b[i]} | {b[i+1]} | {b[i+2]}')\n        if i < 6: print('---+---+---')\n    print()\n\ndef check_win(b, p):\n    wins = [(0,1,2),(3,4,5),(6,7,8),(0,3,6),(1,4,7),(2,5,8),(0,4,8),(2,4,6)]\n    return any(b[a]==b[b_]==b[c]==p for a,b_,c in wins)\n\ndef main():\n    board = list('123456789')\n    current = 'X'\n    for turn in range(9):\n        print_board(board)\n        choice = input(f'Player {current}, pick (1-9): ')\n        idx = int(choice) - 1\n        if idx < 0 or idx > 8 or board[idx] in 'XO':\n            print('Invalid!'); continue\n        board[idx] = current\n        if check_win(board, current):\n            print_board(board)\n            print(f'Player {current} wins!')\n            return\n        current = 'O' if current == 'X' else 'X'\n    print_board(board)\n    print(\"It's a draw!\")\n\nmain()", "Python Tic-Tac-Toe — 2-player console game."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if is_rps {
            let (code, note) = match lang {
                "java" => ("import java.util.Scanner;\nimport java.util.Random;\n\npublic class RockPaperScissors {\n    public static void main(String[] args) {\n        String[] choices = {\"rock\", \"paper\", \"scissors\"};\n        Scanner sc = new Scanner(System.in);\n        Random rng = new Random();\n        int wins = 0, losses = 0, draws = 0;\n\n        System.out.println(\"Rock Paper Scissors! Type 'quit' to exit.\");\n        while (true) {\n            System.out.print(\"Your choice (rock/paper/scissors): \");\n            String player = sc.nextLine().trim().toLowerCase();\n            if (player.equals(\"quit\")) break;\n            int pi = -1;\n            for (int i = 0; i < 3; i++) if (choices[i].equals(player)) pi = i;\n            if (pi == -1) { System.out.println(\"Invalid choice!\"); continue; }\n            int ci = rng.nextInt(3);\n            System.out.println(\"Computer: \" + choices[ci]);\n            if (pi == ci) { System.out.println(\"Draw!\"); draws++; }\n            else if ((pi + 1) % 3 == ci) { System.out.println(\"You lose!\"); losses++; }\n            else { System.out.println(\"You win!\"); wins++; }\n        }\n        System.out.printf(\"Score — Wins: %d, Losses: %d, Draws: %d%n\", wins, losses, draws);\n    }\n}", "Java Rock-Paper-Scissors with score tracking."),
                _ => ("import random\n\ndef play():\n    choices = ['rock', 'paper', 'scissors']\n    wins = losses = draws = 0\n    print('Rock Paper Scissors! Type \"quit\" to exit.')\n    while True:\n        player = input('Your choice: ').strip().lower()\n        if player == 'quit': break\n        if player not in choices:\n            print('Invalid! Choose rock/paper/scissors'); continue\n        computer = random.choice(choices)\n        print(f'Computer: {computer}')\n        if player == computer:\n            print('Draw!'); draws += 1\n        elif (player, computer) in [('rock','scissors'),('paper','rock'),('scissors','paper')]:\n            print('You win!'); wins += 1\n        else:\n            print('You lose!'); losses += 1\n    print(f'Score — Wins: {wins}, Losses: {losses}, Draws: {draws}')\n\nplay()", "Python Rock-Paper-Scissors with score."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // Default game: number guessing
        let (code, note) = match lang {
            "java" => ("import java.util.Scanner;\nimport java.util.Random;\n\npublic class GuessingGame {\n    public static void main(String[] args) {\n        Random rng = new Random();\n        int secret = rng.nextInt(100) + 1;\n        Scanner sc = new Scanner(System.in);\n        int attempts = 0;\n\n        System.out.println(\"I'm thinking of a number between 1 and 100.\");\n        while (true) {\n            System.out.print(\"Your guess: \");\n            int guess = sc.nextInt();\n            attempts++;\n            if (guess < secret) {\n                System.out.println(\"Too low!\");\n            } else if (guess > secret) {\n                System.out.println(\"Too high!\");\n            } else {\n                System.out.println(\"Correct! You got it in \" + attempts + \" attempts.\");\n                break;\n            }\n        }\n    }\n}", "Java number guessing game."),
            "rust" => ("use std::io::{self, Write};\n\nfn main() {\n    let secret = (std::time::SystemTime::now()\n        .duration_since(std::time::UNIX_EPOCH).unwrap()\n        .subsec_nanos() % 100 + 1) as i32;\n    let mut attempts = 0;\n    println!(\"I'm thinking of a number between 1 and 100.\");\n    loop {\n        print!(\"Your guess: \");\n        io::stdout().flush().unwrap();\n        let mut input = String::new();\n        io::stdin().read_line(&mut input).unwrap();\n        let guess: i32 = match input.trim().parse() {\n            Ok(n) => n,\n            Err(_) => { println!(\"Enter a number!\"); continue; }\n        };\n        attempts += 1;\n        match guess.cmp(&secret) {\n            std::cmp::Ordering::Less => println!(\"Too low!\"),\n            std::cmp::Ordering::Greater => println!(\"Too high!\"),\n            std::cmp::Ordering::Equal => {\n                println!(\"Correct! {} attempts.\", attempts);\n                break;\n            }\n        }\n    }\n}", "Rust guessing game — no external deps."),
            _ => ("import random\n\ndef guessing_game():\n    secret = random.randint(1, 100)\n    attempts = 0\n    print('I\\'m thinking of a number between 1 and 100.')\n    while True:\n        guess = int(input('Your guess: '))\n        attempts += 1\n        if guess < secret:\n            print('Too low!')\n        elif guess > secret:\n            print('Too high!')\n        else:\n            print(f'Correct! You got it in {attempts} attempts.')\n            break\n\nguessing_game()", "Python number guessing game."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Star / number patterns (pyramid, diamond, triangle) ──────────────────
    let is_pattern_req = q.contains("star pattern") || q.contains("pattern program")
        || q.contains("pyramid") || q.contains("diamond pattern") || q.contains("triangle pattern")
        || q.contains("triangle loop") || q.contains("triangle program") || q.contains("print triangle")
        || (q.contains("triangle") && (q.contains("loop") || q.contains("print") || q.contains("star") || q.contains("draw") || q.contains("create")))
        || q.contains("number pattern") || q.contains("print pattern")
        || (q.contains("pattern") && (q.contains("program") || q.contains("code") || q.contains("write") || q.contains("print")));
    if is_pattern_req {
        let (code, note) = match lang {
            "java" => ("public class StarPatterns {\n    public static void main(String[] args) {\n        int n = 5;\n\n        // Right triangle\n        System.out.println(\"Right triangle:\");\n        for (int i = 1; i <= n; i++) {\n            for (int j = 1; j <= i; j++) System.out.print(\"* \");\n            System.out.println();\n        }\n\n        // Inverted triangle\n        System.out.println(\"\\nInverted triangle:\");\n        for (int i = n; i >= 1; i--) {\n            for (int j = 1; j <= i; j++) System.out.print(\"* \");\n            System.out.println();\n        }\n\n        // Pyramid\n        System.out.println(\"\\nPyramid:\");\n        for (int i = 1; i <= n; i++) {\n            for (int s = 0; s < n - i; s++) System.out.print(\" \");\n            for (int j = 0; j < 2 * i - 1; j++) System.out.print(\"*\");\n            System.out.println();\n        }\n\n        // Diamond\n        System.out.println(\"\\nDiamond:\");\n        for (int i = 1; i <= n; i++) {\n            for (int s = 0; s < n - i; s++) System.out.print(\" \");\n            for (int j = 0; j < 2 * i - 1; j++) System.out.print(\"*\");\n            System.out.println();\n        }\n        for (int i = n - 1; i >= 1; i--) {\n            for (int s = 0; s < n - i; s++) System.out.print(\" \");\n            for (int j = 0; j < 2 * i - 1; j++) System.out.print(\"*\");\n            System.out.println();\n        }\n\n        // Number triangle\n        System.out.println(\"\\nNumber triangle:\");\n        for (int i = 1; i <= n; i++) {\n            for (int j = 1; j <= i; j++) System.out.print(j + \" \");\n            System.out.println();\n        }\n    }\n}", "Java star patterns — triangle, pyramid, diamond, number triangle."),
            _ => ("n = 5\n\n# Right triangle\nprint('Right triangle:')\nfor i in range(1, n + 1):\n    print('* ' * i)\n\n# Inverted triangle\nprint('\\nInverted triangle:')\nfor i in range(n, 0, -1):\n    print('* ' * i)\n\n# Pyramid\nprint('\\nPyramid:')\nfor i in range(1, n + 1):\n    print(' ' * (n - i) + '*' * (2 * i - 1))\n\n# Diamond\nprint('\\nDiamond:')\nfor i in range(1, n + 1):\n    print(' ' * (n - i) + '*' * (2 * i - 1))\nfor i in range(n - 1, 0, -1):\n    print(' ' * (n - i) + '*' * (2 * i - 1))\n\n# Number triangle\nprint('\\nNumber triangle:')\nfor i in range(1, n + 1):\n    print(' '.join(str(j) for j in range(1, i + 1)))\n\n# Floyd's triangle\nprint(\"\\nFloyd's triangle:\")\nnum = 1\nfor i in range(1, n + 1):\n    for j in range(i):\n        print(num, end=' ')\n        num += 1\n    print()", "Python patterns — triangle, pyramid, diamond, number triangle, Floyd's."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Matrix operations (multiply, transpose, add) ─────────────────────────
    let is_matrix_req = q.contains("matrix") && (q.contains("multi") || q.contains("transpose")
        || q.contains("add") || q.contains("program") || q.contains("code") || q.contains("example")
        || q.contains("operation") || q.contains("write"));
    if is_matrix_req {
        let (code, note) = match lang {
            "java" => ("public class MatrixOps {\n    static void printMatrix(int[][] m) {\n        for (int[] row : m) {\n            for (int v : row) System.out.printf(\"%4d\", v);\n            System.out.println();\n        }\n    }\n\n    static int[][] multiply(int[][] a, int[][] b) {\n        int rows = a.length, cols = b[0].length, inner = b.length;\n        int[][] result = new int[rows][cols];\n        for (int i = 0; i < rows; i++)\n            for (int j = 0; j < cols; j++)\n                for (int k = 0; k < inner; k++)\n                    result[i][j] += a[i][k] * b[k][j];\n        return result;\n    }\n\n    static int[][] transpose(int[][] m) {\n        int rows = m.length, cols = m[0].length;\n        int[][] t = new int[cols][rows];\n        for (int i = 0; i < rows; i++)\n            for (int j = 0; j < cols; j++)\n                t[j][i] = m[i][j];\n        return t;\n    }\n\n    public static void main(String[] args) {\n        int[][] a = {{1, 2, 3}, {4, 5, 6}};\n        int[][] b = {{7, 8}, {9, 10}, {11, 12}};\n\n        System.out.println(\"Matrix A:\");\n        printMatrix(a);\n        System.out.println(\"\\nMatrix B:\");\n        printMatrix(b);\n        System.out.println(\"\\nA x B:\");\n        printMatrix(multiply(a, b));\n        System.out.println(\"\\nTranspose of A:\");\n        printMatrix(transpose(a));\n    }\n}", "Java matrix — multiply, transpose, print."),
            "cpp" => ("#include <iostream>\n#include <vector>\nusing namespace std;\n\ntypedef vector<vector<int>> Matrix;\n\nvoid print(const Matrix& m) {\n    for (auto& row : m) {\n        for (int v : row) printf(\"%4d\", v);\n        cout << endl;\n    }\n}\n\nMatrix multiply(const Matrix& a, const Matrix& b) {\n    int rows = a.size(), cols = b[0].size(), inner = b.size();\n    Matrix r(rows, vector<int>(cols, 0));\n    for (int i = 0; i < rows; i++)\n        for (int j = 0; j < cols; j++)\n            for (int k = 0; k < inner; k++)\n                r[i][j] += a[i][k] * b[k][j];\n    return r;\n}\n\nMatrix transpose(const Matrix& m) {\n    int rows = m.size(), cols = m[0].size();\n    Matrix t(cols, vector<int>(rows));\n    for (int i = 0; i < rows; i++)\n        for (int j = 0; j < cols; j++)\n            t[j][i] = m[i][j];\n    return t;\n}\n\nint main() {\n    Matrix a = {{1,2,3},{4,5,6}};\n    Matrix b = {{7,8},{9,10},{11,12}};\n    cout << \"A x B:\" << endl;\n    print(multiply(a, b));\n    cout << \"\\nTranspose of A:\" << endl;\n    print(transpose(a));\n}", "C++ matrix — multiply, transpose using vectors."),
            _ => ("def print_matrix(m):\n    for row in m:\n        print(' '.join(f'{v:4d}' for v in row))\n\ndef multiply(a, b):\n    rows, inner, cols = len(a), len(b), len(b[0])\n    result = [[0]*cols for _ in range(rows)]\n    for i in range(rows):\n        for j in range(cols):\n            for k in range(inner):\n                result[i][j] += a[i][k] * b[k][j]\n    return result\n\ndef transpose(m):\n    return [[m[i][j] for i in range(len(m))] for j in range(len(m[0]))]\n\na = [[1, 2, 3], [4, 5, 6]]\nb = [[7, 8], [9, 10], [11, 12]]\n\nprint('Matrix A:'); print_matrix(a)\nprint('\\nMatrix B:'); print_matrix(b)\nprint('\\nA x B:'); print_matrix(multiply(a, b))\nprint('\\nTranspose of A:'); print_matrix(transpose(a))", "Python matrix — multiply, transpose (pure Python, no numpy)."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Interface / enum / abstract / struct examples ────────────────────────
    let is_interface_req = q.contains("interface") && !q.contains("gui") && !q.contains("user interface");
    let is_enum_req = q.contains("enum");
    let is_struct_req = q.contains("struct") && !q.contains("data structure");
    if is_interface_req || is_enum_req || is_struct_req {
        if is_enum_req {
            let (code, note) = match lang {
                "java" => ("public class EnumDemo {\n    enum Day {\n        MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY;\n\n        public boolean isWeekend() {\n            return this == SATURDAY || this == SUNDAY;\n        }\n    }\n\n    enum Planet {\n        MERCURY(3.303e+23, 2.4397e6),\n        VENUS(4.869e+24, 6.0518e6),\n        EARTH(5.976e+24, 6.37814e6),\n        MARS(6.421e+23, 3.3972e6);\n\n        private final double mass;\n        private final double radius;\n\n        Planet(double mass, double radius) {\n            this.mass = mass;\n            this.radius = radius;\n        }\n\n        double surfaceGravity() {\n            final double G = 6.67300E-11;\n            return G * mass / (radius * radius);\n        }\n\n        double surfaceWeight(double otherMass) {\n            return otherMass * surfaceGravity();\n        }\n    }\n\n    public static void main(String[] args) {\n        Day today = Day.WEDNESDAY;\n        System.out.println(today + \" is weekend? \" + today.isWeekend());\n\n        for (Day d : Day.values()) {\n            System.out.println(d.name() + \" ordinal=\" + d.ordinal());\n        }\n\n        double earthWeight = 75.0;\n        double mass = earthWeight / Planet.EARTH.surfaceGravity();\n        for (Planet p : Planet.values()) {\n            System.out.printf(\"Your weight on %s: %.2f N%n\", p, p.surfaceWeight(mass));\n        }\n    }\n}", "Java enum — simple, with fields/methods, iteration."),
                "rust" => ("#[derive(Debug)]\nenum Color {\n    Red,\n    Green,\n    Blue,\n    Custom(u8, u8, u8),\n}\n\nimpl Color {\n    fn hex(&self) -> String {\n        match self {\n            Color::Red => \"#FF0000\".into(),\n            Color::Green => \"#00FF00\".into(),\n            Color::Blue => \"#0000FF\".into(),\n            Color::Custom(r, g, b) => format!(\"#{:02X}{:02X}{:02X}\", r, g, b),\n        }\n    }\n}\n\n#[derive(Debug)]\nenum Shape {\n    Circle(f64),\n    Rectangle(f64, f64),\n    Triangle(f64, f64),\n}\n\nimpl Shape {\n    fn area(&self) -> f64 {\n        match self {\n            Shape::Circle(r) => std::f64::consts::PI * r * r,\n            Shape::Rectangle(w, h) => w * h,\n            Shape::Triangle(b, h) => 0.5 * b * h,\n        }\n    }\n}\n\nfn main() {\n    let colors = [Color::Red, Color::Green, Color::Custom(128, 0, 255)];\n    for c in &colors {\n        println!(\"{:?} -> {}\", c, c.hex());\n    }\n\n    let shapes = [Shape::Circle(5.0), Shape::Rectangle(4.0, 6.0), Shape::Triangle(3.0, 8.0)];\n    for s in &shapes {\n        println!(\"{:?} area = {:.2}\", s, s.area());\n    }\n}", "Rust enum — variants with data, methods, pattern matching."),
                _ => ("from enum import Enum, auto\n\nclass Color(Enum):\n    RED = auto()\n    GREEN = auto()\n    BLUE = auto()\n    YELLOW = auto()\n\n    @property\n    def hex(self):\n        mapping = {Color.RED: '#FF0000', Color.GREEN: '#00FF00',\n                   Color.BLUE: '#0000FF', Color.YELLOW: '#FFFF00'}\n        return mapping.get(self, '#000000')\n\nclass Status(Enum):\n    PENDING = 'pending'\n    ACTIVE = 'active'\n    CLOSED = 'closed'\n\n# Usage\nfor color in Color:\n    print(f'{color.name}: {color.hex}')\n\nprint(Color.RED == Color.RED)      # True\nprint(Color.RED == Color.BLUE)     # False\nprint(Color['GREEN'])              # Color.GREEN\nprint(Status.ACTIVE.value)         # 'active'\n\n# Match (Python 3.10+)\nstatus = Status.PENDING\nmatch status:\n    case Status.PENDING: print('Waiting...')\n    case Status.ACTIVE:  print('Running!')\n    case Status.CLOSED:  print('Done.')", "Python Enum — auto values, properties, matching."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if is_interface_req {
            let (code, note) = match lang {
                "java" => ("interface Shape {\n    double area();\n    double perimeter();\n    default String describe() {\n        return String.format(\"%s: area=%.2f, perimeter=%.2f\",\n            getClass().getSimpleName(), area(), perimeter());\n    }\n}\n\ninterface Drawable {\n    void draw();\n}\n\nclass Circle implements Shape, Drawable {\n    private double radius;\n    public Circle(double radius) { this.radius = radius; }\n    public double area() { return Math.PI * radius * radius; }\n    public double perimeter() { return 2 * Math.PI * radius; }\n    public void draw() { System.out.println(\"Drawing circle with radius \" + radius); }\n}\n\nclass Rectangle implements Shape, Drawable {\n    private double width, height;\n    public Rectangle(double w, double h) { width = w; height = h; }\n    public double area() { return width * height; }\n    public double perimeter() { return 2 * (width + height); }\n    public void draw() { System.out.println(\"Drawing \" + width + \"x\" + height + \" rectangle\"); }\n}\n\npublic class InterfaceDemo {\n    public static void main(String[] args) {\n        Shape[] shapes = { new Circle(5), new Rectangle(4, 6) };\n        for (Shape s : shapes) {\n            System.out.println(s.describe());\n            ((Drawable) s).draw();\n        }\n    }\n}", "Java interfaces — default methods, multiple implementation, polymorphism."),
                "rust" => ("trait Shape {\n    fn area(&self) -> f64;\n    fn perimeter(&self) -> f64;\n    fn describe(&self) -> String {\n        format!(\"area={:.2}, perimeter={:.2}\", self.area(), self.perimeter())\n    }\n}\n\nstruct Circle { radius: f64 }\nstruct Rectangle { width: f64, height: f64 }\n\nimpl Shape for Circle {\n    fn area(&self) -> f64 { std::f64::consts::PI * self.radius * self.radius }\n    fn perimeter(&self) -> f64 { 2.0 * std::f64::consts::PI * self.radius }\n}\n\nimpl Shape for Rectangle {\n    fn area(&self) -> f64 { self.width * self.height }\n    fn perimeter(&self) -> f64 { 2.0 * (self.width + self.height) }\n}\n\nfn print_shape(s: &dyn Shape) {\n    println!(\"{}\", s.describe());\n}\n\nfn main() {\n    let shapes: Vec<Box<dyn Shape>> = vec![\n        Box::new(Circle { radius: 5.0 }),\n        Box::new(Rectangle { width: 4.0, height: 6.0 }),\n    ];\n    for s in &shapes {\n        print_shape(s.as_ref());\n    }\n}", "Rust traits (interfaces) — default methods, dynamic dispatch."),
                "typescript" => ("interface Shape {\n    area(): number;\n    perimeter(): number;\n}\n\ninterface Printable {\n    toString(): string;\n}\n\nclass Circle implements Shape, Printable {\n    constructor(private radius: number) {}\n    area(): number { return Math.PI * this.radius ** 2; }\n    perimeter(): number { return 2 * Math.PI * this.radius; }\n    toString(): string { return `Circle(r=${this.radius})`; }\n}\n\nclass Rect implements Shape, Printable {\n    constructor(private w: number, private h: number) {}\n    area(): number { return this.w * this.h; }\n    perimeter(): number { return 2 * (this.w + this.h); }\n    toString(): string { return `Rect(${this.w}x${this.h})`; }\n}\n\nconst shapes: Shape[] = [new Circle(5), new Rect(4, 6)];\nfor (const s of shapes) {\n    console.log(`${s}: area=${s.area().toFixed(2)}, perimeter=${s.perimeter().toFixed(2)}`);\n}", "TypeScript interfaces — multiple interfaces, implementations."),
                _ => ("from abc import ABC, abstractmethod\nimport math\n\nclass Shape(ABC):\n    @abstractmethod\n    def area(self) -> float: ...\n    @abstractmethod\n    def perimeter(self) -> float: ...\n    def describe(self) -> str:\n        return f'{type(self).__name__}: area={self.area():.2f}, perimeter={self.perimeter():.2f}'\n\nclass Circle(Shape):\n    def __init__(self, radius: float): self.radius = radius\n    def area(self) -> float: return math.pi * self.radius ** 2\n    def perimeter(self) -> float: return 2 * math.pi * self.radius\n\nclass Rectangle(Shape):\n    def __init__(self, w: float, h: float): self.w, self.h = w, h\n    def area(self) -> float: return self.w * self.h\n    def perimeter(self) -> float: return 2 * (self.w + self.h)\n\nshapes: list[Shape] = [Circle(5), Rectangle(4, 6)]\nfor s in shapes:\n    print(s.describe())", "Python ABC (interface pattern) — abstract methods, polymorphism."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // struct
        let (code, note) = match lang {
            "rust" => ("#[derive(Debug, Clone)]\nstruct Point {\n    x: f64,\n    y: f64,\n}\n\nimpl Point {\n    fn new(x: f64, y: f64) -> Self { Point { x, y } }\n    fn distance(&self, other: &Point) -> f64 {\n        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()\n    }\n}\n\nimpl std::fmt::Display for Point {\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n        write!(f, \"({}, {})\", self.x, self.y)\n    }\n}\n\n#[derive(Debug)]\nstruct Rectangle {\n    origin: Point,\n    width: f64,\n    height: f64,\n}\n\nimpl Rectangle {\n    fn area(&self) -> f64 { self.width * self.height }\n    fn contains(&self, p: &Point) -> bool {\n        p.x >= self.origin.x && p.x <= self.origin.x + self.width\n        && p.y >= self.origin.y && p.y <= self.origin.y + self.height\n    }\n}\n\nfn main() {\n    let a = Point::new(1.0, 2.0);\n    let b = Point::new(4.0, 6.0);\n    println!(\"{} to {} = {:.2}\", a, b, a.distance(&b));\n\n    let rect = Rectangle { origin: Point::new(0.0, 0.0), width: 10.0, height: 5.0 };\n    println!(\"Area: {:.2}\", rect.area());\n    println!(\"Contains {}: {}\", a, rect.contains(&a));\n}", "Rust structs — methods, Display trait, composition."),
            "c" => ("#include <stdio.h>\n#include <math.h>\n#include <string.h>\n\ntypedef struct {\n    double x, y;\n} Point;\n\ntypedef struct {\n    char name[50];\n    int age;\n    double gpa;\n} Student;\n\ndouble distance(Point a, Point b) {\n    return sqrt((a.x-b.x)*(a.x-b.x) + (a.y-b.y)*(a.y-b.y));\n}\n\nvoid print_student(const Student* s) {\n    printf(\"%s (age %d, GPA %.2f)\\n\", s->name, s->age, s->gpa);\n}\n\nint main() {\n    Point a = {1.0, 2.0};\n    Point b = {4.0, 6.0};\n    printf(\"Distance: %.2f\\n\", distance(a, b));\n\n    Student students[] = {\n        {\"Arun\", 22, 3.8},\n        {\"Kala\", 20, 3.9},\n    };\n    for (int i = 0; i < 2; i++) {\n        print_student(&students[i]);\n    }\n    return 0;\n}", "C structs — typedef, functions, array of structs."),
            _ => ("from dataclasses import dataclass\nimport math\n\n@dataclass\nclass Point:\n    x: float\n    y: float\n\n    def distance(self, other: 'Point') -> float:\n        return math.sqrt((self.x - other.x)**2 + (self.y - other.y)**2)\n\n@dataclass\nclass Student:\n    name: str\n    age: int\n    gpa: float = 0.0\n\n    def is_honors(self) -> bool:\n        return self.gpa >= 3.5\n\na = Point(1.0, 2.0)\nb = Point(4.0, 6.0)\nprint(f'{a} to {b} = {a.distance(b):.2f}')\n\nstudents = [Student('Arun', 22, 3.8), Student('Kala', 20, 3.9)]\nfor s in students:\n    print(f'{s.name}: honors={s.is_honors()}')", "Python dataclass (struct-like) — fields, methods, defaults."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── API / HTTP request ───────────────────────────────────────────────────
    let is_http_req = q.contains("api") || q.contains("http request") || q.contains("http call")
        || q.contains("fetch url") || q.contains("get request") || q.contains("post request")
        || q.contains("rest api") || q.contains("web request") || q.contains("urllib");
    if is_http_req && !q.contains("server") && !q.contains("flask") && !q.contains("express") && !q.contains("fastapi") {
        let (code, note) = match lang {
            "java" => ("import java.net.http.HttpClient;\nimport java.net.http.HttpRequest;\nimport java.net.http.HttpResponse;\nimport java.net.URI;\n\npublic class ApiRequest {\n    public static void main(String[] args) throws Exception {\n        HttpClient client = HttpClient.newHttpClient();\n\n        // GET request\n        HttpRequest getReq = HttpRequest.newBuilder()\n            .uri(URI.create(\"https://jsonplaceholder.typicode.com/posts/1\"))\n            .GET()\n            .build();\n        HttpResponse<String> getResp = client.send(getReq, HttpResponse.BodyHandlers.ofString());\n        System.out.println(\"GET Status: \" + getResp.statusCode());\n        System.out.println(\"Body: \" + getResp.body().substring(0, 100) + \"...\");\n\n        // POST request\n        String json = \"{\\\"title\\\":\\\"Hello\\\",\\\"body\\\":\\\"World\\\",\\\"userId\\\":1}\";\n        HttpRequest postReq = HttpRequest.newBuilder()\n            .uri(URI.create(\"https://jsonplaceholder.typicode.com/posts\"))\n            .header(\"Content-Type\", \"application/json\")\n            .POST(HttpRequest.BodyPublishers.ofString(json))\n            .build();\n        HttpResponse<String> postResp = client.send(postReq, HttpResponse.BodyHandlers.ofString());\n        System.out.println(\"\\nPOST Status: \" + postResp.statusCode());\n        System.out.println(\"Response: \" + postResp.body());\n    }\n}", "Java HTTP client (java.net.http) — GET + POST, JSON."),
            "javascript" | "typescript" => ("// Node.js / Browser fetch API\nasync function apiDemo() {\n    // GET request\n    const getResp = await fetch('https://jsonplaceholder.typicode.com/posts/1');\n    const post = await getResp.json();\n    console.log('GET:', post.title);\n\n    // POST request\n    const postResp = await fetch('https://jsonplaceholder.typicode.com/posts', {\n        method: 'POST',\n        headers: { 'Content-Type': 'application/json' },\n        body: JSON.stringify({ title: 'Hello', body: 'World', userId: 1 }),\n    });\n    const created = await postResp.json();\n    console.log('POST:', created);\n\n    // Error handling\n    try {\n        const resp = await fetch('https://jsonplaceholder.typicode.com/posts/9999');\n        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);\n        const data = await resp.json();\n    } catch (err) {\n        console.error('Error:', err.message);\n    }\n}\n\napiDemo();", "JavaScript fetch API — GET, POST, error handling."),
            _ => ("import urllib.request\nimport json\n\n# GET request\nurl = 'https://jsonplaceholder.typicode.com/posts/1'\nwith urllib.request.urlopen(url) as resp:\n    data = json.loads(resp.read().decode())\n    print(f'GET: {data[\"title\"]}')\n    print(f'Status: {resp.status}')\n\n# POST request\npost_data = json.dumps({'title': 'Hello', 'body': 'World', 'userId': 1}).encode()\nreq = urllib.request.Request(\n    'https://jsonplaceholder.typicode.com/posts',\n    data=post_data,\n    headers={'Content-Type': 'application/json'},\n    method='POST'\n)\nwith urllib.request.urlopen(req) as resp:\n    result = json.loads(resp.read().decode())\n    print(f'POST: {result}')\n\n# Error handling\ntry:\n    urllib.request.urlopen('https://httpstat.us/404')\nexcept urllib.error.HTTPError as e:\n    print(f'Error: {e.code} {e.reason}')\nexcept urllib.error.URLError as e:\n    print(f'Connection error: {e.reason}')", "Python HTTP requests — GET, POST, error handling (no pip install needed)."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Multithreading / concurrency / async ─────────────────────────────────
    let is_thread_req = q.contains("thread") || q.contains("threading") || q.contains("multithread")
        || q.contains("concurren") || q.contains("parallel")
        || (q.contains("async") && (q.contains("program") || q.contains("example") || q.contains("code")));
    if is_thread_req {
        let (code, note) = match lang {
            "java" => ("import java.util.concurrent.*;\nimport java.util.concurrent.atomic.AtomicInteger;\n\npublic class ThreadDemo {\n    static AtomicInteger counter = new AtomicInteger(0);\n\n    public static void main(String[] args) throws Exception {\n        // 1. Thread with Runnable\n        Thread t1 = new Thread(() -> {\n            for (int i = 0; i < 5; i++) {\n                System.out.println(\"Thread-1: \" + counter.incrementAndGet());\n                try { Thread.sleep(100); } catch (InterruptedException e) { break; }\n            }\n        });\n\n        // 2. Thread subclass\n        Thread t2 = new Thread(() -> {\n            for (int i = 0; i < 5; i++) {\n                System.out.println(\"Thread-2: \" + counter.incrementAndGet());\n                try { Thread.sleep(150); } catch (InterruptedException e) { break; }\n            }\n        });\n\n        t1.start();\n        t2.start();\n        t1.join();\n        t2.join();\n        System.out.println(\"Final counter: \" + counter.get());\n\n        // 3. ExecutorService (thread pool)\n        ExecutorService pool = Executors.newFixedThreadPool(3);\n        for (int i = 1; i <= 5; i++) {\n            final int task = i;\n            pool.submit(() -> {\n                System.out.println(\"Task \" + task + \" on \" + Thread.currentThread().getName());\n            });\n        }\n        pool.shutdown();\n        pool.awaitTermination(5, TimeUnit.SECONDS);\n\n        // 4. Future (async result)\n        ExecutorService exec = Executors.newSingleThreadExecutor();\n        Future<Integer> future = exec.submit(() -> {\n            Thread.sleep(500);\n            return 42;\n        });\n        System.out.println(\"Future result: \" + future.get());\n        exec.shutdown();\n    }\n}", "Java threading — Runnable, join, ExecutorService, Future."),
            "rust" => ("use std::sync::{Arc, Mutex, atomic::{AtomicI32, Ordering}};\nuse std::thread;\n\nfn main() {\n    // 1. Simple threads\n    let counter = Arc::new(AtomicI32::new(0));\n    let mut handles = vec![];\n    for id in 0..4 {\n        let c = Arc::clone(&counter);\n        handles.push(thread::spawn(move || {\n            for _ in 0..5 {\n                let val = c.fetch_add(1, Ordering::SeqCst);\n                println!(\"Thread {}: {}\", id, val);\n                thread::sleep(std::time::Duration::from_millis(50));\n            }\n        }));\n    }\n    for h in handles { h.join().unwrap(); }\n    println!(\"Final: {}\", counter.load(Ordering::SeqCst));\n\n    // 2. Shared mutable state with Mutex\n    let data = Arc::new(Mutex::new(vec![]));\n    let mut workers = vec![];\n    for i in 0..4 {\n        let d = Arc::clone(&data);\n        workers.push(thread::spawn(move || {\n            let mut v = d.lock().unwrap();\n            v.push(i * 10);\n        }));\n    }\n    for w in workers { w.join().unwrap(); }\n    println!(\"Data: {:?}\", data.lock().unwrap());\n\n    // 3. Scoped threads (no Arc needed)\n    let mut results = vec![0; 4];\n    thread::scope(|s| {\n        for (i, slot) in results.iter_mut().enumerate() {\n            s.spawn(move || { *slot = i * i; });\n        }\n    });\n    println!(\"Results: {:?}\", results);\n}", "Rust threading — spawn, Arc+AtomicI32, Mutex, scoped threads."),
            _ => ("import threading\nimport concurrent.futures\nimport time\n\n# 1. Basic threading\ncounter = 0\nlock = threading.Lock()\n\ndef worker(name, count):\n    global counter\n    for _ in range(count):\n        with lock:\n            counter += 1\n            print(f'{name}: {counter}')\n        time.sleep(0.05)\n\nt1 = threading.Thread(target=worker, args=('Thread-1', 5))\nt2 = threading.Thread(target=worker, args=('Thread-2', 5))\nt1.start(); t2.start()\nt1.join(); t2.join()\nprint(f'Final counter: {counter}')\n\n# 2. ThreadPoolExecutor\ndef compute(n):\n    time.sleep(0.1)\n    return n * n\n\nwith concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:\n    futures = [pool.submit(compute, i) for i in range(10)]\n    results = [f.result() for f in concurrent.futures.as_completed(futures)]\n    print(f'Pool results: {sorted(results)}')\n\n# 3. Producer-consumer with Queue\nimport queue\n\nq = queue.Queue(maxsize=5)\n\ndef producer():\n    for i in range(10):\n        q.put(i)\n        print(f'Produced: {i}')\n    q.put(None)  # sentinel\n\ndef consumer():\n    while True:\n        item = q.get()\n        if item is None: break\n        print(f'Consumed: {item}')\n\nthreading.Thread(target=producer).start()\nthreading.Thread(target=consumer).start()", "Python threading — locks, ThreadPoolExecutor, producer-consumer."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── File I/O (read, write, file handling) ────────────────────────────────
    let is_file_io = (q.contains("file") && (q.contains("read") || q.contains("write") || q.contains("handling") || q.contains("i/o") || q.contains("io")))
        || q.contains("read file") || q.contains("write file");
    if is_file_io {
        let (code, note) = match lang {
            "java" => ("import java.io.*;\nimport java.nio.file.*;\nimport java.util.List;\n\npublic class FileIODemo {\n    public static void main(String[] args) throws IOException {\n        String filename = \"demo.txt\";\n\n        // Write to file\n        Files.writeString(Path.of(filename), \"Hello, World!\\nLine 2\\nLine 3\\n\");\n        System.out.println(\"File written.\");\n\n        // Read entire file\n        String content = Files.readString(Path.of(filename));\n        System.out.println(\"Content:\\n\" + content);\n\n        // Read line by line\n        List<String> lines = Files.readAllLines(Path.of(filename));\n        for (int i = 0; i < lines.size(); i++) {\n            System.out.println((i + 1) + \": \" + lines.get(i));\n        }\n\n        // Append to file\n        Files.writeString(Path.of(filename), \"Appended line\\n\",\n            StandardOpenOption.APPEND);\n\n        // BufferedReader (for large files)\n        try (BufferedReader br = new BufferedReader(new FileReader(filename))) {\n            String line;\n            while ((line = br.readLine()) != null) {\n                System.out.println(\">> \" + line);\n            }\n        }\n\n        // Check file exists, size, delete\n        Path path = Path.of(filename);\n        System.out.println(\"Exists: \" + Files.exists(path));\n        System.out.println(\"Size: \" + Files.size(path) + \" bytes\");\n        Files.delete(path);\n        System.out.println(\"Deleted.\");\n    }\n}", "Java file I/O — read, write, append, BufferedReader, NIO."),
            _ => ("from pathlib import Path\n\nfilename = 'demo.txt'\n\n# Write\nPath(filename).write_text('Hello, World!\\nLine 2\\nLine 3\\n')\nprint('File written.')\n\n# Read entire file\ncontent = Path(filename).read_text()\nprint(f'Content:\\n{content}')\n\n# Read line by line\nwith open(filename) as f:\n    for i, line in enumerate(f, 1):\n        print(f'{i}: {line.rstrip()}')\n\n# Append\nwith open(filename, 'a') as f:\n    f.write('Appended line\\n')\n\n# Check exists, size\np = Path(filename)\nprint(f'Exists: {p.exists()}')\nprint(f'Size: {p.stat().st_size} bytes')\n\n# Read JSON\nimport json\ndata = {'name': 'Arun', 'scores': [95, 87]}\nPath('data.json').write_text(json.dumps(data, indent=2))\nloaded = json.loads(Path('data.json').read_text())\nprint(f'JSON: {loaded}')\n\n# Cleanup\np.unlink()\nPath('data.json').unlink()\nprint('Cleaned up.')", "Python file I/O — read, write, append, JSON, pathlib."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Dictionary / HashMap / Map ───────────────────────────────────────────
    let is_dict_req = q.contains("dictionary") || q.contains("hashmap") || q.contains("hash map")
        || (q.contains("map") && (q.contains("program") || q.contains("example")) && !q.contains("map filter") && !q.contains("map reduce"));
    if is_dict_req {
        let (code, note) = match lang {
            "java" => ("import java.util.*;\n\npublic class HashMapDemo {\n    public static void main(String[] args) {\n        // Create and populate\n        HashMap<String, Integer> scores = new HashMap<>();\n        scores.put(\"Arun\", 95);\n        scores.put(\"Kala\", 88);\n        scores.put(\"Dev\", 92);\n\n        // Access\n        System.out.println(\"Arun: \" + scores.get(\"Arun\"));\n        System.out.println(\"Contains Dev: \" + scores.containsKey(\"Dev\"));\n        System.out.println(\"Default: \" + scores.getOrDefault(\"Unknown\", 0));\n\n        // Iterate\n        for (Map.Entry<String, Integer> e : scores.entrySet()) {\n            System.out.println(e.getKey() + \" -> \" + e.getValue());\n        }\n\n        // Modify\n        scores.put(\"Arun\", 97);  // update\n        scores.remove(\"Dev\");    // remove\n        scores.putIfAbsent(\"New\", 80);\n\n        // Size and clear\n        System.out.println(\"Size: \" + scores.size());\n        System.out.println(\"Keys: \" + scores.keySet());\n        System.out.println(\"Values: \" + scores.values());\n    }\n}", "Java HashMap — put, get, iterate, modify, keys/values."),
            _ => ("# Create dictionary\nscores = {'Arun': 95, 'Kala': 88, 'Dev': 92}\n\n# Access\nprint(scores['Arun'])           # 95\nprint(scores.get('Unknown', 0)) # 0 (default)\n\n# Iterate\nfor name, score in scores.items():\n    print(f'{name}: {score}')\n\n# Modify\nscores['Arun'] = 97  # update\ndel scores['Dev']    # remove\nscores.setdefault('New', 80)\n\n# Comprehension\nsquared = {k: v**2 for k, v in scores.items()}\nprint(squared)\n\n# Nested\nstudents = {\n    'Arun': {'age': 22, 'courses': ['math', 'cs']},\n    'Kala': {'age': 20, 'courses': ['ai', 'ml']},\n}\nfor name, info in students.items():\n    print(f\"{name} (age {info['age']}): {', '.join(info['courses'])}\")\n\n# Count occurrences\nfrom collections import Counter\nwords = 'the cat sat on the mat the cat'.split()\ncounts = Counter(words)\nprint(counts.most_common(3))", "Python dict — access, iterate, comprehension, nested, Counter."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── String manipulation (general) — reverse/palindrome handled separately below
    let has_reverse = q.contains("reverse");
    let is_string_req = !has_reverse && (q.contains("string") && (q.contains("program") || q.contains("example") || q.contains("manipulat") || q.contains("operation")));
    if is_string_req {
        let (code, note) = match lang {
            "java" => ("public class StringDemo {\n    public static void main(String[] args) {\n        String s = \"Hello, World!\";\n\n        // Basic operations\n        System.out.println(\"Length: \" + s.length());\n        System.out.println(\"Upper: \" + s.toUpperCase());\n        System.out.println(\"Lower: \" + s.toLowerCase());\n        System.out.println(\"Char at 0: \" + s.charAt(0));\n        System.out.println(\"Substring: \" + s.substring(0, 5));\n        System.out.println(\"Contains 'World': \" + s.contains(\"World\"));\n        System.out.println(\"Replace: \" + s.replace(\"World\", \"Java\"));\n        System.out.println(\"Trim: \" + \"  hello  \".trim());\n\n        // Reverse a string\n        String reversed = new StringBuilder(s).reverse().toString();\n        System.out.println(\"Reversed: \" + reversed);\n\n        // Check palindrome\n        String word = \"racecar\";\n        boolean isPalindrome = word.equals(new StringBuilder(word).reverse().toString());\n        System.out.println(word + \" is palindrome: \" + isPalindrome);\n\n        // Split and join\n        String csv = \"apple,banana,cherry\";\n        String[] parts = csv.split(\",\");\n        String joined = String.join(\" | \", parts);\n        System.out.println(\"Joined: \" + joined);\n\n        // StringBuilder for concatenation\n        StringBuilder sb = new StringBuilder();\n        for (int i = 1; i <= 5; i++) sb.append(i).append(\" \");\n        System.out.println(\"Built: \" + sb.toString().trim());\n    }\n}", "Java strings — reverse, palindrome, split, join, StringBuilder."),
            _ => ("s = 'Hello, World!'\n\n# Basic operations\nprint(f'Length: {len(s)}')\nprint(f'Upper: {s.upper()}')\nprint(f'Lower: {s.lower()}')\nprint(f'Substring: {s[:5]}')\nprint(f'Contains: {\"World\" in s}')\nprint(f'Replace: {s.replace(\"World\", \"Python\")}')\nprint(f'Strip: {\"  hello  \".strip()}')\n\n# Reverse\nreversed_s = s[::-1]\nprint(f'Reversed: {reversed_s}')\n\n# Palindrome check\nword = 'racecar'\nprint(f'{word} is palindrome: {word == word[::-1]}')\n\n# Split and join\ncsv = 'apple,banana,cherry'\nparts = csv.split(',')\njoined = ' | '.join(parts)\nprint(f'Joined: {joined}')\n\n# f-string formatting\nname, age = 'Arun', 28\nprint(f'{name} is {age} years old')\nprint(f'{3.14159:.2f}')  # 3.14\nprint(f'{42:08d}')       # 00000042", "Python strings — reverse, palindrome, split, join, f-strings."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Calculator / basic math program ──────────────────────────────────────
    let is_calc_req = q.contains("calculator") || q.contains("calc program")
        || (q.contains("math") && (q.contains("program") || q.contains("basic")))
        || (q.contains("add subtract") || q.contains("arithmetic"));
    if is_calc_req {
        let (code, note) = match lang {
            "java" => ("import java.util.Scanner;\n\npublic class Calculator {\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n        System.out.println(\"Simple Calculator\");\n        System.out.println(\"Operations: +  -  *  /  %\");\n\n        while (true) {\n            System.out.print(\"\\nEnter expression (e.g. 5 + 3) or 'quit': \");\n            String line = sc.nextLine().trim();\n            if (line.equalsIgnoreCase(\"quit\")) break;\n\n            String[] parts = line.split(\"\\\\s+\");\n            if (parts.length != 3) {\n                System.out.println(\"Format: <number> <op> <number>\");\n                continue;\n            }\n            try {\n                double a = Double.parseDouble(parts[0]);\n                double b = Double.parseDouble(parts[2]);\n                double result;\n                switch (parts[1]) {\n                    case \"+\": result = a + b; break;\n                    case \"-\": result = a - b; break;\n                    case \"*\": result = a * b; break;\n                    case \"/\":\n                        if (b == 0) { System.out.println(\"Error: division by zero\"); continue; }\n                        result = a / b; break;\n                    case \"%\": result = a % b; break;\n                    default: System.out.println(\"Unknown operator: \" + parts[1]); continue;\n                }\n                System.out.printf(\"= %.4f%n\", result);\n            } catch (NumberFormatException e) {\n                System.out.println(\"Invalid number.\");\n            }\n        }\n        System.out.println(\"Goodbye!\");\n    }\n}", "Java calculator — interactive, all operators, error handling."),
            _ => ("def calculator():\n    print('Simple Calculator')\n    print('Operations: +  -  *  /  %  **')\n    while True:\n        expr = input('\\n> ').strip()\n        if expr.lower() in ('quit', 'exit', 'q'): break\n        try:\n            parts = expr.split()\n            if len(parts) != 3:\n                print('Format: <number> <op> <number>')\n                continue\n            a, op, b = float(parts[0]), parts[1], float(parts[2])\n            ops = {'+': a+b, '-': a-b, '*': a*b, '/': a/b if b else float('inf'),\n                   '%': a%b, '**': a**b}\n            if op in ops:\n                print(f'= {ops[op]}')\n            else:\n                print(f'Unknown operator: {op}')\n        except (ValueError, ZeroDivisionError) as e:\n            print(f'Error: {e}')\n    print('Goodbye!')\n\ncalculator()", "Python calculator — interactive, all operators."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Prime number / check prime / generate primes ─────────────────────────
    let is_prime_req = q.contains("prime");
    if is_prime_req {
        let (code, note) = match lang {
            "java" => ("public class PrimeNumbers {\n    static boolean isPrime(int n) {\n        if (n < 2) return false;\n        if (n < 4) return true;\n        if (n % 2 == 0 || n % 3 == 0) return false;\n        for (int i = 5; i * i <= n; i += 6) {\n            if (n % i == 0 || n % (i + 2) == 0) return false;\n        }\n        return true;\n    }\n\n    static void sieve(int limit) {\n        boolean[] notPrime = new boolean[limit + 1];\n        for (int i = 2; i * i <= limit; i++) {\n            if (!notPrime[i]) {\n                for (int j = i * i; j <= limit; j += i) notPrime[j] = true;\n            }\n        }\n        System.out.print(\"Primes up to \" + limit + \": \");\n        for (int i = 2; i <= limit; i++) {\n            if (!notPrime[i]) System.out.print(i + \" \");\n        }\n        System.out.println();\n    }\n\n    public static void main(String[] args) {\n        System.out.println(\"7 is prime: \" + isPrime(7));\n        System.out.println(\"10 is prime: \" + isPrime(10));\n        System.out.println(\"97 is prime: \" + isPrime(97));\n        sieve(50);\n    }\n}", "Java prime numbers — isPrime, Sieve of Eratosthenes."),
            _ => ("def is_prime(n: int) -> bool:\n    if n < 2: return False\n    if n < 4: return True\n    if n % 2 == 0 or n % 3 == 0: return False\n    i = 5\n    while i * i <= n:\n        if n % i == 0 or n % (i + 2) == 0: return False\n        i += 6\n    return True\n\ndef sieve(limit: int) -> list[int]:\n    is_p = [True] * (limit + 1)\n    is_p[0] = is_p[1] = False\n    for i in range(2, int(limit**0.5) + 1):\n        if is_p[i]:\n            for j in range(i*i, limit + 1, i): is_p[j] = False\n    return [i for i, p in enumerate(is_p) if p]\n\nprint(f'7 is prime: {is_prime(7)}')\nprint(f'10 is prime: {is_prime(10)}')\nprint(f'Primes up to 50: {sieve(50)}')", "Python primes — is_prime, Sieve of Eratosthenes."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Regex / validation ───────────────────────────────────────────────────
    let is_regex_req = q.contains("regex") || q.contains("regular expression") || q.contains("validation")
        || q.contains("validate email") || q.contains("validate phone");
    if is_regex_req {
        let (code, note) = match lang {
            "java" => ("import java.util.regex.*;\n\npublic class RegexDemo {\n    static boolean isValidEmail(String email) {\n        return email.matches(\"^[\\\\w.+-]+@[\\\\w-]+\\\\.[a-zA-Z]{2,}$\");\n    }\n    static boolean isValidPhone(String phone) {\n        return phone.matches(\"^\\\\+?\\\\d{1,3}[-.\\\\s]?\\\\(?\\\\d{1,4}\\\\)?[-.\\\\s]?\\\\d{3,4}[-.\\\\s]?\\\\d{3,4}$\");\n    }\n\n    public static void main(String[] args) {\n        // Match & validate\n        System.out.println(isValidEmail(\"user@example.com\"));  // true\n        System.out.println(isValidEmail(\"bad@\"));              // false\n        System.out.println(isValidPhone(\"+1-555-123-4567\"));   // true\n\n        // Find all matches\n        String text = \"Call 555-1234 or 555-5678 today\";\n        Matcher m = Pattern.compile(\"\\\\d{3}-\\\\d{4}\").matcher(text);\n        while (m.find()) System.out.println(\"Found: \" + m.group());\n\n        // Replace\n        String cleaned = text.replaceAll(\"\\\\d{3}-\\\\d{4}\", \"XXX-XXXX\");\n        System.out.println(cleaned);\n\n        // Groups\n        Matcher dateMatcher = Pattern.compile(\"(\\\\d{4})-(\\\\d{2})-(\\\\d{2})\")\n            .matcher(\"Today is 2025-01-15\");\n        if (dateMatcher.find()) {\n            System.out.printf(\"Year: %s, Month: %s, Day: %s%n\",\n                dateMatcher.group(1), dateMatcher.group(2), dateMatcher.group(3));\n        }\n    }\n}", "Java regex — validate, find, replace, groups."),
            _ => ("import re\n\ndef is_valid_email(email: str) -> bool:\n    return bool(re.match(r'^[\\w.+-]+@[\\w-]+\\.[a-zA-Z]{2,}$', email))\n\ndef is_valid_phone(phone: str) -> bool:\n    return bool(re.match(r'^\\+?\\d{1,3}[-.\\s]?\\(?\\d{1,4}\\)?[-.\\s]?\\d{3,4}[-.\\s]?\\d{3,4}$', phone))\n\n# Validate\nprint(is_valid_email('user@example.com'))   # True\nprint(is_valid_email('bad@'))               # False\nprint(is_valid_phone('+1-555-123-4567'))    # True\n\n# Find all matches\ntext = 'Call 555-1234 or 555-5678 today'\nphones = re.findall(r'\\d{3}-\\d{4}', text)\nprint(f'Phones: {phones}')\n\n# Replace\ncleaned = re.sub(r'\\d{3}-\\d{4}', 'XXX-XXXX', text)\nprint(cleaned)\n\n# Groups\nm = re.search(r'(\\d{4})-(\\d{2})-(\\d{2})', 'Today is 2025-01-15')\nif m:\n    print(f'Year: {m.group(1)}, Month: {m.group(2)}, Day: {m.group(3)}')\n\n# Split\nparts = re.split(r'[,;\\s]+', 'apple, banana; cherry  date')\nprint(parts)", "Python regex — validate, find, replace, groups, split."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Basic math programs (even/odd, swap, armstrong, area, temp, leap, power, random, gcd/lcm) ──
    let is_basic_math = q.contains("even odd") || q.contains("even or odd") || q.contains("odd even")
        || q.contains("swap") || q.contains("armstrong") || q.contains("area of")
        || q.contains("temperature") || q.contains("celsius") || q.contains("fahrenheit")
        || q.contains("leap year") || q.contains("power of") || q.contains("exponent")
        || q.contains("random number") || q.contains("gcd") || q.contains("lcm")
        || q.contains("greatest common") || q.contains("least common");
    if is_basic_math {
        if q.contains("gcd") || q.contains("lcm") || q.contains("greatest common") || q.contains("least common") {
            let (code, note) = match lang {
                "java" => ("public class GcdLcm {\n    static int gcd(int a, int b) {\n        while (b != 0) { int t = b; b = a % b; a = t; }\n        return a;\n    }\n    static int lcm(int a, int b) { return a / gcd(a, b) * b; }\n\n    public static void main(String[] args) {\n        int a = 36, b = 48;\n        System.out.println(\"GCD(\" + a + \", \" + b + \") = \" + gcd(a, b));\n        System.out.println(\"LCM(\" + a + \", \" + b + \") = \" + lcm(a, b));\n\n        // GCD of three numbers\n        int c = 60;\n        System.out.println(\"GCD(\" + a + \", \" + b + \", \" + c + \") = \" + gcd(gcd(a, b), c));\n    }\n}", "Java GCD & LCM — Euclidean algorithm."),
                _ => ("def gcd(a, b):\n    while b: a, b = b, a % b\n    return a\n\ndef lcm(a, b):\n    return a * b // gcd(a, b)\n\na, b = 36, 48\nprint(f'GCD({a}, {b}) = {gcd(a, b)}')\nprint(f'LCM({a}, {b}) = {lcm(a, b)}')\n\n# Also available in math module (Python 3.9+)\nimport math\nprint(f'math.gcd = {math.gcd(a, b)}')\nprint(f'math.lcm = {math.lcm(a, b)}')", "Python GCD & LCM — Euclidean algorithm + math module."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("armstrong") {
            let (code, note) = match lang {
                "java" => ("public class Armstrong {\n    static boolean isArmstrong(int n) {\n        int original = n, sum = 0, digits = String.valueOf(n).length();\n        while (n > 0) {\n            int d = n % 10;\n            sum += Math.pow(d, digits);\n            n /= 10;\n        }\n        return sum == original;\n    }\n\n    public static void main(String[] args) {\n        System.out.println(\"153 is Armstrong: \" + isArmstrong(153));\n        System.out.println(\"370 is Armstrong: \" + isArmstrong(370));\n        System.out.println(\"123 is Armstrong: \" + isArmstrong(123));\n\n        System.out.print(\"Armstrong numbers 1-1000: \");\n        for (int i = 1; i <= 1000; i++) {\n            if (isArmstrong(i)) System.out.print(i + \" \");\n        }\n        System.out.println();\n    }\n}", "Java Armstrong number — check + find all in range."),
                _ => ("def is_armstrong(n):\n    digits = len(str(n))\n    return sum(int(d)**digits for d in str(n)) == n\n\nprint(f'153 is Armstrong: {is_armstrong(153)}')\nprint(f'123 is Armstrong: {is_armstrong(123)}')\n\nprint('Armstrong numbers 1-1000:', [n for n in range(1, 1001) if is_armstrong(n)])", "Python Armstrong number."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("area") {
            let (code, note) = match lang {
                "java" => ("import java.util.Scanner;\n\npublic class AreaCalculator {\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n\n        // Circle\n        double radius = 5.0;\n        double circleArea = Math.PI * radius * radius;\n        System.out.printf(\"Circle (r=%.1f): area=%.2f, circumference=%.2f%n\",\n            radius, circleArea, 2 * Math.PI * radius);\n\n        // Rectangle\n        double w = 4.0, h = 6.0;\n        System.out.printf(\"Rectangle (%.1fx%.1f): area=%.2f, perimeter=%.2f%n\",\n            w, h, w * h, 2 * (w + h));\n\n        // Triangle\n        double base = 8.0, height = 5.0;\n        System.out.printf(\"Triangle (b=%.1f, h=%.1f): area=%.2f%n\",\n            base, height, 0.5 * base * height);\n\n        // Sphere\n        System.out.printf(\"Sphere (r=%.1f): volume=%.2f, surface=%.2f%n\",\n            radius, (4.0/3) * Math.PI * Math.pow(radius, 3), 4 * Math.PI * radius * radius);\n    }\n}", "Java area calculator — circle, rectangle, triangle, sphere."),
                _ => ("import math\n\nradius = 5.0\nprint(f'Circle (r={radius}): area={math.pi * radius**2:.2f}, circumference={2*math.pi*radius:.2f}')\n\nw, h = 4.0, 6.0\nprint(f'Rectangle ({w}x{h}): area={w*h:.2f}, perimeter={2*(w+h):.2f}')\n\nbase, height = 8.0, 5.0\nprint(f'Triangle (b={base}, h={height}): area={0.5*base*height:.2f}')\n\nprint(f'Sphere (r={radius}): volume={4/3*math.pi*radius**3:.2f}')", "Python area calculator — circle, rectangle, triangle, sphere."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("temperature") || q.contains("celsius") || q.contains("fahrenheit") {
            let (code, note) = match lang {
                "java" => ("import java.util.Scanner;\n\npublic class TempConverter {\n    static double celsiusToFahrenheit(double c) { return c * 9.0 / 5.0 + 32; }\n    static double fahrenheitToCelsius(double f) { return (f - 32) * 5.0 / 9.0; }\n    static double celsiusToKelvin(double c) { return c + 273.15; }\n\n    public static void main(String[] args) {\n        double c = 100.0;\n        System.out.printf(\"%.1f°C = %.1f°F = %.2fK%n\", c, celsiusToFahrenheit(c), celsiusToKelvin(c));\n\n        double f = 212.0;\n        System.out.printf(\"%.1f°F = %.1f°C%n\", f, fahrenheitToCelsius(f));\n\n        System.out.println(\"\\nConversion table:\");\n        for (int i = 0; i <= 100; i += 10) {\n            System.out.printf(\"%4d°C = %6.1f°F%n\", i, celsiusToFahrenheit(i));\n        }\n    }\n}", "Java temperature converter — C/F/K with table."),
                _ => ("def c_to_f(c): return c * 9/5 + 32\ndef f_to_c(f): return (f - 32) * 5/9\ndef c_to_k(c): return c + 273.15\n\nc = 100\nprint(f'{c}°C = {c_to_f(c):.1f}°F = {c_to_k(c):.2f}K')\nprint(f'212°F = {f_to_c(212):.1f}°C')\n\nprint('\\nConversion table:')\nfor i in range(0, 101, 10):\n    print(f'{i:4d}°C = {c_to_f(i):6.1f}°F')", "Python temperature converter — C/F/K."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("leap year") {
            let (code, note) = match lang {
                "java" => ("public class LeapYear {\n    static boolean isLeap(int year) {\n        return (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);\n    }\n\n    public static void main(String[] args) {\n        int[] years = {2000, 1900, 2024, 2023, 2100, 2400};\n        for (int y : years) {\n            System.out.println(y + \" is leap year: \" + isLeap(y));\n        }\n\n        System.out.print(\"Leap years 2000-2050: \");\n        for (int y = 2000; y <= 2050; y++) {\n            if (isLeap(y)) System.out.print(y + \" \");\n        }\n        System.out.println();\n    }\n}", "Java leap year — check + list range."),
                _ => ("def is_leap(year):\n    return (year % 4 == 0 and year % 100 != 0) or (year % 400 == 0)\n\nfor y in [2000, 1900, 2024, 2023, 2100, 2400]:\n    print(f'{y} is leap: {is_leap(y)}')\n\nprint('Leap years 2000-2050:', [y for y in range(2000, 2051) if is_leap(y)])", "Python leap year."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("random") {
            let (code, note) = match lang {
                "java" => ("import java.util.Random;\n\npublic class RandomDemo {\n    public static void main(String[] args) {\n        Random rng = new Random();\n\n        System.out.println(\"Random int: \" + rng.nextInt());\n        System.out.println(\"Random 1-100: \" + (rng.nextInt(100) + 1));\n        System.out.println(\"Random double: \" + rng.nextDouble());\n        System.out.println(\"Random boolean: \" + rng.nextBoolean());\n\n        // Roll dice 10 times\n        System.out.print(\"Dice rolls: \");\n        for (int i = 0; i < 10; i++) System.out.print((rng.nextInt(6) + 1) + \" \");\n        System.out.println();\n\n        // Shuffle array\n        int[] arr = {1, 2, 3, 4, 5};\n        for (int i = arr.length - 1; i > 0; i--) {\n            int j = rng.nextInt(i + 1);\n            int tmp = arr[i]; arr[i] = arr[j]; arr[j] = tmp;\n        }\n        System.out.print(\"Shuffled: \");\n        for (int v : arr) System.out.print(v + \" \");\n        System.out.println();\n    }\n}", "Java Random — int, range, double, dice, shuffle."),
                _ => ("import random\n\nprint('Random int 1-100:', random.randint(1, 100))\nprint('Random float:', random.random())\nprint('Random choice:', random.choice(['apple', 'banana', 'cherry']))\n\nnums = list(range(1, 6))\nrandom.shuffle(nums)\nprint('Shuffled:', nums)\n\nprint('Dice rolls:', [random.randint(1, 6) for _ in range(10)])\nprint('Sample 3:', random.sample(range(1, 50), 3))", "Python random — int, choice, shuffle, sample."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("power") || q.contains("exponent") {
            let (code, note) = match lang {
                "java" => ("public class PowerDemo {\n    static long power(int base, int exp) {\n        long result = 1;\n        for (int i = 0; i < exp; i++) result *= base;\n        return result;\n    }\n\n    static long powerFast(long base, int exp) {\n        long result = 1;\n        while (exp > 0) {\n            if (exp % 2 == 1) result *= base;\n            base *= base;\n            exp /= 2;\n        }\n        return result;\n    }\n\n    public static void main(String[] args) {\n        System.out.println(\"2^10 = \" + power(2, 10));\n        System.out.println(\"3^5  = \" + power(3, 5));\n        System.out.println(\"2^20 (fast) = \" + powerFast(2, 20));\n        System.out.println(\"Math.pow(2,10) = \" + (int)Math.pow(2, 10));\n    }\n}", "Java power — loop, fast exponentiation, Math.pow."),
                _ => ("def power(base, exp):\n    result = 1\n    for _ in range(exp): result *= base\n    return result\n\ndef power_fast(base, exp):\n    result = 1\n    while exp > 0:\n        if exp % 2 == 1: result *= base\n        base *= base\n        exp //= 2\n    return result\n\nprint(f'2^10 = {power(2, 10)}')\nprint(f'2^20 (fast) = {power_fast(2, 20)}')\nprint(f'Built-in: {2**10}')\nprint(f'pow(): {pow(2, 10)}')", "Python power — loop, fast exp, built-in."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("swap") {
            let (code, note) = match lang {
                "java" => ("public class SwapNumbers {\n    public static void main(String[] args) {\n        int a = 10, b = 20;\n        System.out.println(\"Before: a=\" + a + \", b=\" + b);\n\n        // Using temp variable\n        int temp = a; a = b; b = temp;\n        System.out.println(\"After (temp): a=\" + a + \", b=\" + b);\n\n        // Without temp (arithmetic)\n        a = 10; b = 20;\n        a = a + b; b = a - b; a = a - b;\n        System.out.println(\"After (arithmetic): a=\" + a + \", b=\" + b);\n\n        // Without temp (XOR)\n        a = 10; b = 20;\n        a = a ^ b; b = a ^ b; a = a ^ b;\n        System.out.println(\"After (XOR): a=\" + a + \", b=\" + b);\n    }\n}", "Java swap — temp variable, arithmetic, XOR."),
                _ => ("a, b = 10, 20\nprint(f'Before: a={a}, b={b}')\n\n# Pythonic swap\na, b = b, a\nprint(f'After (tuple swap): a={a}, b={b}')\n\n# Without temp (arithmetic)\na, b = 10, 20\na = a + b; b = a - b; a = a - b\nprint(f'After (arithmetic): a={a}, b={b}')\n\n# XOR\na, b = 10, 20\na ^= b; b ^= a; a ^= b\nprint(f'After (XOR): a={a}, b={b}')", "Python swap — tuple, arithmetic, XOR."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // Default: even/odd
        let (code, note) = match lang {
            "java" => ("public class EvenOdd {\n    public static void main(String[] args) {\n        int[] numbers = {1, 2, 3, 4, 5, 10, 15, 20, 0, -3};\n        for (int n : numbers) {\n            System.out.println(n + \" is \" + (n % 2 == 0 ? \"even\" : \"odd\"));\n        }\n\n        // Count even/odd in range\n        int evens = 0, odds = 0;\n        for (int i = 1; i <= 100; i++) {\n            if (i % 2 == 0) evens++; else odds++;\n        }\n        System.out.println(\"1-100: \" + evens + \" evens, \" + odds + \" odds\");\n    }\n}", "Java even/odd check."),
            _ => ("for n in [1, 2, 3, 4, 5, 10, 15, 20, 0, -3]:\n    print(f'{n} is {\"even\" if n % 2 == 0 else \"odd\"}')\n\nevens = [i for i in range(1, 101) if i % 2 == 0]\nodds  = [i for i in range(1, 101) if i % 2 != 0]\nprint(f'1-100: {len(evens)} evens, {len(odds)} odds')", "Python even/odd check."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── OOP extras (encapsulation, getter/setter, overloading, overriding, generics, constructor) ──
    let is_oop_extra = q.contains("encapsulation") || q.contains("getter") || q.contains("setter")
        || q.contains("overloading") || q.contains("overriding") || q.contains("generic")
        || q.contains("constructor") || q.contains("trait");
    if is_oop_extra {
        if q.contains("constructor") {
            let (code, note) = match lang {
                "java" => ("public class ConstructorDemo {\n    private String name;\n    private int age;\n\n    // Default constructor\n    public ConstructorDemo() {\n        this.name = \"Unknown\";\n        this.age = 0;\n    }\n\n    // Parameterized constructor\n    public ConstructorDemo(String name, int age) {\n        this.name = name;\n        this.age = age;\n    }\n\n    // Copy constructor\n    public ConstructorDemo(ConstructorDemo other) {\n        this.name = other.name;\n        this.age = other.age;\n    }\n\n    public String toString() {\n        return name + \" (age: \" + age + \")\";\n    }\n\n    public static void main(String[] args) {\n        ConstructorDemo d1 = new ConstructorDemo();\n        ConstructorDemo d2 = new ConstructorDemo(\"Arun\", 28);\n        ConstructorDemo d3 = new ConstructorDemo(d2);\n        System.out.println(d1);\n        System.out.println(d2);\n        System.out.println(\"Copy: \" + d3);\n    }\n}", "Java constructors — default, parameterized, copy."),
                _ => ("class Person:\n    def __init__(self, name='Unknown', age=0):\n        self.name = name\n        self.age = age\n\n    @classmethod\n    def from_string(cls, s):\n        name, age = s.split(',')\n        return cls(name.strip(), int(age.strip()))\n\n    def __repr__(self):\n        return f'Person({self.name!r}, {self.age})'\n\np1 = Person()\np2 = Person('Arun', 28)\np3 = Person.from_string('Kala, 1')\nprint(p1, p2, p3)", "Python constructors — __init__, @classmethod factory."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("overloading") {
            let (code, note) = match lang {
                "java" => ("public class OverloadingDemo {\n    static int add(int a, int b) { return a + b; }\n    static double add(double a, double b) { return a + b; }\n    static int add(int a, int b, int c) { return a + b + c; }\n    static String add(String a, String b) { return a + b; }\n\n    static void print(int x) { System.out.println(\"int: \" + x); }\n    static void print(double x) { System.out.println(\"double: \" + x); }\n    static void print(String x) { System.out.println(\"String: \" + x); }\n\n    public static void main(String[] args) {\n        System.out.println(add(2, 3));\n        System.out.println(add(2.5, 3.5));\n        System.out.println(add(1, 2, 3));\n        System.out.println(add(\"Hello\", \" World\"));\n        print(42);\n        print(3.14);\n        print(\"Kala\");\n    }\n}", "Java method overloading — same name, different parameters."),
                _ => ("# Python doesn't have true overloading, but we can use *args or singledispatch\nfrom functools import singledispatch\n\n@singledispatch\ndef add(a, b):\n    return a + b\n\n@add.register(str)\ndef _(a: str, b: str):\n    return a + ' ' + b\n\nprint(add(2, 3))          # 5\nprint(add(2.5, 3.5))      # 6.0\nprint(add('Hello', 'World'))  # Hello World\n\n# Or use default args\ndef greet(name, greeting='Hello'):\n    return f'{greeting}, {name}!'\n\nprint(greet('Arun'))             # Hello, Arun!\nprint(greet('Arun', 'Namaste'))  # Namaste, Arun!", "Python overloading — singledispatch + default args."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("overriding") {
            let (code, note) = match lang {
                "java" => ("class Animal {\n    String name;\n    Animal(String name) { this.name = name; }\n    String speak() { return name + \" makes a sound\"; }\n    String toString() { return \"Animal(\" + name + \")\"; }\n}\n\nclass Dog extends Animal {\n    Dog(String name) { super(name); }\n    @Override\n    String speak() { return name + \" says Woof!\"; }\n}\n\nclass Cat extends Animal {\n    Cat(String name) { super(name); }\n    @Override\n    String speak() { return name + \" says Meow!\"; }\n}\n\npublic class OverridingDemo {\n    public static void main(String[] args) {\n        Animal[] animals = { new Dog(\"Rex\"), new Cat(\"Whiskers\"), new Animal(\"Fish\") };\n        for (Animal a : animals) {\n            System.out.println(a.speak());\n        }\n    }\n}", "Java method overriding — @Override, polymorphism."),
                _ => ("class Animal:\n    def __init__(self, name): self.name = name\n    def speak(self): return f'{self.name} makes a sound'\n\nclass Dog(Animal):\n    def speak(self): return f'{self.name} says Woof!'\n\nclass Cat(Animal):\n    def speak(self): return f'{self.name} says Meow!'\n\nfor a in [Dog('Rex'), Cat('Whiskers'), Animal('Fish')]:\n    print(a.speak())", "Python method overriding — inheritance."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("encapsulation") || q.contains("getter") || q.contains("setter") {
            let (code, note) = match lang {
                "java" => ("public class Student {\n    private String name;\n    private int age;\n    private double gpa;\n\n    public Student(String name, int age, double gpa) {\n        this.name = name;\n        setAge(age);\n        setGpa(gpa);\n    }\n\n    public String getName() { return name; }\n    public void setName(String name) { this.name = name; }\n\n    public int getAge() { return age; }\n    public void setAge(int age) {\n        if (age < 0 || age > 150) throw new IllegalArgumentException(\"Invalid age: \" + age);\n        this.age = age;\n    }\n\n    public double getGpa() { return gpa; }\n    public void setGpa(double gpa) {\n        if (gpa < 0 || gpa > 4.0) throw new IllegalArgumentException(\"Invalid GPA: \" + gpa);\n        this.gpa = gpa;\n    }\n\n    public String toString() {\n        return String.format(\"%s (age=%d, GPA=%.2f)\", name, age, gpa);\n    }\n\n    public static void main(String[] args) {\n        Student s = new Student(\"Arun\", 22, 3.8);\n        System.out.println(s);\n        s.setName(\"Arun Kumar\");\n        System.out.println(\"Name: \" + s.getName());\n        try { s.setAge(-5); } catch (Exception e) { System.out.println(\"Error: \" + e.getMessage()); }\n    }\n}", "Java encapsulation — private fields, getters/setters with validation."),
                _ => ("class BankAccount:\n    def __init__(self, owner, balance=0):\n        self._owner = owner\n        self._balance = balance\n\n    @property\n    def owner(self): return self._owner\n\n    @property\n    def balance(self): return self._balance\n\n    @balance.setter\n    def balance(self, value):\n        if value < 0: raise ValueError('Balance cannot be negative')\n        self._balance = value\n\n    def deposit(self, amount):\n        if amount <= 0: raise ValueError('Must be positive')\n        self._balance += amount\n\n    def withdraw(self, amount):\n        if amount > self._balance: raise ValueError('Insufficient funds')\n        self._balance -= amount\n\nacc = BankAccount('Arun', 1000)\nacc.deposit(500)\nacc.withdraw(200)\nprint(f'{acc.owner}: ${acc.balance}')\ntry: acc.balance = -100\nexcept ValueError as e: print(f'Error: {e}')", "Python encapsulation — @property, setter validation."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("generic") {
            let (code, note) = match lang {
                "java" => ("import java.util.Arrays;\n\npublic class GenericsDemo {\n    // Generic method\n    static <T extends Comparable<T>> T max(T a, T b) {\n        return a.compareTo(b) >= 0 ? a : b;\n    }\n\n    // Generic class\n    static class Pair<A, B> {\n        A first; B second;\n        Pair(A first, B second) { this.first = first; this.second = second; }\n        public String toString() { return \"(\" + first + \", \" + second + \")\"; }\n    }\n\n    // Generic stack\n    static class Stack<T> {\n        private Object[] data = new Object[16];\n        private int top = 0;\n        void push(T item) { data[top++] = item; }\n        @SuppressWarnings(\"unchecked\")\n        T pop() { return (T) data[--top]; }\n        int size() { return top; }\n        boolean isEmpty() { return top == 0; }\n    }\n\n    public static void main(String[] args) {\n        System.out.println(\"Max int: \" + max(3, 7));\n        System.out.println(\"Max str: \" + max(\"apple\", \"banana\"));\n\n        Pair<String, Integer> p = new Pair<>(\"Arun\", 28);\n        System.out.println(\"Pair: \" + p);\n\n        Stack<String> stack = new Stack<>();\n        stack.push(\"Hello\");\n        stack.push(\"World\");\n        System.out.println(\"Pop: \" + stack.pop());\n    }\n}", "Java generics — generic method, class, Pair, Stack."),
                _ => ("from typing import TypeVar, Generic, List\n\nT = TypeVar('T')\n\nclass Stack(Generic[T]):\n    def __init__(self): self._items: List[T] = []\n    def push(self, item: T): self._items.append(item)\n    def pop(self) -> T: return self._items.pop()\n    def peek(self) -> T: return self._items[-1]\n    def __len__(self): return len(self._items)\n\ns: Stack[int] = Stack()\ns.push(1); s.push(2); s.push(3)\nprint(f'Pop: {s.pop()}, Size: {len(s)}')\n\ndef max_of(a: T, b: T) -> T:\n    return a if a >= b else b\n\nprint(max_of(3, 7))\nprint(max_of('apple', 'banana'))", "Python generics — typing, Generic class, TypeVar."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // trait
        let (code, note) = match lang {
            "rust" => ("trait Shape {\n    fn area(&self) -> f64;\n    fn perimeter(&self) -> f64;\n    fn name(&self) -> &str;\n    fn describe(&self) -> String {\n        format!(\"{}: area={:.2}, perimeter={:.2}\", self.name(), self.area(), self.perimeter())\n    }\n}\n\nstruct Circle { radius: f64 }\nstruct Rectangle { width: f64, height: f64 }\n\nimpl Shape for Circle {\n    fn area(&self) -> f64 { std::f64::consts::PI * self.radius * self.radius }\n    fn perimeter(&self) -> f64 { 2.0 * std::f64::consts::PI * self.radius }\n    fn name(&self) -> &str { \"Circle\" }\n}\n\nimpl Shape for Rectangle {\n    fn area(&self) -> f64 { self.width * self.height }\n    fn perimeter(&self) -> f64 { 2.0 * (self.width + self.height) }\n    fn name(&self) -> &str { \"Rectangle\" }\n}\n\nfn print_all(shapes: &[&dyn Shape]) {\n    for s in shapes { println!(\"{}\", s.describe()); }\n}\n\nfn main() {\n    let c = Circle { radius: 5.0 };\n    let r = Rectangle { width: 4.0, height: 6.0 };\n    print_all(&[&c, &r]);\n}", "Rust traits — default methods, dynamic dispatch, dyn."),
            _ => ("from abc import ABC, abstractmethod\n\nclass Shape(ABC):\n    @abstractmethod\n    def area(self) -> float: ...\n    @abstractmethod\n    def perimeter(self) -> float: ...\n    def describe(self):\n        return f'{type(self).__name__}: area={self.area():.2f}, perimeter={self.perimeter():.2f}'\n\nclass Circle(Shape):\n    def __init__(self, r): self.r = r\n    def area(self): return 3.14159 * self.r**2\n    def perimeter(self): return 2 * 3.14159 * self.r\n\nclass Rect(Shape):\n    def __init__(self, w, h): self.w, self.h = w, h\n    def area(self): return self.w * self.h\n    def perimeter(self): return 2 * (self.w + self.h)\n\nfor s in [Circle(5), Rect(4, 6)]:\n    print(s.describe())", "Python abstract base class (trait-like) — ABC, polymorphism."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── String extras (anagram, vowels, remove duplicates) ───────────────────
    let is_string_extra = q.contains("anagram") || q.contains("vowel") || q.contains("consonant")
        || q.contains("remove duplicate") || q.contains("count character")
        || q.contains("count word") || q.contains("uppercase") || q.contains("lowercase");
    if is_string_extra {
        if q.contains("anagram") {
            let (code, note) = match lang {
                "java" => ("import java.util.Arrays;\n\npublic class Anagram {\n    static boolean isAnagram(String a, String b) {\n        char[] ca = a.toLowerCase().replaceAll(\"\\\\s\", \"\").toCharArray();\n        char[] cb = b.toLowerCase().replaceAll(\"\\\\s\", \"\").toCharArray();\n        Arrays.sort(ca); Arrays.sort(cb);\n        return Arrays.equals(ca, cb);\n    }\n\n    public static void main(String[] args) {\n        System.out.println(isAnagram(\"listen\", \"silent\"));   // true\n        System.out.println(isAnagram(\"hello\", \"world\"));     // false\n        System.out.println(isAnagram(\"Astronomer\", \"Moon starer\")); // true\n    }\n}", "Java anagram check — sort and compare."),
                _ => ("def is_anagram(a, b):\n    return sorted(a.lower().replace(' ', '')) == sorted(b.lower().replace(' ', ''))\n\nprint(is_anagram('listen', 'silent'))          # True\nprint(is_anagram('hello', 'world'))            # False\nprint(is_anagram('Astronomer', 'Moon starer')) # True", "Python anagram check."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("vowel") || q.contains("consonant") {
            let (code, note) = match lang {
                "java" => ("public class VowelCounter {\n    public static void main(String[] args) {\n        String s = \"Hello World Programming\";\n        int vowels = 0, consonants = 0;\n        for (char c : s.toLowerCase().toCharArray()) {\n            if (\"aeiou\".indexOf(c) >= 0) vowels++;\n            else if (c >= 'a' && c <= 'z') consonants++;\n        }\n        System.out.println(\"\\\"\" + s + \"\\\"\");\n        System.out.println(\"Vowels: \" + vowels);\n        System.out.println(\"Consonants: \" + consonants);\n    }\n}", "Java vowel/consonant counter."),
                _ => ("s = 'Hello World Programming'\nvowels = sum(1 for c in s.lower() if c in 'aeiou')\nconsonants = sum(1 for c in s.lower() if c.isalpha() and c not in 'aeiou')\nprint(f'\"{s}\"')\nprint(f'Vowels: {vowels}, Consonants: {consonants}')", "Python vowel/consonant counter."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // remove duplicates
        let (code, note) = match lang {
            "java" => ("import java.util.LinkedHashSet;\n\npublic class RemoveDuplicates {\n    static String removeDups(String s) {\n        LinkedHashSet<Character> seen = new LinkedHashSet<>();\n        for (char c : s.toCharArray()) seen.add(c);\n        StringBuilder sb = new StringBuilder();\n        for (char c : seen) sb.append(c);\n        return sb.toString();\n    }\n\n    public static void main(String[] args) {\n        System.out.println(removeDups(\"programming\"));  // programin\n        System.out.println(removeDups(\"hello world\"));  // helo wrd\n    }\n}", "Java remove duplicate characters — LinkedHashSet."),
            _ => ("def remove_dups(s):\n    seen = set()\n    return ''.join(c for c in s if not (c in seen or seen.add(c)))\n\nprint(remove_dups('programming'))  # programin\nprint(remove_dups('hello world'))  # helo wrd", "Python remove duplicate characters."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Algo extras (selection sort, insertion sort, recursion) ───────────────
    let is_algo_extra = q.contains("selection sort") || q.contains("insertion sort")
        || q.contains("recursion") || q.contains("recursive");
    if is_algo_extra {
        if q.contains("selection sort") {
            let (code, note) = match lang {
                "java" => ("import java.util.Arrays;\n\npublic class SelectionSort {\n    static void selectionSort(int[] arr) {\n        for (int i = 0; i < arr.length - 1; i++) {\n            int minIdx = i;\n            for (int j = i + 1; j < arr.length; j++) {\n                if (arr[j] < arr[minIdx]) minIdx = j;\n            }\n            int tmp = arr[i]; arr[i] = arr[minIdx]; arr[minIdx] = tmp;\n        }\n    }\n\n    public static void main(String[] args) {\n        int[] arr = {64, 25, 12, 22, 11};\n        System.out.println(\"Before: \" + Arrays.toString(arr));\n        selectionSort(arr);\n        System.out.println(\"After:  \" + Arrays.toString(arr));\n    }\n}", "Java selection sort — O(n²)."),
                _ => ("def selection_sort(arr):\n    for i in range(len(arr) - 1):\n        min_idx = i\n        for j in range(i + 1, len(arr)):\n            if arr[j] < arr[min_idx]: min_idx = j\n        arr[i], arr[min_idx] = arr[min_idx], arr[i]\n    return arr\n\narr = [64, 25, 12, 22, 11]\nprint('Before:', arr)\nprint('After: ', selection_sort(arr))", "Python selection sort — O(n²)."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        // recursion
        let (code, note) = match lang {
            "java" => ("public class RecursionDemo {\n    static int factorial(int n) {\n        return n <= 1 ? 1 : n * factorial(n - 1);\n    }\n\n    static int fibonacci(int n) {\n        if (n <= 1) return n;\n        return fibonacci(n - 1) + fibonacci(n - 2);\n    }\n\n    static int power(int base, int exp) {\n        if (exp == 0) return 1;\n        return base * power(base, exp - 1);\n    }\n\n    static int sum(int n) {\n        return n <= 0 ? 0 : n + sum(n - 1);\n    }\n\n    static void towerOfHanoi(int n, char from, char to, char aux) {\n        if (n == 0) return;\n        towerOfHanoi(n - 1, from, aux, to);\n        System.out.println(\"Move disk \" + n + \" from \" + from + \" to \" + to);\n        towerOfHanoi(n - 1, aux, to, from);\n    }\n\n    public static void main(String[] args) {\n        System.out.println(\"5! = \" + factorial(5));\n        System.out.println(\"fib(10) = \" + fibonacci(10));\n        System.out.println(\"2^8 = \" + power(2, 8));\n        System.out.println(\"sum(10) = \" + sum(10));\n        System.out.println(\"\\nTower of Hanoi (3 disks):\");\n        towerOfHanoi(3, 'A', 'C', 'B');\n    }\n}", "Java recursion — factorial, fibonacci, power, sum, Tower of Hanoi."),
            _ => ("def factorial(n):\n    return 1 if n <= 1 else n * factorial(n - 1)\n\ndef fibonacci(n):\n    if n <= 1: return n\n    return fibonacci(n - 1) + fibonacci(n - 2)\n\ndef power(base, exp):\n    return 1 if exp == 0 else base * power(base, exp - 1)\n\ndef tower_of_hanoi(n, src='A', dst='C', aux='B'):\n    if n == 0: return\n    tower_of_hanoi(n-1, src, aux, dst)\n    print(f'Move disk {n} from {src} to {dst}')\n    tower_of_hanoi(n-1, aux, dst, src)\n\nprint(f'5! = {factorial(5)}')\nprint(f'fib(10) = {fibonacci(10)}')\nprint(f'2^8 = {power(2, 8)}')\nprint('\\nTower of Hanoi (3 disks):')\ntower_of_hanoi(3)", "Python recursion — factorial, fibonacci, power, Tower of Hanoi."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Misc programs (set, csv, todo, student mgmt, bank, datetime, casting, I/O, collections) ──
    let is_misc = q.contains("set ") || q.contains("csv") || q.contains("todo")
        || q.contains("student") || q.contains("bank account") || q.contains("date") || q.contains("time")
        || q.contains("type cast") || q.contains("casting") || q.contains("input output")
        || q.contains("basic input") || q.contains("collection") || q.contains("simple") && !q.contains("three");
    if is_misc {
        if q.contains("csv") {
            let code = "import csv\nfrom pathlib import Path\n\n# Write CSV\nheaders = ['name', 'age', 'city']\nrows = [['Arun', 28, 'Hyderabad'], ['Kala', 1, 'Cloud'], ['Dev', 25, 'Mumbai']]\n\nwith open('people.csv', 'w', newline='') as f:\n    writer = csv.writer(f)\n    writer.writerow(headers)\n    writer.writerows(rows)\nprint('CSV written.')\n\n# Read CSV\nwith open('people.csv') as f:\n    reader = csv.DictReader(f)\n    for row in reader:\n        print(f\"{row['name']} (age {row['age']}) from {row['city']}\")\n\n# Read into list of dicts\nwith open('people.csv') as f:\n    data = list(csv.DictReader(f))\nprint(f'\\nTotal rows: {len(data)}')\nprint(f'Names: {[d[\"name\"] for d in data]}')\n\nPath('people.csv').unlink()  # cleanup";
            return format!("```python\n{}\n```\n\n*Python CSV — read, write, DictReader.*", code);
        }
        if q.contains("todo") {
            let (code, note) = match lang {
                "java" => ("import java.util.*;\n\npublic class TodoApp {\n    static List<String> todos = new ArrayList<>();\n\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n        System.out.println(\"Todo App — commands: add, list, done, quit\");\n        while (true) {\n            System.out.print(\"> \");\n            String cmd = sc.nextLine().trim().toLowerCase();\n            if (cmd.equals(\"quit\")) break;\n            if (cmd.equals(\"add\")) {\n                System.out.print(\"Task: \");\n                todos.add(sc.nextLine().trim());\n                System.out.println(\"Added!\");\n            } else if (cmd.equals(\"list\")) {\n                if (todos.isEmpty()) System.out.println(\"No tasks.\");\n                for (int i = 0; i < todos.size(); i++)\n                    System.out.println((i+1) + \". \" + todos.get(i));\n            } else if (cmd.equals(\"done\")) {\n                System.out.print(\"Task #: \");\n                int idx = Integer.parseInt(sc.nextLine().trim()) - 1;\n                if (idx >= 0 && idx < todos.size()) {\n                    System.out.println(\"Done: \" + todos.remove(idx));\n                } else System.out.println(\"Invalid.\");\n            }\n        }\n    }\n}", "Java Todo app — add, list, done, quit."),
                _ => ("todos = []\n\ndef show():\n    if not todos: print('No tasks.')\n    for i, t in enumerate(todos, 1): print(f'{i}. {t}')\n\nprint('Todo App — commands: add, list, done, quit')\nwhile True:\n    cmd = input('> ').strip().lower()\n    if cmd == 'quit': break\n    elif cmd == 'add':\n        todos.append(input('Task: ').strip())\n        print('Added!')\n    elif cmd == 'list': show()\n    elif cmd == 'done':\n        show()\n        idx = int(input('Task #: ')) - 1\n        if 0 <= idx < len(todos):\n            print(f'Done: {todos.pop(idx)}')\n        else: print('Invalid.')\nprint('Goodbye!')", "Python Todo app — add, list, done, quit."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("student") {
            let (code, note) = match lang {
                "java" => ("import java.util.*;\n\npublic class StudentManagement {\n    static class Student {\n        String name; int age; double gpa;\n        Student(String n, int a, double g) { name=n; age=a; gpa=g; }\n        public String toString() { return String.format(\"%-10s age=%d GPA=%.2f\", name, age, gpa); }\n    }\n\n    public static void main(String[] args) {\n        List<Student> students = new ArrayList<>(Arrays.asList(\n            new Student(\"Arun\", 22, 3.8),\n            new Student(\"Priya\", 21, 3.9),\n            new Student(\"Dev\", 23, 3.5)\n        ));\n\n        // Add\n        students.add(new Student(\"Kala\", 20, 4.0));\n\n        // Display all\n        System.out.println(\"All students:\");\n        students.forEach(System.out::println);\n\n        // Sort by GPA\n        students.sort(Comparator.comparingDouble(s -> -s.gpa));\n        System.out.println(\"\\nSorted by GPA (desc):\");\n        students.forEach(System.out::println);\n\n        // Search\n        students.stream().filter(s -> s.name.equals(\"Arun\"))\n            .findFirst().ifPresent(s -> System.out.println(\"\\nFound: \" + s));\n\n        // Average GPA\n        double avg = students.stream().mapToDouble(s -> s.gpa).average().orElse(0);\n        System.out.printf(\"\\nAverage GPA: %.2f%n\", avg);\n    }\n}", "Java student management — add, display, sort, search, average."),
                _ => ("class Student:\n    def __init__(self, name, age, gpa):\n        self.name, self.age, self.gpa = name, age, gpa\n    def __repr__(self):\n        return f'{self.name:10s} age={self.age} GPA={self.gpa:.2f}'\n\nstudents = [Student('Arun',22,3.8), Student('Priya',21,3.9), Student('Dev',23,3.5)]\nstudents.append(Student('Kala', 20, 4.0))\n\nprint('All students:')\nfor s in students: print(s)\n\nprint('\\nSorted by GPA (desc):')\nfor s in sorted(students, key=lambda s: -s.gpa): print(s)\n\nprint(f'\\nAverage GPA: {sum(s.gpa for s in students)/len(students):.2f}')", "Python student management — add, display, sort, average."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("bank") {
            let (code, note) = match lang {
                "java" => ("public class BankAccount {\n    private String owner;\n    private double balance;\n\n    public BankAccount(String owner, double balance) {\n        this.owner = owner;\n        this.balance = balance;\n    }\n\n    public void deposit(double amount) {\n        if (amount <= 0) throw new IllegalArgumentException(\"Amount must be positive\");\n        balance += amount;\n        System.out.printf(\"Deposited $%.2f. Balance: $%.2f%n\", amount, balance);\n    }\n\n    public void withdraw(double amount) {\n        if (amount > balance) throw new IllegalArgumentException(\"Insufficient funds\");\n        balance -= amount;\n        System.out.printf(\"Withdrew $%.2f. Balance: $%.2f%n\", amount, balance);\n    }\n\n    public void transfer(BankAccount to, double amount) {\n        withdraw(amount);\n        to.deposit(amount);\n        System.out.printf(\"Transferred $%.2f to %s%n\", amount, to.owner);\n    }\n\n    public String toString() { return String.format(\"%s: $%.2f\", owner, balance); }\n\n    public static void main(String[] args) {\n        BankAccount a = new BankAccount(\"Arun\", 1000);\n        BankAccount b = new BankAccount(\"Kala\", 500);\n        a.deposit(500);\n        a.withdraw(200);\n        a.transfer(b, 300);\n        System.out.println(a);\n        System.out.println(b);\n    }\n}", "Java bank account — deposit, withdraw, transfer."),
                _ => ("class BankAccount:\n    def __init__(self, owner, balance=0):\n        self.owner = owner\n        self.balance = balance\n\n    def deposit(self, amount):\n        self.balance += amount\n        print(f'Deposited ${amount:.2f}. Balance: ${self.balance:.2f}')\n\n    def withdraw(self, amount):\n        if amount > self.balance: raise ValueError('Insufficient funds')\n        self.balance -= amount\n        print(f'Withdrew ${amount:.2f}. Balance: ${self.balance:.2f}')\n\n    def transfer(self, to, amount):\n        self.withdraw(amount)\n        to.deposit(amount)\n\n    def __repr__(self): return f'{self.owner}: ${self.balance:.2f}'\n\na = BankAccount('Arun', 1000)\nb = BankAccount('Kala', 500)\na.deposit(500)\na.withdraw(200)\na.transfer(b, 300)\nprint(a)\nprint(b)", "Python bank account — deposit, withdraw, transfer."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("date") || q.contains("time") {
            let (code, note) = match lang {
                "java" => ("import java.time.*;\nimport java.time.format.DateTimeFormatter;\n\npublic class DateTimeDemo {\n    public static void main(String[] args) {\n        // Current date and time\n        LocalDate today = LocalDate.now();\n        LocalTime now = LocalTime.now();\n        LocalDateTime dateTime = LocalDateTime.now();\n        System.out.println(\"Date: \" + today);\n        System.out.println(\"Time: \" + now);\n        System.out.println(\"DateTime: \" + dateTime);\n\n        // Formatting\n        DateTimeFormatter fmt = DateTimeFormatter.ofPattern(\"dd-MM-yyyy HH:mm:ss\");\n        System.out.println(\"Formatted: \" + dateTime.format(fmt));\n\n        // Arithmetic\n        System.out.println(\"Tomorrow: \" + today.plusDays(1));\n        System.out.println(\"Last week: \" + today.minusWeeks(1));\n        System.out.println(\"Next month: \" + today.plusMonths(1));\n\n        // Difference\n        LocalDate birthday = LocalDate.of(1997, 5, 15);\n        Period age = Period.between(birthday, today);\n        System.out.printf(\"Age: %d years, %d months, %d days%n\", age.getYears(), age.getMonths(), age.getDays());\n    }\n}", "Java date/time — LocalDate, formatting, arithmetic, Period."),
                _ => ("from datetime import datetime, date, timedelta\n\n# Current\nnow = datetime.now()\ntoday = date.today()\nprint(f'Now: {now}')\nprint(f'Today: {today}')\nprint(f'Formatted: {now.strftime(\"%d-%m-%Y %H:%M:%S\")}')\n\n# Arithmetic\ntomorrow = today + timedelta(days=1)\nlast_week = today - timedelta(weeks=1)\nprint(f'Tomorrow: {tomorrow}')\nprint(f'Last week: {last_week}')\n\n# Difference\nbirthday = date(1997, 5, 15)\nage = today - birthday\nprint(f'Days since birthday: {age.days} ({age.days // 365} years)')\n\n# Parse string\nparsed = datetime.strptime('2025-01-15 14:30:00', '%Y-%m-%d %H:%M:%S')\nprint(f'Parsed: {parsed}')", "Python datetime — format, arithmetic, parse, timedelta."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("cast") {
            let (code, note) = match lang {
                "java" => ("public class TypeCasting {\n    public static void main(String[] args) {\n        // Widening (implicit) — smaller to larger\n        int i = 42;\n        double d = i;  // int → double (automatic)\n        System.out.println(\"int → double: \" + d);\n\n        // Narrowing (explicit) — larger to smaller\n        double pi = 3.14159;\n        int truncated = (int) pi;  // double → int (loses decimal)\n        System.out.println(\"double → int: \" + truncated);\n\n        // String conversions\n        String numStr = \"123\";\n        int parsed = Integer.parseInt(numStr);\n        double parsedD = Double.parseDouble(\"3.14\");\n        String back = String.valueOf(parsed);\n        System.out.println(\"String → int: \" + parsed);\n        System.out.println(\"int → String: \" + back);\n\n        // char ↔ int\n        char ch = 'A';\n        int ascii = ch;  // char → int (65)\n        char fromInt = (char) 66;  // int → char ('B')\n        System.out.println(\"'A' as int: \" + ascii);\n        System.out.println(\"66 as char: \" + fromInt);\n\n        // Object casting (upcasting / downcasting)\n        Object obj = \"Hello\";  // upcast\n        if (obj instanceof String s) {\n            System.out.println(\"Downcast: \" + s.toUpperCase());\n        }\n    }\n}", "Java type casting — widening, narrowing, String, char, object."),
                _ => ("# Python is dynamically typed — conversion functions instead of casting\na = 42\nb = float(a)       # int → float\nprint(f'int → float: {b}')\n\nc = 3.14\nd = int(c)         # float → int (truncates)\nprint(f'float → int: {d}')\n\n# String conversions\nnum_str = '123'\nparsed = int(num_str)\nback = str(parsed)\nprint(f'str → int: {parsed}, int → str: {back!r}')\n\n# Type checking\nprint(f'type(42) = {type(42).__name__}')\nprint(f'isinstance check: {isinstance(42, int)}')\n\n# bool\nprint(f'bool(0)={bool(0)}, bool(1)={bool(1)}, bool(\"\")={bool(\"\")}')", "Python type conversion — int, float, str, bool, isinstance."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("input") || q.contains("output") || q.contains("basic") {
            let (code, note) = match lang {
                "java" => ("import java.util.Scanner;\n\npublic class InputOutput {\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n\n        // String input\n        System.out.print(\"Enter your name: \");\n        String name = sc.nextLine();\n        System.out.println(\"Hello, \" + name + \"!\");\n\n        // Integer input\n        System.out.print(\"Enter your age: \");\n        int age = sc.nextInt();\n        System.out.println(\"You are \" + age + \" years old.\");\n\n        // Double input\n        System.out.print(\"Enter a decimal: \");\n        double num = sc.nextDouble();\n        System.out.printf(\"You entered: %.2f%n\", num);\n\n        // Formatted output\n        System.out.printf(\"%-10s | %5d | %8.2f%n\", name, age, num);\n        System.out.printf(\"%-10s | %5d | %8.2f%n\", \"Kala\", 1, 3.14);\n    }\n}", "Java I/O — Scanner input, println, printf formatting."),
                _ => ("# Input\nname = input('Enter your name: ')\nage = int(input('Enter your age: '))\nprint(f'Hello, {name}! You are {age} years old.')\n\n# Formatted output\nprint(f'{\"Name\":<10} | {\"Age\":>5} | {\"GPA\":>8}')\nprint(f'{\"Arun\":<10} | {22:>5} | {3.80:>8.2f}')\nprint(f'{\"Kala\":<10} | {1:>5} | {4.00:>8.2f}')", "Python I/O — input, print, f-string formatting."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("set") {
            let (code, note) = match lang {
                "java" => ("import java.util.*;\n\npublic class SetDemo {\n    public static void main(String[] args) {\n        Set<String> fruits = new HashSet<>(Arrays.asList(\"apple\", \"banana\", \"cherry\"));\n        fruits.add(\"date\");\n        fruits.add(\"apple\");  // duplicate ignored\n        System.out.println(\"Set: \" + fruits);\n        System.out.println(\"Contains apple: \" + fruits.contains(\"apple\"));\n        System.out.println(\"Size: \" + fruits.size());\n\n        Set<Integer> a = new HashSet<>(Arrays.asList(1, 2, 3, 4, 5));\n        Set<Integer> b = new HashSet<>(Arrays.asList(4, 5, 6, 7, 8));\n\n        // Union\n        Set<Integer> union = new HashSet<>(a); union.addAll(b);\n        System.out.println(\"Union: \" + union);\n\n        // Intersection\n        Set<Integer> inter = new HashSet<>(a); inter.retainAll(b);\n        System.out.println(\"Intersection: \" + inter);\n\n        // Difference\n        Set<Integer> diff = new HashSet<>(a); diff.removeAll(b);\n        System.out.println(\"Difference: \" + diff);\n    }\n}", "Java Set — add, contains, union, intersection, difference."),
                _ => ("fruits = {'apple', 'banana', 'cherry'}\nfruits.add('date')\nfruits.add('apple')  # duplicate ignored\nprint(f'Set: {fruits}')\nprint(f'Contains apple: {\"apple\" in fruits}')\nprint(f'Size: {len(fruits)}')\n\na = {1, 2, 3, 4, 5}\nb = {4, 5, 6, 7, 8}\nprint(f'Union: {a | b}')\nprint(f'Intersection: {a & b}')\nprint(f'Difference: {a - b}')\nprint(f'Symmetric diff: {a ^ b}')\n\n# Remove duplicates from list\nnums = [1, 2, 2, 3, 3, 3, 4]\nunique = list(set(nums))\nprint(f'Unique: {sorted(unique)}')", "Python sets — add, union, intersection, difference, dedup."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("collection") {
            let code = "import java.util.*;\nimport java.util.stream.Collectors;\n\npublic class CollectionsDemo {\n    public static void main(String[] args) {\n        // List\n        List<String> list = new ArrayList<>(Arrays.asList(\"banana\", \"apple\", \"cherry\"));\n        list.add(\"date\");\n        Collections.sort(list);\n        System.out.println(\"List: \" + list);\n\n        // Set\n        Set<Integer> set = new TreeSet<>(Arrays.asList(5, 3, 1, 4, 2, 3));\n        System.out.println(\"Set: \" + set);\n\n        // Map\n        Map<String, Integer> map = new HashMap<>();\n        map.put(\"Arun\", 95); map.put(\"Kala\", 88); map.put(\"Dev\", 92);\n        System.out.println(\"Map: \" + map);\n        map.forEach((k, v) -> System.out.println(\"  \" + k + \": \" + v));\n\n        // Queue\n        Queue<String> queue = new LinkedList<>();\n        queue.offer(\"first\"); queue.offer(\"second\"); queue.offer(\"third\");\n        System.out.println(\"Queue poll: \" + queue.poll());\n\n        // Stack\n        Deque<Integer> stack = new ArrayDeque<>();\n        stack.push(1); stack.push(2); stack.push(3);\n        System.out.println(\"Stack pop: \" + stack.pop());\n\n        // Stream operations\n        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);\n        int sum = nums.stream().filter(n -> n % 2 == 0).mapToInt(Integer::intValue).sum();\n        System.out.println(\"Sum of evens: \" + sum);\n    }\n}";
            return format!("```java\n{}\n```\n\n*Java Collections — List, Set, Map, Queue, Stack, Streams.*", code);
        }
        // "simple java program" or "simple python program"
        let (code, note) = match lang {
            "java" => ("import java.util.Scanner;\n\npublic class SimpleProgram {\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n        System.out.print(\"Enter your name: \");\n        String name = sc.nextLine();\n        System.out.print(\"Enter a number: \");\n        int num = sc.nextInt();\n\n        System.out.println(\"Hello, \" + name + \"!\");\n        System.out.println(num + \" is \" + (num % 2 == 0 ? \"even\" : \"odd\"));\n\n        int sum = 0;\n        for (int i = 1; i <= num; i++) sum += i;\n        System.out.println(\"Sum 1 to \" + num + \" = \" + sum);\n\n        System.out.println(num + \"! = \" + factorial(num));\n    }\n\n    static long factorial(int n) {\n        return n <= 1 ? 1 : n * factorial(n - 1);\n    }\n}", "Java starter — input, even/odd, sum, factorial."),
            _ => ("name = input('Enter your name: ')\nnum = int(input('Enter a number: '))\n\nprint(f'Hello, {name}!')\nprint(f'{num} is {\"even\" if num % 2 == 0 else \"odd\"}')\nprint(f'Sum 1 to {num} = {sum(range(1, num+1))}')\n\ndef factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)\nprint(f'{num}! = {factorial(num)}')", "Python starter — input, even/odd, sum, factorial."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Three.js + camera / gesture starter (offline template) ───────────────
    let mentions_three = q.contains("three.js") || q.contains("threejs")
        || (q.contains("three") && q.contains(".js"))
        || q.contains("threee"); // typo tolerance
    if (mentions_three || q.contains("webgl"))
        && (q.contains("face") || q.contains("gesture") || q.contains("gestor") || q.contains("gester")
            || q.contains("head") || q.contains("mesh") || q.contains("camera") || q.contains("track")
            || q.contains("interactive"))
    {
        let code = r#"// index.html — open via local static server (gesture needs webcam + HTTPS or localhost)
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Three.js face / gesture starter</title>
  <style>body{margin:0;overflow:hidden;background:#111;} video{position:fixed;left:8px;bottom:8px;width:160px;opacity:.35;border-radius:8px;}</style>
</head>
<body>
<video id="cam" autoplay playsinline muted></video>
<script type="importmap">
{ "imports": { "three": "https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.module.js" } }
</script>
<script type="module">
import * as THREE from "three";

const video = document.getElementById("cam");
const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "user" }, audio: false });
video.srcObject = stream;

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(60, innerWidth / innerHeight, 0.1, 1000);
camera.position.z = 4;
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setSize(innerWidth, innerHeight);
document.body.appendChild(renderer.domElement);

const group = new THREE.Group();
scene.add(group);
const geo = new THREE.IcosahedronGeometry(1, 2);
const mat = new THREE.MeshStandardMaterial({ color: 0x5eead4, flatShading: true, metalness: 0.2, roughness: 0.5 });
group.add(new THREE.Mesh(geo, mat));
scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const sun = new THREE.DirectionalLight(0xffffff, 1);
sun.position.set(3, 5, 5);
scene.add(sun);

// Swap this loop for MediaPipe / TensorFlow face-landmarks → drive group.rotation from nose direction
function animate(t) {
  requestAnimationFrame(animate);
  const wobble = Math.sin(t * 0.002) * 0.15;
  group.rotation.y = wobble + (video.videoWidth ? Math.sin(t * 0.001) * 0.1 : 0);
  group.rotation.x = Math.cos(t * 0.0017) * 0.08;
  renderer.render(scene, camera);
}
animate(0);

addEventListener("resize", () => {
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(innerWidth, innerHeight);
});
</script>
</body>
</html>"#;
        return format!(
            "```html\n{}\n```\n\n\
             *Run with `npx serve .` (or any static server) on **localhost** so `getUserMedia` works. \
             For real **face gestures**, add **MediaPipe Face Mesh** or **@tensorflow-models/face-landmarks-detection** \
             and map landmark points to `group.rotation` or morph targets.*",
            code
        );
    }

    // ── FizzBuzz ─────────────────────────────────────────────────────────────
    if q.contains("fizzbuzz") || q.contains("fizz buzz") {
        let (code, note) = match lang {
            "killer" => (
                "kfn fizzbuzz(n) {\n  i = 1\n  while i <= n {\n    if i % 15 == 0 { print(K\"FizzBuzz\") }\n    else if i % 3 == 0 { print(K\"Fizz\") }\n    else if i % 5 == 0 { print(K\"Buzz\") }\n    else { print(K\"{i}\") }\n    i = i + 1\n  }\n}\nfizzbuzz(100)", "FizzBuzz 1–100 in Killer."),
            "rust"   => ("fn main() {\n    for i in 1..=100 {\n        match (i % 3, i % 5) {\n            (0, 0) => println!(\"FizzBuzz\"),\n            (0, _) => println!(\"Fizz\"),\n            (_, 0) => println!(\"Buzz\"),\n            _      => println!(\"{}\", i),\n        }\n    }\n}", "Idiomatic Rust FizzBuzz using pattern matching."),
            "javascript" => ("for (let i = 1; i <= 100; i++) {\n  console.log(i % 15 === 0 ? 'FizzBuzz' : i % 3 === 0 ? 'Fizz' : i % 5 === 0 ? 'Buzz' : i);\n}", "FizzBuzz 1–100 in JavaScript."),
            _ => ("for i in range(1, 101):\n    if i % 15 == 0:   print('FizzBuzz')\n    elif i % 3 == 0:  print('Fizz')\n    elif i % 5 == 0:  print('Buzz')\n    else:             print(i)", "FizzBuzz 1–100 in Python."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Quicksort ─────────────────────────────────────────────────────────────
    if q.contains("quicksort") || q.contains("quick sort") || (q.contains("quick") && q.contains("sort")) {
        let (code, note) = match lang {
            "killer" => ("# O(n log n) average, O(n²) worst\nkfn quicksort(arr, lo, hi) {\n  if lo < hi {\n    p = partition(arr, lo, hi)\n    quicksort(arr, lo, p - 1)\n    quicksort(arr, p + 1, hi)\n  }\n}\n\nkfn partition(arr, lo, hi) {\n  pivot = arr[hi]\n  i = lo - 1\n  j = lo\n  while j < hi {\n    if arr[j] <= pivot {\n      i = i + 1\n      tmp = arr[i]; arr[i] = arr[j]; arr[j] = tmp\n    }\n    j = j + 1\n  }\n  tmp = arr[i+1]; arr[i+1] = arr[hi]; arr[hi] = tmp\n  i + 1\n}\n\ndata = [3, 6, 8, 10, 1, 2, 1]\nquicksort(data, 0, 6)\nprint(data)", "Quicksort in Killer — in-place."),
            "rust"   => ("// O(n log n) average, O(n²) worst\nfn quicksort(arr: &mut [i32]) {\n    let len = arr.len();\n    if len <= 1 { return; }\n    let pivot_idx = partition(arr);\n    let (left, right) = arr.split_at_mut(pivot_idx);\n    quicksort(left);\n    quicksort(&mut right[1..]);\n}\n\nfn partition(arr: &mut [i32]) -> usize {\n    let pivot = arr[arr.len() - 1];\n    let mut i = 0;\n    for j in 0..arr.len() - 1 {\n        if arr[j] <= pivot {\n            arr.swap(i, j);\n            i += 1;\n        }\n    }\n    arr.swap(i, arr.len() - 1);\n    i\n}\n\nfn main() {\n    let mut v = vec![3, 6, 8, 10, 1, 2, 1];\n    quicksort(&mut v);\n    println!(\"{:?}\", v); // [1, 1, 2, 3, 6, 8, 10]\n}", "Quicksort in Rust — safe slice-based."),
            _ => ("# O(n log n) average, O(n²) worst case, O(log n) space\ndef quicksort(arr, lo=0, hi=None):\n    if hi is None:\n        hi = len(arr) - 1\n    if lo < hi:\n        pivot_idx = partition(arr, lo, hi)\n        quicksort(arr, lo, pivot_idx - 1)\n        quicksort(arr, pivot_idx + 1, hi)\n\ndef partition(arr, lo, hi):\n    pivot = arr[hi]\n    i = lo - 1\n    for j in range(lo, hi):\n        if arr[j] <= pivot:\n            i += 1\n            arr[i], arr[j] = arr[j], arr[i]\n    arr[i+1], arr[hi] = arr[hi], arr[i+1]\n    return i + 1\n\n# Usage\ndata = [3, 6, 8, 10, 1, 2, 1]\nquicksort(data)\nprint(data)  # [1, 1, 2, 3, 6, 8, 10]", "Quicksort in Python — in-place Lomuto partition."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Binary Search (NOT binary search tree — that's handled later) ─────────
    if (q.contains("binary search") || (q.contains("binary") && q.contains("search")))
        && !q.contains("tree") && !q.contains("bst")
    {
        let (code, note) = match lang {
            "killer" => ("# O(log n) time, O(1) space\nkfn binary_search(arr, target) {\n  lo = 0\n  hi = len(arr) - 1\n  while lo <= hi {\n    mid = (lo + hi) / 2\n    if arr[mid] == target { mid }\n    else if arr[mid] < target { lo = mid + 1 }\n    else { hi = mid - 1 }\n  }\n  -1\n}\n\nsorted = [1, 3, 5, 7, 9, 11, 13]\nprint(binary_search(sorted, 7))   # 3\nprint(binary_search(sorted, 6))   # -1", "Binary search in Killer — returns index or -1."),
            "rust"   => ("// O(log n) time, O(1) space\nfn binary_search(arr: &[i32], target: i32) -> Option<usize> {\n    let (mut lo, mut hi) = (0, arr.len());\n    while lo < hi {\n        let mid = lo + (hi - lo) / 2;\n        match arr[mid].cmp(&target) {\n            std::cmp::Ordering::Equal   => return Some(mid),\n            std::cmp::Ordering::Less    => lo = mid + 1,\n            std::cmp::Ordering::Greater => hi = mid,\n        }\n    }\n    None\n}\n\nfn main() {\n    let v = [1, 3, 5, 7, 9, 11, 13];\n    println!(\"{:?}\", binary_search(&v, 7));  // Some(3)\n    println!(\"{:?}\", binary_search(&v, 6));  // None\n}", "Binary search in Rust — returns Option<usize>."),
            _ => ("# O(log n) time, O(1) space\ndef binary_search(arr: list, target) -> int:\n    \"\"\"Returns index of target in sorted arr, or -1 if not found.\"\"\"\n    lo, hi = 0, len(arr) - 1\n    while lo <= hi:\n        mid = (lo + hi) // 2\n        if arr[mid] == target:\n            return mid\n        elif arr[mid] < target:\n            lo = mid + 1\n        else:\n            hi = mid - 1\n    return -1\n\n# Usage\nsorted_arr = [1, 3, 5, 7, 9, 11, 13]\nprint(binary_search(sorted_arr, 7))  # 3\nprint(binary_search(sorted_arr, 6))  # -1", "Binary search in Python — iterative O(log n)."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Merge Sort ──────────────────────────────────────────────────────────
    if q.contains("merge sort") || q.contains("mergesort") {
        let (code, note) = match lang {
            "java" => ("import java.util.Arrays;\n\npublic class MergeSort {\n    static void merge(int[] arr, int l, int m, int r) {\n        int[] left = Arrays.copyOfRange(arr, l, m + 1);\n        int[] right = Arrays.copyOfRange(arr, m + 1, r + 1);\n        int i = 0, j = 0, k = l;\n        while (i < left.length && j < right.length)\n            arr[k++] = left[i] <= right[j] ? left[i++] : right[j++];\n        while (i < left.length) arr[k++] = left[i++];\n        while (j < right.length) arr[k++] = right[j++];\n    }\n\n    static void mergeSort(int[] arr, int l, int r) {\n        if (l < r) {\n            int m = (l + r) / 2;\n            mergeSort(arr, l, m);\n            mergeSort(arr, m + 1, r);\n            merge(arr, l, m, r);\n        }\n    }\n\n    public static void main(String[] args) {\n        int[] data = {38, 27, 43, 3, 9, 82, 10};\n        mergeSort(data, 0, data.length - 1);\n        System.out.println(Arrays.toString(data));\n    }\n}", "Java merge sort — O(n log n), stable."),
            "rust" => ("fn merge_sort(arr: &mut [i32]) {\n    let len = arr.len();\n    if len <= 1 { return; }\n    let mid = len / 2;\n    let mut left = arr[..mid].to_vec();\n    let mut right = arr[mid..].to_vec();\n    merge_sort(&mut left);\n    merge_sort(&mut right);\n    let (mut i, mut j, mut k) = (0, 0, 0);\n    while i < left.len() && j < right.len() {\n        if left[i] <= right[j] { arr[k] = left[i]; i += 1; }\n        else { arr[k] = right[j]; j += 1; }\n        k += 1;\n    }\n    while i < left.len() { arr[k] = left[i]; i += 1; k += 1; }\n    while j < right.len() { arr[k] = right[j]; j += 1; k += 1; }\n}\n\nfn main() {\n    let mut data = vec![38, 27, 43, 3, 9, 82, 10];\n    merge_sort(&mut data);\n    println!(\"{:?}\", data);\n}", "Rust merge sort — O(n log n), stable."),
            "cpp" => ("#include <iostream>\n#include <vector>\nusing namespace std;\n\nvoid merge(vector<int>& a, int l, int m, int r) {\n    vector<int> L(a.begin()+l, a.begin()+m+1), R(a.begin()+m+1, a.begin()+r+1);\n    int i=0,j=0,k=l;\n    while(i<(int)L.size()&&j<(int)R.size()) a[k++]=L[i]<=R[j]?L[i++]:R[j++];\n    while(i<(int)L.size()) a[k++]=L[i++];\n    while(j<(int)R.size()) a[k++]=R[j++];\n}\nvoid mergeSort(vector<int>& a, int l, int r) {\n    if(l<r){ int m=(l+r)/2; mergeSort(a,l,m); mergeSort(a,m+1,r); merge(a,l,m,r); }\n}\nint main(){\n    vector<int> v={38,27,43,3,9,82,10};\n    mergeSort(v,0,v.size()-1);\n    for(int x:v) cout<<x<<\" \";\n}", "C++ merge sort — O(n log n)."),
            "javascript" | "typescript" => ("function mergeSort(arr) {\n  if (arr.length <= 1) return arr;\n  const mid = Math.floor(arr.length / 2);\n  const left = mergeSort(arr.slice(0, mid));\n  const right = mergeSort(arr.slice(mid));\n  const result = [];\n  let i = 0, j = 0;\n  while (i < left.length && j < right.length)\n    result.push(left[i] <= right[j] ? left[i++] : right[j++]);\n  return result.concat(left.slice(i), right.slice(j));\n}\n\nconsole.log(mergeSort([38, 27, 43, 3, 9, 82, 10]));", "Merge sort in JS — O(n log n), stable."),
            "go" => ("package main\n\nimport \"fmt\"\n\nfunc mergeSort(arr []int) []int {\n\tif len(arr) <= 1 { return arr }\n\tmid := len(arr) / 2\n\tleft := mergeSort(arr[:mid])\n\tright := mergeSort(arr[mid:])\n\treturn merge(left, right)\n}\n\nfunc merge(l, r []int) []int {\n\tres := make([]int, 0, len(l)+len(r))\n\ti, j := 0, 0\n\tfor i < len(l) && j < len(r) {\n\t\tif l[i] <= r[j] { res = append(res, l[i]); i++ } else { res = append(res, r[j]); j++ }\n\t}\n\tres = append(res, l[i:]...)\n\tres = append(res, r[j:]...)\n\treturn res\n}\n\nfunc main() {\n\tfmt.Println(mergeSort([]int{38, 27, 43, 3, 9, 82, 10}))\n}", "Go merge sort — O(n log n)."),
            "killer" => ("kfn merge_sort(arr) {\n  if len(arr) <= 1 { arr }\n  else {\n    mid = len(arr) / 2\n    left = merge_sort(arr[0..mid])\n    right = merge_sort(arr[mid..])\n    merge(left, right)\n  }\n}\n\nkfn merge(l, r) {\n  result = []; i = 0; j = 0\n  while i < len(l) and j < len(r) {\n    if l[i] <= r[j] { result.push(l[i]); i = i + 1 }\n    else { result.push(r[j]); j = j + 1 }\n  }\n  while i < len(l) { result.push(l[i]); i = i + 1 }\n  while j < len(r) { result.push(r[j]); j = j + 1 }\n  result\n}\n\ndata = [38, 27, 43, 3, 9, 82, 10]\nprint(merge_sort(data))", "Merge sort in Killer — O(n log n)."),
            _ => ("# O(n log n) time, O(n) space — stable sort\ndef merge_sort(arr):\n    if len(arr) <= 1:\n        return arr\n    mid = len(arr) // 2\n    left  = merge_sort(arr[:mid])\n    right = merge_sort(arr[mid:])\n    return merge(left, right)\n\ndef merge(left, right):\n    result, i, j = [], 0, 0\n    while i < len(left) and j < len(right):\n        if left[i] <= right[j]:\n            result.append(left[i]); i += 1\n        else:\n            result.append(right[j]); j += 1\n    result.extend(left[i:])\n    result.extend(right[j:])\n    return result\n\ndata = [38, 27, 43, 3, 9, 82, 10]\nprint(merge_sort(data))", "Merge sort — O(n log n), stable."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Bubble Sort ─────────────────────────────────────────────────────────
    if q.contains("bubble sort") || q.contains("bubblesort") {
        let (code, note) = match lang {
            "java" => ("import java.util.Arrays;\n\npublic class BubbleSort {\n    static void bubbleSort(int[] arr) {\n        int n = arr.length;\n        for (int i = 0; i < n; i++) {\n            boolean swapped = false;\n            for (int j = 0; j < n - i - 1; j++) {\n                if (arr[j] > arr[j + 1]) {\n                    int tmp = arr[j]; arr[j] = arr[j+1]; arr[j+1] = tmp;\n                    swapped = true;\n                }\n            }\n            if (!swapped) break;\n        }\n    }\n\n    public static void main(String[] args) {\n        int[] data = {64, 34, 25, 12, 22, 11, 90};\n        bubbleSort(data);\n        System.out.println(Arrays.toString(data));\n    }\n}", "Java bubble sort — O(n²) with early-exit."),
            "rust" => ("fn bubble_sort(arr: &mut [i32]) {\n    let n = arr.len();\n    for i in 0..n {\n        let mut swapped = false;\n        for j in 0..n - i - 1 {\n            if arr[j] > arr[j + 1] {\n                arr.swap(j, j + 1);\n                swapped = true;\n            }\n        }\n        if !swapped { break; }\n    }\n}\n\nfn main() {\n    let mut data = vec![64, 34, 25, 12, 22, 11, 90];\n    bubble_sort(&mut data);\n    println!(\"{:?}\", data);\n}", "Rust bubble sort — O(n²) with early-exit."),
            "cpp" => ("#include <iostream>\n#include <vector>\nusing namespace std;\n\nvoid bubbleSort(vector<int>& arr) {\n    int n = arr.size();\n    for (int i = 0; i < n; i++) {\n        bool swapped = false;\n        for (int j = 0; j < n - i - 1; j++) {\n            if (arr[j] > arr[j+1]) {\n                swap(arr[j], arr[j+1]);\n                swapped = true;\n            }\n        }\n        if (!swapped) break;\n    }\n}\n\nint main() {\n    vector<int> data = {64, 34, 25, 12, 22, 11, 90};\n    bubbleSort(data);\n    for (int x : data) cout << x << \" \";\n}", "C++ bubble sort — O(n²) with early-exit."),
            "c" => ("#include <stdio.h>\n\nvoid bubble_sort(int arr[], int n) {\n    for (int i = 0; i < n; i++) {\n        int swapped = 0;\n        for (int j = 0; j < n - i - 1; j++) {\n            if (arr[j] > arr[j+1]) {\n                int tmp = arr[j]; arr[j] = arr[j+1]; arr[j+1] = tmp;\n                swapped = 1;\n            }\n        }\n        if (!swapped) break;\n    }\n}\n\nint main() {\n    int data[] = {64, 34, 25, 12, 22, 11, 90};\n    int n = sizeof(data)/sizeof(data[0]);\n    bubble_sort(data, n);\n    for (int i = 0; i < n; i++) printf(\"%d \", data[i]);\n    return 0;\n}", "C bubble sort — O(n²) with early-exit."),
            "javascript" | "typescript" => ("function bubbleSort(arr) {\n  const n = arr.length;\n  for (let i = 0; i < n; i++) {\n    let swapped = false;\n    for (let j = 0; j < n - i - 1; j++) {\n      if (arr[j] > arr[j+1]) {\n        [arr[j], arr[j+1]] = [arr[j+1], arr[j]];\n        swapped = true;\n      }\n    }\n    if (!swapped) break;\n  }\n  return arr;\n}\n\nconsole.log(bubbleSort([64, 34, 25, 12, 22, 11, 90]));", "Bubble sort in JS — O(n²) with early-exit."),
            "go" => ("package main\n\nimport \"fmt\"\n\nfunc bubbleSort(arr []int) {\n\tn := len(arr)\n\tfor i := 0; i < n; i++ {\n\t\tswapped := false\n\t\tfor j := 0; j < n-i-1; j++ {\n\t\t\tif arr[j] > arr[j+1] {\n\t\t\t\tarr[j], arr[j+1] = arr[j+1], arr[j]\n\t\t\t\tswapped = true\n\t\t\t}\n\t\t}\n\t\tif !swapped { break }\n\t}\n}\n\nfunc main() {\n\tdata := []int{64, 34, 25, 12, 22, 11, 90}\n\tbubbleSort(data)\n\tfmt.Println(data)\n}", "Go bubble sort — O(n²) with early-exit."),
            "csharp" => ("using System;\n\nclass BubbleSort {\n    static void Sort(int[] arr) {\n        int n = arr.Length;\n        for (int i = 0; i < n; i++) {\n            bool swapped = false;\n            for (int j = 0; j < n - i - 1; j++) {\n                if (arr[j] > arr[j+1]) {\n                    (arr[j], arr[j+1]) = (arr[j+1], arr[j]);\n                    swapped = true;\n                }\n            }\n            if (!swapped) break;\n        }\n    }\n\n    static void Main() {\n        int[] data = {64, 34, 25, 12, 22, 11, 90};\n        Sort(data);\n        Console.WriteLine(string.Join(\", \", data));\n    }\n}", "C# bubble sort — O(n²) with early-exit."),
            "kotlin" => ("fun bubbleSort(arr: IntArray) {\n    val n = arr.size\n    for (i in 0 until n) {\n        var swapped = false\n        for (j in 0 until n - i - 1) {\n            if (arr[j] > arr[j+1]) {\n                arr[j] = arr[j+1].also { arr[j+1] = arr[j] }\n                swapped = true\n            }\n        }\n        if (!swapped) break\n    }\n}\n\nfun main() {\n    val data = intArrayOf(64, 34, 25, 12, 22, 11, 90)\n    bubbleSort(data)\n    println(data.joinToString())\n}", "Kotlin bubble sort — O(n²) with early-exit."),
            "swift" => ("func bubbleSort(_ arr: inout [Int]) {\n    let n = arr.count\n    for i in 0..<n {\n        var swapped = false\n        for j in 0..<(n - i - 1) {\n            if arr[j] > arr[j+1] {\n                arr.swapAt(j, j+1)\n                swapped = true\n            }\n        }\n        if !swapped { break }\n    }\n}\n\nvar data = [64, 34, 25, 12, 22, 11, 90]\nbubbleSort(&data)\nprint(data)", "Swift bubble sort — O(n²) with early-exit."),
            "ruby" => ("def bubble_sort(arr)\n  n = arr.length\n  n.times do |i|\n    swapped = false\n    (n - i - 1).times do |j|\n      if arr[j] > arr[j + 1]\n        arr[j], arr[j + 1] = arr[j + 1], arr[j]\n        swapped = true\n      end\n    end\n    break unless swapped\n  end\n  arr\nend\n\ndata = [64, 34, 25, 12, 22, 11, 90]\nputs bubble_sort(data).inspect", "Ruby bubble sort — O(n²) with early-exit."),
            "php" => ("<?php\nfunction bubbleSort(array &$arr): void {\n    $n = count($arr);\n    for ($i = 0; $i < $n; $i++) {\n        $swapped = false;\n        for ($j = 0; $j < $n - $i - 1; $j++) {\n            if ($arr[$j] > $arr[$j+1]) {\n                [$arr[$j], $arr[$j+1]] = [$arr[$j+1], $arr[$j]];\n                $swapped = true;\n            }\n        }\n        if (!$swapped) break;\n    }\n}\n\n$data = [64, 34, 25, 12, 22, 11, 90];\nbubbleSort($data);\nprint_r($data);\n?>", "PHP bubble sort — O(n²) with early-exit."),
            "killer" => ("kfn bubble_sort(arr) {\n  n = len(arr)\n  i = 0\n  while i < n {\n    swapped = false\n    j = 0\n    while j < n - i - 1 {\n      if arr[j] > arr[j + 1] {\n        tmp = arr[j]; arr[j] = arr[j+1]; arr[j+1] = tmp\n        swapped = true\n      }\n      j = j + 1\n    }\n    if not swapped { break }\n    i = i + 1\n  }\n  arr\n}\n\ndata = [64, 34, 25, 12, 22, 11, 90]\nprint(bubble_sort(data))", "Bubble sort in Killer — O(n²) with early-exit."),
            "scala" => ("object BubbleSort {\n  def bubbleSort(arr: Array[Int]): Unit = {\n    val n = arr.length\n    for (i <- 0 until n) {\n      var swapped = false\n      for (j <- 0 until n - i - 1) {\n        if (arr(j) > arr(j+1)) {\n          val tmp = arr(j); arr(j) = arr(j+1); arr(j+1) = tmp\n          swapped = true\n        }\n      }\n      if (!swapped) return\n    }\n  }\n\n  def main(args: Array[String]): Unit = {\n    val data = Array(64, 34, 25, 12, 22, 11, 90)\n    bubbleSort(data)\n    println(data.mkString(\", \"))\n  }\n}", "Scala bubble sort — O(n²) with early-exit."),
            "dart" => ("void bubbleSort(List<int> arr) {\n  int n = arr.length;\n  for (int i = 0; i < n; i++) {\n    bool swapped = false;\n    for (int j = 0; j < n - i - 1; j++) {\n      if (arr[j] > arr[j+1]) {\n        int tmp = arr[j]; arr[j] = arr[j+1]; arr[j+1] = tmp;\n        swapped = true;\n      }\n    }\n    if (!swapped) break;\n  }\n}\n\nvoid main() {\n  var data = [64, 34, 25, 12, 22, 11, 90];\n  bubbleSort(data);\n  print(data);\n}", "Dart bubble sort — O(n²) with early-exit."),
            "bash" => ("#!/bin/bash\narr=(64 34 25 12 22 11 90)\nn=${#arr[@]}\nfor ((i=0; i<n; i++)); do\n  swapped=0\n  for ((j=0; j<n-i-1; j++)); do\n    if [ ${arr[$j]} -gt ${arr[$((j+1))]} ]; then\n      tmp=${arr[$j]}; arr[$j]=${arr[$((j+1))]}; arr[$((j+1))]=$tmp\n      swapped=1\n    fi\n  done\n  [ $swapped -eq 0 ] && break\ndone\necho \"${arr[@]}\"", "Bash bubble sort — O(n²) with early-exit."),
            _ => ("# O(n²) time, O(1) space — simple but slow for large data\ndef bubble_sort(arr):\n    n = len(arr)\n    for i in range(n):\n        swapped = False\n        for j in range(0, n - i - 1):\n            if arr[j] > arr[j + 1]:\n                arr[j], arr[j + 1] = arr[j + 1], arr[j]\n                swapped = True\n        if not swapped:\n            break\n\ndata = [64, 34, 25, 12, 22, 11, 90]\nbubble_sort(data)\nprint(data)", "Bubble sort — O(n²) with early-exit."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Fibonacci ─────────────────────────────────────────────────────────────
    if q.contains("fibonacci") || q.contains("fib sequence") {
        let (code, note) = match lang {
            "killer" => ("kfn fib(n) {\n  if n <= 1 { n }\n  else { fib(n - 1) + fib(n - 2) }\n}\n\n# Iterative version (faster, O(n))\nkfn fib_iter(n) {\n  a = 0; b = 1; i = 0\n  while i < n {\n    tmp = a + b\n    a = b; b = tmp; i = i + 1\n  }\n  a\n}\n\nprint(fib(10))       # 55 (recursive)\nprint(fib_iter(10))  # 55 (iterative)", "Fibonacci in Killer — recursive and iterative versions."),
            "rust"   => ("// Iterative — O(n) time, O(1) space\nfn fib(n: u64) -> u64 {\n    if n <= 1 { return n; }\n    let (mut a, mut b) = (0u64, 1u64);\n    for _ in 2..=n {\n        let tmp = a + b;\n        a = b; b = tmp;\n    }\n    b\n}\n\nfn main() {\n    for i in 0..10 {\n        print!(\"{} \", fib(i));\n    }\n    // 0 1 1 2 3 5 8 13 21 34\n}", "Fibonacci in Rust — iterative, no overflow up to fib(93)."),
            _ => ("# Three approaches — pick the right one for your use case\nfrom functools import lru_cache\n\n# 1. Memoised recursive — O(n) time, O(n) space\n@lru_cache(maxsize=None)\ndef fib_memo(n: int) -> int:\n    if n <= 1: return n\n    return fib_memo(n - 1) + fib_memo(n - 2)\n\n# 2. Iterative — O(n) time, O(1) space  ← recommended\ndef fib(n: int) -> int:\n    a, b = 0, 1\n    for _ in range(n):\n        a, b = b, a + b\n    return a\n\n# 3. Generator — yields infinite sequence\ndef fib_gen():\n    a, b = 0, 1\n    while True:\n        yield a\n        a, b = b, a + b\n\n# Usage\nprint([fib(i) for i in range(10)])  # [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]\nprint(fib(50))  # 12586269025", "Fibonacci in Python — 3 approaches: memoised, iterative, generator."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Factorial ─────────────────────────────────────────────────────────────
    if q.contains("factorial") {
        let (code, note) = match lang {
            "killer" => ("kfn factorial(n) {\n  if n <= 1 { 1 }\n  else { n * factorial(n - 1) }\n}\n\nkfn factorial_iter(n) {\n  result = 1; i = 2\n  while i <= n { result = result * i; i = i + 1 }\n  result\n}\n\nprint(factorial(10))       # 3628800\nprint(factorial_iter(10))  # 3628800", "Factorial in Killer."),
            "rust"   => ("fn factorial(n: u64) -> u64 {\n    (1..=n).product()\n}\n\nfn main() {\n    println!(\"{}\", factorial(10)); // 3628800\n    println!(\"{}\", factorial(20)); // 2432902008176640000\n}", "Factorial in Rust — uses iterator product."),
            _ => ("import math\n\n# Iterative — O(n) time\ndef factorial(n: int) -> int:\n    \"\"\"Returns n! for n >= 0.\"\"\"\n    if n < 0: raise ValueError(\"n must be non-negative\")\n    result = 1\n    for i in range(2, n + 1):\n        result *= i\n    return result\n\n# Or use the built-in:\nprint(math.factorial(10))    # 3628800\nprint(factorial(10))         # 3628800\nprint(factorial(0))          # 1", "Factorial in Python — iterative and stdlib."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Linked List ───────────────────────────────────────────────────────────
    if q.contains("linked list") || q.contains("linkedlist") {
        let (code, note) = match lang {
            "rust" => ("use std::boxed::Box;\n\nstruct Node<T> {\n    val: T,\n    next: Option<Box<Node<T>>>,\n}\n\nstruct LinkedList<T> {\n    head: Option<Box<Node<T>>>,\n    len:  usize,\n}\n\nimpl<T> LinkedList<T> {\n    fn new() -> Self { LinkedList { head: None, len: 0 } }\n\n    fn push_front(&mut self, val: T) {\n        let node = Box::new(Node { val, next: self.head.take() });\n        self.head = Some(node);\n        self.len += 1;\n    }\n\n    fn pop_front(&mut self) -> Option<T> {\n        self.head.take().map(|node| {\n            self.head = node.next;\n            self.len -= 1;\n            node.val\n        })\n    }\n\n    fn len(&self) -> usize { self.len }\n}\n\nfn main() {\n    let mut list: LinkedList<i32> = LinkedList::new();\n    list.push_front(3);\n    list.push_front(2);\n    list.push_front(1);\n    println!(\"len={}\", list.len());\n    while let Some(v) = list.pop_front() {\n        print!(\"{} \", v); // 1 2 3\n    }\n}", "Singly linked list in Rust with generics."),
            _ => ("class Node:\n    def __init__(self, val):\n        self.val  = val\n        self.next = None\n\nclass LinkedList:\n    \"\"\"Singly linked list with O(1) push_front and O(n) push_back.\"\"\"\n    def __init__(self):\n        self.head = None\n        self.size = 0\n\n    def push_front(self, val):\n        node = Node(val)\n        node.next = self.head\n        self.head = node\n        self.size += 1\n\n    def push_back(self, val):\n        node = Node(val)\n        if not self.head:\n            self.head = node\n        else:\n            cur = self.head\n            while cur.next:\n                cur = cur.next\n            cur.next = node\n        self.size += 1\n\n    def pop_front(self):\n        if not self.head:\n            return None\n        val = self.head.val\n        self.head = self.head.next\n        self.size -= 1\n        return val\n\n    def to_list(self):\n        result, cur = [], self.head\n        while cur:\n            result.append(cur.val)\n            cur = cur.next\n        return result\n\n    def __len__(self): return self.size\n    def __repr__(self): return ' -> '.join(map(str, self.to_list()))\n\n# Usage\nll = LinkedList()\nll.push_back(1); ll.push_back(2); ll.push_back(3)\nprint(ll)            # 1 -> 2 -> 3\nprint(ll.pop_front()) # 1\nprint(len(ll))        # 2", "Singly linked list in Python with push_front, push_back, pop_front."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Stack ─────────────────────────────────────────────────────────────────
    if q.contains(" stack") || q.starts_with("stack") || q.contains("push") && q.contains("pop") && !q.contains("linked") {
        let code = "class Stack:\n    \"\"\"LIFO stack with O(1) push, pop, peek.\"\"\"\n    def __init__(self):\n        self._data = []\n\n    def push(self, item):\n        self._data.append(item)\n\n    def pop(self):\n        if self.is_empty():\n            raise IndexError('pop from empty stack')\n        return self._data.pop()\n\n    def peek(self):\n        if self.is_empty():\n            raise IndexError('peek from empty stack')\n        return self._data[-1]\n\n    def is_empty(self) -> bool: return len(self._data) == 0\n    def __len__(self):          return len(self._data)\n    def __repr__(self):         return f'Stack({self._data})'\n\n# Usage\ns = Stack()\ns.push(1); s.push(2); s.push(3)\nprint(s.peek())  # 3\nprint(s.pop())   # 3\nprint(len(s))    # 2";
        return format!("```python\n{}\n```\n\n*Stack class in Python — O(1) push/pop/peek.*", code);
    }

    // ── Queue ─────────────────────────────────────────────────────────────────
    if q.contains(" queue") || q.starts_with("queue") || q.contains("enqueue") || q.contains("dequeue") {
        let code = "from collections import deque\n\nclass Queue:\n    \"\"\"FIFO queue with O(1) enqueue and dequeue.\"\"\"\n    def __init__(self):\n        self._data = deque()\n\n    def enqueue(self, item):\n        self._data.append(item)\n\n    def dequeue(self):\n        if self.is_empty():\n            raise IndexError('dequeue from empty queue')\n        return self._data.popleft()\n\n    def peek(self):           return self._data[0] if self._data else None\n    def is_empty(self) -> bool: return len(self._data) == 0\n    def __len__(self):          return len(self._data)\n    def __repr__(self):         return f'Queue({list(self._data)})'\n\n# Usage\nq = Queue()\nq.enqueue('a'); q.enqueue('b'); q.enqueue('c')\nprint(q.dequeue())  # a\nprint(q.peek())     # b\nprint(len(q))       # 2";
        return format!("```python\n{}\n```\n\n*Queue class in Python — O(1) enqueue/dequeue using deque.*", code);
    }

    // ── Calculator class / OOP ─────────────────────────────────────────────────
    // Guard: don't match "calculator" if it's a unit-test request
    let is_test_req = q.contains("unit test") || q.contains("unittest") || q.contains("pytest") || q.contains("test case") || q.contains("write test");
    if !is_test_req && (q.contains("calculator") || (q.contains("class") && (q.contains("add") || q.contains("multiply") || q.contains("divide")))) {
        let (code, note) = match lang {
            "killer" => ("kfn make_calculator() {\n  # Return a calculator object as a closure map\n  {\n    add:      kfn(a, b) { a + b },\n    subtract: kfn(a, b) { a - b },\n    multiply: kfn(a, b) { a * b },\n    divide:   kfn(a, b) {\n      if b == 0 { print(K\"Error: division by zero\"); 0 }\n      else { a / b }\n    }\n  }\n}\n\ncalc = make_calculator()\nprint(calc.add(10, 5))       # 15\nprint(calc.subtract(10, 5))  # 5\nprint(calc.multiply(10, 5))  # 50\nprint(calc.divide(10, 5))    # 2.0\nprint(calc.divide(10, 0))    # Error: division by zero", "Calculator in Killer using closures."),
            _ => ("class Calculator:\n    \"\"\"Basic four-operation calculator.\"\"\"\n\n    def __init__(self):\n        self.history = []\n\n    def add(self, a: float, b: float) -> float:\n        result = a + b\n        self.history.append(f\"{a} + {b} = {result}\")\n        return result\n\n    def subtract(self, a: float, b: float) -> float:\n        result = a - b\n        self.history.append(f\"{a} - {b} = {result}\")\n        return result\n\n    def multiply(self, a: float, b: float) -> float:\n        result = a * b\n        self.history.append(f\"{a} * {b} = {result}\")\n        return result\n\n    def divide(self, a: float, b: float) -> float:\n        if b == 0:\n            raise ZeroDivisionError(\"Cannot divide by zero\")\n        result = a / b\n        self.history.append(f\"{a} / {b} = {result}\")\n        return result\n\n    def show_history(self):\n        for entry in self.history:\n            print(entry)\n\n# Usage\ncalc = Calculator()\nprint(calc.add(10, 5))       # 15.0\nprint(calc.subtract(10, 3))  # 7.0\nprint(calc.multiply(4, 6))   # 24.0\nprint(calc.divide(15, 4))    # 3.75\ncalc.show_history()", "Calculator class in Python with history tracking."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Unit Tests / Pytest ────────────────────────────────────────────────────
    if q.contains("unit test") || q.contains("unittest") || q.contains("pytest") || q.contains("write test") || q.contains("test case") {
        let code = "import unittest\n\n# ── Class under test ─────────────────────────────────────────────────────────\nclass Calculator:\n    def add(self, a, b):      return a + b\n    def subtract(self, a, b): return a - b\n    def multiply(self, a, b): return a * b\n    def divide(self, a, b):\n        if b == 0: raise ZeroDivisionError\n        return a / b\n\n# ── Test suite ────────────────────────────────────────────────────────────────\nclass TestCalculator(unittest.TestCase):\n\n    def setUp(self):\n        self.calc = Calculator()\n\n    def test_add_positive(self):\n        self.assertEqual(self.calc.add(3, 4), 7)\n\n    def test_add_negative(self):\n        self.assertEqual(self.calc.add(-2, -3), -5)\n\n    def test_subtract(self):\n        self.assertEqual(self.calc.subtract(10, 3), 7)\n\n    def test_multiply(self):\n        self.assertEqual(self.calc.multiply(4, 6), 24)\n\n    def test_divide(self):\n        self.assertAlmostEqual(self.calc.divide(10, 3), 3.333, places=3)\n\n    def test_divide_by_zero(self):\n        with self.assertRaises(ZeroDivisionError):\n            self.calc.divide(5, 0)\n\nif __name__ == '__main__':\n    unittest.main(verbosity=2)";
        return format!("```python\n{}\n```\n\n*Python unittest suite for a Calculator class — run: `python -m pytest test_calc.py -v`*", code);
    }

    // ── Regex / Email validation ───────────────────────────────────────────────
    if q.contains("regex") || q.contains("validate email") || (q.contains("email") && q.contains("valid")) {
        let code = "import re\n\n# RFC-5321 simplified email pattern\nEMAIL_PATTERN = re.compile(\n    r'^[a-zA-Z0-9._%+\\-]+@[a-zA-Z0-9.\\-]+\\.[a-zA-Z]{2,}$'\n)\n\ndef is_valid_email(email: str) -> bool:\n    \"\"\"Returns True if email has valid format.\"\"\"\n    return bool(EMAIL_PATTERN.match(email.strip()))\n\ndef validate_emails(emails: list[str]) -> dict:\n    \"\"\"Returns a dict of email -> bool for a list.\"\"\"\n    return {e: is_valid_email(e) for e in emails}\n\n# Usage\ntest_cases = [\n    'user@example.com',       # True\n    'first.last@domain.co',   # True\n    'invalid-email',          # False\n    '@nodomain.com',          # False\n    'spaces @bad.com',        # False\n    'ok+tag@gmail.com',       # True\n]\nfor email, valid in validate_emails(test_cases).items():\n    print(f\"{email!r:35} -> {valid}\")";
        return format!("```python\n{}\n```\n\n*Email validation with compiled regex in Python — O(n) per check.*", code);
    }

    // ── HTTP Server ───────────────────────────────────────────────────────────
    if q.contains("http server") || q.contains("web server") || (q.contains("server") && (q.contains("port") || q.contains("listen") || q.contains("request"))) {
        let (code, note) = match lang {
            "killer" => ("# Kala/Killer TCP HTTP server\nkfn handle_request(conn) {\n  req  = conn.read()\n  body = K\"<h1>Hello from Killer!</h1><p>Time: {now()}</p>\"\n  resp = K\"HTTP/1.1 200 OK\\r\\nContent-Type: text/html\\r\\nContent-Length: {len(body)}\\r\\n\\r\\n{body}\"\n  conn.write(resp)\n  conn.close()\n}\n\nserver = tcp_listen(\"0.0.0.0\", 8080)\nprint(K\"Server running on http://localhost:8080\")\nwhile true {\n  conn = server.accept()\n  spawn handle_request(conn)\n}", "Minimal HTTP server in Killer using actors."),
            "javascript" => ("const http = require('http');\n\nconst server = http.createServer((req, res) => {\n    if (req.url === '/health') {\n        res.writeHead(200, { 'Content-Type': 'application/json' });\n        res.end(JSON.stringify({ status: 'ok', ts: Date.now() }));\n        return;\n    }\n    res.writeHead(200, { 'Content-Type': 'text/html' });\n    res.end('<h1>Hello from Node.js!</h1>');\n});\n\nserver.listen(3000, () => {\n    console.log('Server running at http://localhost:3000/');\n});", "HTTP server in Node.js — GET / and GET /health"),
            _ => ("from http.server import BaseHTTPRequestHandler, HTTPServer\nimport json\n\nclass Handler(BaseHTTPRequestHandler):\n    def log_message(self, *args): pass  # silence default logs\n\n    def do_GET(self):\n        if self.path == '/health':\n            self._json({'status': 'ok'})\n        elif self.path == '/':\n            self._html('<h1>Hello from Python!</h1>')\n        else:\n            self._json({'error': 'not found'}, 404)\n\n    def do_POST(self):\n        length = int(self.headers.get('Content-Length', 0))\n        body   = json.loads(self.rfile.read(length) or '{}')\n        self._json({'received': body})\n\n    def _html(self, body, status=200):\n        data = body.encode()\n        self.send_response(status)\n        self.send_header('Content-Type', 'text/html')\n        self.send_header('Content-Length', len(data))\n        self.end_headers()\n        self.wfile.write(data)\n\n    def _json(self, obj, status=200):\n        data = json.dumps(obj).encode()\n        self.send_response(status)\n        self.send_header('Content-Type', 'application/json')\n        self.send_header('Content-Length', len(data))\n        self.end_headers()\n        self.wfile.write(data)\n\nif __name__ == '__main__':\n    server = HTTPServer(('', 8080), Handler)\n    print('Listening on http://localhost:8080')\n    server.serve_forever()", "HTTP server in Python stdlib — GET, POST, /health endpoint."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── File Read / Write ─────────────────────────────────────────────────────
    if (q.contains("file") && (q.contains("read") || q.contains("write") || q.contains("io"))) || q.contains("read a file") || q.contains("write to file") {
        let code = "from pathlib import Path\nimport json\n\n# ── Read a text file ─────────────────────────────────────────────────────────\ndef read_text(path: str) -> str:\n    \"\"\"Read entire file as string.\"\"\"\n    return Path(path).read_text(encoding='utf-8')\n\n# ── Write a text file ────────────────────────────────────────────────────────\ndef write_text(path: str, content: str):\n    \"\"\"Write string to file (creates or overwrites).\"\"\"\n    Path(path).write_text(content, encoding='utf-8')\n\n# ── Read lines ───────────────────────────────────────────────────────────────\ndef read_lines(path: str) -> list[str]:\n    \"\"\"Read file as list of lines (strips newlines).\"\"\"\n    return Path(path).read_text(encoding='utf-8').splitlines()\n\n# ── Read JSON ────────────────────────────────────────────────────────────────\ndef read_json(path: str) -> dict:\n    with open(path, 'r', encoding='utf-8') as f:\n        return json.load(f)\n\n# ── Write JSON ───────────────────────────────────────────────────────────────\ndef write_json(path: str, data, indent: int = 2):\n    with open(path, 'w', encoding='utf-8') as f:\n        json.dump(data, f, indent=indent, ensure_ascii=False)\n\n# ── Usage ────────────────────────────────────────────────────────────────────\nwrite_text('hello.txt', 'Hello, World!\\nLine 2')\nprint(read_text('hello.txt'))\n\nwrite_json('data.json', {'name': 'Kala', 'version': 2})\nprint(read_json('data.json'))";
        return format!("```python\n{}\n```\n\n*File read/write utilities in Python — text, lines, and JSON.*", code);
    }

    // ── Async / Fetch (JavaScript) ─────────────────────────────────────────────
    if q.contains("async") || q.contains("await") || q.contains("fetch") || q.contains("promise") {
        let (code, note) = match lang {
            "javascript" | "typescript" => {
                let code = if lang == "typescript" {
                    "interface ApiResponse {\n    userId: number;\n    id: number;\n    title: string;\n    completed: boolean;\n}\n\nasync function fetchTodo(id: number): Promise<ApiResponse> {\n    const res = await fetch(`https://jsonplaceholder.typicode.com/todos/${id}`);\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json() as Promise<ApiResponse>;\n}\n\nasync function main() {\n    try {\n        const todo = await fetchTodo(1);\n        console.log(`Title: ${todo.title}, done: ${todo.completed}`);\n    } catch (err) {\n        console.error('Fetch failed:', err);\n    }\n}\n\nmain();"
                } else {
                    "// Fetch with error handling and JSON parsing\nasync function fetchData(url) {\n    const res = await fetch(url);\n    if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);\n    return res.json();\n}\n\n// POST with JSON body\nasync function postData(url, data) {\n    const res = await fetch(url, {\n        method:  'POST',\n        headers: { 'Content-Type': 'application/json' },\n        body:    JSON.stringify(data),\n    });\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json();\n}\n\n// Usage\n(async () => {\n    const todo = await fetchData('https://jsonplaceholder.typicode.com/todos/1');\n    console.log(todo.title);\n\n    const created = await postData('https://jsonplaceholder.typicode.com/posts', {\n        title: 'My Post', body: 'Hello', userId: 1\n    });\n    console.log('Created id:', created.id);\n})();"
                };
                (code, if lang == "typescript" { "Async/await with TypeScript types — fetch and error handling." } else { "Async/await fetch in JavaScript — GET and POST with error handling." })
            },
            _ => ("import asyncio\nimport aiohttp\n\nasync def fetch(session: aiohttp.ClientSession, url: str) -> dict:\n    \"\"\"Fetch JSON from URL asynchronously.\"\"\"\n    async with session.get(url) as resp:\n        resp.raise_for_status()\n        return await resp.json()\n\nasync def fetch_many(urls: list[str]) -> list[dict]:\n    \"\"\"Fetch multiple URLs concurrently.\"\"\"\n    async with aiohttp.ClientSession() as session:\n        tasks = [fetch(session, url) for url in urls]\n        return await asyncio.gather(*tasks)\n\nasync def main():\n    urls = [\n        'https://jsonplaceholder.typicode.com/todos/1',\n        'https://jsonplaceholder.typicode.com/todos/2',\n        'https://jsonplaceholder.typicode.com/todos/3',\n    ]\n    results = await fetch_many(urls)\n    for r in results:\n        print(r['title'])\n\nif __name__ == '__main__':\n    asyncio.run(main())", "Async HTTP fetch in Python with aiohttp — concurrent requests with asyncio.gather."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── SQL: CREATE TABLE ──────────────────────────────────────────────────────
    if (lang == "sql" || q.contains("sql") || q.contains("tsql") || q.contains("t-sql"))
       && (q.contains("create table") || q.contains("create a table") || q.contains("table for")
           || q.contains("create tsql") || q.contains("table script") || q.contains("table schema")) {
        // Extract entity name from query: "create table for X company" -> X
        let entity = {
            let mut name = String::from("company");
            if let Some(pos) = q.find(" for ") {
                let after = q[pos + 5..].trim();
                // Take words until "company"/"database"/"table"
                let words: Vec<&str> = after.split_whitespace()
                    .take_while(|w| !matches!(*w, "company"|"database"|"table"|"schema"|"script"))
                    .collect();
                if !words.is_empty() {
                    name = words.join("_");
                }
            }
            name
        };
        let code = format!(
            "-- Database schema for {entity}\n\n\
             CREATE TABLE employees (\n\
                 employee_id   INT           PRIMARY KEY IDENTITY(1,1),\n\
                 first_name    NVARCHAR(100) NOT NULL,\n\
                 last_name     NVARCHAR(100) NOT NULL,\n\
                 email         NVARCHAR(255) NOT NULL UNIQUE,\n\
                 phone         NVARCHAR(20)  NULL,\n\
                 hire_date     DATE          NOT NULL DEFAULT GETDATE(),\n\
                 department_id INT           NULL,\n\
                 salary        DECIMAL(12,2) NOT NULL DEFAULT 0,\n\
                 is_active     BIT           NOT NULL DEFAULT 1\n\
             );\n\n\
             CREATE TABLE departments (\n\
                 department_id   INT           PRIMARY KEY IDENTITY(1,1),\n\
                 department_name NVARCHAR(100) NOT NULL UNIQUE,\n\
                 manager_id      INT           NULL,\n\
                 budget          DECIMAL(15,2) NULL,\n\
                 created_at      DATETIME2     NOT NULL DEFAULT GETDATE()\n\
             );\n\n\
             CREATE TABLE orders (\n\
                 order_id    INT           PRIMARY KEY IDENTITY(1,1),\n\
                 customer_id INT           NOT NULL,\n\
                 order_date  DATETIME2     NOT NULL DEFAULT GETDATE(),\n\
                 total       DECIMAL(12,2) NOT NULL,\n\
                 status      NVARCHAR(20)  NOT NULL DEFAULT 'pending'\n\
             );\n\n\
             -- Foreign keys\n\
             ALTER TABLE employees ADD CONSTRAINT FK_emp_dept\n\
                 FOREIGN KEY (department_id) REFERENCES departments(department_id);\n\n\
             ALTER TABLE departments ADD CONSTRAINT FK_dept_mgr\n\
                 FOREIGN KEY (manager_id) REFERENCES employees(employee_id);\n\n\
             -- Index for fast lookups\n\
             CREATE INDEX IX_emp_email ON employees(email);\n\
             CREATE INDEX IX_orders_customer ON orders(customer_id);",
            entity = entity
        );
        return format!("```sql\n{}\n```\n\n*T-SQL CREATE TABLE script for {} — employees, departments, orders with foreign keys and indexes.*", code, entity);
    }

    // ── SQL: SELECT queries (generic) ─────────────────────────────────────────
    if lang == "sql" || q.contains("sql query") || q.contains("select query") || (q.contains("join") && q.contains("table")) {
        let code = "-- Top 10 customers by total purchase amount\nSELECT\n    c.customer_id,\n    c.name,\n    c.email,\n    COUNT(o.order_id)        AS total_orders,\n    SUM(o.amount)            AS total_spent,\n    AVG(o.amount)            AS avg_order_value,\n    MAX(o.created_at)        AS last_order_date\nFROM customers c\nJOIN orders o ON o.customer_id = c.customer_id\nWHERE o.status = 'completed'\n  AND o.created_at >= DATE_SUB(NOW(), INTERVAL 1 YEAR)\nGROUP BY c.customer_id, c.name, c.email\nHAVING total_orders >= 2\nORDER BY total_spent DESC\nLIMIT 10;\n\n-- Find duplicate emails\nSELECT email, COUNT(*) AS cnt\nFROM users\nGROUP BY email\nHAVING cnt > 1\nORDER BY cnt DESC;\n\n-- Running total (window function)\nSELECT\n    order_id,\n    amount,\n    SUM(amount) OVER (ORDER BY created_at) AS running_total\nFROM orders\nORDER BY created_at;";
        return format!("```sql\n{}\n```\n\n*SQL examples: top customers JOIN, duplicate detection, running total window function.*", code);
    }

    // ── Palindrome ────────────────────────────────────────────────────────────
    if q.contains("palindrome") {
        let code = "def is_palindrome(s: str) -> bool:\n    \"\"\"Check if string is a palindrome (ignores case and non-alpha chars).\"\"\"\n    clean = ''.join(c.lower() for c in s if c.isalnum())\n    return clean == clean[::-1]\n\ndef is_palindrome_number(n: int) -> bool:\n    \"\"\"Check if integer is a palindrome without converting to string.\"\"\"\n    if n < 0 or (n % 10 == 0 and n != 0):\n        return False\n    rev = 0\n    while n > rev:\n        rev = rev * 10 + n % 10\n        n //= 10\n    return n == rev or n == rev // 10\n\n# Usage\nprint(is_palindrome('A man a plan a canal Panama'))  # True\nprint(is_palindrome('race a car'))                   # False\nprint(is_palindrome_number(121))   # True\nprint(is_palindrome_number(10))    # False";
        return format!("```python\n{}\n```\n\n*Palindrome check in Python — strings and integers.*", code);
    }

    // ── Two Sum (LeetCode-style) ───────────────────────────────────────────────
    if q.contains("two sum") || (q.contains("two") && q.contains("sum") && q.contains("target")) {
        let code = "# O(n) time, O(n) space — hash map approach\ndef two_sum(nums: list[int], target: int) -> list[int]:\n    \"\"\"\n    Returns indices [i, j] where nums[i] + nums[j] == target.\n    Returns [] if no solution exists.\n    \"\"\"\n    seen = {}  # value -> index\n    for i, num in enumerate(nums):\n        complement = target - num\n        if complement in seen:\n            return [seen[complement], i]\n        seen[num] = i\n    return []\n\n# Usage\nprint(two_sum([2, 7, 11, 15], 9))   # [0, 1]\nprint(two_sum([3, 2, 4], 6))        # [1, 2]\nprint(two_sum([3, 3], 6))           # [0, 1]\nprint(two_sum([1, 2, 3], 10))       # []";
        return format!("```python\n{}\n```\n\n*Two Sum in Python — O(n) hash map solution.*", code);
    }

    // ── Prime / Sieve of Eratosthenes ─────────────────────────────────────────
    if q.contains("prime") || q.contains("sieve") {
        let code = "def sieve_of_eratosthenes(n: int) -> list[int]:\n    \"\"\"Return all primes up to n using Sieve of Eratosthenes. O(n log log n).\"\"\"\n    is_prime = [True] * (n + 1)\n    is_prime[0] = is_prime[1] = False\n    for i in range(2, int(n**0.5) + 1):\n        if is_prime[i]:\n            for j in range(i*i, n + 1, i):\n                is_prime[j] = False\n    return [i for i, p in enumerate(is_prime) if p]\n\ndef is_prime(n: int) -> bool:\n    \"\"\"Check if n is prime. O(sqrt(n)).\"\"\"\n    if n < 2: return False\n    if n == 2: return True\n    if n % 2 == 0: return False\n    for i in range(3, int(n**0.5) + 1, 2):\n        if n % i == 0: return False\n    return True\n\n# Usage\nprint(sieve_of_eratosthenes(50))  # [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]\nprint(is_prime(97))  # True\nprint(is_prime(100)) # False";
        return format!("```python\n{}\n```\n\n*Sieve of Eratosthenes + primality test in Python.*", code);
    }

    // ── Reverse string / array ─────────────────────────────────────────────────
    if q.contains("reverse") && (q.contains("string") || q.contains("array") || q.contains("list")) {
        let wants_func = q.contains("function") || q.contains("method") || q.contains("def ");
        let target_is_string = q.contains("string");
        let (code, note) = match lang {
            "rust" => if target_is_string {
                ("fn reverse_string(s: &str) -> String {\n    s.chars().rev().collect()\n}\n\nfn main() {\n    println!(\"{}\", reverse_string(\"Hello, World!\")); // !dlroW ,olleH\n    println!(\"{}\", reverse_string(\"racecar\"));        // racecar (palindrome)\n}", "Reverse a string in Rust — chars().rev().collect().")
            } else {
                ("fn reverse_array<T: Clone>(arr: &[T]) -> Vec<T> {\n    arr.iter().rev().cloned().collect()\n}\n\nfn reverse_in_place<T>(arr: &mut [T]) {\n    arr.reverse();\n}\n\nfn main() {\n    let nums = vec![1, 2, 3, 4, 5];\n    println!(\"{:?}\", reverse_array(&nums)); // [5, 4, 3, 2, 1]\n\n    let mut v = vec![10, 20, 30];\n    reverse_in_place(&mut v);\n    println!(\"{:?}\", v); // [30, 20, 10]\n}", "Reverse array in Rust — iter().rev() and in-place .reverse().")
            },
            "javascript" | "typescript" => if target_is_string {
                ("function reverseString(s) {\n    return s.split('').reverse().join('');\n}\n\nconsole.log(reverseString('Hello, World!')); // !dlroW ,olleH\nconsole.log(reverseString('racecar'));        // racecar (palindrome)", "Reverse a string in JavaScript — split/reverse/join.")
            } else {
                ("function reverseArray(arr) {\n    return [...arr].reverse();\n}\n\nconst nums = [1, 2, 3, 4, 5];\nconsole.log(reverseArray(nums)); // [5, 4, 3, 2, 1]\nconsole.log(nums);               // [1, 2, 3, 4, 5] — original unchanged", "Reverse array in JavaScript — spread + .reverse().")
            },
            "java" => if target_is_string {
                ("public class ReverseString {\n    public static String reverse(String s) {\n        return new StringBuilder(s).reverse().toString();\n    }\n\n    public static void main(String[] args) {\n        System.out.println(reverse(\"Hello, World!\")); // !dlroW ,olleH\n        System.out.println(reverse(\"racecar\"));        // racecar\n    }\n}", "Reverse a string in Java — StringBuilder.reverse().")
            } else {
                ("import java.util.*;\n\npublic class ReverseArray {\n    public static <T> List<T> reverse(List<T> list) {\n        List<T> copy = new ArrayList<>(list);\n        Collections.reverse(copy);\n        return copy;\n    }\n\n    public static void main(String[] args) {\n        var nums = List.of(1, 2, 3, 4, 5);\n        System.out.println(reverse(nums)); // [5, 4, 3, 2, 1]\n    }\n}", "Reverse a list in Java — Collections.reverse().")
            },
            _ => if target_is_string && wants_func {
                ("def reverse_string(s: str) -> str:\n    \"\"\"Reverse a string.\"\"\"\n    return s[::-1]\n\n\n# Usage\nprint(reverse_string('Hello, World!'))  # !dlroW ,olleH\nprint(reverse_string('racecar'))        # racecar (palindrome)\nprint(reverse_string(''))               # '' (empty string)", "Reverse a string in Python — clean function using slicing.")
            } else if target_is_string {
                ("def reverse_string(s: str) -> str:\n    \"\"\"Reverse a string — 3 approaches.\"\"\"\n    return s[::-1]\n\n# Alternative: using reversed()\ndef reverse_string_v2(s: str) -> str:\n    return ''.join(reversed(s))\n\n# Alternative: manual loop (interview style)\ndef reverse_string_v3(s: str) -> str:\n    chars = list(s)\n    lo, hi = 0, len(chars) - 1\n    while lo < hi:\n        chars[lo], chars[hi] = chars[hi], chars[lo]\n        lo += 1; hi -= 1\n    return ''.join(chars)\n\nprint(reverse_string('Hello, World!'))    # !dlroW ,olleH\nprint(reverse_string_v2('racecar'))       # racecar\nprint(reverse_string_v3('Python'))        # nohtyP", "Reverse a string in Python — 3 approaches: slicing, reversed(), manual swap.")
            } else {
                ("def reverse_list(arr: list) -> list:\n    \"\"\"Return a reversed copy (original unchanged).\"\"\"\n    return arr[::-1]\n\ndef reverse_in_place(arr: list) -> list:\n    \"\"\"Reverse in place — O(1) extra space.\"\"\"\n    lo, hi = 0, len(arr) - 1\n    while lo < hi:\n        arr[lo], arr[hi] = arr[hi], arr[lo]\n        lo += 1; hi -= 1\n    return arr\n\nnums = [1, 2, 3, 4, 5]\nprint(reverse_list(nums))       # [5, 4, 3, 2, 1]\nprint(nums)                     # [1, 2, 3, 4, 5] unchanged\nprint(reverse_in_place(nums))   # [5, 4, 3, 2, 1] in-place", "Reverse list in Python — copy vs in-place approaches.")
            },
        };
        return format!("```{}\n{}\n```\n\n*{}*\n\n---\n💡 **Try next:** *\"Add error handling\"* · *\"Write unit tests for this\"* · *\"Optimize this code\"*", lang, code, note);
    }

    // ── Sort a list ─────────────────────────────────────────────────────────────
    if q.contains("sort") && (q.contains("list") || q.contains("array")) && !q.contains("quicksort") && !q.contains("merge") && !q.contains("bubble") {
        let (code, note) = match lang {
            "killer" => ("# Built-in sort\narr = [3, 1, 4, 1, 5, 9, 2, 6]\nsorted_arr = sort(arr)       # ascending, returns new list\nprint(sorted_arr)\n\n# Sort descending\nreversed_arr = sort_desc(arr)\nprint(reversed_arr)\n\n# Sort objects by field\npeople = [{\"name\": \"Bob\", \"age\": 30}, {\"name\": \"Alice\", \"age\": 25}]\nsorted_people = sort_by(people, \"age\")\nprint(sorted_people)", "Sorting in Killer — built-in sort functions."),
            "rust"   => ("fn main() {\n    let mut nums = vec![3, 1, 4, 1, 5, 9, 2, 6];\n    nums.sort();                                 // ascending in-place\n    println!(\"{:?}\", nums);\n\n    nums.sort_by(|a, b| b.cmp(a));               // descending\n    println!(\"{:?}\", nums);\n\n    let mut words = vec![\"banana\", \"apple\", \"cherry\"];\n    words.sort();                                // lexicographic\n    println!(\"{:?}\", words);\n\n    // Sort structs\n    let mut scores = vec![(\"Alice\", 90), (\"Bob\", 75), (\"Carol\", 85)];\n    scores.sort_by_key(|&(_, score)| std::cmp::Reverse(score)); // highest first\n    println!(\"{:?}\", scores);\n}", "Sorting in Rust — vec.sort(), custom comparators, struct sorting."),
            _ => ("# Python sort — timsort, O(n log n), stable\nnums = [3, 1, 4, 1, 5, 9, 2, 6]\n\n# sorted() — returns new list, original unchanged\nprint(sorted(nums))               # [1, 1, 2, 3, 4, 5, 6, 9]\nprint(sorted(nums, reverse=True)) # [9, 6, 5, 4, 3, 2, 1, 1]\n\n# list.sort() — in-place\nnums.sort()\nprint(nums)\n\n# Sort by key\npeople = [{'name': 'Bob', 'age': 30}, {'name': 'Alice', 'age': 25}, {'name': 'Carol', 'age': 27}]\nby_age  = sorted(people, key=lambda p: p['age'])\nby_name = sorted(people, key=lambda p: p['name'])\nprint([p['name'] for p in by_age])  # ['Alice', 'Carol', 'Bob']\n\n# Sort tuples by second element\npairs = [(1, 'b'), (3, 'a'), (2, 'c')]\nprint(sorted(pairs, key=lambda x: x[1]))  # [(3,'a'), (1,'b'), (2,'c')]", "Sorting in Python — sorted(), in-place sort, key functions."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Rust struct ───────────────────────────────────────────────────────────
    if lang == "rust" && (q.contains("struct") || q.contains("class") || q.contains("impl")) {
        let code = "use std::fmt;\n\n#[derive(Debug, Clone)]\nstruct Person {\n    name: String,\n    age:  u32,\n}\n\nimpl Person {\n    /// Constructor\n    fn new(name: impl Into<String>, age: u32) -> Self {\n        Person { name: name.into(), age }\n    }\n\n    fn greet(&self) -> String {\n        format!(\"Hi, I'm {} and I'm {} years old.\", self.name, self.age)\n    }\n\n    fn is_adult(&self) -> bool {\n        self.age >= 18\n    }\n\n    fn birthday(&mut self) {\n        self.age += 1;\n    }\n}\n\nimpl fmt::Display for Person {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"{}({})\", self.name, self.age)\n    }\n}\n\nfn main() {\n    let mut p = Person::new(\"Alice\", 30);\n    println!(\"{}\", p.greet());\n    println!(\"Adult: {}\", p.is_adult());\n    p.birthday();\n    println!(\"After birthday: {}\", p);\n}";
        return format!("```rust\n{}\n```\n\n*Rust struct with impl, Display, and methods — idiomatic pattern.*", code);
    }

    // ── Hash Map / Dictionary ──────────────────────────────────────────────────
    if q.contains("hash map") || q.contains("hashmap") || q.contains("dictionary") || q.contains(" dict") || q.contains("hash table") || q.contains("hashtable") {
        let (code, note) = match lang {
            "killer" => ("# Killer dictionary (hash map)\nuser = {\"name\": \"Arun\", \"age\": 28, \"role\": \"engineer\"}\nprint(user[\"name\"])    # Arun\n\n# Add / update\nuser[\"city\"] = \"Hyderabad\"\nuser[\"age\"] = 29\n\n# Iterate keys\nkeys = dict_keys(user)\ni = 0\nwhile i < len(keys) {\n  k = keys[i]\n  print(K\"{k}: {user[k]}\")\n  i = i + 1\n}\n\n# Nested dict\nconfig = {\"db\": {\"host\": \"localhost\", \"port\": 5432}, \"debug\": true}\nprint(config[\"db\"][\"host\"])  # localhost", "Hash map (dictionary) in Killer."),
            "rust"   => ("use std::collections::HashMap;\n\nfn main() {\n    let mut scores: HashMap<&str, i32> = HashMap::new();\n\n    // Insert\n    scores.insert(\"Alice\", 95);\n    scores.insert(\"Bob\", 87);\n    scores.insert(\"Carol\", 92);\n\n    // Get\n    if let Some(s) = scores.get(\"Alice\") {\n        println!(\"Alice: {}\", s);\n    }\n\n    // Update - entry API\n    scores.entry(\"Dave\").or_insert(80);\n    *scores.entry(\"Bob\").or_insert(0) += 5;\n\n    // Iterate\n    for (name, score) in &scores {\n        println!(\"{}: {}\", name, score);\n    }\n\n    // Check existence\n    println!(\"Has Alice? {}\", scores.contains_key(\"Alice\"));\n\n    // Word frequency counter\n    let text = \"hello world hello rust world\";\n    let mut freq: HashMap<&str, usize> = HashMap::new();\n    for word in text.split_whitespace() {\n        *freq.entry(word).or_insert(0) += 1;\n    }\n    println!(\"{:?}\", freq);\n}", "HashMap in Rust — insert, get, entry API, word counter."),
            "go"     => ("package main\n\nimport \"fmt\"\n\nfunc main() {\n    // Create map\n    scores := map[string]int{\n        \"Alice\": 95,\n        \"Bob\":   87,\n        \"Carol\": 92,\n    }\n\n    // Access\n    fmt.Println(\"Alice:\", scores[\"Alice\"])\n\n    // Check existence\n    if val, ok := scores[\"Dave\"]; ok {\n        fmt.Println(\"Dave:\", val)\n    } else {\n        fmt.Println(\"Dave not found\")\n    }\n\n    // Add/update\n    scores[\"Dave\"] = 80\n\n    // Delete\n    delete(scores, \"Bob\")\n\n    // Iterate\n    for name, score := range scores {\n        fmt.Printf(\"%s: %d\\n\", name, score)\n    }\n}", "Go map — create, access, check, delete, iterate."),
            "javascript" | "typescript" => ("// Map (preserves insertion order, any key type)\nconst userMap = new Map();\nuserMap.set('name', 'Arun');\nuserMap.set('age', 28);\nuserMap.set('role', 'engineer');\n\nconsole.log(userMap.get('name'));  // Arun\nconsole.log(userMap.has('age'));   // true\nconsole.log(userMap.size);         // 3\n\n// Iterate\nfor (const [key, val] of userMap) {\n    console.log(`${key}: ${val}`);\n}\n\n// Object as dictionary\nconst config = { db: { host: 'localhost', port: 5432 }, debug: true };\nconsole.log(config.db.host);  // localhost\n\n// Word frequency counter\nfunction wordFreq(text) {\n    const freq = new Map();\n    for (const word of text.split(/\\s+/)) {\n        freq.set(word, (freq.get(word) || 0) + 1);\n    }\n    return freq;\n}\nconsole.log([...wordFreq('hello world hello js')]);", "Map and Object dictionary in JavaScript."),
            "java"    => ("import java.util.HashMap;\nimport java.util.Map;\n\npublic class HashMapDemo {\n    public static void main(String[] args) {\n        Map<String, Integer> scores = new HashMap<>();\n        scores.put(\"Alice\", 95);\n        scores.put(\"Bob\", 87);\n        scores.put(\"Carol\", 92);\n\n        // Get\n        System.out.println(\"Alice: \" + scores.get(\"Alice\"));\n\n        // Check\n        System.out.println(\"Has Bob? \" + scores.containsKey(\"Bob\"));\n\n        // getOrDefault\n        System.out.println(\"Dave: \" + scores.getOrDefault(\"Dave\", 0));\n\n        // Iterate\n        for (Map.Entry<String, Integer> entry : scores.entrySet()) {\n            System.out.println(entry.getKey() + \": \" + entry.getValue());\n        }\n\n        // Word frequency\n        String text = \"hello world hello java\";\n        Map<String, Integer> freq = new HashMap<>();\n        for (String w : text.split(\"\\\\s+\")) {\n            freq.merge(w, 1, Integer::sum);\n        }\n        System.out.println(freq);\n    }\n}", "Java HashMap — put, get, iterate, word counter."),
            _ => ("# Python dict — O(1) average get/set\nscores = {'Alice': 95, 'Bob': 87, 'Carol': 92}\n\n# Access\nprint(scores['Alice'])            # 95\nprint(scores.get('Dave', 0))      # 0 (default)\n\n# Add / update\nscores['Dave'] = 80\nscores['Bob'] += 5\n\n# Delete\ndel scores['Carol']\n\n# Iterate\nfor name, score in scores.items():\n    print(f'{name}: {score}')\n\n# Dict comprehension\nsquares = {x: x**2 for x in range(10)}\nprint(squares)  # {0: 0, 1: 1, 2: 4, ...}\n\n# Word frequency counter\nfrom collections import Counter\ntext = 'hello world hello python world'\nfreq = Counter(text.split())\nprint(freq)              # Counter({'hello': 2, 'world': 2, 'python': 1})\nprint(freq.most_common(2))  # [('hello', 2), ('world', 2)]", "Python dict — access, update, comprehension, Counter."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Tree / Binary Tree / BST ──────────────────────────────────────────────
    if q.contains("binary tree") || q.contains("binary search tree") || q.contains("bst") || q.contains("tree traversal") || q.contains("inorder") || q.contains("preorder") || q.contains("postorder") {
        let (code, note) = match lang {
            "rust"   => ("use std::boxed::Box;\n\n#[derive(Debug)]\nstruct TreeNode {\n    val: i32,\n    left: Option<Box<TreeNode>>,\n    right: Option<Box<TreeNode>>,\n}\n\nimpl TreeNode {\n    fn new(val: i32) -> Self {\n        TreeNode { val, left: None, right: None }\n    }\n\n    fn insert(&mut self, val: i32) {\n        if val < self.val {\n            match &mut self.left {\n                Some(node) => node.insert(val),\n                None => self.left = Some(Box::new(TreeNode::new(val))),\n            }\n        } else {\n            match &mut self.right {\n                Some(node) => node.insert(val),\n                None => self.right = Some(Box::new(TreeNode::new(val))),\n            }\n        }\n    }\n\n    fn inorder(&self) -> Vec<i32> {\n        let mut result = Vec::new();\n        if let Some(l) = &self.left { result.extend(l.inorder()); }\n        result.push(self.val);\n        if let Some(r) = &self.right { result.extend(r.inorder()); }\n        result\n    }\n\n    fn search(&self, val: i32) -> bool {\n        if val == self.val { true }\n        else if val < self.val { self.left.as_ref().map_or(false, |n| n.search(val)) }\n        else { self.right.as_ref().map_or(false, |n| n.search(val)) }\n    }\n}\n\nfn main() {\n    let mut root = TreeNode::new(50);\n    for v in [30, 70, 20, 40, 60, 80] { root.insert(v); }\n    println!(\"Inorder: {:?}\", root.inorder());  // [20, 30, 40, 50, 60, 70, 80]\n    println!(\"Found 40? {}\", root.search(40));   // true\n    println!(\"Found 45? {}\", root.search(45));   // false\n}", "Binary Search Tree in Rust — insert, search, inorder traversal."),
            _ => ("class TreeNode:\n    def __init__(self, val):\n        self.val = val\n        self.left = None\n        self.right = None\n\nclass BST:\n    \"\"\"Binary Search Tree — O(log n) avg insert/search.\"\"\"\n    def __init__(self):\n        self.root = None\n\n    def insert(self, val):\n        if not self.root:\n            self.root = TreeNode(val)\n        else:\n            self._insert(self.root, val)\n\n    def _insert(self, node, val):\n        if val < node.val:\n            if node.left: self._insert(node.left, val)\n            else: node.left = TreeNode(val)\n        else:\n            if node.right: self._insert(node.right, val)\n            else: node.right = TreeNode(val)\n\n    def search(self, val) -> bool:\n        return self._search(self.root, val)\n\n    def _search(self, node, val) -> bool:\n        if not node: return False\n        if val == node.val: return True\n        return self._search(node.left if val < node.val else node.right, val)\n\n    def inorder(self) -> list:\n        result = []\n        self._inorder(self.root, result)\n        return result\n\n    def _inorder(self, node, result):\n        if node:\n            self._inorder(node.left, result)\n            result.append(node.val)\n            self._inorder(node.right, result)\n\n# Usage\ntree = BST()\nfor v in [50, 30, 70, 20, 40, 60, 80]:\n    tree.insert(v)\nprint(tree.inorder())    # [20, 30, 40, 50, 60, 70, 80]\nprint(tree.search(40))   # True\nprint(tree.search(45))   # False", "Binary Search Tree in Python — insert, search, inorder."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Graph BFS / DFS ────────────────────────────────────────────────────────
    if q.contains("bfs") || q.contains("dfs") || q.contains("breadth first") || q.contains("depth first") || (q.contains("graph") && (q.contains("search") || q.contains("traversal"))) {
        let (code, note) = match lang {
            "rust" => ("use std::collections::{HashMap, HashSet, VecDeque};\n\ntype Graph = HashMap<&'static str, Vec<&'static str>>;\n\n/// BFS — O(V + E) time, O(V) space\nfn bfs(graph: &Graph, start: &str) -> Vec<String> {\n    let mut visited = HashSet::new();\n    let mut queue = VecDeque::new();\n    let mut order = Vec::new();\n    visited.insert(start.to_string());\n    queue.push_back(start.to_string());\n    while let Some(node) = queue.pop_front() {\n        order.push(node.clone());\n        if let Some(neighbors) = graph.get(node.as_str()) {\n            for &n in neighbors {\n                if visited.insert(n.to_string()) {\n                    queue.push_back(n.to_string());\n                }\n            }\n        }\n    }\n    order\n}\n\n/// DFS — O(V + E) time, O(V) space\nfn dfs(graph: &Graph, start: &str) -> Vec<String> {\n    let mut visited = HashSet::new();\n    let mut stack = vec![start.to_string()];\n    let mut order = Vec::new();\n    while let Some(node) = stack.pop() {\n        if visited.insert(node.clone()) {\n            order.push(node.clone());\n            if let Some(neighbors) = graph.get(node.as_str()) {\n                for &n in neighbors.iter().rev() {\n                    if !visited.contains(n) {\n                        stack.push(n.to_string());\n                    }\n                }\n            }\n        }\n    }\n    order\n}\n\nfn main() {\n    let mut g: Graph = HashMap::new();\n    g.insert(\"A\", vec![\"B\", \"C\"]);\n    g.insert(\"B\", vec![\"A\", \"D\", \"E\"]);\n    g.insert(\"C\", vec![\"A\", \"F\"]);\n    g.insert(\"D\", vec![\"B\"]);\n    g.insert(\"E\", vec![\"B\", \"F\"]);\n    g.insert(\"F\", vec![\"C\", \"E\"]);\n    println!(\"BFS: {:?}\", bfs(&g, \"A\"));\n    println!(\"DFS: {:?}\", dfs(&g, \"A\"));\n}", "Graph BFS & DFS in Rust — adjacency list."),
            _ => ("from collections import deque\n\ndef bfs(graph: dict, start: str) -> list:\n    \"\"\"Breadth-First Search — O(V + E).\"\"\"\n    visited = {start}\n    queue = deque([start])\n    order = []\n    while queue:\n        node = queue.popleft()\n        order.append(node)\n        for neighbor in graph.get(node, []):\n            if neighbor not in visited:\n                visited.add(neighbor)\n                queue.append(neighbor)\n    return order\n\ndef dfs(graph: dict, start: str) -> list:\n    \"\"\"Depth-First Search — O(V + E).\"\"\"\n    visited = set()\n    stack = [start]\n    order = []\n    while stack:\n        node = stack.pop()\n        if node not in visited:\n            visited.add(node)\n            order.append(node)\n            for neighbor in reversed(graph.get(node, [])):\n                if neighbor not in visited:\n                    stack.append(neighbor)\n    return order\n\ndef has_path(graph: dict, src: str, dst: str) -> bool:\n    \"\"\"Check if path exists between src and dst.\"\"\"\n    return dst in bfs(graph, src)\n\n# Usage\ngraph = {\n    'A': ['B', 'C'],\n    'B': ['A', 'D', 'E'],\n    'C': ['A', 'F'],\n    'D': ['B'],\n    'E': ['B', 'F'],\n    'F': ['C', 'E'],\n}\nprint('BFS:', bfs(graph, 'A'))  # ['A', 'B', 'C', 'D', 'E', 'F']\nprint('DFS:', dfs(graph, 'A'))  # ['A', 'B', 'D', 'E', 'F', 'C']\nprint('Path A→F:', has_path(graph, 'A', 'F'))  # True", "Graph BFS & DFS in Python — adjacency list, path check."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── REST API / Express / Flask / FastAPI ──────────────────────────────────
    if q.contains("rest api") || q.contains("api endpoint") || q.contains("flask") || q.contains("fastapi") || q.contains("express") || (q.contains("api") && (q.contains("crud") || q.contains("get") || q.contains("post"))) {
        let (code, note) = match lang {
            "javascript" | "typescript" => ("const express = require('express');\nconst app = express();\napp.use(express.json());\n\n// In-memory store\nlet items = [\n    { id: 1, name: 'Item 1', price: 9.99 },\n    { id: 2, name: 'Item 2', price: 19.99 },\n];\nlet nextId = 3;\n\n// GET all items\napp.get('/api/items', (req, res) => {\n    res.json(items);\n});\n\n// GET single item\napp.get('/api/items/:id', (req, res) => {\n    const item = items.find(i => i.id === parseInt(req.params.id));\n    if (!item) return res.status(404).json({ error: 'Not found' });\n    res.json(item);\n});\n\n// POST create item\napp.post('/api/items', (req, res) => {\n    const { name, price } = req.body;\n    if (!name) return res.status(400).json({ error: 'Name required' });\n    const item = { id: nextId++, name, price: price || 0 };\n    items.push(item);\n    res.status(201).json(item);\n});\n\n// PUT update item\napp.put('/api/items/:id', (req, res) => {\n    const item = items.find(i => i.id === parseInt(req.params.id));\n    if (!item) return res.status(404).json({ error: 'Not found' });\n    if (req.body.name) item.name = req.body.name;\n    if (req.body.price !== undefined) item.price = req.body.price;\n    res.json(item);\n});\n\n// DELETE item\napp.delete('/api/items/:id', (req, res) => {\n    const idx = items.findIndex(i => i.id === parseInt(req.params.id));\n    if (idx === -1) return res.status(404).json({ error: 'Not found' });\n    items.splice(idx, 1);\n    res.status(204).send();\n});\n\napp.listen(3000, () => console.log('API running on http://localhost:3000'));", "Express.js REST API — full CRUD with validation."),
            _ => ("from fastapi import FastAPI, HTTPException\nfrom pydantic import BaseModel\nfrom typing import Optional\n\napp = FastAPI(title='Items API')\n\nclass Item(BaseModel):\n    name: str\n    price: float = 0.0\n    description: Optional[str] = None\n\n# In-memory store\nitems_db: dict[int, dict] = {\n    1: {'id': 1, 'name': 'Item 1', 'price': 9.99},\n    2: {'id': 2, 'name': 'Item 2', 'price': 19.99},\n}\nnext_id = 3\n\n@app.get('/api/items')\ndef list_items():\n    return list(items_db.values())\n\n@app.get('/api/items/{item_id}')\ndef get_item(item_id: int):\n    if item_id not in items_db:\n        raise HTTPException(404, 'Item not found')\n    return items_db[item_id]\n\n@app.post('/api/items', status_code=201)\ndef create_item(item: Item):\n    global next_id\n    new = {'id': next_id, **item.dict()}\n    items_db[next_id] = new\n    next_id += 1\n    return new\n\n@app.put('/api/items/{item_id}')\ndef update_item(item_id: int, item: Item):\n    if item_id not in items_db:\n        raise HTTPException(404, 'Item not found')\n    items_db[item_id].update(item.dict(exclude_unset=True))\n    return items_db[item_id]\n\n@app.delete('/api/items/{item_id}', status_code=204)\ndef delete_item(item_id: int):\n    if item_id not in items_db:\n        raise HTTPException(404, 'Item not found')\n    del items_db[item_id]\n\n# Run: uvicorn main:app --reload\n# Docs: http://localhost:8000/docs", "FastAPI REST API — full CRUD with Pydantic validation and auto-docs."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Try-Catch / Error Handling ──────────────────────────────────────────────
    if q.contains("try catch") || q.contains("try-catch") || q.contains("error handling") || q.contains("exception") || q.contains("try except") {
        let (code, note) = match lang {
            "killer" => ("kfn safe_divide(a, b) {\n  if b == 0 {\n    print(K\"Error: division by zero\")\n    0\n  } else {\n    a / b\n  }\n}\n\nkfn safe_parse(s) {\n  result = to_number(s)\n  if result == nil {\n    print(K\"Error: '{s}' is not a number\")\n    0\n  } else {\n    result\n  }\n}\n\nprint(safe_divide(10, 3))    # 3.333...\nprint(safe_divide(10, 0))    # Error: division by zero\nprint(safe_parse(\"42\"))      # 42\nprint(safe_parse(\"abc\"))     # Error: 'abc' is not a number", "Error handling in Killer — guard-style."),
            "rust" => ("use std::num::ParseIntError;\nuse std::io;\n\n// Custom error type\n#[derive(Debug)]\nenum AppError {\n    Io(io::Error),\n    Parse(ParseIntError),\n    Custom(String),\n}\n\nimpl std::fmt::Display for AppError {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        match self {\n            AppError::Io(e) => write!(f, \"IO error: {}\", e),\n            AppError::Parse(e) => write!(f, \"Parse error: {}\", e),\n            AppError::Custom(s) => write!(f, \"{}\", s),\n        }\n    }\n}\n\nimpl From<io::Error> for AppError {\n    fn from(e: io::Error) -> Self { AppError::Io(e) }\n}\nimpl From<ParseIntError> for AppError {\n    fn from(e: ParseIntError) -> Self { AppError::Parse(e) }\n}\n\nfn divide(a: f64, b: f64) -> Result<f64, AppError> {\n    if b == 0.0 {\n        Err(AppError::Custom(\"Division by zero\".into()))\n    } else {\n        Ok(a / b)\n    }\n}\n\nfn main() {\n    // Result-based error handling\n    match divide(10.0, 3.0) {\n        Ok(val) => println!(\"10/3 = {:.4}\", val),\n        Err(e) => eprintln!(\"Error: {}\", e),\n    }\n\n    // ? operator\n    let result: Result<f64, AppError> = (|| {\n        let a = \"42\".parse::<f64>().map_err(|e| AppError::Parse(e.to_string().parse().unwrap()))?;\n        divide(a, 7.0)\n    })();\n    println!(\"{:?}\", result);\n}", "Error handling in Rust — Result, custom error, From, ? operator."),
            "javascript" | "typescript" => ("// Synchronous try-catch\ntry {\n    const data = JSON.parse('{\"name\": \"Kala\"}');\n    console.log(data.name);\n} catch (error) {\n    console.error('Parse failed:', error.message);\n} finally {\n    console.log('Cleanup done');\n}\n\n// Async try-catch\nasync function fetchData(url) {\n    try {\n        const res = await fetch(url);\n        if (!res.ok) throw new Error(`HTTP ${res.status}`);\n        return await res.json();\n    } catch (error) {\n        if (error instanceof TypeError) {\n            console.error('Network error:', error.message);\n        } else {\n            console.error('Fetch error:', error.message);\n        }\n        return null;\n    }\n}\n\n// Custom error class\nclass ValidationError extends Error {\n    constructor(field, message) {\n        super(message);\n        this.name = 'ValidationError';\n        this.field = field;\n    }\n}\n\nfunction validateEmail(email) {\n    if (!email.includes('@')) {\n        throw new ValidationError('email', 'Invalid email format');\n    }\n    return true;\n}\n\ntry {\n    validateEmail('bad-email');\n} catch (e) {\n    if (e instanceof ValidationError) {\n        console.error(`${e.field}: ${e.message}`);\n    }\n}", "Error handling in JavaScript — try/catch, async, custom errors."),
            "java" => ("import java.io.*;\n\n// Custom exception\nclass BusinessException extends Exception {\n    private final int code;\n    public BusinessException(int code, String msg) {\n        super(msg);\n        this.code = code;\n    }\n    public int getCode() { return code; }\n}\n\npublic class ErrorHandling {\n    public static double divide(double a, double b) throws BusinessException {\n        if (b == 0) throw new BusinessException(400, \"Division by zero\");\n        return a / b;\n    }\n\n    public static void main(String[] args) {\n        // Basic try-catch-finally\n        try {\n            double result = divide(10, 3);\n            System.out.println(\"10/3 = \" + result);\n            \n            divide(10, 0); // throws\n        } catch (BusinessException e) {\n            System.err.println(\"Error \" + e.getCode() + \": \" + e.getMessage());\n        } finally {\n            System.out.println(\"Cleanup done\");\n        }\n\n        // Try-with-resources\n        try (BufferedReader reader = new BufferedReader(new StringReader(\"Hello\"))) {\n            System.out.println(reader.readLine());\n        } catch (IOException e) {\n            e.printStackTrace();\n        }\n\n        // Multi-catch\n        try {\n            int num = Integer.parseInt(\"abc\");\n        } catch (NumberFormatException | ArithmeticException e) {\n            System.err.println(\"Caught: \" + e.getMessage());\n        }\n    }\n}", "Java error handling — custom exception, try-with-resources, multi-catch."),
            _ => ("# Python exception handling — comprehensive guide\n\n# Basic try-except-finally\ntry:\n    result = 10 / 0\nexcept ZeroDivisionError as e:\n    print(f'Error: {e}')         # Error: division by zero\nfinally:\n    print('Cleanup done')\n\n# Multiple exceptions\ntry:\n    value = int('abc')\nexcept (ValueError, TypeError) as e:\n    print(f'Conversion error: {e}')\n\n# Custom exception class\nclass ValidationError(Exception):\n    def __init__(self, field: str, message: str):\n        self.field = field\n        self.message = message\n        super().__init__(f'{field}: {message}')\n\ndef validate_age(age: int) -> int:\n    if age < 0:\n        raise ValidationError('age', 'Must be non-negative')\n    if age > 150:\n        raise ValidationError('age', 'Unrealistic age')\n    return age\n\ntry:\n    validate_age(-5)\nexcept ValidationError as e:\n    print(f'Validation failed — {e.field}: {e.message}')\n\n# Context manager for resource cleanup\nclass FileProcessor:\n    def __init__(self, path):\n        self.path = path\n    def __enter__(self):\n        self.file = open(self.path, 'r')\n        return self.file\n    def __exit__(self, exc_type, exc_val, exc_tb):\n        self.file.close()\n        if exc_type:\n            print(f'Error during processing: {exc_val}')\n        return False  # don't suppress exception\n\n# Retry decorator\nfrom functools import wraps\nimport time\n\ndef retry(max_attempts=3, delay=1.0):\n    def decorator(func):\n        @wraps(func)\n        def wrapper(*args, **kwargs):\n            for attempt in range(max_attempts):\n                try:\n                    return func(*args, **kwargs)\n                except Exception as e:\n                    if attempt == max_attempts - 1:\n                        raise\n                    print(f'Attempt {attempt+1} failed: {e}. Retrying...')\n                    time.sleep(delay)\n        return wrapper\n    return decorator\n\n@retry(max_attempts=3)\ndef flaky_operation():\n    import random\n    if random.random() < 0.7:\n        raise ConnectionError('Network timeout')\n    return 'Success!'\n\nprint(flaky_operation())", "Python error handling — exceptions, custom errors, retry decorator."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Class / OOP / Inheritance ──────────────────────────────────────────────
    if (q.contains("class") && !q.contains("calculator")) || q.contains("inheritance") || q.contains("oop") || q.contains("object oriented") || q.contains("polymorphism") {
        let (code, note) = match lang {
            "killer" => ("# Killer OOP using closures\nkfn Animal(name, sound) {\n  {\n    name: name,\n    sound: sound,\n    speak: kfn() { print(K\"{name} says {sound}!\") },\n    info: kfn() { K\"{name} (sound: {sound})\" }\n  }\n}\n\nkfn Dog(name) {\n  base = Animal(name, \"Woof\")\n  base[\"fetch\"] = kfn(item) { print(K\"{name} fetches the {item}!\") }\n  base\n}\n\nkfn Cat(name) {\n  base = Animal(name, \"Meow\")\n  base[\"purr\"] = kfn() { print(K\"{name} purrs softly...\") }\n  base\n}\n\ndog = Dog(\"Rex\")\ndog.speak()           # Rex says Woof!\ndog.fetch(\"ball\")     # Rex fetches the ball!\n\ncat = Cat(\"Whiskers\")\ncat.speak()           # Whiskers says Meow!\ncat.purr()            # Whiskers purrs softly...", "OOP in Killer using closures and dict composition."),
            "typescript" => ("// TypeScript class with interfaces\ninterface Speakable {\n    speak(): string;\n}\n\nabstract class Animal implements Speakable {\n    constructor(\n        protected name: string,\n        protected age: number\n    ) {}\n\n    abstract speak(): string;\n\n    info(): string {\n        return `${this.name} (age: ${this.age})`;\n    }\n}\n\nclass Dog extends Animal {\n    constructor(name: string, age: number, private breed: string) {\n        super(name, age);\n    }\n\n    speak(): string {\n        return `${this.name} says Woof!`;\n    }\n\n    fetch(item: string): string {\n        return `${this.name} fetches the ${item}!`;\n    }\n}\n\nclass Cat extends Animal {\n    speak(): string {\n        return `${this.name} says Meow!`;\n    }\n\n    purr(): string {\n        return `${this.name} purrs softly...`;\n    }\n}\n\nconst dog = new Dog('Rex', 3, 'Labrador');\nconst cat = new Cat('Whiskers', 5);\n\n// Polymorphism\nconst animals: Animal[] = [dog, cat];\nfor (const a of animals) {\n    console.log(a.speak());\n}", "TypeScript OOP — abstract class, interface, polymorphism."),
            "java" => ("// Abstract class + interface\ninterface Soundable {\n    String makeSound();\n}\n\nabstract class Animal implements Soundable {\n    protected String name;\n    protected int age;\n\n    public Animal(String name, int age) {\n        this.name = name;\n        this.age = age;\n    }\n\n    public String info() {\n        return name + \" (age: \" + age + \")\";\n    }\n\n    @Override\n    public String toString() { return info(); }\n}\n\nclass Dog extends Animal {\n    private String breed;\n\n    public Dog(String name, int age, String breed) {\n        super(name, age);\n        this.breed = breed;\n    }\n\n    @Override\n    public String makeSound() { return name + \" says Woof!\"; }\n\n    public String fetch(String item) {\n        return name + \" fetches the \" + item + \"!\";\n    }\n}\n\nclass Cat extends Animal {\n    public Cat(String name, int age) { super(name, age); }\n\n    @Override\n    public String makeSound() { return name + \" says Meow!\"; }\n}\n\npublic class OOPDemo {\n    public static void main(String[] args) {\n        Animal[] animals = { new Dog(\"Rex\", 3, \"Lab\"), new Cat(\"Whiskers\", 5) };\n        for (Animal a : animals) {\n            System.out.println(a.makeSound());\n        }\n    }\n}", "Java OOP — abstract class, interface, polymorphism."),
            _ => ("from abc import ABC, abstractmethod\n\n# Abstract base class\nclass Animal(ABC):\n    def __init__(self, name: str, age: int):\n        self.name = name\n        self.age = age\n\n    @abstractmethod\n    def speak(self) -> str:\n        pass\n\n    def info(self) -> str:\n        return f'{self.name} (age: {self.age})'\n\n    def __repr__(self) -> str:\n        return f'{type(self).__name__}({self.name!r}, {self.age})'\n\n# Inheritance\nclass Dog(Animal):\n    def __init__(self, name: str, age: int, breed: str):\n        super().__init__(name, age)\n        self.breed = breed\n\n    def speak(self) -> str:\n        return f'{self.name} says Woof!'\n\n    def fetch(self, item: str) -> str:\n        return f'{self.name} fetches the {item}!'\n\nclass Cat(Animal):\n    def speak(self) -> str:\n        return f'{self.name} says Meow!'\n\n    def purr(self) -> str:\n        return f'{self.name} purrs softly...'\n\n# Usage — Polymorphism\ndog = Dog('Rex', 3, 'Labrador')\ncat = Cat('Whiskers', 5)\n\nanimals: list[Animal] = [dog, cat]\nfor a in animals:\n    print(a.speak())   # Each calls its own version\n    print(a.info())\n\nprint(dog.fetch('ball'))  # Dog-specific method\nprint(cat.purr())         # Cat-specific method\nprint(isinstance(dog, Animal))  # True", "Python OOP — ABC, inheritance, polymorphism, __repr__."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Decorator / Design Pattern ──────────────────────────────────────────────
    if q.contains("decorator") || q.contains("singleton") || q.contains("observer") || q.contains("design pattern") || q.contains("factory pattern") {
        if q.contains("singleton") {
            let (code, note) = match lang {
                "rust" => ("use std::sync::OnceLock;\n\nstatic INSTANCE: OnceLock<Config> = OnceLock::new();\n\n#[derive(Debug)]\nstruct Config {\n    db_url: String,\n    max_connections: u32,\n}\n\nimpl Config {\n    fn get() -> &'static Config {\n        INSTANCE.get_or_init(|| Config {\n            db_url: \"postgres://localhost/mydb\".into(),\n            max_connections: 10,\n        })\n    }\n}\n\nfn main() {\n    let c1 = Config::get();\n    let c2 = Config::get();\n    println!(\"{:?}\", c1);\n    println!(\"Same instance: {}\", std::ptr::eq(c1, c2));  // true\n}", "Singleton in Rust — OnceLock (thread-safe)."),
                _ => ("import threading\n\nclass Singleton:\n    \"\"\"Thread-safe singleton — double-checked locking.\"\"\"\n    _instance = None\n    _lock = threading.Lock()\n\n    def __new__(cls, *args, **kwargs):\n        if cls._instance is None:\n            with cls._lock:\n                if cls._instance is None:\n                    cls._instance = super().__new__(cls)\n        return cls._instance\n\n    def __init__(self):\n        if not hasattr(self, '_initialized'):\n            self._initialized = True\n            self.config = {'db': 'localhost', 'port': 5432}\n\n# Usage\na = Singleton()\nb = Singleton()\nprint(a is b)        # True — same instance\na.config['port'] = 3306\nprint(b.config)      # {'db': 'localhost', 'port': 3306}", "Thread-safe Singleton in Python."),
            };
            return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
        }
        if q.contains("observer") {
            let code = "from abc import ABC, abstractmethod\nfrom typing import Any\n\nclass Observer(ABC):\n    @abstractmethod\n    def update(self, event: str, data: Any) -> None:\n        pass\n\nclass EventEmitter:\n    \"\"\"Observable — publish/subscribe pattern.\"\"\"\n    def __init__(self):\n        self._listeners: dict[str, list[Observer]] = {}\n\n    def on(self, event: str, observer: Observer):\n        self._listeners.setdefault(event, []).append(observer)\n\n    def off(self, event: str, observer: Observer):\n        if event in self._listeners:\n            self._listeners[event].remove(observer)\n\n    def emit(self, event: str, data: Any = None):\n        for observer in self._listeners.get(event, []):\n            observer.update(event, data)\n\n# Concrete observers\nclass Logger(Observer):\n    def update(self, event: str, data):\n        print(f'[LOG] {event}: {data}')\n\nclass EmailNotifier(Observer):\n    def update(self, event: str, data):\n        print(f'[EMAIL] Sending notification about {event}')\n\n# Usage\nemitter = EventEmitter()\nlogger = Logger()\nemail = EmailNotifier()\n\nemitter.on('user_created', logger)\nemitter.on('user_created', email)\nemitter.on('error', logger)\n\nemitter.emit('user_created', {'name': 'Arun', 'email': 'arun@killer.dev'})\nemitter.emit('error', 'Database connection failed')";
            return format!("```python\n{}\n```\n\n*Observer pattern in Python — EventEmitter with typed observers.*", code);
        }
        // Default: decorator pattern
        let code = "from functools import wraps\nimport time\n\n# Timer decorator\ndef timer(func):\n    @wraps(func)\n    def wrapper(*args, **kwargs):\n        start = time.perf_counter()\n        result = func(*args, **kwargs)\n        elapsed = time.perf_counter() - start\n        print(f'{func.__name__} took {elapsed:.4f}s')\n        return result\n    return wrapper\n\n# Retry decorator with parameters\ndef retry(max_attempts=3, exceptions=(Exception,)):\n    def decorator(func):\n        @wraps(func)\n        def wrapper(*args, **kwargs):\n            for attempt in range(max_attempts):\n                try:\n                    return func(*args, **kwargs)\n                except exceptions as e:\n                    if attempt == max_attempts - 1:\n                        raise\n                    print(f'Retry {attempt + 1}/{max_attempts}: {e}')\n        return wrapper\n    return decorator\n\n# Cache decorator (simple memoization)\ndef memoize(func):\n    cache = {}\n    @wraps(func)\n    def wrapper(*args):\n        if args not in cache:\n            cache[args] = func(*args)\n        return cache[args]\n    return wrapper\n\n# Auth decorator\ndef require_auth(role='user'):\n    def decorator(func):\n        @wraps(func)\n        def wrapper(user, *args, **kwargs):\n            if not user.get('authenticated'):\n                raise PermissionError('Not authenticated')\n            if user.get('role') != role and role != 'user':\n                raise PermissionError(f'Requires {role} role')\n            return func(user, *args, **kwargs)\n        return wrapper\n    return decorator\n\n# Usage\n@timer\n@memoize\ndef fibonacci(n: int) -> int:\n    if n <= 1: return n\n    return fibonacci(n - 1) + fibonacci(n - 2)\n\n@retry(max_attempts=3)\ndef fetch_data():\n    import random\n    if random.random() < 0.5:\n        raise ConnectionError('Timeout')\n    return {'data': [1, 2, 3]}\n\n@require_auth(role='admin')\ndef delete_user(user, user_id):\n    return f'Deleted user {user_id}'\n\nprint(fibonacci(30))  # 832040\ntry:\n    print(delete_user({'authenticated': True, 'role': 'admin'}, 42))\nexcept PermissionError as e:\n    print(e)";
        return format!("```python\n{}\n```\n\n*Python decorators — timer, retry, memoize, auth — production-ready patterns.*", code);
    }

    // ── List comprehension / Map / Filter / Reduce ──────────────────────────────
    if q.contains("comprehension") || q.contains("map filter") || q.contains("map reduce") || q.contains("lambda") || q.contains("functional") {
        let (code, note) = match lang {
            "javascript" | "typescript" => ("// Array methods — map, filter, reduce\nconst nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];\n\n// Map: transform each element\nconst doubled = nums.map(n => n * 2);\nconsole.log(doubled);  // [2, 4, 6, 8, 10, 12, 14, 16, 18, 20]\n\n// Filter: keep elements matching condition\nconst evens = nums.filter(n => n % 2 === 0);\nconsole.log(evens);  // [2, 4, 6, 8, 10]\n\n// Reduce: accumulate to single value\nconst sum = nums.reduce((acc, n) => acc + n, 0);\nconsole.log(sum);  // 55\n\n// Chaining\nconst result = nums\n    .filter(n => n % 2 === 0)\n    .map(n => n * n)\n    .reduce((acc, n) => acc + n, 0);\nconsole.log(result);  // 220 (4+16+36+64+100)\n\n// Find / Some / Every\nconsole.log(nums.find(n => n > 5));    // 6\nconsole.log(nums.some(n => n > 9));    // true\nconsole.log(nums.every(n => n > 0));   // true\n\n// FlatMap\nconst nested = [[1, 2], [3, 4], [5]];\nconsole.log(nested.flatMap(x => x));  // [1, 2, 3, 4, 5]", "JavaScript functional — map, filter, reduce, chaining."),
            "rust" => ("fn main() {\n    let nums: Vec<i32> = (1..=10).collect();\n\n    // Map\n    let doubled: Vec<i32> = nums.iter().map(|&n| n * 2).collect();\n    println!(\"{:?}\", doubled);\n\n    // Filter\n    let evens: Vec<i32> = nums.iter().filter(|&&n| n % 2 == 0).copied().collect();\n    println!(\"{:?}\", evens);\n\n    // Fold (reduce)\n    let sum: i32 = nums.iter().sum();\n    println!(\"Sum: {}\", sum);\n\n    // Chaining\n    let result: i32 = nums.iter()\n        .filter(|&&n| n % 2 == 0)\n        .map(|&n| n * n)\n        .sum();\n    println!(\"Sum of even squares: {}\", result);\n\n    // Find / Any / All\n    let first_gt5 = nums.iter().find(|&&n| n > 5);\n    println!(\"First > 5: {:?}\", first_gt5);\n    println!(\"Any > 9: {}\", nums.iter().any(|&n| n > 9));\n    println!(\"All > 0: {}\", nums.iter().all(|&n| n > 0));\n\n    // Enumerate + zip\n    for (i, &v) in nums.iter().enumerate().take(3) {\n        println!(\"[{}] = {}\", i, v);\n    }\n}", "Rust iterators — map, filter, fold, chaining, find."),
            _ => ("# Python list comprehensions and functional tools\nnums = list(range(1, 11))  # [1, 2, 3, ..., 10]\n\n# List comprehension\nsquares = [x**2 for x in nums]\nprint(squares)  # [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]\n\n# With filter condition\nevens = [x for x in nums if x % 2 == 0]\nprint(evens)  # [2, 4, 6, 8, 10]\n\n# Nested comprehension (matrix flattening)\nmatrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]\nflat = [x for row in matrix for x in row]\nprint(flat)  # [1, 2, 3, 4, 5, 6, 7, 8, 9]\n\n# Dict comprehension\nword_lengths = {w: len(w) for w in ['hello', 'world', 'python']}\nprint(word_lengths)  # {'hello': 5, 'world': 5, 'python': 6}\n\n# Set comprehension\nunique_lengths = {len(w) for w in ['hi', 'bye', 'hey', 'ok']}\nprint(unique_lengths)  # {2, 3}\n\n# map/filter/reduce\nfrom functools import reduce\n\ndoubled = list(map(lambda x: x * 2, nums))\nprint(doubled)  # [2, 4, 6, ...]\n\nfiltered = list(filter(lambda x: x > 5, nums))\nprint(filtered)  # [6, 7, 8, 9, 10]\n\ntotal = reduce(lambda acc, x: acc + x, nums, 0)\nprint(total)  # 55\n\n# Generator expression (lazy — memory efficient)\nsum_of_squares = sum(x**2 for x in range(1_000_000))\nprint(sum_of_squares)", "Python comprehensions + map/filter/reduce — complete guide."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── CLI tool / argparse / command line ──────────────────────────────────────
    if q.contains("cli") || q.contains("command line") || q.contains("argparse") || q.contains("argument pars") {
        let (code, note) = match lang {
            "rust" => ("use std::env;\n\nfn main() {\n    let args: Vec<String> = env::args().collect();\n\n    if args.len() < 2 {\n        eprintln!(\"Usage: {} <command> [options]\", args[0]);\n        eprintln!(\"Commands: greet, count, help\");\n        std::process::exit(1);\n    }\n\n    match args[1].as_str() {\n        \"greet\" => {\n            let name = args.get(2).map(|s| s.as_str()).unwrap_or(\"World\");\n            println!(\"Hello, {}!\", name);\n        }\n        \"count\" => {\n            let n: usize = args.get(2)\n                .and_then(|s| s.parse().ok())\n                .unwrap_or(10);\n            for i in 1..=n {\n                println!(\"{}\", i);\n            }\n        }\n        \"help\" | \"--help\" | \"-h\" => {\n            println!(\"MyCLI v1.0\");\n            println!(\"Usage: mycli <command>\");\n            println!(\"  greet [name]   Greet someone\");\n            println!(\"  count [n]      Count to n\");\n            println!(\"  help           Show this help\");\n        }\n        cmd => eprintln!(\"Unknown command: {}\", cmd),\n    }\n}", "CLI tool in Rust — subcommands with args."),
            _ => ("import argparse\nimport sys\n\ndef main():\n    parser = argparse.ArgumentParser(\n        prog='mytool',\n        description='A production CLI tool'\n    )\n    parser.add_argument('--version', action='version', version='1.0.0')\n\n    subparsers = parser.add_subparsers(dest='command', help='Available commands')\n\n    # greet command\n    greet_parser = subparsers.add_parser('greet', help='Greet someone')\n    greet_parser.add_argument('name', nargs='?', default='World')\n    greet_parser.add_argument('-u', '--uppercase', action='store_true')\n\n    # count command\n    count_parser = subparsers.add_parser('count', help='Count to N')\n    count_parser.add_argument('n', type=int, default=10, nargs='?')\n    count_parser.add_argument('-r', '--reverse', action='store_true')\n\n    # search command\n    search_parser = subparsers.add_parser('search', help='Search for text')\n    search_parser.add_argument('pattern')\n    search_parser.add_argument('-f', '--file', required=True)\n    search_parser.add_argument('-i', '--ignore-case', action='store_true')\n\n    args = parser.parse_args()\n\n    if args.command == 'greet':\n        msg = f'Hello, {args.name}!'\n        print(msg.upper() if args.uppercase else msg)\n\n    elif args.command == 'count':\n        r = range(args.n, 0, -1) if args.reverse else range(1, args.n + 1)\n        for i in r:\n            print(i)\n\n    elif args.command == 'search':\n        import re\n        flags = re.IGNORECASE if args.ignore_case else 0\n        with open(args.file) as f:\n            for i, line in enumerate(f, 1):\n                if re.search(args.pattern, line, flags):\n                    print(f'{i}: {line.rstrip()}')\n    else:\n        parser.print_help()\n        sys.exit(1)\n\nif __name__ == '__main__':\n    main()\n\n# Usage:\n#   python mytool.py greet Arun --uppercase\n#   python mytool.py count 5 --reverse\n#   python mytool.py search 'TODO' -f main.py -i", "Python CLI with argparse — subcommands, flags, options."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Database / CRUD / SQLite / ORM ────────────────────────────────────────
    if (q.contains("database") || q.contains("sqlite") || q.contains("crud")) && !q.contains("create table") {
        let code = "import sqlite3\nfrom contextlib import contextmanager\nfrom dataclasses import dataclass\nfrom typing import Optional\n\n@dataclass\nclass User:\n    id: Optional[int] = None\n    name: str = ''\n    email: str = ''\n    age: int = 0\n\n@contextmanager\ndef get_db(db_path: str = ':memory:'):\n    conn = sqlite3.connect(db_path)\n    conn.row_factory = sqlite3.Row\n    try:\n        yield conn\n        conn.commit()\n    except Exception:\n        conn.rollback()\n        raise\n    finally:\n        conn.close()\n\ndef init_db(conn):\n    conn.execute('''\n        CREATE TABLE IF NOT EXISTS users (\n            id    INTEGER PRIMARY KEY AUTOINCREMENT,\n            name  TEXT NOT NULL,\n            email TEXT UNIQUE NOT NULL,\n            age   INTEGER DEFAULT 0\n        )\n    ''')\n\n# CRUD operations\ndef create_user(conn, user: User) -> int:\n    cur = conn.execute(\n        'INSERT INTO users (name, email, age) VALUES (?, ?, ?)',\n        (user.name, user.email, user.age)\n    )\n    return cur.lastrowid\n\ndef get_user(conn, user_id: int) -> Optional[User]:\n    row = conn.execute('SELECT * FROM users WHERE id = ?', (user_id,)).fetchone()\n    return User(**dict(row)) if row else None\n\ndef list_users(conn) -> list[User]:\n    rows = conn.execute('SELECT * FROM users ORDER BY name').fetchall()\n    return [User(**dict(r)) for r in rows]\n\ndef update_user(conn, user_id: int, **fields):\n    sets = ', '.join(f'{k} = ?' for k in fields)\n    conn.execute(f'UPDATE users SET {sets} WHERE id = ?', (*fields.values(), user_id))\n\ndef delete_user(conn, user_id: int):\n    conn.execute('DELETE FROM users WHERE id = ?', (user_id,))\n\n# Usage\nwith get_db() as db:\n    init_db(db)\n    uid = create_user(db, User(name='Arun', email='arun@killer.dev', age=28))\n    create_user(db, User(name='Kala', email='kala@ai.dev', age=1))\n    print(f'Created user id={uid}')\n    print(f'Get: {get_user(db, uid)}')\n    update_user(db, uid, age=29)\n    print(f'All: {list_users(db)}')\n    delete_user(db, uid)\n    print(f'After delete: {list_users(db)}')";
        return format!("```python\n{}\n```\n\n*SQLite CRUD in Python — dataclass, context manager, parameterized queries.*", code);
    }

    // ── Dijkstra / Shortest Path ──────────────────────────────────────────────
    if q.contains("dijkstra") || q.contains("shortest path") {
        let code = "import heapq\nfrom collections import defaultdict\n\ndef dijkstra(graph: dict, start: str) -> tuple[dict, dict]:\n    \"\"\"Dijkstra's shortest path — O((V + E) log V).\n    Returns (distances, predecessors) dicts.\"\"\"\n    dist = {start: 0}\n    prev = {start: None}\n    pq = [(0, start)]  # (distance, node)\n\n    while pq:\n        d, u = heapq.heappop(pq)\n        if d > dist.get(u, float('inf')):\n            continue\n        for v, weight in graph.get(u, []):\n            new_dist = d + weight\n            if new_dist < dist.get(v, float('inf')):\n                dist[v] = new_dist\n                prev[v] = u\n                heapq.heappush(pq, (new_dist, v))\n    return dist, prev\n\ndef shortest_path(prev: dict, end: str) -> list:\n    path = []\n    node = end\n    while node is not None:\n        path.append(node)\n        node = prev.get(node)\n    return path[::-1]\n\n# Usage — weighted adjacency list (node -> [(neighbor, weight)])\ngraph = {\n    'A': [('B', 4), ('C', 2)],\n    'B': [('D', 3), ('C', 1)],\n    'C': [('B', 1), ('D', 5)],\n    'D': [('E', 2)],\n    'E': [],\n}\n\ndist, prev = dijkstra(graph, 'A')\nprint('Distances from A:', dist)\nprint('Path A→E:', shortest_path(prev, 'E'))  # ['A', 'C', 'B', 'D', 'E']\nprint('Cost A→E:', dist['E'])                  # 8";
        return format!("```python\n{}\n```\n\n*Dijkstra's shortest path — O((V+E) log V), priority queue, path reconstruction.*", code);
    }

    // ── JSON parsing / handling ────────────────────────────────────────────────
    if q.contains("json") && (q.contains("parse") || q.contains("read") || q.contains("write") || q.contains("handle") || q.contains("work with")) {
        let (code, note) = match lang {
            "javascript" | "typescript" => ("// Parse JSON string\nconst jsonStr = '{\"name\": \"Kala\", \"version\": 2, \"features\": [\"AI\", \"code gen\"]}';\nconst data = JSON.parse(jsonStr);\nconsole.log(data.name);        // Kala\nconsole.log(data.features[0]); // AI\n\n// Stringify with formatting\nconst obj = { user: 'Arun', scores: [95, 87, 92], active: true };\nconsole.log(JSON.stringify(obj, null, 2));\n\n// Safe parse\nfunction safeParse(str) {\n    try {\n        return { ok: true, data: JSON.parse(str) };\n    } catch (e) {\n        return { ok: false, error: e.message };\n    }\n}\nconsole.log(safeParse('{invalid}'));  // { ok: false, error: ... }\n\n// Deep clone\nconst clone = JSON.parse(JSON.stringify(obj));\n\n// Fetch JSON from API\nasync function getJson(url) {\n    const res = await fetch(url);\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json();\n}", "JavaScript JSON — parse, stringify, safe parse, fetch."),
            "rust" => ("// Rust JSON with serde (Cargo.toml: serde = { version = \"1\", features = [\"derive\"] }, serde_json = \"1\")\nuse serde::{Deserialize, Serialize};\nuse serde_json;\n\n#[derive(Debug, Serialize, Deserialize)]\nstruct User {\n    name: String,\n    age: u32,\n    #[serde(default)]\n    active: bool,\n}\n\nfn main() {\n    // Parse JSON string → struct\n    let json_str = r#\"{\"name\": \"Kala\", \"age\": 2, \"active\": true}\"#;\n    let user: User = serde_json::from_str(json_str).unwrap();\n    println!(\"{:?}\", user);\n\n    // Struct → JSON string\n    let json_out = serde_json::to_string_pretty(&user).unwrap();\n    println!(\"{}\", json_out);\n\n    // Dynamic JSON (serde_json::Value)\n    let v: serde_json::Value = serde_json::from_str(json_str).unwrap();\n    println!(\"Name: {}\", v[\"name\"]);\n}", "Rust JSON with serde — typed deserialization and dynamic Value."),
            _ => ("import json\nfrom pathlib import Path\nfrom typing import Optional, Any, Dict\n\n# Parse JSON string\ndata = json.loads('{\"name\": \"Kala\", \"version\": 2, \"features\": [\"AI\", \"code gen\"]}')\nprint(data['name'])        # Kala\nprint(data['features'][0]) # AI\n\n# Python dict → JSON string\nobj = {'user': 'Arun', 'scores': [95, 87, 92], 'active': True}\njson_str = json.dumps(obj, indent=2)\nprint(json_str)\n\n# Read JSON file\ndef read_json(path: str) -> dict:\n    return json.loads(Path(path).read_text(encoding='utf-8'))\n\n# Write JSON file\ndef write_json(path: str, data, indent: int = 2):\n    Path(path).write_text(\n        json.dumps(data, indent=indent, ensure_ascii=False),\n        encoding='utf-8'\n    )\n\n# Safe parse\ndef safe_parse(s: str) -> Optional[Dict[str, Any]]:\n    try:\n        return json.loads(s)\n    except json.JSONDecodeError as e:\n        print(f'Invalid JSON: {e}')\n        return None\n\nprint(safe_parse('{invalid}'))  # Invalid JSON: ...\n\n# Nested access with default\ndef get_nested(data: dict, *keys, default=None):\n    for key in keys:\n        if isinstance(data, dict):\n            data = data.get(key, default)\n        else:\n            return default\n    return data\n\nconfig = {'db': {'host': 'localhost', 'port': 5432}}\nprint(get_nested(config, 'db', 'host'))     # localhost\nprint(get_nested(config, 'db', 'timeout', default=30))  # 30", "Python JSON — parse, write, safe parse, nested access."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Websocket ──────────────────────────────────────────────────────────────
    if q.contains("websocket") || q.contains("web socket") || q.contains("real time") || q.contains("realtime") && q.contains("chat") {
        let (code, note) = match lang {
            "javascript" | "typescript" => ("// WebSocket server (Node.js + ws library)\n// npm install ws\nconst { WebSocketServer } = require('ws');\n\nconst wss = new WebSocketServer({ port: 8080 });\nconst clients = new Set();\n\nwss.on('connection', (ws) => {\n    clients.add(ws);\n    console.log(`Client connected (${clients.size} total)`);\n\n    ws.on('message', (data) => {\n        const msg = data.toString();\n        console.log('Received:', msg);\n        // Broadcast to all other clients\n        for (const client of clients) {\n            if (client !== ws && client.readyState === 1) {\n                client.send(msg);\n            }\n        }\n    });\n\n    ws.on('close', () => {\n        clients.delete(ws);\n        console.log(`Client disconnected (${clients.size} remaining)`);\n    });\n\n    ws.send('Welcome to the chat!');\n});\n\nconsole.log('WebSocket server on ws://localhost:8080');\n\n// --- Client (browser) ---\n// const ws = new WebSocket('ws://localhost:8080');\n// ws.onmessage = (e) => console.log(e.data);\n// ws.send('Hello!');", "WebSocket chat server in Node.js — broadcast to all clients."),
            _ => ("import asyncio\nimport websockets\nimport json\n\nCLIENTS = set()\n\nasync def handler(websocket):\n    CLIENTS.add(websocket)\n    try:\n        async for message in websocket:\n            data = json.loads(message)\n            print(f\"Received: {data}\")\n            # Broadcast to all other clients\n            for client in CLIENTS:\n                if client != websocket:\n                    await client.send(json.dumps(data))\n    finally:\n        CLIENTS.discard(websocket)\n\nasync def main():\n    async with websockets.serve(handler, 'localhost', 8080):\n        print('WebSocket server on ws://localhost:8080')\n        await asyncio.Future()  # run forever\n\nif __name__ == '__main__':\n    asyncio.run(main())\n\n# pip install websockets\n# Client: python -c \"\n# import asyncio, websockets\n# async def chat():\n#     async with websockets.connect('ws://localhost:8080') as ws:\n#         await ws.send('{\\\"msg\\\": \\\"Hello!\\\"}');\n#         print(await ws.recv())\n# asyncio.run(chat())\"", "WebSocket server in Python — broadcast chat with asyncio."),
        };
        return format!("```{}\n{}\n```\n\n*{}*", lang, code, note);
    }

    // ── Login / Auth / JWT ──────────────────────────────────────────────────────
    if q.contains("login") || q.contains("authentication") || q.contains("jwt") || q.contains("auth system") || q.contains("sign in") || q.contains("signup") {
        let code = "import hashlib\nimport hmac\nimport json\nimport base64\nimport time\nimport secrets\nfrom typing import Optional, Tuple, Dict, Any\n\n# --- Password Hashing (never store plaintext!) ---\ndef hash_password(password: str, salt: Optional[str] = None) -> Tuple[str, str]:\n    salt = salt or secrets.token_hex(16)\n    hashed = hashlib.pbkdf2_hmac('sha256', password.encode(), salt.encode(), 100_000)\n    return hashed.hex(), salt\n\ndef verify_password(password: str, stored_hash: str, salt: str) -> bool:\n    computed, _ = hash_password(password, salt)\n    return hmac.compare_digest(computed, stored_hash)\n\n# --- JWT (simplified, production: use PyJWT) ---\ndef base64url_encode(data: bytes) -> str:\n    return base64.urlsafe_b64encode(data).rstrip(b'=').decode()\n\ndef create_jwt(payload: dict, secret: str, exp_hours: int = 24) -> str:\n    header = {'alg': 'HS256', 'typ': 'JWT'}\n    payload['exp'] = int(time.time()) + exp_hours * 3600\n    payload['iat'] = int(time.time())\n    h = base64url_encode(json.dumps(header).encode())\n    p = base64url_encode(json.dumps(payload).encode())\n    sig = hmac.new(secret.encode(), f'{h}.{p}'.encode(), hashlib.sha256).digest()\n    return f'{h}.{p}.{base64url_encode(sig)}'\n\ndef verify_jwt(token: str, secret: str) -> Optional[Dict[str, Any]]:\n    try:\n        h, p, s = token.split('.')\n        expected_sig = hmac.new(secret.encode(), f'{h}.{p}'.encode(), hashlib.sha256).digest()\n        if not hmac.compare_digest(base64url_encode(expected_sig), s):\n            return None\n        padding = 4 - len(p) % 4\n        payload = json.loads(base64.urlsafe_b64decode(p + '=' * padding))\n        if payload.get('exp', 0) < time.time():\n            return None\n        return payload\n    except Exception:\n        return None\n\n# --- User Store ---\nusers_db = {}  # email -> {hash, salt, name}\n\ndef signup(email: str, password: str, name: str) -> str:\n    if email in users_db:\n        return 'Email already registered'\n    pw_hash, salt = hash_password(password)\n    users_db[email] = {'hash': pw_hash, 'salt': salt, 'name': name}\n    return f'User {name} registered successfully'\n\ndef login(email: str, password: str) -> Optional[str]:\n    user = users_db.get(email)\n    if not user or not verify_password(password, user['hash'], user['salt']):\n        return None\n    return create_jwt({'sub': email, 'name': user['name']}, SECRET)\n\n# Usage\nSECRET = secrets.token_hex(32)\nprint(signup('arun@killer.dev', 'MyP@ss123', 'Arun'))\ntoken = login('arun@killer.dev', 'MyP@ss123')\nprint(f'JWT: {token[:50]}...')\npayload = verify_jwt(token, SECRET)\nprint(f'Verified: {payload}')";
        return format!("```python\n{}\n```\n\n*Auth system — password hashing (PBKDF2), JWT creation/verification, signup/login.*", code);
    }

    // ── Web scraping / BeautifulSoup ──────────────────────────────────────────
    if q.contains("scraping") || q.contains("scrape") || q.contains("beautifulsoup") || q.contains("crawl") {
        let code = "import urllib.request\nimport re\nfrom html.parser import HTMLParser\n\nclass SimpleParser(HTMLParser):\n    \"\"\"Lightweight HTML parser — no external dependencies.\"\"\"\n    def __init__(self):\n        super().__init__()\n        self.links = []\n        self.texts = []\n        self.in_body = False\n        self._tag_stack = []\n        self._skip_tags = {'script', 'style', 'noscript'}\n\n    def handle_starttag(self, tag, attrs):\n        self._tag_stack.append(tag)\n        if tag == 'body':\n            self.in_body = True\n        if tag == 'a':\n            for name, val in attrs:\n                if name == 'href' and val:\n                    self.links.append(val)\n\n    def handle_endtag(self, tag):\n        if self._tag_stack and self._tag_stack[-1] == tag:\n            self._tag_stack.pop()\n\n    def handle_data(self, data):\n        if self.in_body and not any(t in self._skip_tags for t in self._tag_stack):\n            text = data.strip()\n            if text:\n                self.texts.append(text)\n\ndef fetch_page(url: str) -> str:\n    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})\n    with urllib.request.urlopen(req, timeout=10) as resp:\n        return resp.read().decode('utf-8', errors='replace')\n\ndef scrape(url: str) -> dict:\n    html = fetch_page(url)\n    parser = SimpleParser()\n    parser.feed(html)\n    # Extract title\n    title_match = re.search(r'<title>(.*?)</title>', html, re.IGNORECASE | re.DOTALL)\n    title = title_match.group(1).strip() if title_match else ''\n    return {\n        'title': title,\n        'links': parser.links[:20],\n        'text': ' '.join(parser.texts[:50])[:500],\n    }\n\n# Usage\nresult = scrape('https://example.com')\nprint(f\"Title: {result['title']}\")\nprint(f\"Links: {len(result['links'])}\")\nprint(f\"Text preview: {result['text'][:200]}...\")";
        return format!("```python\n{}\n```\n\n*Web scraper in Python — zero external deps, HTML parser, link/text extraction.*", code);
    }
    // Very short "write code" style prompts — ship a runnable starter instead of only LLM hints
    let wc_short = q.split_whitespace().count();
    if wants_implementation && wc_short <= 5 && q.contains("code") && !mentions_three && !q.contains("webgl") {
        let starter = match lang {
            "rust" => (
                "fn main() {\n    println!(\"Hello from Kala — replace this with your logic.\");\n}",
                "rustc main.rs && ./main",
            ),
            "javascript" | "typescript" => (
                "console.log('Hello from Kala — replace with your logic.');\n// Or: export function main() { ... }",
                "node script.js (or bundle with your toolchain)",
            ),
            _ => (
                "def main() -> None:\n    \"\"\"Entry point — add your task below.\"\"\"\n    print(\"Hello from Kala — replace with your logic.\")\n\n\nif __name__ == \"__main__\":\n    main()\n",
                "python your_script.py",
            ),
        };
        return format!(
            "```{}\n{}\n```\n\n\
             *Runnable starter. For a full solution name the stack (e.g. \"FastAPI JWT login\") or set `khlm_set_llm` / Ollama.*\n\n\
             **Run:** {}",
            lang, starter.0, starter.1
        );
    }

    // Fallback: produce a useful starter program in the detected language
    let (code, note) = match lang {
        "java" => ("import java.util.Scanner;\n\npublic class Main {\n    public static void main(String[] args) {\n        Scanner sc = new Scanner(System.in);\n        System.out.print(\"Enter your name: \");\n        String name = sc.nextLine();\n        System.out.println(\"Hello, \" + name + \"!\");\n\n        System.out.print(\"Enter a number: \");\n        int n = sc.nextInt();\n        System.out.println(n + \" is \" + (n % 2 == 0 ? \"even\" : \"odd\"));\n\n        int sum = 0;\n        for (int i = 1; i <= n; i++) sum += i;\n        System.out.println(\"Sum 1 to \" + n + \" = \" + sum);\n    }\n}", "Java starter — input, even/odd, loop. Try: `write java for loop` or `write java calculator` for something specific."),
        "rust" => ("use std::io::{self, Write};\n\nfn main() {\n    print!(\"Enter your name: \");\n    io::stdout().flush().unwrap();\n    let mut name = String::new();\n    io::stdin().read_line(&mut name).unwrap();\n    println!(\"Hello, {}!\", name.trim());\n\n    let nums = vec![1, 2, 3, 4, 5];\n    let sum: i32 = nums.iter().sum();\n    println!(\"Sum of {:?} = {}\", nums, sum);\n\n    for i in 1..=10 {\n        println!(\"{}: {}\", i, if i % 2 == 0 { \"even\" } else { \"odd\" });\n    }\n}", "Rust starter — I/O, vectors, loops. Try: `write rust struct` or `write rust enum` for specifics."),
        "javascript" | "typescript" => ("const readline = require('readline');\nconst rl = readline.createInterface({ input: process.stdin, output: process.stdout });\n\nrl.question('Enter your name: ', (name) => {\n    console.log(`Hello, ${name}!`);\n\n    const nums = [1, 2, 3, 4, 5];\n    const sum = nums.reduce((a, b) => a + b, 0);\n    console.log(`Sum of [${nums}] = ${sum}`);\n\n    for (let i = 1; i <= 10; i++) {\n        console.log(`${i}: ${i % 2 === 0 ? 'even' : 'odd'}`);\n    }\n    rl.close();\n});", "JavaScript starter. Try: `write javascript for loop` or `write javascript fetch api` for specifics."),
        "cpp" => ("#include <iostream>\n#include <string>\n#include <vector>\nusing namespace std;\n\nint main() {\n    string name;\n    cout << \"Enter your name: \";\n    getline(cin, name);\n    cout << \"Hello, \" << name << \"!\" << endl;\n\n    vector<int> nums = {1, 2, 3, 4, 5};\n    int sum = 0;\n    for (int n : nums) sum += n;\n    cout << \"Sum = \" << sum << endl;\n\n    for (int i = 1; i <= 10; i++)\n        cout << i << \": \" << (i % 2 == 0 ? \"even\" : \"odd\") << endl;\n    return 0;\n}", "C++ starter. Try: `write c++ for loop` or `write c++ class` for specifics."),
        "go" => ("package main\n\nimport \"fmt\"\n\nfunc main() {\n\tvar name string\n\tfmt.Print(\"Enter your name: \")\n\tfmt.Scanln(&name)\n\tfmt.Printf(\"Hello, %s!\\n\", name)\n\n\tnums := []int{1, 2, 3, 4, 5}\n\tsum := 0\n\tfor _, n := range nums {\n\t\tsum += n\n\t}\n\tfmt.Printf(\"Sum = %d\\n\", sum)\n\n\tfor i := 1; i <= 10; i++ {\n\t\tif i%2 == 0 {\n\t\t\tfmt.Printf(\"%d: even\\n\", i)\n\t\t} else {\n\t\t\tfmt.Printf(\"%d: odd\\n\", i)\n\t\t}\n\t}\n}", "Go starter. Try: `write go for loop` or `write go struct` for specifics."),
        "c" => ("#include <stdio.h>\n\nint main() {\n    char name[100];\n    printf(\"Enter your name: \");\n    fgets(name, sizeof(name), stdin);\n    printf(\"Hello, %s\\n\", name);\n\n    int nums[] = {1, 2, 3, 4, 5};\n    int sum = 0;\n    for (int i = 0; i < 5; i++) sum += nums[i];\n    printf(\"Sum = %d\\n\", sum);\n\n    for (int i = 1; i <= 10; i++)\n        printf(\"%d: %s\\n\", i, i % 2 == 0 ? \"even\" : \"odd\");\n    return 0;\n}", "C starter. Try: `write c for loop` or `write c struct` for specifics."),
        _ => ("name = input('Enter your name: ')\nprint(f'Hello, {name}!')\n\nnums = [1, 2, 3, 4, 5]\nprint(f'Sum of {nums} = {sum(nums)}')\n\nfor i in range(1, 11):\n    print(f'{i}: {\"even\" if i % 2 == 0 else \"odd\"}')\n\ndef factorial(n):\n    return 1 if n <= 1 else n * factorial(n - 1)\n\nprint(f'10! = {factorial(10)}')", "Kala: Python starter. Try: `write python for loop` or `write python class` for specifics."),
    };
    format!("```{}\n{}\n```\n\n*{}*", lang, code, note)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROJECT SCAFFOLDING ENGINE — multi-file project generation
// ═══════════════════════════════════════════════════════════════════════════════

fn kala_project_scaffold(q: &str) -> Option<String> {
    let is_project = q.contains("project") || q.contains("scaffold") || q.contains("boilerplate")
        || q.contains("starter") || q.contains("full stack") || q.contains("full-stack")
        || q.contains("fullstack") || q.contains("app ") || q.ends_with(" app")
        || q.contains("application") || q.contains("website") || q.contains("web app")
        || q.contains("webapp") || q.contains("setup");

    if !is_project { return None; }

    // ── React App ───────────────────────────────────────────────────────────
    if (q.contains("react") && !q.contains("native")) && !q.contains("vue") && !q.contains("angular") {
        if q.contains("django") {
            return Some(scaffold_django_react_fullstack());
        }
        if q.contains("full stack") || q.contains("full-stack") || q.contains("fullstack") || (q.contains("express") || q.contains("node") || q.contains("backend")) {
            return Some(scaffold_react_express_fullstack());
        }
        return Some(scaffold_react_app());
    }

    // ── Next.js App ─────────────────────────────────────────────────────────
    if q.contains("next.js") || q.contains("nextjs") || q.contains("next js") {
        return Some(scaffold_nextjs_app());
    }

    // ── Vue.js App ──────────────────────────────────────────────────────────
    if q.contains("vue") {
        return Some(scaffold_vue_app());
    }

    // ── Angular App ─────────────────────────────────────────────────────────
    if q.contains("angular") {
        return Some(scaffold_angular_app());
    }

    // ── Express API ─────────────────────────────────────────────────────────
    if q.contains("express") && !q.contains("react") {
        return Some(scaffold_express_api());
    }

    // ── Django ───────────────────────────────────────────────────────────────
    if q.contains("django") {
        if q.contains("react") || q.contains("full stack") || q.contains("full-stack") {
            return Some(scaffold_django_react_fullstack());
        }
        return Some(scaffold_django_app());
    }

    // ── FastAPI ──────────────────────────────────────────────────────────────
    if q.contains("fastapi") || q.contains("fast api") {
        return Some(scaffold_fastapi_app());
    }

    // ── Flask ────────────────────────────────────────────────────────────────
    if q.contains("flask") {
        return Some(scaffold_flask_app());
    }

    // ── Spring Boot ──────────────────────────────────────────────────────────
    if q.contains("spring boot") || q.contains("springboot") || q.contains("spring") && q.contains("java") {
        return Some(scaffold_spring_boot_app());
    }

    // ── Rust project ─────────────────────────────────────────────────────────
    if q.contains("rust") {
        if q.contains("api") || q.contains("web") || q.contains("server") {
            return Some(scaffold_rust_web_project());
        }
        return Some(scaffold_rust_project());
    }

    // ── Go project ───────────────────────────────────────────────────────────
    if q.contains("go ") || q.contains("golang") || q.ends_with(" go") {
        return Some(scaffold_go_project());
    }

    // ── Python package ───────────────────────────────────────────────────────
    if q.contains("python") && (q.contains("package") || q.contains("library") || q.contains("module")) {
        return Some(scaffold_python_package());
    }

    // ── Node CLI tool ────────────────────────────────────────────────────────
    if (q.contains("node") || q.contains("npm")) && (q.contains("cli") || q.contains("command line") || q.contains("tool")) {
        return Some(scaffold_node_cli());
    }

    // ── HTML/CSS/JS website ──────────────────────────────────────────────────
    if q.contains("html") || q.contains("static") || (q.contains("website") && !q.contains("react") && !q.contains("vue") && !q.contains("angular")) {
        return Some(scaffold_html_website());
    }

    // ── Docker ───────────────────────────────────────────────────────────────
    if q.contains("docker") {
        return Some(scaffold_docker_compose());
    }

    // ── Generic "create project" without a specific framework ────────────────
    if q.contains("python") {
        return Some(scaffold_python_package());
    }
    if q.contains("java") && !q.contains("javascript") {
        return Some(scaffold_spring_boot_app());
    }
    if q.contains("node") || q.contains("javascript") || q.contains("typescript") {
        return Some(scaffold_express_api());
    }

    None
}

fn file_block(path: &str, lang: &str, code: &str) -> String {
    format!("### `{}`\n```{}\n{}\n```\n", path, lang, code)
}

fn project_header(name: &str, desc: &str, tree: &str) -> String {
    format!("## {} Project: {}\n\n{}\n\n```\n{}\n```\n\n---\n\n", "\u{1F4C1}", name, desc, tree)
}

fn project_footer(run_cmd: &str) -> String {
    format!("\n---\n\n**Quick start:**\n```bash\n{}\n```\n", run_cmd)
}

// ─────────────────────────────────────────────────────────────────────────────
// REACT APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_react_app() -> String {
    let mut out = project_header("React", "Modern React app with hooks, routing, and CSS modules.",
        "my-react-app/\n├── package.json\n├── public/\n│   └── index.html\n├── src/\n│   ├── index.jsx\n│   ├── App.jsx\n│   ├── App.css\n│   ├── components/\n│   │   ├── Header.jsx\n│   │   └── Footer.jsx\n│   └── pages/\n│       ├── Home.jsx\n│       └── About.jsx\n└── .gitignore");
    out += &file_block("package.json", "json", r#"{
  "name": "my-react-app",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.20.0",
    "react-scripts": "5.0.1"
  },
  "scripts": {
    "start": "react-scripts start",
    "build": "react-scripts build",
    "test": "react-scripts test"
  },
  "browserslist": { "production": [">0.2%"], "development": ["last 1 chrome version"] }
}"#);
    out += &file_block("public/index.html", "html", r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>My React App</title>
</head>
<body>
  <div id="root"></div>
</body>
</html>"#);
    out += &file_block("src/index.jsx", "jsx", r#"import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import App from './App';
import './App.css';

ReactDOM.createRoot(document.getElementById('root')).render(
  <BrowserRouter><App /></BrowserRouter>
);"#);
    out += &file_block("src/App.jsx", "jsx", r#"import { Routes, Route, Link } from 'react-router-dom';
import Header from './components/Header';
import Footer from './components/Footer';
import Home from './pages/Home';
import About from './pages/About';

export default function App() {
  return (
    <div className="app">
      <Header />
      <main className="container">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/about" element={<About />} />
        </Routes>
      </main>
      <Footer />
    </div>
  );
}"#);
    out += &file_block("src/App.css", "css", r#"* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #f5f5f5; color: #333; }
.app { min-height: 100vh; display: flex; flex-direction: column; }
.container { max-width: 960px; margin: 0 auto; padding: 2rem; flex: 1; }
nav { background: #1a1a2e; padding: 1rem 2rem; display: flex; gap: 1.5rem; align-items: center; }
nav a { color: #e0e0e0; text-decoration: none; font-weight: 500; }
nav a:hover { color: #00d4ff; }
nav .brand { font-size: 1.4rem; font-weight: 700; color: #00d4ff; }
footer { background: #1a1a2e; color: #aaa; text-align: center; padding: 1rem; margin-top: auto; }
h1 { margin-bottom: 1rem; color: #1a1a2e; }
.card { background: #fff; border-radius: 12px; padding: 1.5rem; margin: 1rem 0; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }"#);
    out += &file_block("src/components/Header.jsx", "jsx", r#"import { Link } from 'react-router-dom';

export default function Header() {
  return (
    <nav>
      <Link to="/" className="brand">MyApp</Link>
      <Link to="/">Home</Link>
      <Link to="/about">About</Link>
    </nav>
  );
}"#);
    out += &file_block("src/components/Footer.jsx", "jsx", r#"export default function Footer() {
  return <footer>&copy; {new Date().getFullYear()} MyApp. Built with Kala.</footer>;
}"#);
    out += &file_block("src/pages/Home.jsx", "jsx", r#"import { useState } from 'react';

export default function Home() {
  const [count, setCount] = useState(0);
  return (
    <div>
      <h1>Welcome Home</h1>
      <div className="card">
        <p>Count: {count}</p>
        <button onClick={() => setCount(c => c + 1)}>Increment</button>
        <button onClick={() => setCount(0)} style={{marginLeft: 8}}>Reset</button>
      </div>
    </div>
  );
}"#);
    out += &file_block("src/pages/About.jsx", "jsx", r#"export default function About() {
  return (
    <div>
      <h1>About</h1>
      <div className="card">
        <p>This project was scaffolded by <strong>Kala AI</strong> — the AI built into the Killer programming language.</p>
      </div>
    </div>
  );
}"#);
    out += &file_block(".gitignore", "text", "node_modules/\nbuild/\n.env\n.DS_Store");
    out += &project_footer("npx create-react-app my-react-app   # or paste these files\ncd my-react-app\nnpm start");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// NEXT.JS APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_nextjs_app() -> String {
    let mut out = project_header("Next.js", "Next.js 14 app with App Router, API routes, and Tailwind CSS.",
        "my-nextjs-app/\n├── package.json\n├── next.config.js\n├── tailwind.config.js\n├── app/\n│   ├── layout.js\n│   ├── page.js\n│   ├── globals.css\n│   ├── about/\n│   │   └── page.js\n│   └── api/\n│       └── hello/\n│           └── route.js\n└── .gitignore");
    out += &file_block("package.json", "json", r#"{
  "name": "my-nextjs-app",
  "version": "1.0.0",
  "private": true,
  "scripts": { "dev": "next dev", "build": "next build", "start": "next start" },
  "dependencies": { "next": "^14.0.0", "react": "^18.2.0", "react-dom": "^18.2.0" },
  "devDependencies": { "tailwindcss": "^3.4.0", "autoprefixer": "^10.4.0", "postcss": "^8.4.0" }
}"#);
    out += &file_block("next.config.js", "javascript", "/** @type {import('next').NextConfig} */\nmodule.exports = { reactStrictMode: true };");
    out += &file_block("tailwind.config.js", "javascript", r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./app/**/*.{js,jsx}', './components/**/*.{js,jsx}'],
  theme: { extend: {} },
  plugins: [],
};"#);
    out += &file_block("app/globals.css", "css", "@tailwind base;\n@tailwind components;\n@tailwind utilities;\n\nbody { @apply bg-gray-50 text-gray-900; }");
    out += &file_block("app/layout.js", "jsx", r#"import './globals.css';
import Link from 'next/link';

export const metadata = { title: 'My Next.js App', description: 'Scaffolded by Kala AI' };

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <nav className="bg-slate-900 text-white p-4 flex gap-6">
          <Link href="/" className="font-bold text-cyan-400 text-xl">MyApp</Link>
          <Link href="/" className="hover:text-cyan-300">Home</Link>
          <Link href="/about" className="hover:text-cyan-300">About</Link>
        </nav>
        <main className="max-w-4xl mx-auto p-8">{children}</main>
        <footer className="bg-slate-900 text-gray-400 text-center p-4 mt-auto">
          &copy; {new Date().getFullYear()} Built with Kala AI
        </footer>
      </body>
    </html>
  );
}"#);
    out += &file_block("app/page.js", "jsx", r#"'use client';
import { useState } from 'react';

export default function Home() {
  const [count, setCount] = useState(0);
  return (
    <div>
      <h1 className="text-3xl font-bold mb-4">Welcome to Next.js</h1>
      <div className="bg-white rounded-xl shadow p-6">
        <p className="text-lg mb-2">Count: {count}</p>
        <button onClick={() => setCount(c => c + 1)} className="bg-cyan-500 text-white px-4 py-2 rounded mr-2 hover:bg-cyan-600">+1</button>
        <button onClick={() => setCount(0)} className="bg-gray-200 px-4 py-2 rounded hover:bg-gray-300">Reset</button>
      </div>
    </div>
  );
}"#);
    out += &file_block("app/about/page.js", "jsx", r#"export default function About() {
  return (
    <div>
      <h1 className="text-3xl font-bold mb-4">About</h1>
      <div className="bg-white rounded-xl shadow p-6">
        <p>Scaffolded by <strong>Kala AI</strong> — the AI in the Killer programming language.</p>
      </div>
    </div>
  );
}"#);
    out += &file_block("app/api/hello/route.js", "javascript", r#"import { NextResponse } from 'next/server';

export async function GET() {
  return NextResponse.json({ message: 'Hello from Kala API!', timestamp: new Date().toISOString() });
}

export async function POST(request) {
  const body = await request.json();
  return NextResponse.json({ received: body, status: 'ok' });
}"#);
    out += &file_block(".gitignore", "text", "node_modules/\n.next/\n.env\n.DS_Store");
    out += &project_footer("npx create-next-app@latest my-nextjs-app   # or paste files\ncd my-nextjs-app\nnpm run dev");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// VUE.JS APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_vue_app() -> String {
    let mut out = project_header("Vue.js", "Vue 3 app with Composition API, Vue Router, and scoped styles.",
        "my-vue-app/\n├── package.json\n├── vite.config.js\n├── index.html\n├── src/\n│   ├── main.js\n│   ├── App.vue\n│   ├── router.js\n│   ├── views/\n│   │   ├── Home.vue\n│   │   └── About.vue\n│   └── components/\n│       └── NavBar.vue\n└── .gitignore");
    out += &file_block("package.json", "json", r#"{
  "name": "my-vue-app",
  "version": "1.0.0",
  "private": true,
  "scripts": { "dev": "vite", "build": "vite build", "preview": "vite preview" },
  "dependencies": { "vue": "^3.4.0", "vue-router": "^4.2.0" },
  "devDependencies": { "@vitejs/plugin-vue": "^5.0.0", "vite": "^5.0.0" }
}"#);
    out += &file_block("vite.config.js", "javascript", "import { defineConfig } from 'vite';\nimport vue from '@vitejs/plugin-vue';\n\nexport default defineConfig({ plugins: [vue()] });");
    out += &file_block("index.html", "html", "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"UTF-8\"/><meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\"/><title>My Vue App</title></head>\n<body><div id=\"app\"></div><script type=\"module\" src=\"/src/main.js\"></script></body>\n</html>");
    out += &file_block("src/main.js", "javascript", "import { createApp } from 'vue';\nimport App from './App.vue';\nimport router from './router';\n\ncreateApp(App).use(router).mount('#app');");
    out += &file_block("src/router.js", "javascript", "import { createRouter, createWebHistory } from 'vue-router';\nimport Home from './views/Home.vue';\nimport About from './views/About.vue';\n\nexport default createRouter({\n  history: createWebHistory(),\n  routes: [\n    { path: '/', component: Home },\n    { path: '/about', component: About },\n  ],\n});");
    out += &file_block("src/App.vue", "html", "<template>\n  <NavBar />\n  <main class=\"container\"><router-view /></main>\n  <footer>&copy; {{ new Date().getFullYear() }} Built with Kala AI</footer>\n</template>\n\n<script setup>\nimport NavBar from './components/NavBar.vue';\n</script>\n\n<style>\n* { margin: 0; padding: 0; box-sizing: border-box; }\nbody { font-family: sans-serif; background: #f5f5f5; }\n.container { max-width: 960px; margin: 0 auto; padding: 2rem; }\nfooter { background: #1a1a2e; color: #aaa; text-align: center; padding: 1rem; }\n</style>");
    out += &file_block("src/components/NavBar.vue", "html", "<template>\n  <nav>\n    <router-link to=\"/\" class=\"brand\">MyApp</router-link>\n    <router-link to=\"/\">Home</router-link>\n    <router-link to=\"/about\">About</router-link>\n  </nav>\n</template>\n\n<style scoped>\nnav { background: #1a1a2e; padding: 1rem 2rem; display: flex; gap: 1.5rem; }\nnav a { color: #e0e0e0; text-decoration: none; }\nnav .brand { font-weight: 700; color: #00d4ff; font-size: 1.3rem; }\nnav a:hover { color: #00d4ff; }\n</style>");
    out += &file_block("src/views/Home.vue", "html", "<template>\n  <div>\n    <h1>Welcome to Vue</h1>\n    <div class=\"card\">\n      <p>Count: {{ count }}</p>\n      <button @click=\"count++\">+1</button>\n      <button @click=\"count = 0\">Reset</button>\n    </div>\n  </div>\n</template>\n\n<script setup>\nimport { ref } from 'vue';\nconst count = ref(0);\n</script>\n\n<style scoped>\n.card { background: #fff; border-radius: 12px; padding: 1.5rem; margin-top: 1rem; box-shadow: 0 2px 8px rgba(0,0,0,.08); }\nbutton { padding: .5rem 1rem; margin-right: .5rem; border: none; border-radius: 6px; cursor: pointer; background: #00d4ff; color: #fff; }\n</style>");
    out += &file_block("src/views/About.vue", "html", "<template>\n  <div>\n    <h1>About</h1>\n    <div class=\"card\"><p>Scaffolded by <strong>Kala AI</strong>.</p></div>\n  </div>\n</template>\n\n<style scoped>\n.card { background: #fff; border-radius: 12px; padding: 1.5rem; margin-top: 1rem; box-shadow: 0 2px 8px rgba(0,0,0,.08); }\n</style>");
    out += &file_block(".gitignore", "text", "node_modules/\ndist/\n.env\n.DS_Store");
    out += &project_footer("npm create vite@latest my-vue-app -- --template vue   # or paste files\ncd my-vue-app\nnpm install && npm run dev");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// ANGULAR APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_angular_app() -> String {
    let mut out = project_header("Angular", "Angular 17 standalone app with routing and components.",
        "my-angular-app/\n├── package.json\n├── tsconfig.json\n├── angular.json\n├── src/\n│   ├── main.ts\n│   ├── index.html\n│   ├── styles.css\n│   └── app/\n│       ├── app.component.ts\n│       ├── app.routes.ts\n│       ├── home/\n│       │   └── home.component.ts\n│       └── about/\n│           └── about.component.ts\n└── .gitignore");
    out += &file_block("src/app/app.component.ts", "typescript", r#"import { Component } from '@angular/core';
import { RouterOutlet, RouterLink } from '@angular/router';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, RouterLink],
  template: `
    <nav>
      <a routerLink="/" class="brand">MyApp</a>
      <a routerLink="/">Home</a>
      <a routerLink="/about">About</a>
    </nav>
    <main class="container"><router-outlet /></main>
    <footer>&copy; 2025 Built with Kala AI</footer>
  `,
  styles: [`
    nav { background: #1a1a2e; padding: 1rem 2rem; display: flex; gap: 1.5rem; }
    nav a { color: #e0e0e0; text-decoration: none; }
    nav .brand { font-weight: 700; color: #00d4ff; font-size: 1.3rem; }
    .container { max-width: 960px; margin: 0 auto; padding: 2rem; }
    footer { background: #1a1a2e; color: #aaa; text-align: center; padding: 1rem; }
  `]
})
export class AppComponent {}"#);
    out += &file_block("src/app/app.routes.ts", "typescript", "import { Routes } from '@angular/router';\nimport { HomeComponent } from './home/home.component';\nimport { AboutComponent } from './about/about.component';\n\nexport const routes: Routes = [\n  { path: '', component: HomeComponent },\n  { path: 'about', component: AboutComponent },\n];");
    out += &file_block("src/app/home/home.component.ts", "typescript", "import { Component } from '@angular/core';\n\n@Component({\n  selector: 'app-home',\n  standalone: true,\n  template: `\n    <h1>Welcome to Angular</h1>\n    <div class=\"card\">\n      <p>Count: {{ count }}</p>\n      <button (click)=\"count = count + 1\">+1</button>\n      <button (click)=\"count = 0\">Reset</button>\n    </div>\n  `,\n  styles: [`.card { background: #fff; border-radius: 12px; padding: 1.5rem; margin-top: 1rem; box-shadow: 0 2px 8px rgba(0,0,0,.08); } button { padding: .5rem 1rem; margin-right: .5rem; border: none; border-radius: 6px; cursor: pointer; background: #00d4ff; color: #fff; }`]\n})\nexport class HomeComponent { count = 0; }");
    out += &file_block("src/app/about/about.component.ts", "typescript", "import { Component } from '@angular/core';\n\n@Component({\n  selector: 'app-about',\n  standalone: true,\n  template: `<h1>About</h1><div class=\"card\"><p>Scaffolded by <strong>Kala AI</strong>.</p></div>`,\n  styles: [`.card { background: #fff; border-radius: 12px; padding: 1.5rem; margin-top: 1rem; box-shadow: 0 2px 8px rgba(0,0,0,.08); }`]\n})\nexport class AboutComponent {}");
    out += &file_block(".gitignore", "text", "node_modules/\ndist/\n.angular/\n.env");
    out += &project_footer("ng new my-angular-app --standalone   # or paste files\ncd my-angular-app\nng serve");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// EXPRESS API
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_express_api() -> String {
    let mut out = project_header("Express API", "REST API with Express.js — CRUD, middleware, error handling.",
        "my-express-api/\n├── package.json\n├── server.js\n├── routes/\n│   └── items.js\n├── middleware/\n│   └── errorHandler.js\n├── .env.example\n├── Dockerfile\n└── .gitignore");
    out += &file_block("package.json", "json", "{\n  \"name\": \"my-express-api\",\n  \"version\": \"1.0.0\",\n  \"main\": \"server.js\",\n  \"scripts\": { \"start\": \"node server.js\", \"dev\": \"node --watch server.js\" },\n  \"dependencies\": { \"express\": \"^4.18.0\", \"cors\": \"^2.8.5\", \"dotenv\": \"^16.3.0\" }\n}");
    out += &file_block("server.js", "javascript", "require('dotenv').config();\nconst express = require('express');\nconst cors = require('cors');\nconst itemsRouter = require('./routes/items');\nconst errorHandler = require('./middleware/errorHandler');\n\nconst app = express();\nconst PORT = process.env.PORT || 3000;\n\napp.use(cors());\napp.use(express.json());\n\napp.get('/', (req, res) => res.json({ status: 'ok', message: 'API is running' }));\napp.use('/api/items', itemsRouter);\napp.use(errorHandler);\n\napp.listen(PORT, () => console.log(`Server running on http://localhost:${PORT}`));");
    out += &file_block("routes/items.js", "javascript", "const express = require('express');\nconst router = express.Router();\n\nlet items = [\n  { id: 1, name: 'Item 1', price: 9.99 },\n  { id: 2, name: 'Item 2', price: 19.99 },\n];\nlet nextId = 3;\n\nrouter.get('/', (req, res) => res.json(items));\n\nrouter.get('/:id', (req, res) => {\n  const item = items.find(i => i.id === +req.params.id);\n  if (!item) return res.status(404).json({ error: 'Not found' });\n  res.json(item);\n});\n\nrouter.post('/', (req, res) => {\n  const { name, price } = req.body;\n  if (!name) return res.status(400).json({ error: 'Name required' });\n  const item = { id: nextId++, name, price: price || 0 };\n  items.push(item);\n  res.status(201).json(item);\n});\n\nrouter.put('/:id', (req, res) => {\n  const item = items.find(i => i.id === +req.params.id);\n  if (!item) return res.status(404).json({ error: 'Not found' });\n  Object.assign(item, req.body);\n  res.json(item);\n});\n\nrouter.delete('/:id', (req, res) => {\n  items = items.filter(i => i.id !== +req.params.id);\n  res.status(204).end();\n});\n\nmodule.exports = router;");
    out += &file_block("middleware/errorHandler.js", "javascript", "module.exports = (err, req, res, _next) => {\n  console.error(err.stack);\n  res.status(err.status || 500).json({\n    error: err.message || 'Internal Server Error',\n  });\n};");
    out += &file_block(".env.example", "text", "PORT=3000\nNODE_ENV=development");
    out += &file_block("Dockerfile", "dockerfile", "FROM node:20-alpine\nWORKDIR /app\nCOPY package*.json ./\nRUN npm ci --production\nCOPY . .\nEXPOSE 3000\nCMD [\"node\", \"server.js\"]");
    out += &file_block(".gitignore", "text", "node_modules/\n.env\n.DS_Store");
    out += &project_footer("mkdir my-express-api && cd my-express-api\n# paste files, then:\nnpm install\nnpm run dev");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// DJANGO APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_django_app() -> String {
    let mut out = project_header("Django", "Django app with models, views, templates, and admin.",
        "myproject/\n├── requirements.txt\n├── manage.py\n├── myproject/\n│   ├── __init__.py\n│   ├── settings.py\n│   ├── urls.py\n│   └── wsgi.py\n├── app/\n│   ├── __init__.py\n│   ├── models.py\n│   ├── views.py\n│   ├── urls.py\n│   ├── admin.py\n│   └── templates/\n│       └── app/\n│           ├── base.html\n│           └── home.html\n├── Dockerfile\n└── .gitignore");
    out += &file_block("requirements.txt", "text", "django>=5.0\ngunicorn>=21.2");
    out += &file_block("manage.py", "python", "#!/usr/bin/env python\nimport os, sys\n\ndef main():\n    os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'myproject.settings')\n    from django.core.management import execute_from_command_line\n    execute_from_command_line(sys.argv)\n\nif __name__ == '__main__': main()");
    out += &file_block("myproject/settings.py", "python", "from pathlib import Path\n\nBASE_DIR = Path(__file__).resolve().parent.parent\nSECRET_KEY = 'change-me-in-production'\nDEBUG = True\nALLOWED_HOSTS = ['*']\n\nINSTALLED_APPS = [\n    'django.contrib.admin', 'django.contrib.auth', 'django.contrib.contenttypes',\n    'django.contrib.sessions', 'django.contrib.messages', 'django.contrib.staticfiles',\n    'app',\n]\n\nMIDDLEWARE = [\n    'django.middleware.security.SecurityMiddleware', 'django.contrib.sessions.middleware.SessionMiddleware',\n    'django.middleware.common.CommonMiddleware', 'django.middleware.csrf.CsrfViewMiddleware',\n    'django.contrib.auth.middleware.AuthenticationMiddleware', 'django.contrib.messages.middleware.MessageMiddleware',\n]\n\nROOT_URLCONF = 'myproject.urls'\nTEMPLATES = [{'BACKEND': 'django.template.backends.django.DjangoTemplates', 'DIRS': [], 'APP_DIRS': True,\n    'OPTIONS': {'context_processors': ['django.template.context_processors.request', 'django.contrib.auth.context_processors.auth', 'django.contrib.messages.context_processors.messages']}}]\n\nDATABASES = {'default': {'ENGINE': 'django.db.backends.sqlite3', 'NAME': BASE_DIR / 'db.sqlite3'}}\nSTATIC_URL = '/static/'");
    out += &file_block("myproject/urls.py", "python", "from django.contrib import admin\nfrom django.urls import path, include\n\nurlpatterns = [\n    path('admin/', admin.site.urls),\n    path('', include('app.urls')),\n]");
    out += &file_block("app/models.py", "python", "from django.db import models\n\nclass Item(models.Model):\n    name = models.CharField(max_length=200)\n    description = models.TextField(blank=True)\n    price = models.DecimalField(max_digits=10, decimal_places=2, default=0)\n    created_at = models.DateTimeField(auto_now_add=True)\n\n    def __str__(self): return self.name\n\n    class Meta: ordering = ['-created_at']");
    out += &file_block("app/views.py", "python", "from django.shortcuts import render\nfrom .models import Item\n\ndef home(request):\n    items = Item.objects.all()[:20]\n    return render(request, 'app/home.html', {'items': items})");
    out += &file_block("app/urls.py", "python", "from django.urls import path\nfrom . import views\n\nurlpatterns = [\n    path('', views.home, name='home'),\n]");
    out += &file_block("app/admin.py", "python", "from django.contrib import admin\nfrom .models import Item\n\n@admin.register(Item)\nclass ItemAdmin(admin.ModelAdmin):\n    list_display = ['name', 'price', 'created_at']\n    search_fields = ['name']");
    out += &file_block("app/templates/app/base.html", "html", "<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\"/><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n<title>{% block title %}MyApp{% endblock %}</title>\n<style>*{margin:0;padding:0;box-sizing:border-box}body{font-family:sans-serif;background:#f5f5f5}\nnav{background:#1a1a2e;padding:1rem 2rem;color:#fff}nav a{color:#00d4ff;text-decoration:none;margin-right:1.5rem}\n.container{max-width:960px;margin:0 auto;padding:2rem}\n.card{background:#fff;border-radius:12px;padding:1.5rem;margin:1rem 0;box-shadow:0 2px 8px rgba(0,0,0,.08)}\nfooter{background:#1a1a2e;color:#aaa;text-align:center;padding:1rem}</style>\n</head><body>\n<nav><a href=\"/\"><strong>MyApp</strong></a><a href=\"/\">Home</a><a href=\"/admin/\">Admin</a></nav>\n<div class=\"container\">{% block content %}{% endblock %}</div>\n<footer>&copy; 2025 Built with Kala AI</footer>\n</body></html>");
    out += &file_block("app/templates/app/home.html", "html", "{% extends 'app/base.html' %}\n{% block title %}Home{% endblock %}\n{% block content %}\n<h1>Items</h1>\n{% for item in items %}\n  <div class=\"card\"><h3>{{ item.name }}</h3><p>${{ item.price }}</p><small>{{ item.created_at|date:'M d, Y' }}</small></div>\n{% empty %}\n  <p>No items yet. <a href=\"/admin/\">Add some in admin</a>.</p>\n{% endfor %}\n{% endblock %}");
    out += &file_block("Dockerfile", "dockerfile", "FROM python:3.12-slim\nWORKDIR /app\nCOPY requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY . .\nRUN python manage.py collectstatic --noinput 2>/dev/null || true\nEXPOSE 8000\nCMD [\"gunicorn\", \"myproject.wsgi:application\", \"--bind\", \"0.0.0.0:8000\"]");
    out += &file_block(".gitignore", "text", "db.sqlite3\n*.pyc\n__pycache__/\n.env\n.DS_Store\nstaticfiles/");
    out += &project_footer("pip install -r requirements.txt\npython manage.py migrate\npython manage.py createsuperuser\npython manage.py runserver");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// FASTAPI APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_fastapi_app() -> String {
    let mut out = project_header("FastAPI", "FastAPI app with SQLite, Pydantic models, CORS, and auto-docs.",
        "my-fastapi-app/\n├── requirements.txt\n├── main.py\n├── models.py\n├── database.py\n├── Dockerfile\n└── .gitignore");
    out += &file_block("requirements.txt", "text", "fastapi>=0.109.0\nuvicorn[standard]>=0.27.0\nsqlalchemy>=2.0.0\npydantic>=2.5.0");
    out += &file_block("database.py", "python", "from sqlalchemy import create_engine\nfrom sqlalchemy.orm import sessionmaker, DeclarativeBase\n\nDATABASE_URL = 'sqlite:///./app.db'\nengine = create_engine(DATABASE_URL, connect_args={'check_same_thread': False})\nSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)\n\nclass Base(DeclarativeBase): pass\n\ndef get_db():\n    db = SessionLocal()\n    try: yield db\n    finally: db.close()");
    out += &file_block("models.py", "python", "from sqlalchemy import Column, Integer, String, Float, DateTime\nfrom datetime import datetime\nfrom database import Base\n\nclass Item(Base):\n    __tablename__ = 'items'\n    id = Column(Integer, primary_key=True, index=True)\n    name = Column(String, nullable=False)\n    description = Column(String, default='')\n    price = Column(Float, default=0.0)\n    created_at = Column(DateTime, default=datetime.utcnow)");
    out += &file_block("main.py", "python", "from fastapi import FastAPI, Depends, HTTPException\nfrom fastapi.middleware.cors import CORSMiddleware\nfrom sqlalchemy.orm import Session\nfrom pydantic import BaseModel\nfrom typing import Optional\nfrom database import engine, get_db, Base\nfrom models import Item\n\nBase.metadata.create_all(bind=engine)\napp = FastAPI(title='My API', description='Scaffolded by Kala AI')\napp.add_middleware(CORSMiddleware, allow_origins=['*'], allow_methods=['*'], allow_headers=['*'])\n\nclass ItemCreate(BaseModel):\n    name: str\n    description: Optional[str] = ''\n    price: float = 0.0\n\nclass ItemOut(ItemCreate):\n    id: int\n    class Config: from_attributes = True\n\n@app.get('/')\ndef root(): return {'status': 'ok', 'docs': '/docs'}\n\n@app.get('/api/items', response_model=list[ItemOut])\ndef list_items(db: Session = Depends(get_db)):\n    return db.query(Item).order_by(Item.id.desc()).limit(50).all()\n\n@app.post('/api/items', response_model=ItemOut, status_code=201)\ndef create_item(data: ItemCreate, db: Session = Depends(get_db)):\n    item = Item(**data.dict())\n    db.add(item); db.commit(); db.refresh(item)\n    return item\n\n@app.get('/api/items/{item_id}', response_model=ItemOut)\ndef get_item(item_id: int, db: Session = Depends(get_db)):\n    item = db.query(Item).filter(Item.id == item_id).first()\n    if not item: raise HTTPException(404, 'Not found')\n    return item\n\n@app.delete('/api/items/{item_id}', status_code=204)\ndef delete_item(item_id: int, db: Session = Depends(get_db)):\n    item = db.query(Item).filter(Item.id == item_id).first()\n    if not item: raise HTTPException(404, 'Not found')\n    db.delete(item); db.commit()");
    out += &file_block("Dockerfile", "dockerfile", "FROM python:3.12-slim\nWORKDIR /app\nCOPY requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY . .\nEXPOSE 8000\nCMD [\"uvicorn\", \"main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8000\"]");
    out += &file_block(".gitignore", "text", "*.db\n*.pyc\n__pycache__/\n.env\n.DS_Store");
    out += &project_footer("pip install -r requirements.txt\nuvicorn main:app --reload\n# Open http://localhost:8000/docs for auto-generated API docs");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// FLASK APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_flask_app() -> String {
    let mut out = project_header("Flask", "Flask app with templates, SQLite, and blueprints.",
        "my-flask-app/\n├── requirements.txt\n├── app.py\n├── templates/\n│   ├── base.html\n│   └── home.html\n├── static/\n│   └── style.css\n├── Dockerfile\n└── .gitignore");
    out += &file_block("requirements.txt", "text", "flask>=3.0.0\ngunicorn>=21.2.0");
    out += &file_block("app.py", "python", "from flask import Flask, render_template, request, jsonify, redirect, url_for\nimport sqlite3, os\n\napp = Flask(__name__)\nDB = 'app.db'\n\ndef get_db():\n    conn = sqlite3.connect(DB)\n    conn.row_factory = sqlite3.Row\n    return conn\n\ndef init_db():\n    with get_db() as db:\n        db.execute('CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, price REAL DEFAULT 0)')\n\n@app.route('/')\ndef home():\n    with get_db() as db:\n        items = db.execute('SELECT * FROM items ORDER BY id DESC').fetchall()\n    return render_template('home.html', items=items)\n\n@app.route('/add', methods=['POST'])\ndef add_item():\n    name = request.form.get('name', '').strip()\n    price = float(request.form.get('price', 0))\n    if name:\n        with get_db() as db:\n            db.execute('INSERT INTO items (name, price) VALUES (?, ?)', (name, price))\n    return redirect(url_for('home'))\n\n@app.route('/api/items')\ndef api_items():\n    with get_db() as db:\n        items = [dict(r) for r in db.execute('SELECT * FROM items').fetchall()]\n    return jsonify(items)\n\nif __name__ == '__main__':\n    init_db()\n    app.run(debug=True)");
    out += &file_block("templates/base.html", "html", "<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\"/><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n<title>{% block title %}MyApp{% endblock %}</title>\n<link rel=\"stylesheet\" href=\"{{ url_for('static', filename='style.css') }}\">\n</head><body>\n<nav><a href=\"/\" class=\"brand\">MyApp</a><a href=\"/\">Home</a><a href=\"/api/items\">API</a></nav>\n<div class=\"container\">{% block content %}{% endblock %}</div>\n<footer>&copy; 2025 Built with Kala AI</footer>\n</body></html>");
    out += &file_block("templates/home.html", "html", "{% extends 'base.html' %}\n{% block content %}\n<h1>Items</h1>\n<form action=\"/add\" method=\"post\" class=\"card\">\n  <input name=\"name\" placeholder=\"Item name\" required/>\n  <input name=\"price\" type=\"number\" step=\"0.01\" placeholder=\"Price\"/>\n  <button type=\"submit\">Add</button>\n</form>\n{% for item in items %}\n  <div class=\"card\"><h3>{{ item.name }}</h3><p>${{ '%.2f'|format(item.price) }}</p></div>\n{% endfor %}\n{% endblock %}");
    out += &file_block("static/style.css", "css", "*{margin:0;padding:0;box-sizing:border-box}body{font-family:sans-serif;background:#f5f5f5}\nnav{background:#1a1a2e;padding:1rem 2rem;display:flex;gap:1.5rem}nav a{color:#e0e0e0;text-decoration:none}nav .brand{font-weight:700;color:#00d4ff;font-size:1.3rem}\n.container{max-width:960px;margin:0 auto;padding:2rem}\n.card{background:#fff;border-radius:12px;padding:1.5rem;margin:1rem 0;box-shadow:0 2px 8px rgba(0,0,0,.08)}\ninput,button{padding:.5rem 1rem;margin:.25rem;border:1px solid #ddd;border-radius:6px}\nbutton{background:#00d4ff;color:#fff;border:none;cursor:pointer}\nfooter{background:#1a1a2e;color:#aaa;text-align:center;padding:1rem}");
    out += &file_block("Dockerfile", "dockerfile", "FROM python:3.12-slim\nWORKDIR /app\nCOPY requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY . .\nEXPOSE 5000\nCMD [\"gunicorn\", \"app:app\", \"--bind\", \"0.0.0.0:5000\"]");
    out += &file_block(".gitignore", "text", "*.db\n*.pyc\n__pycache__/\n.env");
    out += &project_footer("pip install -r requirements.txt\npython app.py\n# Open http://localhost:5000");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// SPRING BOOT APP
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_spring_boot_app() -> String {
    let mut out = project_header("Spring Boot", "Spring Boot 3 REST API with JPA, H2, and validation.",
        "my-spring-app/\n├── pom.xml\n├── src/main/java/com/example/\n│   ├── Application.java\n│   ├── controller/\n│   │   └── ItemController.java\n│   ├── model/\n│   │   └── Item.java\n│   ├── repository/\n│   │   └── ItemRepository.java\n│   └── service/\n│       └── ItemService.java\n├── src/main/resources/\n│   └── application.properties\n├── Dockerfile\n└── .gitignore");
    out += &file_block("pom.xml", "xml", "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd\">\n  <modelVersion>4.0.0</modelVersion>\n  <parent><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-parent</artifactId><version>3.2.0</version></parent>\n  <groupId>com.example</groupId><artifactId>my-spring-app</artifactId><version>1.0.0</version>\n  <properties><java.version>17</java.version></properties>\n  <dependencies>\n    <dependency><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-web</artifactId></dependency>\n    <dependency><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-data-jpa</artifactId></dependency>\n    <dependency><groupId>com.h2database</groupId><artifactId>h2</artifactId><scope>runtime</scope></dependency>\n    <dependency><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-validation</artifactId></dependency>\n  </dependencies>\n  <build><plugins><plugin><groupId>org.springframework.boot</groupId><artifactId>spring-boot-maven-plugin</artifactId></plugin></plugins></build>\n</project>");
    out += &file_block("src/main/java/com/example/Application.java", "java", "package com.example;\n\nimport org.springframework.boot.SpringApplication;\nimport org.springframework.boot.autoconfigure.SpringBootApplication;\n\n@SpringBootApplication\npublic class Application {\n    public static void main(String[] args) {\n        SpringApplication.run(Application.class, args);\n    }\n}");
    out += &file_block("src/main/java/com/example/model/Item.java", "java", "package com.example.model;\n\nimport jakarta.persistence.*;\nimport jakarta.validation.constraints.NotBlank;\n\n@Entity\n@Table(name = \"items\")\npublic class Item {\n    @Id @GeneratedValue(strategy = GenerationType.IDENTITY)\n    private Long id;\n\n    @NotBlank(message = \"Name is required\")\n    private String name;\n\n    private String description;\n    private double price;\n\n    public Item() {}\n    public Item(String name, String description, double price) { this.name = name; this.description = description; this.price = price; }\n\n    public Long getId() { return id; }\n    public String getName() { return name; }\n    public void setName(String name) { this.name = name; }\n    public String getDescription() { return description; }\n    public void setDescription(String d) { this.description = d; }\n    public double getPrice() { return price; }\n    public void setPrice(double p) { this.price = p; }\n}");
    out += &file_block("src/main/java/com/example/repository/ItemRepository.java", "java", "package com.example.repository;\n\nimport com.example.model.Item;\nimport org.springframework.data.jpa.repository.JpaRepository;\n\npublic interface ItemRepository extends JpaRepository<Item, Long> {}");
    out += &file_block("src/main/java/com/example/service/ItemService.java", "java", "package com.example.service;\n\nimport com.example.model.Item;\nimport com.example.repository.ItemRepository;\nimport org.springframework.stereotype.Service;\nimport java.util.List;\n\n@Service\npublic class ItemService {\n    private final ItemRepository repo;\n    public ItemService(ItemRepository repo) { this.repo = repo; }\n\n    public List<Item> findAll() { return repo.findAll(); }\n    public Item findById(Long id) { return repo.findById(id).orElseThrow(() -> new RuntimeException(\"Not found\")); }\n    public Item create(Item item) { return repo.save(item); }\n    public Item update(Long id, Item data) {\n        Item item = findById(id);\n        item.setName(data.getName());\n        item.setDescription(data.getDescription());\n        item.setPrice(data.getPrice());\n        return repo.save(item);\n    }\n    public void delete(Long id) { repo.deleteById(id); }\n}");
    out += &file_block("src/main/java/com/example/controller/ItemController.java", "java", "package com.example.controller;\n\nimport com.example.model.Item;\nimport com.example.service.ItemService;\nimport jakarta.validation.Valid;\nimport org.springframework.http.HttpStatus;\nimport org.springframework.web.bind.annotation.*;\nimport java.util.List;\n\n@RestController\n@RequestMapping(\"/api/items\")\n@CrossOrigin(origins = \"*\")\npublic class ItemController {\n    private final ItemService service;\n    public ItemController(ItemService service) { this.service = service; }\n\n    @GetMapping\n    public List<Item> list() { return service.findAll(); }\n\n    @GetMapping(\"/{id}\")\n    public Item get(@PathVariable Long id) { return service.findById(id); }\n\n    @PostMapping @ResponseStatus(HttpStatus.CREATED)\n    public Item create(@Valid @RequestBody Item item) { return service.create(item); }\n\n    @PutMapping(\"/{id}\")\n    public Item update(@PathVariable Long id, @Valid @RequestBody Item item) { return service.update(id, item); }\n\n    @DeleteMapping(\"/{id}\") @ResponseStatus(HttpStatus.NO_CONTENT)\n    public void delete(@PathVariable Long id) { service.delete(id); }\n}");
    out += &file_block("src/main/resources/application.properties", "properties", "server.port=8080\nspring.datasource.url=jdbc:h2:file:./data/app\nspring.datasource.driver-class-name=org.h2.Driver\nspring.jpa.hibernate.ddl-auto=update\nspring.h2.console.enabled=true");
    out += &file_block("Dockerfile", "dockerfile", "FROM eclipse-temurin:17-jdk-alpine AS build\nWORKDIR /app\nCOPY . .\nRUN ./mvnw package -DskipTests\n\nFROM eclipse-temurin:17-jre-alpine\nWORKDIR /app\nCOPY --from=build /app/target/*.jar app.jar\nEXPOSE 8080\nCMD [\"java\", \"-jar\", \"app.jar\"]");
    out += &file_block(".gitignore", "text", "target/\ndata/\n*.class\n.idea/\n*.iml\n.env");
    out += &project_footer("./mvnw spring-boot:run\n# Open http://localhost:8080/api/items\n# H2 Console: http://localhost:8080/h2-console");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// RUST PROJECT
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_rust_project() -> String {
    let mut out = project_header("Rust", "Rust project with lib, CLI, tests, and CI.",
        "my-rust-project/\n├── Cargo.toml\n├── src/\n│   ├── main.rs\n│   └── lib.rs\n├── tests/\n│   └── integration_test.rs\n├── Dockerfile\n└── .gitignore");
    out += &file_block("Cargo.toml", "toml", "[package]\nname = \"my-rust-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"");
    out += &file_block("src/lib.rs", "rust", "use serde::{Deserialize, Serialize};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct Item {\n    pub id: u64,\n    pub name: String,\n    pub price: f64,\n}\n\nimpl Item {\n    pub fn new(id: u64, name: &str, price: f64) -> Self {\n        Self { id, name: name.to_string(), price }\n    }\n}\n\npub fn filter_by_price(items: &[Item], max_price: f64) -> Vec<&Item> {\n    items.iter().filter(|i| i.price <= max_price).collect()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_filter_by_price() {\n        let items = vec![Item::new(1, \"A\", 10.0), Item::new(2, \"B\", 20.0), Item::new(3, \"C\", 5.0)];\n        let cheap = filter_by_price(&items, 10.0);\n        assert_eq!(cheap.len(), 2);\n    }\n}");
    out += &file_block("src/main.rs", "rust", "use my_rust_project::{Item, filter_by_price};\n\nfn main() {\n    let items = vec![\n        Item::new(1, \"Widget\", 9.99),\n        Item::new(2, \"Gadget\", 24.99),\n        Item::new(3, \"Doohickey\", 4.99),\n    ];\n\n    println!(\"All items:\");\n    for item in &items {\n        println!(\"  {:>2}. {:15} ${:.2}\", item.id, item.name, item.price);\n    }\n\n    let budget = 10.0;\n    let affordable = filter_by_price(&items, budget);\n    println!(\"\\nUnder ${:.2}:\", budget);\n    for item in affordable {\n        println!(\"  - {} (${:.2})\", item.name, item.price);\n    }\n\n    let json = serde_json::to_string_pretty(&items).unwrap();\n    println!(\"\\nJSON:\\n{}\", json);\n}");
    out += &file_block("tests/integration_test.rs", "rust", "use my_rust_project::Item;\n\n#[test]\nfn test_item_creation() {\n    let item = Item::new(1, \"Test\", 42.0);\n    assert_eq!(item.name, \"Test\");\n    assert_eq!(item.price, 42.0);\n}\n\n#[test]\nfn test_serde_roundtrip() {\n    let item = Item::new(1, \"Test\", 9.99);\n    let json = serde_json::to_string(&item).unwrap();\n    let back: Item = serde_json::from_str(&json).unwrap();\n    assert_eq!(back.name, item.name);\n}");
    out += &file_block("Dockerfile", "dockerfile", "FROM rust:1.75-slim AS build\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=build /app/target/release/my-rust-project /usr/local/bin/\nCMD [\"my-rust-project\"]");
    out += &file_block(".gitignore", "text", "target/\n*.swp\n.env");
    out += &project_footer("cargo run\ncargo test");
    out
}

fn scaffold_rust_web_project() -> String {
    let mut out = project_header("Rust Web (Actix)", "Rust web API with Actix-web, Serde, and CORS.",
        "my-rust-api/\n├── Cargo.toml\n├── src/\n│   └── main.rs\n├── Dockerfile\n└── .gitignore");
    out += &file_block("Cargo.toml", "toml", "[package]\nname = \"my-rust-api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nactix-web = \"4\"\nactix-cors = \"0.7\"\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\ntokio = { version = \"1\", features = [\"full\"] }");
    out += &file_block("src/main.rs", "rust", "use actix_web::{web, App, HttpServer, HttpResponse, middleware};\nuse actix_cors::Cors;\nuse serde::{Deserialize, Serialize};\nuse std::sync::Mutex;\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\nstruct Item { id: u64, name: String, price: f64 }\n\n#[derive(Deserialize)]\nstruct CreateItem { name: String, price: f64 }\n\nstruct AppState { items: Mutex<Vec<Item>>, next_id: Mutex<u64> }\n\nasync fn list_items(data: web::Data<AppState>) -> HttpResponse {\n    let items = data.items.lock().unwrap();\n    HttpResponse::Ok().json(&*items)\n}\n\nasync fn create_item(data: web::Data<AppState>, body: web::Json<CreateItem>) -> HttpResponse {\n    let mut items = data.items.lock().unwrap();\n    let mut next_id = data.next_id.lock().unwrap();\n    let item = Item { id: *next_id, name: body.name.clone(), price: body.price };\n    *next_id += 1;\n    items.push(item.clone());\n    HttpResponse::Created().json(item)\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    let state = web::Data::new(AppState {\n        items: Mutex::new(vec![Item { id: 1, name: \"Widget\".into(), price: 9.99 }]),\n        next_id: Mutex::new(2),\n    });\n    println!(\"Server at http://localhost:8080\");\n    HttpServer::new(move || {\n        App::new()\n            .wrap(Cors::permissive())\n            .app_data(state.clone())\n            .route(\"/\", web::get().to(|| async { HttpResponse::Ok().json(serde_json::json!({\"status\": \"ok\"})) }))\n            .route(\"/api/items\", web::get().to(list_items))\n            .route(\"/api/items\", web::post().to(create_item))\n    }).bind(\"0.0.0.0:8080\")?.run().await\n}");
    out += &file_block("Dockerfile", "dockerfile", "FROM rust:1.75-slim AS build\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=build /app/target/release/my-rust-api /usr/local/bin/\nEXPOSE 8080\nCMD [\"my-rust-api\"]");
    out += &file_block(".gitignore", "text", "target/\n.env");
    out += &project_footer("cargo run\n# GET  http://localhost:8080/api/items\n# POST http://localhost:8080/api/items {\"name\": \"Test\", \"price\": 5.99}");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// GO PROJECT
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_go_project() -> String {
    let mut out = project_header("Go", "Go REST API with standard library net/http, JSON, and middleware.",
        "my-go-api/\n├── go.mod\n├── main.go\n├── handler/\n│   └── items.go\n├── Dockerfile\n└── .gitignore");
    out += &file_block("go.mod", "text", "module my-go-api\n\ngo 1.21");
    out += &file_block("main.go", "go", "package main\n\nimport (\n\t\"fmt\"\n\t\"log\"\n\t\"net/http\"\n\t\"my-go-api/handler\"\n)\n\nfunc logger(next http.Handler) http.Handler {\n\treturn http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {\n\t\tlog.Printf(\"%s %s\", r.Method, r.URL.Path)\n\t\tnext.ServeHTTP(w, r)\n\t})\n}\n\nfunc main() {\n\tmux := http.NewServeMux()\n\th := handler.NewItemHandler()\n\n\tmux.HandleFunc(\"GET /api/items\", h.List)\n\tmux.HandleFunc(\"POST /api/items\", h.Create)\n\tmux.HandleFunc(\"GET /\", func(w http.ResponseWriter, r *http.Request) {\n\t\tfmt.Fprintf(w, `{\"status\":\"ok\"}`)\n\t})\n\n\taddr := \":8080\"\n\tlog.Printf(\"Server at http://localhost%s\", addr)\n\tlog.Fatal(http.ListenAndServe(addr, logger(mux)))\n}");
    out += &file_block("handler/items.go", "go", "package handler\n\nimport (\n\t\"encoding/json\"\n\t\"net/http\"\n\t\"sync\"\n)\n\ntype Item struct {\n\tID    int     `json:\"id\"`\n\tName  string  `json:\"name\"`\n\tPrice float64 `json:\"price\"`\n}\n\ntype ItemHandler struct {\n\tmu     sync.Mutex\n\titems  []Item\n\tnextID int\n}\n\nfunc NewItemHandler() *ItemHandler {\n\treturn &ItemHandler{\n\t\titems:  []Item{{ID: 1, Name: \"Widget\", Price: 9.99}},\n\t\tnextID: 2,\n\t}\n}\n\nfunc (h *ItemHandler) List(w http.ResponseWriter, r *http.Request) {\n\th.mu.Lock()\n\tdefer h.mu.Unlock()\n\tw.Header().Set(\"Content-Type\", \"application/json\")\n\tjson.NewEncoder(w).Encode(h.items)\n}\n\nfunc (h *ItemHandler) Create(w http.ResponseWriter, r *http.Request) {\n\tvar input struct {\n\t\tName  string  `json:\"name\"`\n\t\tPrice float64 `json:\"price\"`\n\t}\n\tif err := json.NewDecoder(r.Body).Decode(&input); err != nil {\n\t\thttp.Error(w, `{\"error\":\"invalid json\"}`, 400)\n\t\treturn\n\t}\n\th.mu.Lock()\n\titem := Item{ID: h.nextID, Name: input.Name, Price: input.Price}\n\th.nextID++\n\th.items = append(h.items, item)\n\th.mu.Unlock()\n\tw.Header().Set(\"Content-Type\", \"application/json\")\n\tw.WriteHeader(201)\n\tjson.NewEncoder(w).Encode(item)\n}");
    out += &file_block("Dockerfile", "dockerfile", "FROM golang:1.21-alpine AS build\nWORKDIR /app\nCOPY . .\nRUN go build -o server .\n\nFROM alpine:3.19\nCOPY --from=build /app/server /usr/local/bin/\nEXPOSE 8080\nCMD [\"server\"]");
    out += &file_block(".gitignore", "text", "server\n*.exe\n.env");
    out += &project_footer("go run .\n# GET  http://localhost:8080/api/items\n# POST http://localhost:8080/api/items {\"name\":\"Test\",\"price\":5.99}");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// PYTHON PACKAGE
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_python_package() -> String {
    let mut out = project_header("Python Package", "Python package with pyproject.toml, tests, and CLI entry point.",
        "my-python-pkg/\n├── pyproject.toml\n├── README.md\n├── src/\n│   └── mypkg/\n│       ├── __init__.py\n│       ├── core.py\n│       └── cli.py\n├── tests/\n│   ├── __init__.py\n│   └── test_core.py\n└── .gitignore");
    out += &file_block("pyproject.toml", "toml", "[build-system]\nrequires = [\"setuptools>=68.0\", \"wheel\"]\nbuild-backend = \"setuptools.build_meta\"\n\n[project]\nname = \"mypkg\"\nversion = \"0.1.0\"\ndescription = \"A Python package scaffolded by Kala AI\"\nrequires-python = \">=3.9\"\n\n[project.scripts]\nmypkg = \"mypkg.cli:main\"\n\n[tool.setuptools.packages.find]\nwhere = [\"src\"]");
    out += &file_block("src/mypkg/__init__.py", "python", "from .core import greet, add, fibonacci\n\n__version__ = '0.1.0'");
    out += &file_block("src/mypkg/core.py", "python", "def greet(name: str) -> str:\n    return f'Hello, {name}!'\n\ndef add(a: float, b: float) -> float:\n    return a + b\n\ndef fibonacci(n: int) -> list[int]:\n    if n <= 0: return []\n    if n == 1: return [0]\n    fibs = [0, 1]\n    while len(fibs) < n:\n        fibs.append(fibs[-1] + fibs[-2])\n    return fibs");
    out += &file_block("src/mypkg/cli.py", "python", "import argparse\nfrom . import greet, fibonacci\n\ndef main():\n    parser = argparse.ArgumentParser(description='mypkg CLI')\n    sub = parser.add_subparsers(dest='command')\n\n    hello = sub.add_parser('hello')\n    hello.add_argument('name', help='Your name')\n\n    fib = sub.add_parser('fib')\n    fib.add_argument('n', type=int, help='How many numbers')\n\n    args = parser.parse_args()\n    if args.command == 'hello': print(greet(args.name))\n    elif args.command == 'fib': print(fibonacci(args.n))\n    else: parser.print_help()\n\nif __name__ == '__main__': main()");
    out += &file_block("tests/test_core.py", "python", "from mypkg import greet, add, fibonacci\n\ndef test_greet():\n    assert greet('Kala') == 'Hello, Kala!'\n\ndef test_add():\n    assert add(2, 3) == 5\n    assert add(-1, 1) == 0\n\ndef test_fibonacci():\n    assert fibonacci(0) == []\n    assert fibonacci(1) == [0]\n    assert fibonacci(7) == [0, 1, 1, 2, 3, 5, 8]");
    out += &file_block(".gitignore", "text", "dist/\n*.egg-info/\n__pycache__/\n*.pyc\n.env\n.venv/");
    out += &project_footer("pip install -e .\nmypkg hello World\nmypkg fib 10\npytest tests/");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// NODE CLI TOOL
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_node_cli() -> String {
    let mut out = project_header("Node.js CLI", "Node.js command-line tool with subcommands and colorful output.",
        "my-cli-tool/\n├── package.json\n├── bin/\n│   └── cli.js\n├── src/\n│   ├── commands/\n│   │   ├── greet.js\n│   │   └── calc.js\n│   └── utils.js\n└── .gitignore");
    out += &file_block("package.json", "json", "{\n  \"name\": \"my-cli-tool\",\n  \"version\": \"1.0.0\",\n  \"bin\": { \"mycli\": \"./bin/cli.js\" },\n  \"type\": \"module\"\n}");
    out += &file_block("bin/cli.js", "javascript", "#!/usr/bin/env node\nimport { greet } from '../src/commands/greet.js';\nimport { calc } from '../src/commands/calc.js';\n\nconst [cmd, ...args] = process.argv.slice(2);\n\nswitch (cmd) {\n  case 'greet': greet(args[0] || 'World'); break;\n  case 'calc':  calc(args); break;\n  default:\n    console.log('Usage: mycli <command> [args]');\n    console.log('  greet <name>    Say hello');\n    console.log('  calc <expr>     Calculate (e.g. \"2 + 3\")');\n}");
    out += &file_block("src/commands/greet.js", "javascript", "import { bold, cyan } from '../utils.js';\n\nexport function greet(name) {\n  console.log(`${cyan('Hello')}, ${bold(name)}!`);\n}");
    out += &file_block("src/commands/calc.js", "javascript", "import { bold } from '../utils.js';\n\nexport function calc(args) {\n  const expr = args.join(' ');\n  try {\n    const result = Function(`\"use strict\"; return (${expr})`)();\n    console.log(`${expr} = ${bold(String(result))}`);\n  } catch {\n    console.error('Invalid expression:', expr);\n  }\n}");
    out += &file_block("src/utils.js", "javascript", "export const bold = (s) => `\\x1b[1m${s}\\x1b[0m`;\nexport const cyan = (s) => `\\x1b[36m${s}\\x1b[0m`;\nexport const green = (s) => `\\x1b[32m${s}\\x1b[0m`;\nexport const red = (s) => `\\x1b[31m${s}\\x1b[0m`;");
    out += &file_block(".gitignore", "text", "node_modules/\n.env");
    out += &project_footer("npm link   # install globally\nmycli greet Kala\nmycli calc \"2 * 21\"");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML/CSS/JS WEBSITE
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_html_website() -> String {
    let mut out = project_header("HTML/CSS/JS", "Static website with modern CSS, responsive design, and vanilla JS.",
        "my-website/\n├── index.html\n├── about.html\n├── css/\n│   └── style.css\n├── js/\n│   └── app.js\n└── .gitignore");
    out += &file_block("index.html", "html", "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\"/>\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"/>\n  <title>My Website</title>\n  <link rel=\"stylesheet\" href=\"css/style.css\"/>\n</head>\n<body>\n  <nav>\n    <a href=\"index.html\" class=\"brand\">MyWebsite</a>\n    <a href=\"index.html\">Home</a>\n    <a href=\"about.html\">About</a>\n  </nav>\n\n  <main class=\"container\">\n    <section class=\"hero\">\n      <h1>Welcome to My Website</h1>\n      <p>A beautiful static site built with HTML, CSS, and JavaScript.</p>\n    </section>\n\n    <section class=\"card\">\n      <h2>Counter Demo</h2>\n      <p>Count: <span id=\"count\">0</span></p>\n      <button onclick=\"increment()\">+1</button>\n      <button onclick=\"reset()\">Reset</button>\n    </section>\n\n    <section class=\"card\">\n      <h2>Features</h2>\n      <ul>\n        <li>Responsive design</li>\n        <li>Modern CSS with variables</li>\n        <li>No framework needed</li>\n      </ul>\n    </section>\n  </main>\n\n  <footer>&copy; 2025 Built with Kala AI</footer>\n  <script src=\"js/app.js\"></script>\n</body>\n</html>");
    out += &file_block("about.html", "html", "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\"/><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n  <title>About — My Website</title>\n  <link rel=\"stylesheet\" href=\"css/style.css\"/>\n</head>\n<body>\n  <nav><a href=\"index.html\" class=\"brand\">MyWebsite</a><a href=\"index.html\">Home</a><a href=\"about.html\">About</a></nav>\n  <main class=\"container\">\n    <h1>About</h1>\n    <div class=\"card\"><p>Scaffolded by <strong>Kala AI</strong> — the AI in the Killer programming language.</p></div>\n  </main>\n  <footer>&copy; 2025 Built with Kala AI</footer>\n</body>\n</html>");
    out += &file_block("css/style.css", "css", ":root { --primary: #00d4ff; --bg: #f5f5f5; --dark: #1a1a2e; --card: #ffffff; --radius: 12px; }\n* { margin: 0; padding: 0; box-sizing: border-box; }\nbody { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: var(--bg); color: #333; }\nnav { background: var(--dark); padding: 1rem 2rem; display: flex; gap: 1.5rem; align-items: center; }\nnav a { color: #e0e0e0; text-decoration: none; } nav a:hover { color: var(--primary); }\nnav .brand { font-size: 1.4rem; font-weight: 700; color: var(--primary); }\n.container { max-width: 960px; margin: 0 auto; padding: 2rem; }\n.hero { text-align: center; padding: 3rem 0; }\n.hero h1 { font-size: 2.5rem; margin-bottom: .5rem; color: var(--dark); }\n.card { background: var(--card); border-radius: var(--radius); padding: 1.5rem; margin: 1.5rem 0; box-shadow: 0 2px 8px rgba(0,0,0,.08); }\nbutton { padding: .5rem 1.2rem; border: none; border-radius: 6px; cursor: pointer; background: var(--primary); color: #fff; font-size: 1rem; margin: .25rem; }\nbutton:hover { opacity: .85; }\nul { padding-left: 1.5rem; }\nli { margin: .3rem 0; }\nfooter { background: var(--dark); color: #aaa; text-align: center; padding: 1rem; margin-top: 2rem; }\n@media (max-width: 600px) { .hero h1 { font-size: 1.8rem; } .container { padding: 1rem; } }");
    out += &file_block("js/app.js", "javascript", "let count = 0;\nconst el = document.getElementById('count');\n\nfunction increment() { count++; el.textContent = count; }\nfunction reset() { count = 0; el.textContent = count; }");
    out += &file_block(".gitignore", "text", ".DS_Store\n.env");
    out += &project_footer("# Just open index.html in your browser!\n# Or use a local server:\npython -m http.server 8000\n# Open http://localhost:8000");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// DOCKER COMPOSE
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_docker_compose() -> String {
    let mut out = project_header("Docker Compose", "Multi-service setup with app, database, and reverse proxy.",
        "my-docker-project/\n├── docker-compose.yml\n├── app/\n│   ├── Dockerfile\n│   ├── requirements.txt\n│   └── main.py\n├── nginx/\n│   └── nginx.conf\n├── .env.example\n└── .gitignore");
    out += &file_block("docker-compose.yml", "yaml", "version: '3.9'\n\nservices:\n  app:\n    build: ./app\n    ports:\n      - '${APP_PORT:-8000}:8000'\n    environment:\n      - DATABASE_URL=postgresql://user:pass@db:5432/mydb\n    depends_on:\n      db:\n        condition: service_healthy\n    restart: unless-stopped\n\n  db:\n    image: postgres:16-alpine\n    environment:\n      POSTGRES_USER: user\n      POSTGRES_PASSWORD: pass\n      POSTGRES_DB: mydb\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n    healthcheck:\n      test: ['CMD-SHELL', 'pg_isready -U user -d mydb']\n      interval: 5s\n      retries: 5\n    restart: unless-stopped\n\n  nginx:\n    image: nginx:alpine\n    ports:\n      - '80:80'\n    volumes:\n      - ./nginx/nginx.conf:/etc/nginx/conf.d/default.conf\n    depends_on:\n      - app\n    restart: unless-stopped\n\nvolumes:\n  pgdata:");
    out += &file_block("app/requirements.txt", "text", "fastapi>=0.109.0\nuvicorn[standard]>=0.27.0\npsycopg2-binary>=2.9.0\nsqlalchemy>=2.0.0");
    out += &file_block("app/main.py", "python", "from fastapi import FastAPI\nimport os\n\napp = FastAPI(title='My Docker App')\nDB_URL = os.getenv('DATABASE_URL', 'sqlite:///./app.db')\n\n@app.get('/')\ndef root(): return {'status': 'ok', 'database': DB_URL.split('@')[-1] if '@' in DB_URL else 'sqlite'}\n\n@app.get('/health')\ndef health(): return {'healthy': True}");
    out += &file_block("app/Dockerfile", "dockerfile", "FROM python:3.12-slim\nWORKDIR /app\nCOPY requirements.txt .\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY . .\nEXPOSE 8000\nCMD [\"uvicorn\", \"main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8000\"]");
    out += &file_block("nginx/nginx.conf", "nginx", "upstream app {\n    server app:8000;\n}\n\nserver {\n    listen 80;\n    server_name localhost;\n\n    location / {\n        proxy_pass http://app;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n    }\n}");
    out += &file_block(".env.example", "text", "APP_PORT=8000\nPOSTGRES_PASSWORD=change_me");
    out += &file_block(".gitignore", "text", ".env\n*.pyc\n__pycache__/");
    out += &project_footer("cp .env.example .env\ndocker compose up -d\n# App:  http://localhost:8000\n# Nginx: http://localhost");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// REACT + EXPRESS FULL STACK
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_react_express_fullstack() -> String {
    let mut out = project_header("React + Express Full Stack", "Full-stack app: React frontend + Express REST API backend.",
        "my-fullstack-app/\n├── package.json\n├── backend/\n│   ├── package.json\n│   ├── server.js\n│   └── routes/\n│       └── items.js\n├── frontend/\n│   ├── package.json\n│   ├── public/\n│   │   └── index.html\n│   └── src/\n│       ├── index.jsx\n│       ├── App.jsx\n│       └── App.css\n├── docker-compose.yml\n└── .gitignore");
    out += &file_block("package.json", "json", "{\n  \"name\": \"my-fullstack-app\",\n  \"private\": true,\n  \"scripts\": {\n    \"dev:backend\": \"cd backend && npm run dev\",\n    \"dev:frontend\": \"cd frontend && npm start\",\n    \"dev\": \"npm run dev:backend & npm run dev:frontend\",\n    \"install:all\": \"cd backend && npm install && cd ../frontend && npm install\"\n  }\n}");
    out += &file_block("backend/package.json", "json", "{\n  \"name\": \"backend\",\n  \"version\": \"1.0.0\",\n  \"scripts\": { \"start\": \"node server.js\", \"dev\": \"node --watch server.js\" },\n  \"dependencies\": { \"express\": \"^4.18.0\", \"cors\": \"^2.8.5\" }\n}");
    out += &file_block("backend/server.js", "javascript", "const express = require('express');\nconst cors = require('cors');\nconst itemsRouter = require('./routes/items');\n\nconst app = express();\napp.use(cors());\napp.use(express.json());\napp.use('/api/items', itemsRouter);\napp.get('/api/health', (_, res) => res.json({ status: 'ok' }));\n\nconst PORT = process.env.PORT || 5000;\napp.listen(PORT, () => console.log(`Backend at http://localhost:${PORT}`));");
    out += &file_block("backend/routes/items.js", "javascript", "const express = require('express');\nconst router = express.Router();\nlet items = [{ id: 1, name: 'Item 1', done: false }, { id: 2, name: 'Item 2', done: false }];\nlet nextId = 3;\n\nrouter.get('/', (_, res) => res.json(items));\nrouter.post('/', (req, res) => {\n  const item = { id: nextId++, name: req.body.name, done: false };\n  items.push(item);\n  res.status(201).json(item);\n});\nrouter.patch('/:id', (req, res) => {\n  const item = items.find(i => i.id === +req.params.id);\n  if (!item) return res.status(404).json({ error: 'Not found' });\n  Object.assign(item, req.body);\n  res.json(item);\n});\nrouter.delete('/:id', (req, res) => {\n  items = items.filter(i => i.id !== +req.params.id);\n  res.status(204).end();\n});\nmodule.exports = router;");
    out += &file_block("frontend/package.json", "json", "{\n  \"name\": \"frontend\",\n  \"version\": \"1.0.0\",\n  \"private\": true,\n  \"proxy\": \"http://localhost:5000\",\n  \"dependencies\": { \"react\": \"^18.2.0\", \"react-dom\": \"^18.2.0\", \"react-scripts\": \"5.0.1\" },\n  \"scripts\": { \"start\": \"react-scripts start\", \"build\": \"react-scripts build\" }\n}");
    out += &file_block("frontend/public/index.html", "html", "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"UTF-8\"/><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/><title>My App</title></head>\n<body><div id=\"root\"></div></body>\n</html>");
    out += &file_block("frontend/src/index.jsx", "jsx", "import React from 'react';\nimport ReactDOM from 'react-dom/client';\nimport App from './App';\nimport './App.css';\n\nReactDOM.createRoot(document.getElementById('root')).render(<App />);");
    out += &file_block("frontend/src/App.jsx", "jsx", "import { useState, useEffect } from 'react';\n\nconst API = '/api/items';\n\nexport default function App() {\n  const [items, setItems] = useState([]);\n  const [text, setText] = useState('');\n\n  useEffect(() => { fetch(API).then(r => r.json()).then(setItems); }, []);\n\n  const addItem = async () => {\n    if (!text.trim()) return;\n    const res = await fetch(API, { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ name: text }) });\n    const item = await res.json();\n    setItems([...items, item]);\n    setText('');\n  };\n\n  const toggle = async (id) => {\n    const item = items.find(i => i.id === id);\n    const res = await fetch(`${API}/${id}`, { method: 'PATCH', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ done: !item.done }) });\n    const updated = await res.json();\n    setItems(items.map(i => i.id === id ? updated : i));\n  };\n\n  const remove = async (id) => {\n    await fetch(`${API}/${id}`, { method: 'DELETE' });\n    setItems(items.filter(i => i.id !== id));\n  };\n\n  return (\n    <div className=\"app\">\n      <nav><span className=\"brand\">My Full Stack App</span></nav>\n      <main className=\"container\">\n        <h1>Items</h1>\n        <div className=\"add-form\">\n          <input value={text} onChange={e => setText(e.target.value)} placeholder=\"New item...\" onKeyDown={e => e.key === 'Enter' && addItem()} />\n          <button onClick={addItem}>Add</button>\n        </div>\n        <ul>{items.map(item => (\n          <li key={item.id} className={item.done ? 'done' : ''}>\n            <span onClick={() => toggle(item.id)}>{item.name}</span>\n            <button className=\"del\" onClick={() => remove(item.id)}>×</button>\n          </li>\n        ))}</ul>\n      </main>\n    </div>\n  );\n}");
    out += &file_block("frontend/src/App.css", "css", "* { margin: 0; padding: 0; box-sizing: border-box; }\nbody { font-family: sans-serif; background: #f5f5f5; }\nnav { background: #1a1a2e; padding: 1rem 2rem; } .brand { color: #00d4ff; font-size: 1.4rem; font-weight: 700; }\n.container { max-width: 640px; margin: 2rem auto; padding: 0 1rem; }\n.add-form { display: flex; gap: .5rem; margin: 1rem 0; }\ninput { flex: 1; padding: .6rem; border: 1px solid #ddd; border-radius: 8px; font-size: 1rem; }\nbutton { padding: .6rem 1.2rem; background: #00d4ff; color: #fff; border: none; border-radius: 8px; cursor: pointer; }\nul { list-style: none; } li { background: #fff; padding: 1rem; margin: .5rem 0; border-radius: 8px; display: flex; justify-content: space-between; box-shadow: 0 1px 4px rgba(0,0,0,.06); cursor: pointer; }\nli.done span { text-decoration: line-through; color: #999; }\n.del { background: #ff4757; padding: .3rem .6rem; font-size: 1.1rem; }");
    out += &file_block("docker-compose.yml", "yaml", "version: '3.9'\nservices:\n  backend:\n    build: ./backend\n    ports: ['5000:5000']\n  frontend:\n    build: ./frontend\n    ports: ['3000:3000']\n    depends_on: [backend]");
    out += &file_block(".gitignore", "text", "node_modules/\nbuild/\n.env\n.DS_Store");
    out += &project_footer("npm run install:all\nnpm run dev\n# Frontend: http://localhost:3000\n# Backend:  http://localhost:5000/api/items");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// DJANGO + REACT FULL STACK
// ─────────────────────────────────────────────────────────────────────────────
fn scaffold_django_react_fullstack() -> String {
    let mut out = project_header("Django + React Full Stack", "Django REST API backend + React frontend.",
        "my-django-react/\n├── backend/\n│   ├── requirements.txt\n│   ├── manage.py\n│   ├── config/\n│   │   ├── settings.py\n│   │   └── urls.py\n│   └── api/\n│       ├── models.py\n│       ├── serializers.py\n│       ├── views.py\n│       └── urls.py\n├── frontend/\n│   ├── package.json\n│   └── src/\n│       ├── App.jsx\n│       └── index.jsx\n├── docker-compose.yml\n└── .gitignore");
    out += &file_block("backend/requirements.txt", "text", "django>=5.0\ndjango-rest-framework>=3.14\ndjango-cors-headers>=4.3");
    out += &file_block("backend/config/settings.py", "python", "from pathlib import Path\nBASE_DIR = Path(__file__).resolve().parent.parent\nSECRET_KEY = 'change-me'\nDEBUG = True\nALLOWED_HOSTS = ['*']\nINSTALLED_APPS = ['django.contrib.admin','django.contrib.auth','django.contrib.contenttypes','django.contrib.sessions','django.contrib.messages','django.contrib.staticfiles','rest_framework','corsheaders','api']\nMIDDLEWARE = ['corsheaders.middleware.CorsMiddleware','django.middleware.common.CommonMiddleware','django.middleware.csrf.CsrfViewMiddleware','django.contrib.sessions.middleware.SessionMiddleware','django.contrib.auth.middleware.AuthenticationMiddleware','django.contrib.messages.middleware.MessageMiddleware']\nCORS_ALLOW_ALL_ORIGINS = True\nROOT_URLCONF = 'config.urls'\nDATABASES = {'default': {'ENGINE': 'django.db.backends.sqlite3', 'NAME': BASE_DIR / 'db.sqlite3'}}\nSTATIC_URL = '/static/'\nREST_FRAMEWORK = {'DEFAULT_PAGINATION_CLASS': 'rest_framework.pagination.PageNumberPagination', 'PAGE_SIZE': 20}\nTEMPLATES = [{'BACKEND':'django.template.backends.django.DjangoTemplates','DIRS':[],'APP_DIRS':True,'OPTIONS':{'context_processors':['django.template.context_processors.request','django.contrib.auth.context_processors.auth','django.contrib.messages.context_processors.messages']}}]");
    out += &file_block("backend/config/urls.py", "python", "from django.contrib import admin\nfrom django.urls import path, include\nurlpatterns = [path('admin/', admin.site.urls), path('api/', include('api.urls'))]");
    out += &file_block("backend/api/models.py", "python", "from django.db import models\n\nclass Item(models.Model):\n    name = models.CharField(max_length=200)\n    done = models.BooleanField(default=False)\n    created_at = models.DateTimeField(auto_now_add=True)\n    def __str__(self): return self.name");
    out += &file_block("backend/api/serializers.py", "python", "from rest_framework import serializers\nfrom .models import Item\n\nclass ItemSerializer(serializers.ModelSerializer):\n    class Meta:\n        model = Item\n        fields = '__all__'");
    out += &file_block("backend/api/views.py", "python", "from rest_framework import viewsets\nfrom .models import Item\nfrom .serializers import ItemSerializer\n\nclass ItemViewSet(viewsets.ModelViewSet):\n    queryset = Item.objects.all().order_by('-created_at')\n    serializer_class = ItemSerializer");
    out += &file_block("backend/api/urls.py", "python", "from rest_framework.routers import DefaultRouter\nfrom .views import ItemViewSet\n\nrouter = DefaultRouter()\nrouter.register('items', ItemViewSet)\nurlpatterns = router.urls");
    out += &file_block("frontend/package.json", "json", "{\n  \"name\": \"frontend\",\n  \"proxy\": \"http://localhost:8000\",\n  \"dependencies\": { \"react\": \"^18.2.0\", \"react-dom\": \"^18.2.0\", \"react-scripts\": \"5.0.1\" },\n  \"scripts\": { \"start\": \"react-scripts start\", \"build\": \"react-scripts build\" }\n}");
    out += &file_block("frontend/src/App.jsx", "jsx", "import { useState, useEffect } from 'react';\n\nexport default function App() {\n  const [items, setItems] = useState([]);\n  const [text, setText] = useState('');\n\n  useEffect(() => { fetch('/api/items/').then(r => r.json()).then(d => setItems(d.results || d)); }, []);\n\n  const add = async () => {\n    if (!text) return;\n    const r = await fetch('/api/items/', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({name: text}) });\n    setItems([await r.json(), ...items]); setText('');\n  };\n\n  return (\n    <div style={{maxWidth:600,margin:'2rem auto',fontFamily:'sans-serif'}}>\n      <h1>Django + React</h1>\n      <div style={{display:'flex',gap:8,margin:'1rem 0'}}>\n        <input value={text} onChange={e=>setText(e.target.value)} placeholder=\"New item\" style={{flex:1,padding:8,borderRadius:8,border:'1px solid #ddd'}} onKeyDown={e=>e.key==='Enter'&&add()}/>\n        <button onClick={add} style={{padding:'8px 16px',background:'#00d4ff',color:'#fff',border:'none',borderRadius:8}}>Add</button>\n      </div>\n      {items.map(i => <div key={i.id} style={{background:'#fff',padding:12,margin:8,borderRadius:8,boxShadow:'0 1px 4px rgba(0,0,0,.06)'}}>{i.name}</div>)}\n    </div>\n  );\n}");
    out += &file_block(".gitignore", "text", "node_modules/\ndb.sqlite3\n*.pyc\n__pycache__/\nbuild/\n.env");
    out += &project_footer("# Backend:\ncd backend && pip install -r requirements.txt\npython manage.py migrate && python manage.py runserver\n\n# Frontend (new terminal):\ncd frontend && npm install && npm start");
    out
}

/// Deep reasoning version of khlm_ask_expert — uses thinking-oriented prompt.
pub fn khlm_think_expert(question: &str) -> String {
    let cfg_opt = {
        let lock = config().lock().unwrap();
        if lock.llm_available() { Some(lock.clone()) } else { None }
    };
    if let Some(cfg) = cfg_opt {
        // Multi-turn history + upgraded 7-step reasoning protocol
        let messages = build_messages(KALA_THINK_SYSTEM_PROMPT, question);
        if let Some(out) = llm_call_messages(&messages, &cfg) {
            return out;
        }
    }
    // Offline: native reasoning engine
    String::new() // caller falls back to native_think
}

const KALA_IMAGINE_SYSTEM_PROMPT: &str = "\
You are Kala (काल), a visionary AI built into the Killer programming language.\n\
\n\
## Imagine Mode — Principled Creative Exploration\n\
\n\
### Non-Negotiable Rules\n\
1. **Ground all speculation** in real science, history, logic, or well-reasoned extrapolation.\n\
2. **Be specific** — vague imagination is worthless. Name technologies, timelines, mechanisms.\n\
3. **Explore consequences** — go beyond the obvious to 2nd and 3rd order effects.\n\
4. **Acknowledge tensions** — every bold idea has trade-offs and risks; address them.\n\
5. **No filler** — every sentence must add genuine insight.\n\
6. **Session memory** — if the user continues a thread from earlier in this chat, extend that thread instead of restarting cold.\n\
\n\
### Response Format\n\
## The Vision\n\
## Scientific / Logical Basis\n\
## Who Benefits — And Who Doesn't\n\
## Ripple Effects (1st → 2nd → 3rd order)\n\
## The Key Insight\n\
## Open Question Worth Exploring\n\
\n\
Use storytelling, metaphors, and thought experiments where they illuminate — not where they decorate.";

/// Imagine/what-if mode — visionary, grounded speculation with multi-turn context.
pub fn khlm_imagine_expert(question: &str) -> String {
    let cfg_opt = {
        let lock = config().lock().unwrap();
        if lock.llm_available() { Some(lock.clone()) } else { None }
    };
    if let Some(cfg) = cfg_opt {
        let messages = build_messages(KALA_IMAGINE_SYSTEM_PROMPT, question);
        if let Some(out) = llm_call_messages(&messages, &cfg) {
            return out;
        }
    }
    String::new() // caller falls back to native imagine
}

/// Internal: call LLM with pre-built messages vector.
fn llm_call_messages(messages: &[crate::llm::LlmMessage], cfg: &KhLmPolyglotConfig) -> Option<String> {
    let mut llm_cfg = match cfg.llm_provider.to_lowercase().as_str() {
        "ollama"    => crate::llm::LlmConfig::ollama(&cfg.llm_model),
        "groq"      => crate::llm::LlmConfig::groq(&cfg.llm_api_key, &cfg.llm_model),
        "openai"    => crate::llm::LlmConfig::openai(&cfg.llm_api_key, &cfg.llm_model),
        "anthropic" => crate::llm::LlmConfig::anthropic(&cfg.llm_api_key, &cfg.llm_model),
        _ => return None,
    };
    llm_cfg.temperature = cfg.llm_temperature;
    match crate::llm::complete(&llm_cfg, messages) {
        Ok(r) if !r.content.trim().is_empty() => Some(r.content),
        _ => None,
    }
}

// ── AI Lab ────────────────────────────────────────────────────────────────

/// AI Lab — native Rust demos (math/ML/DL/RL/NLP/LLM primitives) + honest curriculum topics.
/// AGI / ASI / “AI OS” are explained as **research & roadmap**, not shipped product capabilities.
/// Routes to specialized handlers; LLM tier optional when configured.
pub fn khlm_ai_lab(question: &str) -> String {
    let q = question.trim().to_lowercase();
    let llm_ans = ai_lab_llm(question);
    if !llm_ans.is_empty() { return llm_ans; }

    // ── AI topic routing (existing) ─────────────────────────────────────
    if q.contains("multi-agent") || q.contains("multi agent") || q.contains("agent team")
    || q.contains("swarm") || q.contains("consensus") { return lab_multi_agent(); }
    if q.contains("agent") || (q.contains("memory") && q.contains("agent"))
    || q.contains("tool call") || q.contains("autonomous") { return lab_agents(); }
    if q.contains("agi") || q.contains("artificial general") { return lab_agi(); }
    if q.contains("asi") || q.contains("artificial super") || q.contains("superintelligen") { return lab_asi(); }
    if q.contains("ai os") || (q.contains("operating system") && q.contains("ai")) { return lab_aios(); }
    if q.contains("gan") || q.contains("generative adversarial") || q.contains("diffusion")
    || q.contains("vae") || q.contains("generative ai") { return lab_genai(); }
    if q.contains("llm") || q.contains("large language") || q.contains("transformer")
    || q.contains("attention") || q.contains("gpt") || q.contains("bert") { return lab_llm(); }
    if q.contains("nlp") || q.contains("natural language") || q.contains("tokeniz")
    || q.contains("tfidf") || q.contains("word2vec") || q.contains("embedding") { return lab_nlp(); }
    if q.starts_with("rl ") || q.ends_with(" rl") || q.contains(" rl ")
    || q.contains("reinforcement") || q.contains("q-learning") || q.contains("dqn")
    || q.contains("reward") || q.contains("bellman") { return lab_rl(); }
    if q.starts_with("dl ") || q.ends_with(" dl") || q.contains("deep learning")
    || q.contains("neural net") || q.contains("lstm") || q.contains("gru")
    || q.contains("backprop") { return lab_dl(); }
    if q.starts_with("ml ") || q.ends_with(" ml") || q.contains("machine learning")
    || q.contains("linear regression") || q.contains("decision tree")
    || q.contains("random forest") || q.contains("k-means") || q.contains("gradient boost")
    || q.contains("logistic") || q.contains("knn") || q.contains("pca")
    || q.contains("dbscan") || q.contains("clustering") { return lab_ml(); }
    if q.contains("math") || q.contains("gradient") || q.contains("calculus")
    || q.contains("statistic") || q.contains("derivative") || q.contains("linear algebra") { return lab_math(); }
    if q.contains("programming") || q.contains("algorithm") || q.contains("data structure")
    || q.contains("complexity") { return lab_programming(); }
    if q.contains("ai system") || q.contains("pipeline") || q.contains("mlops") { return lab_ai_systems(); }

    // ── Smart fallback: detect intent before showing overview ────────────

    // Explicit overview requests — show overview when asked "what is AI lab" etc.
    let wants_overview = q.contains("what is ai lab") || q.contains("what is the ai lab")
        || q.contains("ai lab capabilities") || q.contains("show capabilities")
        || q.contains("what can you do") || q.contains("show me everything")
        || q.contains("lab overview") || q.contains("what's available")
        || q.contains("whats available") || q.contains("list features")
        || q.contains("what features") || q == "ai lab" || q == "lab"
        || q == "ai" || q == "help";
    if wants_overview { return lab_overview(); }

    // Code generation requests → route to code engine
    let is_code_req = q.contains("write code") || q.contains("write a code")
        || q.contains("code for me") || q.contains("write me")
        || q.contains("write a function") || q.contains("write a program")
        || q.contains("generate code") || q.contains("create code")
        || q.starts_with("code ") || q.starts_with("write ")
        || q.starts_with("implement ");
    if is_code_req {
        return format!(
            "I'd love to help you write code! 💻\n\n\
             For the best code generation experience, **switch to Code mode** (📝 in the sidebar).\n\n\
             But I can also help right here — just tell me specifically what you need:\n\
             - *\"Write a sorting algorithm in Python\"*\n\
             - *\"Create a REST API in Rust\"*\n\
             - *\"Build a linked list in Killer\"*\n\n\
             **What would you like me to code?**"
        );
    }

    // Image/video/audio generation requests → tell user to switch mode or be specific
    let is_gen_req = q.contains("generate image") || q.contains("create image")
        || q.contains("make image") || q.contains("image of")
        || q.contains("generate video") || q.contains("generate audio")
        || q.contains("generate music") || q.contains("draw ");
    if is_gen_req {
        return format!(
            "I can generate that for you! 🎨\n\n\
             For media generation, try **Ask mode** or **Imagine mode** — they have the full generation pipeline.\n\n\
             Just say exactly what you want:\n\
             - *\"Generate image of a sunset over mountains\"*\n\
             - *\"Create video of ocean waves\"*\n\
             - *\"Generate music ambient beat\"*\n\n\
             **Switch to Ask mode and tell me what to generate!**"
        );
    }

    // Vague "help me with AI" requests → guide them interactively
    let is_vague_help = q.contains("help me") || q.contains("can you help")
        || q.contains("help with") || q.contains("ai solution")
        || q.contains("ai help") || (q.contains("help") && q.split_whitespace().count() <= 8);
    if is_vague_help {
        return format!(
            "Of course I can help! 🤝 What kind of AI work are you interested in?\n\n\
             Here's what the **AI Lab** specializes in — pick one and I'll dive deep:\n\n\
             🧮 **Math & Statistics** — *\"show me gradient descent\"*\n\
             📊 **Machine Learning** — *\"explain decision trees\"*\n\
             🧠 **Deep Learning** — *\"how do neural networks work\"*\n\
             🗣 **NLP** — *\"tokenize this sentence\"*\n\
             🎮 **Reinforcement Learning** — *\"explain Q-learning\"*\n\
             🤖 **LLM & Transformers** — *\"how does attention work\"*\n\
             🎨 **Generative AI** — *\"explain diffusion models\"*\n\
             👥 **AI Agents** — *\"build a research agent\"*\n\n\
             Or just ask me any AI question in plain English — I'll figure out where to route it."
        );
    }

    // General question — try expert ask (web + LLM) instead of static table
    let wc = q.split_whitespace().count();
    let is_question = q.starts_with("what ") || q.starts_with("who ") || q.starts_with("where ")
        || q.starts_with("when ") || q.starts_with("why ") || q.starts_with("how ")
        || q.starts_with("is ") || q.starts_with("are ") || q.starts_with("do ")
        || q.starts_with("does ") || q.starts_with("can ") || q.starts_with("could ")
        || q.starts_with("explain ") || q.starts_with("describe ")
        || q.starts_with("tell me");
    if is_question && wc >= 3 {
        return crate::builtin::BuiltinFunctions::kala_expert_ask(question);
    }

    // Fallback: short or ambiguous — prompt user interactively (NOT static table)
    format!(
        "I'm here in **AI Lab** mode — ready to explore AI with you! 🔬\n\n\
         I didn't catch a specific AI topic in your message. Could you be more specific?\n\n\
         Try something like:\n\
         - *\"Explain neural networks\"*\n\
         - *\"Show me machine learning\"*\n\
         - *\"How does attention work in transformers?\"*\n\
         - *\"Run a reinforcement learning demo\"*\n\n\
         Or if you need something outside AI research, try **Ask mode** for general questions \
         or **Code mode** for programming."
    )
}

const KALA_AI_LAB_SYSTEM_PROMPT: &str = "\
You are helping users learn AI inside the Killer programming language runtime. \
**Actually native in Killer (Rust):** educational/primitive stacks in `ml_module` — stats & numeric gradients, \
classical ML (e.g. linear regression, k-means, trees/ensembles where implemented), small NN building blocks \
(dense layers, activations, optimizers), tabular RL (Q-learning and related types), NLP utilities (tokenization, TF-IDF, etc.), \
transformer-style **components** for learning (not a full frontier LLM trainer), separate **GGUF inference** for running \
downloaded models, KhLM routing (deterministic + web agents + optional local model), agent/team demos, Guardian safety hooks, \
and Kala UI modes. \
**Not native as full product subsystems:** AGI, ASI, a complete “AI OS” kernel, or PyTorch-scale distributed training — treat those as \
concepts, curriculum, or roadmap when discussed; never imply they are finished shipped systems inside Killer. \
Say clearly when something needs an external/configured LLM API vs runs fully offline. \
Use formulas and short illustrative pseudo-code. Prior conversation turns are real — stay consistent.\n\n\
Earlier messages in this session are real prior turns — continue threads, reuse definitions the user gave, and answer follow-ups in context.";

fn ai_lab_llm(question: &str) -> String {
    let cfg = config().lock().unwrap();
    let api_key  = cfg.llm_api_key.clone();
    let provider = cfg.llm_provider.clone();
    let model    = cfg.llm_model.clone();
    drop(cfg);
    if api_key.is_empty() && !matches!(provider.as_str(), "ollama"|"lmstudio") {
        return String::new();
    }
    let llm_cfg = match provider.to_lowercase().as_str() {
        "openai"    => crate::llm::LlmConfig::openai(&api_key, &model),
        "anthropic" => crate::llm::LlmConfig::anthropic(&api_key, &model),
        "groq"      => crate::llm::LlmConfig::groq(&api_key, &model),
        _           => crate::llm::LlmConfig::ollama(&model),
    };
    let msgs = build_messages(KALA_AI_LAB_SYSTEM_PROMPT, question);
    match crate::llm::complete(&llm_cfg, &msgs) {
        Ok(r) if !r.content.trim().is_empty() => r.content,
        _ => String::new(),
    }
}

fn lab_math() -> String {
    use crate::ml_module::{Stats, Gradient};
    let data = vec![2.0f64, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let mean = Stats::mean(&data);
    let var  = Stats::variance(&data);
    let std  = Stats::std_dev(&data);
    let grad = Gradient::derivative(&|x: f64| x * x * x, 2.0);
    format!("## ✨ Kala AI Lab — Mathematics Engine\n\n\
**Live computation on `[2,4,4,4,5,5,7,9]`:**\n\
| Measure | Result |\n|---|---|\n\
| Mean | `{:.4}` |\n| Variance | `{:.4}` |\n| Std Dev | `{:.4}` |\n\n\
**Automatic Differentiation** — `d/dx [x³]` at x=2 → **`{:.6}`** (exact: 12.0)\n\n\
### Engine Components\n\
- **Stats**: mean · variance · std · correlation · covariance\n\
- **Gradient**: central-difference derivatives · vector gradients\n\
- **FeatureScaler**: min-max normalization · z-score standardization\n\
- **EDA**: column summaries · missing values · correlation matrix\n\
- **PCA**: principal component analysis\n\n\
```killer\nstats = Stats::new()\nmean = stats.mean([2, 4, 5, 7, 9])\ngrad = Gradient::derivative(fn(x) x*x*x, 2.0)\n```\n\
*Ask: \"run linear regression\" · \"show ML algorithms\" · \"explain PCA\"*",
        mean, var, std, grad)
}

fn lab_ml() -> String {
    use crate::ml_module::{LinearRegression, KMeans, Vector};
    // Live: linear regression
    let x: Vec<Vector> = (1..=5).map(|i| Vector::new(vec![i as f64])).collect();
    let y = vec![2.0f64, 4.0, 5.0, 4.0, 5.0];
    let mut lr = LinearRegression::new(0.01);
    let _ = lr.fit(&x, &y, 100);
    let pred = lr.predict(&[Vector::new(vec![6.0])]);
    // Live: k-means
    let pts: Vec<Vector> = vec![
        Vector::new(vec![1.0,1.0]), Vector::new(vec![1.5,2.0]),
        Vector::new(vec![3.0,4.0]), Vector::new(vec![5.0,7.0]),
        Vector::new(vec![3.5,5.0]),
    ];
    let mut km = KMeans::new(2);
    let _ = km.fit(&pts, 50);
    let c0 = km.predict(&Vector::new(vec![1.2, 1.1])).unwrap_or(0);
    let c1 = km.predict(&Vector::new(vec![4.0, 6.0])).unwrap_or(1);
    format!("## \u{1f916} Kala AI Lab — Machine Learning Engine\n\nLive models trained:\n\n\
**Linear Regression** — trained on x=[1..5], y=[2,4,5,4,5]\n\
→ Prediction for x=6: **`{:.4}`**\n\n\
**K-Means (k=2)** — 5 points clustered:\n\
→ (1.2,1.1) → cluster **{}** · (4.0,6.0) → cluster **{}**\n\n\
### All Algorithms\n\
| Algorithm | Type |\n|---|---|\n\
| `LinearRegression` | Regression |\n\
| `LogisticRegression` | Classification |\n\
| `KNN` | Classification |\n\
| `DecisionTree` | Classif / Regr |\n\
| `RandomForest` | Ensemble |\n\
| `GradientBoosting` | Ensemble |\n\
| `KMeans` | Clustering |\n\
| `DBSCAN` | Density clustering |\n\
| `PCA` | Dimensionality reduction |\n\n\
```killer\nmodel = RandomForest::new(n_trees=100, max_depth=5)\nmodel.fit(X_train, y_train)\n```\n\
*Ask: \"explain gradient boosting\" · \"show DL\" · \"run NLP\"*",
        pred[0], c0, c1)
}

fn lab_dl() -> String {
    use crate::ml_module::DenseLayer;
    let mut layer = DenseLayer::new(3, 2, "relu");
    let out = layer.forward(&[0.5f64, -0.3, 0.8]);
    format!("## \u{1f52c} Kala AI Lab — Deep Learning Engine\n\n\
**Live forward pass:**\n\
`DenseLayer(3→2, relu)` · Input: [0.5, -0.3, 0.8] → Output: **[{:.4}, {:.4}]**\n\n\
### Architecture\n\
| Layer | Class |\n|---|---|\n\
| Fully connected | `DenseLayer::new(in, out, \"relu\")` |\n\
| 1D Convolution | `Conv1D::new(channels, kernel, stride)` |\n\
| LSTM cell | `LSTMCell::new(input, hidden)` |\n\
| GRU cell | `GRUCell::new(input, hidden)` |\n\n\
### Optimizers\n\
`SGD::new(lr, momentum)` · `Adam::new(lr)`\n\n\
### Activations\n\
`Activation::relu(x)` · `sigmoid(x)` · `tanh(x)` · `softmax(v)` · `leaky_relu(x, 0.01)`\n\n\
### Loss\n\
`Loss::mse(pred, target)` · `cross_entropy(probs, class)` · `binary_cross_entropy(p, t)`\n\n\
```killer\nlayer1 = DenseLayer::new(784, 128, \"relu\")\nlayer2 = DenseLayer::new(128, 10, \"softmax\")\nopt    = Adam::new(lr=0.001)\n```\n\
*Ask: \"explain LSTM\" · \"build a classifier\" · \"show transformer\"*",
        out[0], out[1])
}

fn lab_nlp() -> String {
    use crate::ml_module::{Tokenizer, TfIdf};
    let mut tfidf = TfIdf::new();
    let docs = vec!["killer language is fast and powerful",
                    "kala ai runs inside killer natively",
                    "machine learning models need data"];
    tfidf.fit(&docs);
    let vec_ = tfidf.transform("killer ai language");
    let tokens = Tokenizer::tokenize("Kala is an amazing AI system");
    format!("## \u{1f5e3} Kala AI Lab — NLP Engine\n\n\
**Live results:**\n\n\
Tokenize `\"Kala is an amazing AI system\"` → `{:?}`\n\n\
TF-IDF on 3 docs · query `\"killer ai language\"` → {} dimensions\n\n\
### NLP Components\n\
| Component | Purpose |\n|---|---|\n\
| `Tokenizer::tokenize(text)` | Split · lowercase · punct removal |\n\
| `BagOfWords` | Count-based document vectors |\n\
| `TfIdf` | TF × IDF sparse vectors |\n\
| `Word2Vec` | Dense word embeddings (skip-gram) |\n\
| `TextChunker` | Overlapping chunks for RAG |\n\
| `VectorStore` | Cosine-similarity document store |\n\
| `RagPipeline` | Retrieval-augmented generation |\n\n\
```killer\nrag = RagPipeline::new(top_k=5, max_context=2000)\nrag.index(my_doc, embedding)\nanswer = rag.build_prompt(query_emb, question)\n```\n\
*Ask: \"explain word2vec\" · \"show RAG pipeline\" · \"build text classifier\"*",
        tokens, vec_.len())
}

fn lab_rl() -> String {
    use crate::ml_module::QLearning;
    let mut ql = QLearning::new(4, 2, 0.1, 0.9, 0.1);
    ql.update(0, 0, 1.0, 1, false);
    ql.update(1, 1, -0.5, 2, false);
    ql.update(2, 0, 10.0, 3, true);
    let best = ql.best_action(0);
    format!("## 🎮 Kala AI Lab — Reinforcement Learning Engine\n\n\
**Live Q-table updated:**\nQ-Learning · 4 states · 2 actions · 3 transitions simulated\n\
→ State 0 best action: **{}**\n\n\
### RL Algorithms\n\
| Algorithm | Class |\n|---|---|\n\
| Tabular Q-Learning | `QLearning::new(states, actions, lr, gamma, eps)` |\n\
| Deep Q-Network | `DQN` (neural net Q-function) |\n\
| Experience Replay | `ReplayBuffer::new(capacity)` |\n\n\
### Bellman Equation\n\
$$Q(s,a) \\leftarrow Q(s,a) + \\alpha\\bigl[r + \\gamma \\max_{{a'}}Q(s',a') - Q(s,a)\\bigr]$$\n\n\
```killer\nagent = QLearning::new(states=100, actions=4, lr=0.1, gamma=0.99, eps=0.3)\nloop {{\n  action = agent.choose(state)\n  (next, reward) = env.step(action)\n  agent.update(state, action, reward, next)\n  state = next\n}}\n```\n\
*Ask: \"explain DQN\" · \"what is the Bellman equation\" · \"build RL agent\"*",
        best)
}

fn lab_llm() -> String {
    let pe = crate::ml_module::positional_encoding(4, 6);
    format!("## 🧠 Kala AI Lab — LLM / Transformer Engine\n\n\
**Live positional encoding (4 tokens, d=6):**\n\
Row 0: `[{:.3}, {:.3}, {:.3}, {:.3}, {:.3}, {:.3}]`\n\n\
### Transformer Components (pure Rust)\n\
| Component | Class |\n|---|---|\n\
| Scaled dot-product attention | `ScaledDotProductAttention` |\n\
| Multi-head self-attention | `MultiHeadAttention::new(d_model, heads)` |\n\
| Causal (masked) attention | `CausalAttention` |\n\
| Layer normalization | `LayerNorm::new(d_model)` |\n\
| Feed-forward block | `FeedForward::new(d_model, d_ff)` |\n\
| Full encoder layer | `TransformerEncoderLayer` |\n\
| Stacked encoder | `TransformerEncoder::new(d_model, layers)` |\n\n\
### Positional Encoding\n\
$$PE_{{pos,2i}} = \\sin\\!\\Bigl(\\frac{{pos}}{{10000^{{2i/d}}}}\\Bigr) \\quad PE_{{pos,2i+1}} = \\cos\\!\\Bigl(\\frac{{pos}}{{10000^{{2i/d}}}}\\Bigr)$$\n\n\
```killer\nenc = TransformerEncoder::new(d_model=512, n_heads=8, n_layers=6)\nout = enc.forward(token_embeddings)\n```\n\
*Ask: \"explain attention\" · \"GPT vs BERT\" · \"scaled dot product math\"*",
        pe[0][0], pe[0][1], pe[0][2], pe[0][3], pe[0][4], pe[0][5])
}

fn lab_genai() -> String {
    format!("## 🎨 Kala AI Lab — Generative AI Engine\n\n\
### Models (pure Rust)\n\
| Model | Class | Use Case |\n|---|---|---|\n\
| Variational Autoencoder | `VAE::new(input, latent)` | Smooth latent space generation |\n\
| Autoencoder | `Autoencoder::new(dims)` | Compression + reconstruction |\n\
| GAN | `GAN::new(gen, disc)` | Adversarial generation |\n\
| Diffusion (DDPM) | `DiffusionModel::new(steps, b0, b1)` | Noise→data synthesis |\n\n\
### Also Live: Nova Native Generator\n\
- 🖼️ **Image**: `generate image of ocean` → 128×128 PNG, <50ms\n\
- 🎥 **Video**: `generate video of space` → animated GIF, ~80ms\n\
- 🎵 **Audio**: `generate music beat` → WAV, ~60ms\n\n\
### GAN Training\n\
```killer\ngan = GAN::new(gen=Generator::new([100,256,512]),\n               disc=Discriminator::new([512,256,1]))\nfor epoch in 0..1000 {{ gan.train_step(real_data, noise) }}\n```\n\n\
### Diffusion (DDPM)\n\
```killer\nmodel = DiffusionModel::new(steps=1000, beta_start=0.0001, beta_end=0.02)\nnoisy = model.forward_process(x0, t)\nrecon = model.reverse_process(noisy, t)\n```\n\
*Ask: \"explain VAE latent space\" · \"how does diffusion work\" · \"generate a space image\"*")
}

fn lab_agents() -> String {
    use crate::ml_module::{ReasoningAgent, Tool};
    let mut agent = ReasoningAgent::new("Kala", "helpful AI assistant", 20);
    agent.register_tool(Tool { name: "calculator".into(), description: "compute math".into() });
    agent.register_tool(Tool { name: "search".into(), description: "search knowledge".into() });
    agent.add_message("user", "hello");
    format!("## \u{1f916} Kala AI Lab — AI Agents Engine\n\n\
**Live agent initialized:**\nAgent **\"Kala\"** · tools: [calculator, search] · 1 message in context\n\n\
### Agent Components\n\
| Component | Purpose |\n|---|---|\n\
| `ReasoningAgent::new(name, prompt, mem)` | Full ReAct loop |\n\
| `register_tool(tool)` | Add callable tool |\n\
| `AgentMemory` | Short+long-term with recall |\n\
| `MemoryEntry` | Individual memory entry |\n\
| `Tool {{ name, description }}` | Tool definition |\n\n\
### ReAct Pattern\n\
```killer\nagent = ReasoningAgent::new(\"Planner\", \"strategic AI\", 50)\nagent.register_tool(Tool {{ name: \"web_search\", description: \"search web\" }})\nagent.register_tool(Tool {{ name: \"calculator\", description: \"compute math\" }})\n```\n\
### Memory Architecture\n\
- Short-term: sliding window of messages\n- Long-term: importance-ranked with cosine-similarity recall\n- Working: active context during reasoning\n\n\
*Ask: \"build a research agent\" · \"how do tool calls work\" · \"show multi-agent\"*")
}

fn lab_multi_agent() -> String {
    use crate::ml_module::{AgentTeam, ReasoningAgent};
    let mut team = AgentTeam::new();
    team.add_agent(ReasoningAgent::new("Researcher", "finds information", 20));
    team.add_agent(ReasoningAgent::new("Analyst",    "analyzes data",      20));
    team.add_agent(ReasoningAgent::new("Writer",     "synthesizes reports", 20));
    let (decision, yes, no) = team.consensus_vote("should we proceed?");
    let resps = team.broadcast("new task: summarize the AI landscape", vec![0.1f64; 8]);
    format!("## \u{1f465} Kala AI Lab — Multi-Agent System\n\n\
**Live 3-agent team:**\n\
- \u{1f4ac} Broadcast → Researcher: *\"{}\"*\n\
- \u{1f5f3} Vote \"should we proceed?\" → **{}** (yes={}, no={})\n\n\
### API\n\
| Method | Effect |\n|---|---|\n\
| `AgentTeam::new()` | Create team |\n\
| `team.add_agent(agent)` | Add agent |\n\
| `team.broadcast(msg, emb)` | All agents respond |\n\
| `team.consensus_vote(q)` | Majority vote |\n\n\
### Patterns\n\
**Pipeline**: Researcher → Analyst → Writer\n\
**Swarm**: N identical agents parallelize a task\n\
**Consensus**: agents vote on decisions\n\n\
```killer\nteam = AgentTeam::new()\nteam.add_agent(Researcher::new())\nteam.add_agent(Analyst::new())\nresults = team.broadcast(\"analyze climate data\", query_emb)\n```\n\
*Ask: \"design multi-agent pipeline\" · \"swarm intelligence\" · \"consensus voting\"*",
        resps.first().cloned().unwrap_or_default(),
        if decision { "YES ✅" } else { "NO ❌" }, yes, no)
}

fn lab_agi() -> String {
    format!("## 🧠 Kala AI Lab — AGI (curriculum & roadmap)\n\n\
**AGI is not implemented** as a product capability here — this mode is for **definitions, gaps, and research context**.\n\n\
### What Killer actually ships today (narrow / specialist AI)\n\
| Area | In runtime (honest scope) |\n|---|---|\n\
| Reasoning assistance | `native_think`, KhLM tiers, optional GGUF / configured LLM — **not** human-level AGI |\n\
| Language | Tokenizers, TF-IDF, transformer **components** for learning, GGUF chat — **not** a full frontier trainer |\n\
| Memory & RAG | Vector store / RAG-style pieces in stack — bounded, not autonomous long-horizon AGI memory |\n\
| Tools & agents | `ReasoningAgent`, `AgentTeam` demos, `khlm_ai_system` orchestration — **orchestration**, not AGI |\n\
| Safety | `Guardian` — policy checks, not a proof of alignment |\n\n\
### Open problems (field-wide)\n\
Causal reasoning · compositional generalization · continual learning · value alignment · robust world models.\n\n\
> *\"An AGI that cannot be stopped is not intelligent — it's dangerous.\"*\n\
> Killer keeps human oversight hooks; that does **not** mean AGI exists in this codebase.\n\n\
*Ask: \"what's missing for AGI\" · \"causal reasoning\" · \"value alignment\"*")
}

fn lab_asi() -> String {
    format!("## ⚡ Kala AI Lab — ASI (curriculum only)\n\n\
**ASI is not implemented** in Killer — this is **educational framing** about a hypothetical future class of systems.\n\n\
### ASI vs human vs AGI (conceptual)\n\
| Dimension | Human | AGI (goal) | ASI (hypothetical) |\n|---|---|---|---|\n\
| Scope | General | Target: broad competence | Speculative superhuman |\n\
| Self-improvement | Slow | Uncertain | Often discussed as recursive risk |\n\
| Alignment | Social | Open problem | **Critical** open problem |\n\n\
### Where Killer sits\n\
> **Narrow + specialist tooling**: routed LLM/KhLM, native inference, demos, safety hooks — **not** ASI and not AGI.\n\n\
### Alignment-related pieces in Killer (real, but limited)\n\
- **Guardian** — checks content / policy; not a full alignment solution.\n\
- Bounded autonomy by design.\n\n\
*Ask: \"does consciousness need biology\" · \"value alignment\" · \"is ASI inevitable\"*")
}

fn lab_aios() -> String {
    format!("## 💻 Kala AI Lab — “AI OS” (concept vs what Killer is)\n\n\
**Killer is not a general-purpose operating system.** It is a **language runtime + libraries + Kala UI** with AI features baked in. \
The phrase “AI OS” here means **curriculum**: how people *imagine* an AI-first stack — not a shipped kernel replacing Windows/Linux/macOS.\n\n\
### Conceptual stack (how pieces fit — not a real OS monolith)\n\
```\n┌────────────────────────────────────────┐\n\
│  KALA UI — chat / modes / HTTP server  │\n\
├────────────────────────────────────────┤\n\
│  KhLM + Ghost-108 + inference tiers    │\n\
├────────────────────────────────────────┤\n\
│  ml_module · vision · tools · agents    │\n\
├────────────────────────────────────────┤\n\
│  Killer VM · compiler · builtins        │\n\
├────────────────────────────────────────┤\n\
│  Host OS (your machine) + Rust std      │\n\
└────────────────────────────────────────┘\n```\n\n\
### Honest comparison\n\
| Killer + Kala | Traditional OS |\n|---|---|\n\
| App / runtime you run **on** an OS | Manages hardware & processes |\n\
| AI-assisted developer experience | General scheduling & drivers |\n\n\
*Ask: \"what makes Killer AI-native\" · \"compare to Linux\" — we answer as **language+runtime**, not OS replacement.*")
}

fn lab_ai_systems() -> String {
    format!("## ⚙️ Kala AI Lab — AI Systems Architecture\n\n\
### End-to-End ML Pipeline\n\
```\nData → [Ingest] → [Preprocess] → [Feature Eng]\n     → [Train] → [Evaluate] → [Deploy]\n     → [Monitor] → [Feedback] → [Retrain]\n```\n\n\
### Native Module per Stage\n\
| Stage | Module |\n|---|---|\n\
| Ingest | `json_csv` · `file_io` · `http_client` |\n\
| Preprocess | `FeatureScaler` · `LabelEncoder` · `OneHotEncoder` |\n\
| Feature Eng | `PCA` · `TfIdf` · `Word2Vec` · `EDA` |\n\
| Train | `ml_module` — implemented algorithms (see AI Lab demos) |\n\
| Evaluate | `ClassificationMetrics` · `AdvancedMetrics` |\n\
| Deploy | `kala_serve()` — HTTP inference |\n\
| Monitor | `telemetry` · `audit_logger` |\n\n\
### Responsible AI\n\
- `LocalExplainer` — LIME/SHAP feature importance\n\
- `BiasDetector` — fairness metrics\n\
- `DifferentialPrivacy` — ε-δ guarantees\n\
- `ModelCard` — auto-generate model docs\n\n\
*Ask: \"build full ML pipeline\" · \"explain model cards\" · \"differential privacy\"*")
}

fn lab_programming() -> String {
    format!("## 💻 Kala AI Lab — Programming Engine\n\n\
### Code Generation\n\
Say: `write python hello world` · `write html page` · `write rust fibonacci`\n\n\
### Built-in Algorithms\n\
```killer\n// Sorting\nsorted = quick_sort([5,3,1,4,2])\nsorted = merge_sort([5,3,1,4,2])\n// Search\nidx = binary_search(arr, target)\n// Graphs\npath = bfs(graph, start)\ndist = dijkstra(graph, source)\n```\n\n\
### Complexity Reference\n\
| Algorithm | Time | Space |\n|---|---|---|\n\
| QuickSort | O(n log n) avg | O(log n) |\n\
| MergeSort | O(n log n) | O(n) |\n\
| BinarySearch | O(log n) | O(1) |\n\
| Dijkstra | O((V+E) log V) | O(V) |\n\n\
*Ask: \"write a binary tree\" · \"explain Big O\" · \"write a REST API\"*")
}

fn lab_overview() -> String {
    format!("## 🧪 Kala AI Lab — What is real vs curriculum\n\n\
### A) Native Rust engines (runs in this binary)\n\
| Topic | Try saying | What you get |\n|---|---|---|\n\
| **Math** | `math gradient descent` | Live stats / gradients (see Math lab) |\n\
| **ML** | `run linear regression` | Trained toy models (see ML lab) |\n\
| **Deep Learning** | `explain LSTM` | Layer demos + docs |\n\
| **NLP** | `show NLP pipeline` | Tokenize / TF-IDF demos |\n\
| **LLM / Transformers** | `how does attention work` | Component math + positional encodings |\n\
| **GenAI (toy)** | `explain GAN training` | Module APIs + Nova media where wired |\n\
| **RL (tabular)** | `rl q-learning example` | Q-table updates |\n\
| **Agents / multi-agent** | `multi agent team` | `AgentTeam` / `ReasoningAgent` demos |\n\
| **Programming** | `write python ML code` | Routed to code / Ask as appropriate |\n\n\
### B) Curriculum & roadmap (concepts — **not** shipped AGI/ASI/OS)\n\
| Topic | Try saying | Note |\n|---|---|---|\n\
| **AGI** | `what is AGI` | Definitions & gaps — **not** a product tier |\n\
| **ASI** | `explain ASI` | Hypothetical — **not** implemented |\n\
| **“AI OS”** | `ai os architecture` | **Analogy** vs Linux/Windows — Killer is a runtime |\n\
| **AI Systems** | `ai systems pipeline` | Architecture patterns + native modules |\n\n\
### Media (when generation paths are enabled)\n\
- Image / video / audio prompts may route to Nova / Ask / Imagine — timing depends on build and host.\n\n\
*Pick a row from (A) for hands-on Rust demos, or (B) for honest conceptual depth.*")
}

// Unified 5-Tier Router

/// Core routing function.
/// Returns (answer, tier_name, latency_ms).
pub fn khlm_polyglot_ask(operation: &str, lang: &str, code: &str, extra: &str)
    -> (String, &'static str, u128)
{
    let start = Instant::now();
    let cache_key = quick_hash(&format!("{}:{}:{}:{}", operation, lang, code, extra));

    // Check result cache
    {
        let cfg = config().lock().unwrap();
        if let Some(cached) = cfg.cache.get(&cache_key) {
            return (cached.text.clone(), "CAG/Cache", cached.ms);
        }
    }

    // Build the prompt
    let prompt = build_prompt(operation, lang, code, extra);

    // Tier 0: CAG static knowledge base
    // Extract result while holding lock, then release lock BEFORE calling cache_result
    let cag_result: Option<String> = {
        let cfg = config().lock().unwrap();
        if cfg.cag_enabled {
            // Search against code + error context (extra often contains stderr)
            let search_text = format!("{} {} {}", prompt, code, extra).to_lowercase();
            cag_lookup(&search_text, lang)
        } else {
            None
        }
    }; // lock released here

    if let Some(cag_answer) = cag_result {
        let ms = start.elapsed().as_millis();
        let answer = format_answer(&cag_answer, "KhLM-Polyglot/CAG", 0, ms);
        cache_result(cache_key, &answer, 0, ms); // safe: lock already released
        return (answer, "Tier0/CAG", ms);
    }

    // Tier 1: KhLM deterministic (math/conversions inside code)
    // For simple ops KhLM can handle instantly - skip to LLM for code ops

    // Tier 2: LLM (Ollama / Groq / OpenAI / Anthropic)
    {
        let cfg = config().lock().unwrap();
        if cfg.llm_available() {
            let cfg_clone = cfg.clone();
            drop(cfg);
            if let Some(answer) = llm_call(&prompt, &cfg_clone) {
                let ms = start.elapsed().as_millis();
                let formatted = format_answer(&answer, &format!("KhLM-Polyglot/LLM({})", cfg_clone.llm_provider), 2, ms);
                cache_result(cache_key, &formatted, 2, ms);
                return (formatted, "Tier2/LLM", ms);
            }
        }
    }

    // Tier 3: RLM deep reasoning
    {
        let cfg = config().lock().unwrap();
        if cfg.rlm_available() {
            let model = cfg.rlm_model.clone();
            drop(cfg);
            if let Some(answer) = rlm_call(&prompt, &model) {
                let ms = start.elapsed().as_millis();
                let formatted = format_answer(&answer, "KhLM-Polyglot/RLM", 3, ms);
                cache_result(cache_key, &formatted, 3, ms);
                return (formatted, "Tier3/RLM", ms);
            }
        }
    }

    // Tier 4: KhLM / Ghost-108 fallback
    // Skip Ghost-108 web search when LLM+RLM are both offline.
    // Return a guidance message so the user knows how to enable AI backends.
    let (llm_avail, rlm_avail) = {
        let cfg = config().lock().unwrap();
        (cfg.llm_available(), cfg.rlm_available())
    };

    if !llm_avail && !rlm_avail {
        let ms = start.elapsed().as_millis();
        let offline_msg = format!(
            "No AI backend configured for '{}' ({}).\n\
             Enable one:\n\
             - khlm_set_llm(\"ollama\", \"\", \"llama3\")       local Ollama\n\
             - khlm_set_llm(\"groq\", \"<key>\", \"llama3-70b\") Groq cloud\n\
             - khlm_set_rlm(\"/path/to/model.gguf\")           local GGUF",
            operation, lang
        );
        let formatted = format_answer(&offline_msg, "KhLM-Polyglot/Offline", 4, ms);
        cache_result(cache_key, &formatted, 4, ms);
        return (formatted, "Tier4/Offline", ms);
    }

    let khlm_answer = crate::llm::khlm_ask(&prompt);
    let ms = start.elapsed().as_millis();
    let formatted = format_answer(&khlm_answer, "KhLM-Polyglot/KhLM", 4, ms);
    cache_result(cache_key, &formatted, 4, ms);
    (formatted, "Tier4/KhLM", ms)
}

fn cache_result(key: u64, text: &str, tier: u8, ms: u128) {
    if let Ok(mut cfg) = config().lock() {
        cfg.cache.insert(key, CachedAnswer { text: text.to_string(), tier, ms });
    }
}

fn format_answer(text: &str, engine: &str, tier: u8, ms: u128) -> String {
    format!("[KhLM/{} tier={} {}ms]\n{}", engine, tier, ms, text)
}

fn build_prompt(operation: &str, lang: &str, code: &str, extra: &str) -> String {
    match operation {
        "debug" => format!(
            "Debug this {} code.\nCode:\n{}\nError:\n{}\n\nExplain the error and provide a fix.",
            lang, code, extra
        ),
        "suggest" => format!(
            "Review this {} code for performance, readability, and best practices. Give up to 3 concise suggestions.\nCode:\n{}",
            lang, code
        ),
        "explain" => format!(
            "Explain what this {} code does in 2-3 sentences. Be concise.\nCode:\n{}",
            lang, code
        ),
        "fix" => format!(
            "Fix this {} code that has the following error: {}\nCode:\n{}\n\nReturn only the corrected code.",
            lang, extra, code
        ),
        "translate" => format!(
            "Translate this {} code to {}. Preserve all logic exactly.\nCode:\n{}\n\nReturn only the translated code.",
            lang, extra, code
        ),
        _ => format!("{}\nLanguage: {}\nCode:\n{}\n{}", operation, lang, code, extra),
    }
}

// Killer Builtin Functions

fn get_str_arg(args: &[Value], idx: usize, name: &str) -> Result<String, VmError> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(_) => Err(VmError::runtime_error(format!("{} must be a String", name))),
        None    => Err(VmError::runtime_error(format!("Missing argument: {}", name))),
    }
}

/// khlm_debug(code, lang) - AI debug of code, auto-routed through all tiers.
pub fn builtin_khlm_debug(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let code = get_str_arg(args, 0, "code")?;
    let lang = get_str_arg(args, 1, "lang").unwrap_or_else(|_| "unknown".into());
    let error_ctx = get_str_arg(args, 2, "error").unwrap_or_default();
    let (answer, _, _) = khlm_polyglot_ask("debug", &lang, &code, &error_ctx);
    Ok(Value::Str(answer))
}

/// khlm_suggest(code, lang) - performance/readability improvement hints.
pub fn builtin_khlm_suggest(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let code = get_str_arg(args, 0, "code")?;
    let lang = get_str_arg(args, 1, "lang").unwrap_or_else(|_| "unknown".into());
    let (answer, _, _) = khlm_polyglot_ask("suggest", &lang, &code, "");
    Ok(Value::Str(answer))
}

/// khlm_explain(code, lang) - explain what code does.
pub fn builtin_khlm_explain(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let code = get_str_arg(args, 0, "code")?;
    let lang = get_str_arg(args, 1, "lang").unwrap_or_else(|_| "unknown".into());
    let (answer, _, _) = khlm_polyglot_ask("explain", &lang, &code, "");
    Ok(Value::Str(answer))
}

/// khlm_fix(code, error, lang) - return corrected code given an error message.
pub fn builtin_khlm_fix(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let code  = get_str_arg(args, 0, "code")?;
    let error = get_str_arg(args, 1, "error")?;
    let lang  = get_str_arg(args, 2, "lang").unwrap_or_else(|_| "unknown".into());
    let (answer, _, _) = khlm_polyglot_ask("fix", &lang, &code, &error);
    Ok(Value::Str(answer))
}

/// khlm_translate(code, from_lang, to_lang) - translate between languages.
pub fn builtin_khlm_translate(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let code      = get_str_arg(args, 0, "code")?;
    let from_lang = get_str_arg(args, 1, "from_lang")?;
    let to_lang   = get_str_arg(args, 2, "to_lang")?;
    let (answer, _, _) = khlm_polyglot_ask("translate", &from_lang, &code, &to_lang);
    Ok(Value::Str(answer))
}

/// khlm_status() - show which AI tiers are configured and available.
pub fn builtin_khlm_status(_args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let cfg = config().lock().unwrap();

    let llm_status = if cfg.llm_available() {
        format!(
            "ACTIVE  ({} / {}, max_tokens={}, temp={:.2})",
            cfg.llm_provider, cfg.llm_model, cfg.max_tokens, cfg.llm_temperature
        )
    } else {
        "OFFLINE  (set env KILLER_KHLM_LLM_* or khlm_set_llm())".into()
    };

    let rlm_status = if cfg.rlm_available() {
        format!("ACTIVE  ({})", cfg.rlm_model)
    } else {
        "OFFLINE  (set env KILLER_KHLM_GGUF / KILLER_KHLM_RLM or khlm_set_rlm())".into()
    };

    let cache_size = cfg.cache.len();

    let ask_route = if cfg.llm_available() {
        "Kala Ask: online-first → API LLM → KhLM/Ghost web → embedded KB → router\n"
    } else {
        "Kala Ask: offline-first → embedded KB → smart templates → web when allowed\n"
    };

    let out = format!(
        "KhLM-Polyglot Tier Status\n\
         {}\n\
         Tier 0  CAG Pattern Index   {}  <1ms\n\
         Tier 1  KhLM Deterministic  always active  <1ms\n\
         Tier 2  LLM (ext AI)        {}  200-800ms\n\
         Tier 3  RLM (local model)   {}  500-2000ms\n\
         Tier 4  Ghost-108 web       always active  2-8s\n\
         Cache: {} entries\n\
         Or env (loaded at startup):\n\
           KILLER_KHLM_LLM_PROVIDER=ollama  KILLER_KHLM_LLM_MODEL=llama3.2  KILLER_KHLM_LLM_API_KEY=\n\
           KILLER_KHLM_GGUF=path-or-short-name   KILLER_KHLM_LLM_MAX_TOKENS=2048   KILLER_KHLM_LLM_TEMPERATURE=0.7\n\
         Or builtins:\n\
           khlm_set_llm(\"ollama\", \"\", \"llama3\")\n\
           khlm_set_llm(\"groq\", \"<api_key>\", \"llama3-70b-8192\")\n\
           khlm_set_rlm(\"/path/to/deepseek-r1.gguf\")\n",
        ask_route,
        if cfg.cag_enabled { "ACTIVE " } else { "DISABLED" },
        llm_status, rlm_status,
        cache_size,
    );
    Ok(Value::Str(out))
}

/// khlm_set_llm(provider, api_key, model) - configure LLM for Tier 2.
// provider: "ollama" | "groq" | "openai" | "anthropic"
pub fn builtin_khlm_set_llm(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let provider = get_str_arg(args, 0, "provider")?;
    let api_key  = get_str_arg(args, 1, "api_key").unwrap_or_default();
    let model    = get_str_arg(args, 2, "model")
        .unwrap_or_else(|_| default_model_for(&provider));

    let valid = matches!(provider.to_lowercase().as_str(),
        "ollama" | "groq" | "openai" | "anthropic");
    if !valid {
        return Err(VmError::runtime_error(
            "khlm_set_llm: provider must be 'ollama', 'groq', 'openai', or 'anthropic'"
        ));
    }

    let mut cfg = config().lock().unwrap();
    cfg.llm_provider = provider.clone();
    cfg.llm_api_key  = api_key;
    cfg.llm_model    = model.clone();
    Ok(Value::Str(format!(
        "KhLM Tier 2 LLM configured: {} / {}", provider, model
    )))
}

/// khlm_set_rlm(model_path) - configure local RLM model for Tier 3.
pub fn builtin_khlm_set_rlm(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let path = get_str_arg(args, 0, "model_path")?;
    let mut cfg = config().lock().unwrap();
    cfg.rlm_model = path.clone();
    Ok(Value::Str(format!("KhLM Tier 3 RLM configured: {}", path)))
}

/// Clear Polyglot code-op cache plus global LLM/prefetch cache (see `llm::khlm_inference_cache_clear`).
/// Safe to call anytime; does not require LLM capability (fixes stuck wrong answers when LLM is off).
pub fn clear_all_khlm_caches() -> (usize, usize) {
    let poly_n = {
        let mut cfg = config().lock().unwrap();
        let n = cfg.cache.len();
        cfg.cache.clear();
        n
    };
    let llm_n = crate::llm::khlm_inference_cache_clear();
    (poly_n, llm_n)
}

/// khlm_cache_clear() / kala_clear_cache() — clear all KhLM answer caches (Polyglot + inference).
pub fn builtin_khlm_cache_clear(_args: &[Value]) -> Result<Value, VmError> {
    let (poly_n, llm_n) = clear_all_khlm_caches();
    Ok(Value::Str(format!(
        "Caches cleared: KhLM-Polyglot={poly_n} entries, inference/LLM={llm_n} entries."
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
//  khlm_write — Prose Engine
//  khlm_write(topic)          → essay-style prose on any topic
//  khlm_write(topic, style)   → style: "essay" | "summary" | "technical" |
//                               "story" | "formal" | "casual" | "explain"
//
//  Tier 2 (LLM) → GPT-4o / Claude quality prose
//  Offline      → Native multi-paragraph template engine (no external call)
// ─────────────────────────────────────────────────────────────────────────────

pub fn builtin_khlm_write(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_llm()?;
    let topic = get_str_arg(args, 0, "topic")?;
    let style = get_str_arg(args, 1, "style").unwrap_or_else(|_| "essay".into());

    // Tier 2: LLM if available
    {
        let cfg = config().lock().unwrap();
        if cfg.llm_available() {
            let cfg_clone = cfg.clone();
            drop(cfg);
            let topic_for_prompt = if style.to_lowercase() == "story" {
                normalize_story_topic_for_prose(&topic)
            } else {
                topic.clone()
            };
            let prompt = format!(
                "Write a well-structured, engaging {} about: {}\n\n\
                 Requirements:\n\
                 - Start with a direct, compelling opening.\n\
                 - Use ## headings to separate major sections.\n\
                 - 3-5 paragraphs with clear introduction, body and conclusion.\n\
                 - Use specific examples, evidence, or analogies.\n\
                 - Include a **Key Takeaway** section at the end.\n\
                 - Varied sentence structure for readability.\n\
                 Write directly in {} style without meta-commentary.",
                style, topic_for_prompt, style
            );
            if let Some(out) = llm_call(&prompt, &cfg_clone) {
                return Ok(Value::Str(out));
            }
        }
    }

    // Offline native prose engine
    Ok(Value::Str(native_prose(&topic, &style)))
}

// ── Offline Prose Engine ─────────────────────────────────────────────────────

/// True when the "topic" is really a code request — avoid vapid `prose_casual` / essay templates.
fn topic_is_code_or_implementation_request(topic: &str) -> bool {
    let q = topic.to_lowercase();
    let wc = q.split_whitespace().count();
    let langish = q.contains("javascript") || q.contains("typescript") || q.contains("python")
        || q.contains("rust") || q.contains("kotlin") || q.contains("swift") || q.contains("go ")
        || q.contains("java") || q.contains("c++") || q.contains("cpp") || q.contains("csharp")
        || q.contains("ruby") || q.contains("php") || q.contains("sql") || q.contains("html")
        || q.contains("css") || q.contains("bash") || q.contains("shell")
        || q.contains("three.js") || q.contains("threejs") || q.contains("webgl") || q.contains("wgpu")
        || q.contains("react") || q.contains("node.js") || q.contains("nodejs");
    let codewords = q.contains("function") || q.contains(" class ") || q.starts_with("class ")
        || q.contains("algorithm") || q.contains("implement") || q.contains("snippet")
        || q.contains("debug") || q.contains("refactor") || q.contains("unit test")
        || q.contains("gesture") || q.contains("gestor") || q.contains("gester")
        || q.contains("facemesh") || q.contains("landmark") || q.contains("import ")
        || q.contains("api ") || q.contains("endpoint") || q.contains("websocket")
        || q.contains(".js") || q.contains("jsx") || q.contains("json ");
    let not_definitional = !q.starts_with("what is ") && !q.starts_with("what are ")
        && !q.starts_with("why ") && !q.starts_with("explain ") && !q.starts_with("describe ")
        && !q.starts_with("define ") && !q.starts_with("how does ") && !q.starts_with("how do ");
    let short_code_ask = q.contains("code") && not_definitional
        && (q.contains("write") || q.contains("generat") || q.contains("creat") || q.contains("make ")
            || q.contains("give me") || q.contains("show me") || q.contains("need ")
            || (wc <= 6 && !q.starts_with("what") && !q.starts_with("how ")));
    langish || codewords || short_code_ask
}

/// When the user says only "tell me a story" (no subject), the topic must not be pasted into templates verbatim.
fn normalize_story_topic_for_prose(topic: &str) -> String {
    let s = topic.trim().trim_end_matches(|c: char| matches!(c, '.' | '!' | '?')).trim();
    let low = s.to_lowercase();
    let meta_only = matches!(
        low.as_ref(),
        "tell me a story"
            | "tell me story"
            | "tell a story"
            | "a story"
            | "story"
            | "short story"
            | "i want a story"
            | "give me a story"
            | "write a story"
    ) || low.starts_with("tell me a story,")
        || low == "tell me a short story";
    if meta_only {
        return "a lighthouse keeper who finds a message in a bottle that should not exist".to_string();
    }
    let mut rest = low.as_str();
    for pref in [
        "tell me a story about ",
        "tell a story about ",
        "write a story about ",
        "tell me a story ",
        "tell me about ",
    ] {
        if let Some(r) = rest.strip_prefix(pref) {
            rest = r;
            break;
        }
    }
    let stripped = rest.trim();
    if stripped.is_empty()
        || stripped == "a story"
        || stripped == "story"
        || stripped == "me a story"
    {
        return "two friends who build a machine that dreams in color".to_string();
    }
    stripped.to_string()
}

fn native_prose(topic: &str, style: &str) -> String {
    // Offline write engine: never substitute generic prose for obvious code tasks (fixes weak Kala output).
    if topic_is_code_or_implementation_request(topic) {
        return khlm_generate_code(topic);
    }
    match style.to_lowercase().as_str() {
        "summary"   => prose_summary(topic),
        "technical" => prose_technical(topic),
        "story"     => prose_story(&normalize_story_topic_for_prose(topic)),
        "formal"    => prose_formal(topic),
        "casual"    => prose_casual(topic),
        "explain"   => prose_explain(topic),
        _           => prose_essay(topic),
    }
}

/// Detect broad domain from topic text for richer context words
fn topic_domain(topic: &str) -> &'static str {
    let t = topic.to_lowercase();
    if t.contains("ai") || t.contains("machine learning") || t.contains("neural")
       || t.contains("language model") || t.contains("killer") || t.contains("software")
       || t.contains("computer") || t.contains("code") || t.contains("program")
       || t.contains("algorithm") || t.contains("technology") || t.contains("robot")
       || t.contains("artificial") || t.contains("deep learning") || t.contains("data science")
       || t.contains("blockchain") || t.contains("cloud") || t.contains("api")
    { return "technology"; }
    if t.contains("climate") || t.contains("environment") || t.contains("carbon")
       || t.contains("ocean") || t.contains("ecosystem") || t.contains("nature")
    { return "environment"; }
    if t.contains("history") || t.contains("war") || t.contains("empire")
       || t.contains("ancient") || t.contains("revolution") || t.contains("century")
    { return "history"; }
    if t.contains("brain") || t.contains("science") || t.contains("physics")
       || t.contains("biology") || t.contains("chemistry") || t.contains("quantum")
       || t.contains("space") || t.contains("universe")
    { return "science"; }
    if t.contains("economy") || t.contains("market") || t.contains("invest")
       || t.contains("finance") || t.contains("business") || t.contains("startup")
    { return "business"; }
    if t.contains("art ") || t.contains(" art") || t.starts_with("art") && !t.contains("artific")
       || t.contains("music") || t.contains("film") || t.contains("cinema")
       || t.contains("literature") || t.contains("culture") || t.contains("creative")
       || t.contains("painting") || t.contains("sculpture") || t.contains("poetry")
    { return "arts"; }
    "general"
}

/// Pick domain-appropriate vocabulary for variety
fn domain_vocab(domain: &str) -> (&'static str, &'static str, &'static str) {
    // returns (transition_word, evidence_word, impact_word)
    match domain {
        "technology"   => ("Furthermore,",  "empirical benchmarks demonstrate", "transformative potential"),
        "environment"  => ("Moreover,",     "ecological data reveals",          "long-term sustainability"),
        "history"      => ("In addition,",  "historical records show",          "lasting legacy"),
        "science"      => ("Subsequently,", "experimental evidence confirms",   "paradigm-shifting implications"),
        "business"     => ("Additionally,", "market analysis indicates",        "competitive advantage"),
        "arts"         => ("Beyond this,",  "critical examination reveals",     "profound cultural impact"),
        _              => ("Furthermore,",  "careful analysis shows",           "significant consequences"),
    }
}

fn prose_essay(topic: &str) -> String {
    let domain = topic_domain(topic);
    let (transition, evidence, impact) = domain_vocab(domain);

    format!(
        "{topic} is a subject that commands serious attention from thinkers, \
         practitioners, and curious minds alike. At its core, it represents \
         one of the defining challenges and opportunities of our time — \
         one that rewards deep examination and rewards those who engage \
         with it honestly.\n\n\
         To understand {topic} fully, one must first appreciate its origins. \
         The forces that gave rise to {topic} did not emerge overnight; they \
         accumulated over years, shaped by technological change, human \
         ambition, and shifting societal needs. {evidence} that the earliest \
         forms of what we now call {topic} were rudimentary compared to \
         what exists today — yet the seeds were always present.\n\n\
         {transition} the practical implications of {topic} extend far beyond \
         the obvious. Every domain that touches {topic} — from education \
         and governance to industry and interpersonal communication — \
         must reckon with its consequences. The {impact} of {topic} \
         is not abstract: it reshapes daily decisions, redirects resources, \
         and alters what is considered possible.\n\n\
         The critics of {topic} raise valid concerns. No phenomenon of this \
         magnitude arrives without trade-offs, and those who resist simplistic \
         enthusiasm are performing an essential service. The question is not \
         whether {topic} matters, but what governance, ethics, and thoughtful \
         design can make it serve human flourishing rather than undermine it.\n\n\
         In conclusion, {topic} is neither a saviour nor a threat in isolation — \
         it is a mirror. It reflects the values, priorities, and competencies \
         of those who shape it. The most important investment any individual \
         or organisation can make is not simply to adopt {topic}, but to \
         understand it deeply enough to wield it wisely.",
        topic = topic, evidence = evidence,
        transition = transition, impact = impact
    )
}

fn prose_summary(topic: &str) -> String {
    let domain = topic_domain(topic);
    let (_, evidence, impact) = domain_vocab(domain);

    format!(
        "{topic} refers to the body of ideas, practices, and systems \
         centred on a specific and significant concern in {domain}.\n\n\
         Key points:\n\
         • Origins and context: {topic} emerged from the intersection of \
           technological capability, human need, and historical circumstance.\n\
         • Current state: {evidence} that {topic} has reached a critical \
           inflection point, with adoption accelerating across sectors.\n\
         • Impact: The {impact} of {topic} is already measurable in \
           productivity, creativity, and decision-making quality.\n\
         • Open questions: Governance, equitable access, and long-term \
           sustainability remain active areas of debate.\n\n\
         Bottom line: {topic} is significant, consequential, and best \
         understood through direct engagement rather than passive observation.",
        topic = topic, domain = domain,
        evidence = evidence, impact = impact
    )
}

fn prose_technical(topic: &str) -> String {
    format!(
        "Technical Overview: {topic}\n\
         ─────────────────────────────────\n\n\
         Architecture and Design Principles\n\n\
         At the technical level, {topic} is best understood as a composite \
         system where multiple interacting components produce emergent \
         behaviour that neither component can achieve alone. The foundational \
         architecture divides into three layers: the input/data layer, the \
         processing/transformation layer, and the output/application layer.\n\n\
         Key Technical Properties\n\n\
         • Scalability: Systems built around {topic} are designed with \
           horizontal scalability in mind, allowing throughput to increase \
           linearly with added resources under standard load.\n\
         • Fault Tolerance: Redundancy and graceful degradation are built-in \
           assumptions. A well-designed {topic} implementation fails partially \
           rather than catastrophically.\n\
         • Latency Characteristics: The critical path in {topic} processing \
           involves IO-bound and compute-bound stages. Profiling tools should \
           be applied to identify bottlenecks before optimisation.\n\n\
         Implementation Considerations\n\n\
         Engineers working with {topic} must account for: state management \
         across distributed components, serialisation format compatibility, \
         versioning of interfaces, and observability instrumentation. \
         Testing strategies should include unit tests for individual \
         components, integration tests across system boundaries, and \
         load tests to validate performance under realistic conditions.\n\n\
         Performance Benchmark Guidance\n\n\
         Baseline measurements for {topic} should capture p50, p95, and p99 \
         latency across representative workloads. Optimisation effort should \
         target the p99 first, as long-tail latency typically indicates \
         resource contention or garbage collection pauses rather than \
         algorithmic inefficiency.",
        topic = topic
    )
}

fn prose_story(topic: &str) -> String {
    format!(
        "There was a moment — no one could say exactly when — \
         when {topic} changed everything.\n\n\
         It did not announce itself with fanfare. The first signs were small: \
         a conversation that went differently than expected, a result that \
         arrived faster than it should have, a problem that dissolved where \
         it had previously been impenetrable. People noticed, then looked away, \
         then looked back and could not stop looking.\n\n\
         Those who had worked for years on the problem of {topic} felt a \
         strange mixture of vindication and vertigo. They had known this was \
         coming — theoretically, abstractly — but theory and reality are \
         separated by a chasm that only experience can bridge. Now, standing \
         on the other side, the chasm was behind them.\n\n\
         The doubters were not silent. 'This is not what we imagined,' they \
         said. 'The costs are not accounted for. The risks are underestimated.' \
         They were not wrong. But neither were they fully right — because \
         {topic}, like all things that matter, refused to stay inside the \
         boundaries of any one perspective.\n\n\
         Years later, when historians would try to identify the turning point, \
         they would argue about the date. But those who had lived through it \
         knew: the turning point was not an event. It was a gradual recognition, \
         arriving differently for each person, that the world shaped by {topic} \
         was the only world there was now — and the only question remaining \
         was what kind of people they intended to be inside it.",
        topic = topic
    )
}

fn prose_formal(topic: &str) -> String {
    let domain = topic_domain(topic);
    let (transition, evidence, impact) = domain_vocab(domain);

    format!(
        "Executive Analysis: {topic}\n\n\
         Prepared for: General Distribution\n\
         Classification: Informational\n\n\
         1. Executive Summary\n\n\
         This analysis examines {topic} with reference to prevailing \
         knowledge, available evidence, and recognised best practices. \
         The purpose is to provide a clear, unbiased assessment suitable \
         for informed decision-making across relevant stakeholder groups.\n\n\
         2. Background and Context\n\n\
         {topic} has emerged as a subject of considerable significance \
         within the {domain} domain. {evidence} that engagement with \
         {topic} at the institutional level correlates with measurably \
         improved outcomes across multiple performance dimensions. \
         Precedent from comparable contexts further supports the strategic \
         importance of this subject.\n\n\
         3. Key Findings\n\n\
         {transition} evidence-based examination reveals that the {impact} \
         of {topic} manifests across operational, strategic, and reputational \
         dimensions. Organisations that have engaged proactively with {topic} \
         demonstrate statistically superior performance relative to comparator \
         groups. Conversely, those that have delayed engagement report \
         increasing difficulty in course-correcting at a later stage.\n\n\
         4. Recommendations\n\n\
         Based on the foregoing analysis, the following courses of action \
         are recommended:\n\
           (a) Conduct a structured internal assessment of current engagement \
               with {topic} relative to sector benchmarks.\n\
           (b) Establish clear accountability for {topic}-related decisions \
               at the senior leadership level.\n\
           (c) Allocate appropriate resources for capability-building and \
               ongoing monitoring.\n\n\
         5. Conclusion\n\n\
         {topic} represents both a risk and an opportunity. The distinction \
         between the two outcomes rests principally on the quality, timeliness, \
         and strategic coherence of the response. Prompt, informed action is \
         strongly advised.",
        topic = topic, domain = domain,
        transition = transition, evidence = evidence, impact = impact
    )
}

fn prose_casual(topic: &str) -> String {
    format!(
        "Okay, so let's talk about {topic} — because honestly, \
         it's one of those things that sounds complicated but \
         once you actually dig into it, it starts to make a lot of sense.\n\n\
         The short version: {topic} is kind of a big deal, and more people \
         are starting to realise that. Whether you're coming at it from a \
         technical angle, a philosophical one, or just pure curiosity, \
         there's something here for you.\n\n\
         Here's what I find interesting: {topic} doesn't stay neatly inside \
         one box. You'll find it popping up in conversations about technology, \
         culture, work, education — basically anywhere humans are trying to \
         figure out how to do things better (or just... differently). That \
         cross-cutting quality is worth paying attention to.\n\n\
         The thing people get wrong about {topic} is that they either think \
         it's all hype, or they think it'll solve everything. Neither is true. \
         It's a tool — a powerful one — and like most powerful tools, it depends \
         enormously on who's holding it and what they're trying to build.\n\n\
         So yeah. {topic}: complicated enough to be worth your time, \
         approachable enough that you don't need a PhD to have an opinion. \
         Start learning, stay curious, and don't let anyone tell you \
         it's above your pay grade.",
        topic = topic
    )
}

fn prose_explain(topic: &str) -> String {
    format!(
        "What is {topic}?\n\n\
         Put simply: {topic} is a concept, system, or phenomenon that plays \
         an important role in how we understand and interact with the world. \
         To explain it clearly, let's break it down into its essential parts.\n\n\
         The Core Idea\n\n\
         At its most fundamental level, {topic} involves the relationship \
         between inputs — the resources, information, or effort that go in — \
         and outputs — the results, effects, or changes that come out. \
         What makes {topic} distinctive is the nature of the transformation \
         it performs between those two ends.\n\n\
         Why it Matters\n\n\
         {topic} matters for a straightforward reason: it changes what is \
         possible. Before {topic} existed (or was understood), certain \
         problems were intractable, certain goals were unreachable, and \
         certain connections remained unseen. {topic} altered that equation — \
         sometimes gradually, sometimes suddenly.\n\n\
         How to Think About It\n\n\
         A useful mental model: think of {topic} as a lens. A lens does not \
         create what it reveals — the underlying reality was always there. \
         But without the lens, that reality remains blurred or invisible. \
         {topic} sharpens the picture in a specific domain, allowing those \
         who understand it to see — and therefore act — with greater clarity \
         than those who do not.\n\n\
         The Limits\n\n\
         No explanation of {topic} is complete without acknowledging its \
         limits. It is not universal. It is not neutral. Applied without \
         wisdom, any powerful concept — including {topic} — can produce \
         outcomes far removed from those intended. Understanding the limits \
         is as important as understanding the capabilities.",
        topic = topic
    )
}

