//! Session Management v2 — Enhanced session lifecycle

pub struct Session {
    pub id: String,
    pub created_at: u64,
}

impl Session {
    pub fn new(id: String) -> Self {
        Self {
            id,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
