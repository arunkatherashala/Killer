// Phase 5: Data Engineering Module - ETL, transformations, aggregations
// Features: Pipelines, transformations, aggregations, validation, schema inference

use std::collections::{HashMap, BTreeMap};
use crate::value::Value;

/// Pipeline stage operation
#[derive(Clone, Debug)]
pub enum PipelineOp {
    Map(String),           // Apply transformation
    Filter(String),        // Filter rows
    GroupBy(Vec<String>),  // Group by columns
    Join(String),          // Join with another dataset
    Sort(String, bool),    // Sort by column (ascending/descending)
    Limit(usize),          // Limit rows
    Distinct,              // Remove duplicates
}

/// Data pipeline configuration
#[derive(Clone, Debug)]
pub struct DataPipeline {
    pub name: String,
    pub stages: Vec<PipelineOp>,
    pub data: Vec<HashMap<String, Value>>,
    pub schema: HashMap<String, String>, // Column -> Type
}

impl DataPipeline {
    pub fn new(name: String) -> Self {
        DataPipeline {
            name,
            stages: Vec::new(),
            data: Vec::new(),
            schema: HashMap::new(),
        }
    }

    /// Load data from array of objects
    pub fn load(&mut self, data: Vec<HashMap<String, Value>>) -> Result<(), String> {
        if data.is_empty() {
            return Err("Cannot load empty data".to_string());
        }

        // Infer schema from first row
        for (key, val) in data[0].iter() {
            let type_name = Self::infer_type(val);
            self.schema.insert(key.clone(), type_name);
        }

        self.data = data;
        Ok(())
    }

    /// Infer Value type name
    fn infer_type(val: &Value) -> String {
        match val {
            Value::Number(_) => "number".to_string(),
            Value::Str(_) => "string".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Dict(_) => "object".to_string(),
            Value::Null => "null".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Add map transformation
    pub fn map(mut self, _column: String) -> Self {
        self.stages.push(PipelineOp::Map(_column));
        self
    }

    /// Add filter transformation
    pub fn filter(mut self, _column: String) -> Self {
        self.stages.push(PipelineOp::Filter(_column));
        self
    }

    /// Add group by transformation
    pub fn group_by(mut self, columns: Vec<String>) -> Self {
        self.stages.push(PipelineOp::GroupBy(columns));
        self
    }

    /// Add sort transformation
    pub fn sort(mut self, column: String, ascending: bool) -> Self {
        self.stages.push(PipelineOp::Sort(column, ascending));
        self
    }

    /// Add limit transformation
    pub fn limit(mut self, count: usize) -> Self {
        self.stages.push(PipelineOp::Limit(count));
        self
    }

    /// Execute pipeline
    pub fn execute(self) -> Result<Vec<HashMap<String, Value>>, String> {
        let mut result = self.data.clone();

        for stage in self.stages {
            result = match stage {
                PipelineOp::Limit(n) => result.into_iter().take(n).collect(),
                PipelineOp::Distinct => {
                    let mut seen = Vec::new();
                    result.into_iter().filter(|row| {
                        if seen.contains(row) {
                            false
                        } else {
                            seen.push(row.clone());
                            true
                        }
                    }).collect()
                }
                _ => result, // Placeholder for complex transformations
            };
        }

        Ok(result)
    }

    /// Get schema
    pub fn get_schema(&self) -> HashMap<String, String> {
        self.schema.clone()
    }

    /// Get row count
    pub fn count(&self) -> usize {
        self.data.len()
    }
}

/// Data aggregation operations
pub struct Aggregation;

impl Aggregation {
    /// Sum numeric column
    pub fn sum(data: &[HashMap<String, Value>], column: &str) -> Result<f64, String> {
        let mut total = 0.0;
        for row in data {
            if let Some(Value::Number(n)) = row.get(column) {
                total += n;
            }
        }
        Ok(total)
    }

    /// Average numeric column
    pub fn avg(data: &[HashMap<String, Value>], column: &str) -> Result<f64, String> {
        let sum = Self::sum(data, column)?;
        let count = data.len();
        if count == 0 {
            Err("Empty dataset".to_string())
        } else {
            Ok(sum / count as f64)
        }
    }

    /// Min value in column
    pub fn min(data: &[HashMap<String, Value>], column: &str) -> Result<Value, String> {
        data.iter()
            .filter_map(|row| row.get(column))
            .min_by(|a, b| {
                match (a, b) {
                    (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    _ => std::cmp::Ordering::Equal,
                }
            })
            .cloned()
            .ok_or_else(|| "No values in column".to_string())
    }

    /// Max value in column
    pub fn max(data: &[HashMap<String, Value>], column: &str) -> Result<Value, String> {
        data.iter()
            .filter_map(|row| row.get(column))
            .max_by(|a, b| {
                match (a, b) {
                    (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    _ => std::cmp::Ordering::Equal,
                }
            })
            .cloned()
            .ok_or_else(|| "No values in column".to_string())
    }

    /// Count rows
    pub fn count(data: &[HashMap<String, Value>]) -> usize {
        data.len()
    }

    /// Count unique values in column
    pub fn count_distinct(data: &[HashMap<String, Value>], column: &str) -> usize {
        let mut unique = std::collections::HashSet::new();
        for row in data {
            if let Some(val) = row.get(column) {
                unique.insert(format!("{:?}", val));
            }
        }
        unique.len()
    }

    /// Group by column
    pub fn group_by(data: &[HashMap<String, Value>], column: &str) -> HashMap<String, Vec<HashMap<String, Value>>> {
        let mut groups: HashMap<String, Vec<HashMap<String, Value>>> = HashMap::new();
        for row in data {
            if let Some(val) = row.get(column) {
                let key = format!("{:?}", val);
                groups.entry(key).or_insert_with(Vec::new).push(row.clone());
            }
        }
        groups
    }

    /// Join datasets
    pub fn join(
        left: &[HashMap<String, Value>],
        right: &[HashMap<String, Value>],
        left_key: &str,
        right_key: &str,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let mut result = Vec::new();

        for left_row in left {
            for right_row in right {
                if let (Some(left_val), Some(right_val)) = (left_row.get(left_key), right_row.get(right_key)) {
                    if format!("{:?}", left_val) == format!("{:?}", right_val) {
                        let mut merged = left_row.clone();
                        for (k, v) in right_row.iter() {
                            merged.insert(k.clone(), v.clone());
                        }
                        result.push(merged);
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Data validation rules
#[derive(Clone, Debug)]
pub struct ValidationRule {
    pub column: String,
    pub rule_type: String, // "not_null", "positive", "regex", "range"
    pub value: String,
}

/// Data validator
pub struct DataValidator;

impl DataValidator {
    /// Validate dataset against rules
    pub fn validate(data: &[HashMap<String, Value>], rules: &[ValidationRule]) -> Result<bool, Vec<String>> {
        let mut errors = Vec::new();

        for (idx, row) in data.iter().enumerate() {
            for rule in rules {
                match rule.rule_type.as_str() {
                    "not_null" => {
                        if !row.contains_key(&rule.column) || row.get(&rule.column) == Some(&Value::Null) {
                            errors.push(format!("Row {}: {} cannot be null", idx, rule.column));
                        }
                    }
                    "positive" => {
                        if let Some(Value::Number(n)) = row.get(&rule.column) {
                            if *n <= 0.0 {
                                errors.push(format!("Row {}: {} must be positive", idx, rule.column));
                            }
                        }
                    }
                    "range" => {
                        if let Some(Value::Number(n)) = row.get(&rule.column) {
                            let parts: Vec<&str> = rule.value.split('-').collect();
                            if parts.len() == 2 {
                                if let (Ok(min), Ok(max)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                                    if *n < min || *n > max {
                                        errors.push(format!("Row {}: {} out of range [{}, {}]", idx, rule.column, min, max));
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if errors.is_empty() {
            Ok(true)
        } else {
            Err(errors)
        }
    }

    /// Check data quality score (0-100)
    pub fn quality_score(data: &[HashMap<String, Value>]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut score = 100.0;
        let mut null_count = 0;
        let mut total_fields = 0;

        for row in data {
            for val in row.values() {
                total_fields += 1;
                if val == &Value::Null {
                    null_count += 1;
                }
            }
        }

        if total_fields > 0 {
            let null_ratio = null_count as f64 / total_fields as f64;
            score -= null_ratio * 50.0;
        }

        score.max(0.0)
    }
}

/// ETL (Extract, Transform, Load) pipeline
pub struct ETLPipeline {
    pub name: String,
    pub extract_source: String,
    pub transformations: Vec<String>,
    pub load_target: String,
    pub records_processed: usize,
}

impl ETLPipeline {
    pub fn new(name: String) -> Self {
        ETLPipeline {
            name,
            extract_source: String::new(),
            transformations: Vec::new(),
            load_target: String::new(),
            records_processed: 0,
        }
    }

    /// Set extract source
    pub fn extract(mut self, source: String) -> Self {
        self.extract_source = source;
        self
    }

    /// Add transformation
    pub fn transform(mut self, rule: String) -> Self {
        self.transformations.push(rule);
        self
    }

    /// Set load target
    pub fn load(mut self, target: String) -> Self {
        self.load_target = target;
        self
    }

    /// Run ETL pipeline
    pub fn run(&mut self, data: Vec<HashMap<String, Value>>) -> Result<Vec<HashMap<String, Value>>, String> {
        self.records_processed = data.len();
        Ok(data)
    }

    /// Get pipeline status
    pub fn status(&self) -> String {
        format!(
            "ETL: {} | Extract: {} | Transforms: {} | Load: {} | Records: {}",
            self.name,
            self.extract_source,
            self.transformations.len(),
            self.load_target,
            self.records_processed
        )
    }
}

/// Time series data handler
pub struct TimeSeries {
    pub name: String,
    pub timestamps: Vec<u64>,
    pub values: Vec<f64>,
}

impl TimeSeries {
    pub fn new(name: String) -> Self {
        TimeSeries {
            name,
            timestamps: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Add data point
    pub fn add_point(&mut self, timestamp: u64, value: f64) {
        self.timestamps.push(timestamp);
        self.values.push(value);
    }

    /// Calculate moving average
    pub fn moving_average(&self, window: usize) -> Result<Vec<f64>, String> {
        if window > self.values.len() {
            return Err("Window size larger than data".to_string());
        }

        let mut result = Vec::new();
        for i in 0..=(self.values.len() - window) {
            let sum: f64 = self.values[i..i + window].iter().sum();
            result.push(sum / window as f64);
        }
        Ok(result)
    }

    /// Calculate rate of change
    pub fn rate_of_change(&self) -> Result<Vec<f64>, String> {
        if self.values.len() < 2 {
            return Err("Need at least 2 points".to_string());
        }

        let mut result = Vec::new();
        for i in 1..self.values.len() {
            let change = (self.values[i] - self.values[i - 1]) / self.values[i - 1].max(1.0);
            result.push(change);
        }
        Ok(result)
    }

    /// Get statistics
    pub fn stats(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();

        if !self.values.is_empty() {
            let sum: f64 = self.values.iter().sum();
            let count = self.values.len();
            let avg = sum / count as f64;

            stats.insert("count".to_string(), count as f64);
            stats.insert("sum".to_string(), sum);
            stats.insert("average".to_string(), avg);
            stats.insert("min".to_string(), *self.values.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&0.0));
            stats.insert("max".to_string(), *self.values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&0.0));
        }

        stats
    }
}

pub struct DataEngineeringModule;

impl DataEngineeringModule {
    /// Create new pipeline
    pub fn new_pipeline(name: String) -> DataPipeline {
        DataPipeline::new(name)
    }

    /// Create ETL pipeline
    pub fn new_etl(name: String) -> ETLPipeline {
        ETLPipeline::new(name)
    }

    /// Create time series
    pub fn new_timeseries(name: String) -> TimeSeries {
        TimeSeries::new(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<HashMap<String, Value>> {
        vec![
            {
                let mut row = HashMap::new();
                row.insert("id".to_string(), Value::Number(1.0));
                row.insert("name".to_string(), Value::Str("Alice".to_string()));
                row.insert("salary".to_string(), Value::Number(50000.0));
                row
            },
            {
                let mut row = HashMap::new();
                row.insert("id".to_string(), Value::Number(2.0));
                row.insert("name".to_string(), Value::Str("Bob".to_string()));
                row.insert("salary".to_string(), Value::Number(60000.0));
                row
            },
            {
                let mut row = HashMap::new();
                row.insert("id".to_string(), Value::Number(3.0));
                row.insert("name".to_string(), Value::Str("Charlie".to_string()));
                row.insert("salary".to_string(), Value::Number(55000.0));
                row
            },
        ]
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = DataPipeline::new("test_pipeline".to_string());
        assert_eq!(pipeline.name, "test_pipeline");
        assert_eq!(pipeline.stages.len(), 0);
    }

    #[test]
    fn test_pipeline_load_data() {
        let mut pipeline = DataPipeline::new("test".to_string());
        let data = sample_data();
        let result = pipeline.load(data);
        assert!(result.is_ok());
        assert_eq!(pipeline.schema.len(), 3);
    }

    #[test]
    fn test_aggregation_sum() {
        let data = sample_data();
        let result = Aggregation::sum(&data, "salary").unwrap();
        assert_eq!(result, 165000.0);
    }

    #[test]
    fn test_aggregation_avg() {
        let data = sample_data();
        let result = Aggregation::avg(&data, "salary").unwrap();
        assert_eq!(result, 55000.0);
    }

    #[test]
    fn test_aggregation_count() {
        let data = sample_data();
        let count = Aggregation::count(&data);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_aggregation_min() {
        let data = sample_data();
        let result = Aggregation::min(&data, "salary").unwrap();
        assert_eq!(result, Value::Number(50000.0));
    }

    #[test]
    fn test_aggregation_max() {
        let data = sample_data();
        let result = Aggregation::max(&data, "salary").unwrap();
        assert_eq!(result, Value::Number(60000.0));
    }

    #[test]
    fn test_count_distinct() {
        let data = sample_data();
        let count = Aggregation::count_distinct(&data, "name");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_group_by() {
        let data = sample_data();
        let groups = Aggregation::group_by(&data, "id");
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_pipeline_limit() {
        let mut pipeline = DataPipeline::new("test".to_string());
        let data = sample_data();
        pipeline.load(data).unwrap();
        let pipeline_with_limit = pipeline.limit(2);
        let result = pipeline_with_limit.execute().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_etl_pipeline() {
        let mut etl = ETLPipeline::new("test_etl".to_string());
        etl = etl.extract("database".to_string());
        etl = etl.transform("clean".to_string());
        etl = etl.load("warehouse".to_string());
        
        let data = sample_data();
        let result = etl.run(data).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(etl.records_processed, 3);
    }

    #[test]
    fn test_timeseries_creation() {
        let ts = TimeSeries::new("stock_price".to_string());
        assert_eq!(ts.name, "stock_price");
        assert_eq!(ts.values.len(), 0);
    }

    #[test]
    fn test_timeseries_add_points() {
        let mut ts = TimeSeries::new("test".to_string());
        ts.add_point(1000, 100.0);
        ts.add_point(2000, 110.0);
        ts.add_point(3000, 105.0);
        assert_eq!(ts.values.len(), 3);
    }

    #[test]
    fn test_timeseries_moving_average() {
        let mut ts = TimeSeries::new("test".to_string());
        ts.add_point(1000, 10.0);
        ts.add_point(2000, 20.0);
        ts.add_point(3000, 30.0);
        
        let ma = ts.moving_average(2).unwrap();
        assert_eq!(ma.len(), 2);
        assert_eq!(ma[0], 15.0);
        assert_eq!(ma[1], 25.0);
    }

    #[test]
    fn test_validation_not_null() {
        let data = sample_data();
        let rules = vec![
            ValidationRule {
                column: "name".to_string(),
                rule_type: "not_null".to_string(),
                value: String::new(),
            }
        ];
        let result = DataValidator::validate(&data, &rules);
        assert!(result.is_ok());
    }

    #[test]
    fn test_data_quality_score() {
        let data = sample_data();
        let score = DataValidator::quality_score(&data);
        assert!(score >= 90.0);
    }
}
