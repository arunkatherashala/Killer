#![cfg(feature = "legacy-killer-rcore-tests")]
/// Phase 8C Integration Tests
/// Comprehensive testing of scaling orchestration, batch optimization, and incremental injection
/// Tests real-world large-scale optimization scenarios

#[cfg(test)]
mod phase_c_integration_tests {
    use killer_rcore::optimization::{
        ScalingStudyOrchestrator, BinarySize, LoopCategory,
        BatchLoopOptimizer,
        IncrementalInjectionFramework, InjectionSchedule,
    };

    /// Test scaling orchestrator with small binary
    #[test]
    fn test_scaling_orchestrator_small_binary() {
        let mut orchestrator = ScalingStudyOrchestrator::new();

        let result = orchestrator.run_study(BinarySize::Small, LoopCategory::Tiny);

        assert_eq!(result.binary_size, BinarySize::Small);
        assert_eq!(result.loop_category, LoopCategory::Tiny);
        assert!(result.optimization_time_sec > 0.0);
        assert!(result.average_speedup > 4.0);
        assert!(result.success_rate > 0.95);
    }

    /// Test scaling orchestrator with large binary
    #[test]
    fn test_scaling_orchestrator_large_binary() {
        let mut orchestrator = ScalingStudyOrchestrator::new();

        let result = orchestrator.run_study(BinarySize::Large, LoopCategory::Large);

        assert_eq!(result.binary_size, BinarySize::Large);
        assert_eq!(result.loop_category, LoopCategory::Large);
        assert!(result.optimization_time_sec > 10.0);
        assert!(result.average_speedup > 4.0);
        assert!(result.throughput > 0.0);
    }

    /// Test scaling orchestrator performance targets
    #[test]
    fn test_scaling_orchestrator_meets_targets() {
        let mut orchestrator = ScalingStudyOrchestrator::new();

        let result = orchestrator.run_study(BinarySize::Medium, LoopCategory::Medium);

        assert!(result.meets_targets());
    }

    /// Test scaling orchestrator multi-test suite
    #[test]
    fn test_scaling_orchestrator_multi_test_suite() {
        let mut orchestrator = ScalingStudyOrchestrator::new();

        // Test small-to-large progression
        orchestrator.run_study(BinarySize::Small, LoopCategory::Tiny);
        orchestrator.run_study(BinarySize::Medium, LoopCategory::Small);
        orchestrator.run_study(BinarySize::Large, LoopCategory::Medium);
        orchestrator.run_study(BinarySize::Huge, LoopCategory::Large);

        assert_eq!(orchestrator.total_tests_completed, 4);
        assert!(orchestrator.pass_rate() > 0.7);
        assert!(orchestrator.average_speedup() > 4.0);
    }

    /// Test scaling linearity
    #[test]
    fn test_scaling_orchestrator_linearity() {
        let mut orchestrator = ScalingStudyOrchestrator::new();

        orchestrator.run_study(BinarySize::Small, LoopCategory::Tiny);
        orchestrator.run_study(BinarySize::Large, LoopCategory::Huge);

        let linearity = orchestrator.scaling_linearity();
        // Scaling should be sublinear (< 1.0 would be better than linear)
        // But in our simulation it's generally around 0.5-1.5
        assert!(linearity > 0.3 && linearity < 2.0);
    }

    /// Test batch optimizer single loop
    #[test]
    fn test_batch_optimizer_single_loop() {
        let mut optimizer = BatchLoopOptimizer::new();

        let result = optimizer.optimize_loop("matrix_mult", 256);

        assert_eq!(result.status, killer_rcore::optimization::OptimizationStatus::Success);
        assert!(result.predicted_speedup > 3.5);
        assert!(result.ga_generations > 0);
    }

    /// Test batch optimizer with many loops
    #[test]
    fn test_batch_optimizer_many_loops() {
        let mut optimizer = BatchLoopOptimizer::new();

        let loop_ids: Vec<&str> = (0..100).map(|_| "loop").collect();
        let results = optimizer.optimize_batch(&loop_ids);

        assert_eq!(results.len(), 100);
        assert_eq!(optimizer.total_loops_optimized(), 100);
        assert!(optimizer.average_speedup() > 3.5);
    }

    /// Test batch optimizer throughput
    #[test]
    fn test_batch_optimizer_throughput_scaling() {
        let mut optimizer = BatchLoopOptimizer::new();

        let loop_ids: Vec<&str> = (0..50).map(|_| "loop").collect();
        let _results = optimizer.optimize_batch(&loop_ids);

        let throughput = optimizer.throughput_loops_per_sec();
        // Should be very fast in simulation -> high throughput
        assert!(throughput > 100.0);
    }

    /// Test batch optimizer parallelization benefit
    #[test]
    fn test_batch_optimizer_parallelization() {
        let mut serial = BatchLoopOptimizer::new();
        serial.set_parallel(false);
        serial.set_worker_count(1);

        let mut parallel = BatchLoopOptimizer::new();
        parallel.set_parallel(true);
        parallel.set_worker_count(4);

        let loop_ids: Vec<&str> = (0..30).map(|_| "loop").collect();

        let _serial_results = serial.optimize_batch(&loop_ids);
        let _parallel_results = parallel.optimize_batch(&loop_ids);

        // Both should complete but parallel should have speedup benefit
        let parallel_speedup = parallel.parallelization_speedup();
        assert!(parallel_speedup > 2.0 && parallel_speedup <= 4.0);
    }

    /// Test batch optimizer GA convergence logarithmic scaling
    #[test]
    fn test_batch_optimizer_ga_logarithmic_scaling() {
        let mut opt_small = BatchLoopOptimizer::new();
        opt_small.optimize_loop("small", 100);
        let small_gens = opt_small.results[0].ga_generations;

        let mut opt_large = BatchLoopOptimizer::new();
        opt_large.optimize_loop("large", 1000);
        let large_gens = opt_large.results[0].ga_generations;

        // 10x loop count should result in < 2x generations (logarithmic)
        let gen_ratio = large_gens as f64 / small_gens as f64;
        assert!(gen_ratio < 2.5);
    }

    /// Test incremental injection scheduling
    #[test]
    fn test_incremental_injection_immediate_schedule() {
        let mut framework = IncrementalInjectionFramework::new();

        let event = framework.schedule_injection(
            "loop_1",
            "unroll_factor",
            4.0,
            8.0,
            InjectionSchedule::Immediate,
        );

        assert_eq!(event.schedule, InjectionSchedule::Immediate);
        assert_eq!(framework.pending_count, 1);
    }

    /// Test incremental injection execution
    #[test]
    fn test_incremental_injection_execution() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "unroll", 4.0, 8.0, InjectionSchedule::Immediate);
        let result = framework.execute_injection("loop_1", "unroll");

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.state, killer_rcore::optimization::InjectionState::Complete);
        assert!(event.speedup_achieved > 1.0);
    }

    /// Test incremental injection multiple loops
    #[test]
    fn test_incremental_injection_multiple_loops() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "unroll", 4.0, 8.0, InjectionSchedule::Immediate);
        framework.schedule_injection("loop_2", "block", 64.0, 128.0, InjectionSchedule::Immediate);
        framework.schedule_injection("loop_3", "fusion", 1.0, 2.0, InjectionSchedule::Scheduled);

        assert_eq!(framework.pending_count, 3);

        let _ = framework.execute_injection("loop_1", "unroll");
        let _ = framework.execute_injection("loop_2", "block");

        assert_eq!(framework.pending_count, 1);
        assert_eq!(framework.successful_count, 2);
    }

    /// Test incremental injection rollback
    #[test]
    fn test_incremental_injection_rollback() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "param", 1.0, 2.0, InjectionSchedule::Immediate);
        let _ = framework.execute_injection("loop_1", "param");

        assert_eq!(framework.successful_count, 1);

        let rollback = framework.rollback_injection("loop_1", "param");
        assert!(rollback.is_ok());
        assert_eq!(framework.successful_count, 0);
    }

    /// Test incremental injection success metrics
    #[test]
    fn test_incremental_injection_success_rate() {
        let mut framework = IncrementalInjectionFramework::new();

        for i in 0..10 {
            framework.schedule_injection(
                &format!("loop_{}", i),
                "param",
                1.0,
                2.0,
                InjectionSchedule::Immediate,
            );
        }

        for i in 0..8 {
            let _ = framework.execute_injection(&format!("loop_{}", i), "param");
        }

        let success_rate = framework.success_rate();
        assert!(success_rate > 0.7 && success_rate < 0.85);
    }

    /// Test Phase C complete pipeline: scaling + batch + injection
    #[test]
    fn test_phase_c_complete_pipeline() {
        // Stage 1: Run scaling studies
        let mut scaling = ScalingStudyOrchestrator::new();

        scaling.run_study(BinarySize::Small, LoopCategory::Small);
        scaling.run_study(BinarySize::Medium, LoopCategory::Medium);
        scaling.run_study(BinarySize::Large, LoopCategory::Large);

        assert_eq!(scaling.total_tests_completed, 3);
        assert!(scaling.average_speedup() > 4.0);

        // Stage 2: Batch optimize loops
        let mut batch = BatchLoopOptimizer::new();
        batch.set_worker_count(4);

        let loop_ids: Vec<&str> = (0..50).map(|_| "loop").collect();
        let results = batch.optimize_batch(&loop_ids);

        assert_eq!(results.len(), 50);
        assert!(batch.average_speedup() > 3.5);
        assert!(batch.throughput_loops_per_sec() > 100.0);

        // Stage 3: Incremental injection of optimized parameters
        let mut injection = IncrementalInjectionFramework::new();

        for (i, result) in results.iter().enumerate().take(30) {
            injection.schedule_injection(
                &format!("loop_{}", i),
                "optimized_param",
                1.0,
                result.predicted_speedup,
                InjectionSchedule::LazyOnDemand,
            );
        }

        // Execute injections
        for i in 0..15 {
            let _ = injection.execute_injection(&format!("loop_{}", i), "optimized_param");
        }

        assert_eq!(injection.successful_count, 15);
        assert!(injection.average_speedup() > 1.5);

        // Phase C Success Criteria
        assert!(scaling.pass_rate() > 0.7);           // 70%+ scaling tests pass
        assert!(batch.average_speedup() > 4.0);        // 4x+ speedup
        assert!(injection.success_rate() > 0.4);       // 40%+ injection success
    }

    /// Test large-scale scenario: 500 loops with dynamic scheduling
    #[test]
    fn test_large_scale_500_loops() {
        let mut batch = BatchLoopOptimizer::new();
        batch.set_worker_count(8); // Use more workers for large scale

        let loop_ids: Vec<&str> = (0..500)
            .map(|i| {
                Box::leak(format!("loop_{}", i).into_boxed_str()) as &str
            })
            .collect();

        let results = batch.optimize_batch(&loop_ids);

        assert_eq!(results.len(), 500);
        assert!(batch.average_speedup() > 3.8);
        assert!(batch.throughput_loops_per_sec() > 50.0);
        assert!(batch.parallelization_speedup() > 2.0);
    }

    /// Test extreme scale: 1000 loops
    #[test]
    fn test_extreme_scale_1000_loops() {
        let mut scaling = ScalingStudyOrchestrator::new();

        // Run with Huge binary and Huge loop count
        let result = scaling.run_study(BinarySize::Huge, LoopCategory::Huge);

        assert!(result.actual_loop_count > 1000);
        assert!(result.optimization_time_sec > 30.0);
        assert!(result.average_speedup > 4.0);
        assert!(result.success_rate > 0.95);
    }

    /// Test injection overhead analysis
    #[test]
    fn test_injection_overhead_metrics() {
        let mut framework = IncrementalInjectionFramework::new();

        // Schedule and execute 20 injections
        for i in 0..20 {
            framework.schedule_injection(
                &format!("loop_{}", i),
                "param",
                1.0,
                1.5 + (i as f64 * 0.01),
                InjectionSchedule::Immediate,
            );
        }

        for i in 0..20 {
            let _ = framework.execute_injection(&format!("loop_{}", i), "param");
        }

        let avg_overhead = framework.average_injection_overhead_ms();
        assert!(avg_overhead > 0.0 && avg_overhead < 10.0);
        assert!(framework.successful_count == 20);
    }
}
