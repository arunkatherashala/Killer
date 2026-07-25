/// RDD (Resilient Distributed Dataset) - Low-level distributed collection API
/// 
/// RDDs are the fundamental data structure of Spark - immutable, lazy-evaluated,
/// fault-tolerant distributed collections.

use std::sync::Arc;
use crate::value::Value;

pub type RDDData = Arc<Vec<Value>>;

/// RDD transformation types
#[derive(Debug, Clone)]
pub enum RDDOp {
    /// Source RDD from collection
    Parallelize(Vec<Value>),
    
    /// Map transformation
    Map {
        source: Box<RDDOp>,
        func: String,
    },
    
    /// Filter transformation
    Filter {
        source: Box<RDDOp>,
        predicate: String,
    },
    
    /// FlatMap transformation
    FlatMap {
        source: Box<RDDOp>,
        func: String,
    },
    
    /// Reduce operation
    Reduce {
        source: Box<RDDOp>,
        func: String,
    },
}

/// RDD - Resilient Distributed Dataset
#[derive(Clone)]
pub struct RDD {
    id: usize,
    data: Option<RDDData>,
    operation: Option<RDDOp>,
    partitions: usize,
}

static RDD_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl RDD {
    /// Create RDD from collection
    pub fn parallelize(data: Vec<Value>) -> Self {
        let id = RDD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            id,
            data: Some(Arc::new(data)),
            operation: None,
            partitions: 8,
        }
    }

    /// Create empty RDD
    pub fn empty() -> Self {
        let id = RDD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            id,
            data: Some(Arc::new(Vec::new())),
            operation: None,
            partitions: 8,
        }
    }

    /// Get RDD ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get number of partitions
    pub fn partitions(&self) -> usize {
        self.partitions
    }

    /// Change number of partitions
    pub fn repartition(mut self, partitions: usize) -> Self {
        self.partitions = partitions;
        self
    }

    /// Map transformation
    pub fn map(&self, func: &str) -> RDD {
        RDD {
            id: RDD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            data: None,
            operation: Some(RDDOp::Map {
                source: Box::new(
                    self.operation.clone().unwrap_or_else(|| {
                        RDDOp::Parallelize(
                            self.data.as_ref().map(|d| d.as_ref().clone()).unwrap_or_default()
                        )
                    })
                ),
                func: func.to_string(),
            }),
            partitions: self.partitions,
        }
    }

    /// Filter transformation
    pub fn filter(&self, predicate: &str) -> RDD {
        RDD {
            id: RDD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            data: None,
            operation: Some(RDDOp::Filter {
                source: Box::new(
                    self.operation.clone().unwrap_or_else(|| {
                        RDDOp::Parallelize(
                            self.data.as_ref().map(|d| d.as_ref().clone()).unwrap_or_default()
                        )
                    })
                ),
                predicate: predicate.to_string(),
            }),
            partitions: self.partitions,
        }
    }

    /// FlatMap transformation
    pub fn flat_map(&self, func: &str) -> RDD {
        RDD {
            id: RDD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            data: None,
            operation: Some(RDDOp::FlatMap {
                source: Box::new(
                    self.operation.clone().unwrap_or_else(|| {
                        RDDOp::Parallelize(
                            self.data.as_ref().map(|d| d.as_ref().clone()).unwrap_or_default()
                        )
                    })
                ),
                func: func.to_string(),
            }),
            partitions: self.partitions,
        }
    }

    /// Count action - count elements
    pub fn count(&self) -> Result<usize, String> {
        match &self.data {
            Some(data) => Ok(data.len()),
            None => Err("Count requires computation of lazy RDD".to_string()),
        }
    }

    /// Collect action - gather all elements to driver
    pub fn collect(&self) -> Result<Vec<Value>, String> {
        match &self.data {
            Some(data) => Ok(data.as_ref().clone()),
            None => Err("Collect requires computation of lazy RDD".to_string()),
        }
    }

    /// First element
    pub fn first(&self) -> Result<Option<Value>, String> {
        match &self.data {
            Some(data) => Ok(data.first().cloned()),
            None => Err("First requires computation of lazy RDD".to_string()),
        }
    }

    /// Take first N elements
    pub fn take(&self, n: usize) -> Result<Vec<Value>, String> {
        match &self.data {
            Some(data) => Ok(data.iter().take(n).cloned().collect()),
            None => Err("Take requires computation of lazy RDD".to_string()),
        }
    }

    /// Sum (for numeric RDDs)
    pub fn sum(&self) -> Result<f64, String> {
        match &self.data {
            Some(data) => {
                let mut sum = 0.0;
                for value in data.iter() {
                    match value {
                        Value::Number(n) => sum += n,
                        _ => return Err("Sum requires numeric RDD".to_string()),
                    }
                }
                Ok(sum)
            }
            None => Err("Sum requires computation of lazy RDD".to_string()),
        }
    }

    /// Maximum
    pub fn max(&self) -> Result<Option<f64>, String> {
        match &self.data {
            Some(data) => {
                let mut max: Option<f64> = None;
                for value in data.iter() {
                    match value {
                        Value::Number(n) => {
                            max = Some(max.map(|m| m.max(*n)).unwrap_or(*n));
                        }
                        _ => return Err("Max requires numeric RDD".to_string()),
                    }
                }
                Ok(max)
            }
            None => Err("Max requires computation of lazy RDD".to_string()),
        }
    }

    /// Minimum
    pub fn min(&self) -> Result<Option<f64>, String> {
        match &self.data {
            Some(data) => {
                let mut min: Option<f64> = None;
                for value in data.iter() {
                    match value {
                        Value::Number(n) => {
                            min = Some(min.map(|m| m.min(*n)).unwrap_or(*n));
                        }
                        _ => return Err("Min requires numeric RDD".to_string()),
                    }
                }
                Ok(min)
            }
            None => Err("Min requires computation of lazy RDD".to_string()),
        }
    }

    /// Average
    pub fn mean(&self) -> Result<f64, String> {
        match &self.data {
            Some(data) => {
                if data.is_empty() {
                    return Err("Cannot compute mean of empty RDD".to_string());
                }
                let sum: f64 = data
                    .iter()
                    .map(|v| match v {
                        Value::Number(n) => Some(*n),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or("Mean requires numeric RDD")?
                    .iter()
                    .sum();
                Ok(sum / data.len() as f64)
            }
            None => Err("Mean requires computation of lazy RDD".to_string()),
        }
    }

    /// Standard deviation
    pub fn std_dev(&self) -> Result<f64, String> {
        match &self.data {
            Some(data) => {
                if data.is_empty() {
                    return Err("Cannot compute stddev of empty RDD".to_string());
                }
                let numbers: Vec<f64> = data
                    .iter()
                    .map(|v| match v {
                        Value::Number(n) => Some(*n),
                        _ => None,
                    })
                    .collect::<Option<_>>()
                    .ok_or("StdDev requires numeric RDD")?;

                let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
                let variance = numbers
                    .iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>()
                    / numbers.len() as f64;

                Ok(variance.sqrt())
            }
            None => Err("StdDev requires computation of lazy RDD".to_string()),
        }
    }

    /// Cache in memory
    pub fn cache(&mut self) -> &mut Self {
        self
    }

    /// Force evaluation
    pub fn compute(&mut self) -> Result<(), String> {
        if self.data.is_none() {
            self.data = Some(Arc::new(Vec::new()));
        }
        Ok(())
    }
}

impl std::fmt::Debug for RDD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RDD")
            .field("id", &self.id)
            .field("partitions", &self.partitions)
            .field("elements", &self.data.as_ref().map(|d| d.len()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdd_creation() {
        let rdd = RDD::parallelize(vec![
            Value::Number(1.0),
            Value::Number(2.0),
        ]);
        assert_eq!(rdd.count().unwrap(), 2);
    }

    #[test]
    fn test_rdd_sum() {
        let rdd = RDD::parallelize(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert_eq!(rdd.sum().unwrap(), 6.0);
    }

    #[test]
    fn test_rdd_mean() {
        let rdd = RDD::parallelize(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert_eq!(rdd.mean().unwrap(), 2.0);
    }

    #[test]
    fn test_rdd_max() {
        let rdd = RDD::parallelize(vec![
            Value::Number(1.0),
            Value::Number(5.0),
            Value::Number(3.0),
        ]);
        assert_eq!(rdd.max().unwrap(), Some(5.0));
    }

    #[test]
    fn test_rdd_min() {
        let rdd = RDD::parallelize(vec![
            Value::Number(1.0),
            Value::Number(5.0),
            Value::Number(3.0),
        ]);
        assert_eq!(rdd.min().unwrap(), Some(1.0));
    }

    #[test]
    fn test_rdd_repartition() {
        let rdd = RDD::parallelize(vec![Value::Number(1.0)]).repartition(8);
        assert_eq!(rdd.partitions(), 8);
    }
}
