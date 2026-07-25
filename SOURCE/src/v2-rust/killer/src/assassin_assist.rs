// ===============================================================================
// Nova Galaxy Engine v1 — AI Assassin Assist Layer
// ===============================================================================
//
// The Assassin Assist Layer wraps every @lang{} polyglot execution with:
//
//  +--------------------------------------------------------------+
//  |                   AI ASSASSIN ASSIST LAYER                   |
//  |                                                              |
//  |  ① Token Budget Guard  — blocks call if budget exceeded      |
//  |  ② Execution Logger    — ring-buffer of last 200 results     |
//  |  ③ Exception Handler   — catches stderr / non-zero exit      |
//  |  ④ Auto-Debugger       — KhLM/LLM explains the error         |
//  |  ⑤ Optimizer Hints     — on-demand code improvement tips     |
//  |  ⑥ Result Aggregator   — stats: tokens, cost, by-lang        |
//  |  ⑦ Unified Log Writer  — optional disk log for all sessions  |
//  +--------------------------------------------------------------+
//
// Cost model (usd per 1M output tokens — pessimistic/rounded up):
//   Ollama local   →  $0.000   (free)
//   KhLM-Tier1     →  $0.000   (deterministic, no API)
//   KhLM-Tier2     →  $0.000   (knowledge base, no API)
//   Groq free      →  $0.000   (free tier)
//   GPT-4o-mini    →  $0.600
//   Claude-Haiku   →  $1.250
//
// Token estimation: 1 token ≈ 4 characters (GPT-4 calibration)

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::fs::OpenOptions;
use std::io::Write;

// --- Global Singleton ---------------------------------------------------------

static LAYER: OnceLock<Mutex<AssassinAssistLayer>> = OnceLock::new();

pub fn layer() -> &'static Mutex<AssassinAssistLayer> {
    LAYER.get_or_init(|| Mutex::new(AssassinAssistLayer::new()))
}

// --- Cost Table ---------------------------------------------------------------

/// Cost per 1M output tokens in USD cents (integer arithmetic, no floats in table)
#[allow(dead_code)]
struct CostEntry { per_1m_usd_cents: u64 }

fn ai_cost_cents_per_1m(backend: &str) -> u64 {
    match backend.to_lowercase().as_str() {
        "ollama"    => 0,
        "khlm"      => 0,
        "groq"      => 0,
        "openai"    => 60,   // GPT-4o-mini: $0.60 / 1M
        "anthropic" => 125,  // Claude Haiku: $1.25 / 1M
        _           => 0,
    }
}

// --- Types --------------------------------------------------------------------

/// Severity of an execution event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity { Info, Warn, Error, Critical }

impl Severity {
    fn label(&self) -> &'static str {
        match self {
            Severity::Info     => "INFO",
            Severity::Warn     => "WARN",
            Severity::Error    => "ERR ",
            Severity::Critical => "CRIT",
        }
    }
}

/// One execution record stored in the ring buffer.
#[derive(Debug, Clone)]
pub struct AssistEntry {
    /// Auto-incrementing ID
    pub id: u64,
    /// Unix timestamp seconds
    pub timestamp: u64,
    /// Language: "python", "go", "powershell", …
    pub lang: String,
    /// First 120 chars of the code (for display)
    pub code_preview: String,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr (if any)
    pub stderr: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Execution wall-clock ms
    pub duration_ms: u128,
    /// Severity based on exit_code / stderr
    pub severity: Severity,
    /// AI-generated debug/suggestion (populated only when error + AI enabled)
    pub ai_suggestion: Option<String>,
    /// LLM backend used for this AI call (if any)
    pub ai_backend: Option<String>,
    /// Tokens consumed by the AI call
    pub tokens_used: u64,
    /// Cost in USD (fractional, can be 0.0 if local/free)
    pub cost_usd: f64,
}

/// Aggregated statistics across all executions this session.
#[derive(Debug, Clone, Default)]
pub struct AssistStats {
    pub total_executions: u64,
    pub total_errors:     u64,
    pub total_warnings:   u64,
    pub total_tokens:     u64,
    pub total_cost_usd:   f64,
    /// Executions by language
    pub by_lang: HashMap<String, u64>,
    /// Errors by language
    pub errors_by_lang: HashMap<String, u64>,
    /// Average duration ms
    pub avg_duration_ms: f64,
    sum_duration_ms: f64,
}

impl AssistStats {
    fn record(&mut self, entry: &AssistEntry) {
        self.total_executions += 1;
        if entry.exit_code != 0 { self.total_errors += 1; }
        if matches!(entry.severity, Severity::Warn) { self.total_warnings += 1; }
        self.total_tokens += entry.tokens_used;
        self.total_cost_usd += entry.cost_usd;
        *self.by_lang.entry(entry.lang.clone()).or_insert(0) += 1;
        if entry.exit_code != 0 {
            *self.errors_by_lang.entry(entry.lang.clone()).or_insert(0) += 1;
        }
        self.sum_duration_ms += entry.duration_ms as f64;
        self.avg_duration_ms = self.sum_duration_ms / self.total_executions as f64;
    }
}

/// AI model tier used for a particular call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AiTier {
    /// No AI — just log
    Off,
    /// KhLM Tier 1: pure deterministic (zero token cost)
    KhLmDeterministic,
    /// KhLM Tier 2: knowledge base Q&A
    KhLmKnowledge,
    /// Groq or Ollama (free)
    LocalFree,
    /// Remote paid (OpenAI, Anthropic)
    Remote,
}

// --- Core Layer ---------------------------------------------------------------

pub struct AssassinAssistLayer {
    /// Master on/off switch
    pub enabled: bool,

    /// AI assist tier used for auto-debugging
    pub ai_tier: AiTier,

    /// Max tokens allowed per session before AI calls are blocked
    pub token_budget: u64,
    tokens_used_total: u64,

    /// Ring buffer of execution results (capped at max_log_entries)
    pub log: VecDeque<AssistEntry>,
    pub max_log_entries: usize,

    /// Auto-increment counter for entry IDs
    next_id: u64,

    /// Aggregated statistics
    pub stats: AssistStats,

    /// Suggestion cache: (lang + code_hash) → suggestion text
    suggestion_cache: HashMap<u64, String>,

    /// Optional disk log path  (None = memory only)
    pub disk_log_path: Option<String>,

    /// Model path used for Ollama / GGUF inference (empty = skip neural inference)
    pub local_model_path: String,
}

impl AssassinAssistLayer {
    pub fn new() -> Self {
        AssassinAssistLayer {
            enabled:           true,
            ai_tier:           AiTier::KhLmDeterministic,
            token_budget:      100_000,
            tokens_used_total: 0,
            log:               VecDeque::new(),
            max_log_entries:   200,
            next_id:           1,
            stats:             AssistStats::default(),
            suggestion_cache:  HashMap::new(),
            disk_log_path:     None,
            local_model_path:  String::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }

    /// Check if token budget allows an AI call of `estimated_tokens`.
    fn budget_ok(&self, estimated_tokens: u64) -> bool {
        self.tokens_used_total + estimated_tokens <= self.token_budget
    }

    /// Simple token estimator: ~4 chars per token (GPT-4 calibration).
    pub fn estimate_tokens(text: &str) -> u64 {
        ((text.len() as u64) + 3) / 4
    }

    /// Hash a (lang, code) pair quickly for cache keying.
    fn code_hash(lang: &str, code: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in lang.bytes().chain(code.bytes()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Push to ring buffer; evict oldest if over capacity.
    fn push_log(&mut self, entry: AssistEntry) {
        if self.log.len() >= self.max_log_entries {
            self.log.pop_front();
        }
        self.stats.record(&entry);
        self.log.push_back(entry);
    }

    /// Optionally append to disk log file.
    fn write_disk_log(&self, entry: &AssistEntry) {
        let path = match &self.disk_log_path {
            Some(p) => p.clone(),
            None    => return,
        };
        let line = format!(
            "[{}] id={} lang={} exit={} ms={} tokens={} cost=${:.6} sev={}\n  code: {}\n  out:  {}\n  err:  {}\n  ai:   {}\n\n",
            entry.timestamp, entry.id, entry.lang, entry.exit_code,
            entry.duration_ms, entry.tokens_used, entry.cost_usd,
            entry.severity.label(),
            truncate(&entry.code_preview, 80),
            truncate(&entry.stdout, 120),
            truncate(&entry.stderr, 120),
            entry.ai_suggestion.as_deref().map(|s| truncate(s, 200)).unwrap_or("-"),
        );
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(line.as_bytes());
        }
    }

    // -- AI Suggestion Engine --------------------------------------------------

    /// Debug an error using the KhLM-Polyglot 5-tier router.
    /// Returns (suggestion_text, backend_name, tokens_used).
    fn ai_debug_error(
        &mut self,
        lang: &str,
        code: &str,
        stderr: &str,
        exit_code: i32,
    ) -> (String, String, u64) {
        if self.ai_tier == AiTier::Off { return (String::new(), "off".into(), 0); }

        let key = Self::code_hash(lang, &format!("{}:{}:{}", lang, code, stderr));
        if let Some(cached) = self.suggestion_cache.get(&key) {
            return (cached.clone(), "cache".into(), 0);
        }

        // Budget check (estimate based on prompt size)
        let estimated_tokens = (code.len() as u64 + stderr.len() as u64) / 4 * 3;
        if !self.budget_ok(estimated_tokens) {
            return (
                format!("⚠ Token budget exhausted ({}/{}). AI debug skipped.",
                    self.tokens_used_total, self.token_budget),
                "budget".into(), 0
            );
        }

        // Route through KhLM-Polyglot 5-tier: CAG → LLM → RLM → Ghost-108
        let error_ctx = format!("exit code {}\n{}", exit_code, truncate(stderr, 400));
        let (answer, tier_name, _ms) = crate::khlm_polyglot::khlm_polyglot_ask(
            "debug", lang, truncate(code, 600), &error_ctx
        );

        let tokens = Self::estimate_tokens(&answer) + estimated_tokens;
        self.tokens_used_total += tokens;

        // Infer cost from tier name
        let backend_key = if tier_name.contains("openai") { "openai" }
            else if tier_name.contains("anthropic") { "anthropic" }
            else if tier_name.contains("groq") { "groq" }
            else { "khlm" };
        let _cost_usd = (tokens as f64 / 1_000_000.0)
            * (ai_cost_cents_per_1m(backend_key) as f64 / 100.0);

        self.suggestion_cache.insert(key, answer.clone());
        (answer, tier_name.to_string(), tokens)
    }

    /// Ask KhLM-Polyglot for optimization suggestions (5-tier routed).
    pub fn ai_optimize(
        &mut self,
        lang: &str,
        code: &str,
    ) -> (String, String, u64) {
        if self.ai_tier == AiTier::Off { return (String::new(), "off".into(), 0); }

        let key = Self::code_hash(lang, &format!("opt:{}:{}", lang, code));
        if let Some(cached) = self.suggestion_cache.get(&key) {
            return (cached.clone(), "cache".into(), 0);
        }

        let estimated = (code.len() as u64 / 4) * 3;
        if !self.budget_ok(estimated) {
            return ("⚠ Token budget exhausted. Optimization skipped.".into(), "budget".into(), 0);
        }

        // Route through KhLM-Polyglot 5-tier
        let (answer, tier_name, _ms) = crate::khlm_polyglot::khlm_polyglot_ask(
            "suggest", lang, truncate(code, 800), ""
        );

        let tokens = Self::estimate_tokens(&answer) + estimated;
        self.tokens_used_total += tokens;
        self.suggestion_cache.insert(key, answer.clone());
        (answer, tier_name.to_string(), tokens)
    }

    // -- Main Interception Point -----------------------------------------------

    /// Called by `polyglot_exec_assisted()` in polyglot.rs.
    /// Wraps a successful/failed execution result:
    ///   - logs it
    ///   - if error → triggers AI auto-debug (if enabled)
    ///   - returns (enriched_stdout, ai_suggestion_text)
    pub fn process_execution(
        &mut self,
        lang: &str,
        code: &str,
        result: &Result<String, String>,
        duration_ms: u128,
    ) -> (String, Option<String>) {
        let (stdout, stderr, exit_code, severity) = match result {
            Ok(out) => (out.clone(), String::new(), 0i32, Severity::Info),
            Err(e)  => {
                // Try to extract exit code from error message
                let code_num = extract_exit_code(e).unwrap_or(1);
                let sev = if e.to_lowercase().contains("warn") {
                    Severity::Warn
                } else if code_num != 0 {
                    Severity::Error
                } else {
                    Severity::Warn
                };
                (String::new(), e.clone(), code_num, sev)
            }
        };

        let mut ai_suggestion = None;
        let mut ai_backend    = None;
        let mut tokens_used   = 0u64;
        let mut cost_usd      = 0.0f64;

        // Auto-debug on error (if AI enabled and budget available)
        if exit_code != 0 && self.enabled && self.ai_tier != AiTier::Off {
            let (sug, backend, tok) = self.ai_debug_error(lang, code, &stderr, exit_code);
            if !sug.is_empty() && sug != "off" {
                let backend_key = if backend.to_lowercase().contains("openai") { "openai" }
                    else if backend.to_lowercase().contains("anthropic") { "anthropic" }
                    else { "khlm" };
                let c = (tok as f64 / 1_000_000.0)
                    * (ai_cost_cents_per_1m(backend_key) as f64 / 100.0);
                ai_suggestion = Some(sug);
                ai_backend    = Some(backend);
                tokens_used   = tok;
                cost_usd      = c;
            }
        }

        let id = self.next_id();
        let entry = AssistEntry {
            id,
            timestamp:    Self::now_secs(),
            lang:         lang.to_string(),
            code_preview: truncate(code, 120).to_string(),
            stdout:       stdout.clone(),
            stderr:       stderr.clone(),
            exit_code,
            duration_ms,
            severity,
            ai_suggestion: ai_suggestion.clone(),
            ai_backend,
            tokens_used,
            cost_usd,
        };

        self.write_disk_log(&entry);
        self.push_log(entry);

        (stdout, ai_suggestion)
    }
}

// --- Helper Functions ---------------------------------------------------------

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s }
    else { &s[..s.floor_char_boundary(max)] }
}

fn extract_exit_code(err: &str) -> Option<i32> {
    // Look for "status: exit code N" or "exit code N" in error string
    for part in err.split_whitespace() {
        if let Ok(n) = part.trim_end_matches(|c: char| !c.is_numeric()).parse::<i32>() {
            if n != 0 { return Some(n); }
        }
    }
    None
}

// --- Public Builtin API (called from builtin.rs) ------------------------------

/// `nova_assist_log()` → print last N execution logs as formatted string.
pub fn builtin_assist_log(args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    let n: usize = if args.is_empty() {
        10
    } else {
        match &args[0] {
            crate::value::Value::Number(i) => (*i as i64).max(1) as usize,
            _                             => 10,
        }
    };

    let layer = layer().lock().unwrap();
    if layer.log.is_empty() {
        return Ok(crate::value::Value::Str("No executions logged yet.".into()));
    }

    let start = layer.log.len().saturating_sub(n);
    let mut out = String::from("Nova Assassin Assist -- Execution Log\n");
    for entry in layer.log.range(start..) {
        let status = if entry.exit_code == 0 { "OK" } else { "ERR" };
        out.push_str(&format!(
            "  [{}] #{} @{} {}  {}ms  tok={}\n",
            fmt_timestamp(entry.timestamp),
            entry.id,
            entry.lang,
            status,
            entry.duration_ms,
            entry.tokens_used,
        ));
        if !entry.stdout.is_empty() {
            out.push_str(&format!("    out: {}\n", truncate(&entry.stdout, 80)));
        }
        if !entry.stderr.is_empty() {
            out.push_str(&format!("    err: {}\n", truncate(&entry.stderr, 80)));
        }
        if let Some(sug) = &entry.ai_suggestion {
            out.push_str(&format!("    AI: {}\n", truncate(sug, 120)));
        }
    }
    Ok(crate::value::Value::Str(out))
}

/// `nova_assist_status()` → print aggregate stats.
pub fn builtin_assist_status(_args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    let layer = layer().lock().unwrap();
    let s = &layer.stats;

    let mut by_lang = s.by_lang.iter().collect::<Vec<_>>();
    by_lang.sort_by_key(|(k, _)| k.as_str());

    let mut out = String::from("Nova Assassin Assist -- Status\n");
    out.push_str(&format!("  AI Layer    : {}\n", if layer.enabled { "ENABLED" } else { "DISABLED" }));
    out.push_str(&format!("  AI Tier     : {:?}\n", layer.ai_tier));
    out.push_str(&format!("  Token Budget: {}/{}\n", layer.tokens_used_total, layer.token_budget));
    out.push_str(&format!("  Executions  : {}   Errors: {}   Warnings: {}\n",
        s.total_executions, s.total_errors, s.total_warnings));
    out.push_str(&format!("  Avg Duration: {:.1}ms\n", s.avg_duration_ms));
    out.push_str(&format!("  Total Tokens: {}   Cost: ${:.6}\n", s.total_tokens, s.total_cost_usd));
    out.push_str("  By Language:\n");
    for (lang, count) in &by_lang {
        let errs = s.errors_by_lang.get(*lang).copied().unwrap_or(0);
        out.push_str(&format!("    @{} runs={} errors={}\n", lang, count, errs));
    }
    Ok(crate::value::Value::Str(out))
}

/// `nova_assist_debug(code, lang)` → on-demand AI debug of given code.
pub fn builtin_assist_debug(args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    let (code, lang) = match args {
        [crate::value::Value::Str(c), crate::value::Value::Str(l)] => (c.clone(), l.clone()),
        [crate::value::Value::Str(c)] => (c.clone(), "unknown".to_string()),
        _ => return Err(crate::error::VmError::runtime_error(
            "nova_assist_debug(code, lang) expects 1-2 String arguments"
        )),
    };

    // Route through KhLM-Polyglot 5-tier router (CAG → LLM → RLM → Ghost-108)
    let (answer, tier_name, _ms) = crate::khlm_polyglot::khlm_polyglot_ask(
        "debug", &lang, truncate(&code, 800), ""
    );

    let mut lyr = layer().lock().unwrap();
    let tokens = AssassinAssistLayer::estimate_tokens(&answer);
    lyr.tokens_used_total += tokens;
    let _ = tier_name;

    Ok(crate::value::Value::Str(answer))
}

/// `nova_assist_optimize(code, lang)` → on-demand optimization suggestions.
pub fn builtin_assist_optimize(args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    let (code, lang) = match args {
        [crate::value::Value::Str(c), crate::value::Value::Str(l)] => (c.clone(), l.clone()),
        [crate::value::Value::Str(c)] => (c.clone(), "unknown".to_string()),
        _ => return Err(crate::error::VmError::runtime_error(
            "nova_assist_optimize(code, lang) expects 1-2 String arguments"
        )),
    };

    let mut lyr = layer().lock().unwrap();
    let (result, _backend, _tok) = lyr.ai_optimize(&lang, &code);
    Ok(crate::value::Value::Str(result))
}

/// `nova_assist_enable()` → turn on AI layer.
pub fn builtin_assist_enable(_args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    layer().lock().unwrap().enabled = true;
    Ok(crate::value::Value::Str("Nova Assassin Assist: ENABLED".into()))
}

/// `nova_assist_disable()` → turn off AI layer (still logs, no AI calls).
pub fn builtin_assist_disable(_args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    layer().lock().unwrap().enabled = false;
    Ok(crate::value::Value::Str("Nova Assassin Assist: DISABLED (logging still active)".into()))
}

/// `nova_assist_set_budget(n)` → set token budget for this session.
pub fn builtin_assist_set_budget(args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    match args.first() {
        Some(crate::value::Value::Number(n)) => {
            let n = (*n as i64).max(0) as u64;
            layer().lock().unwrap().token_budget = n;
            Ok(crate::value::Value::Str(format!("Token budget set to {}", n)))
        }
        _ => Err(crate::error::VmError::runtime_error(
            "nova_assist_set_budget(n) expects an Int"
        )),
    }
}

/// `nova_assist_set_log(path)` → enable disk logging to a file.
pub fn builtin_assist_set_log(args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    match args.first() {
        Some(crate::value::Value::Str(path)) => {
            let p = path.clone();
            layer().lock().unwrap().disk_log_path = Some(p.clone());
            Ok(crate::value::Value::Str(format!("Disk log enabled → {}", p)))
        }
        _ => Err(crate::error::VmError::runtime_error(
            "nova_assist_set_log(path) expects a String path"
        )),
    }
}

/// `nova_assist_clear()` → clear the execution log and stats.
pub fn builtin_assist_clear(_args: &[crate::value::Value]) -> Result<crate::value::Value, crate::error::VmError> {
    let mut lyr = layer().lock().unwrap();
    lyr.log.clear();
    lyr.stats = AssistStats::default();
    lyr.suggestion_cache.clear();
    lyr.tokens_used_total = 0;
    Ok(crate::value::Value::Str("Assassin Assist log cleared.".into()))
}

// --- Timestamp Formatter ------------------------------------------------------

fn fmt_timestamp(ts: u64) -> String {
    // Simple human-readable: seconds-since-epoch formatted as HH:MM:SS (UTC approx)
    let secs = ts % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
