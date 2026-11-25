//! `@lang { ... }` polyglot blocks: run a block of another language with an interpreter or compiler
//! that is installed on this machine, and return what it printed on stdout.
//!
//! What this is: a subprocess runner. The block text is written to a file in a fresh private
//! temporary directory and the installed runtime (`python3`, `bash`, `javac` + `java`, `rustc`,
//! ...) runs it. Nothing is transpiled and no foreign code runs inside the Killer VM.
//!
//! What every run gets (see docs/OPERATIONS.md for the variables and docs/project/THREAT_MODEL.md
//! for what this does and does not protect):
//!   * the runtime is resolved once to an absolute path (`tool_path`: absolute PATH entries only,
//!     never the current directory; `KILLER_POLYGLOT_<LANG>` overrides, absolute paths only);
//!   * a policy check before anything is probed or started (process capability,
//!     `KILLER_POLYGLOT_ALLOW`, `KILLER_PROFILE=strict`);
//!   * a scrubbed environment (a short allowlist plus `KILLER_POLYGLOT_ENV_PASS`) and a private
//!     working directory that is removed afterwards, also on errors and timeouts;
//!   * a wall-clock timeout and an output cap; when either is hit the whole process tree is killed;
//!   * errors with stable `POLYGLOT_E_*` codes that carry the exit code.
//!
//! This is hygiene, not a sandbox: the child runs with the OS account's full permissions.
//!
//! Ideas, not built: transpiling Python / JS to Killer bytecode, a multi-language VM.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::error::VmError;
use crate::value::Value;

// --- Language table ----------------------------------------------------------------------------

/// How a language is run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `exe [pre_args] <file>`
    Script,
    /// `javac` then `java -cp <dir> KillerMain`
    Java,
    /// `rustc <file> -o <bin>` then `<bin>`
    Rust,
    /// `gcc|clang|cc <file> -o <bin>` then `<bin>`
    C,
    /// `g++|clang++|c++ <file> -o <bin> -std=c++17` then `<bin>`
    Cpp,
}

/// Describes how to run one language.
#[derive(Debug, Clone)]
pub struct LangRuntime {
    /// Canonical name (`@python`).
    pub name: &'static str,
    /// Other names for the same language (`@python3`, `@js`, `@ps`, `@c++`).
    pub aliases: &'static [&'static str],
    /// Candidate programs, tried in order; the first that is found and passes the probe wins.
    pub executables: &'static [&'static str],
    /// File extension of the block file.
    pub ext: &'static str,
    /// Arguments before the block file (`go run <file>`).
    pub pre_args: &'static [&'static str],
    /// Probe: the runtime counts as installed when these arguments exit 0. The first output line
    /// is reported as its version.
    pub detect_args: &'static [&'static str],
    pub kind: Kind,
}

const fn script(name: &'static str, aliases: &'static [&'static str], executables: &'static [&'static str], ext: &'static str, pre_args: &'static [&'static str], detect_args: &'static [&'static str]) -> LangRuntime {
    LangRuntime { name, aliases, executables, ext, pre_args, detect_args, kind: Kind::Script }
}

const PS_FLAGS: &[&str] = &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"];
const PS_DETECT: &[&str] = &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", "$PSVersionTable.PSVersion.ToString()"];

/// Every supported language.
pub static RUNTIMES: &[LangRuntime] = &[
    script("python", &["python3"], &["python3", "python", "py"], "py", &[], &["--version"]),
    script("node", &["js"], &["node", "nodejs"], "js", &[], &["--version"]),
    script("ts", &["typescript"], &["ts-node"], "ts", &[], &["--version"]),
    script("ruby", &[], &["ruby", "ruby3"], "rb", &[], &["--version"]),
    script("perl", &[], &["perl"], "pl", &[], &["--version"]),
    script("php", &[], &["php", "php8", "php7"], "php", &[], &["--version"]),
    script("lua", &[], &["lua", "lua5.4", "lua5.3"], "lua", &[], &["-v"]),
    script("r", &[], &["Rscript"], "R", &[], &["--version"]),
    script("go", &[], &["go"], "go", &["run"], &["version"]),
    script("bash", &[], &["bash"], "sh", &[], &["-c", "echo $BASH_VERSION"]),
    script("sh", &[], &["sh"], "sh", &[], &["-c", "exit 0"]),
    script("powershell", &["ps", "pwsh"], &["powershell", "pwsh"], "ps1", PS_FLAGS, PS_DETECT),
    script("swift", &[], &["swift"], "swift", &[], &["--version"]),
    script("kotlin", &[], &["kotlin", "kotlinc-jvm"], "kts", &["-script"], &["-version"]),
    script("scala", &[], &["scala", "scala3"], "sc", &[], &["-version"]),
    script("elixir", &[], &["elixir"], "exs", &[], &["--version"]),
    script("haskell", &[], &["runghc", "runhaskell"], "hs", &[], &["--version"]),
    script("zig", &[], &["zig"], "zig", &["run"], &["version"]),
    LangRuntime { name: "java", aliases: &[], executables: &["java"], ext: "java", pre_args: &[], detect_args: &["-version"], kind: Kind::Java },
    LangRuntime { name: "rust", aliases: &[], executables: &["rustc"], ext: "rs", pre_args: &[], detect_args: &["--version"], kind: Kind::Rust },
    LangRuntime { name: "c", aliases: &[], executables: &["gcc", "clang", "cc"], ext: "c", pre_args: &[], detect_args: &["--version"], kind: Kind::C },
    LangRuntime { name: "cpp", aliases: &["c++"], executables: &["g++", "clang++", "c++"], ext: "cpp", pre_args: &[], detect_args: &["--version"], kind: Kind::Cpp },
];

/// Find the runtime for a language name or alias (case-insensitive).
pub fn find_runtime(lang: &str) -> Option<&'static LangRuntime> {
    let l = lang.trim().to_lowercase();
    RUNTIMES.iter().find(|r| r.name == l || r.aliases.contains(&l.as_str()))
}

/// Is `lang` a language name a block can use (`@python`, `@java`, ...)?
pub fn is_known_lang(lang: &str) -> bool {
    find_runtime(lang).is_some()
}

/// `python (python3), node (js), ...`: generated from [`RUNTIMES`].
pub fn supported_list() -> String {
    RUNTIMES
        .iter()
        .map(|r| if r.aliases.is_empty() { r.name.to_string() } else { format!("{} ({})", r.name, r.aliases.join(", ")) })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Short override names kept for convenience next to `KILLER_POLYGLOT_<LANG>`.
const SHORT_OVERRIDES: &[&str] = &["python", "node", "bash", "sh", "ruby", "perl", "php", "lua"];

/// The environment variables that override the program for `key` (a language name, or `javac`).
pub fn override_vars(key: &str) -> Vec<String> {
    let up = key.to_uppercase().replace('+', "P");
    let mut v = vec![format!("KILLER_POLYGLOT_{}", up)];
    if SHORT_OVERRIDES.contains(&key) {
        v.push(format!("KILLER_{}", up));
    }
    v
}

// --- Errors ------------------------------------------------------------------------------------

/// Everything that can go wrong with a block. `Display` starts with a stable code.
#[derive(Debug, Clone, PartialEq)]
pub enum PolyglotError {
    UnknownLang(String),
    RuntimeMissing { lang: String, detail: String },
    /// `tree_killed`: every process of the block is known to be dead (see `kill_tree`).
    Timeout { lang: String, secs: f64, tree_killed: bool },
    /// The block (or its compile step, `stage == "compile"`) exited with a non-zero code
    /// (`code == None`: killed by a signal).
    NonZeroExit { lang: String, stage: &'static str, code: Option<i32>, stderr: String, stdout: String },
    OutputTooLarge { lang: String, limit: usize, tree_killed: bool },
    PolicyDenied { lang: String, reason: String },
    Spawn { lang: String, detail: String },
}

/// Longest stderr / stdout excerpt carried in an error message.
pub const SNIPPET_BYTES: usize = 2048;

/// The last `max` bytes of `s` (on a char boundary), marked when cut.
pub fn snippet(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        return t.to_string();
    }
    let mut start = t.len() - max;
    while !t.is_char_boundary(start) {
        start += 1;
    }
    format!("...({} bytes cut)\n{}", start, &t[start..])
}

impl PolyglotError {
    /// The stable code (`POLYGLOT_E_TIMEOUT`, ...).
    pub fn code(&self) -> &'static str {
        match self {
            PolyglotError::UnknownLang(_) => "POLYGLOT_E_UNKNOWN_LANG",
            PolyglotError::RuntimeMissing { .. } => "POLYGLOT_E_RUNTIME_MISSING",
            PolyglotError::Timeout { .. } => "POLYGLOT_E_TIMEOUT",
            PolyglotError::NonZeroExit { .. } => "POLYGLOT_E_EXIT",
            PolyglotError::OutputTooLarge { .. } => "POLYGLOT_E_OUTPUT_LIMIT",
            PolyglotError::PolicyDenied { .. } => "POLYGLOT_E_POLICY_DENIED",
            PolyglotError::Spawn { .. } => "POLYGLOT_E_SPAWN",
        }
    }
}

/// What happened to the block's processes after a limit was hit.
fn killed_text(tree_killed: bool) -> &'static str {
    if tree_killed {
        "its process tree was killed"
    } else {
        "the block was stopped, but its child processes could not all be killed and may still be running"
    }
}

impl std::fmt::Display for PolyglotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            PolyglotError::UnknownLang(l) => write!(f, "{code}: Unknown language: '{l}'. Supported: {}. Check what is installed with: polyglot_list() or `killer polyglot list`", supported_list()),
            PolyglotError::RuntimeMissing { lang, detail } => write!(
                f,
                "{code}: {lang} runtime not found on this machine: {detail}. Install it, or set {} to its absolute path; check what is installed with `killer polyglot list`",
                override_vars(lang)[0]
            ),
            PolyglotError::Timeout { lang, secs, tree_killed } => write!(f, "{code}: @{lang} timed out after {secs} s and {} (raise KILLER_POLYGLOT_TIMEOUT)", killed_text(*tree_killed)),
            PolyglotError::NonZeroExit { lang, stage, code: exit, stderr, stdout } => {
                let what = if stage.is_empty() { format!("@{lang}") } else { format!("@{lang} {stage} step") };
                match exit {
                    Some(c) => write!(f, "{code}: {what} failed with exit code {c}")?,
                    None => write!(f, "{code}: {what} was killed by a signal")?,
                }
                if !stderr.trim().is_empty() {
                    write!(f, ":\n{}", snippet(stderr, SNIPPET_BYTES))?;
                }
                if !stdout.trim().is_empty() {
                    write!(f, "\nOutput before error:\n{}", snippet(stdout, SNIPPET_BYTES / 2))?;
                }
                Ok(())
            }
            PolyglotError::OutputTooLarge { lang, limit, tree_killed } => write!(f, "{code}: @{lang} output limit exceeded ({limit} bytes) and {} (raise KILLER_POLYGLOT_MAX_OUTPUT)", killed_text(*tree_killed)),
            PolyglotError::PolicyDenied { lang, reason } => write!(f, "{code}: @{lang} is not allowed: {reason}"),
            PolyglotError::Spawn { lang, detail } => write!(f, "{code}: could not start @{lang}: {detail}"),
        }
    }
}

// --- Settings (read once from the environment snapshot) ----------------------------------------

/// `KILLER_POLYGLOT_ALLOW`.
#[derive(Debug, Clone, PartialEq)]
pub enum Allow {
    /// Not set: the profile decides.
    Unset,
    /// `all` / `*`.
    All,
    /// A list of canonical language names (`none` = empty).
    Only(Vec<String>),
}

/// Operator settings. Read from the environment snapshot taken by `tool_path` (the first time it is
/// needed, or just before a script calls `env_set`), so a script cannot change them.
#[derive(Debug, Clone)]
pub struct Settings {
    /// `KILLER_PROFILE=strict|prod|production`: blocks are denied unless `KILLER_POLYGLOT_ALLOW` lists them.
    pub strict: bool,
    pub allow: Allow,
    /// Run-step timeout (`None` = no limit). Compile steps get twice this.
    pub timeout: Option<Duration>,
    /// Cap on stdout + stderr bytes of one step.
    pub max_output: usize,
    /// Extra environment variable names passed to the child.
    pub env_pass: Vec<String>,
    /// `KILLER_POLYGLOT_AI=1`: failing blocks may be sent to the AI assist layer.
    pub ai: bool,
    /// `KILLER_AUDIT_LOG`: append one JSON line per run.
    pub audit_log: Option<PathBuf>,
    /// `KILLER_AUDIT_CODE=1`: include the block source in audit lines and the assist disk log.
    pub audit_code: bool,
}

pub const DEFAULT_TIMEOUT_S: u64 = 60;
pub const DEFAULT_MAX_OUTPUT: usize = 8 * 1024 * 1024;

fn truthy(v: &Option<String>) -> bool {
    matches!(v.as_deref().map(|s| s.trim().to_lowercase()).as_deref(), Some("1") | Some("true") | Some("yes") | Some("on"))
}

impl Settings {
    /// Build from a variable lookup (testable).
    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> Settings {
        let strict = matches!(get("KILLER_PROFILE").map(|s| s.trim().to_lowercase()).as_deref(), Some("strict") | Some("prod") | Some("production"));
        let allow = match get("KILLER_POLYGLOT_ALLOW") {
            None => Allow::Unset,
            Some(v) => {
                let items: Vec<String> = v.split(|c: char| c == ',' || c.is_whitespace()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
                if items.iter().any(|s| s == "all" || s == "*") {
                    Allow::All
                } else {
                    Allow::Only(items.iter().filter(|s| *s != "none").map(|s| find_runtime(s).map(|r| r.name.to_string()).unwrap_or_else(|| s.clone())).collect())
                }
            }
        };
        let timeout = match get("KILLER_POLYGLOT_TIMEOUT").and_then(|v| v.trim().parse::<f64>().ok()) {
            Some(s) if s == 0.0 => None,
            Some(s) if s > 0.0 && s.is_finite() => Some(Duration::from_secs_f64(s)),
            _ => Some(Duration::from_secs(DEFAULT_TIMEOUT_S)),
        };
        let max_output = get("KILLER_POLYGLOT_MAX_OUTPUT").and_then(|v| v.trim().parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(DEFAULT_MAX_OUTPUT);
        let env_pass = get("KILLER_POLYGLOT_ENV_PASS").map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
        Settings {
            strict,
            allow,
            timeout,
            max_output,
            env_pass,
            ai: truthy(&get("KILLER_POLYGLOT_AI")),
            audit_log: get("KILLER_AUDIT_LOG").filter(|s| !s.trim().is_empty()).map(PathBuf::from),
            audit_code: truthy(&get("KILLER_AUDIT_CODE")),
        }
    }

    fn compile_timeout(&self) -> Option<Duration> {
        self.timeout.map(|t| t * 2)
    }
}

/// The process-wide settings.
pub fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| Settings::from_lookup(&|n| crate::tool_path::var(n)))
}

// --- Policy ------------------------------------------------------------------------------------

/// Decide whether `lang` (canonical) may run. `Ok(true)` = allowed, and the one-time warning is due.
pub fn policy_decision(lang: &str, s: &Settings, process_capability: bool) -> Result<bool, PolyglotError> {
    let deny = |reason: &str| Err(PolyglotError::PolicyDenied { lang: lang.to_string(), reason: reason.to_string() });
    if !process_capability {
        return deny("the process capability is disabled in this execution context");
    }
    match &s.allow {
        Allow::All => Ok(false),
        Allow::Only(list) if list.iter().any(|l| l == lang) => Ok(false),
        Allow::Only(_) => deny("it is not listed in KILLER_POLYGLOT_ALLOW"),
        Allow::Unset if s.strict => deny("KILLER_PROFILE is strict and KILLER_POLYGLOT_ALLOW does not list it"),
        Allow::Unset => Ok(true),
    }
}

static WARNED: AtomicBool = AtomicBool::new(false);

fn check_policy(rt: &LangRuntime) -> Result<(), PolyglotError> {
    let cap = crate::security::current_capabilities().allow_process_spawn;
    if policy_decision(rt.name, settings(), cap)? && !WARNED.swap(true, Ordering::SeqCst) {
        eprintln!(
            "killer: warning: @{} runs an installed interpreter as a subprocess with this OS account's permissions (not sandboxed). Set KILLER_POLYGLOT_ALLOW=<langs> to allow explicitly and silence this, or KILLER_PROFILE=strict to deny.",
            rt.name
        );
    }
    Ok(())
}

// --- Runtime resolution ------------------------------------------------------------------------

/// A runtime program found on this machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub path: PathBuf,
    pub version: String,
}

static PROBES: AtomicUsize = AtomicUsize::new(0);

/// How many runtime probes this process has spawned (tests check that results are cached).
#[doc(hidden)]
pub fn probe_count() -> usize {
    PROBES.load(Ordering::SeqCst)
}

/// Run `path detect_args` with a short timeout; `Some(version line)` when it exits 0.
fn probe(path: &Path, args: &[&str]) -> Option<String> {
    PROBES.fetch_add(1, Ordering::SeqCst);
    let mut cmd = Command::new(path);
    cmd.args(args);
    scrub_env(&mut cmd, settings());
    cmd.current_dir(std::env::temp_dir());
    let c = run_limited(&mut cmd, Some(Duration::from_secs(30)), 64 * 1024, false).ok()?;
    if c.code != Some(0) {
        return None;
    }
    let text = format!("{}\n{}", String::from_utf8_lossy(&c.stdout), String::from_utf8_lossy(&c.stderr));
    Some(text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string())
}

fn cache() -> &'static Mutex<HashMap<String, Result<Resolved, String>>> {
    static C: OnceLock<Mutex<HashMap<String, Result<Resolved, String>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve a program once per process: an override variable (absolute path only) wins, otherwise
/// the first of `exes` found in an absolute PATH directory that passes the probe.
fn resolve_tool(key: &str, exes: &[&str], detect: &[&str]) -> Result<Resolved, String> {
    if let Some(r) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(key) {
        return r.clone();
    }
    let r = resolve_uncached(key, exes, detect);
    cache().lock().unwrap_or_else(|e| e.into_inner()).insert(key.to_string(), r.clone());
    r
}

fn resolve_uncached(key: &str, exes: &[&str], detect: &[&str]) -> Result<Resolved, String> {
    for var in override_vars(key) {
        if let Some(val) = crate::tool_path::var(&var).filter(|v| !v.trim().is_empty()) {
            let path = crate::tool_path::check_override(&var, &val)?;
            return match probe(&path, detect) {
                Some(version) => Ok(Resolved { path, version }),
                None => Err(format!("{}={} did not run `{}` successfully", var, val, detect.join(" "))),
            };
        }
    }
    for exe in exes {
        if let Some(path) = system_copy(exe).or_else(|| crate::tool_path::find_on_path(exe)) {
            if let Some(version) = probe(&path, detect) {
                return Ok(Resolved { path, version });
            }
        }
    }
    Err(format!(
        "none of {} was found in an absolute PATH directory or passed its probe (the current directory and relative PATH entries are never searched)",
        exes.join(", ")
    ))
}

/// Windows PowerShell lives under %SystemRoot%: prefer that copy.
#[cfg(windows)]
fn system_copy(exe: &str) -> Option<PathBuf> {
    if exe == "powershell" { crate::tool_path::powershell().ok() } else { None }
}
#[cfg(not(windows))]
fn system_copy(_exe: &str) -> Option<PathBuf> {
    None
}

/// Resolve the main program of a language.
pub fn resolve_runtime(rt: &LangRuntime) -> Result<Resolved, PolyglotError> {
    resolve_tool(rt.name, rt.executables, rt.detect_args).map_err(|detail| PolyglotError::RuntimeMissing { lang: rt.name.to_string(), detail })
}

fn resolve_javac() -> Result<Resolved, PolyglotError> {
    resolve_tool("javac", &["javac"], &["-version"]).map_err(|detail| PolyglotError::RuntimeMissing { lang: "java".to_string(), detail: format!("javac: {}", detail) })
}

/// Every language whose runtime resolves here: `(name, absolute path, version)`.
pub fn detect_all_runtimes() -> Vec<(String, String)> {
    RUNTIMES
        .iter()
        .filter_map(|rt| resolve_runtime(rt).ok().map(|r| (rt.name.to_string(), r.path.to_string_lossy().into_owned())))
        .collect()
}

// --- Child environment and working directory --------------------------------------------------

/// Environment variables a child keeps (when the parent has them). Everything else is removed.
pub const ENV_ALLOW: &[&str] = &[
    "PATH", "SYSTEMROOT", "SystemDrive", "TEMP", "TMP", "TMPDIR", "HOME", "USERPROFILE", "LANG", "LC_ALL", "TZ",
    // toolchain locators (no secrets): without them rustup picks another toolchain
    "RUSTUP_HOME", "CARGO_HOME", "RUSTUP_TOOLCHAIN",
];

/// Windows basics most programs need to start (no secrets).
#[cfg(windows)]
pub const ENV_ALLOW_OS: &[&str] = &[
    "WINDIR", "COMSPEC", "PATHEXT", "ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "ProgramData", "APPDATA", "LOCALAPPDATA",
    "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE", "OS", "USERNAME",
];
#[cfg(not(windows))]
pub const ENV_ALLOW_OS: &[&str] = &["USER", "LOGNAME"];

/// Clear the child environment, then copy the allowlisted variables (and `KILLER_POLYGLOT_ENV_PASS`)
/// from the startup snapshot (`tool_path`), so a script's `env_set` cannot change what a block
/// gets, such as its PATH.
fn scrub_env(cmd: &mut Command, s: &Settings) {
    cmd.env_clear();
    for name in ENV_ALLOW.iter().chain(ENV_ALLOW_OS.iter()).map(|n| n.to_string()).chain(s.env_pass.iter().cloned()) {
        if let Some(v) = crate::tool_path::var_os(&name) {
            cmd.env(&name, v);
        }
    }
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
}

static TEMP_SEQ: AtomicUsize = AtomicUsize::new(0);

/// pid + nanosecond clock + per-process counter: unique and not guessable from the pid alone.
fn unique_temp_name(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}_{}_{}_{}", prefix, std::process::id(), nanos, TEMP_SEQ.fetch_add(1, Ordering::SeqCst))
}

/// A fresh private directory (0700 on Unix) removed on drop, also after errors, timeouts and panics.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new(lang: &str) -> Result<WorkDir, PolyglotError> {
        for _ in 0..32 {
            let path = std::env::temp_dir().join(unique_temp_name("killer_nova_galaxy"));
            #[allow(unused_mut)] // only Unix sets a mode
            let mut b = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                b.mode(0o700);
            }
            match b.create(&path) {
                Ok(()) => return Ok(WorkDir(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(PolyglotError::Spawn { lang: lang.to_string(), detail: format!("cannot create a temp directory: {}", e) }),
            }
        }
        Err(PolyglotError::Spawn { lang: lang.to_string(), detail: "cannot create a unique temp directory".to_string() })
    }

    /// Create `name` (owner-only on Unix, never an existing file) holding `content`.
    fn write(&self, lang: &str, name: &str, content: &str) -> Result<PathBuf, PolyglotError> {
        let path = self.0.join(name);
        let mut o = fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&path).map_err(|e| PolyglotError::Spawn { lang: lang.to_string(), detail: format!("cannot create the block file: {}", e) })?;
        f.write_all(content.as_bytes()).map_err(|e| PolyglotError::Spawn { lang: lang.to_string(), detail: format!("cannot write the block file: {}", e) })?;
        Ok(path)
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        // a killed child may hold a file for a moment on Windows
        for _ in 0..20 {
            if fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

// --- Limited execution -------------------------------------------------------------------------

struct Captured {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// `bool` in `Timeout` / `Overflow`: the whole process tree was killed (see [`kill_tree`]).
enum LimitFail {
    Spawn(std::io::Error),
    Timeout(Duration, bool),
    Overflow(usize, bool),
}

/// Windows Job Object holding a block's process tree. Every process the child starts joins the
/// job too (no breakaway is allowed), so terminating the job kills grandchildren also when the
/// direct child has already exited, which `taskkill /T` cannot do (it walks the tree from a live
/// parent). `KILL_ON_JOB_CLOSE` also kills the tree if this process dies while a block runs.
#[cfg(windows)]
#[allow(unsafe_code)] // the kernel32 Job Object calls below
mod job {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    type Handle = *mut c_void;

    /// JOBOBJECT_BASIC_LIMIT_INFORMATION
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimit {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    /// JOBOBJECT_EXTENDED_LIMIT_INFORMATION (IO_COUNTERS is six u64)
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimit {
        basic: BasicLimit,
        io: [u64; 6],
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(job: Handle, class: i32, info: *const c_void, len: u32) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn TerminateJobObject(job: Handle, exit_code: u32) -> i32;
        fn CloseHandle(h: Handle) -> i32;
    }

    pub struct Job(Handle);

    fn set_flags(h: Handle, flags: u32) -> bool {
        let mut info = ExtendedLimit::default();
        info.basic.limit_flags = flags;
        // SAFETY: `h` is a job handle this module owns; `info` is a live, correctly laid out
        // JOBOBJECT_EXTENDED_LIMIT_INFORMATION and the length passed is its size.
        unsafe { SetInformationJobObject(h, JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &info as *const ExtendedLimit as *const c_void, std::mem::size_of::<ExtendedLimit>() as u32) != 0 }
    }

    impl Job {
        /// A new job holding `child`, or `None` when Windows refuses (the caller then falls back
        /// to `taskkill /T`). A process the child starts before it is assigned (a few
        /// microseconds after spawn) is not in the job.
        pub fn for_child(child: &std::process::Child) -> Option<Job> {
            // SAFETY: null attributes and name are allowed; the returned handle is checked.
            let h = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if h.is_null() {
                return None;
            }
            let job = Job(h);
            // SAFETY: `child.as_raw_handle()` is the child's process handle, valid while `child`
            // lives (it is borrowed for this call); `job.0` is the handle created above.
            if !set_flags(job.0, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE) || unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle() as Handle) } == 0 {
                return None; // dropping `job` closes the handle; the child is not in it
            }
            Some(job)
        }

        /// Kill every process in the job. `true` when Windows reports success.
        pub fn terminate(&self) -> bool {
            // SAFETY: `self.0` is a valid job handle owned by `self`.
            unsafe { TerminateJobObject(self.0, 1) != 0 }
        }

        /// Close the job without killing what is still running in it (the normal end of a block:
        /// background processes it started behave as on Unix, where they also survive).
        pub fn release(self) {
            set_flags(self.0, 0);
            // `drop` closes the handle
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a valid handle owned by `self` and closed exactly once, here.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    // --- Descendant walk: processes that are not in the job ---------------------------------
    //
    // A packaged runtime (the Microsoft Store python behind the WindowsApps alias) starts its own
    // children outside every job of its parent, so the job cannot reach them. They are found by
    // parent process id in a process snapshot instead. Windows does not re-parent orphans, so a
    // grandchild still names its exited parent; creation times rule out a reused process id.

    /// PROCESSENTRY32W
    #[repr(C)]
    struct ProcessEntry {
        size: u32,
        usage: u32,
        pid: u32,
        heap_id: usize,
        module_id: u32,
        threads: u32,
        parent_pid: u32,
        priority: i32,
        flags: u32,
        exe: [u16; 260],
    }

    const TH32CS_SNAPPROCESS: u32 = 2;
    const PROCESS_TERMINATE: u32 = 0x1;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const ERROR_INVALID_PARAMETER: u32 = 87;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
        fn Process32FirstW(snap: Handle, entry: *mut ProcessEntry) -> i32;
        fn Process32NextW(snap: Handle, entry: *mut ProcessEntry) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn GetProcessTimes(h: Handle, creation: *mut u64, exit: *mut u64, kernel: *mut u64, user: *mut u64) -> i32;
        fn TerminateProcess(h: Handle, code: u32) -> i32;
        fn GetExitCodeProcess(h: Handle, code: *mut u32) -> i32;
        fn GetLastError() -> u32;
    }

    /// `(creation, exit)` file times of a process handle (`exit == 0` while it runs).
    fn times(h: Handle) -> Option<(u64, u64)> {
        let (mut c, mut e, mut k, mut u) = (0u64, 0u64, 0u64, 0u64);
        // SAFETY: `h` is a valid process handle; the four pointers are live u64s (a FILETIME is
        // two u32 words, 8 bytes, and u64 is at least as aligned).
        (unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) } != 0).then_some((c, e))
    }

    /// `(pid, parent pid, creation time)` of every process this account can query.
    fn snapshot() -> Vec<(u32, u32, u64)> {
        let mut out = Vec::new();
        // SAFETY: plain call; the handle is checked and closed below.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snap.is_null() || snap as isize == -1 {
            return out;
        }
        // SAFETY: an all-zero PROCESSENTRY32W is valid plain data; `size` is set as the API requires.
        let mut e: ProcessEntry = unsafe { std::mem::zeroed() };
        e.size = std::mem::size_of::<ProcessEntry>() as u32;
        // SAFETY: `snap` is a valid snapshot handle and `e` a live entry of the declared size.
        let mut ok = unsafe { Process32FirstW(snap, &mut e) } != 0;
        while ok {
            // SAFETY: plain call; a null handle is checked, a valid one is closed after use.
            let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, e.pid) };
            if !h.is_null() {
                if let Some((created, _)) = times(h) {
                    out.push((e.pid, e.parent_pid, created));
                }
                // SAFETY: `h` was opened above and is closed once.
                unsafe { CloseHandle(h) };
            }
            // SAFETY: as for Process32FirstW
            ok = unsafe { Process32NextW(snap, &mut e) } != 0;
        }
        // SAFETY: `snap` was opened above and is closed once.
        unsafe { CloseHandle(snap) };
        out
    }

    /// Every live descendant of `root` (it may have exited already) that the snapshot can see.
    pub fn descendants(root: &std::process::Child) -> Vec<u32> {
        let Some((root_created, root_exit)) = times(root.as_raw_handle() as Handle) else {
            return Vec::new();
        };
        let procs = snapshot();
        let me = std::process::id();
        // (pid, creation time, exit time or MAX): a child must be created while its parent lived
        let mut found: Vec<(u32, u64, u64)> = vec![(root.id(), root_created, if root_exit == 0 { u64::MAX } else { root_exit })];
        let mut grew = true;
        while grew {
            grew = false;
            for &(pid, ppid, created) in &procs {
                if pid == me || found.iter().any(|f| f.0 == pid) {
                    continue;
                }
                if found.iter().any(|&(p, c, e)| p == ppid && created >= c && created <= e) {
                    found.push((pid, created, u64::MAX));
                    grew = true;
                }
            }
        }
        found.into_iter().skip(1).map(|f| f.0).collect()
    }

    /// Kill `pids`; `true` when each one is dead afterwards (or was already gone).
    pub fn kill_pids(pids: &[u32]) -> bool {
        let mut all = true;
        for &pid in pids {
            // SAFETY: plain call; a null handle is checked, a valid one is closed below.
            let h = unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if h.is_null() {
                // SAFETY: plain call right after the failing one
                all &= unsafe { GetLastError() } == ERROR_INVALID_PARAMETER; // no such process
                continue;
            }
            let mut code = 0u32;
            // SAFETY: `h` is a valid process handle with the rights requested above; `code` is live.
            let dead = unsafe { TerminateProcess(h, 1) != 0 || (GetExitCodeProcess(h, &mut code) != 0 && code != STILL_ACTIVE) };
            all &= dead;
            // SAFETY: `h` was opened above and is closed once.
            unsafe { CloseHandle(h) };
        }
        all
    }
}

/// No Job Objects outside Windows: the process group plays that role.
#[cfg(not(windows))]
mod job {
    pub struct Job;

    impl Job {
        pub fn for_child(_child: &std::process::Child) -> Option<Job> {
            None
        }

        pub fn release(self) {}
    }
}

/// Kill the child and everything it started (Unix: its process group; Windows: its Job Object, or
/// `taskkill /T` when no job could be made, plus every descendant found by parent process id, which
/// also reaches children a packaged runtime started outside the job). Returns `true` only when the
/// whole tree is known to be killed. A grandchild that moved itself to another process group
/// (Unix) or whose own parent already exited (Windows walk) can still survive.
#[cfg_attr(unix, allow(unsafe_code))] // libc::kill
fn kill_tree(child: &mut Child, job: Option<&job::Job>) -> bool {
    let tree;
    #[cfg(unix)]
    {
        let _ = job;
        // SAFETY: plain syscall; a negative pid names the child's process group (set at spawn).
        // ESRCH: no process is left in the group, so nothing of the tree survives.
        let r = unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
        tree = r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
    }
    #[cfg(windows)]
    {
        // list the tree first: once a parent dies its children can no longer be told from strangers
        let escaped = job::descendants(child);
        let mut killed = job.map_or(false, |j| j.terminate());
        if !killed && !matches!(child.try_wait(), Ok(Some(_))) {
            if let Some(root) = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir")) {
                let tk = Path::new(&root).join("System32").join("taskkill.exe");
                // taskkill fails when the child already exited: its children are then not found
                killed = Command::new(tk)
                    .args(["/T", "/F", "/PID", &child.id().to_string()])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_or(false, |s| s.success());
            }
        }
        let root_done = killed || matches!(child.try_wait(), Ok(Some(_)));
        // processes outside the job (a packaged runtime's children) are killed one by one
        tree = job::kill_pids(&escaped) && root_done;
    }
    let _ = child.kill();
    let _ = child.wait();
    tree
}

fn pump<R: Read + Send + 'static>(mut r: R, total: Arc<AtomicUsize>, cap: usize, over: Arc<AtomicBool>, echo: bool) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        let mut printed = 0usize;
        loop {
            match r.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if total.fetch_add(n, Ordering::SeqCst) + n > cap {
                        over.store(true, Ordering::SeqCst);
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if echo {
                        if let Some(end) = buf.iter().rposition(|b| *b == b'\n') {
                            if end + 1 > printed {
                                let out = std::io::stdout();
                                let mut lock = out.lock();
                                let text = String::from_utf8_lossy(&buf[printed..end + 1]).replace("\r\n", "\n");
                                let _ = lock.write_all(text.as_bytes());
                                let _ = lock.flush();
                                printed = end + 1;
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if echo && printed < buf.len() {
            let text = String::from_utf8_lossy(&buf[printed..]).to_string();
            println!("{}", text.trim_end_matches(['\r', '\n']));
        }
        buf
    })
}

/// Run `cmd` with stdin closed and both pipes drained concurrently. Kills the process tree when
/// `timeout` passes or stdout + stderr exceed `cap` bytes. `echo` prints stdout lines as they come.
fn run_limited(cmd: &mut Command, timeout: Option<Duration>, cap: usize, echo: bool) -> Result<Captured, LimitFail> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let start = Instant::now();
    let mut child = cmd.spawn().map_err(LimitFail::Spawn)?;
    let job = job::Job::for_child(&child);
    let total = Arc::new(AtomicUsize::new(0));
    let over = Arc::new(AtomicBool::new(false));
    let t_out = pump(child.stdout.take().expect("piped"), total.clone(), cap, over.clone(), echo);
    let t_err = pump(child.stderr.take().expect("piped"), total.clone(), cap, over.clone(), false);
    let expired = |start: &Instant| timeout.map_or(false, |t| start.elapsed() >= t);
    let mut nap = Duration::from_millis(1);
    let status = loop {
        if over.load(Ordering::SeqCst) {
            return Err(LimitFail::Overflow(cap, kill_tree(&mut child, job.as_ref())));
        }
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {}
            Err(_) => break None,
        }
        if expired(&start) {
            return Err(LimitFail::Timeout(timeout.unwrap_or_default(), kill_tree(&mut child, job.as_ref())));
        }
        std::thread::sleep(nap);
        nap = (nap * 2).min(Duration::from_millis(20));
    };
    // a background grandchild can keep the pipes open after the child exits: stay inside the limits
    while !(t_out.is_finished() && t_err.is_finished()) {
        if over.load(Ordering::SeqCst) {
            return Err(LimitFail::Overflow(cap, kill_tree(&mut child, job.as_ref())));
        }
        if expired(&start) {
            return Err(LimitFail::Timeout(timeout.unwrap_or_default(), kill_tree(&mut child, job.as_ref())));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let stdout = t_out.join().unwrap_or_default();
    let stderr = t_err.join().unwrap_or_default();
    if over.load(Ordering::SeqCst) {
        return Err(LimitFail::Overflow(cap, kill_tree(&mut child, job.as_ref())));
    }
    if let Some(j) = job {
        j.release();
    }
    Ok(Captured { code: status.and_then(|s| s.code()), stdout, stderr })
}

// --- Execution ---------------------------------------------------------------------------------

/// The result of one block run, also when it exited non-zero.
#[derive(Debug, Clone)]
pub struct RunOutput {
    /// stdout decoded as UTF-8 (invalid bytes replaced), trailing whitespace removed.
    pub stdout: String,
    pub stderr: String,
    /// Exit code (`-1` when killed by a signal).
    pub code: i32,
    pub exe: PathBuf,
    pub version: String,
}

struct Ctx<'a> {
    rt: &'static LangRuntime,
    s: &'a Settings,
    wd: WorkDir,
    echo: bool,
    /// `KILLER_ARGS` / `KILLER_ARGS_FILE`
    extra_env: Vec<(String, String)>,
}

impl Ctx<'_> {
    fn step(&self, mut cmd: Command, stage: &'static str, timeout: Option<Duration>) -> Result<Captured, PolyglotError> {
        scrub_env(&mut cmd, self.s);
        if let Ok(cwd) = std::env::current_dir() {
            cmd.env("KILLER_CWD", cwd);
        }
        for (k, v) in &self.extra_env {
            cmd.env(k, v);
        }
        cmd.current_dir(&self.wd.0);
        let lang = self.rt.name.to_string();
        let echo = self.echo && stage.is_empty();
        match run_limited(&mut cmd, timeout, self.s.max_output, echo) {
            Ok(c) => Ok(c),
            Err(LimitFail::Spawn(e)) => Err(PolyglotError::Spawn { lang, detail: e.to_string() }),
            Err(LimitFail::Timeout(t, tree_killed)) => Err(PolyglotError::Timeout { lang, secs: t.as_secs_f64(), tree_killed }),
            Err(LimitFail::Overflow(limit, tree_killed)) => Err(PolyglotError::OutputTooLarge { lang, limit, tree_killed }),
        }
    }

    fn compile(&self, cmd: Command) -> Result<(), PolyglotError> {
        let c = self.step(cmd, "compile", self.s.compile_timeout())?;
        if c.code == Some(0) {
            return Ok(());
        }
        Err(PolyglotError::NonZeroExit {
            lang: self.rt.name.to_string(),
            stage: "compile",
            code: c.code,
            stderr: String::from_utf8_lossy(&c.stderr).into_owned(),
            stdout: String::from_utf8_lossy(&c.stdout).into_owned(),
        })
    }
}

fn binary_name(stem: &str) -> String {
    if cfg!(windows) { format!("{}.exe", stem) } else { stem.to_string() }
}

/// Java: bare statements are wrapped in a `KillerMain` class.
fn wrap_java_if_needed(code: &str) -> String {
    let t = code.trim();
    if t.contains("class ") && t.contains('{') {
        return code.to_string();
    }
    format!("public class KillerMain {{\n    public static void main(String[] args) {{\n{}\n    }}\n}}\n", indent(code, "        "))
}

fn wrap_rust_if_needed(code: &str) -> String {
    if code.contains("fn main(") {
        return code.to_string();
    }
    format!("fn main() {{\n{}\n}}\n", indent(code, "    "))
}

fn wrap_c_if_needed(code: &str) -> String {
    if code.contains("int main(") || code.contains("void main(") {
        return code.to_string();
    }
    format!("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\nint main(int argc, char** argv) {{\n{}\n    return 0;\n}}\n", indent(code, "    "))
}

fn wrap_cpp_if_needed(code: &str) -> String {
    if code.contains("int main(") {
        return code.to_string();
    }
    format!("#include <iostream>\n#include <string>\n#include <vector>\n#include <algorithm>\nusing namespace std;\nint main() {{\n{}\n    return 0;\n}}\n", indent(code, "    "))
}

fn indent(code: &str, pad: &str) -> String {
    code.lines().map(|l| format!("{}{}", pad, l)).collect::<Vec<_>>().join("\n")
}

/// One Killer value passed into a block.
#[derive(Debug, Clone)]
pub struct BlockArg {
    pub name: String,
    /// The value as JSON.
    pub json: String,
    /// The value as shell text: strings as they are, everything else as JSON.
    pub text: String,
}

/// Values passed into a block (`@lang(a, b) { }`).
pub type BlockArgs = Vec<BlockArg>;

fn execute(rt: &'static LangRuntime, code: &str, args: Option<&BlockArgs>, echo: bool) -> Result<RunOutput, PolyglotError> {
    check_policy(rt)?;
    let s = settings();
    let exe = resolve_runtime(rt)?;
    let javac = if rt.kind == Kind::Java { Some(resolve_javac()?) } else { None };
    let mut ctx = Ctx { rt, s, wd: WorkDir::new(rt.name)?, echo, extra_env: Vec::new() };
    let lang = rt.name;
    let (args_env, prelude) = block_args(rt, &ctx.wd, args)?;
    ctx.extra_env = args_env;
    let run_cmd = match rt.kind {
        Kind::Script => {
            let file = ctx.wd.write(lang, &format!("block.{}", rt.ext), &format!("{}{}\n", prelude, code))?;
            let mut c = Command::new(&exe.path);
            c.args(rt.pre_args).arg(file);
            c
        }
        Kind::Java => {
            let file = ctx.wd.write(lang, "KillerMain.java", &wrap_java_if_needed(code))?;
            let mut c = Command::new(&javac.as_ref().expect("resolved").path);
            c.arg("-encoding").arg("UTF-8").arg("-d").arg(&ctx.wd.0).arg(file);
            ctx.compile(c)?;
            let mut r = Command::new(&exe.path);
            r.arg("-Dfile.encoding=UTF-8").arg("-cp").arg(&ctx.wd.0).arg("KillerMain");
            r
        }
        Kind::Rust | Kind::C | Kind::Cpp => {
            let (name, src) = match rt.kind {
                Kind::Rust => ("block.rs", wrap_rust_if_needed(code)),
                Kind::C => ("block.c", wrap_c_if_needed(code)),
                _ => ("block.cpp", wrap_cpp_if_needed(code)),
            };
            let file = ctx.wd.write(lang, name, &src)?;
            let out = ctx.wd.0.join(binary_name("block_bin"));
            let mut c = Command::new(&exe.path);
            c.arg(file).arg("-o").arg(&out);
            if rt.kind == Kind::Cpp {
                c.arg("-std=c++17");
            }
            if rt.kind != Kind::Rust {
                c.arg("-Wall");
            }
            ctx.compile(c)?;
            Command::new(out)
        }
    };
    let c = ctx.step(run_cmd, "", s.timeout)?;
    Ok(RunOutput {
        stdout: String::from_utf8_lossy(&c.stdout).trim_end().to_string(),
        stderr: String::from_utf8_lossy(&c.stderr).into_owned(),
        code: c.code.unwrap_or(-1),
        exe: exe.path,
        version: exe.version,
    })
}

/// Above this many bytes the arguments go through a file (`KILLER_ARGS_FILE`), not the environment.
pub const ARGS_ENV_LIMIT: usize = 8 * 1024;

/// Variables for the block (`@lang(a, b) { }`): `KILLER_ARGS` holds a JSON object of them (above
/// [`ARGS_ENV_LIMIT`] bytes `KILLER_ARGS_FILE` names a file holding it instead). Python, Ruby and
/// Node get a one-line prelude that defines the variables (so their error line numbers are one
/// higher); Bash / sh get one shell assignment per variable on that line. Every other language
/// reads the JSON itself.
fn block_args(rt: &LangRuntime, wd: &WorkDir, args: Option<&BlockArgs>) -> Result<(Vec<(String, String)>, String), PolyglotError> {
    let Some(args) = args.filter(|a| !a.is_empty()) else { return Ok((Vec::new(), String::new())) };
    let json = format!("{{{}}}", args.iter().map(|a| format!("{}:{}", js(&a.name), a.json)).collect::<Vec<_>>().join(","));
    let env = if json.len() > ARGS_ENV_LIMIT {
        let f = wd.write(rt.name, "killer_args.json", &json)?;
        vec![("KILLER_ARGS_FILE".to_string(), f.to_string_lossy().into_owned())]
    } else {
        vec![("KILLER_ARGS".to_string(), json)]
    };
    let names: Vec<&str> = args.iter().map(|a| a.name.as_str()).collect();
    let each = |f: &dyn Fn(&str) -> String| names.iter().map(|n| f(n)).collect::<Vec<_>>().join("; ");
    let prelude = match rt.name {
        "python" => format!(
            "import json as _kj, os as _ko; _ka = _kj.loads(open(_ko.environ['KILLER_ARGS_FILE'], encoding='utf-8').read() if 'KILLER_ARGS_FILE' in _ko.environ else _ko.environ['KILLER_ARGS']); {}; del _kj, _ko, _ka\n",
            each(&|n| format!("{n} = _ka['{n}']"))
        ),
        "ruby" => format!(
            "require 'json'; _ka = JSON.parse(ENV['KILLER_ARGS_FILE'] ? File.read(ENV['KILLER_ARGS_FILE'], encoding: 'UTF-8') : ENV['KILLER_ARGS']); {}\n",
            each(&|n| format!("{n} = _ka['{n}']"))
        ),
        "node" => format!(
            "const _ka = JSON.parse(process.env.KILLER_ARGS_FILE ? require('fs').readFileSync(process.env.KILLER_ARGS_FILE, 'utf8') : process.env.KILLER_ARGS); {};\n",
            each(&|n| format!("let {n} = _ka['{n}']"))
        ),
        "bash" | "sh" => {
            let sq = char::from(39u8).to_string();
            let dq = char::from(34u8).to_string();
            // 'it'"'"'s': a single quote closes, a double-quoted one follows, then it reopens
            let escaped_sq = format!("{sq}{dq}{sq}{dq}{sq}");
            let mut line = Vec::new();
            for a in args {
                line.push(format!("{}={}{}{}", a.name, sq, a.text.replace('\0', "").replace(&sq, &escaped_sq), sq));
            }
            format!("{}\n", line.join("; "))
        }
        _ => String::new(),
    };
    Ok((env, prelude))
}

/// Run a block and return its output, also when it exits non-zero.
pub fn polyglot_run(lang: &str, code: &str, args: Option<&BlockArgs>) -> Result<RunOutput, PolyglotError> {
    run_block(lang, code, args, false)
}

fn run_block(lang: &str, code: &str, args: Option<&BlockArgs>, echo: bool) -> Result<RunOutput, PolyglotError> {
    let code = crate::polyglot_front::decode(code);
    let start = Instant::now();
    let res = match find_runtime(lang) {
        Some(rt) => execute(rt, &code, args, echo),
        None => Err(PolyglotError::UnknownLang(lang.to_string())),
    };
    audit(find_runtime(lang).map_or(lang, |r| r.name), &code, start.elapsed(), &res);
    res
}

/// Run a block; a non-zero exit is an error.
pub fn polyglot_exec_typed(lang: &str, code: &str, args: Option<&BlockArgs>, echo: bool) -> Result<String, PolyglotError> {
    let out = run_block(lang, code, args, echo)?;
    if out.code == 0 {
        Ok(out.stdout)
    } else {
        Err(PolyglotError::NonZeroExit {
            lang: find_runtime(lang).map(|r| r.name).unwrap_or(lang).to_string(),
            stage: "",
            code: if out.code == -1 { None } else { Some(out.code) },
            stderr: out.stderr,
            stdout: out.stdout,
        })
    }
}

/// Run a block and return its stdout (`Err` holds the `POLYGLOT_E_*` message).
///
/// ```killer
/// result = @python { print(10 * 5) }
/// ```
pub fn polyglot_exec(lang: &str, code: &str) -> Result<String, String> {
    polyglot_exec_typed(lang, code, None, false).map_err(|e| e.to_string())
}

// --- AI assist layer ---------------------------------------------------------------------------

/// [`polyglot_exec`] through the assist layer (execution log, optional AI debugging).
pub fn polyglot_exec_assisted(lang: &str, code: &str) -> Result<String, String> {
    assisted(lang, code, None, false)
}

/// May a failed block (its code and stderr) be sent to the AI assist layer? Only with
/// `KILLER_POLYGLOT_AI=1` and the LLM and network capabilities, and only for a non-zero exit of
/// the block itself (never for a missing runtime, a timeout, a policy denial or an output flood).
pub fn ai_assist_allowed(opt_in: bool, err: &PolyglotError) -> bool {
    opt_in
        && matches!(err, PolyglotError::NonZeroExit { .. })
        && crate::security::require_llm().is_ok()
        && crate::security::require_network().is_ok()
}

fn assisted(lang: &str, code: &str, args: Option<&BlockArgs>, echo: bool) -> Result<String, String> {
    let code = crate::polyglot_front::decode(code);
    let start = Instant::now();
    let typed = polyglot_exec_typed(lang, &code, args, echo);
    let allow_ai = matches!(&typed, Err(e) if ai_assist_allowed(settings().ai, e));
    let result = typed.map_err(|e| e.to_string());
    let duration_ms = start.elapsed().as_millis();
    // a suggestion (if any) goes to the assist log (`nova_assist_log()`), not into the error text
    let (stdout, _suggestion) = crate::assassin_assist::layer().lock().unwrap_or_else(|e| e.into_inner()).process_execution(lang, &code, &result, duration_ms, allow_ai);
    result.map(|_| stdout)
}

/// Text for `polyglot_list()`: one line per installed language with its absolute path and version.
pub fn polyglot_list_str() -> String {
    let mut lines = Vec::new();
    for rt in RUNTIMES {
        if let Ok(r) = resolve_runtime(rt) {
            lines.push(format!("  @{}  {}  ({})", rt.name, r.path.display(), r.version));
        }
    }
    if lines.is_empty() {
        return "No external runtimes detected. Killer code itself needs none.".to_string();
    }
    format!("Detected runtimes:\n{}", lines.join("\n"))
}

// --- Audit log ---------------------------------------------------------------------------------

/// A JSON string literal.
fn js(s: &str) -> String {
    crate::json_lang::stringify(&Value::Str(s.to_string()), 0).unwrap_or_else(|_| "\"\"".to_string())
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Append one JSON line for a run to `KILLER_AUDIT_LOG` (when set). The block source is only
/// included with `KILLER_AUDIT_CODE=1`; output is never included. Write errors are reported once
/// on stderr and do not fail the block. No rotation: use logrotate or similar.
fn audit(lang: &str, code: &str, took: Duration, res: &Result<RunOutput, PolyglotError>) {
    let s = settings();
    let Some(path) = &s.audit_log else { return };
    let (exe, version) = match res {
        Ok(o) => (o.exe.to_string_lossy().into_owned(), o.version.clone()),
        Err(_) => cache()
            .lock()
            .ok()
            .and_then(|c| c.get(lang).and_then(|r| r.as_ref().ok().cloned()))
            .map(|r| (r.path.to_string_lossy().into_owned(), r.version))
            .unwrap_or_default(),
    };
    let (exit_code, outcome) = match res {
        Ok(o) if o.code == 0 => (o.code.to_string(), "ok"),
        Ok(o) => (o.code.to_string(), "POLYGLOT_E_EXIT"),
        Err(PolyglotError::NonZeroExit { code, .. }) => (code.map_or("null".to_string(), |c| c.to_string()), "POLYGLOT_E_EXIT"),
        Err(e) => ("null".to_string(), e.code()),
    };
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let mut line = format!(
        "{{\"ts\":{},\"pid\":{},\"lang\":{},\"exe\":{},\"version\":{},\"code_sha256\":\"{}\",\"code_len\":{},\"duration_ms\":{},\"exit_code\":{},\"outcome\":\"{}\"",
        ts,
        std::process::id(),
        js(lang),
        js(&exe),
        js(&version),
        sha256_hex(code.as_bytes()),
        code.len(),
        took.as_millis(),
        exit_code,
        outcome
    );
    if s.audit_code {
        line.push_str(&format!(",\"code\":{}", js(code)));
    }
    line.push_str("}\n");
    let mut o = fs::OpenOptions::new();
    o.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let r = o.open(path).and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = r {
        static WARN: AtomicBool = AtomicBool::new(false);
        if !WARN.swap(true, Ordering::SeqCst) {
            eprintln!("killer: warning: cannot write the audit log {}: {}", path.display(), e);
        }
    }
}

// --- `killer polyglot list` / `killer doctor` ---------------------------------------------------

/// One row of the runtime report.
#[derive(Debug, Clone)]
pub struct RuntimeRow {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    /// `ok`, `missing` or `denied`.
    pub status: &'static str,
    pub path: String,
    pub version: String,
    pub override_var: String,
    pub detail: String,
}

/// Probe every language (denied languages are not probed).
pub fn runtime_rows() -> Vec<RuntimeRow> {
    RUNTIMES
        .iter()
        .map(|rt| {
            let mut row = RuntimeRow { name: rt.name, aliases: rt.aliases, status: "ok", path: String::new(), version: String::new(), override_var: override_vars(rt.name)[0].clone(), detail: String::new() };
            if let Err(e) = policy_decision(rt.name, settings(), true) {
                row.status = "denied";
                row.detail = e.to_string();
                return row;
            }
            let found = resolve_runtime(rt).and_then(|r| if rt.kind == Kind::Java { resolve_javac().map(|_| r) } else { Ok(r) });
            match found {
                Ok(r) => {
                    row.path = r.path.to_string_lossy().into_owned();
                    row.version = r.version;
                }
                Err(e) => {
                    row.status = "missing";
                    row.detail = e.to_string();
                }
            }
            row
        })
        .collect()
}

fn rows_json(rows: &[RuntimeRow]) -> String {
    let items: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "{{\"name\":{},\"aliases\":[{}],\"status\":{},\"path\":{},\"version\":{},\"override\":{}}}",
                js(r.name),
                r.aliases.iter().map(|a| js(a)).collect::<Vec<_>>().join(","),
                js(r.status),
                js(&r.path),
                js(&r.version),
                js(&r.override_var)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn rows_text(rows: &[RuntimeRow]) -> String {
    let mut out = format!("{:<11} {:<8} {:<60} {}\n", "LANGUAGE", "STATUS", "PATH", "VERSION");
    for r in rows {
        out.push_str(&format!("{:<11} {:<8} {:<60} {}\n", r.name, r.status, if r.path.is_empty() { "-" } else { &r.path }, r.version));
    }
    out
}

fn policy_summary(s: &Settings) -> String {
    let policy = match &s.allow {
        Allow::All => "allow all (KILLER_POLYGLOT_ALLOW)".to_string(),
        Allow::Only(l) if l.is_empty() => "deny all (KILLER_POLYGLOT_ALLOW=none)".to_string(),
        Allow::Only(l) => format!("allow {} (KILLER_POLYGLOT_ALLOW)", l.join(",")),
        Allow::Unset if s.strict => "deny all (KILLER_PROFILE strict, no KILLER_POLYGLOT_ALLOW)".to_string(),
        Allow::Unset => "allow all, warn once (default)".to_string(),
    };
    format!(
        "policy: {}\ntimeout: {}\nmax output: {} bytes\nenv pass-through: {}\nAI assist: {}\naudit log: {}\n",
        policy,
        s.timeout.map_or("none".to_string(), |t| format!("{} s run, {} s compile", t.as_secs_f64(), t.as_secs_f64() * 2.0)),
        s.max_output,
        if s.env_pass.is_empty() { "-".to_string() } else { s.env_pass.join(",") },
        if s.ai { "on (KILLER_POLYGLOT_AI)" } else { "off" },
        s.audit_log.as_ref().map_or("-".to_string(), |p| p.display().to_string())
    )
}

/// `killer polyglot list [--json]`
pub fn cmd_polyglot(args: &[String]) -> i32 {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            let rows = runtime_rows();
            if args.iter().any(|a| a == "--json") {
                println!("{}", rows_json(&rows));
            } else {
                print!("{}", rows_text(&rows));
            }
            0
        }
        _ => {
            eprintln!("usage: killer polyglot list [--json]");
            2
        }
    }
}

/// `killer doctor [--require a,b] [--json]`: runtimes and polyglot settings; with `--require`,
/// exit 1 unless every listed language is allowed and installed.
pub fn cmd_doctor(args: &[String]) -> i32 {
    let mut require: Vec<String> = Vec::new();
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--require" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    require.extend(v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
                }
            }
            other => {
                eprintln!("killer doctor: unknown argument `{}`\nusage: killer doctor [--require python,bash] [--json]", other);
                return 2;
            }
        }
        i += 1;
    }
    let rows = runtime_rows();
    if json {
        println!("{}", rows_json(&rows));
    } else {
        print!("{}\n{}", policy_summary(settings()), rows_text(&rows));
    }
    let missing: Vec<String> = require
        .iter()
        .filter(|l| match find_runtime(l) {
            None => true,
            Some(rt) => rows.iter().find(|r| r.name == rt.name).map_or(true, |r| r.status != "ok"),
        })
        .cloned()
        .collect();
    if missing.is_empty() {
        0
    } else {
        eprintln!("killer doctor: required but not available: {}", missing.join(", "));
        1
    }
}

// --- Builtins ----------------------------------------------------------------------------------

fn str_arg(args: &[Value], i: usize, f: &str, what: &str) -> Result<String, VmError> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(VmError::runtime_error(format!("{}: {} must be a string", f, what))),
    }
}

/// `polyglot_exec(lang, code)`: stdout of the block as a string.
pub fn builtin_polyglot_exec(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    if args.len() < 2 {
        return Err(VmError::runtime_error("polyglot_exec(lang, code) requires 2 arguments".to_string()));
    }
    let lang = str_arg(args, 0, "polyglot_exec", "lang")?;
    let code = str_arg(args, 1, "polyglot_exec", "code")?;
    assisted(&lang, &code, None, false).map(Value::Str).map_err(VmError::runtime_error)
}

/// `polyglot_run(lang, code)`: `{"stdout", "stderr", "code"}`; a non-zero exit is not an error.
pub fn builtin_polyglot_run(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    let lang = str_arg(args, 0, "polyglot_run", "lang")?;
    let code = str_arg(args, 1, "polyglot_run", "code")?;
    let out = polyglot_run(&lang, &code, None).map_err(|e| VmError::runtime_error(e.to_string()))?;
    Ok(Value::Dict(crate::value::SharedDict::from_pairs(vec![
        ("stdout".to_string(), Value::Str(out.stdout)),
        ("stderr".to_string(), Value::Str(out.stderr.trim_end().to_string())),
        ("code".to_string(), Value::Number(out.code as f64)),
    ])))
}

/// `polyglot_exec_args(lang, code, "a,b", a, b)`: what `@lang(a, b) { ... }` compiles to.
pub fn builtin_polyglot_exec_args(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    let lang = str_arg(args, 0, "polyglot_exec_args", "lang")?;
    let code = str_arg(args, 1, "polyglot_exec_args", "code")?;
    let names = str_arg(args, 2, "polyglot_exec_args", "names")?;
    let names: Vec<&str> = names.split(',').filter(|n| !n.is_empty()).collect();
    if names.len() != args.len().saturating_sub(3) {
        return Err(VmError::runtime_error("polyglot_exec_args: one value per name is required".to_string()));
    }
    let mut pairs = BlockArgs::new();
    for (n, v) in names.iter().zip(&args[3..]) {
        let json = crate::json_lang::stringify(v, 0).map_err(|e| VmError::runtime_error(format!("@{}({}): {}", lang, n, e)))?;
        let text = match v {
            Value::Str(s) => s.clone(),
            _ => json.clone(),
        };
        pairs.push(BlockArg { name: n.to_string(), json, text });
    }
    assisted(&lang, &code, Some(&pairs), false).map(Value::Str).map_err(VmError::runtime_error)
}

/// `polyglot_list()`: the installed runtimes.
pub fn builtin_polyglot_list(_args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    Ok(Value::Str(polyglot_list_str()))
}

/// `polyglot_check(lang)`: true when the language is allowed and its runtime is installed.
pub fn builtin_polyglot_check(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    if args.is_empty() {
        return Err(VmError::runtime_error("polyglot_check(lang) requires 1 argument".to_string()));
    }
    let lang = str_arg(args, 0, "polyglot_check", "lang")?;
    let Some(rt) = find_runtime(&lang) else { return Ok(Value::Bool(false)) };
    let cap = crate::security::current_capabilities().allow_process_spawn;
    if policy_decision(rt.name, settings(), cap).is_err() {
        return Ok(Value::Bool(false));
    }
    let ok = resolve_runtime(rt).is_ok() && (rt.kind != Kind::Java || resolve_javac().is_ok());
    Ok(Value::Bool(ok))
}

/// `polyglot_stream(lang, code)`: like `polyglot_exec`, but prints each stdout line as it arrives.
pub fn builtin_polyglot_stream(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    if args.len() < 2 {
        return Err(VmError::runtime_error("polyglot_stream(lang, code) requires 2 arguments".to_string()));
    }
    let lang = str_arg(args, 0, "polyglot_stream", "lang")?;
    let code = str_arg(args, 1, "polyglot_stream", "code")?;
    assisted(&lang, &code, None, true).map(Value::Str).map_err(VmError::runtime_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(vars: &[(&str, &str)]) -> Settings {
        let m: HashMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Settings::from_lookup(&move |n| m.get(n).cloned())
    }

    #[test]
    fn error_variants_have_stable_codes_and_exit_codes() {
        let e = PolyglotError::NonZeroExit { lang: "bash".into(), stage: "", code: Some(3), stderr: "boom".into(), stdout: String::new() };
        assert!(matches!(e, PolyglotError::NonZeroExit { code: Some(3), .. }));
        assert!(e.to_string().starts_with("POLYGLOT_E_EXIT: @bash failed with exit code 3:\nboom"));
        assert_eq!(PolyglotError::Timeout { lang: "x".into(), secs: 1.0, tree_killed: true }.code(), "POLYGLOT_E_TIMEOUT");
        assert!(PolyglotError::RuntimeMissing { lang: "python".into(), detail: "d".into() }.to_string().contains("runtime not found"));
    }

    #[test]
    fn limit_errors_claim_a_killed_tree_only_when_it_was() {
        let t = |k| PolyglotError::Timeout { lang: "python".into(), secs: 2.0, tree_killed: k }.to_string();
        assert!(t(true).contains("process tree was killed"), "{}", t(true));
        assert!(!t(false).contains("tree was killed") && t(false).contains("may still be running"), "{}", t(false));
        let o = PolyglotError::OutputTooLarge { lang: "python".into(), limit: 9, tree_killed: false }.to_string();
        assert!(!o.contains("tree was killed") && o.contains("may still be running"), "{o}");
    }

    #[test]
    fn snippet_keeps_the_tail_within_the_cap() {
        let s = "a".repeat(10_000) + "LAST";
        let t = snippet(&s, SNIPPET_BYTES);
        assert!(t.len() <= SNIPPET_BYTES + 40 && t.ends_with("LAST"));
        let u = "\u{e9}".repeat(3000);
        assert!(snippet(&u, 101).len() <= 140);
    }

    #[test]
    fn supported_list_comes_from_the_table() {
        let l = supported_list();
        for rt in RUNTIMES {
            assert!(l.contains(rt.name), "{}", rt.name);
        }
        assert!(PolyglotError::UnknownLang("cobol".into()).to_string().contains(&l));
    }

    #[test]
    fn aliases_resolve_to_one_runtime() {
        assert_eq!(find_runtime("python3").unwrap().name, "python");
        assert_eq!(find_runtime("JS").unwrap().name, "node");
        assert_eq!(find_runtime("ps").unwrap().name, "powershell");
        assert_eq!(find_runtime("c++").unwrap().name, "cpp");
        assert!(find_runtime("cobol").is_none());
    }

    #[test]
    fn settings_parse_limits() {
        let s = settings_with(&[]);
        assert_eq!(s.timeout, Some(Duration::from_secs(DEFAULT_TIMEOUT_S)));
        assert_eq!(s.max_output, DEFAULT_MAX_OUTPUT);
        assert_eq!(s.compile_timeout(), Some(Duration::from_secs(2 * DEFAULT_TIMEOUT_S)));
        let s = settings_with(&[("KILLER_POLYGLOT_TIMEOUT", "0"), ("KILLER_POLYGLOT_MAX_OUTPUT", "1024"), ("KILLER_POLYGLOT_ENV_PASS", "A, B")]);
        assert_eq!(s.timeout, None);
        assert_eq!(s.max_output, 1024);
        assert_eq!(s.env_pass, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn override_variable_names() {
        assert_eq!(override_vars("python"), vec!["KILLER_POLYGLOT_PYTHON".to_string(), "KILLER_PYTHON".to_string()]);
        assert_eq!(override_vars("cpp"), vec!["KILLER_POLYGLOT_CPP".to_string()]);
    }

    #[test]
    fn second_resolution_does_not_probe_again() {
        let rt = find_runtime("zig").unwrap();
        let _ = resolve_runtime(rt);
        let n = probe_count();
        let _ = resolve_runtime(rt);
        let _ = resolve_runtime(rt);
        assert_eq!(probe_count(), n);
    }

    #[test]
    fn ai_assist_needs_opt_in_capabilities_and_a_block_failure() {
        use crate::security::{CapabilityScopeGuard, CapabilitySet};
        let fail = PolyglotError::NonZeroExit { lang: "python".into(), stage: "", code: Some(1), stderr: "x".into(), stdout: String::new() };
        let _t = CapabilityScopeGuard::install(CapabilitySet::trusted_local());
        assert!(ai_assist_allowed(true, &fail));
        assert!(!ai_assist_allowed(false, &fail));
        assert!(!ai_assist_allowed(true, &PolyglotError::Timeout { lang: "python".into(), secs: 1.0, tree_killed: true }));
        assert!(!ai_assist_allowed(true, &PolyglotError::RuntimeMissing { lang: "python".into(), detail: String::new() }));
        {
            let _g = CapabilityScopeGuard::install(CapabilitySet { allow_process_spawn: true, allow_llm: false, ..CapabilitySet::trusted_local() });
            assert!(!ai_assist_allowed(true, &fail));
        }
        let _n = CapabilityScopeGuard::install(CapabilitySet { allow_network: false, ..CapabilitySet::trusted_local() });
        assert!(!ai_assist_allowed(true, &fail));
    }

    #[test]
    fn assist_layer_does_not_ask_a_model_unless_allowed() {
        let before = crate::assassin_assist::ai_call_count();
        let err: Result<String, String> = Err("POLYGLOT_E_EXIT: @python failed with exit code 1".into());
        let _ = crate::assassin_assist::layer().lock().unwrap().process_execution("python", "raise SystemExit(1)", &err, 1, false);
        assert_eq!(crate::assassin_assist::ai_call_count(), before);
    }

    #[test]
    fn limited_run_times_out_and_caps_output() {
        // the test binary itself is a portable long-running / chatty child: `--list` prints test names
        let me = std::env::current_exe().unwrap();
        let mut c = Command::new(&me);
        c.arg("--list");
        match run_limited(&mut c, Some(Duration::from_secs(60)), 64, false) {
            Err(LimitFail::Overflow(64, _)) => {}
            other => panic!("expected overflow, got {}", match other { Ok(_) => "ok", Err(LimitFail::Timeout(..)) => "timeout", Err(LimitFail::Spawn(_)) => "spawn", _ => "?" }),
        }
    }
}
