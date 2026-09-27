//! Token Introspection — Validate and inspect tokens (JWT, OAuth)

pub struct TokenInfo {
    pub active: bool,
    pub scope: String,
}

impl TokenInfo {
    pub fn new() -> Self { Self { active: false, scope: String::new() } }
}
