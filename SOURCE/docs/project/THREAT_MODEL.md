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
| `http_download` writes to sensitive paths | canonical-path destination policy | commit 954ba86; unit tests in `src/` | list of protected locations is not exhaustive |

## What the controls do NOT protect

* **A script is not sandboxed by default.** If you run untrusted `.killer` files with `killer run`, they can do anything you can.
* **`eval`/`exec` run with the caller's powers.** The limits stop accidents and runaway code, not a determined attacker.
* **Memory cap only with the allocator.** Without `CountingAlloc` there is no live-memory check.
* **`panic=abort`.** A panic anywhere ends the process; a server has no supervisor built in.
* **External helper programs.** `curl`, `openssl`, `git` (and `powershell` on Windows) are trusted; a compromised one defeats TLS and HTTP safety.
* **No signatures** on release binaries, packages, or scripts. Checksums only detect tampering if the expected hash comes from an independent place.
* **No secrets management**: environment variables and files holding keys are readable by any script with those capabilities.
* **No audit logging, authentication or authorization** for servers written in Killer.
* The old planned features in `SECURITY.md` ("Planned" section: `@permission`, `--sandbox`, code signing) are **not built**.

## Known gaps

* Registry client and raw Ollama client bypass the SSRF guard.
* No fuzzing in CI beyond the seeded short runs in `tests/robustness.rs`; long runs are `#[ignore]`.
* Socket builtins (`tcp_connect`, `udp_send`) have no destination filter.
* No dependency-vulnerability scan is run from this document's checks (CI has a `cargo audit` step per the tracker; not verified here).
