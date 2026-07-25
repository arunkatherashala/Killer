// Phase 3.2: Type-Specialized Code Generator
// Generates optimized code paths for specific types
// Eliminates polymorphism overhead through monomorphization

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SpecializedCodePath {
    pub function_name: String,
    pub type_signature: String, // e.g., "add_i64_i64" for add(i64, i64)
    pub estimated_speedup: f32,
    pub code_size_bytes: usize,
    pub generated: bool,
}

#[derive(Debug)]
pub struct CodeGenerationConfig {
    /// Maximum code size per function (default 10KB)
    pub max_code_size: usize,
    /// Minimum speedup to generate specialization (default 1.5x)
    pub min_speedup_threshold: f32,
    /// Maximum specializations per function (default 5)
    pub max_specializations: usize,
}

impl Default for CodeGenerationConfig {
    fn default() -> Self {
        CodeGenerationConfig {
            max_code_size: 10 * 1024,        // 10KB
            min_speedup_threshold: 1.5,
            max_specializations: 5,
        }
    }
}

#[derive(Debug)]
pub struct TypeSpecializedCodeGenerator {
    /// Generated specializations per function
    specializations: HashMap<String, Vec<SpecializedCodePath>>,
    /// Configuration
    config: CodeGenerationConfig,
    /// Total code generated
    total_code_generated: usize,
    /// Total specializations created
    total_specializations: usize,
}

impl TypeSpecializedCodeGenerator {
    pub fn new(config: CodeGenerationConfig) -> Self {
        TypeSpecializedCodeGenerator {
            specializations: HashMap::new(),
            config,
            total_code_generated: 0,
            total_specializations: 0,
        }
    }

    /// Generate specialized code path for type combination
    pub fn generate_specialization(
        &mut self,
        function_name: String,
        type_signature: String,
        estimated_speedup: f32,
        estimated_code_size: usize,
    ) -> bool {
        // Check speedup threshold
        if estimated_speedup < self.config.min_speedup_threshold {
            return false;
        }

        // Check code size limit
        if estimated_code_size > self.config.max_code_size {
            return false;
        }

        // Check max specializations per function
        let current_specs = self
            .specializations
            .entry(function_name.clone())
            .or_insert_with(Vec::new);

        if current_specs.len() >= self.config.max_specializations {
            return false;
        }

        // Create specialization
        let spec = SpecializedCodePath {
            function_name,
            type_signature,
            estimated_speedup,
            code_size_bytes: estimated_code_size,
            generated: true,
        };

        current_specs.push(spec);
        self.total_code_generated += estimated_code_size;
        self.total_specializations += 1;

        true
    }

    /// Get all specializations for function
    pub fn get_specializations(&self, function_name: &str) -> Vec<SpecializedCodePath> {
        self.specializations
            .get(function_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if specialization exists for type signature
    pub fn has_specialization(&self, function_name: &str, type_sig: &str) -> bool {
        self.specializations
            .get(function_name)
            .map(|specs| specs.iter().any(|s| s.type_signature == type_sig))
            .unwrap_or(false)
    }

    /// Get estimated total speedup from all specializations
    pub fn get_estimated_total_speedup(&self, function_name: &str) -> f32 {
        let specs = self.get_specializations(function_name);
        if specs.is_empty() {
            return 1.0;
        }

        // Average speedup across specializations
        let sum: f32 = specs.iter().map(|s| s.estimated_speedup).sum();
        sum / specs.len() as f32
    }

    /// Get generation statistics
    pub fn get_statistics(&self) -> CodeGenerationStats {
        CodeGenerationStats {
            total_specializations: self.total_specializations,
            total_code_generated: self.total_code_generated,
            functions_specialized: self.specializations.len(),
            max_code_size: self.config.max_code_size,
            current_memory_used: self.total_code_generated,
            utilization_percent: if self.config.max_code_size * self.specializations.len() > 0 {
                (self.total_code_generated * 100)
                    / (self.config.max_code_size * self.specializations.len())
            } else {
                0
            },
        }
    }

    /// Clear all specializations
    pub fn clear(&mut self) {
        self.specializations.clear();
        self.total_code_generated = 0;
        self.total_specializations = 0;
    }
}

#[derive(Debug, Clone)]
pub struct CodeGenerationStats {
    pub total_specializations: usize,
    pub total_code_generated: usize,
    pub functions_specialized: usize,
    pub max_code_size: usize,
    pub current_memory_used: usize,
    pub utilization_percent: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        let generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());
        assert_eq!(generator.total_specializations, 0);
    }

    #[test]
    fn test_generate_specialization() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        let generated = generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            2.0,
            1024,
        );
        assert!(generated);
        assert_eq!(generator.total_specializations, 1);
    }

    #[test]
    fn test_speedup_threshold() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        // Below threshold
        let generated = generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            1.2, // Below default 1.5x threshold
            1024,
        );
        assert!(!generated);
    }

    #[test]
    fn test_code_size_limit() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig {
            max_code_size: 512,
            ..Default::default()
        });

        let generated = generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            2.0,
            2048, // Exceeds 512 limit
        );
        assert!(!generated);
    }

    #[test]
    fn test_max_specializations() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig {
            max_specializations: 2,
            ..Default::default()
        });

        // Add 2 specializations
        generator.generate_specialization("add".to_string(), "add_i64_i64".to_string(), 2.0, 500);
        generator.generate_specialization("add".to_string(), "add_f64_f64".to_string(), 2.0, 500);

        // Third should fail
        let generated = generator.generate_specialization(
            "add".to_string(),
            "add_string_string".to_string(),
            2.0,
            500,
        );
        assert!(!generated);
    }

    #[test]
    fn test_has_specialization() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            2.0,
            1024,
        );

        assert!(generator.has_specialization("add", "add_i64_i64"));
        assert!(!generator.has_specialization("add", "add_f64_f64"));
    }

    #[test]
    fn test_get_specializations() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            2.0,
            1024,
        );
        generator.generate_specialization(
            "add".to_string(),
            "add_f64_f64".to_string(),
            1.8,
            1024,
        );

        let specs = generator.get_specializations("add");
        assert_eq!(specs.len(), 2);
    }

    #[test]
    fn test_total_speedup_calculation() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        generator.generate_specialization("add".to_string(), "add_i64_i64".to_string(), 2.0, 1024);
        generator.generate_specialization("add".to_string(), "add_f64_f64".to_string(), 1.8, 1024);

        let speedup = generator.get_estimated_total_speedup("add");
        assert!((speedup - 1.9).abs() < 0.01); // Average of 2.0 and 1.8
    }

    #[test]
    fn test_statistics() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        generator.generate_specialization(
            "add".to_string(),
            "add_i64_i64".to_string(),
            2.0,
            2048,
        );
        generator.generate_specialization("mul".to_string(), "mul_i64_i64".to_string(), 2.5, 2048);

        let stats = generator.get_statistics();
        assert_eq!(stats.total_specializations, 2);
        assert_eq!(stats.functions_specialized, 2);
        assert_eq!(stats.total_code_generated, 4096);
    }

    #[test]
    fn test_clear() {
        let mut generator = TypeSpecializedCodeGenerator::new(CodeGenerationConfig::default());

        generator.generate_specialization("add".to_string(), "add_i64_i64".to_string(), 2.0, 1024);
        assert_eq!(generator.total_specializations, 1);

        generator.clear();
        assert_eq!(generator.total_specializations, 0);
    }
}
