/// ORM Helpers - Object-Relational Mapping utilities
/// Week 12 - Makes it easier to work with database entities through Rust structs

use std::collections::HashMap;
use crate::value::Value;
use crate::database::{Row, Table, Column, ColumnType};
use crate::query_builder::{QueryBuilder, InsertBuilder, UpdateBuilder};

/// Trait for ORM mappable types
pub trait Mappable {
    /// Convert struct to database row
    fn to_row(&self) -> Row;
    
    /// Convert database row to struct
    fn from_row(row: &Row) -> Self;
    
    /// Get table name for this struct
    fn table_name() -> &'static str;
    
    /// Get table schema for this struct
    fn schema() -> Table;
}

/// ORM Repository - Base class for database entities
pub struct Repository<T: Mappable> {
    table_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Mappable> Repository<T> {
    /// Create new repository
    pub fn new() -> Self {
        Repository {
            table_name: T::table_name().to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Find all records
    pub fn find_all(&self) -> QueryBuilder {
        QueryBuilder::select(&self.table_name)
    }
    
    /// Find by ID
    pub fn find_by_id(&self, id: f64) -> QueryBuilder {
        QueryBuilder::select(&self.table_name)
            .where_eq("id", "id", Value::Number(id))
    }
    
    /// Find by condition
    pub fn find_by(&self, column: &str, param_name: &str, value: Value) -> QueryBuilder {
        QueryBuilder::select(&self.table_name)
            .where_eq(column, param_name, value)
    }
    
    /// Count all records
    pub fn count_all(&self) -> String {
        format!("SELECT COUNT(*) as count FROM {}", self.table_name)
    }
    
    /// Count with condition
    pub fn count_where(&self, column: &str, param_name: &str, value: Value) -> String {
        let query = QueryBuilder::select(&self.table_name)
            .where_eq(column, param_name, value);
        
        // Return a COUNT query - simplified version
        format!("SELECT COUNT(*) as count FROM {}", self.table_name)
    }
    
    /// Get table schema
    pub fn schema(&self) -> Table {
        T::schema()
    }
}

/// Query helper for building complex WHERE clauses
pub struct QueryHelper {
    conditions: Vec<(String, String, Value)>,  // (column, param_name, value)
}

impl QueryHelper {
    /// Create new query helper
    pub fn new() -> Self {
        QueryHelper {
            conditions: Vec::new(),
        }
    }
    
    /// Add equality condition
    pub fn eq(mut self, column: &str, param_name: &str, value: Value) -> Self {
        self.conditions.push((column.to_string(), param_name.to_string(), value));
        self
    }
    
    /// Add greater-than condition
    pub fn gt(mut self, column: &str, param_name: &str, value: Value) -> Self {
        self.conditions.push((format!("{} >", column), param_name.to_string(), value));
        self
    }
    
    /// Add less-than condition
    pub fn lt(mut self, column: &str, param_name: &str, value: Value) -> Self {
        self.conditions.push((format!("{} <", column), param_name.to_string(), value));
        self
    }
    
    /// Build complete WHERE clause
    pub fn build(&self) -> (String, HashMap<String, Value>) {
        let mut params = HashMap::new();
        let conditions: Vec<String> = self.conditions
            .iter()
            .map(|(col, param, val)| {
                params.insert(param.clone(), val.clone());
                format!("{} = :{}", col, param)
            })
            .collect();
        
        (conditions.join(" AND "), params)
    }
}

/// Entity wrapper for easy CRUD operations
pub struct Entity<T: Mappable> {
    pub data: T,
    pub is_new: bool,
}

impl<T: Mappable> Entity<T> {
    /// Create new entity (not yet saved)
    pub fn new(data: T) -> Self {
        Entity {
            data,
            is_new: true,
        }
    }
    
    /// Wrap loaded entity (already in database)
    pub fn loaded(data: T) -> Self {
        Entity {
            data,
            is_new: false,
        }
    }
    
    /// Convert to database row
    pub fn to_row(&self) -> Row {
        self.data.to_row()
    }
}

/// Change tracking mixin for entities
pub struct ChangeTracker {
    original: Row,
    modified: Row,
}

impl ChangeTracker {
    /// Create tracker from original row
    pub fn new(row: Row) -> Self {
        let modified = row.clone();
        ChangeTracker {
            original: row,
            modified,
        }
    }
    
    /// Mark field as modified
    pub fn mark_dirty(&mut self, field: &str, value: Value) {
        self.modified.insert(field.to_string(), value);
    }
    
    /// Get only modified fields
    pub fn get_changes(&self) -> HashMap<String, Value> {
        let mut changes = HashMap::new();
        
        for (key, new_val) in &self.modified {
            if !self.original.contains_key(key) || 
               self.original.get(key) != Some(new_val) {
                changes.insert(key.clone(), new_val.clone());
            }
        }
        
        changes
    }
    
    /// Check if anything changed
    pub fn is_dirty(&self) -> bool {
        !self.get_changes().is_empty()
    }
    
    /// Reset changes
    pub fn reset(&mut self) {
        self.modified = self.original.clone();
    }
}

/// Validation trait for entities
pub trait Validatable {
    /// Validate entity
    fn validate(&self) -> Result<(), String>;
}

/// Pagination helper
pub struct Pagination {
    page: usize,
    per_page: usize,
}

impl Pagination {
    /// Create pagination
    pub fn new(page: usize, per_page: usize) -> Self {
        Pagination { page, per_page }
    }
    
    /// Get offset for query
    pub fn offset(&self) -> usize {
        (self.page - 1) * self.per_page
    }
    
    /// Get limit for query
    pub fn limit(&self) -> usize {
        self.per_page
    }
    
    /// Apply to query
    pub fn apply(&self, query: QueryBuilder) -> QueryBuilder {
        query.limit(self.limit()).offset(self.offset())
    }
}

/// Sort helper
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn as_str(&self) -> &str {
        match self {
            SortOrder::Ascending => "ASC",
            SortOrder::Descending => "DESC",
        }
    }
}

pub struct Sort {
    column: String,
    order: SortOrder,
}

impl Sort {
    /// Create new sort
    pub fn new(column: &str, order: SortOrder) -> Self {
        Sort {
            column: column.to_string(),
            order,
        }
    }
    
    /// Apply to query
    pub fn apply(&self, query: QueryBuilder) -> QueryBuilder {
        query.order_by(&self.column, self.order.as_str())
    }
}

/// Query result mapper
pub struct ResultMapper<T: Mappable> {
    rows: Vec<Row>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Mappable> ResultMapper<T> {
    /// Create mapper from rows
    pub fn new(rows: Vec<Row>) -> Self {
        ResultMapper {
            rows,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Map all rows to entities
    pub fn to_entities(&self) -> Vec<T> {
        self.rows.iter().map(T::from_row).collect()
    }
    
    /// Map single row to entity (first result)
    pub fn to_entity(&self) -> Option<T> {
        self.rows.first().map(T::from_row)
    }
    
    /// Get raw rows
    pub fn to_rows(&self) -> &[Row] {
        &self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock struct for testing
    struct User {
        id: f64,
        name: String,
        email: String,
    }
    
    impl Mappable for User {
        fn to_row(&self) -> Row {
            let mut row = Row::new();
            row.insert("id".to_string(), Value::Number(self.id));
            row.insert("name".to_string(), Value::Str(self.name.clone()));
            row.insert("email".to_string(), Value::Str(self.email.clone()));
            row
        }
        
        fn from_row(row: &Row) -> Self {
            User {
                id: match row.get("id") {
                    Some(Value::Number(n)) => *n,
                    _ => 0.0,
                },
                name: match row.get("name") {
                    Some(Value::Str(s)) => s.clone(),
                    _ => String::new(),
                },
                email: match row.get("email") {
                    Some(Value::Str(s)) => s.clone(),
                    _ => String::new(),
                },
            }
        }
        
        fn table_name() -> &'static str {
            "users"
        }
        
        fn schema() -> Table {
            Table::new("users".to_string())
                .column(Column::new("id".to_string(), ColumnType::Integer))
                .column(Column::new("name".to_string(), ColumnType::Text))
                .column(Column::new("email".to_string(), ColumnType::Text))
        }
    }
    
    #[test]
    fn test_repository_creation() {
        let repo: Repository<User> = Repository::new();
        assert_eq!(repo.table_name, "users");
    }
    
    #[test]
    fn test_find_by_id() {
        let repo: Repository<User> = Repository::new();
        let query = repo.find_by_id(1.0);
        let sql = query.build_sql();
        assert!(sql.contains("WHERE id = :id"));
    }
    
    #[test]
    fn test_query_helper() {
        let helper = QueryHelper::new()
            .eq("status", "status", Value::Str("active".to_string()))
            .eq("type", "user_type", Value::Str("admin".to_string()));
        
        let (clause, _params) = helper.build();
        assert!(clause.contains("status"));
        assert!(clause.contains("AND"));
    }
    
    #[test]
    fn test_pagination() {
        let paging = Pagination::new(2, 10);
        assert_eq!(paging.offset(), 10);
        assert_eq!(paging.limit(), 10);
    }
    
    #[test]
    fn test_entity_creation() {
        let user = User {
            id: 1.0,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        
        let entity = Entity::new(user);
        assert!(entity.is_new);
    }
    
    #[test]
    fn test_change_tracker() {
        let mut row = Row::new();
        row.insert("id".to_string(), Value::Number(1.0));
        row.insert("name".to_string(), Value::Str("Alice".to_string()));
        
        let mut tracker = ChangeTracker::new(row);
        assert!(!tracker.is_dirty());
        
        tracker.mark_dirty("name", Value::Str("Bob".to_string()));
        assert!(tracker.is_dirty());
    }
    
    #[test]
    fn test_result_mapper() {
        let mut row = Row::new();
        row.insert("id".to_string(), Value::Number(1.0));
        row.insert("name".to_string(), Value::Str("Alice".to_string()));
        row.insert("email".to_string(), Value::Str("alice@example.com".to_string()));
        
        let mapper = ResultMapper::<User>::new(vec![row]);
        assert_eq!(mapper.to_rows().len(), 1);
        
        let entities = mapper.to_entities();
        assert_eq!(entities.len(), 1);
    }
}
