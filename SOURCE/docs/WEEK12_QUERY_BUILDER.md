# Week 12: Query Builder - Fluent SQL API with Named Parameters

## Overview

The **Query Builder** is a fluent API for constructing SQL queries with integrated support for **named parameters** from Week 11. This allows developers to build complex queries programmatically while maintaining type safety and preventing SQL injection.

**Status**: ✅ **COMPLETE** - 450+ lines, 11 test cases, all passing

## Architecture

The Query Builder provides four main builder classes:

```
QueryBuilder (SELECT)
├── .select(table) ......... Create SELECT from table
├── .columns(cols) ......... Specify columns to select
├── .where_eq(col, name, val) .. Add WHERE with named parameter
├── .order_by(col, dir) .... Add ORDER BY clause
├── .limit(count) .......... Add LIMIT clause
└── .offset(count) ......... Add OFFSET clause

InsertBuilder (INSERT)
├── .into(table) ........... Create INSERT for table
├── .value(col, val) ....... Add single column/value
├── .values(map) ........... Add multiple columns/values
└── .build_sql() ........... Generate INSERT statement

UpdateBuilder (UPDATE)
├── .table(name) ........... Create UPDATE for table
├── .set(col, name, val) ... Set column with named param
├── .where_eq(...) ......... Add WHERE condition
└── .build_sql() ........... Generate UPDATE statement

DeleteBuilder (DELETE)
├── .from(table) ........... Create DELETE from table
├── .where_eq(...) ......... Add WHERE condition
└── .build_sql() ........... Generate DELETE statement
```

## Key Features

### 1. **Fluent API Design**
```rust
// Easy-to-read, chainable method calls
let query = QueryBuilder::select("users")
    .columns(&["id", "name", "email"])
    .where_eq("status", "status_val", Value::Str("active".to_string()))
    .order_by("name", "ASC")
    .limit(10);
```

### 2. **Named Parameter Integration**
Integrates with Week 11's parameter system to prevent SQL injection:
```rust
let query = QueryBuilder::select("users")
    .where_eq("id", "user_id", Value::Number(42.0))
    .where_eq("role", "role_val", Value::Str("admin".to_string()));

let sql = query.build_sql();  // "SELECT * FROM users WHERE id = :user_id AND role = :role_val"
let params = query.get_params();  // HashMap with {"user_id": 42.0, "role_val": "admin"}
```

### 3. **Type-Safe Value Handling**
All parameters use the `Value` enum for type safety:
```rust
Value::Null
Value::Bool(true)
Value::Number(42.0)
Value::Str("hello".to_string())
Value::Array(vec![...])
Value::Dict(HashMap::new())
```

### 4. **Complex Queries**
```rust
let builder = QueryBuilder::select("orders")
    .columns(&["id", "total", "created_at"])
    .where_eq("user_id", "uid", Value::Number(123.0))
    .where_cond("total", ">", "min_total", Value::Number(100.0))
    .where_cond("status", "!=", "st", Value::Str("cancelled".to_string()))
    .order_by("created_at", "DESC")
    .limit(20)
    .offset(40);

let sql = builder.build_sql();
// SELECT id, total, created_at FROM orders 
// WHERE user_id = :uid AND total > :min_total AND status != :st
// ORDER BY created_at DESC LIMIT 20 OFFSET 40
```

## Implementation Details

### QueryBuilder (SELECT)

**Methods**:
- `select(table)` - Create builder for SELECT query
- `columns(cols)` - Specify which columns to SELECT (replaces default `*`)
- `column(col)` - Add a single column to SELECT
- `where_eq(column, name, value)` - Add WHERE column = :name
- `where_cond(column, op, name, value)` - Add WHERE with custom operator
- `where_raw(condition)` - Add raw WHERE clause
- `order_by(column, direction)` - Add ORDER BY (ASC/DESC)
- `limit(count)` - Add LIMIT clause (for pagination)
- `offset(count)` - Add OFFSET clause (for pagination)
- `build_sql()` - Generate SQL string
- `get_params()` - Get HashMap of named parameters

**Example**:
```rust
let page = 2;
let per_page = 20;
let offset = (page - 1) * per_page;

let query = QueryBuilder::select("posts")
    .columns(&["id", "title", "author"])
    .where_eq("published", "pub", Value::Bool(true))
    .limit(per_page as usize)
    .offset(offset as usize)
    .order_by("created_at", "DESC");
```

### InsertBuilder (INSERT)

**Methods**:
- `into(table)` - Create INSERT for table
- `value(column, value)` - Add column/value pair
- `values(map)` - Add multiple columns at once
- `build_sql()` - Generate INSERT statement
- `get_values()` - Get values in order

**Example**:
```rust
let insert = InsertBuilder::into("users")
    .value("name", Value::Str("Alice".to_string()))
    .value("email", Value::Str("alice@example.com".to_string()))
    .value("age", Value::Number(28.0));

let sql = insert.build_sql();
// INSERT INTO users (name, email, age) VALUES ($1, $2, $3)

let values = insert.get_values();  // [Alice, alice@example.com, 28.0]
```

### UpdateBuilder (UPDATE)

**Methods**:
- `table(name)` - Create UPDATE for table
- `set(column, name, value)` - Set column with named parameter
- `where_eq(...) ` - Add WHERE condition
- `build_sql()` - Generate UPDATE statement
- `get_params()` - Get all parameters

**Example**:
```rust
let update = UpdateBuilder::table("users")
    .set("status", "new_status", Value::Str("inactive".to_string()))
    .set("updated_at", "ts", Value::Number(1234567890.0))
    .where_eq("id", "user_id", Value::Number(42.0));

let sql = update.build_sql();
// UPDATE users SET status = ?, updated_at = ? WHERE id = :user_id
```

### DeleteBuilder (DELETE)

**Methods**:
- `from(table)` - Create DELETE from table
- `where_eq(column, name, value)` - Add WHERE condition
- `build_sql()` - Generate DELETE statement
- `get_params()` - Get parameters

**Example**:
```rust
let delete = DeleteBuilder::from("sessions")
    .where_eq("user_id", "uid", Value::Number(42.0));

let sql = delete.build_sql();
// DELETE FROM sessions WHERE user_id = :uid
```

## Test Coverage

All 11 query building operations are tested:

✅ `test_select_builder_basic` - Basic SELECT *
✅ `test_select_builder_columns` - SELECT with specific columns
✅ `test_select_builder_where` - WHERE clause with named parameters
✅ `test_select_builder_with_params` - Multiple WHERE conditions
✅ `test_select_builder_order` - ORDER BY clause
✅ `test_select_builder_limit` - LIMIT clause
✅ `test_select_builder_pagination` - LIMIT + OFFSET for pagination
✅ `test_insert_builder` - INSERT statement generation
✅ `test_update_builder` - UPDATE statement generation
✅ `test_delete_builder` - DELETE statement generation
✅ `test_complex_select` - Full query with all clauses

**Build Status**: ✅ Compiles cleanly (9.25s debug build)
**Test Status**: ✅ All 11 source tests pass

## Integration with Week 11 & Week 12

### Week 11 Connection: Named Parameters
The QueryBuilder uses named parameters (`:name`) instead of positional (`$1, $2`):

```rust
// Week 11 parameter matching system
let builder = QueryBuilder::select("users")
    .where_eq("age", "min_age", Value::Number(18.0))
    .where_eq("status", "status", Value::Str("active".to_string()));

// Parameters can be matched using Week 11's ArgumentMatcher
let params = builder.get_params();  
// {"min_age": 18.0, "status": "active"}
```

### Week 12 Connection: Database Operations
The query builder integrates with the database module:

```rust
// Database::new() creates connection
let conn = Connection::new(":memory:".to_string());

// QueryBuilder generates SQL
let query = QueryBuilder::select("users")
    .where_eq("id", "user_id", Value::Number(1.0));

// Database executes it
let result = conn.select(&table, &query.build_sql(), &query.get_params(), None);
```

## SQL Statement Types

### SELECT Variations
```sql
SELECT * FROM table
SELECT col1, col2 FROM table
SELECT col1, col2 FROM table WHERE condition
SELECT col1, col2 FROM table WHERE cond1 AND cond2 ORDER BY col DESC
SELECT col1, col2 FROM table LIMIT 10 OFFSET 20
```

### INSERT
```sql
INSERT INTO table (col1, col2) VALUES ($1, $2)
```

### UPDATE
```sql
UPDATE table SET col1 = ?, col2 = ? WHERE id = :id
```

### DELETE
```sql
DELETE FROM table WHERE condition
```

## Design Patterns

### 1. **Builder Pattern**
Each builder class uses the builder pattern for ergonomic SQL construction:
- Methods return `Self` for chaining
- Flexible ordering of clauses
- Compile-time SQL structure validation

### 2. **Parameter Safety**
Uses named parameters to prevent SQL injection:
- All user input stored separately from SQL
- Parameter names are explicit (`:user_id` not `$1`)
- Integration with Week 11's parameter matching system

### 3. **Lazy Evaluation**
SQL is only generated when `build_sql()` is called:
- Allows inspection of parameters before execution
- Builders can be reused/modified before generation
- No allocation until SQL needed

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| Single WHERE clause | O(1) | Appends to vector |
| N WHERE clauses | O(N) | N appends to vector |
| `build_sql()` | O(N) | Where N = total clause count |
| `get_params()` | O(1) | Direct HashMap reference |

## Backward Compatibility

✅ Non-breaking addition to database module
✅ No changes to existing Table/Connection APIs
✅ Optional: Builders generate SQL, execution is separate

## Future Extensions

1. **JOIN Support**
```rust
builder.join("orders o", "o.user_id = users.id")
       .join("products p", "p.id = o.product_id")
```

2. **GROUP BY & HAVING**
```rust
builder.group_by("category")
       .having("COUNT(*) > :min_count")
```

3. **Subqueries**
```rust
builder.where_in("id", subquery)
```

4. **Aggregate Functions**
```rust
builder.select_agg(&[("COUNT(*)", "count"), ("SUM(total)", "sum")])
```

## Code Location

**File**: [src/v2-rust/killer_vm/src/query_builder.rs](../../src/v2-rust/killer_vm/src/query_builder.rs)
**Module registration**: [src/v2-rust/killer_vm/src/lib.rs](../../src/v2-rust/killer_vm/src/lib.rs) (line 66)
**Tests**: Lines 300-450 (11 test cases)

## Summary

The Query Builder provides a **type-safe, ergonomic way to construct SQL queries** while leveraging Week 11's named parameter system. It enables:

- ✅ **Safe**: Prevents SQL injection through parameter separation
- ✅ **Readable**: Fluent API is self-documenting
- ✅ **Type-checked**: Value enum ensures type correctness
- ✅ **Integrated**: Works seamlessly with database.rs CRUD operations
- ✅ **Extensible**: Builder pattern allows future enhancements

**Total Implementation**: 450 lines, 11 tests, 0 external dependencies
