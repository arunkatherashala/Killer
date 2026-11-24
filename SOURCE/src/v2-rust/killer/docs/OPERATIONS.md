# Operating Killer in production (today)

Killer is beta ([STATUS.md](STATUS.md), [STABILITY.md](STABILITY.md)). This page says how to run it and, just as
important, what is **not** there. Threats and residual risks: `SOURCE/docs/project/THREAT_MODEL.md`.

## Deployment shape

* One native executable (`killer`, plus optional `killer_pkg`, `killer_lsp`, `killer_dap`, `killer-mcp`). Install with
  `install.sh` (checksum-verified; use `--sha256` or `--require-pinned`, see SECURITY.md) or build with
  `cargo build --release --bin killer`.
* Run-time dependencies outside the binary: `curl` for every HTTP/HTTPS client call and LLM call, `openssl` for TLS
  sockets and the TLS server front, `git` for git dependencies, PowerShell on Windows for the TLS server. Killer has no
  TLS stack of its own. Install and pin these on the host; point `KILLER_CURL` / `KILLER_OPENSSL` at absolute paths.
* Release profile uses `panic = "abort"`: an internal panic ends the process. Run under a supervisor (systemd, a
  Windows service wrapper, `restart: always`) because Killer does not restart itself.
* Platforms actually run in CI: x86-64 Windows, x86-64 Linux, Apple Silicon macOS ([PLATFORMS.md](PLATFORMS.md)). The
  JIT is x86-64 only; elsewhere the interpreter is used.
* Running a server: `serve(port, handler, {host: "0.0.0.0"})` (default host `127.0.0.1`). Handlers run on a pool of
  `workers` threads (default min(CPUs, 8)) behind a bounded `queue` (default 64, `503` when full); each worker
  has its own copy of the globals. Put a reverse proxy in front for TLS termination, auth and rate limiting. TLS is available
  through `serve(..., {tls: {cert, key}})` but costs one `openssl s_server` child per connection.

## Environment variables

Read in the source (`grep 'env::var("KILLER_'`) or by the installers. Booleans: "on" means `1`, `true` or `yes` unless noted.

### Security-relevant

| Variable | Default | Meaning | Security impact |
|---|---|---|---|
| `KILLER_ALLOW_PRIVATE_NET` | off | allow `http_*` to loopback and private ranges | disables the main SSRF protection for the whole process; leave off unless the script must reach local services (per-call `allow_private` is narrower) |
| `KILLER_ALLOW_METADATA_NET` | off | allow link-local and cloud-metadata addresses | exposes cloud credentials to any script or injected URL; almost never set |
| `KILLER_TOOL_ALLOW` | unset (deny) | comma list of LLM tools enabled, or `*` | `*` lets a model call file/network tools |
| `KILLER_OPENSSL`, `KILLER_CURL`, `KILLER_POWERSHELL` | PATH search, absolute only | absolute path of helper program | whoever sets these chooses the program run for TLS/HTTP |
| `KILLER_ALLOW_INSECURE_REGISTRY` | off | accept a non-loopback `http://` registry | checksums and publish tokens travel unauthenticated |
| `KILLER_REQUIRE_PINS` | off | `killer install` refuses registry releases without a `[checksums]` pin | stricter, safer |
| `KILLER_REGISTRY`, `KILLER_REGISTRY_TOKEN` | unset | registry URL and publish token | token is a secret; appears in the environment |
| `KILLER_MCP_TOKEN` | unset (no auth) | shared secret required in each `killer-mcp` request | see THREAT_MODEL: pipe-level secret only |
| `KILLER_KHLM_LLM_API_KEY` | unset | API key for the KhLM router's LLM tier | secret in the environment |
| `KILLER_TLS_DEBUG` | off (any value turns on) | trace the TLS server helper | may log connection details to stderr |

### Resource limits

| Variable | Default | Meaning |
|---|---|---|
| `KILLER_EVAL_MAX_MEMORY` | 256 (MB) | memory cap of `eval`/`exec`, 1 to 2048; other values are an error. Live check only in the CLI binaries or with the `mem-cap` feature |
| `KILLER_CHAN_RECV_TIMEOUT_SECS` | 30 | how long a channel receive may block, 1 to 3600 (invalid values fall back to 30) |
| `KILLER_MCP_RUN_TIMEOUT_S` | 30 | wall-clock budget of `killer_run` in `killer-mcp` |
| `KILLER_KHLM_SYNTH_CONTEXT_CHARS` | 12000 | characters of external context fed to RLM synthesis, clamped 512 to 120000 |
| `KILLER_BENCH_MAX_LEVEL` | (benchmark binary) | level cap of `killer_10_level_benchmark` |

### Logging

| Variable | Default | Meaning |
|---|---|---|
| `KILLER_LOG` | `info` | minimum level of the `log_*` builtins (output goes to stderr) |
| `KILLER_LOG_FORMAT` | text | `json` for one JSON object per line |
| `KILLER_LOG_LEVEL` | `INFO` | level for the Rust `logging` module (`from_env`); the language builtins use `KILLER_LOG` |

### AI, GUI and misc

| Variable | Meaning |
|---|---|
| `KILLER_KHLM_LLM_PROVIDER` (`ollama`, `groq`, `openai`, `anthropic`), `KILLER_KHLM_LLM_MODEL`, `KILLER_KHLM_LLM_MAX_TOKENS` (512), `KILLER_KHLM_LLM_TEMPERATURE` (0.7) | KhLM router LLM tier |
| `KILLER_KHLM_RLM`, `KILLER_KHLM_GGUF` | path or name of the local GGUF model |
| `KILLER_KV_Q8` | quantize the inference KV cache |
| `KILLER_GUI_HEADLESS` | non-empty and not `0` forces the headless GUI model (default: headless on every platform but Windows) |
| `KILLER_GUI_HIDDEN` | set to keep native windows hidden (tests) |
| `KILLER_NATIVE` | path of the `killer-native` binary used by `killer-mcp` |

### Compiler switches (debugging only, any value sets them)

`KILLER_NO_FUSION`, `KILLER_NO_PARAM_SLOTS`, `KILLER_NO_RO_CAPTURES`, `KILLER_NO_CAP_SLOTS`, `KILLER_X_NOELIDE`,
`KILLER_X_NORANGE`, `KILLER_X_NOLOOK`. They turn optimizations off to bisect a miscompile; not for production.

### Installer (`install.sh`)

`KILLER_VERSION`, `KILLER_HOME` (default `~/.killer`), `KILLER_REPO`, `KILLER_BASE_URL` (mirror; prints a notice),
`KILLER_SHA256`, `KILLER_REQUIRE_PINNED`, `KILLER_CHECKSUMS_URL`, `KILLER_OS`, `KILLER_ARCH`. `install.ps1` is untested.

### Test-only

`KILLER_FUZZ_ITERS`, `KILLER_FUZZ_SEED` (`tests/robustness.rs`), `KILLER_EVAL_ESCAPE`, `KILLER_CAPS_LOADENV_X`,
`KILLER_EMBED_TEST_KEY`, `KILLER_EMBED_UNSET_VAR`, `KILLER_TEST*`, `KILLER_X`, `KILLER_PFX_PW` (internal, set on the
TLS helper child). Do not set them.

## Resource limits that exist

* `eval`/`exec`: 50M steps, 30 s, 8 nested levels, memory cap above (`tests/eval_safety.rs`, `tests/eval_memory*.rs`).
* Recursion depth, allocation size and nesting limits give catchable errors (`tests/robustness.rs`).
* `serve`: 64 KB headers, 16 MB body, `read_timeout` 10 s, `request_timeout` 60 s, `stream_timeout` 300 s, `workers` min(CPUs, 8) (at most 256), `queue` 64, `drain_timeout` 30 s; TLS front 128 connections.
* `killer-mcp`: 8 MiB message, 64 nesting levels. Package registry client 64 MB / 120 s.
* **No** process-wide memory, CPU or file-descriptor limit: use the operating system (cgroups, ulimit, job objects).

## Logging

`log_*` builtins write to stderr, text or JSON lines (`KILLER_LOG_FORMAT=json`). There is no log rotation, no file
sink, no syslog. Redirect stderr in your supervisor.

## What is missing (do not assume)

* **Observability**: no metrics endpoint, no Prometheus, no OpenTelemetry, no tracing exporter, no built-in health
  endpoint (write a `route("GET", "/health", ...)` yourself). Old docs mention telemetry and probes in the Rust library;
  those are library modules, not wired into the CLI.
* **Container image**: none published, no Dockerfile maintained here. Build your own `FROM` a minimal base with `curl`
  and `openssl` installed.
* **Signed releases**: none; verify with a pinned SHA-256.
* **Real release run**: the release workflow has never run on a real tag; `install.ps1` is untested.
* **Graceful shutdown/reload**: `server_stop()` exists inside a handler; there is no signal handling contract.
* **Clustering, HA, authn/authz for scripts written in Killer**: none.
* **Long-run evidence**: `tests/memory_soak.rs` covers memory in cycles and tasks; there is no multi-day soak result.
