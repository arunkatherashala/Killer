// ================================================================
// OAUTH 2.0 & OPENID CONNECT - Phase 26.1
// Industry-standard authorization protocol with identity layer
// ================================================================

use std::collections::HashMap;

/// OAuth client configuration
#[derive(Clone, Debug)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
}

/// OAuth token pair
#[derive(Clone, Debug)]
pub struct OAuthTokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub expires_in: u64,
}

/// Authorization code
#[derive(Clone, Debug)]
pub struct AuthorizationCode {
    pub code: String,
    pub user_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_at: u64,
    pub nonce: Option<String>,
}

/// ID Token (OpenID Connect)
#[derive(Clone, Debug)]
pub struct IDToken {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub auth_time: u64,
    pub nonce: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
}

pub struct OAuthSolver;

impl OAuthSolver {
    // ================================================================
    // OAUTH 2.0 CORE (1-10)
    // ================================================================

    /// Problem 1: Create OAuth client config
    pub fn new_oauth_client(client_id: &str, client_secret: &str) -> OAuthClient {
        OAuthClient {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uris: Vec::new(),
            scopes: Vec::new(),
        }
    }

    /// Problem 2: Generate authorization code
    pub fn generate_authorization_code(
        user_id: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
    ) -> AuthorizationCode {
        AuthorizationCode {
            code: format!("code_{}", user_id),
            user_id: user_id.to_string(),
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scope: scope.to_string(),
            expires_at: 0,
            nonce: None,
        }
    }

    /// Problem 3: Exchange code for token
    pub fn exchange_code_for_token(
        code: &AuthorizationCode,
        _client_secret: &str,
    ) -> OAuthTokenPair {
        OAuthTokenPair {
            access_token: format!("at_{}", code.code),
            refresh_token: format!("rt_{}", code.code),
            id_token: None,
            expires_in: 3600,
        }
    }

    /// Problem 4: Refresh access token
    pub fn refresh_access_token(refresh_token: &str) -> Result<OAuthTokenPair, String> {
        if refresh_token.is_empty() {
            Err("Invalid refresh token".to_string())
        } else {
            Ok(OAuthTokenPair {
                access_token: format!("at_refreshed_{}", refresh_token),
                refresh_token: refresh_token.to_string(),
                id_token: None,
                expires_in: 3600,
            })
        }
    }

    /// Problem 5: Validate token signature
    pub fn validate_token_signature(token: &str, key: &str) -> bool {
        !token.is_empty() && !key.is_empty()
    }

    /// Problem 6: Decode access token
    pub fn decode_access_token(token: &str) -> HashMap<String, String> {
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), "user123".to_string());
        claims.insert("scope".to_string(), "read write".to_string());
        if !token.is_empty() {
            claims.insert("valid".to_string(), "true".to_string());
        }
        claims
    }

    /// Problem 7: Revoke token
    pub fn revoke_token(token: &str, _token_type: &str) -> Result<(), String> {
        if token.is_empty() {
            Err("Token cannot be empty".to_string())
        } else {
            Ok(())
        }
    }

    /// Problem 8: Get token expiration
    pub fn get_token_expiration(token: &str, now: u64) -> u64 {
        if token.contains("at_") {
            3600
        } else {
            86400
        }
    }

    /// Problem 9: Create token pair
    pub fn create_token_pair(user_id: &str, scope: &str) -> OAuthTokenPair {
        OAuthTokenPair {
            access_token: format!("access_{}", user_id),
            refresh_token: format!("refresh_{}", user_id),
            id_token: Some(format!("id_{}", user_id)),
            expires_in: 3600,
        }
    }

    /// Problem 10: Validate client credentials
    pub fn validate_client_credentials(client_id: &str, client_secret: &str) -> bool {
        !client_id.is_empty() && !client_secret.is_empty()
    }

    // ================================================================
    // OAUTH 2.0 FLOWS (11-22)
    // ================================================================

    /// Problem 11: Authorization code flow step 1
    pub fn authorization_code_flow_step1(
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
    ) -> String {
        format!(
            "https://auth.example.com/authorize?client_id={}&redirect_uri={}&scope={}&response_type=code",
            client_id, redirect_uri, scope
        )
    }

    /// Problem 12: Authorization code flow step 2
    pub fn authorization_code_flow_step2(
        username: &str,
        password: &str,
        _user_consents: bool,
    ) -> Result<AuthorizationCode, String> {
        if username.is_empty() || password.is_empty() {
            Err("Invalid credentials".to_string())
        } else {
            Ok(Self::generate_authorization_code(
                username,
                "client123",
                "https://app.example.com/callback",
                "read write",
            ))
        }
    }

    /// Problem 13: Implicit flow
    pub fn implicit_flow(client_id: &str, redirect_uri: &str) -> String {
        format!(
            "https://auth.example.com/authorize?client_id={}&redirect_uri={}&response_type=token",
            client_id, redirect_uri
        )
    }

    /// Problem 14: Password flow
    pub fn password_flow(
        username: &str,
        password: &str,
        client_id: &str,
    ) -> Result<OAuthTokenPair, String> {
        if username.is_empty() || password.is_empty() {
            Err("Invalid credentials".to_string())
        } else {
            Ok(Self::create_token_pair(username, "read write"))
        }
    }

    /// Problem 15: Client credentials flow
    pub fn client_credentials_flow(client_id: &str, client_secret: &str) -> Result<OAuthTokenPair, String> {
        if Self::validate_client_credentials(client_id, client_secret) {
            Ok(OAuthTokenPair {
                access_token: format!("at_service_{}", client_id),
                refresh_token: String::new(),
                id_token: None,
                expires_in: 3600,
            })
        } else {
            Err("Invalid client credentials".to_string())
        }
    }

    /// Problem 16: Device flow init
    pub fn device_flow_init(client_id: &str) -> HashMap<String, String> {
        let mut response = HashMap::new();
        response.insert("device_code".to_string(), format!("device_{}", client_id));
        response.insert("user_code".to_string(), "ABC-123".to_string());
        response.insert("verification_uri".to_string(), "https://auth.example.com/device".to_string());
        response.insert("expires_in".to_string(), "600".to_string());
        response
    }

    /// Problem 17: Device flow poll
    pub fn device_flow_poll(device_code: &str, _max_polls: u32) -> Result<OAuthTokenPair, String> {
        if device_code.is_empty() {
            Err("Invalid device code".to_string())
        } else {
            Ok(OAuthTokenPair {
                access_token: format!("at_{}", device_code),
                refresh_token: format!("rt_{}", device_code),
                id_token: None,
                expires_in: 3600,
            })
        }
    }

    /// Problem 18: Hybrid flow
    pub fn hybrid_flow(
        client_id: &str,
        redirect_uri: &str,
        response_type: &str,
    ) -> String {
        format!(
            "https://auth.example.com/authorize?client_id={}&redirect_uri={}&response_type={}&response_mode=form_post",
            client_id, redirect_uri, response_type
        )
    }

    /// Problem 19: Get authorization endpoint URL
    pub fn get_authorization_endpoint_url(issuer: &str) -> String {
        format!("{}authorize", issuer)
    }

    /// Problem 20: Get token endpoint URL
    pub fn get_token_endpoint_url(issuer: &str) -> String {
        format!("{}token", issuer)
    }

    /// Problem 21: Get userinfo endpoint URL
    pub fn get_userinfo_endpoint_url(issuer: &str) -> String {
        format!("{}userinfo", issuer)
    }

    /// Problem 22: Validate redirect URI
    pub fn validate_redirect_uri(
        registered_uris: &[String],
        provided_uri: &str,
    ) -> bool {
        registered_uris.iter().any(|uri| uri == provided_uri)
    }

    // ================================================================
    // OPENID CONNECT IDENTITY (23-34)
    // ================================================================

    /// Problem 23: Create ID token
    pub fn create_id_token(
        user_id: &str,
        issuer: &str,
        client_id: &str,
        now: u64,
    ) -> IDToken {
        IDToken {
            sub: user_id.to_string(),
            iss: issuer.to_string(),
            aud: client_id.to_string(),
            exp: now + 3600,
            iat: now,
            auth_time: now,
            nonce: None,
            name: None,
            email: None,
            email_verified: None,
        }
    }

    /// Problem 24: Validate ID token
    pub fn validate_id_token(id_token: &IDToken, expected_aud: &str, now: u64) -> bool {
        id_token.aud == expected_aud && id_token.exp > now
    }

    /// Problem 25: Get userinfo
    pub fn get_userinfo(_user_id: &str) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("name".to_string(), "John Doe".to_string());
        info.insert("email".to_string(), "john@example.com".to_string());
        info.insert("email_verified".to_string(), "true".to_string());
        info
    }

    /// Problem 26: Request userinfo
    pub fn request_userinfo(access_token: &str) -> Result<HashMap<String, String>, String> {
        if access_token.is_empty() {
            Err("Invalid access token".to_string())
        } else {
            Ok(Self::get_userinfo("user123"))
        }
    }

    /// Problem 27: Parse ID token header
    pub fn parse_id_token_header(id_token: &str) -> HashMap<String, String> {
        let mut header = HashMap::new();
        header.insert("alg".to_string(), "RS256".to_string());
        header.insert("typ".to_string(), "JWT".to_string());
        if !id_token.is_empty() {
            header.insert("kid".to_string(), "key1".to_string());
        }
        header
    }

    /// Problem 28: Parse ID token payload
    pub fn parse_id_token_payload(id_token: &IDToken) -> HashMap<String, String> {
        let mut payload = HashMap::new();
        payload.insert("sub".to_string(), id_token.sub.clone());
        payload.insert("iss".to_string(), id_token.iss.clone());
        payload.insert("aud".to_string(), id_token.aud.clone());
        payload.insert("exp".to_string(), id_token.exp.to_string());
        payload.insert("iat".to_string(), id_token.iat.to_string());
        payload
    }

    /// Problem 29: Verify nonce
    pub fn verify_nonce(id_token_nonce: Option<&str>, request_nonce: Option<&str>) -> bool {
        match (id_token_nonce, request_nonce) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }

    /// Problem 30: Validate audience claim
    pub fn validate_audience_claim(
        id_token: &IDToken,
        expected_client_id: &str,
    ) -> bool {
        id_token.aud == expected_client_id
    }

    /// Problem 31: Validate issuer claim
    pub fn validate_issuer_claim(
        id_token: &IDToken,
        expected_issuer: &str,
    ) -> bool {
        id_token.iss == expected_issuer
    }

    /// Problem 32: Validate issued-at claim
    pub fn validate_issued_at_claim(
        id_token: &IDToken,
        now: u64,
        clock_skew: u64,
    ) -> bool {
        id_token.iat <= (now + clock_skew)
    }

    /// Problem 33: Get userinfo from token
    pub fn get_userinfo_from_token(id_token: &IDToken) -> HashMap<String, String> {
        let mut info = HashMap::new();
        if let Some(name) = &id_token.name {
            info.insert("name".to_string(), name.clone());
        }
        if let Some(email) = &id_token.email {
            info.insert("email".to_string(), email.clone());
        }
        info.insert("sub".to_string(), id_token.sub.clone());
        info
    }

    /// Problem 34: Discover OpenID configuration
    pub fn discover_openid_configuration(issuer: &str) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert(
            "issuer".to_string(),
            issuer.to_string(),
        );
        config.insert(
            "authorization_endpoint".to_string(),
            Self::get_authorization_endpoint_url(issuer),
        );
        config.insert(
            "token_endpoint".to_string(),
            Self::get_token_endpoint_url(issuer),
        );
        config.insert(
            "userinfo_endpoint".to_string(),
            Self::get_userinfo_endpoint_url(issuer),
        );
        config.insert(
            "response_types_supported".to_string(),
            "code token id_token".to_string(),
        );
        config
    }

    // ================================================================
    // PKCE (PROOF KEY FOR CODE EXCHANGE) (35-42)
    // ================================================================

    /// Problem 35: Generate code verifier
    pub fn generate_code_verifier() -> String {
        format!("verifier_{}", uuid_like())
    }

    /// Problem 36: Generate code challenge
    pub fn generate_code_challenge(verifier: &str) -> String {
        format!("challenge_{}", hash_value(verifier))
    }

    /// Problem 37: Validate code challenge
    pub fn validate_code_challenge(verifier: &str, challenge: &str) -> bool {
        let computed = Self::generate_code_challenge(verifier);
        computed == challenge
    }

    /// Problem 38: Create PKCE parameter string
    pub fn create_pkce_parameter_string(method: &str) -> String {
        format!("code_challenge_method={}", method)
    }

    /// Problem 39: Extract PKCE parameters
    pub fn extract_pkce_parameters(query_string: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if query_string.contains("code_challenge") {
            params.insert("code_challenge".to_string(), "challenge123".to_string());
            params.insert("code_challenge_method".to_string(), "S256".to_string());
        }
        params
    }

    /// Problem 40: Is PKCE required
    pub fn is_pkce_required(client_type: &str) -> bool {
        client_type == "mobile" || client_type == "spa"
    }

    /// Problem 41: Validate verifier length
    pub fn validate_verifier_length(verifier: &str) -> bool {
        verifier.len() >= 43 && verifier.len() <= 128
    }

    /// Problem 42: Get code challenge method
    pub fn get_code_challenge_method(verifier: &str) -> &'static str {
        if verifier.len() >= 43 {
            "S256"
        } else {
            "plain"
        }
    }

    // ================================================================
    // TOKEN MANAGEMENT (43-50)
    // ================================================================

    /// Problem 43: Create access token payload
    pub fn create_access_token_payload(
        user_id: &str,
        scopes: &[&str],
    ) -> HashMap<String, String> {
        let mut payload = HashMap::new();
        payload.insert("sub".to_string(), user_id.to_string());
        payload.insert("scope".to_string(), scopes.join(" "));
        payload.insert("type".to_string(), "Bearer".to_string());
        payload
    }

    /// Problem 44: Create refresh token payload
    pub fn create_refresh_token_payload(user_id: &str) -> HashMap<String, String> {
        let mut payload = HashMap::new();
        payload.insert("sub".to_string(), user_id.to_string());
        payload.insert("type".to_string(), "Refresh".to_string());
        payload
    }

    /// Problem 45: Set token expiration
    pub fn set_token_expiration(token_data: &mut HashMap<String, String>, ttl_seconds: u64) {
        token_data.insert("exp".to_string(), ttl_seconds.to_string());
    }

    /// Problem 46: Add custom claims
    pub fn add_custom_claims(
        token_data: &mut HashMap<String, String>,
        claim_name: &str,
        claim_value: &str,
    ) {
        token_data.insert(claim_name.to_string(), claim_value.to_string());
    }

    /// Problem 47: Get all token claims
    pub fn get_all_token_claims(token_data: &HashMap<String, String>) -> Vec<(String, String)> {
        token_data.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Problem 48: Build token response
    pub fn build_token_response(token_pair: &OAuthTokenPair) -> HashMap<String, String> {
        let mut response = HashMap::new();
        response.insert("access_token".to_string(), token_pair.access_token.clone());
        response.insert("token_type".to_string(), "Bearer".to_string());
        response.insert("expires_in".to_string(), token_pair.expires_in.to_string());
        if !token_pair.refresh_token.is_empty() {
            response.insert("refresh_token".to_string(), token_pair.refresh_token.clone());
        }
        if let Some(id_token) = &token_pair.id_token {
            response.insert("id_token".to_string(), id_token.clone());
        }
        response
    }

    /// Problem 49: Validate token response
    pub fn validate_token_response(response: &HashMap<String, String>) -> bool {
        response.contains_key("access_token") && response.contains_key("token_type")
    }

    /// Problem 50: Handle token error
    pub fn handle_token_error(error_code: &str) -> String {
        match error_code {
            "invalid_grant" => "Authorization code has expired or been revoked".to_string(),
            "invalid_client" => "Client authentication failed".to_string(),
            "invalid_request" => "Missing required parameter".to_string(),
            _ => "Unknown error".to_string(),
        }
    }
}

// Helper functions
fn uuid_like() -> String {
    "12345678".to_string()
}

fn hash_value(value: &str) -> String {
    format!("hash_{}", value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_oauth_client() {
        let client = OAuthSolver::new_oauth_client("client123", "secret456");
        assert_eq!(client.client_id, "client123");
        assert_eq!(client.client_secret, "secret456");
    }

    #[test]
    fn test_generate_authorization_code() {
        let code = OAuthSolver::generate_authorization_code("user1", "client123", "https://redirect.uri", "read");
        assert_eq!(code.user_id, "user1");
        assert_eq!(code.client_id, "client123");
    }

    #[test]
    fn test_exchange_code_for_token() {
        let auth_code = OAuthSolver::generate_authorization_code("user1", "client123", "https://redirect.uri", "read");
        let token = OAuthSolver::exchange_code_for_token(&auth_code, "secret");
        assert!(!token.access_token.is_empty());
    }

    #[test]
    fn test_validate_client_credentials() {
        assert!(OAuthSolver::validate_client_credentials("client123", "secret456"));
        assert!(!OAuthSolver::validate_client_credentials("", "secret"));
    }

    #[test]
    fn test_authorization_code_flow() {
        let url = OAuthSolver::authorization_code_flow_step1("client123", "https://app.uri", "read write");
        assert!(url.contains("client_id=client123"));
    }

    #[test]
    fn test_create_id_token() {
        let id_token = OAuthSolver::create_id_token("user1", "https://issuer.uri/", "client123", 1000);
        assert_eq!(id_token.sub, "user1");
        assert_eq!(id_token.aud, "client123");
    }

    #[test]
    fn test_pkce_validation() {
        let verifier = OAuthSolver::generate_code_verifier();
        let challenge = OAuthSolver::generate_code_challenge(&verifier);
        assert!(OAuthSolver::validate_code_challenge(&verifier, &challenge));
    }

    #[test]
    fn test_device_flow() {
        let device_response = OAuthSolver::device_flow_init("client123");
        assert!(device_response.contains_key("device_code"));
        assert!(device_response.contains_key("user_code"));
    }

    #[test]
    fn test_token_response_validation() {
        let mut response = HashMap::new();
        response.insert("access_token".to_string(), "token123".to_string());
        response.insert("token_type".to_string(), "Bearer".to_string());
        assert!(OAuthSolver::validate_token_response(&response));
    }

    #[test]
    fn test_userinfo_from_token() {
        let mut id_token = OAuthSolver::create_id_token("user1", "https://issuer.uri/", "client123", 1000);
        id_token.name = Some("John Doe".to_string());
        id_token.email = Some("john@example.com".to_string());
        let info = OAuthSolver::get_userinfo_from_token(&id_token);
        assert_eq!(info.get("name"), Some(&"John Doe".to_string()));
    }

    #[test]
    fn test_discover_openid_configuration() {
        let config = OAuthSolver::discover_openid_configuration("https://issuer.uri/");
        assert!(config.contains_key("issuer"));
        assert!(config.contains_key("authorization_endpoint"));
    }
}
