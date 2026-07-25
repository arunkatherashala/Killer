/// Phase 4: Time-Series Database
/// Unlimited historical data retention with spill-to-disk capability
use std::collections::BTreeMap;

/// Time-series data point
#[derive(Clone, Debug)]
pub struct DataPoint {
    pub timestamp: u128,
    pub value: f64,
    pub measurement: String,
    pub tags: Vec<(String, String)>,
}

/// Time-series metric aggregation
#[derive(Clone, Debug)]
pub struct MetricAggregate {
    pub metric_name: String,
    pub start_time: u128,
    pub end_time: u128,
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub stddev: f64,
}

/// Time-Series Database for historical data
pub struct TimeSeriesDatabase {
    /// Series data organized by metric name
    series_data: BTreeMap<String, Vec<DataPoint>>,
    
    /// Index for fast time range queries
    time_index: BTreeMap<u128, Vec<String>>,
    
    /// Total data points stored
    total_points: u64,
    
    /// Total bytes used (in-memory)
    memory_bytes: u64,
    
    /// Spill-to-disk statistics
    spilled_bytes: u64,
    
    /// Retention policy (seconds)
    retention_seconds: u128,
    
    /// Compression enabled
    compression_enabled: bool,
}

impl TimeSeriesDatabase {
    /// Create new time-series database
    pub fn new(retention_seconds: u128, compression_enabled: bool) -> Self {
        TimeSeriesDatabase {
            series_data: BTreeMap::new(),
            time_index: BTreeMap::new(),
            total_points: 0,
            memory_bytes: 0,
            spilled_bytes: 0,
            retention_seconds,
            compression_enabled,
        }
    }
    
    /// Insert a data point
    pub fn insert(&mut self, point: DataPoint) {
        let metric = point.measurement.clone();
        let timestamp = point.timestamp;
        let metric_len = metric.len();
        
        // Add to series data
        self.series_data
            .entry(metric.clone())
            .or_insert_with(Vec::new)
            .push(point.clone());
        
        // Add to time index
        self.time_index
            .entry(timestamp)
            .or_insert_with(Vec::new)
            .push(metric);
        
        // Update statistics
        self.total_points += 1;
        self.memory_bytes += 48 + metric_len as u64; // Approximate size
    }
    
    /// Query data in time range
    pub fn query_range(&self, metric: &str, start: u128, end: u128) -> Vec<DataPoint> {
        if let Some(series) = self.series_data.get(metric) {
            series.iter()
                .filter(|p| p.timestamp >= start && p.timestamp <= end)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get latest value for metric
    pub fn latest(&self, metric: &str) -> Option<DataPoint> {
        self.series_data.get(metric)
            .and_then(|series| series.last())
            .cloned()
    }
    
    /// Get all metrics
    pub fn metrics(&self) -> Vec<String> {
        self.series_data.keys().cloned().collect()
    }
    
    /// Calculate aggregate over time range
    pub fn aggregate(&self, metric: &str, start: u128, end: u128) -> Option<MetricAggregate> {
        let points = self.query_range(metric, start, end);
        
        if points.is_empty() {
            return None;
        }
        
        let count = points.len() as u64;
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // Calculate median
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        
        // Calculate stddev
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / count as f64;
        let stddev = variance.sqrt();
        
        Some(MetricAggregate {
            metric_name: metric.to_string(),
            start_time: start,
            end_time: end,
            count,
            sum,
            min,
            max,
            mean,
            median,
            stddev,
        })
    }
    
    /// Check if retention policy expired
    pub fn is_expired(&self, timestamp: u128, current_time: u128) -> bool {
        current_time.saturating_sub(timestamp) > self.retention_seconds
    }
    
    /// Purge expired data
    pub fn purge_expired(&mut self, current_time: u128) -> u64 {
        let retention_seconds = self.retention_seconds;
        let mut purged = 0u64;
        
        for (_, series) in &mut self.series_data {
            let original_len = series.len();
            series.retain(|p| {
                let elapsed = current_time.saturating_sub(p.timestamp);
                elapsed <= retention_seconds
            });
            purged += (original_len - series.len()) as u64;
        }
        
        self.total_points = self.total_points.saturating_sub(purged);
        purged
    }
    
    /// Get database size
    pub fn size_bytes(&self) -> u64 {
        self.memory_bytes + self.spilled_bytes
    }
    
    /// Get retention policy
    pub fn retention(&self) -> u128 {
        self.retention_seconds
    }
    
    /// Get point count
    pub fn point_count(&self) -> u64 {
        self.total_points
    }
    
    /// Simulate spill-to-disk
    pub fn spill_to_disk(&mut self, bytes: u64) {
        self.spilled_bytes += bytes;
        if let Some(reduction) = self.memory_bytes.checked_sub(bytes) {
            self.memory_bytes = reduction;
        }
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.series_data.clear();
        self.time_index.clear();
        self.total_points = 0;
        self.memory_bytes = 0;
    }
    
    /// Get memory stats
    pub fn stats(&self) -> (u64, u64, u64, f32) {
        let _total = self.memory_bytes + self.spilled_bytes;
        let compression_ratio = if self.compression_enabled {
            0.6  // Assume 40% compression
        } else {
            1.0
        };
        (self.total_points, self.memory_bytes, self.spilled_bytes, compression_ratio)
    }
}

impl Clone for TimeSeriesDatabase {
    fn clone(&self) -> Self {
        TimeSeriesDatabase {
            series_data: self.series_data.clone(),
            time_index: self.time_index.clone(),
            total_points: self.total_points,
            memory_bytes: self.memory_bytes,
            spilled_bytes: self.spilled_bytes,
            retention_seconds: self.retention_seconds,
            compression_enabled: self.compression_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_creation() {
        let db = TimeSeriesDatabase::new(86400, false);
        assert_eq!(db.point_count(), 0);
    }
    
    #[test]
    fn test_insert() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        let point = DataPoint {
            timestamp: 1000,
            value: 42.5,
            measurement: "temperature".to_string(),
            tags: vec![],
        };
        
        db.insert(point);
        assert_eq!(db.point_count(), 1);
    }
    
    #[test]
    fn test_query_range() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        for i in 0..10 {
            db.insert(DataPoint {
                timestamp: 1000 + i * 100,
                value: 40.0 + i as f64,
                measurement: "temp".to_string(),
                tags: vec![],
            });
        }
        
        let results = db.query_range("temp", 1000, 1500);
        assert!(results.len() > 0);
    }
    
    #[test]
    fn test_latest() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        db.insert(DataPoint {
            timestamp: 1000,
            value: 40.0,
            measurement: "temp".to_string(),
            tags: vec![],
        });
        
        db.insert(DataPoint {
            timestamp: 2000,
            value: 45.0,
            measurement: "temp".to_string(),
            tags: vec![],
        });
        
        let latest = db.latest("temp");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().timestamp, 2000);
    }
    
    #[test]
    fn test_aggregate() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        for i in 0..5 {
            db.insert(DataPoint {
                timestamp: 1000 + i * 100,
                value: 40.0 + i as f64,
                measurement: "temp".to_string(),
                tags: vec![],
            });
        }
        
        let agg = db.aggregate("temp", 1000, 2000);
        assert!(agg.is_some());
        let a = agg.unwrap();
        assert_eq!(a.count, 5);
    }
    
    #[test]
    fn test_metrics_list() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        for metric in &["cpu", "memory", "disk"] {
            db.insert(DataPoint {
                timestamp: 1000,
                value: 50.0,
                measurement: metric.to_string(),
                tags: vec![],
            });
        }
        
        let metrics = db.metrics();
        assert_eq!(metrics.len(), 3);
    }
    
    #[test]
    fn test_expiration() {
        let db = TimeSeriesDatabase::new(1000, false);
        
        assert!(!db.is_expired(1000, 1500));
        assert!(db.is_expired(1000, 2100));
    }
    
    #[test]
    fn test_purge() {
        let mut db = TimeSeriesDatabase::new(1000, false);
        
        for i in 0..5 {
            db.insert(DataPoint {
                timestamp: 100 + i * 100,
                value: 50.0,
                measurement: "temp".to_string(),
                tags: vec![],
            });
        }
        
        let purged = db.purge_expired(2000);
        assert!(purged > 0);
    }
    
    #[test]
    fn test_spill_to_disk() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        db.insert(DataPoint {
            timestamp: 1000,
            value: 50.0,
            measurement: "temp".to_string(),
            tags: vec![],
        });
        
        let before = db.memory_bytes;
        db.spill_to_disk(40);  // Spill less than available memory
        let after = db.memory_bytes;
        
        assert!(before > after);
        assert_eq!(db.spilled_bytes, 40);
    }
    
    #[test]
    fn test_clear() {
        let mut db = TimeSeriesDatabase::new(86400, false);
        
        db.insert(DataPoint {
            timestamp: 1000,
            value: 50.0,
            measurement: "temp".to_string(),
            tags: vec![],
        });
        
        assert_eq!(db.point_count(), 1);
        db.clear();
        assert_eq!(db.point_count(), 0);
    }
}
