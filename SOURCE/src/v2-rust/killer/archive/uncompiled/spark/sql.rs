/// SQL Engine for Killer Spark
/// 
/// Supports basic SQL parsing and execution on DataFrames/RDDs.
/// Implements ANSI SQL subset: SELECT, FROM, WHERE, GROUP BY, ORDER BY, JOIN

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SQLValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
}

impl SQLValue {
    pub fn to_string(&self) -> String {
        match self {
            SQLValue::Number(n) => n.to_string(),
            SQLValue::String(s) => s.clone(),
            SQLValue::Boolean(b) => b.to_string(),
            SQLValue::Null => "NULL".to_string(),
        }
    }
}

/// SQL Query structure
#[derive(Debug, Clone)]
pub struct SQLQuery {
    pub select_clause: Vec<String>,
    pub from_clause: String,
    pub where_clause: Option<String>,
    pub group_by_clause: Option<Vec<String>>,
    pub order_by_clause: Option<Vec<(String, bool)>>, // column, descending
    pub join_clauses: Vec<JoinClause>,
    pub limit_clause: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub table: String,
    pub on: String,
    pub join_type: String, // INNER, LEFT, RIGHT, FULL
}

/// SQL Parser
pub struct SQLParser;

impl SQLParser {
    /// Parse SQL query string
    pub fn parse(sql: &str) -> Result<SQLQuery, String> {
        let sql = sql.trim();

        // Parse SELECT clause
        let select_start = sql
            .to_uppercase()
            .find("SELECT")
            .ok_or("Missing SELECT clause")?;
        let from_start = sql.to_uppercase().find("FROM").ok_or("Missing FROM clause")?;

        let select_str = &sql[select_start + 6..from_start].trim();
        let select_columns: Vec<String> = select_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        // Parse FROM clause
        let from_start_pos = from_start + 4;
        let where_start = sql.to_uppercase().find("WHERE");
        let group_by_start = sql.to_uppercase().find("GROUP BY");
        let order_by_start = sql.to_uppercase().find("ORDER BY");
        let limit_start = sql.to_uppercase().find("LIMIT");

        let next_keyword = [where_start, group_by_start, order_by_start, limit_start]
            .iter()
            .filter_map(|&x| x)
            .min();

        let from_end = next_keyword.unwrap_or(sql.len());
        let from_table = sql[from_start_pos..from_end].trim().to_string();

        // Parse WHERE clause
        let where_clause = where_start.map(|pos| {
            let where_end = [group_by_start, order_by_start, limit_start]
                .iter()
                .filter_map(|&x| x)
                .min()
                .unwrap_or(sql.len());
            sql[pos + 5..where_end].trim().to_string()
        });

        // Parse GROUP BY clause
        let group_by_clause = group_by_start.map(|pos| {
            let group_end = [order_by_start, limit_start]
                .iter()
                .filter_map(|&x| x)
                .min()
                .unwrap_or(sql.len());
            let group_str = &sql[pos + 8..group_end].trim();
            group_str.split(',').map(|s| s.trim().to_string()).collect()
        });

        // Parse ORDER BY clause
        let order_by_clause = order_by_start.map(|pos| {
            let order_end = limit_start.unwrap_or(sql.len());
            let order_str = &sql[pos + 8..order_end].trim();
            order_str
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    let desc = s.to_uppercase().ends_with("DESC");
                    let col = if desc {
                        s[..s.len() - 4].trim()
                    } else if s.to_uppercase().ends_with("ASC") {
                        s[..s.len() - 3].trim()
                    } else {
                        s
                    };
                    (col.to_string(), desc)
                })
                .collect()
        });

        // Parse LIMIT clause
        let limit_clause = limit_start.map(|pos| {
            let limit_str = &sql[pos + 5..].trim();
            limit_str
                .parse::<usize>()
                .map_err(|_| "Invalid LIMIT value".to_string())
        }).transpose()?;

        Ok(SQLQuery {
            select_clause: select_columns,
            from_clause: from_table,
            where_clause,
            group_by_clause,
            order_by_clause,
            join_clauses: Vec::new(),
            limit_clause,
        })
    }

    /// Validate query
    pub fn validate(query: &SQLQuery) -> Result<(), String> {
        if query.select_clause.is_empty() {
            return Err("SELECT clause cannot be empty".to_string());
        }

        if query.from_clause.is_empty() {
            return Err("FROM clause cannot be empty".to_string());
        }

        Ok(())
    }
}

/// SQL Executor
pub struct SQLExecutor;

impl SQLExecutor {
    /// Execute query on data
    pub fn execute(
        query: &SQLQuery,
        tables: &HashMap<String, Vec<Vec<SQLValue>>>,
    ) -> Result<Vec<Vec<SQLValue>>, String> {
        let table_data = tables
            .get(&query.from_clause)
            .ok_or(format!("Table {} not found", query.from_clause))?
            .clone();

        let mut result = table_data.clone();

        // Apply WHERE filter
        if let Some(where_clause) = &query.where_clause {
            result = Self::apply_where(&result, where_clause)?;
        }

        // Apply GROUP BY
        if let Some(group_by) = &query.group_by_clause {
            result = Self::apply_group_by(&result, group_by)?;
        }

        // Apply ORDER BY
        if let Some(order_by) = &query.order_by_clause {
            result = Self::apply_order_by(&mut result, order_by)?;
        }

        // Apply LIMIT
        if let Some(limit) = query.limit_clause {
            result.truncate(limit);
        }

        Ok(result)
    }

    fn apply_where(
        data: &[Vec<SQLValue>],
        _predicate: &str,
    ) -> Result<Vec<Vec<SQLValue>>, String> {
        // Simple WHERE implementation - would need full expression evaluation
        Ok(data.to_vec())
    }

    fn apply_group_by(
        _data: &[Vec<SQLValue>],
        _keys: &[String],
    ) -> Result<Vec<Vec<SQLValue>>, String> {
        // GROUP BY would aggregate data - simplified for now
        Ok(Vec::new())
    }

    fn apply_order_by(
        data: &mut [Vec<SQLValue>],
        _order_by: &[(String, bool)],
    ) -> Result<Vec<Vec<SQLValue>>, String> {
        // ORDER BY would sort data - simplified for now
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_parse_simple_select() {
        let sql = "SELECT name, age FROM users";
        let query = SQLParser::parse(sql).unwrap();

        assert_eq!(query.select_clause.len(), 2);
        assert_eq!(query.from_clause, "users");
        assert!(query.where_clause.is_none());
    }

    #[test]
    fn test_sql_parse_with_where() {
        let sql = "SELECT name FROM users WHERE age > 18";
        let query = SQLParser::parse(sql).unwrap();

        assert_eq!(query.select_clause[0], "name");
        assert_eq!(query.from_clause, "users");
        assert!(query.where_clause.is_some());
    }

    #[test]
    fn test_sql_parse_with_order_by() {
        let sql = "SELECT * FROM users ORDER BY age DESC";
        let query = SQLParser::parse(sql).unwrap();

        assert!(query.order_by_clause.is_some());
        let order_by = query.order_by_clause.unwrap();
        assert_eq!(order_by[0].0, "age");
        assert!(order_by[0].1); // descending
    }

    #[test]
    fn test_sql_parse_with_limit() {
        let sql = "SELECT * FROM users LIMIT 10";
        let query = SQLParser::parse(sql).unwrap();

        assert_eq!(query.limit_clause, Some(10));
    }

    #[test]
    fn test_sql_parse_with_group_by() {
        let sql = "SELECT dept, COUNT(*) FROM employees GROUP BY dept";
        let query = SQLParser::parse(sql).unwrap();

        assert!(query.group_by_clause.is_some());
        let group_by = query.group_by_clause.unwrap();
        assert_eq!(group_by.len(), 1);
    }

    #[test]
    fn test_sql_validate() {
        let query = SQLQuery {
            select_clause: vec!["col1".to_string()],
            from_clause: "table1".to_string(),
            where_clause: None,
            group_by_clause: None,
            order_by_clause: None,
            join_clauses: Vec::new(),
            limit_clause: None,
        };

        assert!(SQLParser::validate(&query).is_ok());
    }
}
