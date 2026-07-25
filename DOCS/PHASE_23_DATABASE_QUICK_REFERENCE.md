# PHASE 23 DATABASE - QUICK REFERENCE GUIDE

**Get started with Killer database connectivity in 5 minutes**

---

## 30-Second Overview

Three new modules ready to use:

```rust
// MongoDB - Document database
use killer_rcore::stdlib_impl::database_mongodb::*;
let conn = MongoSolver::mongodb_connect("mongodb://localhost:27017")?;
let result = MongoSolver::insert_one(&conn, "db", "coll", &doc)?;

// PostgreSQL - Relational database  
use killer_rcore::stdlib_impl::database_postgresql::*;
let conn = PostgresSolver::postgres_connect("postgresql://user:pass@localhost:5432/db")?;
let rows = PostgresSolver::query_all(&conn, "SELECT * FROM users")?;

// Query Builder - Database agnostic
use killer_rcore::stdlib_impl::database_query::*;
let q = QueryBuilderSolver::select("users");
let q = QueryBuilderSolver::where_eq(q, "age", FilterValue::Int(25));
let sql = QueryBuilderSolver::to_sql(&q);
```

---

## Module Directory

### MongoDB (`database_mongodb`)
**42 functions** for NoSQL document operations

Quick finds:
- **Connections:** `mongodb_connect`, `connection_pool` 
- **CRUD:** `find_one`, `insert_one`, `update_one`, `delete_one`
- **Bulk:** `insert_many`, `update_many`, `bulk_write`
- **Aggregation:** `aggregate`, `agg_match`, `agg_sort`
- **Indexes:** `create_index`, `create_unique_index`
- **Tools:** `count_documents`, `exists`, `drop_collection`

### PostgreSQL (`database_postgresql`)
**45 functions** for SQL database operations

Quick finds:
- **Connections:** `postgres_connect`, `connection_pool`
- **Queries:** `query`, `query_with_params`, `query_one`, `count`
- **Statements:** `prepare`, `execute_prepared`, `deallocate`
- **DDL:** `create_table`, `add_column`, `create_index`
- **Transactions:** `begin_transaction`, `commit_transaction`, `rollback_transaction`
- **Schema:** `list_tables`, `list_columns`, `table_info`

### Query Builder (`database_query`)
**40 functions** for database-agnostic queries

Quick finds:
- **Build:** `select`, `select_fields`, `count`, `sum`
- **Filter:** `where_eq`, `where_gt`, `where_in`, `where_like`
- **Sort:** `order_by_asc`, `order_by_desc`
- **Page:** `paginate`, `limit`, `offset`
- **Join:** `inner_join`, `left_join`, `right_join`
- **Compile:** `to_sql`, `validate_query`
- **Map:** `map_result`, `filter_result`, `take`, `skip`

---

## How to Find What You Need

### "I want to..."

**Read from database:**
- MongoDB: `find_one()`, `find_many()`
- PostgreSQL: `query_one()`, `query_all()`, `count()`
- Query Builder: `select()` → `where_*()` → `to_sql()`

**Write to database:**
- MongoDB: `insert_one()`, `insert_many()`, `update_one()`, `delete_one()`
- PostgreSQL: `execute()` with INSERT/UPDATE/DELETE
- Bulk: `bulk_insert()`, `bulk_update()`, `bulk_delete()`

**Speed up similar queries:**
- PostgreSQL: `prepare()` → `execute_prepared()`
- Connection pool: `connection_pool()` → `get_connection()`

**Complex queries:**
- MongoDB aggregation: `aggregate()` + `agg_match()` + `agg_sort()`
- PostgreSQL joins: SQL with JOIN syntax
- Query Builder: `left_join()` + filters

**Make data faster:**
- MongoDB: `create_index()`, `create_compound_index()`
- PostgreSQL: `create_index()` with B-tree/partial/GiST

**Guarantee data consistency:**
- Transactions: `begin_transaction()` → `commit_transaction()`
- PostgreSQL savepoints: `savepoint()` → `rollback_to_savepoint()`

**Explore schema:**
- PostgreSQL: `list_tables()`, `list_columns()`, `table_info()`
- MongoDB: `list_collections()`, `collection_stats()`

---

## 5 Real-World Usage Patterns

### Pattern 1: Connection Pool Setup
```rust
use killer_rcore::stdlib_impl::database_mongodb::*;

// Create pool for concurrency
let conn = MongoSolver::connection_pool(
    "mongodb://localhost:27017", 
    10  // 10 concurrent connections
)?;

// Check status
let (total, available, in_use) = MongoSolver::pool_status(&conn);
println!("Pool: {} total, {} available", total, available);

// Get connection when needed
let c = MongoSolver::get_connection(&conn)?;
// ... do work ...
MongoSolver::release_connection(&conn, c)?;
```

### Pattern 2: Document Insertion & Query
```rust
use killer_rcore::stdlib_impl::database_mongodb::*;
use std::collections::HashMap;

let conn = MongoSolver::mongodb_connect("mongodb://localhost:27017")?;

// Insert
let mut doc = HashMap::new();
doc.insert("name".to_string(), Value::String("Alice".to_string()));
doc.insert("age".to_string(), Value::Int32(30));
let result = MongoSolver::insert_one(&conn, "mydb", "users", &doc)?;
println!("Inserted: {:?}", result.inserted_ids);

// Query back
let mut filter = HashMap::new();
filter.insert("name".to_string(), Value::String("Alice".to_string()));
let user = MongoSolver::find_one(&conn, "mydb", "users", &filter)?;
println!("Found: {:?}", user);
```

### Pattern 3: Prepared Statements (Performance)
```rust
use killer_rcore::stdlib_impl::database_postgresql::*;

let conn = PostgresSolver::postgres_connect(
    "postgresql://localhost:5432/mydb"
)?;

// Prepare once
let stmt = PostgresSolver::prepare(
    &conn,
    "SELECT id, name, email FROM users WHERE age > $1"
)?;

// Execute many times (efficient!)
for threshold in vec![18, 21, 30, 65] {
    let result = PostgresSolver::query_prepared(
        &conn,
        &stmt,
        &[PostgresValue::Int32(threshold)]
    )?;
    println!("Users over {}: {} rows", threshold, result.rows.len());
}
```

### Pattern 4: Transactions for MultiStep Operations
```rust
use killer_rcore::stdlib_impl::database_postgresql::*;

let conn = PostgresSolver::postgres_connect("postgresql://localhost:5432/db")?;

let mut txn = PostgresSolver::begin_transaction(&conn)?;
PostgresSolver::set_isolation_level(&conn, "SERIALIZABLE")?;

// Step 1: Debit account
let _ = PostgresSolver::execute(&conn, 
    "UPDATE accounts SET balance = balance - 100 WHERE id = 1"
)?;

// Step 2: Credit account  
let _ = PostgresSolver::execute(&conn,
    "UPDATE accounts SET balance = balance + 100 WHERE id = 2"
)?;

// Commit both or neither
PostgresSolver::commit_transaction(&mut txn)?;
println!("Transfer complete!");
```

### Pattern 5: Query Builder (Flexible Queries)
```rust
use killer_rcore::stdlib_impl::database_query::*;

// Dynamic query construction
let mut q = QueryBuilderSolver::select("products");

// Add filters based on conditions
if let Some(min_price) = min_price {
    q = QueryBuilderSolver::where_gt(q, "price", FilterValue::Float(min_price));
}

if let Some(categories) = categories {
    q = QueryBuilderSolver::where_in(q, "category", 
        categories.iter().map(|c| FilterValue::String(c.clone())).collect()
    );
}

// Sort & paginate
q = QueryBuilderSolver::order_by_desc(q, "rating");
q = QueryBuilderSolver::paginate(q, page, 20);

// Generate SQL
let sql = QueryBuilderSolver::to_sql(&q);
println!("Query: {}", sql);
// Can execute against any SQL database!
```

---

## Quick Integration Checklist

- [ ] Import database module: `use killer_rcore::stdlib_impl::database_*::*;`
- [ ] Create connection: `mongodb_connect()` or `postgres_connect()`
- [ ] (Optional) Create pool: `connection_pool()` for concurrent access
- [ ] Execute operations: `find_one()`, `query()`, etc.
- [ ] Handle results with `Result<T, Error>` pattern
- [ ] Clean up: connection returned to pool automatically on drop
- [ ] Test with localhost before production
- [ ] Use connection pooling in production (not single connection)

---

## Key Functions by Use Case

| Use Case | Function | Module |
|---|---|---|
| Connect | `mongodb_connect` | MongoDB |
| | `postgres_connect` | PostgreSQL |
| Read single | `find_one` | MongoDB |
| | `query_one` | PostgreSQL |
| Read multiple | `find_many` | MongoDB |
| | `query_all` | PostgreSQL |
| Write | `insert_one` | MongoDB |
| | `execute` | PostgreSQL |
| Update | `update_one` | MongoDB |
| | `execute` | PostgreSQL |
| Delete | `delete_one` | MongoDB |
| | `execute` | PostgreSQL |
| Batch write | `bulk_write` | MongoDB |
| | `bulk_insert` | PostgreSQL |
| Fast queries | `prepare` | PostgreSQL |
| Aggregation | `aggregate` | MongoDB |
| Transactions | `begin_transaction` | PostgreSQL |
| Indexes | `create_index` | Both |
| Schema | `list_tables` | PostgreSQL |
| Builder | `select` → `to_sql` | Query |

---

## Common Q&A

**Q: Should I use MongoDB or PostgreSQL?**  
A: MongoDB for flexible/document data, PostgreSQL for structured/relational data. Use Query Builder for database-agnostic code.

**Q: How do I prevent SQL injection?**  
A: Use `query_with_params()` or `prepare()` with parameter placeholders ($1, $2, etc.). Never concatenate strings!

**Q: How does connection pooling work?**  
A: `connection_pool(uri, size)` creates N connections. `get_connection()` returns one, `release_connection()` returns it. Auto-managed with Arc<Mutex>.

**Q: Can I use transactions with MongoDB?**  
A: Not in current Phase 23.1-23.3 (basic implementation). PostgreSQL transactions are fully supported.

**Q: How do I handle errors?**  
A: All functions return `Result<T, Error>`. Use `?` operator to propagate or `.unwrap()` to panic.

**Q: What's the Query Builder for?**  
A: Database-agnostic query construction. Build once, execute against MongoDB or PostgreSQL using their native adapters.

**Q: How do I see what SQL is generated?**  
A: `QueryBuilderSolver::to_sql(&query_builder)` returns the SQL string for inspection/debugging.

**Q: Can I use this in production?**  
A: Yes, after: (1) testing with real databases, (2) connection pool tuning, (3) error handling review, (4) performance benchmarks.

---

## Integration Examples with Phase 21-22 Stdlib

**Use time_solver to add timestamps:**
```rust
use killer_rcore::stdlib_impl::database_mongodb::*;
use killer_rcore::stdlib_impl::time_solver::*;

let mut doc = HashMap::new();
doc.insert("created_at".to_string(), 
    Value::DateTime(TimeSolver::unix_timestamp_millis()));
MongoSolver::insert_one(&conn, "db", "coll", &doc)?;
```

**Use type_solver for type-safe result mapping:**
```rust
use killer_rcore::stdlib_impl::type_solver::*;

let type_info = TypeSolver::type_info_of::<Row>();
// Can validate row structure before mapping
```

**Use concurrency_solver for query stats:**
```rust
use killer_rcore::stdlib_impl::concurrency_solver::*;

let query_counter = ConcurrencySolver::create_counter();
ConcurrencySolver::increment_counter(&query_counter);
// Track query execution with thread-safe counter
```

---

## Module Statistics

| Module | Functions | Lines | Tests | Type |
|--------|-----------|-------|-------|------|
| database_mongodb | 42 | 520 | 6 | Document DB |
| database_postgresql | 45 | 516 | 5 | Relational DB |
| database_query | 40 | 634 | 6 | Query DSL |
| **TOTAL** | **127** | **1,670** | **17** | **Database** |

---

## Next: Phase 23.4 Advanced Features

Coming soon:
- Complex transaction patterns
- Bulk operation optimizations  
- Advanced aggregations
- Query profiling & statistics
- Connection pool monitoring
- Schema evolution utilities

Estimated: Week 3-4 of Phase 23

---

*Quick Reference | Phase 23.1-23.3 Complete | Killer Standard Library*
