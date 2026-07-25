#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 8 Phase A: Pilot Deployment Integration Tests
/// 
/// Tests the complete deployment orchestration of 3 production applications

#[cfg(test)]
mod pilot_deployment_tests {
    use killer_rcore::optimization::{
        PilotDeployment, DeploymentTarget, DeploymentStage, LoopType,
    };
    
    #[test]
    fn test_web_server_deployment_success() {
        // Web Server: CPU-bound, high impact application
        let targets = vec![
            DeploymentTarget::new("web_server".to_string(), LoopType::CpuBound, 50.0, 5.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Phase 1: Profiling
        assert!(pilot.start_profiling("web_server").is_ok());
        assert_eq!(pilot.get_stage("web_server"), Some(DeploymentStage::Profiling));
        
        assert!(pilot.complete_profiling("web_server", 49.2).is_ok());
        assert_eq!(pilot.get_stage("web_server"), Some(DeploymentStage::Discovery));
        
        // Phase 2: Parameter Discovery (simulated with target 5.0x)
        let predicted_speedup = 5.0;
        assert!(pilot.start_canary("web_server", predicted_speedup).is_ok());
        
        // Phase 3: Canary Deployment (5-10% traffic)
        // Actual result: 4.8x (conservative estimate = 5.0x, 80% threshold = 4.0x)
        let canary_speedup = 4.8;
        let canary_passed = pilot.validate_canary("web_server", canary_speedup, 0.96).unwrap();
        assert!(canary_passed, "Canary should pass: {:.2}x >= {:.2}x threshold", 
               canary_speedup, predicted_speedup * 0.8);
        
        // Phase 4: Beta Deployment (50% traffic)
        assert!(pilot.start_beta("web_server").is_ok());
        assert_eq!(pilot.get_stage("web_server"), Some(DeploymentStage::Beta));
        
        // Phase 5: General Availability
        assert!(pilot.transition_to_ga("web_server", 4.8).is_ok());
        assert_eq!(pilot.get_stage("web_server"), Some(DeploymentStage::GeneralAvailability));
        assert_eq!(pilot.successful_count(), 1);
        
        // Verify metrics
        assert_eq!(pilot.app_count(), 1);
        let report = pilot.status_report();
        assert!(report.contains("web_server"));
        assert!(report.contains("GeneralAvailability"));
    }
    
    #[test]
    fn test_data_pipeline_deployment_success() {
        // Data Pipeline: Memory-bound, medium impact
        let targets = vec![
            DeploymentTarget::new("data_pipeline".to_string(), LoopType::MemoryBound, 120.0, 2.5),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Profiling
        assert!(pilot.start_profiling("data_pipeline").is_ok());
        assert!(pilot.complete_profiling("data_pipeline", 118.5).is_ok());
        
        // Parameter discovery -> Canary
        let predicted = 2.5;
        assert!(pilot.start_canary("data_pipeline", predicted).is_ok());
        
        // Canary: 2.3x (target 2.5x, 80% threshold = 2.0x)
        let actual = 2.3;
        let passed = pilot.validate_canary("data_pipeline", actual, 0.92).unwrap();
        assert!(passed, "Canary should pass: {:.2}x >= {:.2}x", actual, predicted * 0.8);
        
        // Beta & GA
        assert!(pilot.start_beta("data_pipeline").is_ok());
        assert!(pilot.transition_to_ga("data_pipeline", 2.3).is_ok());
        
        assert_eq!(pilot.successful_count(), 1);
    }
    
    #[test]
    fn test_analytics_engine_deployment_success() {
        // Analytics: Mixed workload, complex optimization
        let targets = vec![
            DeploymentTarget::new("analytics_engine".to_string(), LoopType::Mixed, 60.0, 3.5),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Full progression
        assert!(pilot.start_profiling("analytics_engine").is_ok());
        assert!(pilot.complete_profiling("analytics_engine", 59.0).is_ok());
        
        let predicted = 3.5;
        assert!(pilot.start_canary("analytics_engine", predicted).is_ok());
        
        // Canary: 3.1x (target 3.5x, 80% threshold = 2.8x)
        let actual = 3.1;
        let passed = pilot.validate_canary("analytics_engine", actual, 0.88).unwrap();
        assert!(passed);
        
        assert!(pilot.start_beta("analytics_engine").is_ok());
        assert!(pilot.transition_to_ga("analytics_engine", 3.1).is_ok());
        
        assert_eq!(pilot.successful_count(), 1);
    }
    
    #[test]
    fn test_canary_failure_triggers_rollback() {
        // Simulate a canary failure (speedup too low)
        let targets = vec![
            DeploymentTarget::new("failed_app".to_string(), LoopType::CpuBound, 40.0, 4.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        assert!(pilot.start_profiling("failed_app").is_ok());
        assert!(pilot.complete_profiling("failed_app", 40.0).is_ok());
        
        // Start canary with expected 4.0x
        let predicted = 4.0;
        assert!(pilot.start_canary("failed_app", predicted).is_ok());
        
        // Canary fails: 1.8x actual (threshold = 3.2x)
        let actual = 1.8;
        let passed = pilot.validate_canary("failed_app", actual, 0.50).unwrap();
        assert!(!passed, "Canary should fail: {:.2}x < {:.2}x", actual, predicted * 0.8);
        
        // After failed validation, we should rollback instead of advancing to beta
        assert!(pilot.rollback("failed_app", "Canary speedup insufficient").is_ok());
        assert_eq!(pilot.get_stage("failed_app"), Some(DeploymentStage::RolledBack));
        assert_eq!(pilot.successful_count(), 0);
    }
    
    #[test]
    fn test_three_app_pilot_mixed_results() {
        // Comprehensive test: 3 apps, 2 succeed, 1 fails
        let targets = vec![
            DeploymentTarget::new("web_server".to_string(), LoopType::CpuBound, 50.0, 5.0),
            DeploymentTarget::new("data_pipeline".to_string(), LoopType::MemoryBound, 120.0, 2.5),
            DeploymentTarget::new("analytics_engine".to_string(), LoopType::Mixed, 60.0, 3.5),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // ===== APP 1: Web Server (Success) =====
        assert!(pilot.start_profiling("web_server").is_ok());
        assert!(pilot.complete_profiling("web_server", 49.0).is_ok());
        assert!(pilot.start_canary("web_server", 5.0).is_ok());
        let passed1 = pilot.validate_canary("web_server", 4.8, 0.96).unwrap();
        assert!(passed1);
        assert!(pilot.start_beta("web_server").is_ok());
        assert!(pilot.transition_to_ga("web_server", 4.8).is_ok());
        
        // ===== APP 2: Data Pipeline (Success) =====
        assert!(pilot.start_profiling("data_pipeline").is_ok());
        assert!(pilot.complete_profiling("data_pipeline", 119.0).is_ok());
        assert!(pilot.start_canary("data_pipeline", 2.5).is_ok());
        let passed2 = pilot.validate_canary("data_pipeline", 2.3, 0.92).unwrap();
        assert!(passed2);
        assert!(pilot.start_beta("data_pipeline").is_ok());
        assert!(pilot.transition_to_ga("data_pipeline", 2.3).is_ok());
        
        // ===== APP 3: Analytics (Failure & Rollback) =====
        assert!(pilot.start_profiling("analytics_engine").is_ok());
        assert!(pilot.complete_profiling("analytics_engine", 59.0).is_ok());
        assert!(pilot.start_canary("analytics_engine", 3.5).is_ok());
        let passed3 = pilot.validate_canary("analytics_engine", 1.9, 0.55).unwrap();
        assert!(!passed3);  // Below threshold
        assert!(pilot.rollback("analytics_engine", "Canary speedup insufficient").is_ok());
        
        // Verify results
        assert_eq!(pilot.app_count(), 3);
        assert_eq!(pilot.successful_count(), 2);
        assert_eq!(pilot.get_stage("web_server"), Some(DeploymentStage::GeneralAvailability));
        assert_eq!(pilot.get_stage("data_pipeline"), Some(DeploymentStage::GeneralAvailability));
        assert_eq!(pilot.get_stage("analytics_engine"), Some(DeploymentStage::RolledBack));
    }
    
    #[test]
    fn test_deployment_stage_transitions() {
        // Test all valid state transitions
        let targets = vec![
            DeploymentTarget::new("test_app".to_string(), LoopType::CpuBound, 30.0, 4.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Pending -> Profiling
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::Pending));
        assert!(pilot.start_profiling("test_app").is_ok());
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::Profiling));
        
        // Profiling -> Discovery
        assert!(pilot.complete_profiling("test_app", 30.0).is_ok());
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::Discovery));
        
        // Discovery -> Canary
        assert!(pilot.start_canary("test_app", 4.0).is_ok());
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::Canary));
        
        // Validate canary
        assert!(pilot.validate_canary("test_app", 3.8, 0.95).unwrap());
        
        // Canary -> Beta
        assert!(pilot.start_beta("test_app").is_ok());
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::Beta));
        
        // Beta -> GeneralAvailability
        assert!(pilot.transition_to_ga("test_app", 3.8).is_ok());
        assert_eq!(pilot.get_stage("test_app"), Some(DeploymentStage::GeneralAvailability));
    }
    
    #[test]
    fn test_invalid_stage_transitions_rejected() {
        let targets = vec![
            DeploymentTarget::new("test_app".to_string(), LoopType::MemoryBound, 100.0, 2.5),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Cannot go directly to canary
        let result = pilot.start_canary("test_app", 2.5);
        assert!(result.is_err(), "Cannot canary from Pending state");
        
        // Cannot transition to beta without canary
        assert!(pilot.start_profiling("test_app").is_ok());
        assert!(pilot.complete_profiling("test_app", 100.0).is_ok());
        let result = pilot.start_beta("test_app");
        assert!(result.is_err(), "Cannot beta from Discovery state");
        
        // Cannot transition to GA without beta
        assert!(pilot.start_canary("test_app", 2.5).is_ok());
        let result = pilot.transition_to_ga("test_app", 2.5);
        assert!(result.is_err(), "Cannot GA from Canary state");
    }
    
    #[test]
    fn test_deployment_event_logging() {
        let targets = vec![
            DeploymentTarget::new("logging_app".to_string(), LoopType::Mixed, 40.0, 3.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Execute deployment steps (these should log events)
        assert!(pilot.start_profiling("logging_app").is_ok());
        assert!(pilot.complete_profiling("logging_app", 40.0).is_ok());
        assert!(pilot.start_canary("logging_app", 3.0).is_ok());
        let _ = pilot.validate_canary("logging_app", 2.8, 0.93);
        assert!(pilot.start_beta("logging_app").is_ok());
        assert!(pilot.transition_to_ga("logging_app", 2.8).is_ok());
        
        // Get status report with deployment history
        let report = pilot.status_report();
        
        // Verify events were logged (check message content, not stage names)
        assert!(report.contains("logging_app"));
        assert!(report.contains("Baseline"));  // From complete_profiling message
        assert!(report.contains("Canary deployment"));  // From start_canary message
        assert!(report.contains("General Availability"));  // From transition_to_ga message
    }
    
    #[test]
    fn test_nonexistent_app_errors() {
        let targets = vec![
            DeploymentTarget::new("real_app".to_string(), LoopType::CpuBound, 30.0, 4.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // All operations on nonexistent app should fail
        assert!(pilot.start_profiling("fake_app").is_err());
        assert!(pilot.complete_profiling("fake_app", 30.0).is_err());
        assert!(pilot.start_canary("fake_app", 4.0).is_err());
        assert!(pilot.start_beta("fake_app").is_err());
        assert!(pilot.transition_to_ga("fake_app", 4.0).is_err());
        assert!(pilot.rollback("fake_app", "test").is_err());
        assert_eq!(pilot.get_stage("fake_app"), None);
    }
}
