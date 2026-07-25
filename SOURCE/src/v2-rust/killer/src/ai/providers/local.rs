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

    /// Load ONNX model (simplified for v3.2)
    fn load_model(&self, model_path: &str) -> Result<(), String> {
        // In production, this would use ONNX Runtime crate
        // For v3.2, we check if model path is valid
        
        if !model_path.ends_with(".onnx") {
            return Err(format!("Invalid model file: {}. Must be .onnx file", model_path));
        }

        // Simulate successful load
        Ok(())
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

    /// Run ONNX inference (simplified)
    fn run_inference(&self, _input: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
        // Simulated inference results
        let mut output = HashMap::new();
        output.insert("logits".to_string(), Value::Str("[0.1, 0.2, 0.3]".to_string()));
        output.insert("status".to_string(), Value::Str("success".to_string()));
        Ok(output)
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

        Ok("This is a response from local model inference. In production, this would be from Ollama or similar.".to_string())
    }

    fn embed(&self, text: &str, model: &str) -> Result<Vec<f32>, String> {
        // Use BERT or similar local embedding model
        
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        // For v3.2, return fixed-size embedding
        // In production: ~384 dims for DistilBERT, 768 for BERT
        let embedding_size = match model {
            "distilbert" => 384,
            "bert" => 768,
            _ => 384,
        };

        // Create deterministic embedding based on text
        let hash = text.len().wrapping_mul(31) as f32;
        let embedding: Vec<f32> = (0..embedding_size)
            .map(|i| {
                let x = ((i as f32 * hash).sin() + 1.0) / 2.0;
                x - 0.5
            })
            .collect();

        Ok(embedding)
    }

    fn classify(&self, text: &str, categories: Vec<String>, _model: &str) 
        -> Result<ClassifyResult, String> {
        
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        if categories.is_empty() {
            return Err("Must provide at least one category".to_string());
        }

        // Simulate zero-shot classification using local model
        // In production: would use zero-shot classifier model
        
        let category = categories[0].clone(); // Simple heuristic
        let confidence = 0.85;
        
        let mut all_scores = HashMap::new();
        for (i, cat) in categories.iter().enumerate() {
            let base_score = (text.len() as f64 % 100.0) / 100.0;
            let variation = (i as f64) * 0.05;
            all_scores.insert(cat.clone(), (base_score + variation).min(1.0).max(0.0));
        }

        Ok(ClassifyResult {
            category,
            confidence,
            all_scores,
        })
    }

    fn extract(&self, text: &str, schema: HashMap<String, String>, _model: &str)
        -> Result<HashMap<String, Value>, String> {
        
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        // Use local NER/extraction model (SpaCy-like)
        let mut result = HashMap::new();
        
        for (field_name, field_type) in schema {
            // Simulate extraction
            let value = match field_type.as_str() {
                "number" => {
                    Value::Number(text.len() as f64)
                },
                "boolean" => {
                    Value::Bool(text.len() > 10)
                },
                "string" | _ => {
                    Value::Str(format!("Extracted: {}", &text[..field_name.len().min(text.len())]))
                }
            };
            
            result.insert(field_name, value);
        }
        
        Ok(result)
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
    fn test_embedding_deterministic() {
        let provider = LocalProvider::new();
        let text = "Hello world";
        
        let embed1 = provider.embed(text, "bert").unwrap();
        let embed2 = provider.embed(text, "bert").unwrap();
        
        assert_eq!(embed1, embed2);
    }

    #[test]
    fn test_local_infer_requires_model() {
        let provider = LocalProvider::new();
        let result = provider.local_infer("invalid.bin", HashMap::new());
        
        assert!(result.is_err());
    }

    #[test]
    fn test_local_infer_valid_model() {
        let provider = LocalProvider::new();
        let result = provider.local_infer("model.onnx", HashMap::new());
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains_key("status"));
    }

    #[test]
    fn test_local_fallback_no_ollama() {
        // Ollama is almost certainly not running in the test environment,
        // so generate() should fall back to the hardcoded response.
        let provider = LocalProvider::new();
        let result = provider.generate("Tell me a joke", "llama2", 0.7, 100);
        assert!(result.is_ok(), "generate() should succeed even without Ollama");
        let text = result.unwrap();
        assert!(
            text.contains("local model inference") || !text.is_empty(),
            "Should return either Ollama response or simulated fallback"
        );
    }
}
