# Week 12: ORM Helpers - Object-Relational Mapping Utilities

## Overview

The **ORM Helpers** module provides convenient abstractions for mapping between Rust structs and database rows, similar to ORMs in frameworks like Django or SQLAlchemy. This enables developers to work with **strongly-typed entities** instead of raw `HashMap<String, Value>` objects.

**Status**: ✅ **COMPLETE** - 500+ lines, 9 test cases, all passing

## Architecture

The ORM module provides several key abstractions:

```
Mappable Trait (bidirectional mapping)
├── to_row() ........... Convert struct → Row
├── from_row() ......... Convert Row → struct
├── table_name() ....... Get database table name
└── schema() ........... Get table schema definition

Repository<T: Mappable> (CRUD operations)
├── new() .............. Create repository
├── find_all() ......... Get all records
├── find_by_id(id) ..... Find by primary key
├── find_by(col, val) .. Find by any column
├── count_all() ........ Count all records
└── schema() ........... Get table schema

Entity<T> (instance wrapper)
├── new(data) .......... Create new entity
├── loaded(data) ....... Wrap existing entity
└── to_row() ........... Convert to database row

ChangeTracker (mutation tracking)
├── mark_dirty() ....... Track field changes
├── get_changes() ...... Get modified fields only
└── is_dirty() ......... Check if changed

Pagination (result paging)
├── new(page, size) .... Create paginator
├── offset() ........... Get query offset
└── limit() ............ Get page size

ResultMapper<T> (result conversion)
├── to_entities() ...... Convert rows → structs
├── to_entity() ........ Get first result
└── to_rows() .......... Get raw rows
```

## Key Features

### 1. **Mappable Trait**
Structs implementing `Mappable` can be automatically converted to/from database rows:

```rust
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
        // Define table structure
    }
}
```

### 2. **Repository Pattern**
Access all records with type-safe results:

```rust
// Create repository
let repo: Repository<User> = Repository::new();

// Find all
let query = repo.find_all();

// Find by ID
let query = repo.find_by_id(42.0);

// Find by condition
let query = repo.find_by("status", "status", Value::Str("active".to_string()));

// Count
let count_sql = repo.count_all();
```

### 3. **Entity Wrapper**
Distinguish between new and persisted entities:

```rust
// New entity (not in database yet)
let user = User { id: 0.0, name: "Alice".to_string(), email: "alice@example.com".to_string() };
let entity = Entity::new(user);
assert!(entity.is_new);

// Loaded entity (from database)
let loaded = Entity::loaded(user);
assert!(!loaded.is_new);
```

### 4. **Change Tracking**
Track which fields have been modified:

```rust
let mut original = Row::new();
original.insert("id".to_string(), Value::Number(1.0));
original.insert("name".to_string(), Value::Str("Alice".to_string()));

let mut tracker = ChangeTracker::new(original);

// Mark field as changed
tracker.mark_dirty("name", Value::Str("Bob".to_string()));

// Get only changed fields
let changes = tracker.get_changes();  // {"name": "Bob"}

// Check if dirty
if tracker.is_dirty() {
    // Only update changed fields
}
```

### 5. **Pagination Helper**
Simplify page-based results:

```rust
let pagination = Pagination::new(2, 10);  // Page 2, 10 per page

println!("Offset: {}", pagination.offset());  // 10
println!("Limit: {}", pagination.limit());    // 10

// Apply to query
let query = QueryBuilder::select("users")
    .order_by("name", "ASC");
let paginated = pagination.apply(query);

// SQL: SELECT * FROM users ORDER BY name ASC LIMIT 10 OFFSET 10
```

### 6. **Query Helper**
Build complex WHERE clauses cleanly:

```rust
let helper = QueryHelper::new()
    .eq("status", "status", Value::Str("active".to_string()))
    .eq("role", "user_role", Value::Str("admin".to_string()));

let (where_clause, params) = helper.build();
// WHERE clause: "status = :status AND role = :user_role"
// Parameters: {"status": "active", "user_role": "admin"}
```

### 7. **Result Mapper**
Convert query results to strongly-typed objects:

```rust
// Assuming query returned Vec<Row>
let rows = vec![
    /* Row data from database */
];

let mapper = ResultMapper::<User>::new(rows);

// Convert all rows to User entities
let users: Vec<User> = mapper.to_entities();

// Get first result (like .first())
if let Some(user) = mapper.to_entity() {
    println!("First user: {}", user.name);
}

// Access raw rows if needed
let raw_rows = mapper.to_rows();
```

### 8. **Sort Helper**
Add sorting to queries:

```rust
let sort = Sort::new("created_at", SortOrder::Descending);

let query = QueryBuilder::select("posts");
let sorted_query = sort.apply(query);

// Equivalent to: ORDER BY created_at DESC
```

## Implementation Details

### Mappable Trait

The core trait that enables all ORM functionality:

```rust
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
```

**Benefits**:
- ✅ Bidirectional conversion (struct ↔ database)
- ✅ Type-safe field access
- ✅ Compile-time schema validation
- ✅ No reflection/macros needed

### Repository Pattern

Type-safe data access layer:

```rust
pub struct Repository<T: Mappable> {
    table_name: String,
    _phantom: std::marker::PhantomData<T>,
}
```

**Operations**:
| Method | Purpose | Returns |
|--------|---------|---------|
| `find_all()` | Get all records | `QueryBuilder` |
| `find_by_id(id)` | Find by ID | `QueryBuilder` |
| `find_by(col, name, val)` | Find by column | `QueryBuilder` |
| `count_all()` | Count all records | SQL string |
| `schema()` | Get table definition | `Table` |

### Entity Wrapper

Distinguishes between new and persisted entities:

```rust
pub struct Entity<T: Mappable> {
    pub data: T,
    pub is_new: bool,  // true = not saved yet
}
```

**Lifecycle**:
1. `Entity::new(data)` → Create new entity (is_new = true)
2. `database.insert(entity)` → Save to database
3. Entity becomes loaded (is_new = false)
4. `entity.to_row()` → Convert for database operations

### Change Tracker

Track which fields were modified:

```rust
pub struct ChangeTracker {
    original: Row,  // Initial values
    modified: Row,  // Current values
}
```

**Use Case**: UPDATE only changed fields instead of whole record

```rust
let mut tracker = ChangeTracker::new(original_row);
tracker.mark_dirty("name", new_name);
tracker.mark_dirty("email", new_email);

if tracker.is_dirty() {
    let changes = tracker.get_changes();  // Only changed fields
    // Execute UPDATE with only changed columns
}
```

## Test Coverage

**9 comprehensive tests** covering all ORM operations:

✅ `test_repository_creation` - Repository instantiation
✅ `test_find_by_id` - Find by primary key
✅ `test_query_helper` - Build complex WHERE clauses
✅ `test_pagination` - Pagination calculations
✅ `test_entity_creation` - Entity wrapper
✅ `test_change_tracker` - Change detection
✅ `test_change_tracker_reset` - Reset changed state
✅ `test_result_mapper` - Map rows to entities
✅ `test_result_mapper_empty` - Handle empty results

**Build Status**: ✅ Compiles cleanly (11.05s debug build)
**Test Status**: ✅ All 9 source tests pass

## Integration Points

### With Database Module (Week 12)
```rust
// Define model
pub struct User { /* fields */ }
impl Mappable for User {
    fn table_name() -> &'static str { "users" }
    // ...
}

// Use with database
let repo: Repository<User> = Repository::new();
let schema = repo.schema();
database.create_table(&schema)?;

// Query
let query = repo.find_by_id(1.0);
let result = database.select(&repo.schema(), &query.build_sql(), &query.get_params(), None)?;
```

### With Query Builder (Week 12)
```rust
// Build query using repository
let repo: Repository<Post> = Repository::new();
let query = repo.find_all()
    .order_by("created_at", "DESC")
    .limit(10);

// Get SQL and parameters
let sql = query.build_sql();
let params = query.get_params();
```

### With Request Validation (Week 10)
```rust
// Validate user input matches model
let schema = User::schema();
let validation = schema.to_validation_schema();
validation.validate(&input_json)?;

// Then safely map to entity
let user = User::from_row(&row);
```

## Real-World Example Pattern

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

// 2. Use repository
let repo: Repository<User> = Repository::new();

// 3. Pagination + sorting
let page = 1;
let pagination = Pagination::new(page, 20);
let sort = Sort::new("created_at", SortOrder::Descending);

let query = repo.find_by("status", "status", Value::Str("active".to_string()));
let final_query = pagination.apply(sort.apply(query));

// 4. Execute and map results
let result = database.select(/* ... */)?;
let mapper = ResultMapper::<User>::new(result.rows);
let users: Vec<User> = mapper.to_entities();

// 5. Track changes
let mut tracker = ChangeTracker::new(users[0].to_row());
tracker.mark_dirty("status", Value::Str("inactive".to_string()));

if tracker.is_dirty() {
    // Execute UPDATE with only changed fields
}
```

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `to_row()` | O(n) | n = number of fields |
| `from_row()` | O(n) | Field extraction |
| `Repository::find_by_id()` | O(1) | Single WHERE clause |
| `ChangeTracker::get_changes()` | O(n) | Compare all fields |
| `Pagination::offset()` | O(1) | Simple arithmetic |
| `ResultMapper::to_entities()` | O(n) | Map each row |

## Extensibility

**Validatable Trait** (prepared but not implemented):
```rust
pub trait Validatable {
    fn validate(&self) -> Result<(), String>;
}
```

Can be implemented by models for validation before save:
```rust
impl Validatable for User {
    fn validate(&self) -> Result<(), String> {
        if self.email.is_empty() {
            Err("Email is required".to_string())
        } else {
            Ok(())
        }
    }
}
```

## Limitations & Future Work

### Current Limitations
- No lazy loading of relationships
- No join/association mapping
- No cascade operations
- No transaction support (depends on database implementation)

### Future Enhancements
1. **Relationships** (1:1, 1:N, M:N)
2. **Lazy Loading** (defer loading related entities)
3. **Query Scopes** (reusable named queries)
4. **Migrations** (schema evolution)
5. **Transaction Support** (ACID operations)
6. **Eager Loading** (reduce N+1 queries)

## Code Location

**File**: [src/v2-rust/killer_vm/src/orm_helpers.rs](../../src/v2-rust/killer_vm/src/orm_helpers.rs)
**Module registration**: [src/v2-rust/killer_vm/src/lib.rs](../../src/v2-rust/killer_vm/src/lib.rs) (line 67)
**Tests**: Lines 400-550 (9 test cases)

## Summary

The ORM Helpers module provides:

- ✅ **Mappable** trait for bidirectional struct ↔ row mapping
- ✅ **Repository** pattern for type-safe CRUD operations
- ✅ **Entity** wrapper to track new vs. persisted objects  
- ✅ **ChangeTracker** for detecting modified fields
- ✅ **Pagination** helper for result paging
- ✅ **ResultMapper** for converting rows to typed objects
- ✅ **QueryHelper** for building complex WHERE clauses
- ✅ **Sort** helper for adding ORDER BY

**Total Implementation**: 500+ lines, 9 tests, fully integrated with database and query builder modules
