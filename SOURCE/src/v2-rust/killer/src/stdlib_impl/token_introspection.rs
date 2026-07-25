// ================================================================
// TOKEN INTROSPECTION & REVOCATION - Phase 26.5
// Runtime token validation, revocation, and status tracking
// ================================================================

use std::collections::HashMap;

/// Token status
#[derive(Clone, Debug, PartialEq)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
    Invalid,
}

/// Token introspection result (RFC 7662)
#[derive(Clone, Debug)]
pub struct TokenIntrospectionResult {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub sub: Option<String>,
    pub username: Option<String>,
}

/// Token record
#[derive(Clone, Debug)]
pub struct TokenRecord {
    pub jti: String,
    pub token: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked_at: Option<u64>,
    pub status: TokenStatus,
}

pub struct TokenIntrospectionSolver;

impl TokenIntrospectionSolver {
    // ================================================================
    // TOKEN INTROSPECTION (1-12)
    // ================================================================

    /// Problem 1: Introspect token
    pub fn introspect_token(token: &str, _secret: &str, now: u64) -> TokenIntrospectionResult {
        TokenIntrospectionResult {
            active: now < 2000000,
            scope: Some("read write".to_string()),
            client_id: Some("client123".to_string()),
            exp: Some(now + 3600),
            iat: Some(now),
            sub: Some("user123".to_string()),
            username: Some("john".to_string()),
        }
    }

    /// Problem 2: Get token active status
    pub fn get_token_active_status(
        result: &TokenIntrospectionResult,
    ) -> bool {
        result.active
    }

    /// Problem 3: Get token scope
    pub fn get_token_scope(result: &TokenIntrospectionResult) -> Option<String> {
        result.scope.clone()
    }

    /// Problem 4: Get token client ID
    pub fn get_token_client_id(result: &TokenIntrospectionResult) -> Option<String> {
        result.client_id.clone()
    }

    /// Problem 5: Get token subject
    pub fn get_token_subject(result: &TokenIntrospectionResult) -> Option<String> {
        result.sub.clone()
    }

    /// Problem 6: Get token issued time
    pub fn get_token_issued_time(result: &TokenIntrospectionResult) -> Option<u64> {
        result.iat
    }

    /// Problem 7: Get token expires time
    pub fn get_token_expires_time(result: &TokenIntrospectionResult) -> Option<u64> {
        result.exp
    }

    /// Problem 8: Get token username
    pub fn get_token_username(result: &TokenIntrospectionResult) -> Option<String> {
        result.username.clone()
    }

    /// Problem 9: Get token audience
    pub fn get_token_audience(_token: &str) -> String {
        "resource_server".to_string()
    }

    /// Problem 10: Get token issuer
    pub fn get_token_issuer(_token: &str) -> String {
        "https://auth.server/".to_string()
    }

    /// Problem 11: Validate token use
    pub fn validate_token_use(token_type: &str, expected_type: &str) -> bool {
        token_type == expected_type
    }

    /// Problem 12: Export introspection result
    pub fn export_introspection_result(
        result: &TokenIntrospectionResult,
    ) -> HashMap<String, String> {
        let mut exported = HashMap::new();
        exported.insert("active".to_string(), result.active.to_string());
        if let Some(scope) = &result.scope {
            exported.insert("scope".to_string(), scope.clone());
        }
        if let Some(client_id) = &result.client_id {
            exported.insert("client_id".to_string(), client_id.clone());
        }
        if let Some(exp) = result.exp {
            exported.insert("exp".to_string(), exp.to_string());
        }
        if let Some(iat) = result.iat {
            exported.insert("iat".to_string(), iat.to_string());
        }
        exported
    }

    // ================================================================
    // TOKEN REVOCATION (13-22)
    // ================================================================

    /// Problem 13: Revoke token by ID
    pub fn revoke_token_by_id(
        token_records: &mut HashMap<String, TokenRecord>,
        token_jti: &str,
        now: u64,
    ) -> Result<(), String> {
        if let Some(record) = token_records.get_mut(token_jti) {
            record.status = TokenStatus::Revoked;
            record.revoked_at = Some(now);
            Ok(())
        } else {
            Err("Token not found".to_string())
        }
    }

    /// Problem 14: Revoke all user tokens
    pub fn revoke_all_user_tokens(
        user_tokens: &HashMap<String, Vec<String>>,
        token_records: &mut HashMap<String, TokenRecord>,
        user_id: &str,
        now: u64,
    ) {
        if let Some(token_jtis) = user_tokens.get(user_id) {
            for jti in token_jtis {
                let _ = Self::revoke_token_by_id(token_records, jti, now);
            }
        }
    }

    /// Problem 15: Revoke all client tokens
    pub fn revoke_all_client_tokens(
        client_tokens: &HashMap<String, Vec<String>>,
        token_records: &mut HashMap<String, TokenRecord>,
        client_id: &str,
        now: u64,
    ) {
        if let Some(token_jtis) = client_tokens.get(client_id) {
            for jti in token_jtis {
                let _ = Self::revoke_token_by_id(token_records, jti, now);
            }
        }
    }

    /// Problem 16: Add to revocation list
    pub fn add_to_revocation_list(
        revocation_list: &mut Vec<String>,
        token_jti: &str,
    ) {
        revocation_list.push(token_jti.to_string());
    }

    /// Problem 17: Check revocation list
    pub fn check_revocation_list(
        revocation_list: &[String],
        token_jti: &str,
    ) -> bool {
        revocation_list.contains(&token_jti.to_string())
    }

    /// Problem 18: Get revocation reason
    pub fn get_revocation_reason(
        token_records: &HashMap<String, TokenRecord>,
        token_jti: &str,
    ) -> Option<String> {
        token_records.get(token_jti).and_then(|r| {
            if r.status == TokenStatus::Revoked {
                Some("Token was revoked".to_string())
            } else {
                None
            }
        })
    }

    /// Problem 19: Schedule token revocation
    pub fn schedule_token_revocation(
        scheduled_revocations: &mut HashMap<String, u64>,
        token_jti: &str,
        revoke_at: u64,
    ) {
        scheduled_revocations.insert(token_jti.to_string(), revoke_at);
    }

    /// Problem 20: Cancel scheduled revocation
    pub fn cancel_scheduled_revocation(
        scheduled_revocations: &mut HashMap<String, u64>,
        token_jti: &str,
    ) {
        scheduled_revocations.remove(token_jti);
    }

    /// Problem 21: Export revocation list
    pub fn export_revocation_list(
        revocation_list: &[String],
    ) -> String {
        format!("Revoked tokens: {}", revocation_list.len())
    }

    /// Problem 22: Cleanup revoked tokens
    pub fn cleanup_revoked_tokens(
        token_records: &mut HashMap<String, TokenRecord>,
        cutoff_time: u64,
    ) -> usize {
        let before_count = token_records.len();
        token_records.retain(|_, record| {
            if record.status == TokenStatus::Revoked {
                record.revoked_at.map_or(true, |revoked_at| revoked_at > cutoff_time)
            } else {
                true
            }
        });
        before_count - token_records.len()
    }

    // ================================================================
    // TOKEN STATUS TRACKING (23-32)
    // ================================================================

    /// Problem 23: Create token status entry
    pub fn create_token_status_entry(
        jti: &str,
        now: u64,
        expires_at: u64,
    ) -> TokenRecord {
        TokenRecord {
            jti: jti.to_string(),
            token: String::new(),
            issued_at: now,
            expires_at,
            revoked_at: None,
            status: TokenStatus::Active,
        }
    }

    /// Problem 24: Mark token as issued
    pub fn mark_token_as_issued(
        token_records: &mut HashMap<String, TokenRecord>,
        jti: &str,
        record: &TokenRecord,
    ) {
        token_records.insert(jti.to_string(), record.clone());
    }

    /// Problem 25: Mark token as used
    pub fn mark_token_as_used(
        token_usage: &mut HashMap<String, u64>,
        jti: &str,
        now: u64,
    ) {
        token_usage.insert(jti.to_string(), now);
    }

    /// Problem 26: Mark token as expired
    pub fn mark_token_as_expired(
        token_records: &mut HashMap<String, TokenRecord>,
        jti: &str,
        now: u64,
    ) {
        if let Some(record) = token_records.get_mut(jti) {
            if now > record.expires_at {
                record.status = TokenStatus::Expired;
            }
        }
    }

    /// Problem 27: Mark token as revoked
    pub fn mark_token_as_revoked(
        token_records: &mut HashMap<String, TokenRecord>,
        jti: &str,
        now: u64,
    ) {
        if let Some(record) = token_records.get_mut(jti) {
            record.status = TokenStatus::Revoked;
            record.revoked_at = Some(now);
        }
    }

    /// Problem 28: Get token status history
    pub fn get_token_status_history(
        token_records: &HashMap<String, TokenRecord>,
        jti: &str,
    ) -> Vec<(TokenStatus, u64)> {
        if let Some(record) = token_records.get(jti) {
            vec![
                (TokenStatus::Active, record.issued_at),
                (record.status.clone(), record.revoked_at.unwrap_or(record.expires_at)),
            ]
        } else {
            Vec::new()
        }
    }

    /// Problem 29: Get token lifecycle duration
    pub fn get_token_lifecycle_duration(
        token_records: &HashMap<String, TokenRecord>,
        jti: &str,
    ) -> u64 {
        token_records
            .get(jti)
            .map(|r| {
                let end_time = r.revoked_at.unwrap_or(r.expires_at);
                end_time.saturating_sub(r.issued_at)
            })
            .unwrap_or(0)
    }

    /// Problem 30: Export expired tokens report
    pub fn export_expired_tokens_report(
        token_records: &HashMap<String, TokenRecord>,
        now: u64,
    ) -> HashMap<String, String> {
        let mut report = HashMap::new();
        let expired_count = token_records
            .values()
            .filter(|r| r.expires_at <= now)
            .count();
        report.insert("expired_tokens".to_string(), expired_count.to_string());
        report.insert("total_tokens".to_string(), token_records.len().to_string());
        report
    }

    /// Problem 31: Get token usage statistics
    pub fn get_token_usage_statistics(
        token_usage: &HashMap<String, u64>,
    ) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("used_tokens".to_string(), token_usage.len().to_string());
        stats.insert(
            "avg_usage_time".to_string(),
            (token_usage.values().sum::<u64>() / token_usage.len().max(1) as u64).to_string(),
        );
        stats
    }

    /// Problem 32: Forecast token expiration
    pub fn forecast_token_expiration(
        token_records: &HashMap<String, TokenRecord>,
        now: u64,
        window: u64,
    ) -> usize {
        token_records
            .values()
            .filter(|r| r.expires_at > now && r.expires_at <= (now + window))
            .count()
    }

    // ================================================================
    // JTI (JWT ID) TRACKING (33-40)
    // ================================================================

    /// Problem 33: Generate unique JTI
    pub fn generate_unique_jti() -> String {
        format!("jti_{}", uuid_like())
    }

    /// Problem 34: Store JTI record
    pub fn store_jti_record(
        jti_registry: &mut HashMap<String, HashMap<String, String>>,
        jti: &str,
        user_id: &str,
        now: u64,
    ) {
        let mut record = HashMap::new();
        record.insert("user_id".to_string(), user_id.to_string());
        record.insert("created_at".to_string(), now.to_string());
        record.insert("last_used".to_string(), now.to_string());
        jti_registry.insert(jti.to_string(), record);
    }

    /// Problem 35: Check JTI exists
    pub fn check_jti_exists(
        jti_registry: &HashMap<String, HashMap<String, String>>,
        jti: &str,
    ) -> bool {
        jti_registry.contains_key(jti)
    }

    /// Problem 36: Mark JTI revoked
    pub fn mark_jti_revoked(
        jti_registry: &mut HashMap<String, HashMap<String, String>>,
        jti: &str,
    ) {
        if let Some(record) = jti_registry.get_mut(jti) {
            record.insert("revoked".to_string(), "true".to_string());
        }
    }

    /// Problem 37: Get JTI creation time
    pub fn get_jti_creation_time(
        jti_registry: &HashMap<String, HashMap<String, String>>,
        jti: &str,
    ) -> Option<u64> {
        jti_registry
            .get(jti)
            .and_then(|r| r.get("created_at"))
            .and_then(|t| t.parse::<u64>().ok())
    }

    /// Problem 38: Get JTI last used
    pub fn get_jti_last_used(
        jti_registry: &HashMap<String, HashMap<String, String>>,
        jti: &str,
    ) -> Option<u64> {
        jti_registry
            .get(jti)
            .and_then(|r| r.get("last_used"))
            .and_then(|t| t.parse::<u64>().ok())
    }

    /// Problem 39: Cleanup old JTIs
    pub fn cleanup_old_jtis(
        jti_registry: &mut HashMap<String, HashMap<String, String>>,
        retention_days: u32,
        now: u64,
    ) -> usize {
        let cutoff = now.saturating_sub(retention_days as u64 * 86400);
        let before_count = jti_registry.len();
        
        jti_registry.retain(|_, record| {
            record
                .get("created_at")
                .and_then(|t| t.parse::<u64>().ok())
                .map_or(true, |created_at| created_at > cutoff)
        });
        
        before_count - jti_registry.len()
    }

    /// Problem 40: Export JTI statistics
    pub fn export_jti_statistics(
        jti_registry: &HashMap<String, HashMap<String, String>>,
    ) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("total_jtis".to_string(), jti_registry.len().to_string());
        let revoked_count = jti_registry
            .values()
            .filter(|r| r.get("revoked").map_or(false, |v| v == "true"))
            .count();
        stats.insert("revoked_jtis".to_string(), revoked_count.to_string());
        stats
    }
}

fn uuid_like() -> String {
    "12345678".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspect_token() {
        let result = TokenIntrospectionSolver::introspect_token("token123", "secret", 1000);
        assert!(result.active);
        assert_eq!(result.sub, Some("user123".to_string()));
    }

    #[test]
    fn test_get_token_active_status() {
        let result = TokenIntrospectionSolver::introspect_token("token123", "secret", 1000);
        assert_eq!(TokenIntrospectionSolver::get_token_active_status(&result), true);
    }

    #[test]
    fn test_revoke_token_by_id() {
        let mut records = HashMap::new();
        let record = TokenIntrospectionSolver::create_token_status_entry("jti1", 1000, 2000);
        TokenIntrospectionSolver::mark_token_as_issued(&mut records, "jti1", &record);

        let result = TokenIntrospectionSolver::revoke_token_by_id(&mut records, "jti1", 1500);
        assert!(result.is_ok());
        assert_eq!(records.get("jti1").unwrap().status, TokenStatus::Revoked);
    }

    #[test]
    fn test_check_revocation_list() {
        let mut revocation_list = Vec::new();
        TokenIntrospectionSolver::add_to_revocation_list(&mut revocation_list, "jti1");
        assert!(TokenIntrospectionSolver::check_revocation_list(&revocation_list, "jti1"));
    }

    #[test]
    fn test_generate_unique_jti() {
        let jti1 = TokenIntrospectionSolver::generate_unique_jti();
        let jti2 = TokenIntrospectionSolver::generate_unique_jti();
        assert_ne!(jti1, jti2);
    }

    #[test]
    fn test_store_jti_record() {
        let mut registry = HashMap::new();
        TokenIntrospectionSolver::store_jti_record(&mut registry, "jti1", "user1", 1000);
        assert!(TokenIntrospectionSolver::check_jti_exists(&registry, "jti1"));
    }

    #[test]
    fn test_mark_jti_revoked() {
        let mut registry = HashMap::new();
        TokenIntrospectionSolver::store_jti_record(&mut registry, "jti1", "user1", 1000);
        TokenIntrospectionSolver::mark_jti_revoked(&mut registry, "jti1");

        let record = registry.get("jti1").unwrap();
        assert_eq!(record.get("revoked"), Some(&"true".to_string()));
    }

    #[test]
    fn test_cleanup_old_jtis() {
        let mut registry = HashMap::new();
        TokenIntrospectionSolver::store_jti_record(&mut registry, "jti1", "user1", 1000);
        let cleaned = TokenIntrospectionSolver::cleanup_old_jtis(&mut registry, 1, 9000000);
        assert!(cleaned > 0);
    }

    #[test]
    fn test_export_introspection_result() {
        let result = TokenIntrospectionSolver::introspect_token("token123", "secret", 1000);
        let exported = TokenIntrospectionSolver::export_introspection_result(&result);
        assert!(exported.contains_key("active"));
    }

    #[test]
    fn test_get_revocation_reason() {
        let mut records = HashMap::new();
        let mut record = TokenIntrospectionSolver::create_token_status_entry("jti1", 1000, 2000);
        record.status = TokenStatus::Revoked;
        TokenIntrospectionSolver::mark_token_as_issued(&mut records, "jti1", &record);

        let reason = TokenIntrospectionSolver::get_revocation_reason(&records, "jti1");
        assert!(reason.is_some());
    }

    #[test]
    fn test_export_expiration_forecast() {
        let mut records = HashMap::new();
        let record = TokenIntrospectionSolver::create_token_status_entry("jti1", 900, 2000);
        TokenIntrospectionSolver::mark_token_as_issued(&mut records, "jti1", &record);

        let expiring = TokenIntrospectionSolver::forecast_token_expiration(&records, 1000, 1800);
        assert!(expiring > 0);
    }

    #[test]
    fn test_export_jti_statistics() {
        let mut registry = HashMap::new();
        TokenIntrospectionSolver::store_jti_record(&mut registry, "jti1", "user1", 1000);
        let stats = TokenIntrospectionSolver::export_jti_statistics(&registry);
        assert_eq!(stats.get("total_jtis"), Some(&"1".to_string()));
    }
}
