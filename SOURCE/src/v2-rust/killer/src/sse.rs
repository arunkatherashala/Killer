//! Server-Sent Events (SSE) — Push notifications to clients

pub struct SseEvent { pub data: String }

impl SseEvent {
    pub fn new(data: &str) -> Self { Self { data: data.to_string() } }
    pub fn format(&self) -> String { format!("data: {}\n\n", self.data) }
}
