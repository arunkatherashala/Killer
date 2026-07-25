// AI Utilities
// src/ai/utils.rs
//
// Common utilities for AI operations

use crate::value::Value;
use std::collections::HashMap;

/// Parse model parameters from a Killer Value
pub fn parse_model_params(params: &Value) -> HashMap<String, String> {
    let mut result = HashMap::new();

    match params {
        Value::Dict(map) => {
            for (key, val) in map.iter() {
                if let Value::Str(s) = val {
                    result.insert(key.clone(), s.clone());
                } else {
                    result.insert(key.clone(), val.to_string());
                }
            }
        }
        _ => {}
    }

    result
}

/// Validate and normalize a model name
pub fn normalize_model_name(model: &str) -> String {
    model
        .to_lowercase()
        .trim()
        .replace(" ", "-")
        .replace("_", "-")
}

/// Generate a cache key from parameters
pub fn generate_cache_key(operation: &str, params: &HashMap<String, String>) -> String {
    let mut key = format!("{}:", operation);
    let mut sorted_params: Vec<_> = params.iter().collect();
    sorted_params.sort_by_key(|(k, _)| *k);

    for (k, v) in sorted_params {
        key.push_str(&format!("{}={},", k, v));
    }

    // Simple hash for the key to keep it reasonable length
    format!("{}:{}:hash_{}", operation, key.len(), key.len())
}

/// Validate prompt for generation
pub fn validate_prompt(prompt: &str) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    if prompt.len() > 100000 {
        return Err("Prompt too long (max 100000 characters)".to_string());
    }

    Ok(())
}

/// Validate classification categories
pub fn validate_categories(categories: &[String]) -> Result<(), String> {
    if categories.is_empty() {
        return Err("At least one category is required".to_string());
    }

    if categories.len() > 100 {
        return Err("Too many categories (max 100)".to_string());
    }

    for cat in categories {
        if cat.is_empty() {
            return Err("Category cannot be empty".to_string());
        }
        if cat.len() > 100 {
            return Err("Category too long (max 100 characters)".to_string());
        }
    }

    Ok(())
}

/// Sanitize text for API calls (remove problematic characters)
pub fn sanitize_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_string()
}

/// Truncate text to max length
pub fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length - 3])
    }
}

/// Convert Value to embedding vector
pub fn value_to_embedding(value: &Value) -> Option<Vec<f32>> {
    match value {
        Value::Array(arr) => {
            let mut embedding = Vec::new();
            for item in arr {
                match item {
                    Value::Number(n) => embedding.push(n as f32),
                    _ => return None,
                }
            }
            Some(embedding)
        }
        _ => None,
    }
}

/// Convert embedding vector to Value
pub fn embedding_to_value(embedding: &[f32]) -> Value {
    let arr: Vec<Value> = embedding
        .iter()
        .map(|&f| Value::Number(f as f64))
        .collect();
    Value::from(arr)
}

/// Calculate cosine similarity between embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot_product += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denominator = (norm_a.sqrt()) * (norm_b.sqrt());
    if denominator == 0.0 {
        Some(0.0)
    } else {
        Some(dot_product / denominator)
    }
}

/// Format model response for Killer output
pub fn format_response(response: &str) -> Value {
    Value::Str(response.trim().to_string())
}

/// Format error response
pub fn format_error(error: &str) -> Value {
    Value::Dict(Box::new({
        let mut map = HashMap::new();
        map.insert("error".to_string(), Value::Str(error.to_string()));
        map.insert(
            "code".to_string(),
            Value::Str("AI_ERROR".to_string()),
        );
        map
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model_name() {
        assert_eq!(normalize_model_name("GPT-3.5"), "gpt-3.5");
        assert_eq!(normalize_model_name("BERT Base"), "bert-base");
    }

    #[test]
    fn test_validate_prompt() {
        assert!(validate_prompt("hello").is_ok());
        assert!(validate_prompt("").is_err());
    }

    #[test]
    fn test_validate_categories() {
        assert!(validate_categories(&["positive".to_string(), "negative".to_string()]).is_ok());
        assert!(validate_categories(&[]).is_err());
    }

    #[test]
    fn test_sanitize_text() {
        let text = "hello\x00world";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains('\x00'));
    }

    #[test]
    fn test_truncate_text() {
        let text = "hello world";
        assert_eq!(truncate_text(text, 5), "he...");
        assert_eq!(truncate_text(text, 20), "hello world");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), Some(1.0));

        let c = vec![1.0, 0.0];
        let d = vec![0.0, 1.0];
        assert_eq!(cosine_similarity(&c, &d), Some(0.0));
    }
}
