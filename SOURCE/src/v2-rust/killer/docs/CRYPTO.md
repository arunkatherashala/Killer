# Cryptography in Killer: what is implemented, what is verified, what is not

Killer uses the RustCrypto crates `chacha20poly1305`, `sha2`, `hmac`, `pbkdf2` and `subtle` for every primitive below
except SHA-1 and base64, which are still written in this crate (SHA-1 stays hand-written on purpose: it is used only for the WebSocket handshake accept key in `net_sock::accept_key`, and no builtin exposes it). These
crates are the first external dependencies of the project apart from `libc` on Unix (system calls for the JIT). The code in this repository is a thin wrapper
(`aead`, `encryption`, `android_security`, `ghost_vm_sha`); the duplicate hand-written SHA-256, HMAC, PBKDF2, ChaCha20
and Poly1305 code was removed. Each primitive is checked against published test vectors in `tests/crypto_vectors.rs`
(integration tests, run against both the Rust functions and the Killer builtins through the `killer` binary) and in
unit tests next to the code, so the wiring and the wire formats are verified here. An independent Python
implementation (`tests/data/crypto_reference.py`: `hashlib`/`hmac` plus ChaCha20 and Poly1305 re-written from the RFC
with big integers) cross-checks 258 more generated inputs; that test is skipped with a message when Python is absent.

**Read this first.** The `chacha20poly1305` crate had an independent NCC Group security audit ([crate README](https://github.com/RustCrypto/AEADs/tree/master/chacha20poly1305#readme), [NCC report](https://research.nccgroup.com/2020/02/26/public-report-rustcrypto-aes-gcm-and-chacha20poly1305-implementation-review/); 2020, together with
`aes-gcm`; it did not cover Killer, and the versions in use today are newer than the audited ones). `sha2`, `hmac`,
`pbkdf2` and `subtle` are widely used and reviewed RustCrypto implementations; this document does not call them audited. Killer's own integration, its blob format and the interpreter around it
have not been audited by a cryptographer, and this document makes no side-channel or key-wiping claims beyond what the
crates themselves document. Strings in the interpreter may be copied freely, so secrets are not wiped from memory. It
is reasonable for tokens, checksums, protecting a file with a password, and teaching; for high-value secrets use a
reviewed, purpose-built tool and review the integration yourself.

## Verified primitives

| Primitive | Where | Standard and vectors checked | Killer builtin |
|---|---|---|---|
| SHA-256 | `builtin::sha256_digest`, `android_security::sha256`, `ghost_vm_sha`, `encryption::sha256` | FIPS 180-4: `""`, `"abc"`, the 448-bit and 896-bit messages, one million `a`, and every padding boundary (55, 56, 63, 64, 65, 111 ... 129 bytes) through every wrapper in this crate | `sha256`, `sha256_bytes`, `secure_hash`, `secure_hash_file` |
| SHA-512 | `encryption::sha512` | FIPS 180-4: `""`, `"abc"`, 896-bit, one million `a`, boundaries of the 128-byte block | none |
| SHA-1 | `net_sock::sha1` (WebSocket handshake only, not for security) | FIPS 180: `""`, `"abc"`, 448-bit, 896-bit, one million `a`, boundaries | none |
| HMAC-SHA256 | `encryption::hmac_sha256`, `ghost_vm_sha::hmac_sha256`, `lang_builtins` | RFC 4231 test cases 1 to 7 (including the 131-byte keys hashed first and the truncated case 5), empty key and data | `hmac_sha256` |
| PBKDF2-HMAC-SHA256 | `encryption::pbkdf2_hmac_sha256`, `android_security::pbkdf2_sha256` | RFC 7914 section 11 (`passwd`/`salt` 1 iteration, `Password`/`NaCl` 80000 iterations, 64 bytes), the usual `password`/`salt` 1, 2, 4096 iteration vectors, the 40-byte and NUL-byte vectors, empty password and salt, multi-block output | used inside `secure_encrypt` |
| ChaCha20 | `aead` (`chacha20poly1305` crate) | RFC 8439 2.3.2 (block function), 2.4.2 (encryption), A.1 | used inside `secure_encrypt` |
| Poly1305 | `aead` (`chacha20poly1305` crate) | RFC 8439 2.5.2, A.3 vectors 1 to 9 (carry and `2^130 - 5` edge cases), split updates at every block boundary | used inside `secure_encrypt` |
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

`secure_encrypt` derives a 256-bit key with PBKDF2-HMAC-SHA256 (600,000 iterations, random 16-byte salt) and seals the
text with ChaCha20-Poly1305 under a fresh random 96-bit nonce. The result is base64 of
`"KSE3" | iterations(u32, big endian) | salt(16) | nonce(12) | ciphertext | tag(16)`; the whole 24-byte header (magic,
iteration count, salt) is authenticated as associated data, so editing any of it makes decryption fail. The iteration
count lives in the header so it can be raised later without another format change. On decrypt the stored count is
untrusted: values outside 100,000..=10,000,000 are rejected before any key derivation (a crafted header cannot make
`secure_decrypt` run for hours).
`secure_decrypt` verifies the tag before it produces any plain text and fails with one message for a wrong password or
changed data. The Python reference decrypts Killer output and Killer decrypts Python output (checked in the tests).

Limits:

* Format history: `KSE3` (current, 600,000 rounds, whole header authenticated); `KSE2` (100,000 rounds, only the magic
  authenticated); and the original scheme (no header: a SHA-256 keystream with a SHA-256 tag, never reviewed).
  `secure_decrypt` reads all three; nothing writes the older two. Migration is explicit: the Rust function
  `android_security::rewrap(ciphertext, password)` decrypts and re-seals old data as `KSE3` (`None` when it is already
  current), so a store can upgrade each record the next time the password is available.
* A hash at 600,000 rounds takes about 7 seconds in an unoptimised (debug) build and a small fraction of that in a
  release build (`cargo test --lib encryption::tests::new_hashes -- --nocapture` prints the time on your machine).
  Use a long random password.
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

## `encryption.rs` formats and key management

* Password hashes: version 2 (written now) is `$pbkdf2-sha256$v=2$i=600000$<salt hex>$<hash hex>`; version 1 (earlier
  releases, `$pbkdf2$100000$<salt>$<hash>`) still verifies. `PasswordHash::parse` reads both,
  `PasswordHash::needs_rehash` flags version 1, and `PasswordHasher::verify_and_upgrade` returns a fresh version 2 hash
  after a successful check so the caller can store it. A stored hash is untrusted input: `verify_password` rejects
  iteration counts above 10,000,000, version 2 hashes below 600,000 (downgrade), version 1 hashes below 1,000, unknown
  versions, and absurd salt or hash lengths, before it derives anything.
* `EncryptedData` carries a `version`. Version 2 (written now) authenticates the header (version, algorithm, key
  version, iteration count, salt) as associated data; version 1 (no associated data) still decrypts. Changing the
  version field of a version 2 value to 1 does not bypass the check: the tag no longer matches.
* `KeyManager::rotate_key` starts a new key version and keeps the old ones, so earlier data still decrypts:
  `KeyManager::encrypt` records the key version in the (authenticated) header and `KeyManager::decrypt` picks the matching
  key. A key marked compromised still decrypts old data (so it can be migrated) but is refused for new data.
* Key material is wiped (best effort) with volatile writes and a compiler fence (`encryption::wipe`): derived keys and
  the 32-byte copies used for sealing, and every stored key when a `KeyManager` is dropped. This is best effort, not a
  guarantee: copies handed out by `get_key` belong to the caller, the OS may page memory out, and the interpreter copies
  strings freely. No dependency on `zeroize` was added.

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

The Poly1305 tag comparison inside `aead::open` is done by the `chacha20poly1305` crate. The password-hash, PIN and
Ghost capsule signature comparisons (`encryption::constant_time_compare`, used by `android_security` and `ghost_vm`)
use the `subtle` crate's constant-time equality. Beyond what those crates document, nothing here has been verified
against timing, cache or power side channels.

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
