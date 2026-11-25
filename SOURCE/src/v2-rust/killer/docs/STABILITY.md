# Stability and deprecation policy

Killer is **beta**. This page says what you can rely on, what may still change, and how a change is announced.
It describes the policy and what the repository actually does today; where a practice is not yet followed it says so.

## Versioning

* The crate version is the one in `Cargo.toml` (`killer-native` 1.0.0 at the time of writing). The release workflow
  checks that a release tag matches it (see [RELEASING.md](RELEASING.md)).
* The version number is **1.0.0 but the status is beta**: the number does not promise the strict semantic-versioning
  guarantees of a mature 1.x product. Until the status in [STATUS.md](STATUS.md) says otherwise, treat minor releases
  as allowed to contain the breaking changes listed below, always announced.
* Intended scheme once stable: patch = bug fixes only, minor = additions plus announced deprecations, major = removals.
  No release has been cut under this scheme yet (there is no tagged release run, see [FEATURES.md](FEATURES.md)).

## What is stable

Stable means: a change needs an entry under **Breaking** in `CHANGELOG.md` and a release-note paragraph.

* The core syntax documented in [LANGUAGE.md](LANGUAGE.md) (functions, classes, `if`/`for`/`while`, `try`/`catch`,
  `match`, closures, comprehensions, imports) and the meaning of operators, including floor `%`.
* The names and documented behaviour of builtins listed in [LANGUAGE.md](LANGUAGE.md) and
  [STDLIB_INDEX.md](STDLIB_INDEX.md). The index is generated (`docs/gen_stdlib_index.py`), so a removed name shows up in review.
* The `KSE2`/`KSE3` encrypted-blob formats and the `$pbkdf2-sha256$v=N$...` hash formats: old versions stay readable
  (see [CRYPTO.md](CRYPTO.md)).
* The documented environment variables in [OPERATIONS.md](OPERATIONS.md) marked "security" or "limits".

Evidence for "stable" is the test suite and `docs/check_examples.py` (every example in the docs runs). Nothing here is
a formal compatibility test against an older release: no such test exists.

## What is experimental (may change without a deprecation period)

* Anything the documentation marks partial in [FEATURES.md](FEATURES.md): `killer wasm`, the x86-64 JIT internals, the
  VS Code extension, installers.
* The AI layer: KhLM router, affect/imagination engines, vector memory, Kala UI, ghost VM, `kore`/Nova formats, GUI,
  `killer-mcp` message surface, LLM provider options.
* The Rust library API (`killer_native::*` modules). It is large, not curated, and about 190 files are not even
  compiled (see [STATUS.md](STATUS.md)). Embedders should expect breakage.
* Compiler internals: bytecode, optimizer, the `KILLER_NO_*` / `KILLER_X_*` switches (debugging aids).
* Error message text (tests match some of it, but it is not a contract).

## How breaking changes are announced

1. A `### Breaking` / `### Behaviour changes` entry in `CHANGELOG.md` and in `docs/RELEASE_NOTES.md`. Today the
   changelog has `Changed` and `Security` sections; a dedicated **Breaking** heading is adopted from the next release.
2. **One-release deprecation warning where feasible**: the old form keeps working and prints a warning for one
   release. This is *not* possible for security fixes and for changes that turn silent wrong behaviour into an error
   (the cases listed below were all immediate).
3. **Environment opt-outs for security tightening** where the old behaviour has legitimate uses, so a deployment can
   keep running while it migrates: `KILLER_ALLOW_PRIVATE_NET=1`, `KILLER_ALLOW_METADATA_NET=1`,
   `KILLER_ALLOW_INSECURE_REGISTRY=1`, `KILLER_TOOL_ALLOW=...`. These are the supported escape hatches and are
   expected to stay; they are listed with their security impact in [OPERATIONS.md](OPERATIONS.md). Not every
   change has one (for example `x--` and the process capability for `env()` have none).

## Behaviour changes users have already seen

All are in `CHANGELOG.md`, `docs/RELEASE_NOTES.md` or the git history. "Test" is a file that fails if the change is undone.

| Change | Before | Now | Opt-out | Test |
|---|---|---|---|---|
| `env()`, `args()`, `load_env()` need the process capability | usable by a restricted script | permission error under a restricted capability set | none (run with the process capability) | `tests/capabilities.rs` (sweep over all gated builtins; no test names `env()` alone) |
| `x--` / `i--` | silently parsed as a comment (endless loops) | compile error telling you to write `x -= 1` | none | `tests/one_line_blocks.rs` |
| `%` on numbers | truncated (`-7 % 3` was `-1`) | floor modulo (`2`); `rem(a, b)` is the old behaviour | use `rem` | `tests/modulo.rs` |
| SSRF guard on HTTP builtins | any address reachable | loopback and private ranges refused; link-local and cloud metadata always refused | `allow_private` option or `KILLER_ALLOW_PRIVATE_NET=1`; metadata only `KILLER_ALLOW_METADATA_NET=1` | `tests/ssrf_guard.rs` |
| Password hashing and `secure_encrypt` | PBKDF2 100,000 rounds, `KSE2` | 600,000 rounds, `KSE3` / `$pbkdf2-sha256$v=2$...`; old data still verifies/decrypts; counts above 10,000,000 rejected | none; `rewrap` / `verify_and_upgrade` migrate | `tests/crypto_vectors.rs` |
| `serve` handlers | one request at a time on the VM thread; handlers shared the script's globals | bounded worker pool (`workers`, `queue`, `503` when full); each worker has its own copy of the globals; `max_requests` counts across workers; `server_stop` drains up to `drain_timeout` | `workers: 1` (handler on the script's VM and data, one at a time, as before); share state with `mutex_*`, channels or a database | `tests/http_pool.rs`, `tests/tls_server.rs` |
| `http_serve` / `serve` | accept loop with no per-request limits | `read_timeout` 10 s, `request_timeout` 60 s, header and body caps, optional TLS | options of `serve` | `tests/net_ai.rs`, `tests/tls_server.rs` (`tests/http_server_tests.rs` only tests request parsing) |
| LLM tool calling | file and network tools callable | default deny; enable with `enable_tool(name)` or `KILLER_TOOL_ALLOW` | `KILLER_TOOL_ALLOW=*` | unit tests in `src/tool_calling.rs` (`tool_allowed_in`); no integration test |
| `eval` / `exec` | fresh VM with full trust, unbounded | caller's capabilities, 50M steps, 30 s, 8 nesting levels, 256 MB cap | `max_memory_mb`, `KILLER_EVAL_MAX_MEMORY` (raises the memory cap only) | `tests/eval_safety.rs`, `tests/eval_memory*.rs` |
| `curl -k` in web-search fetchers and image download | certificates not checked | verified | `insecure: true` on `http_request` only (prints a warning) | `tests/no_insecure_curl.rs` |
| `openssl` / `curl` lookup | bare name via PATH | absolute path only, relative PATH entries and cwd ignored | `KILLER_OPENSSL`, `KILLER_CURL` (absolute) | `tests/tool_resolution.rs` |
| Old Rust `encryption` module | XOR labelled as AES | real ChaCha20-Poly1305; cannot read old output | none | `tests/crypto_vectors.rs` |
| `base64_decode`, `hex_decode` | lenient / could panic | strict errors | none | `tests/crypto_vectors.rs` |
| Plain-HTTP package registries | accepted | loopback only | `KILLER_ALLOW_INSECURE_REGISTRY=1` | unit test `plain_http_registry_needs_opt_in_unless_loopback` in `src/pkg_registry.rs` |
| Dependencies | none | RustCrypto crates (`chacha20poly1305`, `sha2`, `hmac`, `pbkdf2`, `subtle`); `libc` on Unix | none | `Cargo.toml` |

The test column was checked by grepping the named file; it was not re-run for this page. Threat-by-threat proof is in `SOURCE/docs/project/THREAT_MODEL.md`.

## Known gaps in this policy

* No automated check that `CHANGELOG.md` was updated for a behaviour change.
* No old-release compatibility test suite.
* No published release has run under this policy yet.
