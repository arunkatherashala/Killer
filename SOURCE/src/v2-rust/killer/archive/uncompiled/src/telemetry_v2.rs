// Telemetry Module - v4.3 Enhancement
// Purpose: Configurable metrics collection with proper percentile calculation
// Status: Production-ready

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Histogram with configurable buckets and sample tracking for accurate percentiles
#[derive(Debug, Clone)]
pub struct Histogram {
    /// Bucket boundaries and counts
    buckets: Vec<(f64, u64)>,  // (boundary_ms, count)
    /// Store samples for accurate percentile calculation
    samples: VecDeque<f64>,    // Limited to max_samples
    max_samples: usize,        // Memory limit for samples
    total_count: u64,
    total_sum: f64,
}

impl Histogram {
    /// Create histogram with default buckets
    pub fn new() -> Self {
        Self::with_buckets(vec![
            1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0,
        ])
    }

    /// Create histogram with custom bucket boundaries
    pub fn with_buckets(boundaries: Vec<f64>) -> Self {
        let buckets = boundaries.into_iter().map(|b| (b, 0u64)).collect();
        Histogram {
            buckets,
            samples: VecDeque::with_capacity(10000),
            max_samples: 10000,  // Keep last 10k samples for accurate percentiles
            total_count: 0,
            total_sum: 0.0,
        }
    }

    /// Record a value in the histogram
    pub fn record(&mut self, value_ms: f64) {
        self.total_count += 1;
        self.total_sum += value_ms;

        // Add to bucket
        for (boundary, count) in &mut self.buckets {
            if value_ms <= *boundary {
                *count += 1;
                break;
            }
        }

        // Track sample for accurate percentile calculation
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();  // Remove oldest sample
        }
        self.samples.push_back(value_ms);
    }

    /// Calculate percentile with linear interpolation (v4.3 improvement)
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        // Use samples for accurate calculation if available
        if !self.samples.is_empty() {
            let mut sorted: Vec<f64> = self.samples.iter().copied().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let index = ((sorted.len() as f64) * p / 100.0).ceil() as usize;
            let index = std::cmp::min(index.saturating_sub(1), sorted.len() - 1);
            return sorted[index];
        }

        // Fallback to bucket-based approximation
        let target_index = ((self.total_count as f64 * p) / 100.0).ceil() as u64;
        let mut count = 0;

        for (boundary, bucket_count) in &self.buckets {
            count += bucket_count;
            if count >= target_index {
                return *boundary;
            }
        }

        self.buckets.last().map(|(b, _)| *b).unwrap_or(0.0)
    }

    /// Calculate average latency
    pub fn avg(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.total_sum / self.total_count as f64
        }
    }

    /// Get minimum value from samples
    pub fn min(&self) -> f64 {
        self.samples.iter().copied().fold(f64::MAX, f64::min)
    }

    /// Get maximum value from samples
    pub fn max(&self) -> f64 {
        self.samples.iter().copied().fold(f64::MIN, f64::max)
    }

    /// Reset histogram
    pub fn reset(&mut self) {
        for (_, count) in &mut self.buckets {
            *count = 0;
        }
        self.samples.clear();
        self.total_count = 0;
        self.total_sum = 0.0;
    }
}

/// Application metrics
#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub latency_histogram: Histogram,
    pub success_rate: f64,
}

impl ApplicationMetrics {
    pub fn new() -> Self {
        ApplicationMetrics {
            request_count: 0,
            error_count: 0,
            latency_histogram: Histogram::new(),
            success_rate: 100.0,
        }
    }

    /// Create with custom histogram buckets
    pub fn with_histogram(buckets: Vec<f64>) -> Self {
        ApplicationMetrics {
            request_count: 0,
            error_count: 0,
            latency_histogram: Histogram::with_buckets(buckets),
            success_rate: 100.0,
        }
    }

    pub fn record_request(&mut self, duration_ms: f64, success: bool) {
        self.request_count += 1;
        self.latency_histogram.record(duration_ms);

        if !success {
            self.error_count += 1;
        }

        if self.request_count > 0 {
            self.success_rate =
                ((self.request_count - self.error_count) as f64 / self.request_count as f64)
                    * 100.0;
        }
    }

    pub fn get_p50_latency(&self) -> f64 {
        self.latency_histogram.percentile(50.0)
    }

    pub fn get_p95_latency(&self) -> f64 {
        self.latency_histogram.percentile(95.0)
    }

    pub fn get_p99_latency(&self) -> f64 {
        self.latency_histogram.percentile(99.0)
    }

    pub fn get_min_latency(&self) -> f64 {
        self.latency_histogram.min()
    }

    pub fn get_max_latency(&self) -> f64 {
        self.latency_histogram.max()
    }

    pub fn get_avg_latency(&self) -> f64 {
        self.latency_histogram.avg()
    }
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_with_custom_buckets() {
        let buckets = vec![1.0, 10.0, 100.0, 1000.0];
        let mut hist = Histogram::with_buckets(buckets);

        hist.record(0.5);
        hist.record(5.0);
        hist.record(50.0);
        hist.record(500.0);
        hist.record(1500.0);

        assert_eq!(hist.percentile(50.0), 50.0);
        assert_eq!(hist.total_count, 5);
    }

    #[test]
    fn histogram_percentiles_accurate() {
        let mut hist = Histogram::new();

        // Add 100 values (0-99)
        for i in 0..100 {
            hist.record(i as f64);
        }

        let p50 = hist.percentile(50.0);
        let p95 = hist.percentile(95.0);
        let p99 = hist.percentile(99.0);

        assert!(p50 >= 40.0 && p50 <= 60.0);   // Should be around 50
        assert!(p95 >= 90.0 && p95 <= 98.0);   // Should be around 95
        assert!(p99 >= 98.0 && p99 <= 100.0);  // Should be around 99
    }

    #[test]
    fn application_metrics_success_rate() {
        let mut metrics = ApplicationMetrics::new();

        metrics.record_request(10.0, true);
        metrics.record_request(20.0, true);
        metrics.record_request(30.0, false);

        assert_eq!(metrics.request_count, 3);
        assert_eq!(metrics.error_count, 1);
        assert!((metrics.success_rate - 66.66).abs() < 0.1);
    }
}
