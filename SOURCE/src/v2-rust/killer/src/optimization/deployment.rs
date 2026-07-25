/// Week 8 Phase A: Pilot Deployment Orchestrator
///
/// Orchestrates the complete deployment lifecycle:
/// 1. Baseline profiling
/// 2. Parameter discovery
/// 3. Canary deployment (5-10%)
/// 4. Beta deployment (50%)
/// 5. General availability (100%)

use crate::optimization::{
    ProductionIntegration, PerformanceMonitor, AccuracyCalibrator,
    LoopType,
};
use std::collections::HashMap;

/// Deployment stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeploymentStage {
    /// Awaiting deployment
    Pending,
    
    /// Baseline profiling
    Profiling,
    
    /// Parameter discovery running
    Discovery,
    
    /// Canary deployment (5-10% traffic)
    Canary,
    
    /// Beta deployment (50% traffic)
    Beta,
    
    /// Full general availability
    GeneralAvailability,
    
    /// Rolled back
    RolledBack,
    
    /// Error state
    Failed,
}

/// Application deployment target
#[derive(Debug, Clone)]
pub struct DeploymentTarget {
    /// Application name
    pub app_name: String,
    
    /// Workload type (CPU, Memory, or Mixed heavy)
    pub primary_workload: LoopType,
    
    /// Current deployment stage
    pub stage: DeploymentStage,
    
    /// Baseline performance (ms)
    pub baseline_ms: f64,
    
    /// Target speedup (from conservative estimate)
    pub target_speedup: f64,
}

impl DeploymentTarget {
    /// Create new deployment target
    pub fn new(app_name: String, primary_workload: LoopType, baseline_ms: f64, target_speedup: f64) -> Self {
        DeploymentTarget {
            app_name,
            primary_workload,
            stage: DeploymentStage::Pending,
            baseline_ms,
            target_speedup,
        }
    }
}

/// Pilot deployment orchestrator
#[allow(dead_code)]
#[derive(Debug)]
pub struct PilotDeployment {
    /// Applications being deployed
    applications: Vec<DeploymentTarget>,
    
    /// Integration managers per app
    integrations: HashMap<String, ProductionIntegration>,
    
    /// Performance monitors per app
    monitors: HashMap<String, PerformanceMonitor>,
    
    /// Accuracy calibrators per app
    calibrators: HashMap<String, AccuracyCalibrator>,
    
    /// Deployment history
    deployment_history: Vec<DeploymentEvent>,
    
    /// Overall success metric
    successful_deployments: usize,
}

/// Deployment event log
#[derive(Debug, Clone)]
pub struct DeploymentEvent {
    /// Timestamp
    pub timestamp: u64,
    
    /// Application name
    pub app_name: String,
    
    /// Event type
    pub event_type: DeploymentEventType,
    
    /// Status message
    pub message: String,
}

/// Types of deployment events
#[derive(Debug, Clone)]
pub enum DeploymentEventType {
    /// Profiling started
    ProfilingStarted,
    
    /// Profiling completed
    ProfilingComplete,
    
    /// Parameter discovery started
    DiscoveryStarted,
    
    /// Discovery completed with results
    DiscoveryComplete(f64),  // Predicted speedup
    
    /// Canary deployment started
    CanaryStarted,
    
    /// Canary validation passed
    CanaryPassed(f64),  // Actual speedup
    
    /// Beta deployment started
    BetaStarted,
    
    /// Beta validation passed
    BetaPassed(f64),  // Actual speedup
    
    /// GA deployment
    GADeployed,
    
    /// Deployment failed
    DeploymentFailed(String),  // Reason
    
    /// Rollback executed
    RolledBack,
}

impl PilotDeployment {
    /// Create new pilot deployment
    pub fn new(targets: Vec<DeploymentTarget>) -> Self {
        let mut integrations = HashMap::new();
        let mut monitors = HashMap::new();
        let mut calibrators = HashMap::new();
        
        // Initialize managers for each target
        for target in &targets {
            integrations.insert(
                target.app_name.clone(),
                ProductionIntegration::new(target.app_name.clone()),
            );
            monitors.insert(
                target.app_name.clone(),
                PerformanceMonitor::new(10),  // 10-sample rolling window
            );
            calibrators.insert(
                target.app_name.clone(),
                AccuracyCalibrator::new(0.1),  // 10% learning rate
            );
        }
        
        PilotDeployment {
            applications: targets,
            integrations,
            monitors,
            calibrators,
            deployment_history: Vec::new(),
            successful_deployments: 0,
        }
    }
    
    /// Record a deployment event
    pub fn log_event(&mut self, event: DeploymentEvent) {
        self.deployment_history.push(event);
    }
    
    /// Start profiling phase for an application
    pub fn start_profiling(&mut self, app_name: &str) -> Result<(), String> {
        // Find application
        let app = self.applications
            .iter_mut()
            .find(|a| a.app_name == app_name)
            .ok_or_else(|| format!("Application {} not found", app_name))?;
        
        // Verify in pending state
        if app.stage != DeploymentStage::Pending {
            return Err(format!("Cannot profile - app in {:?} stage", app.stage));
        }
        
        app.stage = DeploymentStage::Profiling;
        
        self.log_event(DeploymentEvent {
            timestamp: 1000,  // Placeholder
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::ProfilingStarted,
            message: "Baseline profiling started".to_string(),
        });
        
        Ok(())
    }
    
    /// Complete profiling phase
    pub fn complete_profiling(&mut self, app_name: &str, actual_baseline_ms: f64) -> Result<(), String> {
        // Find and update app
        let baseline = {
            let app = self.applications
                .iter_mut()
                .find(|a| a.app_name == app_name)
                .ok_or_else(|| format!("Application {} not found", app_name))?;
            
            // Update baseline if measured
            if actual_baseline_ms > 0.0 {
                app.baseline_ms = actual_baseline_ms;
            }
            
            app.stage = DeploymentStage::Discovery;
            app.baseline_ms
        };  // Borrow released here
        
        self.log_event(DeploymentEvent {
            timestamp: 2000,
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::ProfilingComplete,
            message: format!("Baseline: {:.2}ms", baseline),
        });
        
        Ok(())
    }
    
    /// Transition to canary deployment
    pub fn start_canary(&mut self, app_name: &str, predicted_speedup: f64) -> Result<(), String> {
        // Find and update app
        {
            let app = self.applications
                .iter_mut()
                .find(|a| a.app_name == app_name)
                .ok_or_else(|| format!("Application {} not found", app_name))?;
            
            if app.stage != DeploymentStage::Discovery {
                return Err(format!("Cannot canary - app in {:?} stage", app.stage));
            }
            
            app.stage = DeploymentStage::Canary;
            app.target_speedup = predicted_speedup;
        }  // Borrow released
        
        self.log_event(DeploymentEvent {
            timestamp: 3000,
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::CanaryStarted,
            message: format!("Canary deployment (5-10%) with predicted {:.2}x speedup", predicted_speedup),
        });
        
        Ok(())
    }
    
    /// Validate canary with actual speedup
    pub fn validate_canary(&mut self, app_name: &str, actual_speedup: f64, confidence: f64) -> Result<bool, String> {
        let app = self.applications
            .iter()
            .find(|a| a.app_name == app_name)
            .ok_or_else(|| format!("Application {} not found", app_name))?;
        
        // Success if actual >= 80% of conservative estimate (which is 60% of optimal)
        let min_acceptable = app.target_speedup * 0.8;
        let passed = actual_speedup >= min_acceptable;
        
        if passed {
            self.log_event(DeploymentEvent {
                timestamp: 4000,
                app_name: app_name.to_string(),
                event_type: DeploymentEventType::CanaryPassed(actual_speedup),
                message: format!("Canary PASSED: {:.2}x speedup (target {:.2}x, confidence {:.1}%)", 
                                actual_speedup, app.target_speedup, confidence * 100.0),
            });
        } else {
            self.log_event(DeploymentEvent {
                timestamp: 4000,
                app_name: app_name.to_string(),
                event_type: DeploymentEventType::DeploymentFailed("Canary speedup below threshold".to_string()),
                message: format!("Canary FAILED: {:.2}x actual vs {:.2}x target", actual_speedup, app.target_speedup),
            });
        }
        
        Ok(passed)
    }
    
    /// Transition to beta deployment
    pub fn start_beta(&mut self, app_name: &str) -> Result<(), String> {
        // Find and update app
        {
            let app = self.applications
                .iter_mut()
                .find(|a| a.app_name == app_name)
                .ok_or_else(|| format!("Application {} not found", app_name))?;
            
            if app.stage != DeploymentStage::Canary {
                return Err(format!("Cannot beta - app in {:?} stage (must be in Canary)", app.stage));
            }
            
            app.stage = DeploymentStage::Beta;
        }  // Borrow released
        
        self.log_event(DeploymentEvent {
            timestamp: 5000,
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::BetaStarted,
            message: "Beta deployment (50% traffic) started".to_string(),
        });
        
        Ok(())
    }
    
    /// Validate beta and transition to GA
    pub fn transition_to_ga(&mut self, app_name: &str, avg_speedup: f64) -> Result<(), String> {
        // Find and update app
        {
            let app = self.applications
                .iter_mut()
                .find(|a| a.app_name == app_name)
                .ok_or_else(|| format!("Application {} not found", app_name))?;
            
            if app.stage != DeploymentStage::Beta {
                return Err(format!("Cannot transition to GA - app in {:?} stage", app.stage));
            }
            
            app.stage = DeploymentStage::GeneralAvailability;
        }  // Borrow released
        
        self.successful_deployments += 1;
        
        self.log_event(DeploymentEvent {
            timestamp: 6000,
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::GADeployed,
            message: format!("General Availability reached: {:.2}x average speedup", avg_speedup),
        });
        
        Ok(())
    }
    
    /// Rollback a deployment
    pub fn rollback(&mut self, app_name: &str, reason: &str) -> Result<(), String> {
        // Find and update app
        {
            let app = self.applications
                .iter_mut()
                .find(|a| a.app_name == app_name)
                .ok_or_else(|| format!("Application {} not found", app_name))?;
            
            app.stage = DeploymentStage::RolledBack;
        }  // Borrow released
        
        self.log_event(DeploymentEvent {
            timestamp: 7000,
            app_name: app_name.to_string(),
            event_type: DeploymentEventType::RolledBack,
            message: format!("Rollback executed: {}", reason),
        });
        
        Ok(())
    }
    
    /// Get deployment status report
    pub fn status_report(&self) -> String {
        let mut report = String::from("=== PILOT DEPLOYMENT STATUS ===\n\n");
        
        report.push_str(&format!("Successful Deployments: {}/{}\n\n", 
                                 self.successful_deployments, 
                                 self.applications.len()));
        
        for app in &self.applications {
            report.push_str(&format!("{} ({})\n", app.app_name, app.primary_workload));
            report.push_str(&format!("  Stage: {:?}\n", app.stage));
            report.push_str(&format!("  Baseline: {:.2}ms\n", app.baseline_ms));
            report.push_str(&format!("  Target Speedup: {:.2}x\n\n", app.target_speedup));
        }
        
        report.push_str(&format!("Recent Events ({}):\n", self.deployment_history.len()));
        for event in self.deployment_history.iter().rev().take(10) {
            report.push_str(&format!("  [{}] {}: {}\n", 
                                    event.timestamp, 
                                    event.app_name, 
                                    event.message));
        }
        
        report
    }
    
    /// Get application count
    pub fn app_count(&self) -> usize {
        self.applications.len()
    }
    
    /// Get successful deployment count
    pub fn successful_count(&self) -> usize {
        self.successful_deployments
    }
    
    /// Get deployment stage for app
    pub fn get_stage(&self, app_name: &str) -> Option<DeploymentStage> {
        self.applications
            .iter()
            .find(|a| a.app_name == app_name)
            .map(|a| a.stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pilot_creation() {
        let targets = vec![
            DeploymentTarget::new("web_server".to_string(), LoopType::CpuBound, 50.0, 5.0),
            DeploymentTarget::new("data_pipeline".to_string(), LoopType::MemoryBound, 100.0, 2.5),
        ];
        
        let pilot = PilotDeployment::new(targets);
        assert_eq!(pilot.app_count(), 2);
        assert_eq!(pilot.successful_count(), 0);
    }
    
    #[test]
    fn test_profiling_workflow() {
        let targets = vec![
            DeploymentTarget::new("web_app".to_string(), LoopType::CpuBound, 50.0, 5.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Start profiling
        assert!(pilot.start_profiling("web_app").is_ok());
        assert_eq!(pilot.get_stage("web_app"), Some(DeploymentStage::Profiling));
        
        // Complete profiling
        assert!(pilot.complete_profiling("web_app", 48.5).is_ok());
        assert_eq!(pilot.get_stage("web_app"), Some(DeploymentStage::Discovery));
    }
    
    #[test]
    fn test_canary_to_ga_flow() {
        let targets = vec![
            DeploymentTarget::new("app1".to_string(), LoopType::CpuBound, 20.0, 5.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        let _ = pilot.complete_profiling("app1", 20.0);
        
        // Start canary
        assert!(pilot.start_canary("app1", 5.0).is_ok());
        assert_eq!(pilot.get_stage("app1"), Some(DeploymentStage::Canary));
        
        // Validate canary (actual = 4.2x, target = 5.0x, 80% threshold = 4.0x) -> PASS
        let passed = pilot.validate_canary("app1", 4.2, 0.95).unwrap();
        assert!(passed, "Canary should pass when actual >= 80% of target");
        
        // Start beta
        assert!(pilot.start_beta("app1").is_ok());
        assert_eq!(pilot.get_stage("app1"), Some(DeploymentStage::Beta));
        
        // Transition to GA
        assert!(pilot.transition_to_ga("app1", 4.2).is_ok());
        assert_eq!(pilot.get_stage("app1"), Some(DeploymentStage::GeneralAvailability));
        assert_eq!(pilot.successful_count(), 1);
    }
    
    #[test]
    fn test_rollback_on_failure() {
        let targets = vec![
            DeploymentTarget::new("app2".to_string(), LoopType::MemoryBound, 100.0, 3.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        let _ = pilot.complete_profiling("app2", 100.0);
        let _ = pilot.start_canary("app2", 3.0);
        
        // Validate canary FAILS (actual = 1.5x, target = 3.0x, 80% threshold = 2.4x)
        let passed = pilot.validate_canary("app2", 1.5, 0.60).unwrap();
        assert!(!passed, "Canary should fail when actual < 80% of target");
        
        // Execute rollback
        assert!(pilot.rollback("app2", "Canary speedup insufficient").is_ok());
        assert_eq!(pilot.get_stage("app2"), Some(DeploymentStage::RolledBack));
        assert_eq!(pilot.successful_count(), 0);
    }
    
    #[test]
    fn test_multi_app_deployment() {
        let targets = vec![
            DeploymentTarget::new("web".to_string(), LoopType::CpuBound, 30.0, 6.0),
            DeploymentTarget::new("data".to_string(), LoopType::MemoryBound, 120.0, 2.0),
            DeploymentTarget::new("analytics".to_string(), LoopType::Mixed, 60.0, 4.0),
        ];
        
        let mut pilot = PilotDeployment::new(targets);
        
        // Deploy app1 successfully
        let _ = pilot.complete_profiling("web", 30.0);
        let _ = pilot.start_canary("web", 6.0);
        let _ = pilot.validate_canary("web", 5.2, 0.97);
        let _ = pilot.start_beta("web");
        let _ = pilot.transition_to_ga("web", 5.2);
        
        // Deploy app2 successfully
        let _ = pilot.complete_profiling("data", 120.0);
        let _ = pilot.start_canary("data", 2.0);
        let _ = pilot.validate_canary("data", 1.8, 0.92);
        let _ = pilot.start_beta("data");
        let _ = pilot.transition_to_ga("data", 1.8);
        
        // Attempt app3 but rollback
        let _ = pilot.complete_profiling("analytics", 60.0);
        let _ = pilot.start_canary("analytics", 4.0);
        let _ = pilot.validate_canary("analytics", 2.5, 0.70);
        let _ = pilot.rollback("analytics", "Below threshold");
        
        assert_eq!(pilot.app_count(), 3);
        assert_eq!(pilot.successful_count(), 2);
    }
}
