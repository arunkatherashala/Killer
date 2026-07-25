// AI Providers Module
// src/ai/providers/mod.rs
//
// Provider abstraction layer for different AI backends

pub mod local;
pub mod openai;

use crate::value::Value;
use std::collections::HashMap;
use crate::ai::ClassifyResult;

pub use local::LocalProvider;
pub use openai::OpenAIProvider;

/// Trait for AI providers
/// All backends must implement this interface
pub trait Provider: Send {
    /// Generate text
    fn generate(
        &self,
        prompt: &str,
        model: &str,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String, String>;

    /// Generate embeddings
    fn embed(&self, text: &str, model: &str) -> Result<Vec<f32>, String>;

    /// Classify text
    fn classify(
        &self,
        text: &str,
        categories: Vec<String>,
        model: &str,
    ) -> Result<ClassifyResult, String>;

    /// Extract structured data
    fn extract(
        &self,
        text: &str,
        schema: HashMap<String, String>,
        model: &str,
    ) -> Result<HashMap<String, Value>, String>;

    /// Local inference
    fn local_infer(
        &self,
        model_path: &str,
        input: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String>;

    /// Get provider name
    fn name(&self) -> &str;
}

/// Provider manager handles multiple backends
pub struct ProviderManager {
    providers: HashMap<String, Box<dyn Provider>>,
    default_provider: String,
}

impl ProviderManager {
    /// Create new provider manager
    pub fn new() -> Self {
        let mut providers: HashMap<String, Box<dyn Provider>> = HashMap::new();

        // Register default providers
        providers.insert(
            "openai".to_string(),
            Box::new(OpenAIProvider::new()) as Box<dyn Provider>,
        );
        providers.insert(
            "local".to_string(),
            Box::new(LocalProvider::new()) as Box<dyn Provider>,
        );

        ProviderManager {
            providers,
            default_provider: "openai".to_string(),
        }
    }

    /// Execute inference with selected provider
    pub fn infer(
        &self,
        provider: &str,
        prompt: &str,
        model: &str,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String, String> {
        let provider_name = if provider.is_empty() {
            &self.default_provider
        } else {
            provider
        };

        match self.providers.get(provider_name) {
            Some(p) => p.generate(prompt, model, temperature, max_tokens),
            None => Err(format!(
                "Provider '{}' not found. Available: {:?}",
                provider_name,
                self.providers.keys().collect::<Vec<_>>()
            )),
        }
    }

    /// Generate embeddings
    pub fn embed(&self, text: &str, model: &str) -> Result<Vec<f32>, String> {
        match self.providers.get(&self.default_provider) {
            Some(p) => p.embed(text, model),
            None => Err("No default provider configured".to_string()),
        }
    }

    /// Classify text
    pub fn classify(
        &self,
        text: &str,
        categories: Vec<String>,
        model: &str,
    ) -> Result<ClassifyResult, String> {
        match self.providers.get(&self.default_provider) {
            Some(p) => p.classify(text, categories, model),
            None => Err("No default provider configured".to_string()),
        }
    }

    /// Extract structured data
    pub fn extract(
        &self,
        text: &str,
        schema: HashMap<String, String>,
        model: &str,
    ) -> Result<HashMap<String, Value>, String> {
        match self.providers.get(&self.default_provider) {
            Some(p) => p.extract(text, schema, model),
            None => Err("No default provider configured".to_string()),
        }
    }

    /// Local inference
    pub fn local_infer(
        &self,
        model_path: &str,
        input: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        match self.providers.get("local") {
            Some(p) => p.local_infer(model_path, input),
            None => Err("Local provider not available".to_string()),
        }
    }

    /// Configure provider
    pub fn configure(
        &mut self,
        provider: &str,
        config: HashMap<String, Value>,
    ) -> Result<(), String> {
        if provider == "openai" {
            // Extract API key from config
            if let Some(Value::Str(_api_key)) = config.get("api_key") {
                // Update OpenAI provider with API key
                // This would require mutable trait methods
                self.default_provider = "openai".to_string();
                return Ok(());
            }
        }

        if provider == "local" {
            self.default_provider = "local".to_string();
            return Ok(());
        }

        Err(format!("Unknown provider: {}", provider))
    }

    /// Get provider configuration
    pub fn get_config(&self, provider: &str) -> Result<HashMap<String, Value>, String> {
        if provider == "openai" {
            let mut config = HashMap::new();
            config.insert(
                "name".to_string(),
                Value::Str("OpenAI".to_string()),
            );
            config.insert(
                "models".to_string(),
                Value::Str("gpt-4, gpt-3.5-turbo, gpt-4-vision".to_string()),
            );
            return Ok(config);
        }

        if provider == "local" {
            let mut config = HashMap::new();
            config.insert(
                "name".to_string(),
                Value::Str("Local ONNX".to_string()),
            );
            config.insert("status".to_string(), Value::Str("ready".to_string()));
            return Ok(config);
        }

        Err(format!("Provider '{}' not found", provider))
    }

    /// List available providers
    pub fn list_available(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

