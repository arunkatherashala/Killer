#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 7 Phase A, B, C: Integration Tests
/// Tests for Production Integration, Monitoring, and Calibration

#[cfg(test)]
mod production_integration_tests {
    use killer_rcore::optimization::{
        ProductionIntegration, LoopType, LoopFeatures, OptimizationParams,
    };
    
    #[test]
    fn test_loop_detection_cpu_bound() {
        let mut integration = ProductionIntegration::new("cpu_app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.2,
            arithmetic_intensity: 0.95,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        
        // Detect CPU-bound loop
        let detected = integration.detect_loop(
            "matrix_multiply".to_string(),
            LoopType::CpuBound,
            features,
            0x4000,
            0.98,
            15.0,
        );
        
        assert_eq!(detected.loop_type, LoopType::CpuBound);
        assert_eq!(detected.confidence, 0.98);
        assert_eq!(integration.total_detected(), 1);
    }
    
    #[test]
    fn test_loop_detection_memory_bound() {
        let mut integration = ProductionIntegration::new("mem_app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.8,
            arithmetic_intensity: 0.2,
            branch_density: 0.3,
            trip_count: 5000,
            vectorizable: false,
        };
        
        let detected = integration.detect_loop(
            "array_scan".to_string(),
            LoopType::MemoryBound,
            features,
            0x2000,
            0.92,
            8.0,
        );
        
        assert_eq!(detected.loop_type, LoopType::MemoryBound);
        assert_eq!(integration.total_detected(), 1);
    }
    
    #[test]
    fn test_optimization_injection_success() {
        let mut integration = ProductionIntegration::new("test_app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        integration.detect_loop(
            "loop_1".to_string(),
            LoopType::CpuBound,
            features,
            0x1000,
            0.95,
            5.0,
        );
        
        let params = OptimizationParams::cpu_bound();
        let injection = integration.inject_optimization(
            "loop_1".to_string(),
            1000u64,
            params,
        );
        
        assert!(injection.is_ok());
        assert_eq!(integration.successfully_injected(), 1);
        
        let inj = injection.unwrap();
        assert_eq!(inj.parameters.unroll_factor, 8);  // CPU-bound has unroll=8
        assert!(inj.parameters.vectorize);
    }
    
    #[test]
    fn test_multi_loop_detection_and_injection() {
        let mut integration = ProductionIntegration::new("complex_app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        
        // Detect 3 different loops
        integration.detect_loop("loop_a".to_string(), LoopType::CpuBound, features.clone(), 0x1000, 0.95, 5.0);
        integration.detect_loop("loop_b".to_string(), LoopType::MemoryBound, features.clone(), 0x2000, 0.90, 10.0);
        integration.detect_loop("loop_c".to_string(), LoopType::Mixed, features.clone(), 0x3000, 0.88, 7.0);
        
        assert_eq!(integration.total_detected(), 3);
        
        // Inject optimizations
        let _ = integration.inject_optimization("loop_a".to_string(), 1000, OptimizationParams::cpu_bound());
        let _ = integration.inject_optimization("loop_b".to_string(), 1001, OptimizationParams::memory_bound());
        let _ = integration.inject_optimization("loop_c".to_string(), 1002, OptimizationParams::mixed());
        
        assert_eq!(integration.successfully_injected(), 3);
        assert_eq!(integration.injection_rate(), 1.0);
    }
    
    #[test]
    fn test_optimization_failure_handling() {
        let mut integration = ProductionIntegration::new("testing_app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        integration.detect_loop("loop_1".to_string(), LoopType::CpuBound, features, 0x1000, 0.95, 5.0);
        
        // Successfully inject
        let _ = integration.inject_optimization("loop_1".to_string(), 1000, OptimizationParams::cpu_bound());
        assert_eq!(integration.successfully_injected(), 1);
        
        // Mark as failed
        integration.mark_failed("loop_1");
        assert_eq!(integration.successfully_injected(), 0);
    }
    
    #[test]
    fn test_detection_rate() {
        let mut integration = ProductionIntegration::new("app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        
        // Detect some loops (increment total_detected 5 times)
        for i in 0..5 {
            integration.detect_loop(
                format!("loop_{}", i),
                LoopType::CpuBound,
                features.clone(),
                0x1000 + (i as u64 * 0x1000),
                0.90,
                5.0,
            );
        }
        
        assert_eq!(integration.total_detected(), 5);
        assert!(integration.detection_rate() >= 0.9);
    }
}

#[cfg(test)]
mod monitoring_tests {
    use killer_rcore::optimization::{PerformanceMonitor, LoopType};
    
    #[test]
    fn test_monitor_recording() {
        let mut monitor = PerformanceMonitor::new(10);
        
        monitor.record(
            1000u64,
            "app_1".to_string(),
            "loop_id".to_string(),
            LoopType::CpuBound,
            10.0,
            9.8,
            10.0,
            1.02,
        );
        
        assert_eq!(monitor.snapshots().len(), 1);
        assert!(monitor.current_accuracy(LoopType::CpuBound).is_some());
    }
    
    #[test]
    fn test_monitor_accuracy_tracking() {
        let mut monitor = PerformanceMonitor::new(10);
        
        // Record perfect predictions
        for _ in 0..3 {
            monitor.record(1000u64, "a".to_string(), "l".to_string(), LoopType::CpuBound, 10.0, 10.0, 10.0, 1.0);
        }
        
        if let Some(acc) = monitor.current_accuracy(LoopType::CpuBound) {
            assert!(acc > 0.95, "Perfect predictions should be very accurate");
        }
    }
    
    #[test]
    fn test_monitor_rolling_window() {
        let mut monitor = PerformanceMonitor::new(5);
        
        // Record 10 measurements
        for i in 0..10 {
            monitor.record(
                1000u64 + i,
                "app".to_string(),
                format!("loop_{}", i),
                LoopType::CpuBound,
                5.0 + i as f64,
                5.0 + i as f64,
                10.0,
                10.0 / (5.0 + i as f64),
            );
        }
        
        assert_eq!(monitor.snapshots().len(), 10);  // All snapshots kept
    }
    
    #[test]
    fn test_monitor_alert_thresholds() {
        let mut monitor = PerformanceMonitor::new(10);
        monitor.set_accuracy_threshold(0.75);
        monitor.set_speedup_threshold(2.0);
        
        // Record poor accuracy
        monitor.record(1000u64, "a".to_string(), "l".to_string(), LoopType::CpuBound, 10.0, 2.0, 10.0, 5.0);
        
        // Should trigger accuracy alert (large error)
        assert!(monitor.accuracy_alert(LoopType::CpuBound));
    }
    
    #[test]
    fn test_monitor_prediction_error_stats() {
        let mut monitor = PerformanceMonitor::new(10);
        
        // Record varied predictions
        monitor.record(1000u64, "a".to_string(), "1".to_string(), LoopType::CpuBound, 10.0, 10.0, 10.0, 1.0);
        monitor.record(1001u64, "a".to_string(), "2".to_string(), LoopType::CpuBound, 10.0, 12.0, 10.0, 0.83);
        monitor.record(1002u64, "a".to_string(), "3".to_string(), LoopType::CpuBound, 10.0, 8.0, 10.0, 1.25);
        
        let (min, avg, max) = monitor.prediction_error_stats();
        assert!(min >= 0.0);
        assert!(avg >= min);
        assert!(max >= avg);
    }
}

#[cfg(test)]
mod calibration_tests {
    use killer_rcore::optimization::{AccuracyCalibrator, LoopType};
    
    #[test]
    fn test_calibrator_creation() {
        let cal = AccuracyCalibrator::new(0.1);
        assert!(!cal.has_converged());
    }
    
    #[test]
    fn test_record_conservative_prediction() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Actual > Predicted means our prediction was too conservative
        cal.record(LoopType::CpuBound, 10.0, 12.0);
        
        let factor = cal.get_factor(LoopType::CpuBound).unwrap();
        assert!(factor.adjustment > 0.0, "Should increase multiplier");
        assert!(factor.effective() > 1.0, "Effective factor should be > 1.0");
    }
    
    #[test]
    fn test_record_optimistic_prediction() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Actual < Predicted means our prediction was too optimistic
        cal.record(LoopType::MemoryBound, 10.0, 7.0);
        
        let factor = cal.get_factor(LoopType::MemoryBound).unwrap();
        assert!(factor.adjustment < 0.0, "Should decrease multiplier");
        assert!(factor.effective() < 1.0, "Effective factor should be < 1.0");
    }
    
    #[test]
    fn test_convergence_with_perfect_predictions() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record perfect predictions for all loop types
        for _ in 0..5 {
            cal.record(LoopType::CpuBound, 10.0, 10.0);
            cal.record(LoopType::MemoryBound, 5.0, 5.0);
            cal.record(LoopType::Mixed, 7.0, 7.0);
        }
        
        assert!(cal.has_converged(), "Should converge with perfect predictions");
    }
    
    #[test]
    fn test_adjusted_prediction() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record that actual > predicted (conservative estimates)
        cal.record(LoopType::CpuBound, 5.0, 6.0);
        cal.record(LoopType::CpuBound, 5.0, 6.0);
        
        let adjusted = cal.adjust_prediction(LoopType::CpuBound, 5.0);
        assert!(adjusted > 5.0, "Should increase prediction for conservative type");
    }
    
    #[test]
    fn test_confidence_score_improvement() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Initial confidence
        let initial_conf = cal.confidence_score(LoopType::CpuBound);
        assert_eq!(initial_conf, 0.5);  // Default
        
        // Add measurements
        for _ in 0..5 {
            cal.record(LoopType::CpuBound, 10.0, 9.9);
        }
        
        let improved_conf = cal.confidence_score(LoopType::CpuBound);
        assert!(improved_conf > initial_conf, "Confidence should improve");
    }
    
    #[test]
    fn test_overall_accuracy() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record varied predictions
        cal.record(LoopType::CpuBound, 10.0, 10.0);
        cal.record(LoopType::CpuBound, 10.0, 10.5);
        cal.record(LoopType::CpuBound, 10.0, 9.5);
        
        let accuracy = cal.overall_accuracy();
        assert!(accuracy > 0.5, "Should have reasonable accuracy");
        assert!(accuracy <= 1.0, "Accuracy cannot exceed 100%");
    }
    
    #[test]
    fn test_calibration_limits() {
        let mut cal = AccuracyCalibrator::new(0.5);  // High learning rate
        
        // Record extreme discrepancies many times
        for _ in 0..10 {
            cal.record(LoopType::Mixed, 1.0, 100.0);
        }
        
        let factor = cal.get_factor(LoopType::Mixed).unwrap();
        assert!(factor.adjustment <= 0.5, "Adjustment should be bounded at +0.5");
        assert!(factor.adjustment >= -0.5, "Adjustment should be bounded at -0.5");
    }
}

#[cfg(test)]
mod end_to_end_integration_tests {
    use killer_rcore::optimization::{
        ProductionIntegration, PerformanceMonitor, AccuracyCalibrator,
        LoopType, LoopFeatures, OptimizationParams,
    };
    
    #[test]
    fn test_full_pipeline_detection_monitoring_calibration() {
        // Step 1: Detect loops in application
        let mut integration = ProductionIntegration::new("e2e_app".to_string());
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        
        integration.detect_loop("main_loop".to_string(), LoopType::CpuBound, features.clone(), 0x1000, 0.95, 20.0);
        integration.detect_loop("helper_loop".to_string(), LoopType::MemoryBound, features.clone(), 0x2000, 0.88, 30.0);
        
        assert_eq!(integration.total_detected(), 2);
        
        // Step 2: Inject optimizations
        let _ = integration.inject_optimization("main_loop".to_string(), 1000, OptimizationParams::cpu_bound());
        let _ = integration.inject_optimization("helper_loop".to_string(), 1001, OptimizationParams::memory_bound());
        
        assert_eq!(integration.successfully_injected(), 2);
        
        // Step 3: Monitor performance
        let mut monitor = PerformanceMonitor::new(10);
        
        // Simulate predicted speedups from Phase 5
        let predicted_speedup_cpu = 11.5;  // From earlier tests
        let predicted_speedup_mem = 3.0;
        
        // Record actual measurements
        monitor.record(1000u64, "e2e_app".to_string(), "main_loop".to_string(), 
                      LoopType::CpuBound, predicted_speedup_cpu, 11.2, 20.0, 1.79);
        monitor.record(1001u64, "e2e_app".to_string(), "helper_loop".to_string(),
                      LoopType::MemoryBound, predicted_speedup_mem, 3.5, 30.0, 8.57);
        
        assert_eq!(monitor.snapshots().len(), 2);
        
        // Step 4: Calibrate predictions
        let mut calibrator = AccuracyCalibrator::new(0.1);
        
        calibrator.record(LoopType::CpuBound, predicted_speedup_cpu, 11.2);
        calibrator.record(LoopType::MemoryBound, predicted_speedup_mem, 3.5);
        
        let accuracy = calibrator.overall_accuracy();
        assert!(accuracy > 0.7, "Should have >70% accuracy");
        
        // Verify monitoring has accuracy data
        if let Some(cpu_acc) = monitor.current_accuracy(LoopType::CpuBound) {
            assert!(cpu_acc > 0.5);  // At least moderately accurate
        }
    }
}
