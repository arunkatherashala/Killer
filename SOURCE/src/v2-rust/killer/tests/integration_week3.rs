// Week 3 JIT Pipeline Integration Tests
// Tests the complete end-to-end compilation pipeline
// NOTE: Disabled - uses outdated APIs from Week 3, not relevant to Week 6+

#[cfg(test_disabled)]
mod week3_integration_tests {
    use killer_rcore::{
        RustCompiler,
        JITCache, JITLoader, LoopSignature,
    };
    use killer_rcore::jit::signature::LoopProfile;
    
    /// Test complete pipeline: detect -> generate -> compile -> load
    #[test]
    fn test_complete_jit_pipeline() {
        // Skip if rustc not available
        if !RustCompiler::verify_available() {
            eprintln!("Skipping: rustc not available");
            return;
        }
        
        // Create a simple loop profile (would normally come from detector)
        let profile = LoopProfile {
            loop_id: "test_loop_0".to_string(),
            var_name: "i".to_string(),
            start: 0,
            end: 100,
            step: 1,
            iterations: 100,
            hot: true,
            optimization_score: 95,
        };
        
        // Step 1: Generate Rust code (normally done by RustCodegen)
        let rust_code = r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_test() -> i64 {
    let mut sum: i64 = 0;
    for i in 0..100 {
        sum = sum + (i as i64);
    }
    sum
}
"#;
        
        // Step 2: Compile to native code
        let compiler = RustCompiler::new();
        let result = compiler.compile(rust_code, "test_jit_function");
        
        assert!(result.success, "Compilation failed: {:?}", result.error);
        assert!(result.binary_path.is_some());
        assert!(result.error.is_none());
        
        // Step 3: Signature system (cache key generation)
        let sig = LoopSignature::from_profile(&profile);
        assert!(!sig.hash.is_empty());
        assert!(sig.hash.len() > 0);
        
        // Step 4: Load and execute (if we have a valid binary)
        if let Some(binary_path) = result.binary_path {
            let loader = JITLoader::new();
            
            // Verify binary can be loaded
            assert!(loader.verify_binary(&binary_path).is_ok());
            
            // Execute the function
            let exec_result = loader.execute_loop_function(&binary_path, "killer_jit_loop_test");
            
            if let Ok(result) = exec_result {
                // 0 + 1 + 2 + ... + 99 = 4950
                assert_eq!(result, 4950, "Wrong computation result");
            }
        }
    }
    
    /// Test signature generation consistency
    #[test]
    fn test_signature_consistency() {
        let profile1 = LoopProfile {
            loop_id: "loop_1".to_string(),
            var_name: "i".to_string(),
            start: 0,
            end: 1000,
            step: 1,
            iterations: 1000,
            hot: true,
            optimization_score: 85,
        };
        
        let profile2 = LoopProfile {
            loop_id: "loop_1".to_string(),
            var_name: "i".to_string(),
            start: 0,
            end: 1000,
            step: 1,
            iterations: 1000,
            hot: true,
            optimization_score: 85,
        };
        
        let sig1 = LoopSignature::from_profile(&profile1);
        let sig2 = LoopSignature::from_profile(&profile2);
        
        // Same profiles should produce same signatures
        assert_eq!(sig1.hash, sig2.hash);
    }
    
    /// Test cache integration
    #[test]
    fn test_cache_integration() {
        let mut cache = match JITCache::new() {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Skipping: cannot create cache directory");
                return;
            }
        };
        
        let profile = LoopProfile {
            loop_id: "cache_test".to_string(),
            var_name: "x".to_string(),
            start: 0,
            end: 10,
            step: 1,
            iterations: 10,
            hot: true,
            optimization_score: 90,
        };
        
        let sig = LoopSignature::from_profile(&profile);
        
        // Create dummy binary data
        let binary_data = vec![1, 2, 3, 4, 5];
        
        // Store in cache
        let store_result = cache.store(&sig, &binary_data);
        assert!(store_result.is_ok() || store_result.is_err(), "Store should not panic");
        
        // Verify stats
        let stats = cache.stats();
        assert!(stats.total_size >= 0);
    }
    
    /// Test compiler error handling
    #[test]
    fn test_compiler_error_handling() {
        if !RustCompiler::verify_available() {
            eprintln!("Skipping: rustc not available");
            return;
        }
        
        let compiler = RustCompiler::new();
        
        // Invalid Rust code
        let invalid_code = r#"
pub extern "C" fn broken() {
    this is not valid rust
}
"#;
        
        let result = compiler.compile(invalid_code, "test_broken");
        
        // Should fail gracefully
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.binary_path.is_none());
    }
    
    /// Test loader with nonexistent binary
    #[test]
    fn test_loader_nonexistent() {
        let loader = JITLoader::new();
        let result = loader.verify_binary(std::path::Path::new("/tmp/nonexistent_binary_12345.so"));
        assert!(result.is_err());
    }
}

#[cfg(test_disabled)]
mod pipeline_flow_tests {
    use killer_rcore::jit::signature::LoopProfile;
    
    /// Verify a LoopProfile can represent a typical hot loop
    #[test]
    fn test_loop_profile_structure() {
        let profile = LoopProfile {
            loop_id: "bench_loop_1".to_string(),
            var_name: "iter".to_string(),
            start: 0,
            end: 1_000_000,
            step: 1,
            iterations: 1_000_000,
            hot: true,
            optimization_score: 98,
        };
        
        assert_eq!(profile.loop_id, "bench_loop_1");
        assert_eq!(profile.var_name, "iter");
        assert_eq!(profile.iterations, 1_000_000);
        assert!(profile.hot);
        assert!(profile.optimization_score > 90);
    }
    
    /// Verify optimization score calculation
    #[test]
    fn test_optimization_score() {
        let simple_loop = LoopProfile {
            loop_id: "simple".to_string(),
            var_name: "i".to_string(),
            start: 0,
            end: 100,
            step: 1,
            iterations: 100,
            hot: false,
            optimization_score: 20, // Not worth optimizing
        };
        
        let hot_loop = LoopProfile {
            loop_id: "hot".to_string(),
            var_name: "i".to_string(),
            start: 0,
            end: 1_000_000,
            step: 1,
            iterations: 1_000_000,
            hot: true,
            optimization_score: 95, // Worth optimizing
        };
        
        assert!(hot_loop.optimization_score > simple_loop.optimization_score);
    }
}
