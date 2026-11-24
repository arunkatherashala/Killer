/// Apache Spark Integration for Killer V2
/// 
/// Full distributed computing ecosystem with DataFrame, SQL, RDD, MLlib, GraphX, and Streaming APIs

pub mod context;
pub mod dataframe;
pub mod graph;
pub mod io;
pub mod io_parallel;
pub mod memory;
pub mod ml;
pub mod optimizer;
pub mod rdd;
pub mod schema;
pub mod sql;
pub mod streaming;

pub use context::{SparkContext, SparkSession};
pub use dataframe::{DataFrame, DataFrameWriter};
pub use graph::Graph;
pub use io::{DataSink, DataSource, FileBuilder, FileFormat};
pub use io_parallel::{IOMetrics, Partition, ParallelDataSink, ParallelDataSource, ThreadPool};
pub use memory::{MemoryManager, MemoryPool, MemoryStats, SpillableCache};
pub use ml::{DecisionTreeModel, KMeansModel, LinearRegressionModel, LogisticRegressionModel, MLlib};
pub use optimizer::{CostEstimator, CostModel, PlanOptimizer, QueryCost, QueryPlan};
pub use rdd::RDD;
pub use schema::{DataType, Field, Schema};
pub use sql::{SQLExecutor, SQLParser, SQLQuery};
pub use streaming::{DStream, RDD as StreamingRDD, StreamingContext};

/// Create a new Spark session
pub fn session() -> SparkSession {
    SparkSession::new()
}

/// Get Spark session builder
pub fn sql(query: &str) -> Result<SQLQuery, String> {
    SQLParser::parse(query)
}

/// Create new Spark context
pub fn context() -> SparkContext {
    SparkContext::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spark_session_creation() {
        let spark = session();
        assert!(!spark.app_name_val().is_empty());
    }

    #[test]
    fn test_sql_parsing() {
        let query = sql("SELECT * FROM users").unwrap();
        assert_eq!(query.from_clause, "users");
    }

    #[test]
    fn test_context_creation() {
        let ctx = context();
        assert!(ctx.parallelism() > 0);
    }
}
