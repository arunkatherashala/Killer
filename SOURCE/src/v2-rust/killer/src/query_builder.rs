/// Query Builder - Fluent API for building SQL queries with named parameters
/// Week 12 - Integrates with Week 11 named parameters

use std::collections::HashMap;
use crate::value::Value;

/// SQL Query Builder
pub struct QueryBuilder {
    select_cols: Vec<String>,
    table_name: String,
    where_clause: Vec<String>,
    where_params: HashMap<String, Value>,
    order_by: Option<String>,
    limit_val: Option<usize>,
    offset_val: Option<usize>,
}

impl QueryBuilder {
    /// Create new SELECT query builder
    pub fn select(table: &str) -> Self {
        QueryBuilder {
            select_cols: vec!["*".to_string()],
            table_name: table.to_string(),
            where_clause: Vec::new(),
            where_params: HashMap::new(),
            order_by: None,
            limit_val: None,
            offset_val: None,
        }
    }

    /// Specify columns to select
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.select_cols = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Add specific columns to select (instead of *)
    pub fn column(mut self, col: &str) -> Self {
        if self.select_cols == vec!["*".to_string()] {
            self.select_cols = vec![col.to_string()];
        } else {
            self.select_cols.push(col.to_string());
        }
        self
    }

    /// Add WHERE clause with named parameter
    pub fn where_eq(mut self, column: &str, name: &str, value: Value) -> Self {
        self.where_clause.push(format!("{} = :{}", column, name));
        self.where_params.insert(name.to_string(), value);
        self
    }

    /// Add WHERE clause with comparison
    pub fn where_cond(mut self, column: &str, op: &str, name: &str, value: Value) -> Self {
        self.where_clause.push(format!("{} {} :{}", column, op, name));
        self.where_params.insert(name.to_string(), value);
        self
    }

    /// Add raw WHERE clause
    pub fn where_raw(mut self, condition: &str) -> Self {
        self.where_clause.push(condition.to_string());
        self
    }

    /// Order by column
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some(format!("{} {}", column, direction.to_uppercase()));
        self
    }

    /// Limit results
    pub fn limit(mut self, count: usize) -> Self {
        self.limit_val = Some(count);
        self
    }

    /// Offset results
    pub fn offset(mut self, count: usize) -> Self {
        self.offset_val = Some(count);
        self
    }

    /// Build SQL string
    pub fn build_sql(&self) -> String {
        let mut sql = format!(
            "SELECT {} FROM {}",
            self.select_cols.join(", "),
            self.table_name
        );

        if !self.where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", self.where_clause.join(" AND ")));
        }

        if let Some(ref ob) = self.order_by {
            sql.push_str(&format!(" ORDER BY {}", ob));
        }

        if let Some(limit) = self.limit_val {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset_val {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    /// Get parameters for parameterized query
    pub fn get_params(&self) -> &HashMap<String, Value> {
        &self.where_params
    }
}

/// Insert Query Builder
pub struct InsertBuilder {
    table_name: String,
    columns: Vec<String>,
    values: Vec<Value>,
}

impl InsertBuilder {
    /// Create new INSERT query builder
    pub fn into(table: &str) -> Self {
        InsertBuilder {
            table_name: table.to_string(),
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Add column and value
    pub fn value(mut self, column: &str, value: Value) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value);
        self
    }

    /// Add multiple columns and values
    pub fn values(mut self, pairs: HashMap<String, Value>) -> Self {
        for (col, val) in pairs {
            self.columns.push(col);
            self.values.push(val);
        }
        self
    }

    /// Build SQL string
    pub fn build_sql(&self) -> String {
        let placeholders = (0..self.values.len())
            .map(|i| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table_name,
            self.columns.join(", "),
            placeholders
        )
    }

    /// Get values in order
    pub fn get_values(&self) -> &[Value] {
        &self.values
    }
}

/// Update Query Builder
pub struct UpdateBuilder {
    table_name: String,
    updates: HashMap<String, Value>,
    where_clause: Vec<String>,
    where_params: HashMap<String, Value>,
}

impl UpdateBuilder {
    /// Create new UPDATE query builder
    pub fn table(table: &str) -> Self {
        UpdateBuilder {
            table_name: table.to_string(),
            updates: HashMap::new(),
            where_clause: Vec::new(),
            where_params: HashMap::new(),
        }
    }

    /// Set column value with named parameter
    pub fn set(mut self, column: &str, name: &str, value: Value) -> Self {
        self.updates.insert(column.to_string(), value.clone());
        self.where_params.insert(name.to_string(), value);
        self
    }

    /// Add WHERE clause
    pub fn where_eq(mut self, column: &str, name: &str, value: Value) -> Self {
        self.where_clause.push(format!("{} = :{}", column, name));
        self.where_params.insert(name.to_string(), value);
        self
    }

    /// Build SQL string
    pub fn build_sql(&self) -> String {
        let set_clauses: Vec<String> = self.updates
            .iter()
            .map(|(col, _)| format!("{} = ?", col))
            .collect();

        let mut sql = format!(
            "UPDATE {} SET {}",
            self.table_name,
            set_clauses.join(", ")
        );

        if !self.where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", self.where_clause.join(" AND ")));
        }

        sql
    }

    /// Get all parameters
    pub fn get_params(&self) -> &HashMap<String, Value> {
        &self.where_params
    }
}

/// Delete Query Builder
pub struct DeleteBuilder {
    table_name: String,
    where_clause: Vec<String>,
    where_params: HashMap<String, Value>,
}

impl DeleteBuilder {
    /// Create new DELETE query builder
    pub fn from(table: &str) -> Self {
        DeleteBuilder {
            table_name: table.to_string(),
            where_clause: Vec::new(),
            where_params: HashMap::new(),
        }
    }

    /// Add WHERE clause
    pub fn where_eq(mut self, column: &str, name: &str, value: Value) -> Self {
        self.where_clause.push(format!("{} = :{}", column, name));
        self.where_params.insert(name.to_string(), value);
        self
    }

    /// Build SQL string
    pub fn build_sql(&self) -> String {
        let mut sql = format!("DELETE FROM {}", self.table_name);

        if !self.where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", self.where_clause.join(" AND ")));
        }

        sql
    }

    /// Get parameters
    pub fn get_params(&self) -> &HashMap<String, Value> {
        &self.where_params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_builder_basic() {
        let query = QueryBuilder::select("users").build_sql();
        assert_eq!(query, "SELECT * FROM users");
    }

    #[test]
    fn test_select_builder_columns() {
        let query = QueryBuilder::select("users")
            .columns(&["id", "name", "email"])
            .build_sql();
        assert!(query.contains("SELECT id, name, email FROM users"));
    }

    #[test]
    fn test_select_builder_where() {
        let query = QueryBuilder::select("users")
            .where_eq("id", "user_id", Value::Number(1.0))
            .build_sql();
        assert!(query.contains("WHERE id = :user_id"));
    }

    #[test]
    fn test_select_builder_with_params() {
        let query = QueryBuilder::select("users")
            .where_eq("name", "search_name", Value::Str("Alice".to_string()))
            .where_eq("age", "min_age", Value::Number(18.0));
        
        let sql = query.build_sql();
        assert!(sql.contains("WHERE"));
        assert_eq!(query.get_params().len(), 2);
    }

    #[test]
    fn test_select_builder_order() {
        let query = QueryBuilder::select("users")
            .order_by("name", "ASC")
            .build_sql();
        assert!(query.contains("ORDER BY name ASC"));
    }

    #[test]
    fn test_select_builder_limit() {
        let query = QueryBuilder::select("users")
            .limit(10)
            .build_sql();
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_select_builder_pagination() {
        let query = QueryBuilder::select("users")
            .limit(10)
            .offset(20)
            .build_sql();
        assert!(query.contains("LIMIT 10"));
        assert!(query.contains("OFFSET 20"));
    }

    #[test]
    fn test_insert_builder() {
        let query = InsertBuilder::into("users")
            .value("name", Value::Str("Alice".to_string()))
            .value("email", Value::Str("alice@example.com".to_string()))
            .build_sql();
        
        assert!(query.contains("INSERT INTO users"));
        assert!(query.contains("name, email"));
    }

    #[test]
    fn test_update_builder() {
        let query = UpdateBuilder::table("users")
            .set("name", "new_name", Value::Str("Bob".to_string()))
            .where_eq("id", "user_id", Value::Number(1.0))
            .build_sql();
        
        assert!(query.contains("UPDATE users SET"));
        assert!(query.contains("WHERE id = :user_id"));
    }

    #[test]
    fn test_delete_builder() {
        let query = DeleteBuilder::from("users")
            .where_eq("id", "user_id", Value::Number(1.0))
            .build_sql();
        
        assert!(query.contains("DELETE FROM users"));
        assert!(query.contains("WHERE id = :user_id"));
    }

    #[test]
    fn test_complex_select() {
        let query = QueryBuilder::select("users")
            .columns(&["id", "name"])
            .where_eq("status", "status_val", Value::Str("active".to_string()))
            .where_cond("age", ">", "min_age", Value::Number(21.0))
            .order_by("created_at", "DESC")
            .limit(50)
            .build_sql();
        
        assert!(query.contains("SELECT id, name"));
        assert!(query.contains("WHERE"));
        assert!(query.contains("ORDER BY created_at DESC"));
        assert!(query.contains("LIMIT 50"));
    }
}
