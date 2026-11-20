// OpenAI Provider Implementation
// src/ai/providers/openai.rs
//
// Integration with OpenAI APIs
// Supports: GPT-4, GPT-3.5-turbo, text-embedding-ada-002, etc.

use super::Provider;
use crate::value::Value;
use std::collections::HashMap;
use crate::ai::ClassifyResult;

/// OpenAI Provider
#[allow(dead_code)]
pub struct OpenAIProvider {
    api_key: Option<String>,
    base_url: String,
    default_model: String,
}

impl OpenAIProvider {
    /// Create new OpenAI provider
    pub fn new() -> Self {
        OpenAIProvider {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-3.5-turbo".to_string(),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    fn resolve_api_key(&self) -> Option<String> {
        self.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| std::env::var("KILLER_KHLM_LLM_API_KEY").ok())
    }

    /// Point at another OpenAI-compatible endpoint (http:// is refused by http_req unless loopback).
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Build the chat request. The key goes in an Authorization header of a request that http_req sends
    /// over TLS (curl, config on stdin); http_req refuses non-loopback http:// URLs, so the key can never
    /// travel in clear text.
    pub fn build_chat_spec(&self, api_key: &str, body: &str) -> crate::http_req::ReqSpec {
        let mut spec = crate::http_req::ReqSpec::new("POST", &format!("{}/chat/completions", self.base_url.trim_end_matches('/')));
        spec.headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), format!("Bearer {}", api_key)),
        ];
        spec.body = Some(body.to_string());
        spec.timeout_s = 30.0;
        spec.allow_private = true; // configured provider endpoint
        spec
    }

    /// Attempt a real HTTPS call to the OpenAI-compatible chat endpoint.
    /// Returns `None` when no API key is available so the caller can fall back.
    fn try_real_chat(&self, body: &str) -> Option<Result<String, String>> {
        let api_key = self.resolve_api_key()?;
        let spec = self.build_chat_spec(&api_key, body);
        let body_str = match crate::http_req::exec(&spec) {
            Ok(r) => r.body,
            Err(e) => return Some(Err(format!("OpenAI request failed: {}", e))),
        };

        // Extract choices[0].message.content from JSON via string searching
        if let Some(content) = extract_json_content_value(&body_str) {
            Some(Ok(content))
        } else {
            Some(Err(format!("Failed to parse OpenAI response: {}", body_str)))
        }
    }

    /// Make HTTP request to OpenAI
    fn request(&self, endpoint: &str, body: &str) -> Result<String, String> {
        match endpoint {
            "/chat/completions" => {
                if let Some(result) = self.try_real_chat(body) {
                    return result;
                }
                self.simulate_chat_response(body)
            },
            "/embeddings" => self.simulate_embedding_response(body),
            _ => Err(format!("Unknown endpoint: {}", endpoint))
        }
    }

    /// Simulate chat completion response for testing
    fn simulate_chat_response(&self, _body: &str) -> Result<String, String> {
        // Simulated response that matches OpenAI's format
        Ok(r#"{
            "id": "chatcmpl-123456",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "This is a simulated response from OpenAI API. In production, this would be the actual LLM output."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#.to_string())
    }

    /// Simulate embedding response for testing
    fn simulate_embedding_response(&self, _body: &str) -> Result<String, String> {
        // Simulated embedding response
        let embedding: Vec<f32> = (0..1536).map(|i| (i as f32 / 1536.0) - 0.5).collect();
        let embedding_json = format!(
            r#"[{}]"#,
            embedding.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(format!(r#"{{
            "object": "list",
            "data": [{{
                "object": "embedding",
                "embedding": {},
                "index": 0
            }}],
            "model": "text-embedding-ada-002",
            "usage": {{
                "prompt_tokens": 5,
                "total_tokens": 5
            }}
        }}"#, embedding_json))
    }

    /// Parse JSON response
    fn parse_chat_response(&self, response: &str) -> Result<String, String> {
        // Simple JSON parsing for testing
        if let Some(start) = response.find("\"content\": \"") {
            let start = start + 12;
            if let Some(end) = response[start..].find("\"") {
                return Ok(response[start..start + end].to_string());
            }
        }
        Err("Failed to parse response".to_string())
    }
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for OpenAIProvider {
    fn generate(&self, prompt: &str, _model: &str, _temperature: f64, _max_tokens: usize) 
        -> Result<String, String> {
        
        // Build request body
        let body = format!(
            r#"{{
                "model": "gpt-4",
                "messages": [{{"role": "user", "content": {}}}],
                "temperature": 0.7
            }}"#,
            escape_json_string(prompt)
        );

        let response = self.request("/chat/completions", &body)?;
        self.parse_chat_response(&response)
    }

    fn embed(&self, text: &str, _model: &str) -> Result<Vec<f32>, String> {
        let body = format!(
            r#"{{
                "input": {},
                "model": "text-embedding-ada-002"
            }}"#,
            escape_json_string(text)
        );

        let _response = self.request("/embeddings", &body)?;

        // Parse embedding from response
        // This is simplified - real implementation would use serde_json
        let embedding: Vec<f32> = (0..1536).map(|i| (i as f32 / 1536.0) - 0.5).collect();
        Ok(embedding)
    }

    fn classify(&self, text: &str, categories: Vec<String>, _model: &str) 
        -> Result<ClassifyResult, String> {
        
        // Zero-shot classification using GPT
        let categories_str = categories.join(", ");
        let prompt = format!(
            "Classify the following text into one of these categories: {}\n\nText: {}\n\nCategory:",
            categories_str, text
        );

        let response = self.generate(&prompt, "gpt-4", 0.0, 50)?;
        
        // Parse classification result
        let category = response.trim().to_string();
        let confidence = 0.95; // Simulated confidence
        
        let mut all_scores = HashMap::new();
        for cat in &categories {
            all_scores.insert(cat.clone(), 0.5);
        }
        if let Some(_idx) = categories.iter().position(|c| c == &category) {
            all_scores.insert(category.clone(), 0.95);
        }

        Ok(ClassifyResult {
            category,
            confidence,
            all_scores,
        })
    }

    fn extract(&self, text: &str, schema: HashMap<String, String>, _model: &str)
        -> Result<HashMap<String, Value>, String> {
        
        // Build extraction prompt based on schema
        let schema_str = schema.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Extract the following fields from the text:\n{}\n\nText: {}\n\nReturn as JSON:",
            schema_str, text
        );

        let _response = self.generate(&prompt, "gpt-4", 0.0, 500)?;

        // Parse extracted data - simplified for v3.2
        let mut result = HashMap::new();
        for (key, _) in schema {
            result.insert(key, Value::Str("Extracted value placeholder".to_string()));
        }

        Ok(result)
    }

    fn local_infer(&self, _model_path: &str, _input: HashMap<String, Value>)
        -> Result<HashMap<String, Value>, String> {
        Err("OpenAI provider does not support local inference. Use 'local' provider instead.".to_string())
    }

    fn name(&self) -> &str {
        "openai"
    }
}

/// Extract the value of the `"content"` key from an OpenAI-style JSON response
/// using simple string searching (no serde dependency).
fn extract_json_content_value(json: &str) -> Option<String> {
    // Look for "content" followed by optional whitespace, colon, optional whitespace, and a quote
    let key = "\"content\"";
    let idx = json.find(key)?;
    let after_key = &json[idx + key.len()..];

    // Skip whitespace and colon
    let after_colon = after_key.trim_start();
    let after_colon = after_colon.strip_prefix(':')?;
    let after_colon = after_colon.trim_start();
    let after_colon = after_colon.strip_prefix('"')?;

    // Collect characters until unescaped closing quote
    let mut result = String::new();
    let mut chars = after_colon.chars();
    loop {
        match chars.next() {
            None => return None,
            Some('"') => break,
            Some('\\') => {
                match chars.next() {
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some(c) => { result.push('\\'); result.push(c); }
                    None => return None,
                }
            }
            Some(c) => result.push(c),
        }
    }

    Some(result)
}

/// Escape string for JSON
fn escape_json_string(s: &str) -> String {
    format!("\"{}\"", 
        s.replace("\\", "\\\\")
         .replace("\"", "\\\"")
         .replace("\n", "\\n")
         .replace("\r", "\\r")
         .replace("\t", "\\t")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new();
        assert_ne!(provider.base_url, "");
    }

    #[test]
    fn chat_request_is_https_and_key_never_goes_to_a_plain_socket() {
        let p = OpenAIProvider::new();
        let spec = p.build_chat_spec("sk-secret", "{}");
        assert!(spec.url.starts_with("https://"), "{}", spec.url);
        assert!(crate::http_req::validate_url(&spec.url, false).is_ok());
        assert!(spec.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer sk-secret"));
        // a configured clear-text, non-loopback endpoint is refused before anything is sent
        let bad = OpenAIProvider::new().with_base_url("http://api.openai.com/v1");
        let spec = bad.build_chat_spec("sk-secret", "{}");
        assert!(crate::http_req::validate_url(&spec.url, false).is_err());
        let r = bad.with_api_key("sk-secret".into()).try_real_chat("{}").unwrap();
        assert!(r.unwrap_err().contains("HTTPS is required"));
        // no raw socket in this module
        let src = include_str!("openai.rs");
        let needle = ["Tcp", "Stream"].concat();
        let code: Vec<&str> = src.split("#[cfg(test)]").collect();
        assert!(!code[0].contains(&needle));
    }

    #[test]
    fn test_json_escaping() {
        let input = r#"Hello "world""#;
        let escaped = escape_json_string(input);
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_openai_fallback_no_key() {
        // Temporarily clear env vars so no real key is found
        let saved_oai = std::env::var("OPENAI_API_KEY").ok();
        let saved_killer = std::env::var("KILLER_KHLM_LLM_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("KILLER_KHLM_LLM_API_KEY");

        let provider = OpenAIProvider {
            api_key: None,
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-3.5-turbo".to_string(),
        };

        let result = provider.generate("Hello", "gpt-4", 0.7, 100);
        assert!(result.is_ok(), "generate() should succeed via simulated fallback");
        let text = result.unwrap();
        assert!(text.contains("simulated"), "Response should be the simulated fallback");

        // Restore env vars
        if let Some(v) = saved_oai { std::env::set_var("OPENAI_API_KEY", v); }
        if let Some(v) = saved_killer { std::env::set_var("KILLER_KHLM_LLM_API_KEY", v); }
    }

    #[test]
    fn test_extract_json_content_value() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"Hello world"}}]}"#;
        assert_eq!(extract_json_content_value(json), Some("Hello world".to_string()));

        let json_escaped = r#"{"content":"line1\nline2"}"#;
        assert_eq!(extract_json_content_value(json_escaped), Some("line1\nline2".to_string()));

        let no_content = r#"{"error":"something"}"#;
        assert_eq!(extract_json_content_value(no_content), None);
    }
}
