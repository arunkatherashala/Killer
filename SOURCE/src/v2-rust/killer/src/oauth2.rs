//! OAuth 2.0 — Authorization protocol implementation

pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
}

impl OAuth2Config {
    pub fn new(client_id: &str, client_secret: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        }
    }
}
