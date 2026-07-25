/// DataFrame - Distributed immutable collection
/// 
/// Lazy evaluation: transformations are not executed until an action is called.
/// Chainable API for fluent operations.

use std::sync::Arc;
use crate::value::Value;
use crate::spark::schema::{Schema, Field, DataType};

/// Row represents a single record in a DataFrame
pub type Row = Vec<Value>;

/// DataFrame configuration options
#[derive(Debug, Clone)]
pub struct DataFrameConfig {
    pub partitions: usize,
    pub cache_enabled: bool,
    pub broadcast_threshold: usize,
}

impl Default for DataFrameConfig {
    fn default() -> Self {
        Self {
            partitions: 8,
            cache_enabled: true,
            broadcast_threshold: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Lazy evaluation plan for DataFrame operations
#[derive(Debug, Clone)]
pub enum Operation {
    /// Source operation
    Source(Vec<Row>),
    
    /// Map operation: transform each row
    Map {
        source: Box<Operation>,
        func: String, // Function name/identifier
    },
    
    /// Filter operation: keep rows matching predicate
    Filter {
        source: Box<Operation>,
        predicate: String, // Predicate expression
    },
    
    /// GroupBy operation
    GroupBy {
        source: Box<Operation>,
        keys: Vec<String>,
    },
    
    /// Aggregation operation
    Aggregate {
        source: Box<Operation>,
        aggs: Vec<(String, String)>, // (column, operation)
    },
    
    /// Join operation
    Join {
        left: Box<Operation>,
        right: Box<Operation>,
        on: String,
        join_type: String, // inner, left, right, outer
    },
    
    /// OrderBy operation
    OrderBy {
        source: Box<Operation>,
        by: Vec<(String, bool)>, // (column, ascending)
    },
    
    /// Select specific columns
    Select {
        source: Box<Operation>,
        columns: Vec<String>,
    },
}

/// Main DataFrame abstraction
#[derive(Clone)]
pub struct DataFrame {
    schema: Schema,
    operation: Operation,
    config: DataFrameConfig,
}

impl DataFrame {
    /// Create a new DataFrame from raw data
    pub fn new(schema: Schema, data: Vec<Row>) -> Self {
        Self {
            schema,
            operation: Operation::Source(data),
            config: DataFrameConfig::default(),
        }
    }

    /// Create empty DataFrame
    pub fn empty(schema: Schema) -> Self {
        Self {
            schema,
            operation: Operation::Source(Vec::new()),
            config: DataFrameConfig::default(),
        }
    }

    /// Get schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Get number of partitions
    pub fn partitions(&self) -> usize {
        self.config.partitions
    }

    /// Set number of partitions
    pub fn repartition(mut self, partitions: usize) -> Self {
        self.config.partitions = partitions;
        self
    }

    /// Select specific columns
    pub fn select(&self, columns: &[&str]) -> Result<DataFrame, String> {
        let new_schema = self.schema.select(columns)?;
        
        Ok(DataFrame {
            schema: new_schema,
            operation: Operation::Select {
                source: Box::new(self.operation.clone()),
                columns: columns.iter().map(|s| s.to_string()).collect(),
            },
            config: self.config.clone(),
        })
    }

    /// Filter rows based on predicate
    pub fn filter(&self, predicate: &str) -> DataFrame {
        DataFrame {
            schema: self.schema.clone(),
            operation: Operation::Filter {
                source: Box::new(self.operation.clone()),
                predicate: predicate.to_string(),
            },
            config: self.config.clone(),
        }
    }

    /// Map transformation (transform each row)
    pub fn map(&self, func: &str) -> DataFrame {
        DataFrame {
            schema: self.schema.clone(),
            operation: Operation::Map {
                source: Box::new(self.operation.clone()),
                func: func.to_string(),
            },
            config: self.config.clone(),
        }
    }

    /// GroupBy operation
    pub fn group_by(&self, keys: &[&str]) -> DataFrame {
        DataFrame {
            schema: self.schema.clone(),
            operation: Operation::GroupBy {
                source: Box::new(self.operation.clone()),
                keys: keys.iter().map(|s| s.to_string()).collect(),
            },
            config: self.config.clone(),
        }
    }

    /// OrderBy operation
    pub fn order_by(&self, columns: &[&str]) -> DataFrame {
        let by: Vec<_> = columns.iter()
            .map(|c| {
                let desc = c.ends_with("_desc");
                let col_name = if desc { &c[..c.len()-5] } else { c };
                (col_name.to_string(), !desc)
            })
            .collect();

        DataFrame {
            schema: self.schema.clone(),
            operation: Operation::OrderBy {
                source: Box::new(self.operation.clone()),
                by,
            },
            config: self.config.clone(),
        }
    }

    /// Get DataFrame info
    pub fn info(&self) -> String {
        format!(
            "DataFrame: {} columns, {} partitions\nSchema:\n{}",
            self.schema.len(),
            self.partitions(),
            self.schema
        )
    }

    /// Show first N rows
    pub fn show(&self, n: usize) -> String {
        match &self.operation {
            Operation::Source(rows) => {
                let mut output = String::new();
                output.push_str(&format!("Showing {} rows:\n", n.min(rows.len())));
                for (i, row) in rows.iter().take(n).enumerate() {
                    output.push_str(&format!("Row {}: {:?}\n", i, row));
                }
                output
            }
            _ => "Show not yet implemented for complex operations".to_string(),
        }
    }

    /// Count rows (action - triggers execution)
    pub fn count(&self) -> Result<usize, String> {
        match &self.operation {
            Operation::Source(rows) => Ok(rows.len()),
            _ => Err("Count not yet implemented for complex operations".to_string()),
        }
    }

    /// Collect all data to driver (action - triggers execution)
    pub fn collect(&self) -> Result<Vec<Row>, String> {
        match &self.operation {
            Operation::Source(rows) => Ok(rows.clone()),
            _ => Err("Collect not yet implemented for complex operations".to_string()),
        }
    }

    /// Enable caching (persist in memory)
    pub fn cache(&mut self) -> &mut Self {
        self.config.cache_enabled = true;
        self
    }

    /// Create temporary view
    pub fn create_temp_view(&self, name: &str) -> Result<(), String> {
        // Would register this DataFrame as a SQL table
        Ok(())
    }

    /// Write DataFrame to storage
    pub fn write(&self) -> DataFrameWriter {
        DataFrameWriter::new(self.clone())
    }

    /// Get column names
    pub fn column_names(&self) -> Vec<String> {
        self.schema
            .fields()
            .iter()
            .map(|f| f.name.clone())
            .collect()
    }
}

impl std::fmt::Debug for DataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataFrame")
            .field("schema", &self.schema)
            .field("partitions", &self.config.partitions)
            .field("cache_enabled", &self.config.cache_enabled)
            .finish()
    }
}

/// DataFrame Writer for saving data
pub struct DataFrameWriter {
    df: DataFrame,
    mode: String,
    format: String,
    path: String,
}

impl DataFrameWriter {
    pub fn new(df: DataFrame) -> Self {
        Self {
            df,
            mode: "error".to_string(),
            format: "parquet".to_string(),
            path: String::new(),
        }
    }

    /// Set write mode (overwrite, append, ignore, error)
    pub fn mode(mut self, mode: &str) -> Self {
        self.mode = mode.to_string();
        self
    }

    /// Write as CSV
    pub fn csv(mut self, path: &str) -> Result<(), String> {
        self.format = "csv".to_string();
        self.path = path.to_string();
        self.save()
    }

    /// Write as Parquet
    pub fn parquet(mut self, path: &str) -> Result<(), String> {
        self.format = "parquet".to_string();
        self.path = path.to_string();
        self.save()
    }

    /// Write as JSON
    pub fn json(mut self, path: &str) -> Result<(), String> {
        self.format = "json".to_string();
        self.path = path.to_string();
        self.save()
    }

    /// Save the DataFrame
    pub fn save(&self) -> Result<(), String> {
        println!(
            "DataFrameWriter: saving {} rows as {} to {}",
            self.df.count()?,
            self.format,
            self.path
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataframe_creation() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
        ]);
        let df = DataFrame::empty(schema);
        assert_eq!(df.schema().len(), 2);
    }

    #[test]
    fn test_dataframe_with_data() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
        ]);
        let data = vec![
            vec![Value::Str("Alice".to_string()), Value::Number(25.0)],
            vec![Value::Str("Bob".to_string()), Value::Number(30.0)],
        ];
        let df = DataFrame::new(schema, data);
        assert_eq!(df.count().unwrap(), 2);
    }

    #[test]
    fn test_dataframe_select() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
        ]);
        let df = DataFrame::empty(schema);
        let selected = df.select(&["name"]).unwrap();
        assert_eq!(selected.schema().len(), 1);
    }

    #[test]
    fn test_column_names() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
        ]);
        let df = DataFrame::empty(schema);
        let names = df.column_names();
        assert_eq!(names, vec!["name", "age"]);
    }

    #[test]
    fn test_repartition() {
        let schema = Schema::from_tuples(vec![("id", DataType::Int64)]);
        let df = DataFrame::empty(schema).repartition(16);
        assert_eq!(df.partitions(), 16);
    }
}
