# Week 8 - Complete Implementation Plan
**Target**: 7,100+ lines of production code across 5 major systems  
**Status**: Planning Phase - Ready for Execution  
**Date**: March 13, 2026

---

## Executive Summary

Week 8 focuses on enterprise hardening and scalability:
1. **Query Optimizer** (500 lines) - Cost-based optimization for SQL queries
2. **Parallel I/O** (800 lines) - Multi-threaded read/write with thread pools
3. **Memory Management** (600 lines) - Spill-to-disk when RAM constrained
4. **IDE/LSP Server** (2,500 lines) - Full language server protocol support
5. **Python Foundation** (2,500 lines) - Generators, comprehensions, decorators

### Completion Timeline
- **Query Optimizer**: Days 1-2 (foundation for other work)
- **Parallel I/O**: Days 2-3 (uses optimizer hooks)
- **Memory Management**: Days 3-4 (works with both)
- **Python Foundation**: Days 1-3 (parallel with infrastructure)
- **IDE/LSP Server**: Days 4-7 (final integration)

### Total Effort
- **Lines of Code**: 7,100
- **Modules**: 5 new + modifications to existing
- **Tests**: 40+ comprehensive test cases
- **Build Time**: ~60s (release profile)
- **Expected Binary Size**: 1.3-1.5 MB

---

## 1. Query Optimizer (500 lines)

### Purpose
Replace naive sequential query execution with cost-based optimization. Analyze query plans, estimate costs, and choose optimal execution strategies.

### Architecture

```
QueryOptimizer
├── CostEstimator
│   ├── Selectivity estimation
│   ├── Cardinality calculation
│   └── Cost models (I/O, CPU, memory)
├── PlanOptimizer
│   ├── Join reordering
│   ├── Predicate pushdown
│   └── Projection elimination
├── ExecutionPlan
│   ├── Node types (Scan, Filter, Join, Aggregate)
│   ├── Physical vs logical plans
│   └── Plan comparator
└── Executor (enhanced)
    ├── Plan interpretation
    ├── Operation dispatch
    └── Statistics collection
```

### Key Components

**1. CostModel** (120 lines)
```rust
pub struct CostModel {
    io_cost_per_row: f64,        // 0.01
    cpu_cost_per_row: f64,       // 0.1
    memory_cost_factor: f64,     // 1.0
    network_cost_factor: f64,    // 0.5
}

pub struct QueryCost {
    io_cost: f64,
    cpu_cost: f64,
    memory_cost: f64,
    total_cost: f64,
    estimated_rows: usize,
}
```

**2. ExecutionPlan** (150 lines)
```rust
pub enum PlanNode {
    TableScan {
        table: String,
        filters: Vec<Expr>,
    },
    Filter {
        source: Box<PlanNode>,
        predicate: Expr,
    },
    Project {
        source: Box<PlanNode>,
        columns: Vec<String>,
    },
    Join {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        condition: Expr,
    },
    Aggregate {
        source: Box<PlanNode>,
        group_by: Vec<String>,
        aggregations: Vec<(String, AggFunc)>,
    },
}

pub struct QueryPlan {
    root: PlanNode,
    estimated_cost: QueryCost,
    statistics: TableStats,
}
```

**3. Optimizer Rules** (230 lines)
- **Rule 1**: Predicate pushdown (filter before join)
- **Rule 2**: Projection elimination (select only needed columns)
- **Rule 3**: Join reordering (minimize intermediate rows)
- **Rule 4**: Aggregate pushdown (reduce cardinality early)
- **Rule 5**: Column pruning (remove unused computations)

### Implementation Steps

**Step 1**: Add `optimizer.rs` module to `src/spark/`
```rust
// src/spark/optimizer.rs
pub mod cost_model;
pub mod plan;
pub mod rules;
pub mod executor;

pub use cost_model::CostModel;
pub use plan::{PlanNode, QueryPlan};
```

**Step 2**: Integrate with SQL executor
```rust
// src/spark/sql.rs (modified)
pub fn execute_optimized(&self, query: &str) -> Result<DataFrame> {
    let parsed = self.parse(query)?;
    let plan = self.optimize(&parsed)?;  // NEW
    let result = self.execute_plan(&plan)?;  // NEW
    Ok(result)
}
```

**Step 3**: Add 8 test cases
- Test cost estimation accuracy
- Test plan nodes creation
- Test predicate pushdown optimization
- Test join reordering
- Test aggregate pushdown
- Test complex multi-join optimization
- Test cost comparison between plans
- Test statistics collection

### Performance Impact
- **Join queries**: 40-60% faster (better join ordering)
- **Large table scans**: 30-50% faster (predicate pushdown)
- **Complex aggregates**: 20-35% faster (early reduction)
- **Overhead**: ~5% for simple queries (plan optimization cost)

---

## 2. Parallel I/O System (800 lines)

### Purpose
Replace single-threaded I/O with thread pool-based parallel reading and writing. Enable concurrent access to multiple partitions.

### Architecture

```
ParallelIOSystem
├── ThreadPool
│   ├── Worker threads (default 8)
│   ├── Work queue (unbounded)
│   └── Synchronization primitives
├── DataSource (enhanced)
│   ├── Parallel reader
│   ├── Partition aware
│   └── Statistics collection
├── DataSink (enhanced)
│   ├── Parallel writer
│   ├── Buffering strategy
│   └── Error handling
└── IOMetrics
    ├── Throughput tracking
    ├── Latency percentiles
    └── Thread utilization
```

### Key Components

**1. ThreadPool** (150 lines)
```rust
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
    active_tasks: Arc<AtomicUsize>,
}

pub enum Message {
    NewJob(Job),
    Terminate,
}

pub type Job = Box<dyn FnOnce() + Send + 'static>;
```

**2. ParallelDataSource** (200 lines)
```rust
pub struct ParallelDataSource {
    path: String,
    format: FileFormat,
    partitions: usize,
    chunk_size: usize,
    thread_pool: ThreadPool,
}

impl ParallelDataSource {
    pub fn read_parallel(&self) -> Result<Vec<Partition>> {
        // Distribute partitions across thread pool
        // Collect results with synchronization
        // Return combined data
    }
}
```

**3. ParallelDataSink** (200 lines)
```rust
pub struct ParallelDataSink {
    path: String,
    format: FileFormat,
    partitions: usize,
    buffer_size: usize,
    thread_pool: ThreadPool,
}

impl ParallelDataSink {
    pub fn write_parallel(&self, data: &[Partition]) -> Result<()> {
        // Distribute write tasks across partitions
        // Buffer data in memory first
        // Flush in parallel
        // Synchronize completion
    }
}
```

**4. IOMetrics** (100 lines)
```rust
pub struct IOMetrics {
    bytes_read: Arc<AtomicUsize>,
    bytes_written: Arc<AtomicUsize>,
    operations: Arc<AtomicUsize>,
    latencies: Arc<Mutex<Vec<u64>>>,
}

pub struct IOStats {
    throughput_mbps: f64,
    avg_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
}
```

**5. PartitionManager** (150 lines)
```rust
pub struct PartitionManager {
    partitions: Vec<Partition>,
    partition_count: usize,
}

impl PartitionManager {
    pub fn distribute_work(&self, work: &[u8], count: usize) -> Vec<PartitionTask> {
        // Distribute work evenly across partitions
        // Consider data locality
        // Return task assignments
    }
}
```

### Implementation Steps

**Step 1**: Create `src/spark/io_parallel.rs`
```rust
// src/spark/io_parallel.rs (new)
pub mod thread_pool;
pub mod data_source;
pub mod data_sink;
pub mod metrics;
pub mod partition;

pub use thread_pool::ThreadPool;
pub use data_source::ParallelDataSource;
pub use data_sink::ParallelDataSink;
pub use metrics::IOMetrics;
```

**Step 2**: Update `io.rs` to use parallel versions
```rust
// src/spark/io.rs (enhanced)
pub async fn read_csv_parallel(path: &str, partitions: usize) -> Result<DataFrame> {
    let source = ParallelDataSource::new(path, FileFormat::CSV, partitions);
    let partitions = source.read_parallel().await?;
    Ok(DataFrame::from_partitions(partitions))
}
```

**Step 3**: Add 10 test cases
- Test thread pool creation and termination
- Test thread pool work distribution
- Test parallel CSV reading
- Test parallel JSON writing
- Test partition assignment
- Test thread synchronization
- Test error handling in workers
- Test graceful shutdown
- Test metrics collection accuracy
- Test high concurrency (100+ tasks)

### Performance Impact
- **4-core CPU**: 2.5-3.2x faster I/O
- **8-core CPU**: 4.5-6.0x faster I/O
- **16-core CPU**: 8.0-11.0x faster I/O
- **I/O bound**: Nearly linear scaling up to core count
- **Throughput**: 500+ MB/sec on local SSDs

---

## 3. Memory Management System (600 lines)

### Purpose
Handle datasets larger than available RAM by spilling to disk with smart caching. Implement LRU eviction and memory pooling.

### Architecture

```
MemoryManager
├── MemoryPool
│   ├── Allocation tracking
│   ├── Free space management
│   └── Defragmentation
├── Spillable Cache
│   ├── LRU eviction policy
│   ├── Disk spilling
│   └── Automatic refresh
├── Stats Tracker
│   ├── Memory usage
│   ├── Spill count
│   └── Hit/miss metrics
└── DiskBuffer
    ├── Temporary storage
    ├── Compression
    └── Cleanup
```

### Key Components

**1. MemoryPool** (120 lines)
```rust
pub struct MemoryPool {
    max_size: usize,
    current_usage: Arc<AtomicUsize>,
    allocations: Arc<Mutex<Vec<MemoryBlock>>>,
}

pub struct MemoryBlock {
    id: usize,
    size: usize,
    owner: String,
    allocated_at: Instant,
}

impl MemoryPool {
    pub fn allocate(&self, size: usize, owner: &str) -> Result<MemoryRegion> {
        // Check available memory
        // Allocate or spill if needed
        // Track allocation
    }
}
```

**2. SpillableCache** (200 lines)
```rust
pub struct SpillableCache {
    memory_cache: LruCache<String, Vec<u8>>,
    disk_path: PathBuf,
    max_memory: usize,
    spill_threshold: f64,
}

impl SpillableCache {
    pub fn put(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        if self.memory_usage() + value.len() > self.max_memory {
            let evicted = self.evict_lru()?;
            self.spill_to_disk(&evicted)?;
        }
        self.memory_cache.put(key, value);
        Ok(())
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(value) = self.memory_cache.get(key) {
            return Ok(Some(value.clone()));
        }
        // Load from disk
        self.load_from_disk(key)
    }
}
```

**3. LRU Cache** (100 lines)
```rust
pub struct LruCache<K: Clone + Eq + Hash, V> {
    cache: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K: Clone + Eq + Hash, V> LruCache<K, V> {
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.cache.contains_key(key) {
            // Move to end (most recently used)
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            return self.cache.get(key);
        }
        None
    }

    pub fn put(&mut self, key: K, value: V) {
        if self.cache.len() >= self.capacity {
            let evicted = self.order.pop_front().unwrap();
            self.cache.remove(&evicted);
        }
        self.cache.insert(key.clone(), value);
        self.order.push_back(key);
    }
}
```

**4. DiskBuffer** (100 lines)
```rust
pub struct DiskBuffer {
    spill_dir: PathBuf,
    max_size: usize,
    current_size: Arc<AtomicUsize>,
    compression: bool,
}

impl DiskBuffer {
    pub fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let path = self.spill_dir.join(key);
        let compressed = if self.compression {
            compress_data(data)?
        } else {
            data.to_vec()
        };
        fs::write(&path, compressed)?;
        self.current_size.fetch_add(compressed.len(), Ordering::Relaxed);
        Ok(())
    }

    pub fn read(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.spill_dir.join(key);
        let data = fs::read(&path)?;
        if self.compression {
            decompress_data(&data)
        } else {
            Ok(data)
        }
    }
}
```

**5. MemoryStats** (80 lines)
```rust
pub struct MemoryStats {
    total_allocated: usize,
    total_freed: usize,
    spill_count: usize,
    spill_bytes: usize,
    cache_hits: usize,
    cache_misses: usize,
    peak_memory: usize,
}

impl MemoryStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }

    pub fn spill_ratio(&self) -> f64 {
        if self.total_allocated == 0 { 0.0 } else {
            self.spill_bytes as f64 / self.total_allocated as f64
        }
    }
}
```

### Implementation Steps

**Step 1**: Create `src/spark/memory.rs`
```rust
// src/spark/memory.rs (new)
pub mod pool;
pub mod cache;
pub mod lru;
pub mod disk;
pub mod stats;

pub use pool::MemoryPool;
pub use cache::SpillableCache;
pub use disk::DiskBuffer;
pub use stats::MemoryStats;
```

**Step 2**: Integrate with DataFrame
```rust
// src/spark/dataframe.rs (enhanced)
pub struct DataFrame {
    // ... existing fields
    memory_manager: Arc<MemoryManager>,
    cache: SpillableCache,
}
```

**Step 3**: Add 8 test cases
- Test memory pool allocation
- Test LRU eviction policy
- Test disk spilling
- Test cache hit/miss tracking
- Test automatic spilling when memory full
- Test refresh from disk
- Test compression on spill
- Test memory stats accuracy

### Performance Impact
- **Datasets < 1GB**: No spill, full memory speed
- **Datasets 1-10GB**: 50-70% of memory speed (occasional spill)
- **Datasets > 10GB**: 20-40% of memory speed (frequent spill)
- **Memory overhead**: 50MB per 1GB of spilled data
- **Disk overhead**: 5-20% depending on compression

---

## 4. IDE/LSP Server (2,500 lines)

### Purpose
Full Language Server Protocol implementation for editor integration. Enable syntax highlighting, autocomplete, go-to-definition, type checking, and debugging.

### Architecture

```
LanguageServer
├── LSP Protocol Handler (500 lines)
│   ├── Message parsing
│   ├── JSON-RPC 2.0
│   └── Request/response routing
├── Symbols Registry (400 lines)
│   ├── Function/class/variable definitions
│   ├── Type information
│   └── Scope management
├── Semantic Analyzer (500 lines)
│   ├── Type inference
│   ├── Error detection
│   └── Warning generation
├── Completion Provider (400 lines)
│   ├── Function/variable suggestions
│   ├── Keyword completion
│   └── Context-aware ranking
├── Hover Provider (200 lines)
│   ├── Type information on hover
│   ├── Documentation
│   └── Return types
├── Definition/Reference Finder (300 lines)
│   ├── Go to definition
│   ├── Find all references
│   └── Rename support
└── Debugger (200 lines)
    ├── Breakpoint management
    ├── Stack frame inspection
    └── Variable inspection
```

### Key Components

**1. LSP Server Core** (500 lines)
```rust
// src/server/lsp.rs (new)
pub struct LanguageServer {
    stdio: StdioTransport,
    symbols: Arc<RwLock<SymbolTable>>,
    document_store: Arc<RwLock<DocumentStore>>,
}

impl LanguageServer {
    pub fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult> {
        // Register capabilities
        // Set up workspace
    }

    pub fn did_open(&self, params: DidOpenTextDocumentParams) -> Result<()> {
        // Parse document
        // Extract symbols
        // Publish diagnostics
    }

    pub fn did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        // Update document
        // Re-analyze
        // Update diagnostics
    }

    pub fn completion(&self, params: CompletionParams) -> Result<Vec<CompletionItem>> {
        // Get context
        // Generate suggestions
        // Rank by relevance
    }

    pub fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // Find symbol at position
        // Get type and docs
        // Format response
    }

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Vec<Location>> {
        // Find symbol
        // Return definition location
    }

    pub fn find_references(&self, params: ReferenceParams) -> Result<Vec<Location>> {
        // Find symbol
        // Search all documents
        // Return all references
    }
}
```

**2. Symbol Table** (400 lines)
```rust
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
    scopes: Vec<Scope>,
    current_scope: usize,
}

pub struct Symbol {
    name: String,
    kind: SymbolKind,
    location: Location,
    type_info: Option<Type>,
    documentation: Option<String>,
}

pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Parameter,
    Type,
    Constant,
    Enum,
    Interface,
}

impl SymbolTable {
    pub fn define(&mut self, symbol: Symbol) -> Result<()> {
        // Check for duplicates in current scope
        // Add to symbols
        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // Search current and parent scopes
        self.symbols.get(name)
    }

    pub fn lookup_references(&self, name: &str) -> Vec<Location> {
        // Find all uses of symbol
        vec![]
    }
}
```

**3. Semantic Analyzer** (500 lines)
```rust
pub struct SemanticAnalyzer {
    symbol_table: Arc<RwLock<SymbolTable>>,
    type_checker: TypeChecker,
}

impl SemanticAnalyzer {
    pub fn analyze(&self, ast: &Ast) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Type checking
        diagnostics.extend(self.type_checker.check(ast));

        // Undefined variables
        diagnostics.extend(self.check_undefined(ast));

        // Type mismatches
        diagnostics.extend(self.check_type_mismatches(ast));

        // Unused variables
        diagnostics.extend(self.check_unused(ast));

        // Unreachable code
        diagnostics.extend(self.check_unreachable(ast));

        diagnostics
    }
}
```

**4. Completion Provider** (400 lines)
```rust
pub struct CompletionProvider {
    symbol_table: Arc<RwLock<SymbolTable>>,
}

impl CompletionProvider {
    pub fn get_completions(&self, position: &Position, context: &CompletionContext) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add keywords
        items.extend(self.keyword_completions());

        // Add symbols from scope
        items.extend(self.symbol_completions(position));

        // Add builtins
        items.extend(self.builtin_completions());

        // Filter and rank by prefix
        self.filter_and_rank(items, &context.prefix)
    }
}
```

**5. Hover Provider** (200 lines)
```rust
pub struct HoverProvider {
    symbol_table: Arc<RwLock<SymbolTable>>,
}

impl HoverProvider {
    pub fn get_hover(&self, position: &Position) -> Option<Hover> {
        // Find symbol at position
        let symbol = self.find_symbol_at(position)?;

        // Format hover content
        let contents = HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!(
                "**{}**: {}\n\n{}",
                symbol.name,
                symbol.type_info.as_ref().map(|t| t.to_string()).unwrap_or_default(),
                symbol.documentation.as_ref().unwrap_or(&String::new())
            ),
        });

        Some(Hover {
            contents,
            range: None,
        })
    }
}
```

**6. Definition Finder** (300 lines)
```rust
pub struct DefinitionFinder {
    symbol_table: Arc<RwLock<SymbolTable>>,
    documents: Arc<RwLock<DocumentStore>>,
}

impl DefinitionFinder {
    pub fn goto_definition(&self, position: &Position) -> Option<Location> {
        // Find symbol at position
        // Return its definition location
    }

    pub fn find_all_references(&self, position: &Position) -> Vec<Location> {
        // Find symbol at position
        // Find all references in all documents
        // Return all locations
    }

    pub fn prepare_rename(&self, position: &Position) -> Option<Range> {
        // Find symbol at position
        // Return its range
    }

    pub fn rename(&self, position: &Position, new_name: &str) -> Vec<TextEdit> {
        // Find symbol at position
        // Find all references
        // Generate TextEdits for all locations
    }
}
```

**7. Debugger** (200 lines)
```rust
pub struct Debugger {
    breakpoints: HashMap<(String, usize), Breakpoint>,
    call_stack: Vec<StackFrame>,
    variables: HashMap<String, Variable>,
}

pub struct Breakpoint {
    file: String,
    line: usize,
    condition: Option<String>,
    hit_count: usize,
}

pub struct StackFrame {
    function: String,
    file: String,
    line: usize,
    variables: HashMap<String, Variable>,
}

impl Debugger {
    pub fn set_breakpoint(&mut self, file: String, line: usize) {
        self.breakpoints.insert((file, line), Breakpoint {
            file,
            line,
            condition: None,
            hit_count: 0,
        });
    }

    pub fn hit_breakpoint(&mut self, file: &str, line: usize) -> bool {
        self.breakpoints.contains_key(&(file.to_string(), line))
    }
}
```

### Implementation Steps

**Step 1**: Create `src/server/` directory structure
```
src/server/
├── mod.rs
├── lsp.rs (main server)
├── symbols.rs (symbol table)
├── semantic.rs (type checker)
├── completion.rs
├── hover.rs
├── definition.rs
├── debugger.rs
└── transport.rs (stdio, socket)
```

**Step 2**: Update `Cargo.toml` with LSP dependencies
```toml
[dependencies]
lsp-types = "0.95"
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

**Step 3**: Add 15 test cases
- Test LSP initialization
- Test document open/change
- Test symbol extraction
- Test completion suggestions
- Test hover information
- Test go-to-definition
- Test find-all-references
- Test rename refactoring
- Test diagnostic publishing
- Test error detection
- Test type inference
- Test scope management
- Test breakpoint setting
- Test call stack inspection
- Test variable inspection

### Performance Impact
- **Completion response**: < 100ms
- **Hover response**: < 50ms
- **Reference finding**: 200-500ms (all documents)
- **Diagnostic analysis**: 100-300ms per document
- **Memory**: 100-200MB per workspace

---

## 5. Python Foundation Layer (2,500 lines)

### Purpose
Add Python-style features to Killer: generators, list/dict comprehensions, decorators, context managers, async/await prep.

### Architecture

```
PythonLayer
├── Generator System (400 lines)
│   ├── Generator trait
│   ├── yield keyword
│   ├── Lazy evaluation
│   └── Generator nesting
├── Comprehensions (500 lines)
│   ├── List comprehensions
│   ├── Dict comprehensions
│   ├── Set comprehensions
│   └── Generator expressions
├── Decorators (400 lines)
│   ├── Decorator parsing
│   ├── Function wrapping
│   ├── Class decorators
│   └── Decorator composition
├── Context Managers (400 lines)
│   ├── __enter__/__exit__
│   ├── with statement
│   ├── Exception handling
│   └── Resource cleanup
├── Type Hints (400 lines)
│   ├── Type annotation parsing
│   ├── Type checking
│   └── Runtime type info
└── Async/Await Prep (300 lines)
    ├── async/await keywords
    ├── Promise/Future tracking
    └── Callback infrastructure
```

### Key Components

**1. Generator System** (400 lines)
```rust
// src/python/generators.rs (new)
pub trait Generator {
    fn next(&mut self) -> Option<Value>;
    fn is_exhausted(&self) -> bool;
}

pub struct GeneratorState {
    locals: HashMap<String, Value>,
    pc: usize,  // Program counter
    exhausted: bool,
}

pub struct KillerGenerator {
    state: GeneratorState,
    function: String,
    closure: HashMap<String, Value>,
}

impl Generator for KillerGenerator {
    fn next(&mut self) -> Option<Value> {
        // Resume execution from last yield
        // Execute until next yield
        // Save state and return
    }
}

pub fn handle_yield(value: Value, state: &mut GeneratorState) -> Value {
    // Suspend generator
    // Save program counter
    // Return value
    value
}
```

**2. Comprehensions** (500 lines)
```rust
// src/python/comprehensions.rs (new)
pub fn list_comprehension(
    expr: &Expr,
    for_clause: &ForClause,
    conditions: &[Expr],
) -> Result<Value> {
    let mut result = Vec::new();

    for item in evaluate_iter(for_clause.iter)? {
        // Bind variable
        env.set(&for_clause.var, item);

        // Check conditions
        if conditions.iter().all(|c| eval_bool(c, env)?) {
            // Evaluate expression
            result.push(eval(expr, env)?);
        }
    }

    Ok(Value::List(result))
}

pub fn dict_comprehension(
    key_expr: &Expr,
    value_expr: &Expr,
    for_clause: &ForClause,
    conditions: &[Expr],
) -> Result<Value> {
    let mut result = HashMap::new();

    for item in evaluate_iter(for_clause.iter)? {
        env.set(&for_clause.var, item);

        if conditions.iter().all(|c| eval_bool(c, env)?) {
            let key = eval(key_expr, env)?;
            let value = eval(value_expr, env)?;
            result.insert(key.to_string(), value);
        }
    }

    Ok(Value::Dict(result))
}

pub fn set_comprehension(
    expr: &Expr,
    for_clause: &ForClause,
    conditions: &[Expr],
) -> Result<Value> {
    let mut result = HashSet::new();

    for item in evaluate_iter(for_clause.iter)? {
        env.set(&for_clause.var, item);

        if conditions.iter().all(|c| eval_bool(c, env)?) {
            result.insert(eval(expr, env)?.to_string());
        }
    }

    Ok(Value::Set(result))
}

pub fn generator_expression(
    expr: &Expr,
    for_clause: &ForClause,
    conditions: &[Expr],
) -> Result<Value> {
    // Return generator instead of materializing list
    Ok(Value::Generator(Box::new(
        GeneratorExpr::new(expr, for_clause, conditions)
    )))
}
```

**3. Decorators** (400 lines)
```rust
// src/python/decorators.rs (new)
pub struct Decorator {
    name: String,
    args: Vec<Value>,
}

pub fn apply_decorator(func: Function, decorator: &Decorator) -> Result<Function> {
    // Call decorator with function
    // Return wrapped function
    match decorator.name.as_str() {
        "staticmethod" => Ok(Function {
            is_static: true,
            ..func
        }),
        "classmethod" => Ok(Function {
            is_class_method: true,
            ..func
        }),
        "property" => Ok(Function {
            is_property: true,
            ..func
        }),
        custom_decorator => {
            // Call decorator function with func as argument
            // Return the wrapped result
            call_function(custom_decorator, vec![Value::Function(func)])
        }
    }
}

pub fn apply_decorators(mut func: Function, decorators: &[Decorator]) -> Result<Function> {
    for decorator in decorators {
        func = apply_decorator(func, decorator)?;
    }
    Ok(func)
}

pub fn class_decorator(class: Class, decorator: &Decorator) -> Result<Class> {
    // Similar to function decorators
    // Call decorator with class as argument
    // Return decorated class
    Ok(class)
}
```

**4. Context Managers** (400 lines)
```rust
// src/python/context.rs (new)
pub trait ContextManager {
    fn enter(&mut self) -> Result<Value>;
    fn exit(&mut self) -> Result<()>;
}

pub struct WithStatement {
    context_expr: Expr,
    var_name: Option<String>,
    body: Vec<Statement>,
}

pub fn execute_with(stmt: &WithStatement, env: &mut Env) -> Result<Value> {
    // Evaluate context expression
    let mut ctx = eval_context(&stmt.context_expr, env)?;

    // Call __enter__
    let enter_result = ctx.enter()?;

    // Bind to variable if specified
    if let Some(var) = &stmt.var_name {
        env.set(var, enter_result);
    }

    // Execute body, catch exceptions
    let result = execute_statements(&stmt.body, env);

    // Call __exit__ regardless
    let exit_result = ctx.exit();

    // Return body result or propagate exit error
    result.or(exit_result.map(|_| Value::Null))
}

pub struct FileContextManager {
    path: String,
    file: Option<File>,
}

impl ContextManager for FileContextManager {
    fn enter(&mut self) -> Result<Value> {
        self.file = Some(File::open(&self.path)?);
        Ok(Value::File(self.file.take().unwrap()))
    }

    fn exit(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()?;
        }
        Ok(())
    }
}
```

**5. Type Hints** (400 lines)
```rust
// src/python/types.rs (new)
pub enum TypeHint {
    Simple(String),            // int, str, bool
    Generic(String, Vec<TypeHint>),  // List[int], Dict[str, int]
    Union(Vec<TypeHint>),      // int | str
    Optional(Box<TypeHint>),   // int?
    Callable(Vec<TypeHint>, Box<TypeHint>),  // (int, str) -> bool
}

pub struct TypeChecker {
    type_annotations: HashMap<String, TypeHint>,
}

impl TypeChecker {
    pub fn check_function_call(&self, func_name: &str, args: &[Value]) -> Result<()> {
        // Get function signature
        let sig = get_function_signature(func_name)?;

        // Check arg count
        if args.len() != sig.params.len() {
            return Err(format!(
                "Function {} expects {} args, got {}",
                func_name, sig.params.len(), args.len()
            ));
        }

        // Check arg types
        for (i, (param_hint, arg)) in sig.params.iter().zip(args).enumerate() {
            let arg_type = infer_type(arg);
            if !self.is_compatible(&arg_type, param_hint) {
                return Err(format!(
                    "Argument {} has wrong type: expected {}, got {}",
                    i, param_hint, arg_type
                ));
            }
        }

        Ok(())
    }
}
```

**6. Async/Await Preparation** (300 lines)
```rust
// src/python/async.rs (new)
pub enum Future<T> {
    Pending,
    Ready(T),
    Error(String),
}

pub async fn async_function(name: &str, args: &[Value]) -> Result<Value> {
    // Mark function as async
    // Return Future<Value>
    // Support await syntax
    Ok(Value::Null)
}

pub fn await_future(future: &Future<Value>) -> Result<Value> {
    match future {
        Future::Pending => Err("Future not ready".to_string()),
        Future::Ready(v) => Ok(v.clone()),
        Future::Error(e) => Err(e.clone()),
    }
}

pub struct EventLoop {
    pending_futures: Vec<Box<dyn Future>>,
    current_task: Option<String>,
}

impl EventLoop {
    pub fn run_until_complete(&mut self, future: Box<dyn Future>) -> Result<Value> {
        // Execute all pending futures
        // Keep going until all complete
        Ok(Value::Null)
    }
}
```

### Implementation Steps

**Step 1**: Create `src/python/` directory structure
```
src/python/
├── mod.rs
├── generators.rs
├── comprehensions.rs
├── decorators.rs
├── context.rs
├── types.rs
└── async.rs
```

**Step 2**: Update parser to support Python syntax
```rust
// src/parser.rs (enhanced)
fn parse_generator_expr(&mut self) -> Result<Expr> {
    // for var in iterable if condition
    // yield expression
}

fn parse_comprehension(&mut self) -> Result<Expr> {
    // [expr for var in iterable if cond]
    // {key: val for var in iterable if cond}
    // {expr for var in iterable if cond}
}

fn parse_decorator(&mut self) -> Result<Decorator> {
    // @decorator_name(args)
}

fn parse_with_statement(&mut self) -> Result<Statement> {
    // with expr as var:
    //     body
}
```

**Step 3**: Update VM to execute Python features
```rust
// src/vm.rs (enhanced)
fn execute_yield(&mut self, value: Value) -> Result<Value> {
    // Suspend current generator
    // Save state
    // Return value
}

fn handle_comprehension(&mut self, comp: &Comprehension) -> Result<Value> {
    // Evaluate comprehension expression
    // Return result
}
```

**Step 4**: Add 10 test cases
- Test simple generator
- Test generator with multiple yields
- Test list comprehension
- Test dict comprehension with conditions
- Test generator expression laziness
- Test decorator application
- Test function decorator stacking
- Test context manager enter/exit
- Test context manager exception handling
- Test type hints enforcement

### Performance Impact
- **Generators**: Lazy evaluation saves memory (10-100x for large sequences)
- **Comprehensions**: 15-30% faster than explicit loops
- **Decorators**: < 5% overhead
- **Context managers**: Automatic resource cleanup, no performance penalty
- **Type hints**: Negligible cost if optional checking

---

## Integration Points

### Cross-Module Dependencies

**Query Optimizer → Parallel I/O**
- Optimizer generates optimal partition assignments
- Parallel I/O executes against optimized plans

**Parallel I/O → Memory Management**
- I/O uses memory manager for allocation
- Large reads trigger spilling

**Memory Management → Query Optimizer**
- Cost estimator considers memory availability
- Spill cost factored into query plans

**IDE/LSP → Python Foundation**
- LSP provides completion for Python keywords
- Type hints display in hover information

**Python Foundation → Query Optimizer**
- Comprehensions transpiled to optimized queries
- Generators integrated with Spark streaming

---

## Testing Strategy

### Unit Tests (30 tests)
- Each module has 5-6 dedicated tests
- Test happy path, edge cases, errors

### Integration Tests (10 tests)
- Test cross-module interactions
- Query optimizer → Parallel I/O
- Memory management → Spilling

### Load Tests (5 tests)
- 100GB dataset with spilling
- 1000 concurrent operations
- Long-running generators

### Benchmarks (5 tests)
- Query optimization impact
- I/O throughput scaling
- Memory spill performance

---

## Build & Deployment

### Build Commands
```bash
# All modules
cargo build --release

# Specific module test
cargo test -p spark --lib

# Full test suite
cargo test --all

# Benchmark
cargo bench
```

### Build Artifacts
- **Binary**: killer-native.exe (~1.3 MB)
- **Library**: libkiller_vm.rlib (~2 MB)
- **Tests**: 50+ test executables

### Deployment Process
1. Build release binary
2. Run full test suite (should pass 100%)
3. Run performance benchmarks
4. Package binary + standard library
5. Create release on GitHub

---

## Success Criteria

### Week 8 Completion (100% success)
- ✅ Query Optimizer: 500 lines, 5 tests, < 200ms query plan time
- ✅ Parallel I/O: 800 lines, 10 tests, 80% CPU utilization
- ✅ Memory Management: 600 lines, 8 tests, handles 100GB datasets
- ✅ IDE/LSP: 2,500 lines, 15 tests, < 100ms response times
- ✅ Python Layer: 2,500 lines, 10 tests, all features working

### Code Quality
- ✅ Zero compiler warnings
- ✅ 50+ tests, 100% pass rate
- ✅ Code coverage > 80% per module
- ✅ Full documentation with examples

### Performance Standards
- ✅ Build time: < 60 seconds
- ✅ Test time: < 30 seconds
- ✅ LSP response: < 100ms
- ✅ Query planning: < 200ms
- ✅ I/O throughput: > 500 MB/sec

---

## Timeline

### Days 1-2: Query Optimizer
- `src/spark/optimizer.rs` (500 lines)
- Integrated with SQL executor
- 5 passing tests

### Days 2-3: Parallel I/O
- `src/spark/io_parallel.rs` (800 lines)
- Thread pool implementation
- 10 passing tests

### Days 3-4: Memory Management
- `src/spark/memory.rs` (600 lines)
- LRU caching and spilling
- 8 passing tests

### Days 1-3 (Parallel): Python Foundation
- `src/python/` directory (2,500 lines across 5 files)
- Generators, comprehensions, decorators
- 10 passing tests

### Days 4-7: IDE/LSP Server
- `src/server/` directory (2,500 lines across 7 files)
- Full protocol implementation
- 15 passing tests

### Day 7: Integration & Testing
- Cross-module integration
- Load and performance tests
- Build verification
- Release binary creation

---

**End of Week 8 Plan**
**Ready to execute →**

