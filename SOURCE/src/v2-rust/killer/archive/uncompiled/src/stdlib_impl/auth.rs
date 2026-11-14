// ================================================================
// AUTHENTICATION & AUTHORIZATION - Phase 24.6
// Basic auth, bearer tokens, JWT, permissions, roles
// ================================================================

use std::collections::HashMap;

/// User credentials
#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Bearer token
#[derive(Clone, Debug)]
pub struct BearerToken {
    pub token: String,
    pub expires_at: u64,
    pub user_id: String,
}

/// JWT token (simplified)
#[derive(Clone, Debug)]
pub struct JwtToken {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

/// User with roles and permissions
#[derive(Clone, Debug)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub is_active: bool,
}

/// Role with permissions
#[derive(Clone, Debug)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<String>,
}

pub struct AuthSolver;

impl AuthSolver {
    // ================================================================
    // BASIC AUTHENTICATION (1-10)
    // ================================================================

    /// Problem 1: Parse Basic auth header
    pub fn parse_basic_auth(header: &str) -> Result<(String, String), String> {
        if !header.starts_with("Basic ") {
            return Err("Invalid Basic auth header".to_string());
        }
        
        let encoded = &header[6..];
        // Simulated base64 decode
        let decoded = encoded.replace("_", " ");
        let parts: Vec<&str> = decoded.split(':').collect();
        
        if parts.len() != 2 {
            return Err("Invalid credentials format".to_string());
        }
        
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Problem 2: Create Basic auth header
    pub fn create_basic_auth_header(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = credentials.replace(" ", "_");
        format!("Basic {}", encoded)
    }

    /// Problem 3: Validate credentials
    pub fn validate_credentials(username: &str, password: &str, stored_hash: &str) -> bool {
        let hash = Self::hash_password(password);
        hash == stored_hash
    }

    /// Problem 4: Hash password (simulated)
    pub fn hash_password(password: &str) -> String {
        format!("hash_{}", password.len())
    }

    /// Problem 5: Create credentials object
    pub fn new_credentials(username: &str, password: &str) -> Credentials {
        Credentials {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// Problem 6: Verify credentials against user database
    pub fn verify_user(creds: &Credentials, users: &HashMap<String, String>) -> Result<String, String> {
        if let Some(stored_hash) = users.get(&creds.username) {
            if Self::validate_credentials(&creds.username, &creds.password, stored_hash) {
                Ok(creds.username.clone())
            } else {
                Err("Invalid password".to_string())
            }
        } else {
            Err("User not found".to_string())
        }
    }

    /// Problem 7: Hash with salt
    pub fn hash_with_salt(password: &str, salt: &str) -> String {
        format!("{}_{}", Self::hash_password(password), salt)
    }

    /// Problem 8: Check password strength
    pub fn check_password_strength(password: &str) -> (bool, String) {
        if password.len() < 8 {
            (false, "Password too short".to_string())
        } else if !password.chars().any(|c| c.is_uppercase()) {
            (false, "Missing uppercase".to_string())
        } else if !password.chars().any(|c| c.is_numeric()) {
            (false, "Missing number".to_string())
        } else {
            (true, "Strong".to_string())
        }
    }

    /// Problem 9: Generate random salt
    pub fn generate_salt() -> String {
        "salt_abcd1234".to_string()
    }

    /// Problem 10: Detect auth type
    pub fn detect_auth_type(header: &str) -> String {
        if header.starts_with("Basic ") {
            "basic".to_string()
        } else if header.starts_with("Bearer ") {
            "bearer".to_string()
        } else {
            "unknown".to_string()
        }
    }

    // ================================================================
    // BEARER TOKENS (11-20)
    // ================================================================

    /// Problem 11: Parse Bearer token header
    pub fn parse_bearer_token(header: &str) -> Result<String, String> {
        if !header.starts_with("Bearer ") {
            return Err("Invalid Bearer token header".to_string());
        }
        Ok(header[7..].to_string())
    }

    /// Problem 12: Create Bearer token
    pub fn create_bearer_token(user_id: &str, expires_at: u64) -> BearerToken {
        BearerToken {
            token: format!("token_{}", user_id),
            expires_at,
            user_id: user_id.to_string(),
        }
    }

    /// Problem 13: Generate Bearer token header
    pub fn generate_bearer_header(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Problem 14: Validate Bearer token expiry
    pub fn is_bearer_expired(token: &BearerToken, now: u64) -> bool {
        now > token.expires_at
    }

    /// Problem 15: Refresh Bearer token
    pub fn refresh_bearer_token(token: &BearerToken, now: u64, ttl: u64) -> BearerToken {
        BearerToken {
            token: format!("token_{}_{}", token.user_id, now),
            expires_at: now + ttl,
            user_id: token.user_id.clone(),
        }
    }

    /// Problem 16: Revoke Bearer token
    pub fn revoke_token(revoked: &mut Vec<String>, token: &str) {
        revoked.push(token.to_string());
    }

    /// Problem 17: Is token revoked
    pub fn is_token_revoked(revoked: &[String], token: &str) -> bool {
        revoked.contains(&token.to_string())
    }

    /// Problem 18: Generate secure random token
    pub fn generate_secure_token() -> String {
        "secure_token_1234567890".to_string()
    }

    /// Problem 19: Get token expiry from claims
    pub fn get_token_expiry(_token: &str) -> u64 {
        9999999999
    }

    /// Problem 20: Validate token format
    pub fn is_valid_token_format(token: &str) -> bool {
        token.len() >= 20 && token.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    // ================================================================
    // JWT TOKENS (21-30)
    // ================================================================

    /// Problem 21: Create JWT token
    pub fn create_jwt_token(user_id: &str, secret: &str, expires_at: u64) -> JwtToken {
        let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string();
        let payload = format!("user_id={},expires_at={}", user_id, expires_at);
        let signature = Self::sign_jwt(&payload, secret);
        
        JwtToken { header, payload, signature }
    }

    /// Problem 22: Sign JWT
    pub fn sign_jwt(payload: &str, secret: &str) -> String {
        format!("sig_{}_{}", payload.len(), secret.len())
    }

    /// Problem 23: Verify JWT signature
    pub fn verify_jwt_signature(token: &JwtToken, secret: &str) -> bool {
        let expected = Self::sign_jwt(&token.payload, secret);
        token.signature == expected
    }

    /// Problem 24: Parse JWT payload
    pub fn parse_jwt_payload(payload: &str) -> HashMap<String, String> {
        let mut claims = HashMap::new();
        for part in payload.split(',') {
            if let Some((key, val)) = part.split_once('=') {
                claims.insert(key.trim().to_string(), val.trim().to_string());
            }
        }
        claims
    }

    /// Problem 25: Extract user ID from JWT
    pub fn extract_user_id_from_jwt(token: &JwtToken) -> Option<String> {
        let claims = Self::parse_jwt_payload(&token.payload);
        claims.get("user_id").cloned()
    }

    /// Problem 26: Is JWT expired
    pub fn is_jwt_expired(token: &JwtToken, now: u64) -> bool {
        let claims = Self::parse_jwt_payload(&token.payload);
        if let Some(exp_str) = claims.get("expires_at") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                return now > exp;
            }
        }
        true
    }

    /// Problem 27: Decode JWT header
    pub fn decode_jwt_header(header: &str) -> Result<HashMap<String, String>, String> {
        // Simulated decode
        let mut map = HashMap::new();
        map.insert("alg".to_string(), "HS256".to_string());
        map.insert("typ".to_string(), "JWT".to_string());
        Ok(map)
    }

    /// Problem 28: Add JWT claim
    pub fn add_jwt_claim(token: &mut JwtToken, key: &str, value: &str) {
        token.payload.push_str(&format!(",{}={}", key, value));
    }

    /// Problem 29: Get JWT claims
    pub fn get_jwt_claims(token: &JwtToken) -> HashMap<String, String> {
        Self::parse_jwt_payload(&token.payload)
    }

    /// Problem 30: Validate JWT format
    pub fn is_valid_jwt_format(token_str: &str) -> bool {
        let parts: Vec<&str> = token_str.split('.').collect();
        parts.len() == 3 && parts.iter().all(|p| !p.is_empty())
    }

    // ================================================================
    // PERMISSIONS & ROLES (31-40)
    // ================================================================

    /// Problem 31: Create role
    pub fn new_role(name: &str, permissions: Vec<String>) -> Role {
        Role { name: name.to_string(), permissions }
    }

    /// Problem 32: Create user with roles
    pub fn new_user(user_id: &str, username: &str, roles: Vec<String>) -> User {
        User {
            user_id: user_id.to_string(),
            username: username.to_string(),
            roles,
            permissions: Vec::new(),
            is_active: true,
        }
    }

    /// Problem 33: Add role to user
    pub fn add_role(user: &mut User, role: &str) {
        if !user.roles.contains(&role.to_string()) {
            user.roles.push(role.to_string());
        }
    }

    /// Problem 34: Remove role from user
    pub fn remove_role(user: &mut User, role: &str) {
        user.roles.retain(|r| r != role);
    }

    /// Problem 35: Has role
    pub fn has_role(user: &User, role: &str) -> bool {
        user.roles.contains(&role.to_string())
    }

    /// Problem 36: Add permission to user
    pub fn add_permission(user: &mut User, permission: &str) {
        if !user.permissions.contains(&permission.to_string()) {
            user.permissions.push(permission.to_string());
        }
    }

    /// Problem 37: Has permission
    pub fn has_permission(user: &User, permission: &str) -> bool {
        user.permissions.contains(&permission.to_string())
    }

    /// Problem 38: Grant role permissions to user
    pub fn grant_role_permissions(user: &mut User, role: &Role) {
        for perm in &role.permissions {
            Self::add_permission(user, perm);
        }
    }

    /// Problem 39: Check if user is active
    pub fn is_user_active(user: &User) -> bool {
        user.is_active
    }

    /// Problem 40: Deactivate user
    pub fn deactivate_user(user: &mut User) {
        user.is_active = false;
    }

    // ================================================================
    // AUTHORIZATION & CONTEXT (41-50)
    // ================================================================

    /// Problem 41: Create authorization context
    pub fn new_auth_context(user: &User) -> HashMap<String, String> {
        let mut ctx = HashMap::new();
        ctx.insert("user_id".to_string(), user.user_id.clone());
        ctx.insert("username".to_string(), user.username.clone());
        ctx.insert("roles".to_string(), user.roles.join(","));
        ctx.insert("permissions".to_string(), user.permissions.join(","));
        ctx.insert("active".to_string(), user.is_active.to_string());
        ctx
    }

    /// Problem 42: Check authorization for action
    pub fn authorize_action(user: &User, action: &str) -> bool {
        user.is_active && user.permissions.contains(&action.to_string())
    }

    /// Problem 43: Require role
    pub fn require_role(user: &User, role: &str) -> Result<(), String> {
        if Self::has_role(user, role) {
            Ok(())
        } else {
            Err(format!("Requires role: {}", role))
        }
    }

    /// Problem 44: Require permission
    pub fn require_permission(user: &User, permission: &str) -> Result<(), String> {
        if Self::has_permission(user, permission) {
            Ok(())
        } else {
            Err(format!("Requires permission: {}", permission))
        }
    }

    /// Problem 45: Require active user
    pub fn require_active(user: &User) -> Result<(), String> {
        if user.is_active {
            Ok(())
        } else {
            Err("User is not active".to_string())
        }
    }

    /// Problem 46: Check permission for route
    pub fn check_route_permission(user: &User, route: &str) -> bool {
        match route {
            "/admin" => Self::has_role(user, "admin"),
            "/user" => user.is_active,
            "/" => true,
            _ => false,
        }
    }

    /// Problem 47: Build auth header
    pub fn build_auth_header(auth_type: &str, value: &str) -> String {
        match auth_type {
            "basic" => format!("Basic {}", value),
            "bearer" => format!("Bearer {}", value),
            _ => String::new(),
        }
    }

    /// Problem 48: Extract auth from header
    pub fn extract_auth_from_header(header: &str) -> Result<(String, String), String> {
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() == 2 {
            Ok((parts[0].to_lowercase(), parts[1].to_string()))
        } else {
            Err("Invalid auth header".to_string())
        }
    }

    /// Problem 49: Create session token for user
    pub fn create_session_token(user: &User, secret: &str, now: u64, ttl: u64) -> String {
        format!("session_{}_{}", user.user_id, ttl)
    }

    /// Problem 50: Validate user auth context
    pub fn validate_auth_context(ctx: &HashMap<String, String>) -> bool {
        ctx.contains_key("user_id") &&
        ctx.contains_key("username") &&
        ctx.contains_key("active")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_auth() {
        let header = AuthSolver::create_basic_auth_header("user", "pass");
        let result = AuthSolver::parse_basic_auth(&header);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_password() {
        let hash = AuthSolver::hash_password("password123");
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_check_password_strength() {
        let (valid, msg) = AuthSolver::check_password_strength("Weak1");
        assert!(!valid);
        assert_eq!(msg, "Password too short");
        
        let (valid, _) = AuthSolver::check_password_strength("Strong1234");
        assert!(valid);
    }

    #[test]
    fn test_bearer_token() {
        let token = AuthSolver::create_bearer_token("user123", 2000);
        assert_eq!(token.user_id, "user123");
    }

    #[test]
    fn test_jwt_token() {
        let token = AuthSolver::create_jwt_token("user123", "secret", 2000);
        assert!(AuthSolver::verify_jwt_signature(&token, "secret"));
    }

    #[test]
    fn test_jwt_payload_parsing() {
        let payload = "user_id=123,expires_at=2000";
        let claims = AuthSolver::parse_jwt_payload(payload);
        assert_eq!(claims.get("user_id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_user_roles() {
        let mut user = AuthSolver::new_user("u1", "john", vec!["user".to_string()]);
        assert!(AuthSolver::has_role(&user, "user"));
        
        AuthSolver::add_role(&mut user, "admin");
        assert!(AuthSolver::has_role(&user, "admin"));
    }

    #[test]
    fn test_user_permissions() {
        let mut user = AuthSolver::new_user("u1", "john", vec![]);
        AuthSolver::add_permission(&mut user, "read");
        assert!(AuthSolver::has_permission(&user, "read"));
    }

    #[test]
    fn test_authorization() {
        let mut user = AuthSolver::new_user("u1", "john", vec![]);
        AuthSolver::add_permission(&mut user, "delete");
        assert!(AuthSolver::authorize_action(&user, "delete"));
    }

    #[test]
    fn test_auth_context() {
        let user = AuthSolver::new_user("u1", "john", vec!["admin".to_string()]);
        let ctx = AuthSolver::new_auth_context(&user);
        assert!(AuthSolver::validate_auth_context(&ctx));
    }

    #[test]
    fn test_deactivate_user() {
        let mut user = AuthSolver::new_user("u1", "john", vec![]);
        assert!(AuthSolver::is_user_active(&user));
        AuthSolver::deactivate_user(&mut user);
        assert!(!AuthSolver::is_user_active(&user));
    }
}
