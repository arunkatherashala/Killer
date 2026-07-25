# Killer AI Framework - Documentation

**Killer AI** brings intelligent code generation, pattern matching, and agent orchestration to the Killer language.

## Overview

The Killer AI Framework consists of **two complementary subsystems**:

### 1. **killer_db** - Vector Database (Phase 6)
High-performance vector storage for semantic search and knowledge retrieval.

**Use Case**: Store Killer patterns, examples, and documentation as embeddings. Agents query killer_db to find relevant context before generating code.

### 2. **killer_tool_use_dsl** - Tool Definition Language (Phase 5)
Framework for defining tools that LLMs can call. Enables agent function calling with safety guarantees.

**Use Case**: Mark Killer functions as `@tool` so LLMs can discover and invoke them safely.

## Quick Start

### killer_db: Store & Search Embeddings

```rust
use killer_db::{KillerDB, Vector, SearchQuery};

// Create database
let mut db = KillerDB::new();

// Insert vector with metadata
let doc = Vector::new("ghost_layer_001", vec![0.23, -0.45, ..., 0.81])
    .with_metadata("title", "Ghost Layer Optimization")
    .with_metadata("category", "performance");

db.insert(doc).ok();

// Search for similar vectors
let query = SearchQuery::new(
    Vector::new("q", vec![0.25, -0.43, ..., 0.79]),
    top_k: 5
);

let results = db.search(&query);
for result in results {
    println!("{}: {} similarity", result.vector.id, result.score);
}
```

### killer_tool_use_dsl: Define Tools for LLMs

```rust
use killer_tool_use_dsl::{ToolDefinition, ToolParameter, ToolRegistry};

// Define a tool
let optimize_tool = ToolDefinition::new(
    "optimize_code",
    "Apply Ghost Layer optimizations to Killer code"
)
.with_category("optimization")
.with_parameter(
    ToolParameter::new("code", "string", "Killer code to optimize")
)
.with_parameter(
    ToolParameter::new("target", "string", "Optimization target")
        .with_enum(vec!["speed".to_string(), "memory".to_string()])
)
.with_return_type("string");

// Register tool
let mut registry = ToolRegistry::new();
registry.register(optimize_tool);

// Generate LLM-compatible schema
let openai_schema = registry.to_openai_schema();
let claude_schema = registry.to_claude_schema();
```

## Architecture

```
+-----------------------------------------+
|      Killer AI Agent (Your Code)        |
+------------+----------------------------+
             |
      +------+------+
      ↓             ↓
+--------------+  +------------------+
|  killer_db   |  |killer_tool_use_dsl
| (Knowledge)  |  | (Capabilities)
+--------------+  +------------------+
| Embeddings   |  | Tool definitions |
| Metadata     |  | Schemas (OpenAI) |
| Collections  |  | Schemas (Claude) |
| Search       |  | Registry         |
| Stats        |  | Execution        |
+--------------+  +------------------+
      ↓             ↓
      +------+------+
             ↓
    +---------------------+
    |  LLM (GPT-4/Claude) |
    |  Gets context from  |
    |  killer_db + tools  |
    +---------------------+
```

## Documentation Structure

- **[killer_db API Reference](killer_db_api.md)** - Complete killer_db API
- **[killer_tool_use_dsl API Reference](killer_tool_use_dsl_api.md)** - Tool DSL API
- **[Integration Guide](integration_guide.md)** - Using both together
- **[Examples](examples.md)** - Real-world code examples
- **[Best Practices](best_practices.md)** - Performance & design tips

## Key Concepts

### Vectors & Embeddings
A **Vector** is a numerical representation of text/code as a list of floating-point numbers. Vectors allow semantic similarity comparison.

```rust
Vector {
    id: "doc_001",
    values: [0.5, -0.3, 0.8, ...],  // 1,536+ dimensions
    metadata: {"topic": "optimization"},
    timestamp: 1710768000,
}
```

### Tools & Function Calling
A **Tool** is a function that an LLM can discover and invoke. killer_tool_use_dsl generates LLM-compatible schemas.

```rust
Tool {
    name: "search_docs",
    parameters: [
        Parameter { name: "query", type: "string" },
        Parameter { name: "limit", type: "number" }
    ],
    return_type: "array"
}
```

### Semantic Search
killer_db finds **semantically similar** vectors, not just keyword matches.

- Query: "real-time system optimization"
- Results: Ghost Layer docs (0.94), hot path detection (0.89), JIT guide (0.85)

## Phase Roadmap

| Phase | Component | Status | Focus |
|-------|-----------|--------|-------|
| 5 | killer_tool_use_dsl | ✅ Complete | Tool definitions, schemas, registry |
| 6 | killer_db | ✅ Complete | Vector storage, search, metadata |
| 7 | Multi-Agent Patterns | ⏳ Coming | Agent orchestration, cooperation |
| 8 | HNSW Indexing | ⏳ Coming | Performance: O(N) → O(log N) |
| 9+ | Advanced Features | ⏳ Planned | Filtering, clustering, backends |

## Performance

### Current (Phase 6)
- **Vector Search**: O(N) linear scan
- **Small datasets**: < 1ms (< 1K vectors)
- **Medium datasets**: < 100ms (< 100K vectors)
- **Tool Registration**: < 1ms (up to 1K tools)

### Future (Phase 8+)
- **HNSW Index**: O(log N) hierarchical search
- **Large datasets**: < 10ms (1M+ vectors)
- **Schema generation**: < 1ms (10K+ tools)

## Storage

### Memory Usage
```
Per Vector (small):   ~28 bytes
Per Vector (1.5K dim): ~6 KB
Per Tool Definition:  ~100-500 bytes
Per Metadata Entry:   ~50 bytes
```

### Scale Examples
```
10 MB killer_db    → 1.7M small vectors
1 MB tool registry → 10K tools
100 MB total       → Production AI agent
```

## Security

killer_db stores only:
- ✅ Embeddings (numerical)
- ✅ Metadata (JSON strings)
- ✅ IDs (strings)

killer_db does NOT store:
- ❌ Credentials (use environment variables)
- ❌ Passwords (use external vaults)
- ❌ User data (store separately)

## Getting Started

1. **[Installation](installation.md)** - Add killer_db + killer_tool_use_dsl to your project
2. **[Basic Usage](basic_usage.md)** - Simple examples
3. **[Building Agents](building_agents.md)** - Create AI agents
4. **[API Reference](api_reference.md)** - Complete API documentation

## Contributing

Killer AI is open source. Contributions welcome!

- Report issues: Create a GitHub issue
- Add features: Submit a pull request
- Improve docs: Edit markdown files
- Share examples: Add to examples/ folder

## License

Killer AI Framework is part of the Killer language - MIT License

## Support

- **Documentation**: See links above
- **Examples**: Check [examples/](examples.md)
- **Community**: Join Killer Discord
- **Issues**: GitHub Issues

---

**Last Updated**: March 18, 2026  
**Version**: Phase 5-6 (Pre-release)  
**Status**: Production-ready for basic use, expanding functionality
