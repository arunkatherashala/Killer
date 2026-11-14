# Cryptography in Killer: what is implemented, what is verified, what is not

Killer has no external crates, so every primitive below is written in this crate. Each one that exists is checked
against published test vectors in `tests/crypto_vectors.rs` (integration tests, run against both the Rust functions and
the Killer builtins through the `killer` binary) and in unit tests next to the code. An independent Python
implementation (`tests/data/crypto_reference.py`: `hashlib`/`hmac` plus ChaCha20 and Poly1305 re-written from the RFC
with big integers) cross-checks 258 more generated inputs; that test is skipped with a message when Python is absent.

**Read this first.** This code has not been audited by a cryptographer. It makes no side-channel guarantees beyond the
two constant-time comparisons listed below, does not wipe keys from memory, and runs in an interpreter whose strings
may be copied freely. It is reasonable for tokens, checksums, protecting a file with a password, and teaching; do not
treat it as a replacement for an audited library (OpenSSL, libsodium, ring) for high-value secrets.

## Verified primitives

| Primitive | Where | Standard and vectors checked | Killer builtin |
|---|---|---|---|
| SHA-256 | `builtin::sha256_digest`, `android_security::sha256`, `ghost_vm_sha`, `encryption::sha256` | FIPS 180-4: `""`, `"abc"`, the 448-bit and 896-bit messages, one million `a`, and every padding boundary (55, 56, 63, 64, 65, 111 ... 129 bytes) in all five implementations | `sha256`, `sha256_bytes`, `secure_hash`, `secure_hash_file` |
| SHA-512 | `encryption::sha512` | FIPS 180-4: `""`, `"abc"`, 896-bit, one million `a`, boundaries of the 128-byte block | none |
| SHA-1 | `net_sock::sha1` (WebSocket handshake only, not for security) | FIPS 180: `""`, `"abc"`, 448-bit, 896-bit, one million `a`, boundaries | none |
| HMAC-SHA256 | `encryption::hmac_sha256`, `ghost_vm_sha::hmac_sha256`, `lang_builtins` | RFC 4231 test cases 1 to 7 (including the 131-byte keys hashed first and the truncated case 5), empty key and data | `hmac_sha256` |
| PBKDF2-HMAC-SHA256 | `encryption::pbkdf2_hmac_sha256`, `android_security::pbkdf2_sha256` | RFC 7914 section 11 (`passwd`/`salt` 1 iteration, `Password`/`NaCl` 80000 iterations, 64 bytes), the usual `password`/`salt` 1, 2, 4096 iteration vectors, the 40-byte and NUL-byte vectors, empty password and salt, multi-block output | used inside `secure_encrypt` |
| ChaCha20 | `aead` | RFC 8439 2.3.2 (block function), 2.4.2 (encryption), A.1 | used inside `secure_encrypt` |
| Poly1305 | `aead` | RFC 8439 2.5.2, A.3 vectors 1 to 9 (carry and `2^130 - 5` edge cases), split updates at every block boundary | used inside `secure_encrypt` |
| ChaCha20-Poly1305 AEAD | `aead::seal` / `aead::open`, `encryption::EncryptionEngine` | RFC 8439 2.8.2 and A.5, plus 10 lengths at block boundaries from an independent implementation | `secure_encrypt`, `secure_decrypt` |
| base64 | `lang_builtins`, `builtin` (`b64_*`), `compression_module`, `net_sock`, and the encoders in `image_gen`, `nova_*`, `vision`, `android_audio`, `net_tls_server` | RFC 4648 section 10 (`""`, `f`, `fo`, `foo`, `foob`, `fooba`, `foobar`), all 256 byte values, `+` and `/` | `base64_encode`, `base64_decode`, `b64_encode`, `b64_decode` |
| WebSocket accept key | `net_sock::accept_key` | RFC 6455 section 1.3 (`dGhlIHNhbXBsZSBub25jZQ==` gives `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`) | `ws_accept` / `ws_connect` |
| OS random | `os_random::fill` (`BCryptGenRandom` on Windows, `/dev/urandom` elsewhere, no fallback) | statistical sanity only (a random generator has no test vector) | `secure_random_bytes`, `uuid` |

Behaviour checked as well as vectors: a flipped bit in any byte of ciphertext or tag, a wrong or all-zero tag, a wrong
key, nonce or associated data, truncated and extended messages, and swapped blocks are all rejected and no plain text
is returned; derived keys change with the salt, the password and the iteration count; password hashes and
`secure_encrypt` output use a fresh random salt and nonce each time (2000 engine nonces and repeated `secure_encrypt`
calls were all distinct); keys that are not exactly 32 bytes are refused.

## `secure_encrypt(text, password)` and `secure_decrypt(data, password)`

`secure_encrypt` derives a 256-bit key with PBKDF2-HMAC-SHA256 (100,000 iterations, random 16-byte salt) and seals the
text with ChaCha20-Poly1305 under a fresh random 96-bit nonce. The result is base64 of
`"KSE2" | salt(16) | nonce(12) | ciphertext | tag(16)`; the `KSE2` header is also authenticated as associated data.
`secure_decrypt` verifies the tag before it produces any plain text and fails with one message for a wrong password or
changed data. The Python reference decrypts Killer output and Killer decrypts Python output (checked in the tests).

Limits:

* 100,000 iterations is below the 600,000 that OWASP currently suggests for PBKDF2-HMAC-SHA256; the cost is kept low
  because the pure-Rust hash is slow in unoptimised builds. Use a long random password.
* `secure_decrypt` still reads data written by the older scheme (no `KSE2` header: a SHA-256 keystream with a SHA-256
  tag, never reviewed). It is read-only; nothing writes that format any more.
* `secure_random_bytes(n)` silently caps `n` at 1024 and returns base64.
* `secure_pin_set` stores a salted PBKDF2 hash (10,000 iterations) of the PIN in process memory only; the lockout
  after 3 wrong attempts is what protects a 4-digit PIN, not the hash.

## Decoding strictness

* `b64_decode` and `compression_module::base64_decode` are strict RFC 4648: padding required, `=` only at the end,
  length a multiple of 4, unused bits zero, no characters outside the alphabet.
* `base64_decode` is deliberately lenient about whitespace, missing padding and the URL-safe alphabet (`-`, `_`), but
  rejects everything ambiguous: a lone trailing symbol, wrong padding, data after padding, non-zero trailing bits.
* `hex_decode` accepts upper and lower case and rejects odd length and non-hex characters.
* Decoding to a string fails if the bytes are not valid UTF-8 (`b64_decode`, `hex_decode`); `base64_decode` returns
  bytes in that case.

## Not implemented

These return an error rather than something that looks like the real thing:

* AES-256-GCM and AES-256-CBC (`encryption::EncryptionAlgorithm`, `cryptography_solver::aes_*`). Use ChaCha20-Poly1305.
* SHA-3, Argon2id, scrypt, HKDF (`encryption::HashingAlgorithm::SHA3_*`, `KeyDerivationFunction`).
* MD5, RSA, elliptic curves, digital signatures, Diffie-Hellman. TLS and HTTPS use the system `openssl`, not this code.

`random()` is not a cryptographic generator (use `secure_random_bytes` or `uuid`). Equality of secrets in your own
scripts with `==` is not constant time.

## What was wrong before this audit (all fixed)

* `encryption.rs` was a fake. `encrypt` XOR-ed the text with the key (labelled AES-256-GCM or ChaCha20-Poly1305), its
  "tag" was a 16-byte SHA-256 placeholder that `decrypt` never checked, `simulate_sha256` XOR-folded the input into 32
  bytes, `pbkdf2_simulate` was a few XORs, and keys, salts and nonces came from a time-seeded LCG. It now uses real
  ChaCha20-Poly1305, SHA-256, SHA-512, PBKDF2 and the OS generator, and refuses the algorithms it cannot do.
* `cryptography_solver::aes_encrypt` returned the plain text unchanged and `sha256` returned 32 zero bytes.
* `hex_decode` panicked (crashing the interpreter) on odd-length input or multi-byte characters.
* `b64_decode` accepted `====` (as a NUL byte), `Zg=a`, `Zh==` and other malformed input; `base64_decode` accepted a
  lone trailing symbol, `Zg=`, and non-zero trailing bits. `compression_module::base64_encode` omitted the padding.
* `uuid()` took its bytes from `RandomState`, whose keys are only incremented between calls; it now reads the OS
  generator.
* The PIN was a bare SHA-256 compared with `==`; it is now salted, slow and compared in constant time.

Source files that were never compiled (`src/stdlib_impl/*`, `src/security_module.rs`) contain placeholder
"simulated" hashes and encryption (a `hash * 31` "SHA-256", `format!("encrypted_{}", 0)`). They are not part of the
build and must not be wired in without being replaced.

## Constant time

The Poly1305 tag comparison in `aead::open`, `encryption::constant_time_compare` (used for password hashes) and the PIN
comparison OR together the XOR of every byte pair before deciding. That is established by reading the code, not by
timing measurements. The arithmetic in ChaCha20 and Poly1305 has no secret-dependent branches or table lookups; SHA-256
and PBKDF2 are plain integer code. None of this has been verified against cache or power side channels.

## Running the checks

```
cargo test --test crypto_vectors          # vectors, tamper tests, Killer builtins, Python cross-check
cargo test --lib aead encryption          # unit tests (RFC 8439 appendix A, FIPS vectors)
```

```killer
println(sha256("abc"))
println(hmac_sha256("Jefe", "what do ya want for nothing?"))
println(base64_encode("foobar"))
# expect: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
# expect: 5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
# expect: Zm9vYmFy
```
