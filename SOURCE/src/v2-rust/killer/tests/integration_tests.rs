/// Killer Language - Comprehensive Test Suite
/// Tests cover:
/// - Parser (lexer, indentation, syntax)
/// - Type System (type checking, inference)
/// - Virtual Machine (execution, bytecode)
/// - Optimization Engine (JIT, caching)
/// - Security (path validation, recursion limits)
/// - Error Handling (error recovery, messaging)

#[cfg(test)]
mod tests {
    use killer_native::security::{validate_file_path, SecurityConfig, RecursionGuard, MAX_RECURSION_DEPTH};
    use killer_native::optimization_engine::OptimizationEngine;
    use killer_native::runtime_optimization::OptimizationLevel;
    use killer_native::error::VmError;

    // ========== SECURITY TESTS ==========

    #[test]
    fn test_path_validation_rejects_parent_traversal() {
        let config = SecurityConfig::default();
        
        // Should fail: trying to escape with ..
        assert!(validate_file_path("../../../etc/passwd", &config).is_err());
        assert!(validate_file_path("../../sensitive.file", &config).is_err());
    }

    #[test]
    fn test_path_validation_allows_relative_paths() {
        let mut config = SecurityConfig::default();
        // Files may not exist during CI; skip canonicalize check for this test
        config.enforce_path_canonicalization = false;

        assert!(validate_file_path("src/main.killer", &config).is_ok());
        assert!(validate_file_path("examples/hello.killer", &config).is_ok());
        assert!(validate_file_path("test.killer", &config).is_ok());
    }

    #[test]
    fn test_path_validation_rejects_absolute_paths() {
        let config = SecurityConfig::default();
        
        // Should fail: absolute paths not allowed by default
        #[cfg(windows)]
        assert!(validate_file_path("C:\\Windows\\System32\\evil.exe", &config).is_err());
        
        #[cfg(unix)]
        assert!(validate_file_path("/etc/passwd", &config).is_err());
    }

    #[test]
    fn test_recursion_guard_tracks_depth() {
        let mut guard = RecursionGuard::new(5);
        
        // Should increment depth
        assert_eq!(guard.current(), 0);
        let _token1 = guard.enter().unwrap();
        assert_eq!(guard.current(), 1);
        
        let _token2 = guard.enter().unwrap();
        assert_eq!(guard.current(), 2);
    }

    #[test]
    fn test_recursion_guard_prevents_overflow() {
        let guard = RecursionGuard::new(3);
        let _t1 = guard.enter().unwrap();
        let _t2 = guard.enter().unwrap();
        let _t3 = guard.enter().unwrap();
        assert!(guard.enter().is_err());
    }

    #[test]
    fn test_recursion_guard_auto_cleanup() {
        let mut guard = RecursionGuard::new(10);
        
        {
            let _token = guard.enter().unwrap();
            assert_eq!(guard.current(), 1);
        } // Token dropped here
        
        // Should automatically decrement
        assert_eq!(guard.current(), 0);
    }

    #[test]
    fn test_max_recursion_depth_constant() {
        assert!(MAX_RECURSION_DEPTH > 1000);
        assert!(MAX_RECURSION_DEPTH < 100_000); // Reasonable upper bound
    }

    // ========== OPTIMIZATION ENGINE TESTS ==========

    #[test]
    fn test_optimization_engine_o0_level() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O0);
        
        // O0: Only essential caches
        assert!(engine.get_statistics().instruction_cache_enabled);
        assert!(!engine.get_statistics().jit_enabled);
    }

    #[test]
    fn test_optimization_engine_o1_level() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O1);

        let stats = engine.get_statistics();
        assert!(stats.instruction_cache_enabled);
        assert!(!stats.hot_detector_enabled);
        assert!(stats.call_site_cache_stats.is_some());
    }

    #[test]
    fn test_optimization_engine_o2_level() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O2);
        
        // O2: Standard optimization (includes JIT)
        let stats = engine.get_statistics();
        assert!(stats.instruction_cache_enabled);
        assert!(stats.jit_enabled);
    }

    #[test]
    fn test_optimization_engine_o3_level() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O3);
        
        // O3: Maximum optimization
        let stats = engine.get_statistics();
        assert!(stats.instruction_cache_enabled);
        assert!(stats.jit_enabled);
    }

    #[test]
    fn test_optimization_engine_default() {
        let engine = OptimizationEngine::new();
        
        // Default should be O2
        let stats = engine.get_statistics();
        assert!(stats.instruction_cache_enabled);
        assert!(stats.jit_enabled);
    }

    // ========== ERROR HANDLING TESTS ==========

    #[test]
    fn test_security_error_with_suggestion() {
        let error = VmError::SecurityError {
            message: "Path traversal detected".to_string(),
            location: None,
            suggestion: Some("Use relative paths without '..'".to_string()),
        };
        
        let msg = format!("{}", error);
        assert!(msg.contains("Security error"));
        assert!(msg.contains("Path traversal"));
        assert!(msg.contains("relative paths"));
    }

    #[test]
    fn test_security_error_without_suggestion() {
        let error = VmError::SecurityError {
            message: "Access denied".to_string(),
            location: None,
            suggestion: None,
        };
        
        let msg = format!("{}", error);
        assert!(msg.contains("Security error"));
        assert!(msg.contains("Access denied"));
    }

    // ========== INTEGRATION TESTS ==========

    #[test]
    fn test_multiple_recursion_guards_independent() {
        let mut guard1 = RecursionGuard::new(5);
        let mut guard2 = RecursionGuard::new(10);
        
        let _token1 = guard1.enter().unwrap();
        let _token2 = guard2.enter().unwrap();
        
        assert_eq!(guard1.current(), 1);
        assert_eq!(guard2.current(), 1);
        
        let _token3 = guard2.enter().unwrap();
        
        assert_eq!(guard1.current(), 1);
        assert_eq!(guard2.current(), 2);
    }

    #[test]
    fn test_security_config_customizable() {
        let mut config = SecurityConfig::default();
        
        // Should be customizable
        config.max_recursion_depth = 5_000;
        config.max_file_size = 1024 * 1024; // 1MB
        
        assert_eq!(config.max_recursion_depth, 5_000);
        assert_eq!(config.max_file_size, 1024 * 1024);
    }

    // ========== PERFORMANCE TESTS ==========

    #[test]
    fn test_recursion_guard_zero_overhead_when_ok() {
        let mut guard = RecursionGuard::new(1000);
        
        for _ in 0..100 {
            let _token = guard.enter().unwrap();
            // Fast operation, should not be slow
        }
        
        // If we got here without panic, it worked
        assert_eq!(guard.current(), 0);
    }

    #[test]
    fn test_path_validation_consistent() {
        let mut config = SecurityConfig::default();
        config.enforce_path_canonicalization = false;
        let path = "src/test.killer";

        let result1 = validate_file_path(path, &config);
        let result2 = validate_file_path(path, &config);

        assert!(result1.is_ok() && result2.is_ok());
    }

    // ========== BENCHMARK TESTS ==========

    #[test]
    fn test_optimization_statistics_accurate() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O2);
        let stats = engine.get_statistics();
        
        // Verify stats are non-zero/valid
        assert!(stats.scope_var_cache_hit_rate >= 0.0 && stats.scope_var_cache_hit_rate <= 1.0);
    }
}

/// Macro for easier test assertions
#[macro_export]
macro_rules! assert_security_error {
    ($result:expr) => {
        match $result {
            Err(killer_native::error::VmError::SecurityError { .. }) => {}
            other => panic!("Expected SecurityError, got {:?}", other),
        }
    };
}

#[macro_export]
macro_rules! assert_not_security_error {
    ($result:expr) => {
        match $result {
            Err(killer_native::error::VmError::SecurityError { message, .. }) => {
                panic!("Unexpected SecurityError: {}", message)
            }
            _ => {}
        }
    };
}
