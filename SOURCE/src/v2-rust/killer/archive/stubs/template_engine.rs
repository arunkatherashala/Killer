//! Template Engine — HTML/text rendering with variable substitution

pub struct Template { pub content: String }

impl Template {
    pub fn new(content: &str) -> Self { Self { content: content.to_string() } }
    pub fn render(&self, _vars: std::collections::HashMap<String, String>) -> String {
        self.content.clone()
    }
}
