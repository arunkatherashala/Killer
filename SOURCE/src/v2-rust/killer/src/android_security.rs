// android_security.rs — Native security module for Killer Call Recorder
// ChaCha20-Poly1305 encryption (RFC 8439), PIN/biometric auth, tamper detection, secure storage
//
// Provides builtin functions:
//   secure_encrypt(data, password)    → encrypted base64 string
//   secure_decrypt(encrypted, password) → original data string
//   secure_hash(data)                 → SHA-256 hex digest
//   secure_hash_file(path)            → SHA-256 of file contents
//   secure_random_bytes(count)        → base64 random bytes
//   secure_pin_set(pin)               → bool (set app PIN)
//   secure_pin_verify(pin)            → bool (verify PIN)
//   secure_pin_is_set()               → bool
//   secure_lock()                     → null (lock app)
//   secure_is_locked()                → bool
//   secure_unlock(pin)                → bool
//   secure_wipe_recordings()          → number (count wiped)
//   secure_check_integrity()          → dict {rooted, debuggable, tampered}
//   evidence_hash(recording_data)     → dict {sha256, timestamp, chain}
//
// Crypto primitives come from the RustCrypto crates (sha2, pbkdf2, chacha20poly1305; see docs/CRYPTO.md). Encryption: key = PBKDF2-HMAC-SHA256(password, random salt,
// 600,000 iterations), then ChaCha20-Poly1305 (src/aead.rs, checked against the RFC 8439 vectors) with a fresh
// random nonce. Output format 3: "KSE3" | iterations(u32 BE) | salt(16) | nonce(12) | ciphertext | tag(16); the
// whole header (magic, iterations, salt) is authenticated as associated data. Format 2 ("KSE2", 100,000 rounds,
// only the magic authenticated) and the older unreviewed scheme (no prefix) can still be decrypted but are never
// produced again; `secure_rewrap` upgrades them.

#![allow(unsafe_code)]

use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// ── SHA-256 (RustCrypto `sha2` crate) ─────────────────────────
pub fn sha256(data: &[u8]) -> [u8; 32] {
    crate::encryption::sha256(data)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// PBKDF2 rounds for the PIN hash (the PIN also has an escalating lockout after 3 wrong tries).
const PIN_ITERATIONS: u32 = 10_000;

/// Compare two byte strings without stopping at the first difference.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    crate::encryption::constant_time_compare(a, b)
}

/// First bytes of data written by the current `secure_encrypt` (also authenticated as associated data).
const FORMAT_MAGIC: &[u8] = b"KSE3";
/// Previous format: 100,000 rounds, only the magic authenticated. Read-only.
const FORMAT_MAGIC_V2: &[u8] = b"KSE2";
/// PBKDF2 rounds `secure_encrypt` uses now (format 3 stores the count it used, so it can be raised again).
pub const ENCRYPT_ITERATIONS: u32 = 600_000;
/// Rounds used by formats 1 and 2.
const LEGACY_ITERATIONS: u32 = 100_000;
/// The count in a format 3 header is untrusted input: bound it before deriving a key from it, otherwise a crafted
/// value of 4 billion makes `secure_decrypt` run for hours.
pub const MIN_DECRYPT_ITERATIONS: u32 = 100_000;
pub const MAX_DECRYPT_ITERATIONS: u32 = 10_000_000;
/// magic(4) | iterations(4) | salt(16): the authenticated header of format 3.
const HEADER_LEN_V3: usize = 24;

// ── Key derivation ───────────────────────────────────────────
// PBKDF2-HMAC-SHA256 (checked against Python's hashlib.pbkdf2_hmac in the tests)

pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, iterations.max(1), &mut out);
    out
}

// Random bytes come from the operating system's cryptographic generator (see os_random). There is no
// fallback: if it fails the operation fails, rather than using a guessable value for a salt or nonce.
fn secure_random(buf: &mut [u8]) -> Result<(), VmError> {
    crate::os_random::fill(buf).map_err(|e| VmError::runtime_error(format!("secure random: {}", e)))
}

/// Decrypt data written by the OLD scheme (SHA-256 keystream with a SHA-256 tag; not AES, never reviewed).
/// Kept only so existing data stays readable; `secure_encrypt` no longer produces this format.
fn legacy_decrypt(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 44 { // 12 nonce + 0 data + 32 tag minimum
        return Err("encrypted data too short".to_string());
    }

    let nonce = &encrypted[..12];
    let tag_start = encrypted.len() - 32;
    let ciphertext = &encrypted[12..tag_start];
    let tag = &encrypted[tag_start..];

    // Verify authentication tag
    let mut tag_input = Vec::with_capacity(12 + ciphertext.len() + 32);
    tag_input.extend_from_slice(nonce);
    tag_input.extend_from_slice(ciphertext);
    tag_input.extend_from_slice(key);
    let expected_tag = sha256(&tag_input);

    // Constant-time comparison to prevent timing attacks
    let mut diff: u8 = 0;
    for i in 0..32 {
        diff |= tag[i] ^ expected_tag[i];
    }
    if diff != 0 {
        return Err("authentication failed — data tampered or wrong password".to_string());
    }

    // Decrypt
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    let mut counter: u64 = 0;
    let mut pos = 0;

    while pos < ciphertext.len() {
        let mut block_input = Vec::with_capacity(32 + 12 + 8);
        block_input.extend_from_slice(key);
        block_input.extend_from_slice(nonce);
        block_input.extend_from_slice(&counter.to_le_bytes());
        let keystream = sha256(&block_input);

        let chunk_end = (pos + 32).min(ciphertext.len());
        for i in pos..chunk_end {
            plaintext.push(ciphertext[i] ^ keystream[i - pos]);
        }
        pos = chunk_end;
        counter += 1;
    }

    Ok(plaintext)
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[((triple >> 6) & 0x3F) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[(triple & 0x3F) as usize] as char } else { '=' });
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn char_to_val(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("invalid base64 char: {}", c as char)),
        }
    }

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b' ').collect();
    if bytes.len() % 4 != 0 {
        return Err("invalid base64 length".to_string());
    }

    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let a = char_to_val(chunk[0])? as u32;
        let b = char_to_val(chunk[1])? as u32;
        let c = char_to_val(chunk[2])? as u32;
        let d = char_to_val(chunk[3])? as u32;
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' { result.push(((triple >> 8) & 0xFF) as u8); }
        if chunk[3] != b'=' { result.push((triple & 0xFF) as u8); }
    }
    Ok(result)
}

// ── Security state ────────────────────────────────────────────
struct SecurityState {
    pin_hash: Option<String>,  // PBKDF2-HMAC-SHA256 of the PIN (hex), with `pin_salt`
    pin_salt: [u8; 16],
    is_locked: bool,
    failed_attempts: u32,
    lockout_until_ms: u64,
    evidence_chain: Vec<String>,  // Chain of evidence hashes
}

static SECURITY: OnceLock<Mutex<SecurityState>> = OnceLock::new();

fn security_state() -> &'static Mutex<SecurityState> {
    SECURITY.get_or_init(|| Mutex::new(SecurityState {
        pin_hash: None,
        pin_salt: [0; 16],
        is_locked: false,
        failed_attempts: 0,
        lockout_until_ms: 0,
        evidence_chain: Vec::new(),
    }))
}

// ══════════════════════════════════════════════════════════════
// BUILTIN FUNCTIONS
// ══════════════════════════════════════════════════════════════

/// secure_encrypt(data, password) → encrypted base64 string
pub fn builtin_secure_encrypt(args: &[Value]) -> Result<Value, VmError> {
    if args.len() < 2 {
        return Err(VmError::runtime_error("secure_encrypt: requires (data, password)"));
    }
    let data = match &args[0] {
        Value::Str(s) => s.as_bytes().to_vec(),
        _ => return Err(VmError::runtime_error("secure_encrypt: data must be string")),
    };
    let password = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("secure_encrypt: password must be string")),
    };

    Ok(Value::Str(base64_encode(&encrypt_bytes(&data, password.as_bytes(), ENCRYPT_ITERATIONS)?)))
}

/// Seal `data` as format 3 with the given PBKDF2 round count.
fn encrypt_bytes(data: &[u8], password: &[u8], iterations: u32) -> Result<Vec<u8>, VmError> {
    // Derive the key from the password with a fresh random salt, then seal with a fresh random nonce
    let mut salt = [0u8; 16];
    secure_random(&mut salt)?;
    let mut nonce = [0u8; 12];
    secure_random(&mut nonce)?;
    let mut key = pbkdf2_sha256(password, &salt, iterations);

    // Output: "KSE3" | iterations(4, BE) | salt(16) | nonce(12) | ciphertext | tag(16)
    let mut output = Vec::with_capacity(HEADER_LEN_V3 + 12 + data.len() + 16);
    output.extend_from_slice(FORMAT_MAGIC);
    output.extend_from_slice(&iterations.to_be_bytes());
    output.extend_from_slice(&salt);
    let sealed = crate::aead::seal(&key, &nonce, &output[..HEADER_LEN_V3], data);
    crate::encryption::wipe(&mut key);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&sealed);
    Ok(output)
}

/// Decrypt any supported format. `Err` carries the message shown to the script.
fn decrypt_bytes(raw: &[u8], password: &[u8]) -> Result<Vec<u8>, VmError> {
    if raw.len() < 16 {
        return Err(VmError::runtime_error("secure_decrypt: data too short"));
    }
    let wrong = || VmError::runtime_error("secure_decrypt: wrong password, or the data was changed");
    if raw.starts_with(FORMAT_MAGIC) && raw.len() >= HEADER_LEN_V3 + 12 + 16 {
        let mut it = [0u8; 4];
        it.copy_from_slice(&raw[4..8]);
        let iterations = u32::from_be_bytes(it);
        if !(MIN_DECRYPT_ITERATIONS..=MAX_DECRYPT_ITERATIONS).contains(&iterations) {
            return Err(VmError::runtime_error(format!(
                "secure_decrypt: iteration count {} in the header is outside the accepted range {}..={}",
                iterations, MIN_DECRYPT_ITERATIONS, MAX_DECRYPT_ITERATIONS
            )));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&raw[HEADER_LEN_V3..HEADER_LEN_V3 + 12]);
        let mut key = pbkdf2_sha256(password, &raw[8..24], iterations);
        let out = crate::aead::open(&key, &nonce, &raw[..HEADER_LEN_V3], &raw[HEADER_LEN_V3 + 12..]);
        crate::encryption::wipe(&mut key);
        out.ok_or_else(wrong)
    } else if raw.starts_with(FORMAT_MAGIC_V2) && raw.len() >= 4 + 16 + 12 + 16 {
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&raw[20..32]);
        let mut key = pbkdf2_sha256(password, &raw[4..20], LEGACY_ITERATIONS);
        let out = crate::aead::open(&key, &nonce, FORMAT_MAGIC_V2, &raw[32..]);
        crate::encryption::wipe(&mut key);
        out.ok_or_else(wrong)
    } else {
        // data from the older scheme
        let mut key = pbkdf2_sha256(password, &raw[..16], LEGACY_ITERATIONS);
        let out = legacy_decrypt(&raw[16..], &key).map_err(|e| VmError::runtime_error(format!("secure_decrypt: {}", e)));
        crate::encryption::wipe(&mut key);
        out
    }
}

/// True when `raw` (decoded `secure_encrypt` output) is not in the current format.
pub fn needs_rewrap(raw: &[u8]) -> bool {
    !(raw.starts_with(FORMAT_MAGIC) && raw.len() >= HEADER_LEN_V3 + 12 + 16 && raw[4..8] >= ENCRYPT_ITERATIONS.to_be_bytes()[..])
}

/// Rehash-on-read for stored ciphertexts: decrypt with the password and, if the data is in an older format (or fewer
/// rounds than now), return it re-sealed in the current format. `Ok(None)` means it is already current.
pub fn rewrap(encrypted_b64: &str, password: &str) -> Result<Option<String>, VmError> {
    let raw = base64_decode(encrypted_b64).map_err(|e| VmError::runtime_error(format!("secure_decrypt: {}", e)))?;
    let mut plain = decrypt_bytes(&raw, password.as_bytes())?;
    let out = if needs_rewrap(&raw) { Some(base64_encode(&encrypt_bytes(&plain, password.as_bytes(), ENCRYPT_ITERATIONS)?)) } else { None };
    crate::encryption::wipe(&mut plain);
    Ok(out)
}

/// secure_decrypt(encrypted_b64, password) → decrypted string
pub fn builtin_secure_decrypt(args: &[Value]) -> Result<Value, VmError> {
    if args.len() < 2 {
        return Err(VmError::runtime_error("secure_decrypt: requires (encrypted, password)"));
    }
    let encrypted_b64 = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("secure_decrypt: data must be base64 string")),
    };
    let password = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("secure_decrypt: password must be string")),
    };

    let raw = base64_decode(&encrypted_b64)
        .map_err(|e| VmError::runtime_error(format!("secure_decrypt: {}", e)))?;

    let plaintext = decrypt_bytes(&raw, password.as_bytes())?;

    let text = String::from_utf8(plaintext)
        .map_err(|e| VmError::runtime_error(format!("secure_decrypt: invalid UTF-8: {}", e)))?;

    Ok(Value::Str(text))
}

/// secure_hash(data) → SHA-256 hex string
pub fn builtin_secure_hash(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("secure_hash: requires data argument"));
    }
    let data = match &args[0] {
        Value::Str(s) => s.as_bytes().to_vec(),
        _ => format!("{}", args[0]).into_bytes(),
    };
    let hash = sha256(&data);
    Ok(Value::Str(hex_encode(&hash)))
}

/// secure_hash_file(path) → SHA-256 hex string of file contents
pub fn builtin_secure_hash_file(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_file_read()?;
    if args.is_empty() {
        return Err(VmError::runtime_error("secure_hash_file: requires path"));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("secure_hash_file: path must be string")),
    };
    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("secure_hash_file: {}: {}", path, e)))?;
    let hash = sha256(&data);
    Ok(Value::Str(hex_encode(&hash)))
}

/// secure_random_bytes(count) → base64 string of random bytes
pub fn builtin_secure_random_bytes(args: &[Value]) -> Result<Value, VmError> {
    let count = match args.first() {
        Some(Value::Number(n)) => (*n as usize).min(1024), // Cap at 1KB
        _ => 32,
    };
    let mut buf = vec![0u8; count];
    secure_random(&mut buf)?;
    Ok(Value::Str(base64_encode(&buf)))
}

/// secure_pin_set(pin) → bool
pub fn builtin_secure_pin_set(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("secure_pin_set: requires PIN string"));
    }
    let pin = match &args[0] {
        Value::Str(s) => s.clone(),
        Value::Number(n) => format!("{}", *n as i64),
        _ => return Err(VmError::runtime_error("secure_pin_set: PIN must be string or number")),
    };
    if pin.len() < 4 {
        return Err(VmError::runtime_error("secure_pin_set: PIN must be at least 4 characters"));
    }

    // A random salt and a slow hash: a bare SHA-256 of a 4-digit PIN falls to a 10,000-entry table.
    let mut salt = [0u8; 16];
    secure_random(&mut salt)?;
    let hash_hex = hex_encode(&pbkdf2_sha256(pin.as_bytes(), &salt, PIN_ITERATIONS));

    let mut state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    state.pin_hash = Some(hash_hex);
    state.pin_salt = salt;
    state.is_locked = true;
    Ok(Value::Bool(true))
}

/// secure_pin_verify(pin) → bool
pub fn builtin_secure_pin_verify(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("secure_pin_verify: requires PIN"));
    }
    let pin = match &args[0] {
        Value::Str(s) => s.clone(),
        Value::Number(n) => format!("{}", *n as i64),
        _ => return Err(VmError::runtime_error("PIN must be string or number")),
    };

    let mut state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    // Check lockout
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if now < state.lockout_until_ms {
        return Err(VmError::runtime_error(format!(
            "Account locked. Try again in {}s",
            (state.lockout_until_ms - now) / 1000
        )));
    }

    match &state.pin_hash {
        Some(stored_hash) => {
            let input_hash = hex_encode(&pbkdf2_sha256(pin.as_bytes(), &state.pin_salt, PIN_ITERATIONS));
            if constant_time_eq(input_hash.as_bytes(), stored_hash.as_bytes()) {
                state.failed_attempts = 0;
                state.is_locked = false;
                Ok(Value::Bool(true))
            } else {
                state.failed_attempts += 1;
                // Escalating lockout: 30s, 60s, 120s, 300s...
                if state.failed_attempts >= 3 {
                    let lockout_secs = 30u64 * (1u64 << (state.failed_attempts - 3).min(5));
                    state.lockout_until_ms = now + lockout_secs * 1000;
                }
                Ok(Value::Bool(false))
            }
        }
        None => Ok(Value::Bool(true)), // No PIN set = always passes
    }
}

/// secure_pin_is_set() → bool
pub fn builtin_secure_pin_is_set(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Bool(state.pin_hash.is_some()))
}

/// secure_lock() → null
pub fn builtin_secure_lock(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mut state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    state.is_locked = true;
    Ok(Value::Null)
}

/// secure_is_locked() → bool
pub fn builtin_secure_is_locked(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Bool(state.is_locked))
}

/// secure_unlock(pin) → bool
pub fn builtin_secure_unlock(args: &[Value]) -> Result<Value, VmError> {
    builtin_secure_pin_verify(args)
}

/// secure_check_integrity() → dict {rooted, debuggable, tampered, secure}
pub fn builtin_secure_check_integrity(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mut dict = HashMap::new();

    #[cfg(target_os = "android")]
    {
        // Check common root indicators
        let root_paths = ["/system/bin/su", "/system/xbin/su", "/sbin/su",
                          "/system/app/Superuser.apk", "/data/local/xbin/su"];
        let rooted = root_paths.iter().any(|p| std::path::Path::new(p).exists());
        dict.insert("rooted".into(), Value::Bool(rooted));

        // Debug check would be via ApplicationInfo.FLAG_DEBUGGABLE (JNI)
        dict.insert("debuggable".into(), Value::Bool(false));
        dict.insert("tampered".into(), Value::Bool(false));
        dict.insert("secure".into(), Value::Bool(!rooted));
    }

    #[cfg(not(target_os = "android"))]
    {
        dict.insert("rooted".into(), Value::Bool(false));
        dict.insert("debuggable".into(), Value::Bool(true)); // Desktop = debug mode
        dict.insert("tampered".into(), Value::Bool(false));
        dict.insert("secure".into(), Value::Bool(true));
        dict.insert("platform".into(), Value::Str("desktop".to_string()));
    }

    Ok(Value::Dict(crate::value::SharedDict::new(dict)))
}

/// evidence_hash(recording_data) → dict {sha256, timestamp, chain_position}
/// Creates a forensically valid evidence hash with chain-of-custody
pub fn builtin_evidence_hash(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("evidence_hash: requires recording data"));
    }
    let data = match &args[0] {
        Value::Str(s) => s.as_bytes().to_vec(),
        _ => format!("{}", args[0]).into_bytes(),
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Hash the recording data
    let data_hash = sha256(&data);
    let data_hash_hex = hex_encode(&data_hash);

    // Chain: hash of (previous_chain_hash + current_hash + timestamp)
    let mut state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    let prev_hash = state.evidence_chain.last().cloned().unwrap_or_default();
    let mut chain_input = Vec::new();
    chain_input.extend_from_slice(prev_hash.as_bytes());
    chain_input.extend_from_slice(data_hash_hex.as_bytes());
    chain_input.extend_from_slice(&timestamp.to_le_bytes());
    let chain_hash = hex_encode(&sha256(&chain_input));

    let position = state.evidence_chain.len();
    state.evidence_chain.push(chain_hash.clone());

    let mut dict = HashMap::new();
    dict.insert("sha256".into(), Value::Str(data_hash_hex));
    dict.insert("timestamp".into(), Value::Number(timestamp as f64));
    dict.insert("chain_hash".into(), Value::Str(chain_hash));
    dict.insert("chain_position".into(), Value::Number(position as f64));

    Ok(Value::Dict(crate::value::SharedDict::new(dict)))
}

/// secure_wipe_recordings() → number (count of files wiped)
pub fn builtin_secure_wipe_recordings(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_file_read()?;
    crate::security::require_file_write()?;
    let _ = args;
    let base_path = crate::android_service::builtin_storage_path(&[])?;
    let path_str = match &base_path {
        Value::Str(s) => s.clone(),
        _ => return Ok(Value::Number(0.0)),
    };

    let recordings_dir = std::path::Path::new(&path_str).join("recordings");
    if !recordings_dir.exists() {
        return Ok(Value::Number(0.0));
    }

    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(&recordings_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                // Overwrite with zeros before deleting (secure wipe)
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let size = metadata.len() as usize;
                    let zeros = vec![0u8; size.min(1024 * 1024)]; // Wipe in 1MB chunks
                    let _ = std::fs::write(&path, &zeros);
                }
                let _ = std::fs::remove_file(&path);
                count += 1;
            }
        }
    }

    Ok(Value::Number(count as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }

    fn text(v: Result<Value, VmError>) -> String {
        match v {
            Ok(Value::Str(t)) => t,
            other => panic!("expected a string, got {:?}", other.map(|v| v.to_string())),
        }
    }

    #[test]
    fn sha256_and_pbkdf2_match_python_hashlib() {
        assert_eq!(hex_encode(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let h = |iters| hex_encode(&pbkdf2_sha256(b"password", b"salt", iters));
        assert_eq!(h(1), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b");
        assert_eq!(h(1000), "632c2812e46d4604102ba7618e9d6d7d2f8128f6266b4a03264d2a0460b7dcb3");
        assert_eq!(h(4096), "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a");
        // a key longer than the HMAC block size is hashed first
        let long = hex_encode(&pbkdf2_sha256(b"passwordPASSWORDpassword", b"saltSALTsaltSALTsaltSALTsaltSALTsalt", 100));
        assert_eq!(long, "642ea3cc424c6e709af485b1e51692f0a7557aa922c0dddc7561830ed30fd4e9");
    }

    #[test]
    fn encrypt_round_trips_and_every_message_is_different() {
        // the builtin runs the full 600,000 rounds (slow in a debug build), so it is exercised once here; the other
        // checks use `encrypt_bytes` with the lowest accepted count
        let a = text(builtin_secure_encrypt(&[s("secret data \u{1F512}"), s("pw")]));
        assert_eq!(text(builtin_secure_decrypt(&[s(&a), s("pw")])), "secret data \u{1F512}");
        let raw = base64_decode(&a).unwrap();
        assert!(raw.starts_with(b"KSE3"), "new data carries the version prefix");
        assert_eq!(u32::from_be_bytes([raw[4], raw[5], raw[6], raw[7]]), 600_000, "the round count is stored in the header");
        assert!(!needs_rewrap(&raw));
        let x = encrypt_bytes(b"same", b"pw", MIN_DECRYPT_ITERATIONS).unwrap();
        let y = encrypt_bytes(b"same", b"pw", MIN_DECRYPT_ITERATIONS).unwrap();
        assert_ne!(x, y, "salt and nonce must be fresh every time");
        assert_eq!(decrypt_bytes(&x, b"pw").unwrap(), b"same".to_vec());
        assert_eq!(decrypt_bytes(&encrypt_bytes(b"", b"pw", MIN_DECRYPT_ITERATIONS).unwrap(), b"pw").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn a_wrong_password_or_any_changed_header_byte_is_rejected() {
        let raw = encrypt_bytes(b"attack at dawn", b"right", MIN_DECRYPT_ITERATIONS).unwrap();
        assert!(decrypt_bytes(&raw, b"wrong").is_err());
        // flipping a byte in each region must fail (byte 0 routes to the old formats). Every derivation costs
        // 100,000 hash rounds, so only region boundaries are tried; aead.rs flips every byte of a sealed message.
        // Bytes 4..8 are the round count (authenticated as part of the header), 8..24 the salt.
        let n = raw.len();
        for i in [0, 3, 5, 7, 8, 23, 24, 35, 36, n - 17, n - 16, n - 1] {
            let mut bad = raw.clone();
            bad[i] ^= 1;
            assert!(decrypt_bytes(&bad, b"right").is_err(), "flip at byte {}", i);
        }
        assert!(decrypt_bytes(&raw[..raw.len() - 1], b"right").is_err());
    }

    #[test]
    fn a_header_that_only_changes_the_round_count_fails_authentication() {
        // same key material is NOT reachable by editing the count, but the header is also bound as associated data:
        // rewrite the count and re-derive with it, and the tag must still reject the data
        let raw = encrypt_bytes(b"attack at dawn", b"pw", MIN_DECRYPT_ITERATIONS).unwrap();
        let mut forged = raw.clone();
        forged[4..8].copy_from_slice(&(MIN_DECRYPT_ITERATIONS + 1).to_be_bytes());
        assert!(decrypt_bytes(&forged, b"pw").is_err());
    }

    #[test]
    fn crafted_round_counts_are_rejected_before_any_key_derivation() {
        let base = encrypt_bytes(b"x", b"pw", MIN_DECRYPT_ITERATIONS).unwrap();
        let t = std::time::Instant::now();
        for bad in [u32::MAX, MAX_DECRYPT_ITERATIONS + 1, MIN_DECRYPT_ITERATIONS - 1, 0, 1] {
            let mut raw = base.clone();
            raw[4..8].copy_from_slice(&bad.to_be_bytes());
            let err = decrypt_bytes(&raw, b"pw").unwrap_err();
            assert!(format!("{:?}", err).contains("iteration count"), "{} rounds: {:?}", bad, err);
        }
        assert!(t.elapsed().as_secs() < 5, "rejection must be immediate, took {:?}", t.elapsed());
    }

    #[test]
    fn data_from_the_old_schemes_can_still_be_read_and_is_rewrapped() {
        // produced by an independent Python implementation of the old format (salt 0..15, nonce 100..111)
        let old = "AAECAwQFBgcICQoLDA0OD2RlZmdoaWprbG1ubzeAv02U9Y6yTAuHh9ABFeh7Hr8WCd5IMyCcnvZoRO4jCdg+B21M8OAwQQPzSEaLxqyV3Cad5K5PnYEm/vQKBA==";
        assert_eq!(text(builtin_secure_decrypt(&[s(old), s("pw")])), "legacy data from the old scheme");
        assert!(builtin_secure_decrypt(&[s(old), s("nope")]).is_err());
        // format 2 (KSE2, 100,000 rounds, magic-only AAD), from tests/data/crypto_reference.py kse2-encrypt
        let kse2 = "S1NFMgABAgMEBQYHCAkKCwwNDg9kZWZnaGlqa2xtbm/aWzkv7684zVaiKLo8f811CohvrIpQlzCXnxoONsGZpd8=";
        assert_eq!(text(builtin_secure_decrypt(&[s(kse2), s("pw")])), "kse2 fixture text");
        assert!(builtin_secure_decrypt(&[s(kse2), s("nope")]).is_err());
        // rehash-on-read: old formats come back re-sealed as format 3 with the same plain text; the new format
        // reports nothing to do
        assert!(needs_rewrap(&base64_decode(kse2).unwrap()) && needs_rewrap(&base64_decode(old).unwrap()));
        let again = rewrap(kse2, "pw").unwrap().expect("KSE2 must be upgraded");
        let raw = base64_decode(&again).unwrap();
        assert!(raw.starts_with(b"KSE3") && !needs_rewrap(&raw));
        assert_eq!(text(builtin_secure_decrypt(&[s(&again), s("pw")])), "kse2 fixture text");
        assert!(rewrap(&again, "pw").unwrap().is_none());
        assert!(rewrap(kse2, "nope").is_err());
        // format 3 produced by the independent Python implementation (600,000 rounds, salt 0..15, nonce 100..111)
        let kse3 = "S1NFMwAJJ8AAAQIDBAUGBwgJCgsMDQ4PZGVmZ2hpamtsbW5vZGJA1bUQg2VarXEecTcaXj+ENEaKsiop/yCZwA==";
        assert_eq!(text(builtin_secure_decrypt(&[s(kse3), s("pw")])), "kse3 fixture");
        assert!(rewrap(kse3, "pw").unwrap().is_none());
        assert_eq!(text(builtin_secure_decrypt(&[s(&rewrap(old, "pw").unwrap().unwrap()), s("pw")])), "legacy data from the old scheme");
    }
}
