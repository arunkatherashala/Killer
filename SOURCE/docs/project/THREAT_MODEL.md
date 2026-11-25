# Killer threat model

Scope: the `killer` interpreter and its CLI tools (`killer`, `killer_super`, `killer_pkg`, `killer-mcp`, LSP/DAP, installers).
Companion to [SECURITY.md](SECURITY.md), which has the detailed trust notes. Test paths are relative to
`SOURCE/src/v2-rust/killer/`. "Proof" means a test that fails if the mitigation is removed; it was found by reading the
test file, not re-run for this document. Killer itself has not been independently audited.

## Assets

* The host user's files, credentials (`~/.ssh`, `.aws`, API keys in environment variables) and network position (cloud
  metadata, internal services).
* Secrets the script handles: API keys, passwords, encryption keys.
* Availability of long-running processes (an HTTP server, MCP server).
* Integrity of what gets installed: release binaries and packages.

## Trust boundaries

| Party | Trusted? | Notes |
|---|---|---|
| The person who starts Killer (env vars, command line, installed binary) | yes | `KILLER_*` variables are set by them, never by scripts (a script can `env_set` only with the process capability) |
| Script author | **yes by default**: a script runs with the user's powers | Restriction exists only if the embedder gives the VM a reduced capability set (file, network, process, native code). The `killer run` CLI runs scripts with full trust |
| Script input (arguments, files, stdin, JSON/CSV) | no | handled as data; limits on depth, sizes |
| Network peers (HTTP/WebSocket/TLS clients, registry, LLM providers, MCP clients) | no | |
| Text passed to `eval`/`exec` | as trusted as the caller | inherits all of the caller's capabilities |
| Helper programs `openssl`, `curl`, `git`, `powershell` | trusted, found via absolute PATH lookup | |
| Runtimes started by `@lang { }` polyglot blocks (`python3`, `bash`, `javac`, `rustc`, ...) | trusted, found via absolute PATH lookup or `KILLER_POLYGLOT_<LANG>` | the block's code runs **outside** the Killer VM with the OS account's full powers; see "Polyglot blocks" below |

## Attacker models

1. **Hostile input to a trusted script** (crafted JSON, request, file): wants crash, hang, memory exhaustion.
2. **Hostile network peer** of a server the script runs (`serve`, `tls_listen`, `ws_accept`, `killer-mcp`, `kpm-registry`).
3. **Script that must be confined** (generated or plugin code in an embedding with a restricted capability set).
4. **Prompt-injected LLM** that drives tool calling.
5. **Supply-chain attacker** replacing a release asset, a package or a registry response.
6. **Local attacker** who can plant files in the current directory or a relative PATH entry.

## Threat table

| Threat | Mitigation | Proof (test) | Residual risk |
|---|---|---|---|
| Restricted script reads/writes files, uses network, spawns processes, calls native code | capability set checked in every builtin that touches those resources | `tests/capabilities.rs` (`restricted_sweep_has_no_side_effects`, `file_builtins_require_file_capabilities`, `network_builtins_...`, `process_and_native_builtins_...`) | the CLI runs scripts fully trusted; capabilities are an embedding feature, not a default sandbox. `env()`, `args()`, `load_env()` are gated too |
| `eval` used to escape restrictions | fresh VM receives the caller's capabilities | `tests/eval_safety.rs` (`eval_under_restricted_capabilities_cannot_read_write_or_spawn`) | `eval` of untrusted text still has **all** of the caller's powers: never evaluate untrusted text |
| `eval` loops forever / recurses | 50M steps, 30 s, 8 nested levels | `tests/eval_safety.rs` (`infinite_loop_inside_eval_is_stopped`, `deep_recursion_...`, `eval_nesting_is_limited_to_eight_levels`) | a single blocking builtin (`sleep_ms(1e9)`) is not interrupted |
| `eval` exhausts memory | 256 MB default cap (counting allocator + pre-checks at big allocation sites) | `tests/eval_memory.rs`, `tests/eval_memory_cli.rs` (live check in CLI binaries), `tests/eval_memory_process.rs`, `tests/memory_soak.rs` (threads cannot bypass) | cap counts requested bytes, peak can be about 2x the cap; **live check exists only with the counting allocator** (CLI binaries or `mem-cap` feature): an embedder without it gets only pre-checks (`tests/eval_memory_embedded.rs` proves the weaker guarantee); processes started by the code are not counted |
| Whole-process crash on any Rust panic | none: `panic = "abort"` in the release profile, so an uncaught panic ends the process (and `catch_unwind` is not available in release builds). Fuzz and crash-regression tests aim to remove panics | `tests/robustness.rs` (seeded fuzz), `tests/crash_fixes.rs` | **a panic is a denial of service** for a server; `killer-mcp` wraps tool calls in `catch_unwind`, which only works in builds with unwinding |
| SSRF: script or LLM fetches cloud metadata, loopback, private hosts | `http_req` resolves the host, checks every address, pins curl to those addresses, no redirects; metadata/link-local always blocked | `tests/ssrf_guard.rs` (`loopback_and_private_targets_are_refused_by_default`, `allow_private_option_and_env_opt_in_...`, `metadata_and_link_local_stay_blocked_even_with_allow_private`) | not covered: raw-socket Ollama client (fixed localhost:11434), package-registry client, `tcp_connect`/sockets (these need the network capability but have no address filter) |
| TLS certificate checks disabled | no `-k` in source except the explicit `insecure: true` option, which prints a warning | `tests/no_insecure_curl.rs` | a script author can still pass `insecure: true` |
| API key sent in clear text / on a command line | curl config via stdin; a source scan rejects bearer keys on raw sockets | `tests/no_insecure_curl.rs` (`no_source_file_writes_a_bearer_key_to_a_raw_socket`) | the registry publish token travels in clear over plain-HTTP registries |
| Planted `curl`/`openssl` in cwd or relative PATH | absolute-path lookup from a startup PATH snapshot; override must be absolute | `tests/tool_resolution.rs` (`a_planted_tool_in_the_current_directory_is_ignored`, `script_that_changes_path_with_env_set_cannot_redirect_the_tool`, ...) | a writable directory already on the startup PATH; **TLS and HTTP depend on external `curl` and `openssl` binaries whose behaviour and version Killer does not control** (no built-in TLS: rustls is not built) |
| Weak/incorrect crypto | RustCrypto primitives behind thin wrappers, versioned formats, PBKDF2 600k rounds, stored counts bounded | `tests/crypto_vectors.rs` (FIPS 180-4, RFC 4231, RFC 7914, RFC 8439, RFC 4648 vectors; tamper tests; Python cross-check) | not audited; no AES, SHA-3, Argon2, public-key crypto; no key wiping or side-channel claims ([CRYPTO.md](../../src/v2-rust/killer/docs/CRYPTO.md)) |
| Hostile MCP client (huge or nested messages, hangs, panics) | 8 MiB body cap, 64 nesting levels, run budget (30 s default), optional token, worker thread with `catch_unwind` | `tests/mcp_hardening.rs` | the token is a shared secret on a pipe; `killer_run` runs any readable `.killer` file; no step budget |
| Installer / release tampering | independent SHA-256 (`--sha256`) or pinned `checksums/<tag>.sha256`; `--require-pinned` | `tests/installer_integrity.rs` (`a_release_rewritten_consistently_is_caught_by_the_independent_checksum`, `require_pinned_refuses_...`) | **no signatures** on releases or packages; default install trusts the release's own `.sha256`; `install.ps1` is untested; the release workflow has never run on a real tag |
| Package substitution | lockfile content hashes, `[checksums]` pins, manifest equals archive manifest, git ref validation | `tests/pkg_driver.rs`, `tests/packages.rs` | trust on first use for unpinned dependencies; no signed index, no revocation |
| Prompt-injected LLM calls dangerous tools | file/network tools denied unless enabled (`enable_tool`, `KILLER_TOOL_ALLOW`) | unit tests in `src/tool_calling.rs` | `KILLER_TOOL_ALLOW=*` removes protection; no integration test; live providers never tested |
| Slow or malicious clients of `serve` | one request at a time; header (64 KB) and body (16 MB) caps; `read_timeout`, `request_timeout`; TLS front limits 128 connections | `tests/net_ai.rs`, `tests/tls_server.rs` | single-threaded accept loop: one slow SSE stream or a blocking builtin stalls everyone; no rate limiting; no authentication layer |
| WebSocket abuse | 64 MB message cap, control-frame limits, Origin check option | unit tests in `src/` socket code; `tests/sockets.rs` | origin check is opt-in per `ws_accept` |
| Memory leaks in long runs (cycles, generators) | cycle collection | `tests/memory_soak.rs` | only the cases tested |
| Data races in tasks | tasks get deep copies; channels and mutex cells | `tests/concurrency_stress.rs`, `tests/thread_safety.rs` | tested by stress, not proven |
| Polyglot block hangs or floods output | wall-clock timeout (`KILLER_POLYGLOT_TIMEOUT`, 60 s run / 120 s compile) and output cap (`KILLER_POLYGLOT_MAX_OUTPUT`, 8 MiB); the process tree is killed (Unix process group; Windows Job Object plus descendants found by parent pid, `taskkill /T` as fallback) and the error says so only when it was proven (Windows: job terminated, no descendant outside it, runtime not a Store app alias); a regular interpreter on PATH is preferred over a Windows Store app alias | `tests/polyglot.rs` (`timeout_kills_python_and_is_catchable`, `bash_sleep_timeout_kills_the_process_tree`, `timeout_kills_a_grandchild_after_its_parent_exited`, `store_alias_python_timeout_does_not_claim_a_killed_tree`, `polyglot_list_warns_about_a_store_alias_runtime`, `output_cap_stops_a_flood`), unit tests `polyglot::tests::escaped_store_alias_tree_gets_the_truthful_message`, `polyglot::tests::a_regular_interpreter_wins_over_an_earlier_store_alias` | a grandchild that leaves its process group (Unix), or whose own parent exited before the walk (Windows, outside the job), survives; so can the children of a Store app alias runtime when it is the only one installed (the error says they may still run); no memory or CPU limit (see below) |
| Polyglot block reads secrets from the environment | child environment cleared and rebuilt from an allowlist with startup values (a script's `env_set` cannot change them); `KILLER_POLYGLOT_ENV_PASS` for explicit pass-through | `tests/polyglot.rs` (`env_is_scrubbed_and_pass_through_is_explicit`, `env_set_does_not_change_the_block_environment`) | the child can still read every file the user can (`~/.aws`, `.env`); hygiene, not a boundary |
| Planted `python3` / `bash` in cwd or a relative PATH entry | runtimes resolved once to absolute paths from the startup PATH snapshot, cwd and relative entries skipped; overrides must be absolute | `tests/polyglot.rs` (`planted_runtime_in_a_relative_path_entry_is_not_run`, Unix), unit test `tool_path::tests::relative_and_current_directory_entries_are_skipped` | a writable directory already on the startup PATH |
| Polyglot blocks where none are wanted | policy: process capability, `KILLER_POLYGLOT_ALLOW`, `KILLER_PROFILE=strict` (default deny); checked before any runtime is probed | `tests/polyglot.rs` (`allow_list_admits_only_the_listed_languages`, `allow_none_and_strict_profile_deny`), `tests/capabilities.rs` | `killer run` default is allow (with a one-time warning) |
| Block code and stderr sent to an LLM provider | AI assist is opt-in (`KILLER_POLYGLOT_AI=1`) and needs the LLM and network capabilities; never for timeouts, denials or missing runtimes | unit tests `polyglot::tests::ai_assist_needs_opt_in_capabilities_and_a_block_failure`, `tests/polyglot.rs` (`errors_carry_no_ai_boilerplate`) | once opted in, the code and stderr of failing blocks leave the machine |
| `http_download` writes to sensitive paths | canonical-path destination policy | commit 954ba86; unit tests in `src/` | list of protected locations is not exhaustive |

## What the controls do NOT protect

* **A script is not sandboxed by default.** If you run untrusted `.killer` files with `killer run`, they can do anything you can.
* **`eval`/`exec` run with the caller's powers.** The limits stop accidents and runaway code, not a determined attacker.
* **Memory cap only with the allocator.** Without `CountingAlloc` there is no live-memory check.
* **`panic=abort`.** A panic anywhere ends the process; a server has no supervisor built in.
* **Polyglot blocks are not sandboxed.** `@python { }` and friends run an installed interpreter as a subprocess
  with the OS account's permissions. The trust boundary is the user's OS account: the block can read and write any
  file the user can, open network connections and start programs. The capability flags `allow_network` and
  `allow_file_*` do not apply inside the block (only `allow_process_spawn` decides whether it starts at all). The
  environment scrub, private working directory, timeout and output cap are hygiene and accident protection, not a
  security boundary. There is no memory or CPU limit (on Unix an operator can wrap `killer` in `ulimit -v`; Windows
  would need a Job Object, which needs a new dependency, so it is not built).
* **External helper programs.** `curl`, `openssl`, `git` (and `powershell` on Windows) are trusted; a compromised one defeats TLS and HTTP safety.
* **No signatures** on release binaries, packages, or scripts. Checksums only detect tampering if the expected hash comes from an independent place.
* **No secrets management**: environment variables and files holding keys are readable by any script with those capabilities.
* **No audit logging, authentication or authorization** for servers written in Killer.
* The old planned features in `SECURITY.md` ("Planned" section: `@permission`, `--sandbox`, code signing) are **not built**.

## Polyglot blocks

What runs: the block body (taken verbatim from the source) in a file in a fresh 0700 temp directory, by a runtime
resolved to an absolute path. Defaults and switches are in [OPERATIONS.md](../../src/v2-rust/killer/docs/OPERATIONS.md).

* Policy order for every block: unknown language (error), process capability, `KILLER_POLYGLOT_ALLOW`,
  `KILLER_PROFILE=strict|prod` (default deny), then the one-time warning for the default allow. All are decided
  before any runtime is probed or started.
* Settings are read from the environment snapshot taken at first use (or just before the first `env_set`), so a
  script cannot widen the policy, raise the limits or turn on AI assist by changing its own environment.
* Data egress: the AI assist layer sends a failing block's code and stderr to the configured KhLM / LLM backend
  only with `KILLER_POLYGLOT_AI=1` and both the LLM and network capabilities.
* Audit: `KILLER_AUDIT_LOG` records every run (language, absolute runtime path, version, SHA-256 of the code, exit
  code, outcome); the code only with `KILLER_AUDIT_CODE=1`, the output never. The log is not tamper-evident.
* Windows Store app aliases (`%LOCALAPPDATA%\Microsoft\WindowsApps\python3.exe` and any path under a `WindowsApps`
  directory) start their children outside the block's Job Object, and once the packaged parent exits the
  parent-pid walk cannot link them, so a timeout cannot contain them. Runtime resolution therefore skips aliases
  while any regular interpreter on PATH runs; `killer polyglot list` and `killer doctor` print a warning when an
  alias is still used (`"app_alias":true` in `--json`), and a timeout or output-limit error then says the block's
  process was killed but child processes may still be running, with the `KILLER_POLYGLOT_<LANG>` hint. On
  Windows the error claims a killed tree only when the job was terminated and every descendant found was in it.
* Minimal deployment for scripts that need polyglot: an image with only `python3` and `bash`, read-only root file
  system, a non-root user, no network (`docker run --read-only --tmpfs /tmp --user 1000 --network none`),
  `KILLER_PROFILE=strict` and `KILLER_POLYGLOT_ALLOW=python,bash`.

## Known gaps

* Registry client and raw Ollama client bypass the SSRF guard.
* No fuzzing in CI beyond the seeded short runs in `tests/robustness.rs`; long runs are `#[ignore]`.
* Socket builtins (`tcp_connect`, `udp_send`) have no destination filter.
* Polyglot blocks: no memory / CPU limits, no per-block network or file restriction, daemonized grandchildren can
  survive the process-tree kill, and so can the children of a Windows Store alias runtime (the error says so).
* No dependency-vulnerability scan is run from this document's checks (CI has a `cargo audit` step per the tracker; not verified here).
