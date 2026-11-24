// Local AI Provider (ONNX Runtime)
// src/ai/providers/local.rs
//
// On-device inference using ONNX Runtime
// No external API calls, full privacy preservation

use super::Provider;
use crate::value::Value;
use std::collections::HashMap;
use crate::ai::ClassifyResult;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::time::Duration;

/// Local ONNX Provider
#[allow(dead_code)]
pub struct LocalProvider {
    cache: HashMap<String, Vec<f32>>,
}

impl LocalProvider {
    /// Create new local provider
    pub fn new() -> Self {
        LocalProvider {
            cache: HashMap::new(),
        }
    }

    /// ONNX loading is not implemented (no ONNX runtime is linked).
    fn load_model(&self, model_path: &str) -> Result<(), String> {
        if !model_path.ends_with(".onnx") {
            return Err(format!("Invalid model file: {}. Must be .onnx file", model_path));
        }
        Err("local ONNX inference is not implemented (no ONNX runtime is linked)".to_string())
    }

    /// Try connecting to a local Ollama instance for text generation.
    /// Uses configured model from KhLmPolyglotConfig if `model` is empty.
    /// Returns `None` if Ollama is unreachable so the caller can fall back.
    fn try_ollama_with_model(prompt: &str, model: &str) -> Option<String> {
        let model_name = if model.is_empty() {
            let cfg = crate::khlm_polyglot::config().lock().ok()?;
            if !cfg.llm_model.is_empty() {
                cfg.llm_model.clone()
            } else {
                "llama3".to_string()
            }
        } else {
            model.to_string()
        };

        let host = "127.0.0.1";
        let port = 11434u16;
        let addr = format!("{}:{}", host, port);
        let timeout = Duration::from_secs(2);

        let mut stream = TcpStream::connect_timeout(
            &addr.parse().ok()?,
            timeout,
        ).ok()?;

        stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        let escaped_model = model_name
            .replace('\\', "\\\\")
            .replace('"', "\\\"");

        let body = format!(
            r#"{{"model":"{}","prompt":"{}","stream":false}}"#,
            escaped_model, escaped_prompt
        );

        let request = format!(
            "POST /api/generate HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            host, port, body.len(), body
        );

        stream.write_all(request.as_bytes()).ok()?;
        stream.flush().ok()?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).ok()?;

        let text = String::from_utf8_lossy(&response).to_string();

        // Strip HTTP headers
        let body_str = if let Some(pos) = text.find("\r\n\r\n") {
            &text[pos + 4..]
        } else if let Some(pos) = text.find("\n\n") {
            &text[pos + 2..]
        } else {
            &text
        };

        // Extract "response" value from Ollama JSON via string searching
        let key = "\"response\"";
        let idx = body_str.find(key)?;
        let after_key = &body_str[idx + key.len()..];
        let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;

        let mut result = String::new();
        let mut chars = after_colon.chars();
        loop {
            match chars.next() {
                None => return None,
                Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some(c) => { result.push('\\'); result.push(c); }
                    None => return None,
                },
                Some(c) => result.push(c),
            }
        }

        if result.is_empty() { None } else { Some(result) }
    }

    /// ONNX inference is not implemented.
    #[allow(dead_code)]
    fn run_inference(&self, _input: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        Err("local ONNX inference is not implemented".to_string())
    }
}

impl Default for LocalProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for LocalProvider {
    fn generate(&self, prompt: &str, model: &str, _temperature: f64, _max_tokens: usize) 
        -> Result<String, String> {
        
        if prompt.is_empty() {
            return Err("Prompt cannot be empty".to_string());
        }

        if let Some(response) = LocalProvider::try_ollama_with_model(prompt, model) {
            return Ok(response);
        }

        Err("local provider: Ollama is not reachable at 127.0.0.1:11434 (no other local backend is implemented)".to_string())
    }

    fn embed(&self, text: &str, _model: &str) -> Result<Vec<f32>, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }
        Err("local provider: embeddings are not implemented".to_string())
    }

    fn classify(&self, text: &str, categories: Vec<String>, _model: &str)
        -> Result<ClassifyResult, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }
        if categories.is_empty() {
            return Err("Must provide at least one category".to_string());
        }
        Err("local provider: classification is not implemented".to_string())
    }

    fn extract(&self, text: &str, _schema: HashMap<String, String>, _model: &str)
        -> Result<HashMap<String, Value>, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }
        Err("local provider: extraction is not implemented".to_string())
    }

    fn local_infer(&self, model_path: &str, input: HashMap<String, Value>)
        -> Result<HashMap<String, Value>, String> {
        
        // Load model
        self.load_model(model_path)?;

        // Run inference
        self.run_inference(input)
    }

    fn name(&self) -> &str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_creation() {
        let provider = LocalProvider::new();
        assert_eq!(provider.cache.len(), 0);
    }

    #[test]
    fn unimplemented_features_error_instead_of_returning_fake_results() {
        let provider = LocalProvider::new();
        assert!(provider.embed("hello", "bert").is_err());
        assert!(provider.classify("hello", vec!["a".to_string()], "m").is_err());
        assert!(provider.extract("hello", HashMap::new(), "m").is_err());
        assert!(provider.local_infer("model.onnx", HashMap::new()).is_err());
        assert!(provider.local_infer("invalid.bin", HashMap::new()).is_err());
    }
}
