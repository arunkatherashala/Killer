# Phase 23: Database Integration & Query Bindings - Master Plan

**Date:** March 18, 2026  
**Status:** Ready to Launch  
**Builds On:** Phase 20 (FFI) + Phase 21-22 (Stdlib)  
**Target:** Database connectivity for production Killer applications

---

## 🎯 Phase 23 Mission

Enable Killer applications to seamlessly integrate with **MongoDB** and **PostgreSQL** databases through:
- Type-safe query builders
- Connection pooling and management
- Transaction support
- ORM-like abstractions
- Full integration with Phase 21-22 stdlib
- ARU principle (Always Ready to Use + Keep Exploring Organised)

---

## 📊 Architecture Overview

```
Phase 23: Database Layer
├── MongoDB Integration (database_mongodb.rs)
│   ├── Connection pooling
│   ├── BSON document handling
│   ├── Query builder (find, insert, update, delete)
│   ├── Aggregation pipeline
│   └── Transaction support
│
├── PostgreSQL Integration (database_postgresql.rs)
│   ├── Connection pool management
│   ├── Prepared statement caching
│   ├── Query builder (SELECT, INSERT, UPDATE, DELETE)
│   ├── Transaction handling
│   └── Schema inspection
│
└── Query & ORM Layer (database_query.rs)
    ├── Generic query builder trait
    ├── Filter DSL
    ├── Join operations
    ├── Pagination helpers
    ├── Result mapping
    └── Connection pool abstraction

Integrates with Phase 21-22:
└── Uses: time_solver (timestamps)
    Uses: io_solver (serialization)
    Uses: type_solver (type safety)
    Uses: concurrency_solver (thread-safe pools)
```

---

## 🗄️ Module Specifications

### 1. database_mongodb.rs (~450 lines, 40+ functions)

**MongoDB Operations:**
```rust
// Connection management
pub fn mongodb_connect(uri: &str) -> Result<MongoConnection>
pub fn connection_pool(uri: &str, pool_size: usize) -> Result<ConnectionPool>

// CRUD Operations
pub fn find_one(conn: &MongoConnection, db: &str, coll: &str, filter: &Document) 
  -> Result<Option<Document>>
pub fn find_many(conn: &MongoConnection, db: &str, coll: &str, filter: &Document) 
  -> Result<Vec<Document>>
pub fn insert_one(conn: &MongoConnection, db: &str, coll: &str, doc: &Document) 
  -> Result<InsertResult>
pub fn insert_many(conn: &MongoConnection, db: &str, coll: &str, docs: &[Document]) 
  -> Result<InsertResult>
pub fn update_one(conn: &MongoConnection, db: &str, coll: &str, filter: &Document, 
  update: &Document) -> Result<UpdateResult>
pub fn update_many(conn: &MongoConnection, db: &str, coll: &str, filter: &Document, 
  update: &Document) -> Result<UpdateResult>
pub fn delete_one(conn: &MongoConnection, db: &str, coll: &str, filter: &Document) 
  -> Result<DeleteResult>
pub fn delete_many(conn: &MongoConnection, db: &str, coll: &str, filter: &Document) 
  -> Result<DeleteResult>

// Aggregation
pub fn aggregate(conn: &MongoConnection, db: &str, coll: &str, pipeline: &[Document]) 
  -> Result<Vec<Document>>

// Transactions
pub fn start_transaction(conn: &MongoConnection) -> Result<Transaction>
pub fn commit_transaction(tx: &mut Transaction) -> Result<()>
pub fn rollback_transaction(tx: &mut Transaction) -> Result<()>

// Indexing
pub fn create_index(conn: &MongoConnection, db: &str, coll: &str, keys: &Document) 
  -> Result<String>
pub fn list_indexes(conn: &MongoConnection, db: &str, coll: &str) -> Result<Vec<Document>>

// Utility
pub fn database_list(conn: &MongoConnection) -> Result<Vec<String>>
pub fn collection_list(conn: &MongoConnection, db: &str) -> Result<Vec<String>>
pub fn collection_stats(conn: &MongoConnection, db: &str, coll: &str) -> Result<Stats>
```

**Key Features:**
- Connection pooling with configurable size
- BSON document API (HashMap-like interface)
- Bulk operations support
- TTL index support
- Full aggregation pipeline
- Session management for transactions
- Change streams for real-time data

### 2. database_postgresql.rs (~450 lines, 40+ functions)

**PostgreSQL Operations:**
```rust
// Connection management
pub fn postgres_connect(connection_string: &str) -> Result<PgConnection>
pub fn connection_pool(connection_string: &str, pool_size: usize) -> Result<ConnectionPool>

// Query execution
pub fn execute_query(conn: &PgConnection, sql: &str, params: &[&dyn ToSql]) 
  -> Result<u64>
pub fn query_one(conn: &PgConnection, sql: &str, params: &[&dyn ToSql]) 
  -> Result<Option<Row>>
pub fn query_all(conn: &PgConnection, sql: &str, params: &[&dyn ToSql]) 
  -> Result<Vec<Row>>

// Prepared statements
pub fn prepare_statement(conn: &PgConnection, sql: &str) -> Result<Statement>
pub fn execute_prepared(stmt: &Statement, params: &[&dyn ToSql]) -> Result<u64>
pub fn query_prepared(stmt: &Statement, params: &[&dyn ToSql]) -> Result<Vec<Row>>

// Transaction management
pub fn begin_transaction(conn: &mut PgConnection) -> Result<()>
pub fn commit_transaction(conn: &mut PgConnection) -> Result<()>
pub fn rollback_transaction(conn: &mut PgConnection) -> Result<()>

// Schema inspection
pub fn table_exists(conn: &PgConnection, schema: &str, table: &str) -> Result<bool>
pub fn list_tables(conn: &PgConnection, schema: &str) -> Result<Vec<String>>
pub fn table_columns(conn: &PgConnection, schema: &str, table: &str) 
  -> Result<Vec<ColumnInfo>>
pub fn table_primary_key(conn: &PgConnection, schema: &str, table: &str) 
  -> Result<Vec<String>>

// Utility operations
pub fn vacuum_table(conn: &PgConnection, table: &str) -> Result<()>
pub fn analyze_table(conn: &PgConnection, table: &str) -> Result<()>
pub fn table_size(conn: &PgConnection, schema: &str, table: &str) -> Result<u64>
pub fn database_size(conn: &PgConnection, db: &str) -> Result<u64>

// JSON support
pub fn json_query(conn: &PgConnection, sql: &str, params: &[&dyn ToSql]) 
  -> Result<Vec<serde_json::Value>>

// Bulk operations
pub fn bulk_insert(conn: &PgConnection, table: &str, rows: &[Row]) -> Result<u64>
pub fn bulk_update(conn: &PgConnection, table: &str, updates: &[UpdateRow]) -> Result<u64>
pub fn bulk_delete(conn: &PgConnection, table: &str, filters: &[DeleteFilter]) -> Result<u64>
```

**Key Features:**
- Connection pooling with async support
- Prepared statement caching
- Full ACID transaction support
- JSON/JSONB data type support
- Schema introspection
- Bulk operations for batch processing
- Query cost estimation

### 3. database_query.rs (~350 lines, 35+ functions)

**Generic Query Builder:**
```rust
// Generic builder interface
trait QueryBuilder<T: QueryProvider> {
    fn select(&mut self, fields: &[&str]) -> &mut Self
    fn filter(&mut self, field: &str, op: FilterOp, value: &Value) -> &mut Self
    fn order_by(&mut self, field: &str, ascending: bool) -> &mut Self
    fn limit(&mut self, count: u64) -> &mut Self
    fn offset(&mut self, count: u64) -> &mut Self
    fn join(&mut self, other_table: &str, on: (&str, &str)) -> &mut Self
    fn group_by(&mut self, fields: &[&str]) -> &mut Self
    fn having(&mut self, condition: &str) -> &mut Self
    fn execute(&self, provider: &T) -> Result<Vec<Row>>
}

// Filter operations
pub enum FilterOp {
    Equal, NotEqual, GreaterThan, GreaterOrEqual,
    LessThan, LessOrEqual, In, NotIn,
    Like, NotLike, Exists, Between,
    And, Or, Not
}

// Query convenience functions
pub fn select(table: &str) -> SelectBuilder
pub fn insert(table: &str, data: &Document) -> InsertBuilder
pub fn update(table: &str, filter: &Document) -> UpdateBuilder
pub fn delete(table: &str) -> DeleteBuilder

// Helper functions
pub fn filter_builder() -> FilterBuilder
pub fn join_builder(left_table: &str, right_table: &str) -> JoinBuilder
pub fn paginate(page: u32, per_page: u32) -> PaginationHelper
pub fn sort_field(field: &str, ascending: bool) -> SortField

// Result mapping
pub fn map_to_struct<T: FromRow>(rows: &[Row]) -> Result<Vec<T>>
pub fn first_or_none<T: FromRow>(rows: &[Row]) -> Result<Option<T>>
pub fn single<T: FromRow>(rows: &[Row]) -> Result<T>
```

**Key Features:**
- Database-agnostic query building
- Type-safe filters
- Automatic SQL generation
- Cursor-based pagination
- Result mapping to Rust structs
- Connection pooling wrapper
- Lazy query execution

---

## 🔗 Integration with Phase 21-22 Stdlib

**Harmonious Integration Points:**

```
Phase 23 Database ← → Phase 21-22 Stdlib
│
├─ time_solver
│  ├─ Timestamps for created_at, updated_at fields
│  ├─ Query execution timing
│  └─ Connection pool timeout management
│
├─ io_solver
│  ├─ Loading connection configs from files
│  ├─ Exporting query results to CSV
│  └─ Binary serialization of blobs
│
├─ type_solver
│  ├─ Type mapping between Killer ↔ DB types
│  ├─ Generic struct mapping from rows
│  └─ Type checking for query parameters
│
├─ concurrency_solver
│  ├─ Thread-safe connection pools
│  ├─ Atomic counters for query statistics
│  └─ Lock-free read replicas
│
└─ statistics_solver
   ├─ Query performance metrics
   ├─ Data distribution analysis
   └─ Database optimization recommendations
```

---

## 📋 Implementation Phases

### Phase 23.1: MongoDB Foundation (Week 1)
- [ ] Connection pooling
- [ ] CRUD operations (find, insert, update, delete)
- [ ] Basic query builder
- [ ] 12+ unit tests
- [ ] Quick reference guide

### Phase 23.2: PostgreSQL Foundation (Week 2)
- [ ] Connection pooling
- [ ] Query execution (SELECT, INSERT, UPDATE, DELETE)
- [ ] Prepared statements
- [ ] 12+ unit tests
- [ ] Quick reference guide

### Phase 23.3: Query Abstraction Layer (Week 3)
- [ ] Generic query builder
- [ ] Filter DSL
- [ ] Result mapping
- [ ] Pagination helpers
- [ ] 10+ utility functions
- [ ] 8+ unit tests

### Phase 23.4: Advanced Features (Week 4)
- [ ] Transactions
- [ ] Aggregations
- [ ] Bulk operations
- [ ] Schema introspection
- [ ] Change streams (MongoDB)
- [ ] 10+ tests

### Phase 23.5: Documentation & ARU (Week 5)
- [ ] Completion report (like 21-22)
- [ ] Quick reference guide
- [ ] Integration examples
- [ ] Deployment checklist
- [ ] Performance benchmarks

---

## 🎯 Success Metrics

| Metric | Target |
|--------|--------|
| Total Functions | 115+ |
| Total Lines | 1,200+ |
| Unit Tests | 40+ |
| Module Count | 3 |
| Code Coverage | 15%+ |
| Documentation Pages | 3 |
| Ready for Production | ✅ Yes |

---

## 🚀 Quick Launch Plan

**Week 1 - Phase 23.1 (MongoDB)**
```
Day 1-2: Connection pooling + FFI bindings
Day 3-4: CRUD operations (find, insert, update, delete)
Day 5: Query builder + documentation
```

**Week 2 - Phase 23.2 (PostgreSQL)**
```
Day 1-2: Connection pooling + libpq bindings
Day 3-4: Query execution + prepared statements
Day 5: PostgreSQL query builder + documentation
```

**Week 3 - Phase 23.3 (Query Abstraction)**
```
Day 1-2: Generic query builder interface
Day 3-4: Filter DSL + result mapping
Day 5: Pagination + documentation
```

**Week 4 - Phase 23.4 (Advanced)**
```
Day 1-2: Transactions (both databases)
Day 3: Aggregations + bulk operations
Day 4: Schema introspection
Day 5: Complete testing suite
```

**Week 5 - Phase 23.5 (ARU Documentation)**
```
Day 1-2: Completion report + integration guide
Day 3: Quick reference + examples
Day 4: Performance benchmarks
Day 5: Deployment validation
```

---

## 🔧 Technical Stack

### Dependencies (Proposed)
- **MongoDB**: `mongodb` crate with async support
- **PostgreSQL**: `postgres` + `tokio-postgres` (async)
- **Connection Pooling**: `deadpool` or `r2d2`
- **Serialization**: `serde` + `serde_json`
- **Type Mapping**: Use Phase 21-22 `type_solver`

### FFI Connections (Phase 20 Integration)
- MongoDB C driver via FFI
- libpq via FFI
- Optional: Use Rust drivers (preferred for safety)

---

## 📐 Example Usage (Preview)

```rust
use killer_rcore::stdlib_impl::database_mongodb;
use killer_rcore::stdlib_impl::database_query;
use killer_rcore::stdlib_impl::time_solver;

fn main() -> Result<()> {
    // MongoDB example
    let mongo = database_mongodb::connection_pool(
        "mongodb://localhost:27017",
        10
    )?;
    
    let docs = database_mongodb::find_many(
        &mongo,
        "mydb",
        "mycoll",
        &document!{"age": {"$gt": 30}}
    )?;
    
    // PostgreSQL example
    let pg = database_postgresql::connection_pool(
        "postgresql://user:pass@localhost/mydb",
        10
    )?;
    
    let results = database_query::select("users")
        .filter("age", FilterOp::GreaterThan, 30)
        .order_by("created_at", false)
        .limit(100)
        .execute(&pg)?;
    
    // Integration with time_solver
    let now = time_solver::unix_timestamp_millis();
    database_query::insert("audit_log", &document!{
        "timestamp": now,
        "action": "query_executed"
    }).execute(&pg)?;
    
    Ok(())
}
```

---

## 🎓 ARU Principle Application

### Always Ready to Use ✅
- Simple API: `database_mongodb::find_one()` style
- Connection pooling (auto-managed)
- Error handling with Results
- No external configuration needed
- Works immediately after `use` statement

### Keep Exploring Organised ✅
- 3 focused modules (MongoDB, PostgreSQL, Query)
- Each module ~350-450 lines (manageable)
- 40+ functions per module (discoverable)
- Quick reference guide (part of Phase 23.5)
- Example patterns documented
- Integration points clearly marked

---

## 📚 Documentation Deliverables (Phase 23.5)

1. **PHASE_23_DATABASE_COMPLETION_REPORT.md**
   - Architecture overview
   - All 3 modules with function signatures
   - Integration examples
   - Performance characteristics

2. **PHASE_23_DATABASE_QUICK_REFERENCE.md**
   - 30-second quick start
   - Module directory
   - Common patterns (5-10 examples)
   - Q&A guide

3. **PHASE_23_DATABASE_INTEGRATION_GUIDE.md**
   - Integration checklist
   - Deployment procedures
   - Troubleshooting guide
   - Performance tuning

---

## 🔄 Phase Roadmap

```
Phase 20: FFI System ✅
  ↓
Phase 21-22: Stdlib (454 functions) ✅
  ↓
Phase 23: Database Integration (115+ functions) ⏳ READY TO LAUNCH
  ↓
Phase 24: Web Framework (HTTP/2, WebSocket)
  ↓
Phase 25: Distributed Systems (RPC, Consensus)
  ↓
Phase 26: ML Operations (Inference, Training)
```

---

## ✅ Prerequisites

- ✅ Phase 20 FFI complete (C interop ready)
- ✅ Phase 21-22 Stdlib complete (utility functions available)
- ✅ Workspace structure ready (`_TOOLS/killer_rcore`)
- ✅ Cargo.toml configured for extensions
- ✅ Unit test framework established

**Status: ✅ ALL PREREQUISITES MET - READY TO LAUNCH PHASE 23**

---

**Next Session:** Begin Phase 23.1 (MongoDB Foundation) — Create connection pooling, CRUD operations, initial testing suite.

**Estimated Delivery:** 5 weeks for complete Phase 23 with full ARU compliance and production-ready code.

---

*Prepared: March 18, 2026*  
*Builds on: Phase 20 (FFI) + Phase 21-22 (Stdlib)*  
*Next: Phase 23 Database Integration*  
*ARU Ready: Always Ready to Use + Keep Exploring Organised*
