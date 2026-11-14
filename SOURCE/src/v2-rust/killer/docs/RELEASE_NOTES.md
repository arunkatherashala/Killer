# Release notes (next release)

## Security: read this before upgrading

**Old encryption in the Rust crate was a fake. Treat anything it protected as unprotected.**
Before this release the `encryption` module of the `killer-native` crate (`EncryptionEngine`, `HashingEngine`,
the `EncryptionAlgorithm` / `KeyDerivationFunction` types) did not encrypt. It XOR-ed the text with the key while
labelling the result "AES-256-GCM" or "ChaCha20-Poly1305"; its authentication tag was a placeholder that `decrypt`
never checked, its "SHA-256" and "PBKDF2" were not those functions, and keys, salts and nonces came from a
time-seeded generator. `cryptography_solver::aes_encrypt` returned the plain text unchanged and its `sha256`
returned 32 zero bytes.

What this means for you:

* If you called that Rust API directly (as a library user) and stored or sent the output, **the data is readable by
  anyone who has it and anyone can alter it undetected**. Do not rely on it, and **rotate any secret that was
  protected only by it** (passwords, keys, tokens).
* The new code uses real ChaCha20-Poly1305, SHA-256, SHA-512 and PBKDF2 and **cannot decrypt the old output**:
  decrypt it first with the old version, then encrypt again with this one.
* AES-256-GCM, AES-256-CBC, SHA-3, Argon2, HKDF and scrypt are not implemented; asking for them is now an error
  instead of silently doing something else.
* The Killer language builtins `secure_encrypt` / `secure_decrypt` (format KSE2) were real ChaCha20-Poly1305 and
  are not affected. `secure_decrypt` can still read data written by an older, never-reviewed scheme (a SHA-256
  keystream with a SHA-256 tag, not AES); that data was not strong encryption, so **decrypt it and re-encrypt it with
  `secure_encrypt`**.
* Nothing else in the crate called the fake module, so Killer scripts that did not use it directly are not affected.

Other security fixes in this release:

* `eval` / `exec` ran with full trust, so a restricted script could use them to read files, use the network or run
  processes. They now inherit the caller's restrictions, stop after a step and time budget and refuse deep nesting.
  Never `eval` text you do not trust.
* TLS and HTTP helpers (`openssl`, `curl`) are now resolved to absolute paths, so a planted program in the current
  directory or a relative `PATH` entry is never run. `KILLER_OPENSSL` and `KILLER_CURL` can point at a specific
  absolute path.
* API keys no longer appear on a command line (vision calls) and the Windows TLS server no longer uses a fixed
  password for its temporary certificate bundle.
* `curl -k` (certificate checks off) is gone from the LLM web-search fetchers and from image download. Certificates are
  verified by default. Behind a proxy that replaces certificates, set `CURL_CA_BUNDLE` to the proxy's CA file. The
  only way to turn checks off is the explicit `insecure: true` option of `http_request` (also `embed` and streaming
  calls), which prints a warning to stderr naming the host each time.
* `hex_decode` panicked (crashing the interpreter) on odd-length or multi-byte input; malformed base64 was accepted;
  `uuid()` did not use the operating system's random generator; the PIN was an unsalted hash. All fixed.

## Behaviour changes

* `%` is floor modulo for numbers (`-7 % 3` is `2`). Use `rem(a, b)` for the truncated remainder.
* `i--` is a compile error (`--` starts a comment, so it did nothing); write `i -= 1`.
* `base64_decode` is stricter: malformed input and non-zero trailing bits are rejected.

## New

* TLS servers (`tls_listen`, `tls_accept`, HTTPS from `serve`), native GUI windows, hygienic macros, SQL joins and
  subqueries, WebAssembly strings and lists, TLS client sockets and `wss://`, `killer debug`, `killer wasm`.
* `+=` and `++` work inside one-line blocks such as `for s in xs { total += s }`.

## Not tested

* The live LLM provider tests (`docs/LLM_TESTING.md`) have never run against a real OpenAI, Anthropic or Ollama
  service. Offline tests against a local fake server pass.
* The Windows installer script `install.ps1` and a real tag run of the release workflow are untested.
* Native GUI windows were driven by tests but never looked at on a screen.
