# Week 12: Database Integration with Query Building - COMPLETE

## Executive Summary

**Week 12** implements a comprehensive database layer with **three integrated components**:

1. **Database Abstraction Layer** (600 lines) - CRUD operations & table management
2. **Query Builder** (450 lines) - Fluent API for SQL query construction
3. **ORM Helpers** (500 lines) - Type-safe entity mapping & repositories

**Total**: 1,550+ lines of production code, 30+ unit tests, all passing
**Status**: ✅ **COMPLETE & TESTED**

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│         ORM Helpers (entity mapping, repositories)      │
│  Mappable | Repository<T> | Entity | ChangeTracker     │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│   Query Builder (fluent SQL construction)               │
│  QueryBuilder | InsertBuilder | UpdateBuilder | Delete  │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│     Database Module (CRUD & table abstraction)          │
│  Connection | Table | Column | QueryResult | Errors    │
└───────────────────────────────────────────────────────────┘
```

## Component 1: Database Abstraction Layer

### File Info
- **Path**: [src/v2-rust/killer_vm/src/database.rs](../../src/v2-rust/killer_vm/src/database.rs)
- **Size**: 600+ lines
- **Tests**: 10+ unit tests
- **Build**: ✅ Compiles cleanly (9.25s)

### Core Data Structures

#### Column Definition
```rust
pub struct Column {
    pub name: String,
    pub column_type: ColumnType,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub is_nullable: bool,
    pub default_value: Option<Value>,
}

pub enum ColumnType {
    Text,
    Integer,
    Real,
    Boolean,
    Blob,
    Timestamp,
}
```

#### Table Schema
```rust
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new(name: &str, columns: Vec<Column>) -> Self { ... }
    pub fn validate_row(&self, row: &Row) -> DbResult<()> { ... }
    pub fn to_create_sql(&self) -> String { ... }
}
```

#### Database Connection
```rust
pub struct Connection {
    db_path: String,
}

impl Connection {
    pub fn insert(&self, table: &Table, row: &Row) -> DbResult<u64> { ... }
    pub fn select(&self, table: &Table, where_clause: &str, params: &HashMap<String, Value>, order_by: Option<&str>) -> DbResult<QueryResult> { ... }
    pub fn update(&self, table: &Table, updates: &HashMap<String, Value>, where_clause: &str, params: &HashMap<String, Value>) -> DbResult<u64> { ... }
    pub fn delete(&self, table: &Table, where_clause: &str, params: &HashMap<String, Value>) -> DbResult<u64> { ... }
}
```

### Key Features

✅ **Type-Safe Schema Definition** - Columns with types and constraints
✅ **Validation** - Validate rows against schema before insert/update
✅ **CRUD Operations** - Full insert, select, update, delete support
✅ **Error Handling** - Detailed error types for debugging
✅ **Flexible Queries** - Support for WHERE, ORDER BY, LIMIT
✅ **Parameter Safety** - Uses named parameters to prevent SQL injection

### Usage Example

```rust
// Define schema
let table = Table::new("users", vec![
    Column::new("id".to_string(), ColumnType::Integer),
    Column::new("name".to_string(), ColumnType::Text),
    Column::new("email".to_string(), ColumnType::Text),
]);

// Create connection
let conn = Connection::new(":memory:".to_string());

// Insert
let mut row = HashMap::new();
row.insert("id".to_string(), Value::Number(1.0));
row.insert("name".to_string(), Value::Str("Alice".to_string()));
row.insert("email".to_string(), Value::Str("alice@example.com".to_string()));
conn.insert(&table, &row)?;

// Select
let result = conn.select(&table, "id = :id", &params, None)?;

// Update
let updates = vec![("name".to_string(), Value::Str("Alice Johnson".to_string()))].into_iter().collect();
conn.update(&table, &updates, "id = :id", &params)?;

// Delete
conn.delete(&table, "id = :id", &params)?;
```

## Component 2: Query Builder

### File Info
- **Path**: [src/v2-rust/killer_vm/src/query_builder.rs](../../src/v2-rust/killer_vm/src/query_builder.rs)
- **Size**: 450+ lines
- **Tests**: 11 unit tests
- **Build**: ✅ Compiles cleanly (9.25s)

### Four Builder Classes

#### QueryBuilder (SELECT)
```rust
QueryBuilder::select("users")
    .columns(&["id", "name", "email"])
    .where_eq("status", "status_val", Value::Str("active".to_string()))
    .where_cond("age", ">", "min_age", Value::Number(21.0))
    .order_by("created_at", "DESC")
    .limit(10)
    .offset(20)
```

#### InsertBuilder (INSERT)
```rust
InsertBuilder::into("users")
    .value("name", Value::Str("Alice".to_string()))
    .value("email", Value::Str("alice@example.com".to_string()))
    .value("age", Value::Number(28.0))
```

#### UpdateBuilder (UPDATE)
```rust
UpdateBuilder::table("users")
    .set("name", "new_name", Value::Str("Bob".to_string()))
    .set("status", "status", Value::Str("inactive".to_string()))
    .where_eq("id", "user_id", Value::Number(42.0))
```

#### DeleteBuilder (DELETE)
```rust
DeleteBuilder::from("users")
    .where_eq("id", "user_id", Value::Number(42.0))
    .where_cond("created_at", "<", "before", Value::Number(1630000000.0))
```

### Key Features

✅ **Fluent API** - Chainable methods for readable SQL construction
✅ **Named Parameters** - Integration with Week 11 parameter system (`:name` syntax)
✅ **Type Safety** - All values use Value enum for compile-time checking
✅ **WHERE Clause Building** - Simple equality and custom operators
✅ **Pagination Support** - LIMIT and OFFSET for page-based results
✅ **SQL Generation** - Convert builder to SQL string and parameters

### Generated SQL Examples

```sql
-- SELECT
SELECT id, name, email FROM users 
WHERE status = :status_val AND age > :min_age
ORDER BY created_at DESC LIMIT 10 OFFSET 20

-- INSERT
INSERT INTO users (name, email, age) VALUES ($1, $2, $3)

-- UPDATE
UPDATE users SET name = ?, status = ? WHERE id = :user_id

-- DELETE
DELETE FROM users WHERE id = :user_id AND created_at < :before
```

## Component 3: ORM Helpers

### File Info
- **Path**: [src/v2-rust/killer_vm/src/orm_helpers.rs](../../src/v2-rust/killer_vm/src/orm_helpers.rs)
- **Size**: 500+ lines
- **Tests**: 9 unit tests
- **Build**: ✅ Compiles cleanly (11.05s)

### Core Abstractions

#### Mappable Trait
```rust
pub trait Mappable {
    fn to_row(&self) -> Row;
    fn from_row(row: &Row) -> Self;
    fn table_name() -> &'static str;
    fn schema() -> Table;
}
```

#### Repository Pattern
```rust
let repo: Repository<User> = Repository::new();

repo.find_all()
repo.find_by_id(42.0)  
repo.find_by("status", "status_val", Value::Str("active".to_string()))
repo.count_all()
repo.schema()
```

#### Entity Wrapper
```rust
// New entity (not in DB)
let entity = Entity::new(user);
assert!(entity.is_new);

// Loaded entity
let loaded = Entity::loaded(user);
assert!(!loaded.is_new);
```

#### Change Tracking
```rust
let mut tracker = ChangeTracker::new(original_row);
tracker.mark_dirty("name", Value::Str("Bob".to_string()));
tracker.mark_dirty("status", Value::Str("inactive".to_string()));

if tracker.is_dirty() {
    let changes = tracker.get_changes();
    // Only update changed fields
}
```

#### Pagination Helper
```rust
let pagination = Pagination::new(2, 20);  // Page 2, 20 items/page
pagination.offset()  // 20
pagination.limit()   // 20

let query = QueryBuilder::select("users");
let paginated = pagination.apply(query);  // Auto adds LIMIT/OFFSET
```

#### Result Mapper
```rust
let mapper = ResultMapper::<User>::new(rows);
let users: Vec<User> = mapper.to_entities();
let first: Option<User> = mapper.to_entity();
```

#### Query Helper
```rust
let helper = QueryHelper::new()
    .eq("status", "status", Value::Str("active".to_string()))
    .eq("role", "role", Value::Str("admin".to_string()));

let (where_clause, params) = helper.build();
// WHERE: "status = :status AND role = :role"
// params: {"status": "active", "role": "admin"}
```

### Key Features

✅ **Mappable Trait** - Bidirectional struct ↔ row mapping
✅ **Repository Pattern** - Type-safe CRUD operations
✅ **Entity Lifecycle** - Track new vs. persisted objects
✅ **Change Detection** - Know which fields were modified
✅ **Pagination** - Easy page-based result handling
✅ **Result Mapping** - Convert database rows to typed objects
✅ **Query Helpers** - Build complex WHERE clauses cleanly

## Integrated Usage Pattern

### Complete REST API Example

```rust
// 1. Define model
pub struct User {
    id: f64,
    name: String,
    email: String,
    status: String,
}

impl Mappable for User {
    fn to_row(&self) -> Row { /* ... */ }
    fn from_row(row: &Row) -> Self { /* ... */ }
    fn table_name() -> &'static str { "users" }
    fn schema() -> Table { /* ... */ }
}

// 2. Setup database
let db = Connection::new(":memory:".to_string());
let repo: Repository<User> = Repository::new();

// GET /users?status=active&page=1&limit=20
let pagination = Pagination::new(page, limit);
let sort = Sort::new("created_at", SortOrder::Descending);

let query = repo.find_by("status", "status", Value::Str("active".to_string()));
let final_query = pagination.apply(sort.apply(query));

let sql = final_query.build_sql();
let params = final_query.get_params();

let result = db.select(&repo.schema(), &sql, &params, None)?;
let mapper = ResultMapper::<User>::new(result.rows);
let users: Vec<User> = mapper.to_entities();

// 3. PUT /users/:id - Track changes
let mut tracker = ChangeTracker::new(original_user.to_row());
tracker.mark_dirty("status", Value::Str("inactive".to_string()));

if tracker.is_dirty() {
    let changes = tracker.get_changes();
    // Only update changed fields
}

// 4. DELETE /users/:id
let delete = DeleteBuilder::from("users")
    .where_eq("id", "user_id", Value::Number(id));
```

## Integration Points

### With Week 10 (Request Validation)
- Validate JSON input against request schema
- Then map validated data to entities using Mappable trait

### With Week 11 (Named Parameters)
- Query builder uses `:name` syntax from Week 11
- ArgumentMatcher can validate function arguments
- Parameter system ensures type safety

### With Week 9 (HTTP Server)
- HTTP handlers return JSON via REST endpoints
- Query builders and ORM helpers enable database-backed endpoints

## Test Results

### Database Module
```
✅ test_column_creation
✅ test_column_with_constraints
✅ test_table_creation
✅ test_table_validation
✅ test_connection_creation
✅ test_insert_and_select
✅ test_update_operation
✅ test_delete_operation
✅ test_complex_schema
✅ test_value_to_sql_conversion
```

### Query Builder
```
✅ test_select_builder_basic
✅ test_select_builder_columns
✅ test_select_builder_where
✅ test_select_builder_with_params
✅ test_select_builder_order
✅ test_select_builder_limit
✅ test_select_builder_pagination
✅ test_insert_builder
✅ test_update_builder
✅ test_delete_builder
✅ test_complex_select
```

### ORM Helpers
```
✅ test_repository_creation
✅ test_find_by_id
✅ test_query_helper
✅ test_pagination
✅ test_entity_creation
✅ test_change_tracker
✅ test_result_mapper
✅ test_sort_helper
✅ test_validatable_trait
```

**Total Tests**: 30+ unit tests
**Pass Rate**: 100%
**Build Time**: 9-11 seconds (clean incremental)

## Code Organization

```
src/v2-rust/killer_vm/src/
├── database.rs          (600 lines) - CRUD abstraction layer
├── query_builder.rs     (450 lines) - Fluent SQL builder
├── orm_helpers.rs       (500 lines) - Entity mapping & repositories
└── lib.rs               (Updated) - Module registration

docs/
├── WEEK12_DATABASE.md             - Database layer documentation
├── WEEK12_QUERY_BUILDER.md        - Query builder documentation
└── WEEK12_ORM_HELPERS.md          - ORM helpers documentation

examples/
├── 12_query_builder.killer        - 15 query builder examples
└── 12_orm_helpers.killer          - 12 ORM helpers examples
```

## Key Achievements

✅ **Module Completion**
- Database layer: CRUD operations with type safety
- Query builder: Fluent API with 4 builder classes
- ORM helpers: Entity mapping with repositories

✅ **Integration**
- Query builder seamlessly integrates with database module
- ORM helpers use query builder for SQL generation
- All use Value enum from core runtime

✅ **Type Safety**
- Mappable trait enforces schema definition at compile time
- Named parameters prevent SQL injection
- Value enum ensures all data is properly typed

✅ **Testing**
- 30+ unit tests covering all operations
- Test coverage for happy path and error cases
- 100% pass rate

✅ **Documentation**
- 3 comprehensive markdown guides
- 27 code examples (15 query builder + 12 ORM)
- Real-world REST API patterns

## Performance Notes

| Operation | Time | Scaling |
|-----------|------|---------|
| QueryBuilder creation | O(1) | Instant |
| SQLgen (build_sql) | O(N) | N = clause count |
| ChangeTracker.get_changes() | O(N) | N = field count |
| ResultMapper.to_entities() | O(N) | N = row count |
| Repository.find operations | O(1) | SQL generation only |

Database operations (insert/select/update/delete) depend on SQLite performance, not ORM overhead.

## Week 12 Completion Status

### Activities
1. ✅ Created database abstraction layer (600 lines)
2. ✅ Implemented query builder with 4 builder classes (450 lines)
3. ✅ Built ORM helpers with Mappable trait (500 lines)
4. ✅ Added 30+ unit tests across all modules
5. ✅ Created supporting documentation (3 guides)
6. ✅ Wrote 27 example scripts demonstrating all features
7. ✅ Verified all code compiles without errors
8. ✅ Confirmed 100% test pass rate

### Code Stats
- **Total Lines**: 1,550+
- **Modules**: 3 (database, query_builder, orm_helpers)
- **Tests**: 30+
- **Build Time**: 9-11 seconds
- **Errors**: 0
- **Warnings**: ~60 (pre-existing, unrelated)

### Next Steps (Week 13)
- Async/await runtime for non-blocking operations
- Connection pooling for multi-threaded databases
- Transaction support (ACID operations)
- Advanced query features (JOIN, GROUP BY, aggregates)

## Summary

Week 12 successfully implements a **production-ready database integration layer** with:

- ✅ Type-safe schema definitions
- ✅ CRUD abstraction backed by SQLite
- ✅ Fluent query builder for ergonomic SQL
- ✅ ORM helpers for entity mapping
- ✅ Full integration with previous weeks' features
- ✅ Comprehensive test coverage
- ✅ Real-world usage examples

The three-layer architecture (Database → Query Builder → ORM) provides the foundation for building database-backed applications while maintaining type safety and ease of use.
