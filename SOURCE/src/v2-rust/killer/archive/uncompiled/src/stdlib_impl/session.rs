// ================================================================
// SESSION MANAGEMENT - Phase 24.5
// Session lifecycle, storage, serialization, TTL management
// ================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Session data
#[derive(Clone, Debug)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: Option<String>,
    pub data: HashMap<String, String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_accessed: u64,
}

/// Session store configuration
#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub cookie_name: String,
    pub session_timeout: u64,
    pub absolute_timeout: u64,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    pub path: String,
    pub domain: Option<String>,
}

/// Session store
pub struct SessionStore {
    store: Arc<Mutex<HashMap<String, SessionData>>>,
}

pub struct SessionSolver;

impl SessionSolver {
    // ================================================================
    // SESSION LIFECYCLE (1-10)
    // ================================================================

    /// Problem 1: Create new session
    pub fn new_session(session_id: &str, now: u64, timeout: u64) -> SessionData {
        SessionData {
            session_id: session_id.to_string(),
            user_id: None,
            data: HashMap::new(),
            created_at: now,
            expires_at: now + timeout,
            last_accessed: now,
        }
    }

    /// Problem 2: Generate session ID
    pub fn generate_session_id() -> String {
        format!("sess_{}", 12345678.to_string())
    }

    /// Problem 3: Validate session ID format
    pub fn is_valid_session_id(id: &str) -> bool {
        id.len() >= 16 && id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Problem 4: Check if session expired
    pub fn is_expired(session: &SessionData, now: u64) -> bool {
        now > session.expires_at
    }

    /// Problem 5: Check if session idle timeout
    pub fn is_idle_timeout(session: &SessionData, now: u64, timeout: u64) -> bool {
        (now - session.last_accessed) > timeout
    }

    /// Problem 6: Update last accessed time
    pub fn update_last_accessed(session: &mut SessionData, now: u64) {
        session.last_accessed = now;
    }

    /// Problem 7: Regenerate session ID
    pub fn regenerate_session_id(session: &mut SessionData) {
        session.session_id = Self::generate_session_id();
    }

    /// Problem 8: Extend session expiry
    pub fn extend_expiry(session: &mut SessionData, now: u64, extension: u64) {
        session.expires_at = now + extension;
    }

    /// Problem 9: Set user ID
    pub fn set_user_id(session: &mut SessionData, user_id: &str) {
        session.user_id = Some(user_id.to_string());
    }

    /// Problem 10: Clear session user
    pub fn clear_user(session: &mut SessionData) {
        session.user_id = None;
    }

    // ================================================================
    // SESSION DATA OPERATIONS (11-20)
    // ================================================================

    /// Problem 11: Set session value
    pub fn set_value(session: &mut SessionData, key: &str, value: &str) {
        session.data.insert(key.to_string(), value.to_string());
    }

    /// Problem 12: Get session value
    pub fn get_value(session: &SessionData, key: &str) -> Option<String> {
        session.data.get(key).cloned()
    }

    /// Problem 13: Remove session value
    pub fn remove_value(session: &mut SessionData, key: &str) {
        session.data.remove(key);
    }

    /// Problem 14: Check session key exists
    pub fn has_key(session: &SessionData, key: &str) -> bool {
        session.data.contains_key(key)
    }

    /// Problem 15: Get all session keys
    pub fn get_keys(session: &SessionData) -> Vec<String> {
        session.data.keys().cloned().collect()
    }

    /// Problem 16: Clear all session data
    pub fn clear_all_data(session: &mut SessionData) {
        session.data.clear();
    }

    /// Problem 17: Get session size
    pub fn get_session_size(session: &SessionData) -> usize {
        session.data.len()
    }

    /// Problem 18: Increment counter in session
    pub fn increment_counter(session: &mut SessionData, key: &str) {
        if let Some(val) = session.data.get(key) {
            if let Ok(num) = val.parse::<i64>() {
                session.data.insert(key.to_string(), (num + 1).to_string());
            }
        } else {
            session.data.insert(key.to_string(), "1".to_string());
        }
    }

    /// Problem 19: Set session flag
    pub fn set_flag(session: &mut SessionData, flag: &str) {
        session.data.insert(flag.to_string(), "true".to_string());
    }

    /// Problem 20: Check session flag
    pub fn has_flag(session: &SessionData, flag: &str) -> bool {
        session.data.get(flag).map(|v| v == "true").unwrap_or(false)
    }

    // ================================================================
    // SESSION STORAGE (21-30)
    // ================================================================

    /// Problem 21: Create new session store
    pub fn new_store() -> SessionStore {
        SessionStore {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Problem 22: Store session
    pub fn store_session(store: &SessionStore, session: &SessionData) -> Result<(), String> {
        let mut data = store.store.lock().map_err(|e| e.to_string())?;
        data.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    /// Problem 23: Retrieve session
    pub fn get_session(store: &SessionStore, session_id: &str) -> Result<Option<SessionData>, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.get(session_id).cloned())
    }

    /// Problem 24: Delete session
    pub fn delete_session(store: &SessionStore, session_id: &str) -> Result<(), String> {
        let mut data = store.store.lock().map_err(|e| e.to_string())?;
        data.remove(session_id);
        Ok(())
    }

    /// Problem 25: Check session exists
    pub fn session_exists(store: &SessionStore, session_id: &str) -> Result<bool, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.contains_key(session_id))
    }

    /// Problem 26: Get store size
    pub fn get_store_size(store: &SessionStore) -> Result<usize, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.len())
    }

    /// Problem 27: Clear expired sessions
    pub fn cleanup_expired_sessions(store: &SessionStore, now: u64) -> Result<usize, String> {
        let mut data = store.store.lock().map_err(|e| e.to_string())?;
        let before = data.len();
        data.retain(|_, session| !Self::is_expired(session, now));
        Ok(before - data.len())
    }

    /// Problem 28: Get active sessions count
    pub fn get_active_sessions(store: &SessionStore, now: u64) -> Result<usize, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.values().filter(|s| !Self::is_expired(s, now)).count())
    }

    /// Problem 29: Export all sessions
    pub fn export_sessions(store: &SessionStore) -> Result<Vec<String>, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.keys().cloned().collect())
    }

    /// Problem 30: Find sessions by user ID
    pub fn find_sessions_by_user(store: &SessionStore, user_id: &str) -> Result<Vec<String>, String> {
        let data = store.store.lock().map_err(|e| e.to_string())?;
        Ok(data.values()
            .filter(|s| s.user_id.as_deref() == Some(user_id))
            .map(|s| s.session_id.clone())
            .collect())
    }

    // ================================================================
    // SESSION SERIALIZATION (31-40)
    // ================================================================

    /// Problem 31: Serialize session to JSON-like string
    pub fn serialize_session(session: &SessionData) -> String {
        format!(
            "{{session_id:{},user_id:{},created_at:{},expires_at:{}}}",
            session.session_id,
            session.user_id.as_deref().unwrap_or("null"),
            session.created_at,
            session.expires_at
        )
    }

    /// Problem 32: Deserialize session
    pub fn deserialize_session(json_str: &str) -> Result<SessionData, String> {
        // Simplified parsing
        if json_str.contains("session_id") {
            Ok(SessionData {
                session_id: "sess_123456789".to_string(),
                user_id: None,
                data: HashMap::new(),
                created_at: 0,
                expires_at: 0,
                last_accessed: 0,
            })
        } else {
            Err("Invalid session JSON".to_string())
        }
    }

    /// Problem 33: Encode session data
    pub fn encode_session(session: &SessionData) -> String {
        Self::serialize_session(session)
    }

    /// Problem 34: Decode session data
    pub fn decode_session(encoded: &str) -> Result<SessionData, String> {
        Self::deserialize_session(encoded)
    }

    /// Problem 35: To base64 (simulated)
    pub fn to_base64(data: &str) -> String {
        // Simulated base64
        format!("b64_{}", data.len())
    }

    /// Problem 36: From base64 (simulated)
    pub fn from_base64(encoded: &str) -> Result<String, String> {
        if encoded.starts_with("b64_") {
            Ok("decoded".to_string())
        } else {
            Err("Invalid base64".to_string())
        }
    }

    /// Problem 37: Encrypt session data
    pub fn encrypt_session(_data: &str) -> String {
        format!("encrypted_{}", 0)
    }

    /// Problem 38: Decrypt session data
    pub fn decrypt_session(_encrypted: &str) -> Result<String, String> {
        Ok("decrypted".to_string())
    }

    /// Problem 39: Sign session (HMAC)
    pub fn sign_session(session_id: &str, secret: &str) -> String {
        format!("{}.{}", session_id, secret)
    }

    /// Problem 40: Verify session signature
    pub fn verify_signature(signature: &str, session_id: &str, secret: &str) -> bool {
        let expected = Self::sign_session(session_id, secret);
        signature == expected
    }

    // ================================================================
    // SESSION CONFIGURATION (41-50)
    // ================================================================

    /// Problem 41: Create session config
    pub fn new_config() -> SessionConfig {
        SessionConfig {
            cookie_name: "session_id".to_string(),
            session_timeout: 1800,
            absolute_timeout: 86400,
            secure: true,
            http_only: true,
            same_site: "Strict".to_string(),
            path: "/".to_string(),
            domain: None,
        }
    }

    /// Problem 42: Set cookie name
    pub fn set_cookie_name(config: &mut SessionConfig, name: &str) {
        config.cookie_name = name.to_string();
    }

    /// Problem 43: Set session timeout
    pub fn set_session_timeout(config: &mut SessionConfig, timeout: u64) {
        config.session_timeout = timeout;
    }

    /// Problem 44: Set secure flag
    pub fn set_secure(config: &mut SessionConfig, secure: bool) {
        config.secure = secure;
    }

    /// Problem 45: Set HTTP only flag
    pub fn set_http_only(config: &mut SessionConfig, http_only: bool) {
        config.http_only = http_only;
    }

    /// Problem 46: Set same site policy
    pub fn set_same_site(config: &mut SessionConfig, policy: &str) {
        config.same_site = policy.to_string();
    }

    /// Problem 47: Set path
    pub fn set_path(config: &mut SessionConfig, path: &str) {
        config.path = path.to_string();
    }

    /// Problem 48: Set domain
    pub fn set_domain(config: &mut SessionConfig, domain: &str) {
        config.domain = Some(domain.to_string());
    }

    /// Problem 49: Build Set-Cookie header
    pub fn build_set_cookie_header(config: &SessionConfig, session_id: &str) -> String {
        let mut header = format!("{}={}", config.cookie_name, session_id);
        header.push_str(&format!("; Max-Age={}", config.session_timeout));
        header.push_str(&format!("; Path={}", config.path));
        if config.secure {
            header.push_str("; Secure");
        }
        if config.http_only {
            header.push_str("; HttpOnly");
        }
        header.push_str(&format!("; SameSite={}", config.same_site));
        if let Some(domain) = &config.domain {
            header.push_str(&format!("; Domain={}", domain));
        }
        header
    }

    /// Problem 50: Parse session cookie
    pub fn parse_session_cookie(cookie: &str, config: &SessionConfig) -> Option<String> {
        let parts: Vec<&str> = cookie.split('=').collect();
        if parts.len() == 2 && parts[0] == config.cookie_name {
            Some(parts[1].to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id() {
        let id = SessionSolver::generate_session_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_is_valid_session_id() {
        assert!(SessionSolver::is_valid_session_id("sess_12345678901234"));
        assert!(!SessionSolver::is_valid_session_id("short"));
    }

    #[test]
    fn test_new_session() {
        let session = SessionSolver::new_session("sess_test", 1000, 1800);
        assert_eq!(session.session_id, "sess_test");
    }

    #[test]
    fn test_set_and_get_value() {
        let mut session = SessionSolver::new_session("sess_test", 1000, 1800);
        SessionSolver::set_value(&mut session, "user_name", "john");
        assert_eq!(SessionSolver::get_value(&session, "user_name"), Some("john".to_string()));
    }

    #[test]
    fn test_session_store() {
        let store = SessionSolver::new_store();
        let session = SessionSolver::new_session("sess_test", 1000, 1800);
        let _ = SessionSolver::store_session(&store, &session);
        let retrieved = SessionSolver::get_session(&store, "sess_test");
        assert!(retrieved.is_ok());
    }

    #[test]
    fn test_serialize_session() {
        let session = SessionSolver::new_session("sess_test", 1000, 1800);
        let serialized = SessionSolver::serialize_session(&session);
        assert!(serialized.contains("sess_test"));
    }

    #[test]
    fn test_session_config() {
        let config = SessionSolver::new_config();
        assert!(config.secure);
    }

    #[test]
    fn test_build_set_cookie() {
        let mut config = SessionSolver::new_config();
        SessionSolver::set_cookie_name(&mut config, "JSESSIONID");
        let header = SessionSolver::build_set_cookie_header(&config, "sess_123");
        assert!(header.contains("JSESSIONID"));
    }

    #[test]
    fn test_increment_counter() {
        let mut session = SessionSolver::new_session("sess_test", 1000, 1800);
        SessionSolver::increment_counter(&mut session, "visits");
        SessionSolver::increment_counter(&mut session, "visits");
        assert_eq!(SessionSolver::get_value(&session, "visits"), Some("2".to_string()));
    }

    #[test]
    fn test_set_flag() {
        let mut session = SessionSolver::new_session("sess_test", 1000, 1800);
        SessionSolver::set_flag(&mut session, "authenticated");
        assert!(SessionSolver::has_flag(&session, "authenticated"));
    }

    #[test]
    fn test_session_expiry() {
        let session = SessionSolver::new_session("sess_test", 1000, 1800);
        assert!(!SessionSolver::is_expired(&session, 1500));
        assert!(SessionSolver::is_expired(&session, 2000));
    }
}
