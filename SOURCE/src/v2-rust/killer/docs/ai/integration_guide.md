# Integration Guide: killer_db + killer_tool_use_dsl

This guide shows how to combine **killer_db** (knowledge retrieval) and **killer_tool_use_dsl** (function calling) to create AI agents.

## Architecture

```
+-----------------------------+
|   Your Killer AI Agent      |
|  (with @ai_assist annotation)
+------------+----------------+
             |
      +------+------+
      ↓             ↓
+-------------+  +--------------+
| killer_db   |  |  Tool Registry
| (Knowledge) |  |  (Capabilities)
+-------------+  +--------------+
      ↓             ↓
      +------+------+
             ↓
    +---------------------+
    |  LLM (GPT-4/Claude) |
    |  + Context + Tools  |
    +----------+----------+
               ↓
         Generated Code
```

## Step 1: Set Up killer_db

```rust
use killer_db::{KillerDB, Vector};

fn setup_knowledge_base() -> KillerDB {
    let mut db = KillerDB::new();
    
    // Create collections
    db.create_collection("patterns", 1536).ok();
    db.create_collection("examples", 1536).ok();
    
    // Insert Killer patterns (pre-embedded)
    let ghost_layer_doc = Vector::new("pattern_001", vec![
        0.48, -0.23, 0.81, /*... 1533 more values ...*/
    ])
    .with_metadata("title", "Ghost Layer Optimization")
    .with_metadata("category", "patterns")
    .with_metadata("topic", "performance");
    
    db.insert(ghost_layer_doc).ok();
    
    // Insert more patterns, examples, etc.
    
    db
}
```

## Step 2: Set Up Tool Registry

```rust
use killer_tool_use_dsl::*;

fn setup_tools() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    
    // Define optimization tool
    let optimize_tool = ToolDefinition::new(
        "optimize_code",
        "Apply Ghost Layer and Assassin Layer optimizations"
    )
    .with_category("ai")
    .with_parameter(
        ToolParameter::new("code", "string", "Killer code to optimize")
    )
    .with_parameter(
        ToolParameter::new("target", "string", "Optimization: speed or memory")
            .with_enum(vec!["speed".to_string(), "memory".to_string()])
    )
    .with_return_type("string")
    .with_confidence(0.95);
    
    registry.register(optimize_tool);
    
    // Define code review tool
    let review_tool = ToolDefinition::new(
        "review_code",
        "Review code for best practices and security"
    )
    .with_category("ai")
    .with_parameter(
        ToolParameter::new("code", "string", "Code to review")
    )
    .with_return_type("object")
    .with_confidence(0.92);
    
    registry.register(review_tool);
    
    registry
}
```

## Step 3: Create AI Agent

```rust
use killer_llm_integration::LLMClient;

struct KillerAIAgent {
    db: KillerDB,
    tools: ToolRegistry,
    executor: ToolExecutor,
    llm: LLMClient,
}

impl KillerAIAgent {
    pub fn new(db: KillerDB, tools: ToolRegistry, llm: LLMClient) -> Self {
        let executor = ToolExecutor::new(tools.clone())
            .with_max_retries(3)
            .with_timeout(5000);
        
        KillerAIAgent {
            db,
            tools,
            executor,
            llm,
        }
    }
    
    pub fn generate_code(&mut self, request: &str) -> Result<String, String> {
        // Step 1: Convert request to embedding
        let request_embedding = self.llm.embed(request)?;
        
        // Step 2: Search killer_db for similar patterns
        let query = SearchQuery::new(
            Vector::new("q", request_embedding),
            top_k: 5
        )
        .with_min_score(0.7);
        
        let context_docs = self.db.search(&query);
        let context = context_docs.iter()
            .map(|r| r.vector.metadata.get("title").unwrap_or(&"Unknown".to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        
        // Step 3: Get available tools
        let tool_schemas = self.tools.to_openai_schema();
        
        // Step 4: Create prompt with context and tools
        let prompt = format!(
            "You are a Killer language expert assistant.\n\
            Use these patterns from Killer: {}\n\
            You have these tools available: {}\n\
            \n\
            User request: {}\n\
            \n\
            Generate optimized Killer code.",
            context, tool_schemas, request
        );
        
        // Step 5: Call LLM
        let response = self.llm.generate(&prompt)?;
        
        // Step 6: If response includes tool calls, execute them
        // (Parse response for tool invocations)
        if response.contains("@tool[") {
            // Parse and execute tool calls
            // Then incorporate results back into response
        }
        
        Ok(response)
    }
}
```

## Step 4: Complete Example

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up components
    let db = setup_knowledge_base();
    let tools = setup_tools();
    let llm = LLMClient::new(LLMConfig::openai_gpt4());
    
    // Create agent
    let mut agent = KillerAIAgent::new(db, tools, llm);
    
    // Generate code
    let request = "Build a real-time HTTP server with 1000 req/sec throughput";
    let code = agent.generate_code(request)?;
    
    println!("Generated code:\n{}", code);
    
    Ok(())
}
```

## Real Workflow Example

### User Input
```
@ai_assist("Build a real-time data pipeline like SuperProcessor")
fn build_pipeline() {
    // AI fills this in
}
```

### What Happens Behind the Scenes

**1. Request Processing**
```
LLM sees: "@ai_assist('Build real-time data pipeline')"
```

**2. Context Retrieval**
```
Agent queries killer_db:
  Query: embed("Build real-time data pipeline")
  Results:
    - SuperProcessor docs (0.94 similarity)
    - Stream processing guide (0.89)
    - Hot path detection (0.86)
    - Performance benchmarks (0.82)
```

**3. Tool Discovery**
```
Available tools:
  - optimize_code(code, target)
  - review_code(code)
  - benchmark(code)
  - profile(code)
```

**4. LLM Processing**
```
LLM Prompt:
  "You are Killer expert
   Patterns: SuperProcessor, Stream Processing, Hot Path
   Tools: optimize_code, review_code, benchmark
   Create: real-time pipeline code"
```

**5. LLM Response**
```
Generated code with @tool calls:
  fn build_pipeline() {
      let processor = SuperProcessor::new()
          .with_ghost_layer()
      @tool[optimize_code](processor_code, "speed")
      @tool[benchmark](processor_code)
  }
```

**6. Tool Execution**
```
Agent executes:
  - optimize_code() → optimized version
  - benchmark() → performance metrics
```

**7. Final Result**
```
Returned to user:
  - Production-ready code ✅
  - Optimizations applied ✅
  - Performance validated ✅
```

## Advanced: Custom Tool Implementation

```rust
impl KillerAIAgent {
    pub fn register_custom_tool(
        &mut self,
        name: &str,
        description: &str,
        handler: Box<dyn Fn(&str) -> String>
    ) {
        let tool = ToolDefinition::new(name, description)
            .with_category("custom")
            .with_parameter(
                ToolParameter::new("input", "string", "Tool input")
            )
            .with_return_type("string");
        
        self.tools.register(tool);
        // Store handler for later execution
    }
}
```

## Performance Tips

### 1. Pre-Embed Knowledge
```rust
// Instead of embedding every time:
// ❌ let embedding = llm.embed(doc);  // Slow

// ✅ Pre-embed and store in killer_db
db.insert(Vector::new("doc_1", pre_computed_embedding));
```

### 2. Use Collections
```rust
// Organize vectors by type
db.create_collection("patterns", 1536);
db.create_collection("examples", 1536);
db.create_collection("tutorials", 1536);

// Search specific collections
let query = SearchQuery::new(vec, 5)
    .with_filter("collection", "patterns");
```

### 3. Batch Operations
```rust
// Instead of inserting one by one:
let vectors = vec![vec1, vec2, vec3, vec4, vec5];
db.batch_insert(vectors)?;  // Much faster
```

### 4. Set Min Scores
```rust
// Filter out low-relevance results early
let query = SearchQuery::new(vec, 10)
    .with_min_score(0.75);  // Only good matches
```

### 5. Cache Tool Schemas
```rust
// Generate schemas once, reuse many times
let cached_schema = registry.to_openai_schema();
// Use cached_schema for multiple LLM calls
```

## Error Handling

```rust
pub fn safe_generate_code(&mut self, request: &str) -> Result<String, String> {
    // Validate input
    if request.is_empty() {
        return Err("Request cannot be empty".to_string());
    }
    
    if request.len() > 10000 {
        return Err("Request too long".to_string());
    }
    
    // Try to generate code
    match self.generate_code(request) {
        Ok(code) => {
            // Validate output
            if code.is_empty() {
                Err("No code generated".to_string())
            } else {
                Ok(code)
            }
        }
        Err(e) => Err(format!("Generation failed: {}", e)),
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_basic() {
        let db = setup_knowledge_base();
        let tools = setup_tools();
        let llm = MockLLMClient::new();
        
        let mut agent = KillerAIAgent::new(db, tools, llm);
        let result = agent.generate_code("Simple function").ok();
        
        assert!(result.is_some());
    }
    
    #[test]
    fn test_tool_execution() {
        let tools = setup_tools();
        let mut executor = ToolExecutor::new(tools);
        
        let invocation = ToolInvocation::new("optimize_code")
            .with_arg("code", ToolArgument::String("fn add(a, b) { a + b }".to_string()))
            .with_arg("target", ToolArgument::String("speed".to_string()));
        
        let result = executor.execute(&invocation);
        assert!(result.success);
    }
}
```

## Deployment Checklist

- [ ] killer_db loaded with patterns/examples
- [ ] Tool registry configured
- [ ] LLM client initialized
- [ ] Error handling in place
- [ ] Performance tested (< 100ms per query)
- [ ] Memory usage acceptable (< 1GB)
- [ ] Logging configured
- [ ] Monitoring metrics set up

---

**Next**: Check out [Examples](examples.md) for more patterns.

**Last Updated**: March 18, 2026
