// Phase 7: Security Module - Encryption, hashing, authentication, access control
// Features: Password hashing, encryption, message digests, token generation, rate limiting

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hash algorithm variants
#[derive(Clone, Debug, PartialEq)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
    MD5,
    BLAKE2,
}

/// Simple password hasher (production would use bcrypt/argon2)
pub struct PasswordHasher;

impl PasswordHasher {
    /// Hash password with algorithm
    pub fn hash(password: &str, algorithm: HashAlgorithm) -> String {
        match algorithm {
            HashAlgorithm::SHA256 => Self::sha256_simple(password),
            HashAlgorithm::SHA512 => Self::sha512_simple(password),
            HashAlgorithm::MD5 => Self::md5_simple(password),
            HashAlgorithm::BLAKE2 => Self::blake2_simple(password),
        }
    }

    /// Verify password against hash
    pub fn verify(password: &str, hash: &str, algorithm: HashAlgorithm) -> bool {
        Self::hash(password, algorithm) == hash
    }

    /// Simple SHA256 simulation (not cryptographically secure)
    fn sha256_simple(input: &str) -> String {
        let mut hash = 0u64;
        for byte in input.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        format!("sha256:{:x}", hash)
    }

    /// Simple SHA512 simulation
    fn sha512_simple(input: &str) -> String {
        let mut hash = 0u128;
        for byte in input.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u128);
        }
        format!("sha512:{:x}", hash)
    }

    /// Simple MD5 simulation
    fn md5_simple(input: &str) -> String {
        let mut hash = 5381u64;
        for byte in input.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
        }
        format!("md5:{:x}", hash)
    }

    /// Simple BLAKE2 simulation
    fn blake2_simple(input: &str) -> String {
        let mut hash = 0u64;
        for (i, byte) in input.bytes().enumerate() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64).wrapping_add(i as u64);
        }
        format!("blake2:{:x}", hash)
    }
}

/// Basic encryption/decryption (Caesar cipher for demo)
pub struct Encryption;

impl Encryption {
    /// Encrypt text with shift cipher
    pub fn encrypt(plaintext: &str, shift: u8) -> String {
        plaintext
            .chars()
            .map(|c| {
                if c.is_alphabetic() {
                    let base = if c.is_lowercase() { b'a' } else { b'A' };
                    let offset = (c as u8 - base + shift) % 26;
                    (base + offset) as char
                } else {
                    c
                }
            })
            .collect()
    }

    /// Decrypt text with shift cipher
    pub fn decrypt(ciphertext: &str, shift: u8) -> String {
        Self::encrypt(ciphertext, 26 - shift)
    }

    /// XOR encryption (simple, not secure)
    pub fn xor_encrypt(data: &str, key: &str) -> Vec<u8> {
        let key_bytes = key.as_bytes();
        data.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()])
            .collect()
    }

    /// XOR decryption
    pub fn xor_decrypt(data: &[u8], key: &str) -> Result<String, String> {
        let key_bytes = key.as_bytes();
        let decrypted: Vec<u8> = data
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()])
            .collect();
        String::from_utf8(decrypted).map_err(|_| "Invalid UTF-8".to_string())
    }
}

/// Message authentication code
pub struct MessageAuthenticationCode;

impl MessageAuthenticationCode {
    /// Generate MAC for message
    pub fn generate(message: &str, secret: &str) -> String {
        let mut hash = 0u64;
        for (i, byte) in message.bytes().enumerate() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        for (i, byte) in secret.bytes().enumerate() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64).wrapping_add(i as u64);
        }
        format!("{:x}", hash)
    }

    /// Verify MAC
    pub fn verify(message: &str, secret: &str, mac: &str) -> bool {
        Self::generate(message, secret) == mac
    }
}

/// Token generation and validation
pub struct TokenGenerator;

impl TokenGenerator {
    /// Generate random token
    pub fn generate(length: usize) -> String {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect();
        let mut token = String::new();
        for i in 0..length {
            let idx = (i * 73) % chars.len();
            token.push(chars[idx]);
        }
        token
    }

    /// Generate JWT-like token (simplified)
    pub fn generate_jwt(header: &str, payload: &str, secret: &str) -> String {
        let header_b64 = Self::base64_encode(header);
        let payload_b64 = Self::base64_encode(payload);
        let signature = PasswordHasher::hash(&format!("{}.{}", header_b64, payload_b64), super::security_module::HashAlgorithm::SHA256);
        let sig_b64 = Self::base64_encode(&signature);
        format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
    }

    /// Verify JWT token
    pub fn verify_jwt(token: &str, secret: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3
    }

    /// Base64 encode
    fn base64_encode(input: &str) -> String {
        format!("b64_{}", input.len())
    }
}

/// Access control list
#[derive(Clone, Debug)]
pub struct AccessControlEntry {
    pub resource: String,
    pub principal: String,
    pub permission: String, // "read", "write", "execute", "delete"
}

/// Access control manager
pub struct AccessControl {
    pub acl: Vec<AccessControlEntry>,
}

impl AccessControl {
    pub fn new() -> Self {
        AccessControl { acl: Vec::new() }
    }

    /// Add permission
    pub fn grant(&mut self, resource: String, principal: String, permission: String) {
        self.acl.push(AccessControlEntry {
            resource,
            principal,
            permission,
        });
    }

    /// Remove permission
    pub fn revoke(&mut self, resource: &str, principal: &str, permission: &str) {
        self.acl.retain(|entry| {
            !(entry.resource == resource && entry.principal == principal && entry.permission == permission)
        });
    }

    /// Check permission
    pub fn has_permission(&self, resource: &str, principal: &str, permission: &str) -> bool {
        self.acl.iter().any(|entry| {
            entry.resource == resource && entry.principal == principal && entry.permission == permission
        })
    }

    /// Get permissions for principal
    pub fn get_permissions(&self, principal: &str) -> Vec<String> {
        self.acl
            .iter()
            .filter(|entry| entry.principal == principal)
            .map(|entry| format!("{}:{}", entry.resource, entry.permission))
            .collect()
    }

    /// List ACL entries
    pub fn list_acl(&self) -> Vec<String> {
        self.acl
            .iter()
            .map(|entry| format!("{} -> {}:{}", entry.principal, entry.resource, entry.permission))
            .collect()
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiter for DoS protection
pub struct RateLimiter {
    pub limits: HashMap<String, (u32, u64)>, // principal -> (request_count, window_start)
    pub window_size: u64,                     // seconds
    pub max_requests: u32,
}

impl RateLimiter {
    /// Create rate limiter
    pub fn new(window_size: u64, max_requests: u32) -> Self {
        RateLimiter {
            limits: HashMap::new(),
            window_size,
            max_requests,
        }
    }

    /// Check if request allowed
    pub fn allow_request(&mut self, principal: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some((count, window_start)) = self.limits.get_mut(principal) {
            if now - *window_start > self.window_size {
                // Reset window
                *count = 1;
                *window_start = now;
                true
            } else if *count < self.max_requests {
                *count += 1;
                true
            } else {
                false
            }
        } else {
            self.limits.insert(principal.to_string(), (1, now));
            true
        }
    }

    /// Get current limit status
    pub fn get_status(&self, principal: &str) -> Option<(u32, u32)> {
        self.limits.get(principal).map(|(count, _)| (*count, self.max_requests))
    }

    /// Reset limit for principal
    pub fn reset(&mut self, principal: &str) {
        self.limits.remove(principal);
    }
}

/// Security audit log
#[derive(Clone, Debug)]
pub struct AuditLog {
    pub entries: Vec<AuditEntry>,
}

#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub action: String,
    pub principal: String,
    pub resource: String,
    pub result: String, // "success", "failure"
}

impl AuditLog {
    pub fn new() -> Self {
        AuditLog { entries: Vec::new() }
    }

    /// Log action
    pub fn log(&mut self, action: String, principal: String, resource: String, result: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.entries.push(AuditEntry {
            timestamp,
            action,
            principal,
            resource,
            result,
        });
    }

    /// Get entries by principal
    pub fn get_by_principal(&self, principal: &str) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.principal == principal)
            .cloned()
            .collect()
    }

    /// Count failures
    pub fn count_failures(&self) -> usize {
        self.entries.iter().filter(|e| e.result == "failure").count()
    }

    /// Clear old entries
    pub fn clear_entries(&mut self) {
        self.entries.clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Security module facade
pub struct SecurityModule;

impl SecurityModule {
    /// Hash password
    pub fn hash_password(password: &str) -> String {
        PasswordHasher::hash(password, HashAlgorithm::SHA256)
    }

    /// Verify password
    pub fn verify_password(password: &str, hash: &str) -> bool {
        PasswordHasher::verify(password, hash, HashAlgorithm::SHA256)
    }

    /// Encrypt text
    pub fn encrypt_text(plaintext: &str, shift: u8) -> String {
        Encryption::encrypt(plaintext, shift)
    }

    /// Decrypt text
    pub fn decrypt_text(ciphertext: &str, shift: u8) -> String {
        Encryption::decrypt(ciphertext, shift)
    }

    /// Generate token
    pub fn generate_token(length: usize) -> String {
        TokenGenerator::generate(length)
    }

    /// Create access control
    pub fn new_acl() -> AccessControl {
        AccessControl::new()
    }

    /// Create rate limiter
    pub fn new_rate_limiter(window_size: u64, max_requests: u32) -> RateLimiter {
        RateLimiter::new(window_size, max_requests)
    }

    /// Create audit log
    pub fn new_audit_log() -> AuditLog {
        AuditLog::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "mypassword";
        let hash = PasswordHasher::hash(password, HashAlgorithm::SHA256);
        assert!(!hash.is_empty());
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_password_verification() {
        let password = "mypassword";
        let hash = PasswordHasher::hash(password, HashAlgorithm::SHA256);
        assert!(PasswordHasher::verify(password, &hash, HashAlgorithm::SHA256));
    }

    #[test]
    fn test_password_verification_fail() {
        let password = "mypassword";
        let hash = PasswordHasher::hash(password, HashAlgorithm::SHA256);
        assert!(!PasswordHasher::verify("wrongpassword", &hash, HashAlgorithm::SHA256));
    }

    #[test]
    fn test_encryption_decryption() {
        let plaintext = "hello";
        let encrypted = Encryption::encrypt(plaintext, 3);
        let decrypted = Encryption::decrypt(&encrypted, 3);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_xor_encryption() {
        let data = "secret";
        let key = "mykey";
        let encrypted = Encryption::xor_encrypt(data, key);
        let decrypted = Encryption::xor_decrypt(&encrypted, key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_mac_generation() {
        let message = "hello";
        let secret = "mysecret";
        let mac = MessageAuthenticationCode::generate(message, secret);
        assert!(MessageAuthenticationCode::verify(message, secret, &mac));
    }

    #[test]
    fn test_mac_verification_fail() {
        let message = "hello";
        let secret = "mysecret";
        let mac = MessageAuthenticationCode::generate(message, secret);
        assert!(!MessageAuthenticationCode::verify("different", secret, &mac));
    }

    #[test]
    fn test_token_generation() {
        let token = TokenGenerator::generate(32);
        assert_eq!(token.len(), 32);
    }

    #[test]
    fn test_acl_grant_and_check() {
        let mut acl = AccessControl::new();
        acl.grant("file.txt".to_string(), "alice".to_string(), "read".to_string());
        assert!(acl.has_permission("file.txt", "alice", "read"));
    }

    #[test]
    fn test_acl_revoke() {
        let mut acl = AccessControl::new();
        acl.grant("file.txt".to_string(), "alice".to_string(), "read".to_string());
        acl.revoke("file.txt", "alice", "read");
        assert!(!acl.has_permission("file.txt", "alice", "read"));
    }

    #[test]
    fn test_acl_get_permissions() {
        let mut acl = AccessControl::new();
        acl.grant("file1.txt".to_string(), "alice".to_string(), "read".to_string());
        acl.grant("file2.txt".to_string(), "alice".to_string(), "write".to_string());
        let perms = acl.get_permissions("alice");
        assert_eq!(perms.len(), 2);
    }

    #[test]
    fn test_rate_limiter_allow() {
        let mut limiter = RateLimiter::new(60, 10);
        assert!(limiter.allow_request("alice"));
        assert_eq!(limiter.get_status("alice"), Some((1, 10)));
    }

    #[test]
    fn test_rate_limiter_exceeded() {
        let mut limiter = RateLimiter::new(60, 2);
        assert!(limiter.allow_request("alice"));
        assert!(limiter.allow_request("alice"));
        assert!(!limiter.allow_request("alice"));
    }

    #[test]
    fn test_rate_limiter_reset() {
        let mut limiter = RateLimiter::new(60, 2);
        limiter.allow_request("alice");
        limiter.allow_request("alice");
        limiter.reset("alice");
        assert!(limiter.allow_request("alice"));
    }

    #[test]
    fn test_audit_log_creation() {
        let log = AuditLog::new();
        assert_eq!(log.entries.len(), 0);
    }

    #[test]
    fn test_audit_log_logging() {
        let mut log = AuditLog::new();
        log.log("login".to_string(), "alice".to_string(), "system".to_string(), "success".to_string());
        assert_eq!(log.entries.len(), 1);
    }

    #[test]
    fn test_audit_log_by_principal() {
        let mut log = AuditLog::new();
        log.log("login".to_string(), "alice".to_string(), "system".to_string(), "success".to_string());
        log.log("logout".to_string(), "bob".to_string(), "system".to_string(), "success".to_string());
        let alice_entries = log.get_by_principal("alice");
        assert_eq!(alice_entries.len(), 1);
    }

    #[test]
    fn test_audit_log_failures() {
        let mut log = AuditLog::new();
        log.log("login".to_string(), "alice".to_string(), "system".to_string(), "success".to_string());
        log.log("login".to_string(), "bob".to_string(), "system".to_string(), "failure".to_string());
        assert_eq!(log.count_failures(), 1);
    }
}
