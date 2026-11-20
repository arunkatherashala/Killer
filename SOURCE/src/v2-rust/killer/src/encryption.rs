// Encryption module: authenticated encryption, hashing, password hashing and key management.
//
// Everything here is real and checked against published test vectors (tests/crypto_vectors.rs):
//   * encryption: ChaCha20-Poly1305 (RFC 8439) through `crate::aead`, with a fresh random 96-bit nonce per message
//   * hashing: SHA-256 (FIPS 180-4) and SHA-512 (FIPS 180-4)
//   * password hashing: PBKDF2-HMAC-SHA256 (RFC 8018) with a random salt
//   * randomness: the operating system generator through `crate::os_random`, never a time-seeded one
//
// What is NOT implemented, and says so with an error instead of pretending: AES-256-GCM, AES-256-CBC, SHA-3,
// Argon2id, HKDF and scrypt. This code has not been audited and makes no side-channel guarantees beyond a
// constant-time tag and password-hash comparison.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Encryption algorithm. Only `ChaCha20Poly1305` is implemented; the AES variants return an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    AES256GCM,  // not implemented
    AES256CBC,  // not implemented
    ChaCha20Poly1305,
}

impl std::fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionAlgorithm::AES256GCM => write!(f, "AES-256-GCM"),
            EncryptionAlgorithm::AES256CBC => write!(f, "AES-256-CBC"),
            EncryptionAlgorithm::ChaCha20Poly1305 => write!(f, "ChaCha20-Poly1305"),
        }
    }
}

/// Hashing algorithm. SHA-256 and SHA-512 are implemented; the SHA-3 variants return an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashingAlgorithm {
    SHA256,
    SHA512,
    SHA3_256,
    SHA3_512,
}

impl std::fmt::Display for HashingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashingAlgorithm::SHA256 => write!(f, "SHA-256"),
            HashingAlgorithm::SHA512 => write!(f, "SHA-512"),
            HashingAlgorithm::SHA3_256 => write!(f, "SHA3-256"),
            HashingAlgorithm::SHA3_512 => write!(f, "SHA3-512"),
        }
    }
}

/// Key derivation function. Only `PBKDF2` is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    PBKDF2,
    Argon2id,    // not implemented
    HKDF,        // not implemented
    Scrypt,      // not implemented
}

impl std::fmt::Display for KeyDerivationFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyDerivationFunction::PBKDF2 => write!(f, "PBKDF2"),
            KeyDerivationFunction::Argon2id => write!(f, "Argon2id"),
            KeyDerivationFunction::HKDF => write!(f, "HKDF"),
            KeyDerivationFunction::Scrypt => write!(f, "Scrypt"),
        }
    }
}

/// Encrypted data container
#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,        // 12 random bytes, new for every message
    pub tag: Vec<u8>,          // 16-byte Poly1305 authentication tag
    pub salt: Vec<u8>,         // for key derivation (empty when the caller supplies a raw key)
    pub algorithm: EncryptionAlgorithm,
    pub iteration_count: u32,  // for KDF
    /// Container version. 1 = legacy (no associated data); 2 = the header (version, algorithm, key version,
    /// KDF iterations, salt) is authenticated as associated data. New data is always written as version 2.
    pub version: u8,
    /// Version of the managed key that sealed this message (0 when the caller supplied a raw key).
    pub key_version: u32,
}

/// Container version written by `EncryptionEngine::encrypt`.
pub const ENCRYPTED_DATA_VERSION: u8 = 2;

impl EncryptedData {
    /// The bytes authenticated as associated data for version 2: a change to the version, algorithm, key
    /// version, iteration count or salt makes `decrypt` fail instead of being silently ignored.
    fn header_aad(&self) -> Vec<u8> {
        let mut aad = Vec::with_capacity(24 + self.salt.len());
        aad.extend_from_slice(b"KENC");
        aad.push(self.version);
        aad.push(match self.algorithm {
            EncryptionAlgorithm::AES256GCM => 1,
            EncryptionAlgorithm::ChaCha20Poly1305 => 2,
            EncryptionAlgorithm::AES256CBC => 3,
        });
        aad.extend_from_slice(&self.key_version.to_be_bytes());
        aad.extend_from_slice(&self.iteration_count.to_be_bytes());
        aad.extend_from_slice(&(self.salt.len() as u32).to_be_bytes());
        aad.extend_from_slice(&self.salt);
        aad
    }
}

/// Best-effort wipe of key material: volatile writes plus a compiler fence so the stores are not optimised away.
/// This does NOT guarantee that no copy survives (the OS may have paged the memory out, and `Vec` growth or
/// moves can leave earlier copies); it only clears the buffer we own.
#[allow(unsafe_code)]
pub fn wipe(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        // SAFETY: `b` is a valid, aligned, exclusive reference to a byte.
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// Key bytes that are wiped (best effort) when dropped.
struct Secret(Vec<u8>);

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

impl EncryptedData {
    pub fn to_hex(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            hex_encode(&self.ciphertext),
            hex_encode(&self.nonce),
            hex_encode(&self.tag),
            hex_encode(&self.salt)
        )
    }
}

/// Password hash with metadata
#[derive(Debug, Clone)]
pub struct PasswordHash {
    pub hash: Vec<u8>,
    pub salt: Vec<u8>,
    pub algorithm: KeyDerivationFunction,
    pub iterations: u32,
    pub memory: u32,  // for Argon2 (unused)
    /// 1 = legacy (100,000 rounds; text form `$pbkdf2$<iterations>$<salt>$<hash>`),
    /// 2 = current (`$pbkdf2-sha256$v=2$i=<iterations>$<salt>$<hash>`, at least 600,000 rounds).
    pub version: u8,
}

/// Iteration bounds for verification. A stored hash is untrusted input: without an upper bound a crafted
/// `$pbkdf2$4294967295$..` makes `verify_password` spin for hours (denial of service).
pub const PBKDF2_MAX_ITERATIONS: u32 = 10_000_000;
/// Lowest count accepted for a legacy (version 1) hash. Version 2 hashes must carry at least `PBKDF2_ITERATIONS`.
pub const PBKDF2_MIN_LEGACY_ITERATIONS: u32 = 1_000;
/// Iterations of version 1 hashes (what earlier releases wrote).
pub const PBKDF2_LEGACY_ITERATIONS: u32 = 100_000;

impl PasswordHash {
    pub fn to_string(&self) -> String {
        match self.version {
            1 => format!(
                "${}${}${}${}",
                match self.algorithm {
                    KeyDerivationFunction::Argon2id => "argon2",
                    KeyDerivationFunction::PBKDF2 => "pbkdf2",
                    _ => "unknown",
                },
                self.iterations,
                hex_encode(&self.salt),
                hex_encode(&self.hash)
            ),
            v => format!("$pbkdf2-sha256$v={}$i={}${}${}", v, self.iterations, hex_encode(&self.salt), hex_encode(&self.hash)),
        }
    }

    /// Parse either text form. Does not check the iteration bounds (`verify_password` does).
    pub fn parse(text: &str) -> Result<PasswordHash, String> {
        let parts: Vec<&str> = text.split('$').collect();
        let num = |s: &str| s.parse::<u32>().map_err(|_| format!("bad number in password hash: {}", s));
        let (version, iterations, salt, hash) = match parts.as_slice() {
            ["", "pbkdf2", i, salt, hash] => (1u8, num(i)?, *salt, *hash),
            ["", "pbkdf2-sha256", v, i, salt, hash] => {
                let v = v.strip_prefix("v=").ok_or("missing version")?;
                let i = i.strip_prefix("i=").ok_or("missing iteration count")?;
                let v = num(v)?;
                if v != 2 {
                    return Err(format!("unsupported password hash version {}", v));
                }
                (2u8, num(i)?, *salt, *hash)
            }
            _ => return Err("unrecognised password hash format".to_string()),
        };
        Ok(PasswordHash {
            hash: hex_decode(hash)?,
            salt: hex_decode(salt)?,
            algorithm: KeyDerivationFunction::PBKDF2,
            iterations,
            memory: 0,
            version,
        })
    }

    /// True when this hash is older than what new hashes use and should be rewritten after a successful check.
    pub fn needs_rehash(&self) -> bool {
        self.version < 2 || self.iterations < PBKDF2_ITERATIONS
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return Err("bad hex in password hash".to_string());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| "bad hex in password hash".to_string()))
        .collect()
}

/// Encryption engine
pub struct EncryptionEngine {
    algorithm: EncryptionAlgorithm,
}

fn not_implemented_cipher(algorithm: EncryptionAlgorithm) -> String {
    format!("{} is not implemented; use ChaCha20-Poly1305", algorithm)
}

impl EncryptionEngine {
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        EncryptionEngine { algorithm }
    }

    pub fn default() -> Self {
        EncryptionEngine::new(EncryptionAlgorithm::ChaCha20Poly1305)
    }

    /// Encrypt and authenticate `plaintext` under a 32-byte `key` with a fresh random nonce.
    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<EncryptedData, String> {
        if self.algorithm != EncryptionAlgorithm::ChaCha20Poly1305 {
            return Err(not_implemented_cipher(self.algorithm));
        }
        self.encrypt_with(plaintext, key, 0)
    }

    fn encrypt_with(&self, plaintext: &[u8], key: &[u8], key_version: u32) -> Result<EncryptedData, String> {
        let mut key: [u8; 32] = key.try_into().map_err(|_| "Key must be exactly 32 bytes".to_string())?;
        let mut nonce = [0u8; 12];
        crate::os_random::fill(&mut nonce)?;
        let mut enc = EncryptedData {
            ciphertext: vec![],
            nonce: nonce.to_vec(),
            tag: vec![],
            salt: vec![],
            algorithm: self.algorithm,
            iteration_count: 1,
            version: ENCRYPTED_DATA_VERSION,
            key_version,
        };
        let mut sealed = crate::aead::seal(&key, &nonce, &enc.header_aad(), plaintext);
        wipe(&mut key);
        enc.tag = sealed.split_off(sealed.len() - 16);
        enc.ciphertext = sealed;
        Ok(enc)
    }

    /// Verify the tag, then decrypt. Any change to the ciphertext, nonce, tag or key is an error.
    pub fn decrypt(&self, encrypted: &EncryptedData, key: &[u8]) -> Result<Vec<u8>, String> {
        if self.algorithm != EncryptionAlgorithm::ChaCha20Poly1305 || encrypted.algorithm != self.algorithm {
            return Err(not_implemented_cipher(if encrypted.algorithm != self.algorithm { encrypted.algorithm } else { self.algorithm }));
        }
        let mut key: [u8; 32] = key.try_into().map_err(|_| "Key must be exactly 32 bytes".to_string())?;
        let nonce: [u8; 12] = encrypted.nonce.as_slice().try_into().map_err(|_| "Nonce must be 12 bytes".to_string())?;
        if encrypted.tag.len() != 16 {
            return Err("Tag must be 16 bytes".to_string());
        }
        // version 1 (written before the header was authenticated) has no associated data; version 2 binds the header
        let aad = match encrypted.version {
            0 | 1 => Vec::new(),
            ENCRYPTED_DATA_VERSION => encrypted.header_aad(),
            v => {
                wipe(&mut key);
                return Err(format!("Unsupported encrypted data version {}", v));
            }
        };
        let mut sealed = encrypted.ciphertext.clone();
        sealed.extend_from_slice(&encrypted.tag);
        let out = crate::aead::open(&key, &nonce, &aad, &sealed);
        wipe(&mut key);
        out.ok_or_else(|| "Authentication failed: wrong key or the data was changed".to_string())
    }
}

/// Hashing engine
pub struct HashingEngine {
    algorithm: HashingAlgorithm,
}

impl HashingEngine {
    pub fn new(algorithm: HashingAlgorithm) -> Self {
        HashingEngine { algorithm }
    }

    pub fn hash(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self.algorithm {
            HashingAlgorithm::SHA256 => Ok(sha256(data).to_vec()),
            HashingAlgorithm::SHA512 => Ok(sha512(data).to_vec()),
            other => Err(format!("{} is not implemented", other)),
        }
    }

    pub fn verify(&self, data: &[u8], hash: &[u8]) -> Result<bool, String> {
        Ok(constant_time_compare(&self.hash(data)?, hash))
    }
}

/// Password hasher
pub struct PasswordHasher {
    kdf: KeyDerivationFunction,
}

/// PBKDF2 rounds for new password hashes (OWASP's figure for HMAC-SHA256). Older hashes (100,000 rounds, version 1)
/// store their own count, still verify, and report `needs_rehash()`; `verify_and_upgrade` returns the replacement.
pub const PBKDF2_ITERATIONS: u32 = 600_000;

impl PasswordHasher {
    pub fn new(kdf: KeyDerivationFunction) -> Self {
        PasswordHasher { kdf }
    }

    pub fn hash_password(&self, password: &str) -> Result<PasswordHash, String> {
        match self.kdf {
            KeyDerivationFunction::PBKDF2 => {
                let mut salt = vec![0u8; 16];
                crate::os_random::fill(&mut salt)?;
                let hash = pbkdf2_hmac_sha256(password.as_bytes(), &salt, PBKDF2_ITERATIONS, 32);
                Ok(PasswordHash { hash, salt, algorithm: KeyDerivationFunction::PBKDF2, iterations: PBKDF2_ITERATIONS, memory: 0, version: 2 })
            }
            other => Err(format!("{} is not implemented; use PBKDF2", other)),
        }
    }

    pub fn verify_password(&self, password: &str, hash: &PasswordHash) -> Result<bool, String> {
        if hash.algorithm != KeyDerivationFunction::PBKDF2 || self.kdf != KeyDerivationFunction::PBKDF2 {
            return Err("only PBKDF2 hashes can be verified".to_string());
        }
        // The stored hash is untrusted input: bound every size before doing any work with it.
        if hash.version > 2 {
            return Err(format!("unsupported password hash version {}", hash.version));
        }
        let floor = if hash.version >= 2 { PBKDF2_ITERATIONS } else { PBKDF2_MIN_LEGACY_ITERATIONS };
        if hash.iterations > PBKDF2_MAX_ITERATIONS || hash.iterations < floor {
            return Err(format!(
                "password hash iteration count {} is outside the accepted range {}..={}",
                hash.iterations, floor, PBKDF2_MAX_ITERATIONS
            ));
        }
        if !(16..=64).contains(&hash.hash.len()) || hash.salt.len() < 8 || hash.salt.len() > 1024 {
            return Err("password hash has an invalid salt or hash length".to_string());
        }
        let mut computed = pbkdf2_hmac_sha256(password.as_bytes(), &hash.salt, hash.iterations, hash.hash.len());
        // Constant-time comparison to prevent timing attacks
        let ok = constant_time_compare(&computed, &hash.hash);
        wipe(&mut computed);
        Ok(ok)
    }

    /// Verify, and when the password is right but the hash is older than the current format, also return a fresh
    /// current-version hash for the caller to store in its place (`None` when nothing needs to change).
    pub fn verify_and_upgrade(&self, password: &str, hash: &PasswordHash) -> Result<(bool, Option<PasswordHash>), String> {
        let ok = self.verify_password(password, hash)?;
        if ok && hash.needs_rehash() {
            return Ok((true, Some(self.hash_password(password)?)));
        }
        Ok((ok, None))
    }
}

/// Key manager. Every id keeps all of its key versions (oldest first, the last one is current): `rotate_key`
/// starts a new version for new data but never discards the old one, so data sealed earlier still decrypts.
/// Key bytes are wiped (best effort) when a manager is dropped.
pub struct KeyManager {
    keys: Arc<Mutex<HashMap<String, Vec<Secret>>>>,
    key_metadata: Arc<Mutex<HashMap<String, KeyMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_id: String,
    pub created_at: SystemTime,
    pub rotated_at: Option<SystemTime>,
    pub algorithm: EncryptionAlgorithm,
    pub status: KeyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Active,
    Rotated,
    Compromised,
    Deprecated,
}

impl KeyManager {
    pub fn new() -> Self {
        KeyManager {
            keys: Arc::new(Mutex::new(HashMap::new())),
            key_metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn generate_key(&self, key_id: String, algorithm: EncryptionAlgorithm) -> Result<(), String> {
        // 32 random bytes from the operating system generator
        let key = Secret(generate_random_bytes(32)?);

        // generating over an existing id replaces it; the old versions are wiped by Drop
        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .insert(key_id.clone(), vec![key]);

        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            created_at: SystemTime::now(),
            rotated_at: None,
            algorithm,
            status: KeyStatus::Active,
        };

        self.key_metadata
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .insert(key_id, metadata);

        Ok(())
    }

    /// The current key (a copy: the caller owns it and should `wipe` it when done).
    pub fn get_key(&self, key_id: &str) -> Result<Vec<u8>, String> {
        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get(key_id)
            .and_then(|v| v.last())
            .map(|k| k.0.clone())
            .ok_or_else(|| format!("Key not found: {}", key_id))
    }

    /// Number of the current key version (1 for a fresh key, +1 per rotation).
    pub fn current_key_version(&self, key_id: &str) -> Result<u32, String> {
        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get(key_id)
            .map(|v| v.len() as u32)
            .ok_or_else(|| format!("Key not found: {}", key_id))
    }

    /// A specific older (or the current) version of a key, 1-based.
    pub fn get_key_version(&self, key_id: &str, version: u32) -> Result<Vec<u8>, String> {
        self.keys
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get(key_id)
            .and_then(|v| v.get((version as usize).wrapping_sub(1)))
            .map(|k| k.0.clone())
            .ok_or_else(|| format!("Key not found: {} version {}", key_id, version))
    }

    /// Start a new key version. The previous versions are kept so existing ciphertexts still decrypt.
    pub fn rotate_key(&self, key_id: &str) -> Result<(), String> {
        let new_key = Secret(generate_random_bytes(32)?);

        {
            let mut keys = self.keys.lock().map_err(|_| "Failed to acquire lock".to_string())?;
            keys.get_mut(key_id).ok_or_else(|| format!("Key not found: {}", key_id))?.push(new_key);
        }

        if let Ok(mut metadata) = self.key_metadata.lock() {
            if let Some(meta) = metadata.get_mut(key_id) {
                meta.rotated_at = Some(SystemTime::now());
                if meta.status != KeyStatus::Compromised {
                    meta.status = KeyStatus::Active;
                }
            }
        }

        Ok(())
    }

    /// Encrypt under the current version of a managed key; the version is recorded (and authenticated) in the result.
    /// A key marked compromised is not used for new data.
    pub fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<EncryptedData, String> {
        let status = self
            .key_metadata
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get(key_id)
            .map(|m| (m.status, m.algorithm));
        let (status, algorithm) = status.ok_or_else(|| format!("Key not found: {}", key_id))?;
        if status == KeyStatus::Compromised {
            return Err(format!("Key {} is marked compromised and is not used for new data", key_id));
        }
        let version = self.current_key_version(key_id)?;
        let mut key = self.get_key_version(key_id, version)?;
        let out = EncryptionEngine::new(algorithm).encrypt_with(plaintext, &key, version);
        wipe(&mut key);
        out
    }

    /// Decrypt with the key version recorded in the data (so data from before a rotation still opens). Data with
    /// key version 0 (raw-key data) is not accepted here.
    pub fn decrypt(&self, key_id: &str, data: &EncryptedData) -> Result<Vec<u8>, String> {
        if data.key_version == 0 {
            return Err("Data was not sealed with a managed key".to_string());
        }
        let mut key = self.get_key_version(key_id, data.key_version)?;
        let out = EncryptionEngine::new(data.algorithm).decrypt(data, &key);
        wipe(&mut key);
        out
    }

    pub fn mark_compromised(&self, key_id: &str) -> Result<(), String> {
        self.key_metadata
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .get_mut(key_id)
            .map(|meta| meta.status = KeyStatus::Compromised)
            .ok_or_else(|| format!("Key not found: {}", key_id))
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

// Utility functions

fn generate_random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len];
    crate::os_random::fill(&mut buf)?;
    Ok(buf)
}

fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// SHA-256 (FIPS 180-4), via the RustCrypto `sha2` crate.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(data).into()
}

/// SHA-512 (FIPS 180-4), via the RustCrypto `sha2` crate.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    use sha2::Digest;
    sha2::Sha512::digest(data).into()
}

/// HMAC-SHA256 (RFC 2104 / RFC 4231), via the RustCrypto `hmac` crate.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// PBKDF2-HMAC-SHA256 (RFC 8018) for any output length, via the RustCrypto `pbkdf2` crate.
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, dk_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; dk_len];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, iterations.max(1), &mut out);
    out
}

/// Compare two byte strings without stopping at the first difference (length is not secret).
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_engine_creation() {
        let engine = EncryptionEngine::new(EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(engine.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn engine_round_trips_and_rejects_tampering() {
        let engine = EncryptionEngine::default();
        let key = [7u8; 32];
        let enc = engine.encrypt(b"attack at dawn", &key).unwrap();
        assert_ne!(enc.ciphertext, b"attack at dawn".to_vec(), "not stored in the clear");
        assert_eq!(engine.decrypt(&enc, &key).unwrap(), b"attack at dawn".to_vec());
        let enc2 = engine.encrypt(b"attack at dawn", &key).unwrap();
        assert_ne!(enc.nonce, enc2.nonce, "fresh nonce every message");
        assert_ne!(enc.ciphertext, enc2.ciphertext);
        let mut bad = enc.clone();
        bad.ciphertext[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        let mut bad = enc.clone();
        bad.tag[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        let mut bad = enc.clone();
        bad.nonce[0] ^= 1;
        assert!(engine.decrypt(&bad, &key).is_err());
        assert!(engine.decrypt(&enc, &[8u8; 32]).is_err());
        assert!(engine.encrypt(b"x", &[0u8; 31]).is_err(), "key length is checked");
        assert!(engine.encrypt(b"x", &[0u8; 33]).is_err());
    }

    #[test]
    fn unimplemented_algorithms_fail_loudly() {
        assert!(EncryptionEngine::new(EncryptionAlgorithm::AES256GCM).encrypt(b"x", &[0u8; 32]).is_err());
        assert!(EncryptionEngine::new(EncryptionAlgorithm::AES256CBC).encrypt(b"x", &[0u8; 32]).is_err());
        assert!(HashingEngine::new(HashingAlgorithm::SHA3_256).hash(b"x").is_err());
        assert!(PasswordHasher::new(KeyDerivationFunction::Argon2id).hash_password("x").is_err());
    }

    #[test]
    fn test_generate_key_manager() {
        let manager = KeyManager::new();
        let result = manager.generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305);

        assert!(result.is_ok());

        let key = manager.get_key("key1");
        assert!(key.is_ok());
        assert_eq!(key.unwrap().len(), 32);
    }

    #[test]
    fn test_password_verification() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let hash = hasher.hash_password("password123").unwrap();
        let other = hasher.hash_password("password123").unwrap();
        assert_ne!(hash.salt, other.salt, "random salt");
        assert_ne!(hash.hash, other.hash, "same password, different salt, different hash");

        let verified = hasher.verify_password("password123", &hash).unwrap();
        assert!(verified);

        let wrong = hasher.verify_password("wrongpassword", &hash).unwrap();
        assert!(!wrong);
    }

    #[test]
    fn test_constant_time_compare() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 2, 3, 4];
        let c = vec![1, 2, 3, 5];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
        assert!(!constant_time_compare(&a, &a[..3]));
    }

    #[test]
    fn test_key_rotation() {
        let manager = KeyManager::new();
        manager
            .generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let key_before = manager.get_key("key1").unwrap();

        manager.rotate_key("key1").unwrap();

        let key_after = manager.get_key("key1").unwrap();

        // After rotation, key should be different
        assert_ne!(key_before, key_after);
    }

    #[test]
    fn test_mark_compromised() {
        let manager = KeyManager::new();
        manager
            .generate_key("key1".to_string(), EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let result = manager.mark_compromised("key1");
        assert!(result.is_ok());
    }

    // ---- versioned formats, header authentication, bounds, rotation ----

    /// Salt 0..15, password "hunter2", 100,000 rounds: computed with Python's hashlib.pbkdf2_hmac, i.e. what a
    /// version 1 hash written by an earlier release looks like.
    const LEGACY_HASH: &str = "$pbkdf2$100000$000102030405060708090a0b0c0d0e0f$844d6e092def956a3d2773e41d721bf2d95a1d8f524d970ca76cd20528a03679";

    #[test]
    fn legacy_password_hash_still_verifies_and_asks_for_a_rehash() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let old = PasswordHash::parse(LEGACY_HASH).unwrap();
        assert_eq!((old.version, old.iterations), (1, 100_000));
        assert_eq!(old.to_string(), LEGACY_HASH, "legacy text form round-trips");
        assert!(old.needs_rehash());
        assert!(hasher.verify_password("hunter2", &old).unwrap());
        assert!(!hasher.verify_password("hunter3", &old).unwrap());
        // wrong password: no replacement; right password: a current-version replacement that verifies
        let (ok, up) = hasher.verify_and_upgrade("nope", &old).unwrap();
        assert!(!ok && up.is_none());
        let (ok, up) = hasher.verify_and_upgrade("hunter2", &old).unwrap();
        let up = up.expect("legacy hash must be upgraded");
        assert!(ok);
        assert_eq!((up.version, up.iterations), (2, PBKDF2_ITERATIONS));
        assert!(!up.needs_rehash());
        assert!(hasher.verify_password("hunter2", &up).unwrap());
        // the current text form round-trips and a current hash is not rewritten again
        let again = PasswordHash::parse(&up.to_string()).unwrap();
        assert_eq!((again.version, again.iterations, &again.salt, &again.hash), (2, PBKDF2_ITERATIONS, &up.salt, &up.hash));
        let (ok, none) = hasher.verify_and_upgrade("hunter2", &again).unwrap();
        assert!(ok && none.is_none());
    }

    #[test]
    fn new_hashes_use_at_least_600k_rounds_and_report_the_time() {
        assert!(PBKDF2_ITERATIONS >= 600_000);
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let t = std::time::Instant::now();
        let h = hasher.hash_password("timing").unwrap();
        let took = t.elapsed();
        // run with --nocapture to see it; the profile (debug or release) changes this a lot
        println!("one PBKDF2-HMAC-SHA256 hash at {} rounds took {:?} ({} build)", h.iterations, took, if cfg!(debug_assertions) { "debug" } else { "release" });
        assert!(h.iterations >= 600_000 && h.version == 2);
        assert!(h.to_string().starts_with("$pbkdf2-sha256$v=2$i=600000$"));
    }

    #[test]
    fn crafted_iteration_counts_are_rejected_without_doing_the_work() {
        let hasher = PasswordHasher::new(KeyDerivationFunction::PBKDF2);
        let t = std::time::Instant::now();
        for bad in [u32::MAX, PBKDF2_MAX_ITERATIONS + 1, 0] {
            let mut h = PasswordHash::parse(LEGACY_HASH).unwrap();
            h.iterations = bad;
            assert!(hasher.verify_password("hunter2", &h).is_err(), "{} rounds must be refused", bad);
            let text = format!("$pbkdf2${}$000102030405060708090a0b0c0d0e0f$844d6e092def956a3d2773e41d721bf2d95a1d8f524d970ca76cd20528a03679", bad);
            assert!(hasher.verify_password("hunter2", &PasswordHash::parse(&text).unwrap()).is_err());
        }
        // a version 2 hash may not claim fewer rounds than the current count (downgrade), a legacy one not below the floor
        let mut h = PasswordHash::parse(LEGACY_HASH).unwrap();
        h.version = 2;
        assert!(hasher.verify_password("hunter2", &h).is_err());
        h.version = 1;
        h.iterations = PBKDF2_MIN_LEGACY_ITERATIONS - 1;
        assert!(hasher.verify_password("hunter2", &h).is_err());
        // absurd sizes and unknown versions
        let mut h = PasswordHash::parse(LEGACY_HASH).unwrap();
        h.hash = vec![0; 1 << 20];
        assert!(hasher.verify_password("hunter2", &h).is_err());
        let mut h = PasswordHash::parse(LEGACY_HASH).unwrap();
        h.version = 9;
        assert!(hasher.verify_password("hunter2", &h).is_err());
        assert!(PasswordHash::parse("$pbkdf2-sha256$v=3$i=600000$00$00").is_err());
        assert!(PasswordHash::parse("garbage").is_err());
        assert!(PasswordHash::parse("$pbkdf2$12$zz$00").is_err());
        assert!(t.elapsed().as_secs() < 5, "rejection must not run the key derivation (took {:?})", t.elapsed());
    }

    #[test]
    fn header_is_authenticated_as_associated_data() {
        let engine = EncryptionEngine::default();
        let key = [5u8; 32];
        let enc = engine.encrypt(b"payload", &key).unwrap();
        assert_eq!(enc.version, ENCRYPTED_DATA_VERSION);
        assert_eq!(engine.decrypt(&enc, &key).unwrap(), b"payload".to_vec());
        let mut b = enc.clone();
        b.salt = vec![1, 2, 3];
        assert!(engine.decrypt(&b, &key).is_err(), "salt change");
        let mut b = enc.clone();
        b.iteration_count = 2;
        assert!(engine.decrypt(&b, &key).is_err(), "iteration count change");
        let mut b = enc.clone();
        b.key_version = 7;
        assert!(engine.decrypt(&b, &key).is_err(), "key version change");
        let mut b = enc.clone();
        b.version = 1; // a downgrade to the unauthenticated container must not be accepted either
        assert!(engine.decrypt(&b, &key).is_err(), "version downgrade");
        let mut b = enc.clone();
        b.version = 3;
        assert!(engine.decrypt(&b, &key).is_err(), "unknown version");
    }

    #[test]
    fn version_1_ciphertexts_without_associated_data_still_decrypt() {
        // what the previous release wrote: no associated data, iteration_count 1, no salt
        let key = [9u8; 32];
        let nonce = [3u8; 12];
        let mut sealed = crate::aead::seal(&key, &nonce, &[], b"old data");
        let tag = sealed.split_off(sealed.len() - 16);
        let old = EncryptedData {
            ciphertext: sealed,
            nonce: nonce.to_vec(),
            tag,
            salt: vec![],
            algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
            iteration_count: 1,
            version: 1,
            key_version: 0,
        };
        assert_eq!(EncryptionEngine::default().decrypt(&old, &key).unwrap(), b"old data".to_vec());
    }

    #[test]
    fn rotate_key_keeps_old_versions_so_old_data_still_decrypts() {
        let m = KeyManager::new();
        m.generate_key("k".to_string(), EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let old = m.encrypt("k", b"before rotation").unwrap();
        assert_eq!(old.key_version, 1);
        let k1 = m.get_key("k").unwrap();
        m.rotate_key("k").unwrap();
        assert_eq!(m.current_key_version("k").unwrap(), 2);
        assert_ne!(m.get_key("k").unwrap(), k1);
        assert_eq!(m.get_key_version("k", 1).unwrap(), k1, "the old key is kept");
        let new = m.encrypt("k", b"after rotation").unwrap();
        assert_eq!(new.key_version, 2);
        assert_eq!(m.decrypt("k", &old).unwrap(), b"before rotation".to_vec());
        assert_eq!(m.decrypt("k", &new).unwrap(), b"after rotation".to_vec());
        // a swapped key version is caught by the authenticated header and by the key itself
        let mut lie = old.clone();
        lie.key_version = 2;
        assert!(m.decrypt("k", &lie).is_err());
        assert!(m.decrypt("k", &EncryptionEngine::default().encrypt(b"x", &k1).unwrap()).is_err(), "raw-key data is not managed data");
        assert!(m.get_key_version("k", 0).is_err() && m.get_key_version("k", 3).is_err());
        assert!(m.rotate_key("missing").is_err());
        // a compromised key still opens old data (so it can be migrated) but is never used for new data
        m.mark_compromised("k").unwrap();
        assert_eq!(m.decrypt("k", &old).unwrap(), b"before rotation".to_vec());
        assert!(m.encrypt("k", b"x").is_err());
    }

    #[test]
    fn wipe_zeroes_the_buffer() {
        let mut k = vec![0xAAu8; 64];
        wipe(&mut k);
        assert!(k.iter().all(|b| *b == 0));
        let mut s = Secret(vec![1, 2, 3]);
        wipe(&mut s.0);
        assert_eq!(s.0, vec![0, 0, 0]);
    }
}
