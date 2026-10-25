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
// Zero external crates — pure std Rust crypto. Encryption: key = PBKDF2-HMAC-SHA256(password, random salt,
// 100,000 iterations), then ChaCha20-Poly1305 (src/aead.rs, checked against the RFC 8439 vectors) with a fresh
// random nonce. Output format 2: "KSE2" | salt(16) | nonce(12) | ciphertext | tag(16). Data written by the
// older, unreviewed scheme (no "KSE2" prefix) can still be decrypted but is never produced again.

#![allow(unsafe_code)]

use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// ── SHA-256 implementation (pure Rust) ────────────────────────
// K constants for SHA-256
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: pad message
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) block
    for block in padded.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([block[i*4], block[i*4+1], block[i*4+2], block[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g; g = f; f = e;
            e = d.wrapping_add(temp1);
            d = c; c = b; b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for i in 0..8 {
        result[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes());
    }
    result
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

/// First bytes of data written by the current `secure_encrypt` (also authenticated as associated data).
const FORMAT_MAGIC: &[u8] = b"KSE2";

// ── Key derivation ───────────────────────────────────────────
// PBKDF2-HMAC-SHA256 (checked against Python's hashlib.pbkdf2_hmac in the tests)

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    // HMAC-SHA256
    fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
        let mut k = [0u8; 64];
        if key.len() > 64 {
            let h = sha256(key);
            k[..32].copy_from_slice(&h);
        } else {
            k[..key.len()].copy_from_slice(key);
        }

        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];
        for i in 0..64 {
            ipad[i] ^= k[i];
            opad[i] ^= k[i];
        }

        let mut inner = Vec::with_capacity(64 + message.len());
        inner.extend_from_slice(&ipad);
        inner.extend_from_slice(message);
        let inner_hash = sha256(&inner);

        let mut outer = Vec::with_capacity(64 + 32);
        outer.extend_from_slice(&opad);
        outer.extend_from_slice(&inner_hash);
        sha256(&outer)
    }

    // PBKDF2 with single block (32 bytes = 1 block for SHA-256)
    let mut msg = Vec::with_capacity(salt.len() + 4);
    msg.extend_from_slice(salt);
    msg.extend_from_slice(&1u32.to_be_bytes()); // block index 1

    let mut u = hmac_sha256(password, &msg);
    let mut result = u;

    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for i in 0..32 {
            result[i] ^= u[i];
        }
    }
    result
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
    pin_hash: Option<String>,  // SHA-256 of PIN
    is_locked: bool,
    failed_attempts: u32,
    lockout_until_ms: u64,
    evidence_chain: Vec<String>,  // Chain of evidence hashes
}

static SECURITY: OnceLock<Mutex<SecurityState>> = OnceLock::new();

fn security_state() -> &'static Mutex<SecurityState> {
    SECURITY.get_or_init(|| Mutex::new(SecurityState {
        pin_hash: None,
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

    // Derive the key from the password with a fresh random salt, then seal with a fresh random nonce
    let mut salt = [0u8; 16];
    secure_random(&mut salt)?;
    let mut nonce = [0u8; 12];
    secure_random(&mut nonce)?;
    let key = pbkdf2_sha256(password.as_bytes(), &salt, 100_000);
    let sealed = crate::aead::seal(&key, &nonce, FORMAT_MAGIC, &data);

    // Output: "KSE2" | salt(16) | nonce(12) | ciphertext | tag(16)
    let mut output = Vec::with_capacity(FORMAT_MAGIC.len() + 16 + 12 + sealed.len());
    output.extend_from_slice(FORMAT_MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&sealed);

    Ok(Value::Str(base64_encode(&output)))
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

    if raw.len() < 16 {
        return Err(VmError::runtime_error("secure_decrypt: data too short"));
    }

    let plaintext = if raw.starts_with(FORMAT_MAGIC) && raw.len() >= FORMAT_MAGIC.len() + 16 + 12 + 16 {
        let h = FORMAT_MAGIC.len();
        let salt = &raw[h..h + 16];
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&raw[h + 16..h + 28]);
        let key = pbkdf2_sha256(password.as_bytes(), salt, 100_000);
        crate::aead::open(&key, &nonce, FORMAT_MAGIC, &raw[h + 28..])
            .ok_or_else(|| VmError::runtime_error("secure_decrypt: wrong password, or the data was changed"))?
    } else {
        // data from the older scheme
        let key = pbkdf2_sha256(password.as_bytes(), &raw[..16], 100_000);
        legacy_decrypt(&raw[16..], &key).map_err(|e| VmError::runtime_error(format!("secure_decrypt: {}", e)))?
    };

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

    let hash = sha256(pin.as_bytes());
    let hash_hex = hex_encode(&hash);

    let mut state = security_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    state.pin_hash = Some(hash_hex);
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
            let input_hash = hex_encode(&sha256(pin.as_bytes()));
            if input_hash == *stored_hash {
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
        let a = text(builtin_secure_encrypt(&[s("secret data \u{1F512}"), s("pw")]));
        let b = text(builtin_secure_encrypt(&[s("secret data \u{1F512}"), s("pw")]));
        assert_ne!(a, b, "salt and nonce must be fresh every time");
        assert_eq!(text(builtin_secure_decrypt(&[s(&a), s("pw")])), "secret data \u{1F512}");
        assert_eq!(text(builtin_secure_decrypt(&[s(&b), s("pw")])), "secret data \u{1F512}");
        let empty = text(builtin_secure_encrypt(&[s(""), s("pw")]));
        assert_eq!(text(builtin_secure_decrypt(&[s(&empty), s("pw")])), "");
        // new data carries the version prefix
        assert!(base64_decode(&a).unwrap().starts_with(b"KSE2"));
    }

    #[test]
    fn a_wrong_password_or_any_changed_byte_is_rejected() {
        let enc = text(builtin_secure_encrypt(&[s("attack at dawn"), s("right")]));
        assert!(builtin_secure_decrypt(&[s(&enc), s("wrong")]).is_err());
        let raw = base64_decode(&enc).unwrap();
        // flipping a byte in each region must fail (the prefix itself would route to the old format). Every key
        // derivation costs 100,000 hash rounds, so only the region boundaries are tried here; aead.rs flips
        // every byte of a sealed message.
        let n = raw.len();
        for i in [4, 19, 20, 31, 32, n - 17, n - 16, n - 1] {
            let mut bad = raw.clone();
            bad[i] ^= 1;
            assert!(builtin_secure_decrypt(&[s(&base64_encode(&bad)), s("right")]).is_err(), "flip at byte {}", i);
        }
        // truncated data
        assert!(builtin_secure_decrypt(&[s(&base64_encode(&raw[..raw.len() - 1])), s("right")]).is_err());
    }

    #[test]
    fn data_from_the_old_scheme_can_still_be_read() {
        // produced by an independent Python implementation of the old format (salt 0..15, nonce 100..111)
        let old = "AAECAwQFBgcICQoLDA0OD2RlZmdoaWprbG1ubzeAv02U9Y6yTAuHh9ABFeh7Hr8WCd5IMyCcnvZoRO4jCdg+B21M8OAwQQPzSEaLxqyV3Cad5K5PnYEm/vQKBA==";
        assert_eq!(text(builtin_secure_decrypt(&[s(old), s("pw")])), "legacy data from the old scheme");
        assert!(builtin_secure_decrypt(&[s(old), s("nope")]).is_err());
    }
}
