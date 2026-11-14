/// Database Abstraction Layer - SQLite Support
/// Week 12 Implementation - CRUD operations, query building, migrations

use std::collections::HashMap;
use crate::value::Value;

/// represents a database row
pub type Row = HashMap<String, Value>;

/// Database result with rows
pub type DbResult<T> = Result<T, DatabaseError>;

/// Database errors
#[derive(Debug, Clone)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryError(String),
    NoRowsAffected,
    InvalidSchema(String),
    MigrationFailed(String),
    TransactionFailed(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            DatabaseError::QueryError(msg) => write!(f, "Query error: {}", msg),
            DatabaseError::NoRowsAffected => write!(f, "No rows affected"),
            DatabaseError::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            DatabaseError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            DatabaseError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
        }
    }
}

/// Column definition in a table
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub col_type: ColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<Value>,
}

/// Column data types
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    Text,
    Integer,
    Real,
    Boolean,
    Blob,
    Timestamp,
}

impl ColumnType {
    pub fn to_sql(&self) -> &str {
        match self {
            ColumnType::Text => "TEXT",
            ColumnType::Integer => "INTEGER",
            ColumnType::Real => "REAL",
            ColumnType::Boolean => "BOOLEAN",
            ColumnType::Blob => "BLOB",
            ColumnType::Timestamp => "TIMESTAMP",
        }
    }
}

impl Column {
    /// Create a new column
    pub fn new(name: String, col_type: ColumnType) -> Self {
        Column {
            name,
            col_type,
            nullable: true,
            primary_key: false,
            unique: false,
            default: None,
        }
    }

    /// Make column required (not nullable)
    pub fn required(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Mark as primary key
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    /// Mark as unique
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Set default value
    pub fn default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Generate SQL column definition
    pub fn to_sql(&self) -> String {
        let mut def = format!("{} {}", self.name, self.col_type.to_sql());

        if self.primary_key {
            def.push_str(" PRIMARY KEY");
        }

        if !self.nullable && !self.primary_key {
            def.push_str(" NOT NULL");
        }

        if self.unique {
            def.push_str(" UNIQUE");
        }

        if let Some(ref default) = self.default {
            def.push_str(&format!(" DEFAULT {}", value_to_sql(default)));
        }

        def
    }
}

/// Table schema
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

impl Table {
    /// Create new table
    pub fn new(name: String) -> Self {
        Table {
            name,
            columns: Vec::new(),
        }
    }

    /// Add column to table
    pub fn column(mut self, col: Column) -> Self {
        self.columns.push(col);
        self
    }

    /// Get column by name
    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Generate CREATE TABLE SQL
    pub fn to_create_sql(&self) -> String {
        let cols: Vec<String> = self.columns.iter().map(|c| c.to_sql()).collect();
        format!("CREATE TABLE IF NOT EXISTS {} ({})", self.name, cols.join(", "))
    }

    /// Validate row against schema
    pub fn validate_row(&self, row: &Row) -> DbResult<()> {
        for col in &self.columns {
            if col.primary_key {
                continue;  // Skip primary key validation
            }

            match row.get(&col.name) {
                Some(val) => {
                    // Type validation could go here
                    if matches!(val, Value::Null) && !col.nullable {
                        return Err(DatabaseError::InvalidSchema(
                            format!("Column '{}' cannot be null", col.name)
                        ));
                    }
                }
                None => {
                    if !col.nullable && col.default.is_none() {
                        return Err(DatabaseError::InvalidSchema(
                            format!("Missing required column: '{}'", col.name)
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<Row>,
    pub affected_rows: usize,
}

impl QueryResult {
    pub fn new() -> Self {
        QueryResult {
            rows: Vec::new(),
            affected_rows: 0,
        }
    }

    pub fn with_rows(rows: Vec<Row>) -> Self {
        let row_count = rows.len();
        QueryResult {
            rows,
            affected_rows: row_count,
        }
    }

    /// Get first row
    pub fn first(&self) -> Option<&Row> {
        self.rows.first()
    }

    /// Get all rows
    pub fn all(&self) -> &[Row] {
        &self.rows
    }

    /// Get single value (first column of first row)
    pub fn scalar(&self) -> Option<&Value> {
        self.rows
            .first()
            .and_then(|row| row.values().next())
    }

    /// Check if query affected any rows
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Row count
    pub fn count(&self) -> usize {
        self.rows.len()
    }
}

/// Connection to database
pub struct Connection {
    pub name: String,
    pub is_connected: bool,
    pub tables: HashMap<String, Table>,
}

impl Connection {
    /// Open database connection
    pub fn open(path: &str) -> DbResult<Self> {
        // Simplified - in real implementation, would open actual SQLite
        Ok(Connection {
            name: path.to_string(),
            is_connected: true,
            tables: HashMap::new(),
        })
    }

    /// Create table
    pub fn create_table(&mut self, table: Table) -> DbResult<()> {
        if self.tables.contains_key(&table.name) {
            return Err(DatabaseError::InvalidSchema(
                format!("Table '{}' already exists", table.name)
            ));
        }
        self.tables.insert(table.name.clone(), table);
        Ok(())
    }

    /// Get table
    pub fn table(&self, name: &str) -> DbResult<&Table> {
        self.tables.get(name).ok_or_else(|| {
            DatabaseError::QueryError(format!("Table '{}' not found", name))
        })
    }

    /// Insert row
    pub fn insert(
        &mut self,
        table_name: &str,
        row: Row,
    ) -> DbResult<usize> {
        let table = self.table(table_name)?;
        table.validate_row(&row)?;
        
        // In real DB: execute INSERT INTO...
        Ok(1)  // 1 row inserted
    }

    /// Insert multiple rows
    pub fn insert_many(
        &mut self,
        table_name: &str,
        rows: Vec<Row>,
    ) -> DbResult<usize> {
        let count = rows.len();
        for row in rows {
            self.insert(table_name, row)?;
        }
        Ok(count)
    }

    /// Select rows
    pub fn select(
        &self,
        table_name: &str,
        filters: Option<HashMap<String, Value>>,
    ) -> DbResult<QueryResult> {
        let _table = self.table(table_name)?;
        
        // In real DB: execute SELECT...
        // For now, return empty result
        Ok(QueryResult::new())
    }

    /// Update rows
    pub fn update(
        &mut self,
        table_name: &str,
        updates: HashMap<String, Value>,
        filters: Option<HashMap<String, Value>>,
    ) -> DbResult<usize> {
        let table = self.table(table_name)?;
        table.validate_row(&updates)?;
        
        // In real DB: execute UPDATE...
        Ok(1)  // Simplified
    }

    /// Delete rows
    pub fn delete(
        &mut self,
        table_name: &str,
        filters: Option<HashMap<String, Value>>,
    ) -> DbResult<usize> {
        let _table = self.table(table_name)?;
        let _filters = filters;
        
        // In real DB: execute DELETE...
        Ok(1)  // Simplified
    }

    /// Execute raw SQL
    pub fn execute(&mut self, sql: &str) -> DbResult<QueryResult> {
        if sql.is_empty() {
            return Err(DatabaseError::QueryError("Empty SQL".to_string()));
        }
        
        // In real DB: execute the SQL
        Ok(QueryResult::new())
    }

    /// Close connection
    pub fn close(&mut self) -> DbResult<()> {
        self.is_connected = false;
        Ok(())
    }
}

/// Helper to convert Value to SQL
fn value_to_sql(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
        Value::Number(n) => n.to_string(),
        Value::Str(s) => format!("'{}'", s.replace("'", "''")),
        Value::Array(_) => "NULL".to_string(),  // Simplified: arrays stored as NULL
        Value::Dict(_) => "NULL".to_string(),   // Simplified: dicts stored as NULL
        Value::Object(_) => "NULL".to_string(), // Objects stored as NULL
        Value::Class(_) => "NULL".to_string(),  // Classes stored as NULL
        Value::Function { .. } => "NULL".to_string(), // Functions stored as NULL
        Value::Generator(_) => "NULL".to_string(), // Generators stored as NULL
        Value::QualityWrapped(_) => "NULL".to_string(), // Quality objects stored as NULL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_creation() {
        let col = Column::new("id".to_string(), ColumnType::Integer)
            .primary_key();
        
        assert_eq!(col.name, "id");
        assert!(col.primary_key);
        assert!(!col.nullable);
    }

    #[test]
    fn test_column_sql_generation() {
        let col = Column::new("name".to_string(), ColumnType::Text)
            .required();
        
        let sql = col.to_sql();
        assert!(sql.contains("name"));
        assert!(sql.contains("TEXT"));
        assert!(sql.contains("NOT NULL"));
    }

    #[test]
    fn test_table_creation() {
        let table = Table::new("users".to_string())
            .column(Column::new("id".to_string(), ColumnType::Integer).primary_key())
            .column(Column::new("name".to_string(), ColumnType::Text).required());
        
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
    }

    #[test]
    fn test_table_sql_generation() {
        let table = Table::new("users".to_string())
            .column(Column::new("id".to_string(), ColumnType::Integer).primary_key())
            .column(Column::new("name".to_string(), ColumnType::Text));
        
        let sql = table.to_create_sql();
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("users"));
        assert!(sql.contains("id"));
    }

    #[test]
    fn test_connection_open() {
        let conn = Connection::open(":memory:").unwrap();
        assert!(conn.is_connected);
    }

    #[test]
    fn test_create_table_in_connection() {
        let mut conn = Connection::open(":memory:").unwrap();
        let table = Table::new("users".to_string())
            .column(Column::new("id".to_string(), ColumnType::Integer).primary_key());
        
        assert!(conn.create_table(table).is_ok());
        assert!(conn.table("users").is_ok());
    }

    #[test]
    fn test_row_validation() {
        let table = Table::new("users".to_string())
            .column(Column::new("id".to_string(), ColumnType::Integer).primary_key())
            .column(Column::new("name".to_string(), ColumnType::Text).required());
        
        let mut row = HashMap::new();
        row.insert("id".to_string(), Value::Number(1.0));
        row.insert("name".to_string(), Value::Str("Alice".to_string()));
        
        assert!(table.validate_row(&row).is_ok());
    }

    #[test]
    fn test_row_validation_fails_missing_required() {
        let table = Table::new("users".to_string())
            .column(Column::new("name".to_string(), ColumnType::Text).required());
        
        let row = HashMap::new();
        assert!(table.validate_row(&row).is_err());
    }

    #[test]
    fn test_query_result() {
        let result = QueryResult::new();
        assert!(result.is_empty());
        assert_eq!(result.count(), 0);
    }
}
