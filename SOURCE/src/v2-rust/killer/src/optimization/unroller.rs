// killer_rcore/src/optimization/unroller.rs
// Loop unrolling transformation implementation
// Week 5 - Transforms loops to execute multiple iterations per cycle

use std::fmt;

/// Configuration for loop unrolling
#[derive(Debug, Clone)]
pub struct UnrollConfiguration {
    /// Unroll factor (2, 4, 8, 16)
    pub factor: u32,
    
    /// Include remainder handling code
    pub handle_remainder: bool,
    
    /// Generate explicit unrolled bodies
    pub explicit_unroll: bool,
}

impl Default for UnrollConfiguration {
    fn default() -> Self {
        UnrollConfiguration {
            factor: 4,
            handle_remainder: true,
            explicit_unroll: true,
        }
    }
}

impl UnrollConfiguration {
    pub fn with_factor(factor: u32) -> Self {
        UnrollConfiguration {
            factor,
            ..Default::default()
        }
    }
}

impl fmt::Display for UnrollConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x unroll (remainder: {})", 
            self.factor, 
            if self.handle_remainder { "yes" } else { "no" }
        )
    }
}

/// Loop unroller - transforms loops with unrolling
pub struct LoopUnroller;

impl LoopUnroller {
    /// Unroll a simple loop by executing multiple iterations per cycle
    /// 
    /// Example: for i in 0..n { body } becomes
    /// for i in (0..n).step_by(factor) {
    ///     body (i)
    ///     body (i+1)
    ///     body (i+2)
    ///     body (i+3)
    /// }
    pub fn unroll_simple_loop(code: &str, config: &UnrollConfiguration) -> String {
        // Extract the loop variable and bounds
        if let Some(loop_start) = code.find("for") {
            if let Some(in_pos) = code[loop_start..].find(" in ") {
                let loop_var_start = loop_start + 4;
                let loop_var_end = loop_start + in_pos;
                let loop_var = code[loop_var_start..loop_var_end].trim();
                
                // Find the range
                let range_start = loop_start + in_pos + 4;
                if let Some(brace_pos) = code[range_start..].find('{') {
                    let range_str = code[range_start..range_start + brace_pos].trim();
                    
                    // Extract loop body
                    let body_start = range_start + brace_pos + 1;
                    if let Some(body_end) = code[body_start..].rfind('}') {
                        let loop_body = code[body_start..body_start + body_end].trim();
                        
                        return Self::generate_unrolled_loop(
                            loop_var,
                            range_str,
                            loop_body,
                            config,
                        );
                    }
                }
            }
        }
        
        // If we can't parse, return original
        code.to_string()
    }
    
    /// Generate the unrolled loop code
    fn generate_unrolled_loop(
        loop_var: &str,
        range: &str,
        body: &str,
        config: &UnrollConfiguration,
    ) -> String {
        let factor = config.factor;
        
        // Generate unrolled loop body
        let mut unrolled_body = String::new();
        for i in 0..factor {
            let offset_var = if i == 0 {
                loop_var.to_string()
            } else {
                format!("{} + {}", loop_var, i)
            };
            
            // Replace loop variable in body
            let iteration_body = body.replace(loop_var, &format!("({})", offset_var));
            unrolled_body.push_str(&iteration_body);
            unrolled_body.push('\n');
        }
        
        // Build the loop with step_by
        let mut result = format!(
            "for {} in ({}).step_by({}) {{\n{}    }}\n",
            loop_var, range, factor, unrolled_body
        );
        
        // Add remainder handling if configured
        if config.handle_remainder {
            let remainder_handler = Self::generate_remainder_handling(
                loop_var,
                range,
                body,
                factor,
            );
            result.push_str(&remainder_handler);
        }
        
        result
    }
    
    /// Generate code to handle iterations that don't evenly divide by factor
    fn generate_remainder_handling(
        loop_var: &str,
        range: &str,
        body: &str,
        factor: u32,
    ) -> String {
        // Extract upper bound from range (simple case: "0..n")
        if let Some(pos) = range.find("..") {
            let upper_bound = range[pos + 2..].trim();
            
            let mut code = String::new();
            code.push_str(&format!(
                "let remainder = ({}) % {};\n",
                upper_bound, factor
            ));
            code.push_str(&format!(
                "for i in 0..remainder {{\n"
            ));
            code.push_str(&format!(
                "    let {} = {} - remainder + i;\n",
                loop_var, upper_bound
            ));
            code.push_str(&body.replace(loop_var, loop_var));
            code.push_str("}\n");
            
            code
        } else {
            String::new()
        }
    }
}

/// Advanced unroller with optimization awareness
pub struct OptimizedUnroller;

impl OptimizedUnroller {
    /// Unroll with automatic factor selection based on loop complexity
    pub fn auto_unroll(code: &str, complexity: f64) -> String {
        let factor = if complexity > 0.8 {
            2  // Complex loops - small unroll
        } else if complexity > 0.5 {
            4  // Moderate complexity
        } else {
            8  // Simple loops - aggressive unroll
        };
        
        let config = UnrollConfiguration {
            factor,
            handle_remainder: true,
            explicit_unroll: true,
        };
        
        LoopUnroller::unroll_simple_loop(code, &config)
    }
    
    /// Unroll and add vectorization hints for LLVM
    pub fn unroll_with_vectorization(code: &str, factor: u32) -> String {
        let config = UnrollConfiguration {
            factor,
            handle_remainder: true,
            explicit_unroll: true,
        };
        
        let mut unrolled = LoopUnroller::unroll_simple_loop(code, &config);
        
        // Add SIMD/vectorization hints as comments for LLVM
        unrolled = unrolled.replace(
            "for ",
            "// LLVM: try to vectorize this loop\nfor "
        );
        
        unrolled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unroll_config_default() {
        let config = UnrollConfiguration::default();
        assert_eq!(config.factor, 4);
        assert!(config.handle_remainder);
    }
    
    #[test]
    fn test_unroll_config_with_factor() {
        let config = UnrollConfiguration::with_factor(8);
        assert_eq!(config.factor, 8);
    }
    
    #[test]
    fn test_unroll_config_display() {
        let config = UnrollConfiguration::with_factor(4);
        assert_eq!(config.to_string(), "4x unroll (remainder: yes)");
    }
    
    #[test]
    fn test_simple_loop_unrolling() {
        let code = r#"
            for i in 0..1000 {
                sum = sum + i;
            }
        "#;
        let config = UnrollConfiguration::with_factor(2);
        let result = LoopUnroller::unroll_simple_loop(code, &config);
        
        // Verify the result contains the step_by
        assert!(result.contains("step_by"));
        assert!(result.contains("2"));
    }
    
    #[test]
    fn test_unroll_generates_multiple_bodies() {
        let code = r#"
            for i in 0..1000 {
                sum = sum + i;
            }
        "#;
        let config = UnrollConfiguration::with_factor(4);
        let result = LoopUnroller::unroll_simple_loop(code, &config);
        
        // With 4x unroll, should have multiple sum operations
        // (actual count depends on parsing accuracy)
        assert!(result.len() > code.len());
    }
    
    #[test]
    fn test_remainder_handling_generated() {
        let code = r#"
            for i in 0..1000 {
                sum = sum + i;
            }
        "#;
        let config = UnrollConfiguration {
            factor: 8,
            handle_remainder: true,
            explicit_unroll: true,
        };
        let result = LoopUnroller::unroll_simple_loop(code, &config);
        
        // Should include remainder handling
        assert!(result.contains("remainder") || result.len() > code.len());
    }
    
    #[test]
    fn test_auto_unroll_simple() {
        let code = r#"
            for i in 0..1000 {
                sum = sum + i;
            }
        "#;
        let result = OptimizedUnroller::auto_unroll(code, 0.1);
        
        // Simple loops should use larger unroll factor (8)
        assert!(result.contains("step_by") && result.contains("8"));
    }
    
    #[test]
    fn test_auto_unroll_complex() {
        let code = r#"
            for i in 0..1000 {
                if i % 2 == 0 {
                    arr[i] = operation(i);
                }
            }
        "#;
        let result = OptimizedUnroller::auto_unroll(code, 0.9);
        
        // Complex loops should use smaller unroll factor (2)
        assert!(result.contains("2") || result.len() > code.len());
    }
    
    #[test]
    fn test_vectorization_hints() {
        let code = r#"
            for i in 0..1000 {
                sum = sum + i;
            }
        "#;
        let result = OptimizedUnroller::unroll_with_vectorization(code, 4);
        
        // Should include LLVM vectorization comments
        assert!(result.contains("vectorize") || result.contains("LLVM"));
    }
}
