# Killer AI Examples

Practical examples using killer_db + killer_tool_use_dsl.

## Table of Contents
1. [Basic Vector Search](#basic-vector-search)
2. [Tool Discovery](#tool-discovery)
3. [Real-time Agent](#real-time-agent)
4. [Code Optimization](#code-optimization)
5. [Multi-Agent Collaboration](#multi-agent-collaboration)

---

## Basic Vector Search

### Example 1: Search for Optimization Patterns

```rust
use killer_db::{KillerDB, Vector, SearchQuery, SimilarityMetric};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = KillerDB::new();
    
    // Index Killer documentation
    let docs = vec![
        Vector::new("doc_001", vec![0.82, -0.15, 0.45, /*...*/])
            .with_metadata("title", "Ghost Layer Optimization")
            .with_metadata("category", "performance"),
        
        Vector::new("doc_002", vec![0.80, -0.17, 0.47, /*...*/])
            .with_metadata("title", "Hot Path Detection")
            .with_metadata("category", "performance"),
        
        Vector::new("doc_003", vec![0.31, 0.72, -0.09, /*...*/])
            .with_metadata("title", "Actor Model Patterns")
            .with_metadata("category", "concurrency"),
    ];
    
    db.batch_insert(docs)?;
    
    // Search for performance optimizations
    let performance_query = Vector::new("q", vec![0.81, -0.16, 0.46, /*...*/]);
    let results = db.search(
        &SearchQuery::new(performance_query, 3)
            .with_min_score(0.7)
            .with_metric(SimilarityMetric::Cosine)
    );
    
    println!("Performance-related docs:");
    for result in results {
        println!("  {} - Similarity: {:.2}", 
            result.vector.metadata.get("title").unwrap(),
            result.score);
    }
    
    Ok(())
}
```

### Example 2: Filtered Search

```rust
fn search_by_category(db: &KillerDB, category: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // In future phase, this would be optimized with metadata filtering
    // For now, retrieve and filter manually
    for doc_id in ["doc_001", "doc_002", "doc_003"] {
        if let Some(vec) = db.get(doc_id) {
            if let Some(doc_cat) = vec.metadata.get("category") {
                if doc_cat == category {
                    results.push(vec_id.clone());
                }
            }
        }
    }
    
    results
}

// Usage
let concurrency_docs = search_by_category(&db, "concurrency");
```

---

## Tool Discovery

### Example 1: Register Killer Tools

```rust
use killer_tool_use_dsl::*;

fn setup_killer_tools() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    
    // Tool 1: Code Optimization
    registry.register(
        ToolDefinition::new(
            "optimize",
            "Apply Ghost Layer optimizations to Killer code"
        )
        .with_category("ai")
        .with_parameter(ToolParameter::new("code", "string", "Source code"))
        .with_parameter(
            ToolParameter::new("level", "string", "Optimization level")
                .with_enum(vec!["conservative".into(), "aggressive".into()])
                .optional()
        )
        .with_return_type("string")
        .with_example("optimize(code, \"aggressive\")")
        .with_confidence(0.95)
    );
    
    // Tool 2: Security Review
    registry.register(
        ToolDefinition::new(
            "security_review",
            "Check code for security vulnerabilities"
        )
        .with_category("ai")
        .with_parameter(ToolParameter::new("code", "string", "Source code"))
        .with_return_type("object")
        .with_confidence(0.90)
    );
    
    // Tool 3: Performance Analysis
    registry.register(
        ToolDefinition::new(
            "analyze_performance",
            "Analyze code for performance bottlenecks"
        )
        .with_category("ai")
        .with_parameter(ToolParameter::new("code", "string", "Source code"))
        .with_return_type("object")
        .with_confidence(0.92)
    );
    
    registry
}

fn main() {
    let registry = setup_killer_tools();
    
    // List all tools
    println!("Available tools:");
    for tool_name in registry.list() {
        println!("  - {}", tool_name);
    }
    
    // Generate LLM schemas
    let openai_schema = registry.to_openai_schema();
    println!("\nOpenAI schema:\n{}", openai_schema);
}
```

### Example 2: Tool Execution

```rust
fn execute_optimization(registry: ToolRegistry, code: &str) -> Result<String, String> {
    let mut executor = ToolExecutor::new(registry)
        .with_max_retries(2)
        .with_timeout(5000);
    
    let invocation = ToolInvocation::new("optimize")
        .with_source("llm")
        .with_arg("code", ToolArgument::String(code.to_string()))
        .with_arg("level", ToolArgument::String("aggressive".to_string()));
    
    let result = executor.execute(&invocation);
    
    if result.success {
        Ok(result.output)
    } else {
        Err(result.error_message.unwrap())
    }
}
```

---

## Real-time Agent

### Complete AI Agent Example

```rust
use killer_db::KillerDB;
use killer_tool_use_dsl::*;
use killer_llm_integration::LLMClient;

struct RealtimeAgent {
    db: KillerDB,
    registry: ToolRegistry,
    executor: ToolExecutor,
}

impl RealtimeAgent {
    pub fn new(db: KillerDB, registry: ToolRegistry) -> Self {
        let executor = ToolExecutor::new(registry.clone())
            .with_max_retries(3)
            .with_timeout(5000);
        
        RealtimeAgent { db, registry, executor }
    }
    
    pub fn assist_with_code(&mut self, request: &str) -> Result<String, String> {
        println!("User request: {}", request);
        
        // Step 1: Query knowledge base
        let relevant_patterns = self.find_relevant_patterns(request)?;
        println!("Found {} relevant patterns", relevant_patterns.len());
        
        // Step 2: Get available tools
        let tools_available = self.registry.list();
        println!("Using {} tools", tools_available.len());
        
        // Step 3: Generate code (simplified - normally call LLM)
        let generated_code = self.generate_code(request, &relevant_patterns)?;
        
        // Step 4: Optimize it
        let optimized = self.optimize_code(&generated_code)?;
        
        Ok(optimized)
    }
    
    fn find_relevant_patterns(&self, request: &str) -> Result<Vec<String>, String> {
        // Simplified - normally would embed and search
        let patterns = vec![
            "Ghost Layer".to_string(),
            "Hot Path Detection".to_string(),
        ];
        Ok(patterns)
    }
    
    fn generate_code(&self, request: &str, patterns: &[String]) -> Result<String, String> {
        // Simplified - normally would call LLM
        Ok(format!(
            "// Generated code for: {}\n// Using: {}\nfn my_impl() {{\n}}\n",
            request,
            patterns.join(", ")
        ))
    }
    
    fn optimize_code(&mut self, code: &str) -> Result<String, String> {
        let invocation = ToolInvocation::new("optimize")
            .with_arg("code", ToolArgument::String(code.to_string()))
            .with_arg("level", ToolArgument::String("aggressive".to_string()));
        
        let result = self.executor.execute(&invocation);
        
        if result.success {
            Ok(result.output)
        } else {
            Err(result.error_message.unwrap_or_default())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = KillerDB::new();
    let registry = setup_killer_tools();
    let mut agent = RealtimeAgent::new(db, registry);
    
    let request = "Build a high-performance HTTP server for real-time data";
    agent.assist_with_code(request)?;
    
    Ok(())
}
```

---

## Code Optimization

### Example: Multi-Step Optimization

```rust
fn optimize_full_pipeline(
    mut agent: RealtimeAgent,
    code: &str
) -> Result<OptimizationResult, String> {
    
    // Step 1: Security Review
    let security_result = execute_tool(&mut agent, "security_review", code)?;
    println!("Security review: {}", security_result);
    
    // Step 2: Performance Analysis
    let perf_result = execute_tool(&mut agent, "analyze_performance", code)?;
    println!("Performance: {}", perf_result);
    
    // Step 3: Optimization
    let optimized = execute_tool(&mut agent, "optimize", code)?;
    println!("Optimized code:\n{}", optimized);
    
    Ok(OptimizationResult {
        original: code.to_string(),
        optimized,
        security: security_result,
        performance: perf_result,
    })
}

struct OptimizationResult {
    original: String,
    optimized: String,
    security: String,
    performance: String,
}

fn execute_tool(
    agent: &mut RealtimeAgent,
    tool: &str,
    code: &str
) -> Result<String, String> {
    let invocation = ToolInvocation::new(tool)
        .with_arg("code", ToolArgument::String(code.to_string()));
    
    let result = agent.executor.execute(&invocation);
    
    if result.success {
        Ok(result.output)
    } else {
        Err(result.error_message.unwrap_or_default())
    }
}
```

---

## Multi-Agent Collaboration

### Example: Team of AI Agents

```rust
struct AITeam {
    researcher: TeamAgent,
    reviewer: TeamAgent,
    optimizer: TeamAgent,
}

struct TeamAgent {
    name: String,
    expertise: String,
    registry: ToolRegistry,
}

impl AITeam {
    pub fn solve_problem(&mut self, request: &str) -> Result<Solution, String> {
        // Step 1: Researcher finds patterns
        let patterns = self.researcher.research(request)?;
        
        // Step 2: Reviewer checks quality
        let reviewed = self.reviewer.review(&patterns)?;
        
        // Step 3: Optimizer improves
        let optimized = self.optimizer.optimize(&reviewed)?;
        
        Ok(Solution {
            request: request.to_string(),
            patterns,
            reviewed,
            optimized,
        })
    }
}

struct Solution {
    request: String,
    patterns: String,
    reviewed: String,
    optimized: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut team = AITeam {
        researcher: TeamAgent {
            name: "Researcher".to_string(),
            expertise: "Pattern Discovery".to_string(),
            registry: ToolRegistry::new(),
        },
        reviewer: TeamAgent {
            name: "Reviewer".to_string(),
            expertise: "Quality Assurance".to_string(),
            registry: ToolRegistry::new(),
        },
        optimizer: TeamAgent {
            name: "Optimizer".to_string(),
            expertise: "Performance".to_string(),
            registry: ToolRegistry::new(),
        },
    };
    
    let solution = team.solve_problem("Build scalable API")?;
    println!("Solution: {:?}", solution);
    
    Ok(())
}
```

---

## Advanced Patterns

### Pattern 1: Adaptive Optimization

```rust
fn adaptive_optimize(
    code: &str,
    metrics: &PerformanceMetrics
) -> Result<String, String> {
    if metrics.latency_ms > 100 {
        optimize_for("speed", code)
    } else if metrics.memory_mb > 512 {
        optimize_for("memory", code)
    } else {
        Ok(code.to_string())
    }
}

struct PerformanceMetrics {
    latency_ms: u32,
    memory_mb: u32,
}

fn optimize_for(target: &str, code: &str) -> Result<String, String> {
    // Tool invocation for specific target
    Ok(format!("/* Optimized for {} */\n{}", target, code))
}
```

### Pattern 2: Iterative Refinement

```rust
fn iterative_improve(
    mut agent: RealtimeAgent,
    code: &str,
    iterations: usize
) -> Result<String, String> {
    let mut current = code.to_string();
    
    for i in 0..iterations {
        println!("Iteration {}", i + 1);
        
        // Optimize
        current = execute_optimization(&mut agent, &current)?;
        
        // Review
        let review = execute_review(&mut agent, &current)?;
        
        if review.contains("✓") {
            break;  // Good enough
        }
    }
    
    Ok(current)
}
```

---

**See also**: [Integration Guide](integration_guide.md) for more patterns

**Last Updated**: March 18, 2026
