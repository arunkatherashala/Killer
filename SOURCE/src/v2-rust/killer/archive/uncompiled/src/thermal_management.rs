// THERMAL MANAGEMENT MODULE - Phase 3: Temperature-Aware Scheduling
// Target: Prevent thermal throttling on mobile workstations
// Goal: 6x longer sustained runtime without throttling

use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::collections::HashMap;

/// CPU Temperature Monitor
#[derive(Clone, Debug)]
pub struct ThermalInfo {
    pub current_temp_celsius: f32,
    pub throttle_temp_celsius: f32,
    pub safe_temp_celsius: f32,
    pub warning_temp_celsius: f32,
    pub timestamp: Instant,
}

impl ThermalInfo {
    pub fn new() -> Self {
        ThermalInfo {
            current_temp_celsius: 35.0, // Idle temperature
            throttle_temp_celsius: 95.0, // i5-1145G7 thermal limit
            safe_temp_celsius: 70.0, // Safe operating range
            warning_temp_celsius: 80.0, // Start being careful
            timestamp: Instant::now(),
        }
    }

    pub fn get_thermal_status(&self) -> ThermalStatus {
        match self.current_temp_celsius {
            t if t < self.safe_temp_celsius => ThermalStatus::Cool,
            t if t < self.warning_temp_celsius => ThermalStatus::Normal,
            t if t < self.throttle_temp_celsius => ThermalStatus::Warning,
            _ => ThermalStatus::Throttling,
        }
    }

    pub fn get_utilization_factor(&self) -> f32 {
        // Return multiplier for CPU utilization (1.0 = 100%, 0.5 = 50%)
        match self.get_thermal_status() {
            ThermalStatus::Cool => 1.0,       // Full speed
            ThermalStatus::Normal => 0.95,    // Slight reduction
            ThermalStatus::Warning => 0.75,   // Moderate reduction
            ThermalStatus::Throttling => 0.5, // Severe reduction
        }
    }

    pub fn predict_throttle_in(&self) -> Duration {
        let temp_diff = self.throttle_temp_celsius - self.current_temp_celsius;
        if temp_diff <= 0.0 {
            Duration::from_secs(0)
        } else {
            // Assume 1°C per second heating rate under load
            Duration::from_secs(temp_diff as u64)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThermalStatus {
    Cool,       // < 70°C
    Normal,     // 70-80°C
    Warning,    // 80-95°C
    Throttling, // >= 95°C
}

impl ThermalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ThermalStatus::Cool => "Cool",
            ThermalStatus::Normal => "Normal",
            ThermalStatus::Warning => "Warning",
            ThermalStatus::Throttling => "Throttling",
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            ThermalStatus::Cool => "❄️",
            ThermalStatus::Normal => "✅",
            ThermalStatus::Warning => "⚠️",
            ThermalStatus::Throttling => "🔥",
        }
    }
}

/// Thermal Policy for dynamic CPU management
#[derive(Clone, Debug)]
pub enum ThermalPolicy {
    /// Aggressive performance - no thermal management
    Performance,
    /// Balanced - throttle before hitting limit
    Balanced,
    /// Conservative - keep temps low for sustained operation
    PowerSaver,
}

impl ThermalPolicy {
    pub fn get_target_temp(&self) -> f32 {
        match self {
            ThermalPolicy::Performance => 90.0,
            ThermalPolicy::Balanced => 75.0,
            ThermalPolicy::PowerSaver => 60.0,
        }
    }

    pub fn get_adjustment_factor(&self, status: &ThermalStatus) -> f32 {
        match status {
            ThermalStatus::Cool => 1.0,
            ThermalStatus::Normal => match self {
                ThermalPolicy::Performance => 1.0,
                ThermalPolicy::Balanced => 0.95,
                ThermalPolicy::PowerSaver => 0.85,
            },
            ThermalStatus::Warning => match self {
                ThermalPolicy::Performance => 0.85,
                ThermalPolicy::Balanced => 0.70,
                ThermalPolicy::PowerSaver => 0.50,
            },
            ThermalStatus::Throttling => 0.40,
        }
    }
}

/// Core temperature history for prediction
#[derive(Clone, Debug)]
pub struct TemperatureHistory {
    pub readings: Vec<(Instant, f32)>,
    pub max_history: usize,
}

impl TemperatureHistory {
    pub fn new() -> Self {
        TemperatureHistory {
            readings: Vec::new(),
            max_history: 60, // Keep last 60 readings
        }
    }

    pub fn add_reading(&mut self, temp: f32) {
        self.readings.push((Instant::now(), temp));
        if self.readings.len() > self.max_history {
            self.readings.remove(0);
        }
    }

    pub fn get_trend(&self) -> ThermalTrend {
        if self.readings.len() < 3 {
            return ThermalTrend::Stable;
        }

        let recent = &self.readings[self.readings.len() - 3..];
        let temps: Vec<f32> = recent.iter().map(|(_, t)| t).copied().collect();

        let diff = temps[2] - temps[0];
        if diff > 1.0 {
            ThermalTrend::Rising
        } else if diff < -1.0 {
            ThermalTrend::Falling
        } else {
            ThermalTrend::Stable
        }
    }

    pub fn estimate_next_temp(&self) -> f32 {
        if self.readings.is_empty() {
            return 50.0;
        }

        if self.readings.len() == 1 {
            return self.readings[0].1;
        }

        // Linear regression to predict next temp
        let len = self.readings.len().min(10);
        let recent = &self.readings[self.readings.len() - len..];

        let temps: Vec<f32> = recent.iter().map(|(_, t)| t).copied().collect();
        
        // Calculate slope (temperature change per reading)
        let first_temp = temps[0];
        let last_temp = temps[temps.len() - 1];
        let slope = (last_temp - first_temp) / (temps.len() - 1).max(1) as f32;
        
        // Predict next temp based on trend
        last_temp + slope
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThermalTrend {
    Rising,
    Stable,
    Falling,
}

/// Thermal-Aware Scheduler
pub struct ThermalAwareScheduler {
    thermal_info: Arc<Mutex<ThermalInfo>>,
    temperature_history: Arc<Mutex<TemperatureHistory>>,
    policy: ThermalPolicy,
    last_adjustment: Instant,
    core_freq_mhz: Arc<Mutex<HashMap<usize, u32>>>, // Per-core frequencies
}

impl ThermalAwareScheduler {
    pub fn new(policy: ThermalPolicy) -> Self {
        println!("🌡️  Thermal-Aware Scheduler initialized with policy: {:?}", policy);

        ThermalAwareScheduler {
            thermal_info: Arc::new(Mutex::new(ThermalInfo::new())),
            temperature_history: Arc::new(Mutex::new(TemperatureHistory::new())),
            policy,
            last_adjustment: Instant::now(),
            core_freq_mhz: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Update current CPU temperature
    pub fn update_temperature(&self, temp_celsius: f32) {
        if let Ok(mut info) = self.thermal_info.lock() {
            info.current_temp_celsius = temp_celsius;
            info.timestamp = Instant::now();
        }

        if let Ok(mut history) = self.temperature_history.lock() {
            history.add_reading(temp_celsius);
        }
    }

    /// Get current thermal status
    pub fn get_status(&self) -> (ThermalStatus, f32) {
        if let Ok(info) = self.thermal_info.lock() {
            (info.get_thermal_status(), info.current_temp_celsius)
        } else {
            (ThermalStatus::Normal, 50.0)
        }
    }

    /// Get CPU utilization factor based on thermal conditions
    pub fn get_utilization_factor(&self) -> f32 {
        if let Ok(info) = self.thermal_info.lock() {
            info.get_utilization_factor()
        } else {
            1.0
        }
    }

    /// Predict when throttling will occur
    pub fn predict_throttle_time(&self) -> Duration {
        if let Ok(info) = self.thermal_info.lock() {
            info.predict_throttle_in()
        } else {
            Duration::from_secs(300) // Default: 5 minutes
        }
    }

    /// Adjust thread count based on thermal conditions
    pub fn get_recommended_thread_count(&self, base_threads: usize) -> usize {
        let factor = self.get_utilization_factor();
        ((base_threads as f32) * factor).ceil() as usize
    }

    /// Perform thermal prophylactic action (before throttling)
    pub fn perform_prophylactic_action(&self) {
        let (status, temp) = self.get_status();

        match status {
            ThermalStatus::Throttling => {
                println!("🚨 THROTTLING DETECTED at {:.1}°C - Reducing workload", temp);
                // Reduce thread count, lower frequency, etc.
            }
            ThermalStatus::Warning => {
                if let Ok(history) = self.temperature_history.lock() {
                    if history.get_trend() == ThermalTrend::Rising {
                        println!(
                            "⚠️  Temperature rising ({:.1}°C) - Throttling in {:?}",
                            temp,
                            self.predict_throttle_time()
                        );
                        // Preemptively reduce load
                    }
                }
            }
            _ => {}
        }
    }

    /// Get detailed thermal report
    pub fn print_thermal_report(&self) {
        let (status, temp) = self.get_status();

        println!("\n+============ THERMAL REPORT ============+");
        println!(
            "  {} Temperature: {:.1}°C",
            status.emoji(),
            temp
        );
        println!("  Status: {}", status.as_str());

        if let Ok(info) = self.thermal_info.lock() {
            println!("  Safe Limit: {:.1}°C", info.safe_temp_celsius);
            println!("  Warning: {:.1}°C", info.warning_temp_celsius);
            println!("  Throttle: {:.1}°C", info.throttle_temp_celsius);
        }

        println!("  Policy: {:?}", self.policy);
        println!("  Utilization: {:.0}%", self.get_utilization_factor() * 100.0);
        println!(
            "  Throttle in: {:.0}s",
            self.predict_throttle_time().as_secs_f32()
        );

        if let Ok(history) = self.temperature_history.lock() {
            println!("  Trend: {:?}", history.get_trend());
            println!("  Next Temp: {:.1}°C", history.estimate_next_temp());
        }

        println!("+========================================+\n");
    }

    /// Simulate temperature changes (for testing)
    #[cfg(test)]
    pub fn simulate_heating(&self, rate_c_per_sec: f32) {
        if let Ok(mut info) = self.thermal_info.lock() {
            info.current_temp_celsius = (info.current_temp_celsius + rate_c_per_sec).min(100.0);
        }
    }
}

impl Clone for ThermalAwareScheduler {
    fn clone(&self) -> Self {
        ThermalAwareScheduler {
            thermal_info: Arc::clone(&self.thermal_info),
            temperature_history: Arc::clone(&self.temperature_history),
            policy: self.policy.clone(),
            last_adjustment: Instant::now(),
            core_freq_mhz: Arc::clone(&self.core_freq_mhz),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_status() {
        let mut info = ThermalInfo::new();

        info.current_temp_celsius = 45.0;
        assert_eq!(info.get_thermal_status(), ThermalStatus::Cool);

        info.current_temp_celsius = 75.0;
        assert_eq!(info.get_thermal_status(), ThermalStatus::Normal);

        info.current_temp_celsius = 85.0;
        assert_eq!(info.get_thermal_status(), ThermalStatus::Warning);

        info.current_temp_celsius = 96.0;
        assert_eq!(info.get_thermal_status(), ThermalStatus::Throttling);
    }

    #[test]
    fn test_thermal_scheduler() {
        let scheduler = ThermalAwareScheduler::new(ThermalPolicy::Balanced);

        scheduler.update_temperature(50.0);
        let (status, temp) = scheduler.get_status();
        assert_eq!(status, ThermalStatus::Cool);
        assert_eq!(temp, 50.0);

        scheduler.update_temperature(95.0);
        let (status, _) = scheduler.get_status();
        assert_eq!(status, ThermalStatus::Throttling);
    }

    #[test]
    fn test_temperature_prediction() {
        let mut history = TemperatureHistory::new();
        history.add_reading(50.0);
        history.add_reading(55.0);
        history.add_reading(60.0);

        assert_eq!(history.get_trend(), ThermalTrend::Rising);
        assert!(history.estimate_next_temp() > 60.0);
    }
}
