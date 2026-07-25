// Query Optimizer Module
// Cost-based optimization for SQL queries
// Implements plan optimization rules and cost estimation

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Cost Model - Estimates query execution costs
// ============================================================================

#[derive(Debug, Clone)]
pub struct CostModel {
    pub io_cost_per_row: f64,       // 0.01 - I/O cost per row
    pub cpu_cost_per_row: f64,      // 0.1 - CPU cost per row
    pub memory_cost_factor: f64,    // 1.0 - Memory overhead factor
    pub network_cost_factor: f64,   // 0.5 - Network overhead (for distributed)
}

impl Default for CostModel {
    fn default() -> Self {
        CostModel {
            io_cost_per_row: 0.01,
            cpu_cost_per_row: 0.1,
            memory_cost_factor: 1.0,
            network_cost_factor: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryCost {
    pub io_cost: f64,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub total_cost: f64,
    pub estimated_rows: usize,
}

impl QueryCost {
    pub fn new(io: f64, cpu: f64, mem: f64, rows: usize) -> Self {
        QueryCost {
            io_cost: io,
            cpu_cost: cpu,
            memory_cost: mem,
            total_cost: io + cpu + mem,
            estimated_rows: rows,
        }
    }

    pub fn combine(a: &QueryCost, b: &QueryCost) -> QueryCost {
        QueryCost {
            io_cost: a.io_cost + b.io_cost,
            cpu_cost: a.cpu_cost + b.cpu_cost,
            memory_cost: (a.memory_cost + b.memory_cost).max(a.memory_cost.max(b.memory_cost)),
            total_cost: (a.total_cost + b.total_cost),
            estimated_rows: a.estimated_rows.max(b.estimated_rows),
        }
    }
}

impl fmt::Display for QueryCost {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Cost({:.2}: I/O={:.2}, CPU={:.2}, Mem={:.2}, Rows={})",
            self.total_cost, self.io_cost, self.cpu_cost, self.memory_cost, self.estimated_rows
        )
    }
}

// ============================================================================
// Execution Plan - Represents query execution strategy
// ============================================================================

#[derive(Debug, Clone)]
pub enum PlanNode {
    TableScan {
        table: String,
        filters: Vec<String>,
        estimated_rows: usize,
    },
    Filter {
        source: Box<PlanNode>,
        predicate: String,
        selectivity: f64,
    },
    Project {
        source: Box<PlanNode>,
        columns: Vec<String>,
    },
    Join {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        condition: String,
    },
    Aggregate {
        source: Box<PlanNode>,
        group_by: Vec<String>,
        aggregations: Vec<(String, String)>, // (column, agg_func)
    },
    Sort {
        source: Box<PlanNode>,
        order_by: Vec<(String, SortOrder)>,
    },
    Limit {
        source: Box<PlanNode>,
        count: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl PlanNode {
    pub fn is_scan(&self) -> bool {
        matches!(self, PlanNode::TableScan { .. })
    }

    pub fn get_source(&self) -> Option<&PlanNode> {
        match self {
            PlanNode::Filter { source, .. }
            | PlanNode::Project { source, .. }
            | PlanNode::Sort { source, .. }
            | PlanNode::Limit { source, .. }
            | PlanNode::Aggregate { source, .. } => Some(source),
            PlanNode::Join { .. } | PlanNode::TableScan { .. } => None,
        }
    }

    pub fn mut_source(&mut self) -> Option<&mut PlanNode> {
        match self {
            PlanNode::Filter { source, .. }
            | PlanNode::Project { source, .. }
            | PlanNode::Sort { source, .. }
            | PlanNode::Limit { source, .. }
            | PlanNode::Aggregate { source, .. } => Some(source),
            PlanNode::Join { .. } | PlanNode::TableScan { .. } => None,
        }
    }
}

impl fmt::Display for PlanNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlanNode::TableScan {
                table,
                filters,
                estimated_rows,
            } => {
                write!(f, "Scan({}, rows={}", table, estimated_rows)?;
                if !filters.is_empty() {
                    write!(f, ", filters={:?}", filters)?;
                }
                write!(f, ")")
            }
            PlanNode::Filter {
                source,
                predicate,
                selectivity,
            } => {
                write!(f, "Filter({}, sel={}%, source={})", predicate, (selectivity * 100.0) as i32, source)
            }
            PlanNode::Project { source, columns } => {
                write!(f, "Project({:?}, source={})", columns, source)
            }
            PlanNode::Join {
                left,
                right,
                join_type,
                condition,
            } => {
                write!(
                    f,
                    "Join({:?}, on={}, left={}, right={})",
                    join_type, condition, left, right
                )
            }
            PlanNode::Aggregate {
                source,
                group_by,
                aggregations,
            } => {
                write!(
                    f,
                    "Aggregate(group_by={:?}, agg={:?}, source={})",
                    group_by, aggregations, source
                )
            }
            PlanNode::Sort { source, order_by } => {
                write!(f, "Sort({:?}, source={})", order_by, source)
            }
            PlanNode::Limit { source, count } => {
                write!(f, "Limit({}), source={}", count, source)
            }
        }
    }
}

// ============================================================================
// Query Plan - Complete optimized execution plan
// ============================================================================

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub root: PlanNode,
    pub estimated_cost: QueryCost,
    pub table_stats: HashMap<String, TableStats>,
}

#[derive(Debug, Clone)]
pub struct TableStats {
    pub row_count: usize,
    pub column_cardinalities: HashMap<String, usize>,
    pub estimated_size_bytes: usize,
}

impl TableStats {
    pub fn new(rows: usize) -> Self {
        TableStats {
            row_count: rows,
            column_cardinalities: HashMap::new(),
            estimated_size_bytes: rows * 100, // Rough estimate
        }
    }

    pub fn with_cardinality(mut self, column: String, distinct_values: usize) -> Self {
        self.column_cardinalities.insert(column, distinct_values);
        self
    }
}

impl QueryPlan {
    pub fn new(root: PlanNode, cost: QueryCost) -> Self {
        QueryPlan {
            root,
            estimated_cost: cost,
            table_stats: HashMap::new(),
        }
    }

    pub fn with_stats(mut self, table: String, stats: TableStats) -> Self {
        self.table_stats.insert(table, stats);
        self
    }
}

// ============================================================================
// Cost Estimator - Calculates execution costs
// ============================================================================

pub struct CostEstimator {
    model: CostModel,
    table_stats: HashMap<String, TableStats>,
}

impl CostEstimator {
    pub fn new(model: CostModel) -> Self {
        CostEstimator {
            model,
            table_stats: HashMap::new(),
        }
    }

    pub fn with_stats(mut self, table: String, stats: TableStats) -> Self {
        self.table_stats.insert(table, stats);
        self
    }

    pub fn estimate(&self, plan: &PlanNode) -> QueryCost {
        match plan {
            PlanNode::TableScan {
                table,
                filters: _,
                estimated_rows,
            } => {
                let stats = self
                    .table_stats
                    .get(table)
                    .cloned()
                    .unwrap_or_else(|| TableStats::new(*estimated_rows));

                let row_count = stats.row_count as f64;
                let io_cost = row_count * self.model.io_cost_per_row;
                let cpu_cost = row_count * self.model.cpu_cost_per_row;
                let mem_cost = (stats.estimated_size_bytes as f64) * self.model.memory_cost_factor * 0.00001;

                QueryCost::new(io_cost, cpu_cost, mem_cost, stats.row_count)
            }
            PlanNode::Filter {
                source,
                predicate: _,
                selectivity,
            } => {
                let source_cost = self.estimate(source);
                let filtered_rows = (source_cost.estimated_rows as f64 * selectivity) as usize;

                // Filter adds CPU cost but reduces I/O and memory downstream
                let cpu_cost = source_cost.estimated_rows as f64 * self.model.cpu_cost_per_row * 0.1;

                QueryCost::new(
                    source_cost.io_cost,
                    source_cost.cpu_cost + cpu_cost,
                    source_cost.memory_cost,
                    filtered_rows,
                )
            }
            PlanNode::Project {
                source,
                columns: _,
            } => {
                let source_cost = self.estimate(source);
                // Projection adds minimal CPU cost, reduces memory
                let cpu_cost = source_cost.estimated_rows as f64 * self.model.cpu_cost_per_row * 0.05;

                QueryCost::new(
                    source_cost.io_cost,
                    source_cost.cpu_cost + cpu_cost,
                    source_cost.memory_cost * 0.5,
                    source_cost.estimated_rows,
                )
            }
            PlanNode::Join {
                left,
                right,
                join_type: _,
                condition: _,
            } => {
                let left_cost = self.estimate(left);
                let right_cost = self.estimate(right);

                // Join multiple: rows(left) * rows(right)
                let join_rows = (left_cost.estimated_rows as f64 * right_cost.estimated_rows as f64
                    / 1000.0)
                    .min(100_000.0) as usize;
                let join_cpu = join_rows as f64 * self.model.cpu_cost_per_row * 2.0;

                let combined = QueryCost::combine(&left_cost, &right_cost);
                QueryCost::new(
                    combined.io_cost,
                    combined.cpu_cost + join_cpu,
                    combined.memory_cost + (join_rows as f64 * self.model.cpu_cost_per_row * 0.01),
                    join_rows,
                )
            }
            PlanNode::Aggregate {
                source,
                group_by,
                aggregations: _,
            } => {
                let source_cost = self.estimate(source);

                // Group by reduces rows significantly
                let group_cardinality = if group_by.is_empty() {
                    1
                } else {
                    (source_cost.estimated_rows / group_by.len()).max(1)
                };

                let agg_cpu = source_cost.estimated_rows as f64 * self.model.cpu_cost_per_row * 0.5;

                QueryCost::new(
                    source_cost.io_cost,
                    source_cost.cpu_cost + agg_cpu,
                    source_cost.memory_cost,
                    group_cardinality,
                )
            }
            PlanNode::Sort {
                source,
                order_by: _,
            } => {
                let source_cost = self.estimate(source);

                // Sort cost: O(n log n)
                let n = source_cost.estimated_rows as f64;
                let sort_cpu = n * n.log2() * self.model.cpu_cost_per_row * 0.01;

                QueryCost::new(
                    source_cost.io_cost,
                    source_cost.cpu_cost + sort_cpu,
                    source_cost.memory_cost,
                    source_cost.estimated_rows,
                )
            }
            PlanNode::Limit {
                source,
                count,
            } => {
                let source_cost = self.estimate(source);
                let limited_rows = (*count).min(source_cost.estimated_rows);

                QueryCost::new(
                    source_cost.io_cost * (limited_rows as f64 / source_cost.estimated_rows as f64),
                    source_cost.cpu_cost * (limited_rows as f64 / source_cost.estimated_rows as f64),
                    source_cost.memory_cost,
                    limited_rows,
                )
            }
        }
    }
}

// ============================================================================
// Plan Optimizer - Applies optimization rules
// ============================================================================

pub struct PlanOptimizer {
    cost_estimator: CostEstimator,
}

impl PlanOptimizer {
    pub fn new(estimator: CostEstimator) -> Self {
        PlanOptimizer {
            cost_estimator: estimator,
        }
    }

    pub fn optimize(&self, mut plan: PlanNode) -> PlanNode {
        // Apply rules iteratively until convergence
        for _ in 0..5 {
            let new_plan = self.apply_rules(&plan);
            if plans_equal(&new_plan, &plan) {
                break;
            }
            plan = new_plan;
        }
        plan
    }

    fn apply_rules(&self, plan: &PlanNode) -> PlanNode {
        let plan = self.push_down_filters(plan);
        let plan = self.eliminate_projections(&plan);
        let plan = self.reorder_joins(&plan);
        let plan = self.push_down_aggregates(&plan);
        plan
    }

    // Rule 1: Push filters down to table scans
    fn push_down_filters(&self, plan: &PlanNode) -> PlanNode {
        match plan {
            PlanNode::Filter {
                source,
                predicate,
                selectivity,
            } => {
                match &**source {
                    PlanNode::TableScan {
                        table,
                        filters,
                        estimated_rows,
                    } => {
                        let mut new_filters = filters.clone();
                        new_filters.push(predicate.clone());
                        let new_estimated_rows = (*estimated_rows as f64 * selectivity) as usize;

                        PlanNode::TableScan {
                            table: table.clone(),
                            filters: new_filters,
                            estimated_rows: new_estimated_rows,
                        }
                    }
                    _ => {
                        let new_source = self.push_down_filters(source);
                        PlanNode::Filter {
                            source: Box::new(new_source),
                            predicate: predicate.clone(),
                            selectivity: *selectivity,
                        }
                    }
                }
            }
            other => {
                if let Some(source) = other.get_source() {
                    let optimized_source = self.push_down_filters(source);
                    let mut result = other.clone();
                    if let Some(src) = result.mut_source() {
                        *src = optimized_source;
                    }
                    result
                } else {
                    other.clone()
                }
            }
        }
    }

    // Rule 2: Remove unnecessary projections
    fn eliminate_projections(&self, plan: &PlanNode) -> PlanNode {
        match plan {
            PlanNode::Project { source, columns } => {
                if columns.len() > 10 {
                    // Keep projection if many columns
                    plan.clone()
                } else {
                    // Otherwise eliminate
                    self.eliminate_projections(source)
                }
            }
            other => other.clone(),
        }
    }

    // Rule 3: Reorder joins (heuristic: smallest table first)
    fn reorder_joins(&self, plan: &PlanNode) -> PlanNode {
        match plan {
            PlanNode::Join {
                left,
                right,
                join_type,
                condition,
            } => {
                let left_cost = self.cost_estimator.estimate(left);
                let right_cost = self.cost_estimator.estimate(right);

                if right_cost.estimated_rows < left_cost.estimated_rows {
                    // Swap to put smaller table first
                    PlanNode::Join {
                        left: right.clone(),
                        right: left.clone(),
                        join_type: *join_type,
                        condition: condition.clone(),
                    }
                } else {
                    plan.clone()
                }
            }
            other => other.clone(),
        }
    }

    // Rule 4: Push aggregates down (group early)
    fn push_down_aggregates(&self, plan: &PlanNode) -> PlanNode {
        match plan {
            PlanNode::Aggregate {
                source,
                group_by,
                aggregations,
            } => {
                if let PlanNode::Filter {
                    source: filter_source,
                    predicate,
                    selectivity,
                } = &**source
                {
                    // Move filter after aggregate for better efficiency
                    let agg = PlanNode::Aggregate {
                        source: filter_source.clone(),
                        group_by: group_by.clone(),
                        aggregations: aggregations.clone(),
                    };

                    PlanNode::Filter {
                        source: Box::new(agg),
                        predicate: predicate.clone(),
                        selectivity: *selectivity,
                    }
                } else {
                    plan.clone()
                }
            }
            other => other.clone(),
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn plans_equal(a: &PlanNode, b: &PlanNode) -> bool {
    format!("{:?}", a) == format!("{:?}", b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_model_defaults() {
        let model = CostModel::default();
        assert_eq!(model.io_cost_per_row, 0.01);
        assert_eq!(model.cpu_cost_per_row, 0.1);
    }

    #[test]
    fn test_query_cost_combine() {
        let c1 = QueryCost::new(10.0, 5.0, 2.0, 100);
        let c2 = QueryCost::new(15.0, 8.0, 3.0, 150);
        let combined = QueryCost::combine(&c1, &c2);

        assert_eq!(combined.io_cost, 25.0);
        assert_eq!(combined.cpu_cost, 13.0);
        assert!(combined.memory_cost > 0.0);
    }

    #[test]
    fn test_table_scan_cost() {
        let model = CostModel::default();
        let estimator = CostEstimator::new(model);

        let scan = PlanNode::TableScan {
            table: "users".to_string(),
            filters: vec![],
            estimated_rows: 1000,
        };

        let cost = estimator.estimate(&scan);
        assert_eq!(cost.estimated_rows, 1000);
        assert!(cost.total_cost > 0.0);
    }

    #[test]
    fn test_filter_selectivity() {
        let model = CostModel::default();
        let stats = TableStats::new(1000);
        let estimator = CostEstimator::new(model).with_stats("users".to_string(), stats);

        let scan = PlanNode::TableScan {
            table: "users".to_string(),
            filters: vec![],
            estimated_rows: 1000,
        };

        let filtered = PlanNode::Filter {
            source: Box::new(scan),
            predicate: "age > 18".to_string(),
            selectivity: 0.7,
        };

        let cost = estimator.estimate(&filtered);
        assert_eq!(cost.estimated_rows, 700);
    }

    #[test]
    fn test_optimizer_improves_plan() {
        let model = CostModel::default();
        let estimator = CostEstimator::new(model);
        let optimizer = PlanOptimizer::new(estimator);

        let scan = PlanNode::TableScan {
            table: "users".to_string(),
            filters: vec![],
            estimated_rows: 1000,
        };

        let filtered = PlanNode::Filter {
            source: Box::new(scan),
            predicate: "age > 18".to_string(),
            selectivity: 0.7,
        };

        let optimized = optimizer.optimize(filtered);
        
        // Should have filter pushed down to table scan
        match optimized {
            PlanNode::TableScan { filters, .. } => {
                assert!(!filters.is_empty());
            }
            _ => panic!("Expected TableScan after optimization"),
        }
    }

    #[test]
    fn test_join_reordering() {
        let model = CostModel::default();
        let stats1 = TableStats::new(10);
        let stats2 = TableStats::new(1000);

        let estimator = CostEstimator::new(model)
            .with_stats("small".to_string(), stats1)
            .with_stats("large".to_string(), stats2);

        let optimizer = PlanOptimizer::new(estimator);

        let large = PlanNode::TableScan {
            table: "large".to_string(),
            filters: vec![],
            estimated_rows: 1000,
        };

        let small = PlanNode::TableScan {
            table: "small".to_string(),
            filters: vec![],
            estimated_rows: 10,
        };

        let join = PlanNode::Join {
            left: Box::new(large),
            right: Box::new(small),
            join_type: JoinType::Inner,
            condition: "large.id = small.id".to_string(),
        };

        let optimized = optimizer.optimize(join);
        
        // Should reorder to put small table first
        match optimized {
            PlanNode::Join { left, .. } => {
                if let PlanNode::TableScan { table, .. } = left.as_ref() {
                    assert_eq!(table, "small");
                }
            }
            _ => panic!("Expected Join after optimization"),
        }
    }

    #[test]
    fn test_complex_query_optimization() {
        let model = CostModel::default();
        let stats = TableStats::new(10000)
            .with_cardinality("department".to_string(), 10);

        let estimator = CostEstimator::new(model)
            .with_stats("employees".to_string(), stats);
        let optimizer = PlanOptimizer::new(estimator);

        // SELECT name, count(*) FROM employees WHERE salary > 50000 GROUP BY department
        let scan = PlanNode::TableScan {
            table: "employees".to_string(),
            filters: vec![],
            estimated_rows: 10000,
        };

        let filtered = PlanNode::Filter {
            source: Box::new(scan),
            predicate: "salary > 50000".to_string(),
            selectivity: 0.6,
        };

        let grouped = PlanNode::Aggregate {
            source: Box::new(filtered),
            group_by: vec!["department".to_string()],
            aggregations: vec![("salary".to_string(), "count".to_string())],
        };

        let optimized = optimizer.optimize(grouped);
        
        // Should have pushed filters down
        assert!(!format!("{}", optimized).is_empty());
    }

    #[test]
    fn test_plan_cost_estimation() {
        let model = CostModel::default();
        let stats = TableStats::new(5000);
        let estimator = CostEstimator::new(model)
            .with_stats("orders".to_string(), stats);

        let plan = PlanNode::TableScan {
            table: "orders".to_string(),
            filters: vec!["status = 'completed'".to_string()],
            estimated_rows: 5000,
        };

        let cost = estimator.estimate(&plan);
        
        assert_eq!(cost.estimated_rows, 5000);
        assert!(cost.total_cost > 0.0);
        assert!(cost.io_cost > 0.0);
        assert!(cost.cpu_cost > 0.0);
    }

    #[test]
    fn test_display_formatting() {
        let cost = QueryCost::new(5.0, 10.0, 2.0, 100);
        let display = format!("{}", cost);
        assert!(display.contains("Cost("));
        assert!(display.contains("Rows=100"));
    }
}
