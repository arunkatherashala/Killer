// ================================================================
// GENERIC QUERY BUILDER & ORM ABSTRACTION - Phase 23.3
// Filter DSL, result mapping, query construction, pagination
// ================================================================

use std::collections::HashMap;

/// Query filter operators
#[derive(Clone, Debug)]
pub enum FilterOp {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    In,
    NotIn,
    Like,
    Between,
    IsNull,
    IsNotNull,
}

/// Filter value
#[derive(Clone, Debug)]
pub enum FilterValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<FilterValue>),
}

/// Single filter condition
#[derive(Clone, Debug)]
pub struct FilterCondition {
    pub field: String,
    pub operator: FilterOp,
    pub value: FilterValue,
}

/// Combined filters with logical operators
#[derive(Clone, Debug)]
pub enum Filter {
    Condition(FilterCondition),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
}

/// Sort direction
#[derive(Clone, Debug)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Sort specification
#[derive(Clone, Debug)]
pub struct Sort {
    pub field: String,
    pub direction: SortDirection,
}

/// Pagination info
#[derive(Clone, Debug)]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
}

/// Query join
#[derive(Clone, Debug)]
pub struct Join {
    pub join_type: JoinType,
    pub table: String,
    pub on_field: String,
    pub local_field: String,
}

/// Join types
#[derive(Clone, Debug)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// Generic query builder
#[derive(Clone, Debug)]
pub struct QueryBuilder {
    pub table: String,
    pub select_fields: Vec<String>,
    pub filters: Vec<Filter>,
    pub sorts: Vec<Sort>,
    pub pagination: Option<Pagination>,
    pub joins: Vec<Join>,
    pub limit_count: Option<u32>,
    pub offset_count: Option<u32>,
}

/// Query result wrapper
#[derive(Clone, Debug)]
pub struct QueryResultSet<T> {
    pub data: Vec<T>,
    pub count: u64,
    pub total_count: Option<u64>,
}

/// Query statistics
#[derive(Clone, Debug)]
pub struct QueryStats {
    pub execution_time_ms: u64,
    pub rows_examined: u64,
    pub rows_returned: u64,
    pub database: String,
}

/// Query Query Builder Solver
pub struct QueryBuilderSolver;

impl QueryBuilderSolver {
    // ================================================================
    // QUERY BUILDER CREATION (1-8)
    // ================================================================

    /// Problem 1: Create new query builder
    pub fn select(table: &str) -> QueryBuilder {
        QueryBuilder {
            table: table.to_string(),
            select_fields: vec!["*".to_string()],
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        }
    }

    /// Problem 2: Build query with specific fields
    pub fn select_fields(table: &str, fields: &[&str]) -> QueryBuilder {
        QueryBuilder {
            table: table.to_string(),
            select_fields: fields.iter().map(|s| s.to_string()).collect(),
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        }
    }

    /// Problem 3: Add field to select
    pub fn with_field(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        if qb.select_fields == vec!["*".to_string()] {
            qb.select_fields.clear();
        }
        qb.select_fields.push(field.to_string());
        qb
    }

    /// Problem 4: Remove field from select
    pub fn without_field(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        qb.select_fields.retain(|f| f != field);
        qb
    }

    /// Problem 5: Select distinct
    pub fn select_distinct(table: &str, field: &str) -> QueryBuilder {
        let mut qb = QueryBuilder {
            table: table.to_string(),
            select_fields: vec![format!("DISTINCT {}", field)],
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        };
        qb
    }

    /// Problem 6: Count query
    pub fn count(table: &str) -> QueryBuilder {
        QueryBuilder {
            table: table.to_string(),
            select_fields: vec!["COUNT(*)".to_string()],
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        }
    }

    /// Problem 7: Sum aggregation
    pub fn sum(table: &str, field: &str) -> QueryBuilder {
        QueryBuilder {
            table: table.to_string(),
            select_fields: vec![format!("SUM({})", field)],
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        }
    }

    /// Problem 8: Average aggregation
    pub fn avg(table: &str, field: &str) -> QueryBuilder {
        QueryBuilder {
            table: table.to_string(),
            select_fields: vec![format!("AVG({})", field)],
            filters: vec![],
            sorts: vec![],
            pagination: None,
            joins: vec![],
            limit_count: None,
            offset_count: None,
        }
    }

    // ================================================================
    // FILTER OPERATIONS (9-20)
    // ================================================================

    /// Problem 9: Add equals filter
    pub fn where_eq(mut qb: QueryBuilder, field: &str, value: FilterValue) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::Equals,
            value,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 10: Add not equals filter
    pub fn where_ne(mut qb: QueryBuilder, field: &str, value: FilterValue) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::NotEquals,
            value,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 11: Add greater than filter
    pub fn where_gt(mut qb: QueryBuilder, field: &str, value: FilterValue) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::GreaterThan,
            value,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 12: Add less than filter
    pub fn where_lt(mut qb: QueryBuilder, field: &str, value: FilterValue) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::LessThan,
            value,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 13: Add IN filter
    pub fn where_in(mut qb: QueryBuilder, field: &str, values: Vec<FilterValue>) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::In,
            value: FilterValue::Array(values),
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 14: Add LIKE filter (string matching)
    pub fn where_like(mut qb: QueryBuilder, field: &str, pattern: &str) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::Like,
            value: FilterValue::String(pattern.to_string()),
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 15: Add BETWEEN filter
    pub fn where_between(mut qb: QueryBuilder, field: &str, lower: FilterValue, upper: FilterValue) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::Between,
            value: FilterValue::Array(vec![lower, upper]),
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 16: Add IS NULL filter
    pub fn where_null(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::IsNull,
            value: FilterValue::Null,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 17: Add IS NOT NULL filter
    pub fn where_not_null(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        let filter = Filter::Condition(FilterCondition {
            field: field.to_string(),
            operator: FilterOp::IsNotNull,
            value: FilterValue::Null,
        });
        qb.filters.push(filter);
        qb
    }

    /// Problem 18: AND multiple filters
    pub fn and_filters(filters: Vec<Filter>) -> Filter {
        Filter::And(filters)
    }

    /// Problem 19: OR multiple filters
    pub fn or_filters(filters: Vec<Filter>) -> Filter {
        Filter::Or(filters)
    }

    /// Problem 20: NOT filter
    pub fn not_filter(filter: Filter) -> Filter {
        Filter::Not(Box::new(filter))
    }

    // ================================================================
    // SORTING & PAGINATION (21-28)
    // ================================================================

    /// Problem 21: Add ascending sort
    pub fn order_by_asc(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        qb.sorts.push(Sort {
            field: field.to_string(),
            direction: SortDirection::Ascending,
        });
        qb
    }

    /// Problem 22: Add descending sort
    pub fn order_by_desc(mut qb: QueryBuilder, field: &str) -> QueryBuilder {
        qb.sorts.push(Sort {
            field: field.to_string(),
            direction: SortDirection::Descending,
        });
        qb
    }

    /// Problem 23: Add multiple sorts
    pub fn order_by_multi(mut qb: QueryBuilder, sorts: Vec<Sort>) -> QueryBuilder {
        qb.sorts.extend(sorts);
        qb
    }

    /// Problem 24: Set limit
    pub fn limit(mut qb: QueryBuilder, count: u32) -> QueryBuilder {
        qb.limit_count = Some(count);
        qb
    }

    /// Problem 25: Set offset
    pub fn offset(mut qb: QueryBuilder, count: u32) -> QueryBuilder {
        qb.offset_count = Some(count);
        qb
    }

    /// Problem 26: Set pagination
    pub fn paginate(mut qb: QueryBuilder, page: u32, per_page: u32) -> QueryBuilder {
        qb.pagination = Some(Pagination { page, per_page });
        qb
    }

    /// Problem 27: Calculate offset from page
    pub fn page_to_offset(page: u32, per_page: u32) -> u32 {
        (page - 1) * per_page
    }

    /// Problem 28: Calculate page count
    pub fn calculate_pages(total: u64, per_page: u64) -> u64 {
        (total + per_page - 1) / per_page
    }

    // ================================================================
    // JOINS (29-35)
    // ================================================================

    /// Problem 29: Inner join
    pub fn inner_join(mut qb: QueryBuilder, table: &str, on_field: &str, local_field: &str) -> QueryBuilder {
        qb.joins.push(Join {
            join_type: JoinType::Inner,
            table: table.to_string(),
            on_field: on_field.to_string(),
            local_field: local_field.to_string(),
        });
        qb
    }

    /// Problem 30: Left join
    pub fn left_join(mut qb: QueryBuilder, table: &str, on_field: &str, local_field: &str) -> QueryBuilder {
        qb.joins.push(Join {
            join_type: JoinType::Left,
            table: table.to_string(),
            on_field: on_field.to_string(),
            local_field: local_field.to_string(),
        });
        qb
    }

    /// Problem 31: Right join
    pub fn right_join(mut qb: QueryBuilder, table: &str, on_field: &str, local_field: &str) -> QueryBuilder {
        qb.joins.push(Join {
            join_type: JoinType::Right,
            table: table.to_string(),
            on_field: on_field.to_string(),
            local_field: local_field.to_string(),
        });
        qb
    }

    /// Problem 32: Full outer join
    pub fn full_join(mut qb: QueryBuilder, table: &str, on_field: &str, local_field: &str) -> QueryBuilder {
        qb.joins.push(Join {
            join_type: JoinType::Full,
            table: table.to_string(),
            on_field: on_field.to_string(),
            local_field: local_field.to_string(),
        });
        qb
    }

    // ================================================================
    // QUERY COMPILATION (33-38)
    // ================================================================

    /// Problem 33: Build SQL query string
    pub fn to_sql(qb: &QueryBuilder) -> String {
        let mut sql = format!("SELECT {} FROM {}", qb.select_fields.join(", "), qb.table);
        
        if !qb.joins.is_empty() {
            for join in &qb.joins {
                let join_type = match join.join_type {
                    JoinType::Inner => "INNER JOIN",
                    JoinType::Left => "LEFT JOIN",
                    JoinType::Right => "RIGHT JOIN",
                    JoinType::Full => "FULL OUTER JOIN",
                };
                sql.push_str(&format!(" {} {} ON {}.{} = {}.{}",
                    join_type, join.table, qb.table, join.local_field,
                    join.table, join.on_field));
            }
        }
        
        if !qb.filters.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&Self::filter_to_sql(&qb.filters[0]));
        }
        
        if !qb.sorts.is_empty() {
            sql.push_str(" ORDER BY ");
            let sort_strs: Vec<String> = qb.sorts.iter().map(|s| {
                let dir = match s.direction {
                    SortDirection::Ascending => "ASC",
                    SortDirection::Descending => "DESC",
                };
                format!("{} {}", s.field, dir)
            }).collect();
            sql.push_str(&sort_strs.join(", "));
        }
        
        if let Some(limit) = qb.limit_count {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = qb.offset_count {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        sql
    }

    /// Problem 34: Filter to SQL
    pub fn filter_to_sql(filter: &Filter) -> String {
        match filter {
            Filter::Condition(cond) => {
                let op_str = match cond.operator {
                    FilterOp::Equals => "=",
                    FilterOp::NotEquals => "!=",
                    FilterOp::GreaterThan => ">",
                    FilterOp::GreaterThanOrEqual => ">=",
                    FilterOp::LessThan => "<",
                    FilterOp::LessThanOrEqual => "<=",
                    FilterOp::In => "IN",
                    FilterOp::NotIn => "NOT IN",
                    FilterOp::Like => "LIKE",
                    FilterOp::Between => "BETWEEN",
                    FilterOp::IsNull => "IS NULL",
                    FilterOp::IsNotNull => "IS NOT NULL",
                };
                format!("{} {}", cond.field, op_str)
            },
            Filter::And(filters) => {
                let parts: Vec<String> = filters.iter().map(Self::filter_to_sql).collect();
                format!("({})", parts.join(" AND "))
            },
            Filter::Or(filters) => {
                let parts: Vec<String> = filters.iter().map(Self::filter_to_sql).collect();
                format!("({})", parts.join(" OR "))
            },
            Filter::Not(f) => format!("NOT {}", Self::filter_to_sql(f)),
        }
    }

    /// Problem 35: Validate query
    pub fn validate_query(qb: &QueryBuilder) -> bool {
        !qb.table.is_empty() && !qb.select_fields.is_empty()
    }

    // ================================================================
    // RESULT MAPPING (36-40)
    // ================================================================

    /// Problem 36: Map query result
    pub fn map_result<T: Clone>(result: &QueryResultSet<T>, transformer: fn(&T) -> T) -> QueryResultSet<T> {
        QueryResultSet {
            data: result.data.iter().map(transformer).collect(),
            count: result.count,
            total_count: result.total_count,
        }
    }

    /// Problem 37: Filter result rows
    pub fn filter_result<T: Clone>(result: &QueryResultSet<T>, predicate: fn(&T) -> bool) -> QueryResultSet<T> {
        QueryResultSet {
            data: result.data.iter().filter(|r| predicate(r)).cloned().collect(),
            count: result.data.len() as u64,
            total_count: result.total_count,
        }
    }

    /// Problem 38: Sort result rows
    pub fn sort_result<T: Clone + Ord>(result: &QueryResultSet<T>) -> QueryResultSet<T> {
        let mut data = result.data.clone();
        data.sort();
        QueryResultSet {
            data,
            count: result.count,
            total_count: result.total_count,
        }
    }

    /// Problem 39: Take N results
    pub fn take<T: Clone>(result: &QueryResultSet<T>, count: usize) -> QueryResultSet<T> {
        QueryResultSet {
            data: result.data.iter().take(count).cloned().collect(),
            count: std::cmp::min(result.count, count as u64),
            total_count: result.total_count,
        }
    }

    /// Problem 40: Skip N results
    pub fn skip<T: Clone>(result: &QueryResultSet<T>, count: usize) -> QueryResultSet<T> {
        QueryResultSet {
            data: result.data.iter().skip(count).cloned().collect(),
            count: if result.count > count as u64 { result.count - count as u64 } else { 0 },
            total_count: result.total_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_builder() {
        let qb = QueryBuilderSolver::select("users");
        assert_eq!(qb.table, "users");
        assert_eq!(qb.select_fields, vec!["*"]);
    }

    #[test]
    fn test_select_fields() {
        let qb = QueryBuilderSolver::select_fields("users", &["id", "name", "email"]);
        assert_eq!(qb.select_fields.len(), 3);
    }

    #[test]
    fn test_filter_builder() {
        let qb = QueryBuilderSolver::select("users");
        let qb = QueryBuilderSolver::where_eq(qb, "age", FilterValue::Int(25));
        assert_eq!(qb.filters.len(), 1);
    }

    #[test]
    fn test_pagination() {
        let offset = QueryBuilderSolver::page_to_offset(2, 10);
        assert_eq!(offset, 10);
    }

    #[test]
    fn test_to_sql() {
        let qb = QueryBuilderSolver::select("products");
        let sql = QueryBuilderSolver::to_sql(&qb);
        assert!(sql.contains("SELECT * FROM products"));
    }

    #[test]
    fn test_validate_query() {
        let qb = QueryBuilderSolver::select("users");
        assert!(QueryBuilderSolver::validate_query(&qb));
    }
}
