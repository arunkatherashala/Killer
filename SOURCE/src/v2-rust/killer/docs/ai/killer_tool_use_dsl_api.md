# killer_tool_use_dsl API Reference

**killer_tool_use_dsl** enables defining tools that LLMs can discover and call.

## Table of Contents
1. [ToolParameter](#toolparameter)
2. [ToolDefinition](#tooldefinition)
3. [ToolRegistry](#toolregistry)
4. [ToolInvocation](#toolinvocation)
5. [ToolExecutor](#toolexecutor)
6. [Examples](#examples)

---

## ToolParameter

### Description
Defines a single parameter for a tool/function that an LLM can call.

### Definition
```rust
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,              // "string", "number", "boolean", "array", "object"
    pub description: String,
    pub required: bool,
    pub enum_values: Option<Vec<String>>,
}
```

### Constructor

#### `ToolParameter::new(name: &str, param_type: &str, description: &str) -> Self`
Create new parameter (required by default).

**Parameters:**
- `name` - Parameter name (e.g., "query", "limit")
- `param_type` - Parameter type: "string", "number", "boolean", "array", "object"
- `description` - What this parameter does

**Example:**
```rust
let query_param = ToolParameter::new("query", "string", "Search query text");
```

### Methods

#### `optional(self) -> Self`
Mark parameter as optional (chainable).

**Returns:** Self

**Example:**
```rust
let limit = ToolParameter::new("limit", "number", "Max results")
    .optional();  // Now optional
```

#### `with_enum(self, values: Vec<String>) -> Self`
Restrict parameter to specific values (chainable).

**Parameters:**
- `values` - Allowed values

**Returns:** Self

**Example:**
```rust
let mode = ToolParameter::new("mode", "string", "Optimization mode")
    .with_enum(vec!["speed".to_string(), "memory".to_string()]);
```

#### `to_json_schema(self) -> String`
Generate JSON schema for LLM compatibility.

**Returns:** JSON schema string

**Example:**
```rust
let schema = param.to_json_schema();
// {"type": "string", "description": "Search query..."}
```

---

## ToolDefinition

### Description
Complete definition of a tool/function that LLMs can call.

### Definition
```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: String,               // "data", "computation", "io", "analysis", "ai"
    pub parameters: Vec<ToolParameter>,
    pub return_type: String,            // "string", "number", "object", "array"
    pub example_usage: String,
    pub confidence_score: f32,          // 0-1: how well LLM understands
}
```

### Constructor

#### `ToolDefinition::new(name: &str, description: &str) -> Self`
Create new tool definition.

**Parameters:**
- `name` - Function name (e.g., "search", "optimize_code")
- `description` - What this tool does

**Example:**
```rust
let tool = ToolDefinition::new(
    "search_patterns",
    "Find Killer optimization patterns"
);
```

### Methods

#### `with_category(self, category: &str) -> Self`
Set tool category (chainable).

**Parameters:**
- `category` - One of: "data", "computation", "io", "analysis", "ai"

**Returns:** Self

**Example:**
```rust
let tool = ToolDefinition::new("search", "Search docs")
    .with_category("data");
```

#### `with_parameter(self, param: ToolParameter) -> Self`
Add parameter to tool (chainable).

**Parameters:**
- `param` - Parameter definition

**Returns:** Self

**Example:**
```rust
let tool = ToolDefinition::new("search", "Search docs")
    .with_parameter(ToolParameter::new("query", "string", "Search query"))
    .with_parameter(ToolParameter::new("limit", "number", "Max results").optional());
```

#### `with_return_type(self, return_type: &str) -> Self`
Set return type (chainable).

**Parameters:**
- `return_type` - "string", "number", "object", or "array"

**Returns:** Self

**Example:**
```rust
let tool = ToolDefinition::new("search", "Find docs")
    .with_return_type("array");  // Returns list of results
```

#### `with_example(self, example: &str) -> Self`
Add example usage (chainable).

**Parameters:**
- `example` - Example code or usage description

**Returns:** Self

**Example:**
```rust
let tool = ToolDefinition::new("search", "Search docs")
    .with_example("search_patterns('Ghost Layer')");
```

#### `with_confidence(self, score: f32) -> Self`
Set LLM confidence score (chainable).

**Parameters:**
- `score` - 0.0-1.0 (how well LLM understands this tool)

**Returns:** Self

**Example:**
```rust
let tool = ToolDefinition::new("search", "Search docs")
    .with_confidence(0.95);  // High confidence
```

#### `to_openai_schema(self) -> String`
Generate OpenAI function calling schema.

**Returns:** JSON schema for OpenAI

**Example:**
```rust
let schema = tool.to_openai_schema();
// Includes: name, description, parameters, required, etc.
```

#### `to_claude_schema(self) -> String`
Generate Claude tool definition schema.

**Returns:** JSON schema for Claude

**Example:**
```rust
let schema = tool.to_claude_schema();
// Claude-compatible tool definition
```

---

## ToolRegistry

### Description
Stores and manages all available tools. Generates schemas for LLMs.

### Definition
```rust
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    execution_count: HashMap<String, u32>,
    success_count: HashMap<String, u32>,
}
```

### Constructor

#### `ToolRegistry::new() -> Self`
Create empty tool registry.

**Example:**
```rust
let mut registry = ToolRegistry::new();
```

### Methods

#### `register(self, tool: ToolDefinition)`
Add tool to registry.

**Parameters:**
- `tool` - Tool definition

**Example:**
```rust
let search_tool = ToolDefinition::new("search", "Search");
registry.register(search_tool);
```

#### `get(self, name: &str) -> Option<&ToolDefinition>`
Get tool by name.

**Parameters:**
- `name` - Tool name

**Returns:** Tool definition or None

**Example:**
```rust
if let Some(tool) = registry.get("search") {
    println!("Found tool: {}", tool.name);
}
```

#### `list(self) -> Vec<String>`
List all tool names.

**Returns:** Vector of tool names

**Example:**
```rust
let tools = registry.list();
for name in tools {
    println!("Tool: {}", name);
}
```

#### `by_category(self, category: &str) -> Vec<ToolDefinition>`
Get all tools in category.

**Parameters:**
- `category` - Category name

**Returns:** Vector of tool definitions

**Example:**
```rust
let search_tools = registry.by_category("data");
for tool in search_tools {
    println!("Search tool: {}", tool.name);
}
```

#### `record_execution(&mut self, tool_name: &str, success: bool)`
Update execution statistics.

**Parameters:**
- `tool_name` - Tool name
- `success` - Whether execution succeeded

**Example:**
```rust
registry.record_execution("search", true);   // Success
registry.record_execution("optimize", false); // Failed
```

#### `get_stats(self, tool_name: &str) -> Option<(u32, f32)>`
Get tool execution statistics.

**Returns:** (execution_count, success_rate) or None

**Example:**
```rust
if let Some((count, rate)) = registry.get_stats("search") {
    println!("Tool: {} calls, {:.1}% success", count, rate * 100.0);
}
```

#### `to_openai_schema(self) -> String`
Generate OpenAI schema for ALL tools.

**Returns:** JSON array of tool schemas

**Example:**
```rust
let schema = registry.to_openai_schema();
// Send to OpenAI API for function calling
```

#### `to_claude_schema(self) -> String`
Generate Claude schema for ALL tools.

**Returns:** JSON array of tool schemas

**Example:**
```rust
let schema = registry.to_claude_schema();
// Send to Claude API
```

---

## ToolInvocation

### Description
Represents LLM function call with arguments.

### Definition
```rust
pub struct ToolInvocation {
    pub tool_name: String,
    pub arguments: HashMap<String, ToolArgument>,
    pub source: String,                 // "openai", "claude", "ollama", "user"
}

pub enum ToolArgument {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<String>),
}
```

### Constructor

#### `ToolInvocation::new(tool_name: &str) -> Self`
Create new tool invocation.

**Parameters:**
- `tool_name` - Name of tool to invoke

**Example:**
```rust
let invocation = ToolInvocation::new("search");
```

### Methods

#### `with_arg(self, name: &str, value: ToolArgument) -> Self`
Add argument (chainable).

**Parameters:**
- `name` - Argument name
- `value` - Argument value (String, Number, Boolean, or Array)

**Returns:** Self

**Example:**
```rust
let invocation = ToolInvocation::new("search")
    .with_arg("query", ToolArgument::String("Ghost Layer".to_string()))
    .with_arg("limit", ToolArgument::Number(10.0))
    .with_arg("enabled", ToolArgument::Boolean(true));
```

#### `with_source(self, source: &str) -> Self`
Set source (chainable).

**Parameters:**
- `source` - "openai", "claude", "ollama", "user"

**Returns:** Self

**Example:**
```rust
let invocation = ToolInvocation::new("search")
    .with_source("openai");
```

#### `get_arg(self, name: &str) -> Option<&ToolArgument>`
Get argument value.

**Parameters:**
- `name` - Argument name

**Returns:** Argument or None

**Example:**
```rust
if let Some(query) = invocation.get_arg("query") {
    println!("Query: {}", query);
}
```

---

## ToolExecutor

### Description
Safely executes tool invocations with retries and validation.

### Definition
```rust
pub struct ToolExecutor {
    registry: ToolRegistry,
    max_retries: u32,
    timeout_ms: u64,
}
```

### Constructor

#### `ToolExecutor::new(registry: ToolRegistry) -> Self`
Create new executor.

**Parameters:**
- `registry` - Tool registry to use

**Example:**
```rust
let executor = ToolExecutor::new(registry);
```

### Methods

#### `with_max_retries(self, retries: u32) -> Self`
Set max retry attempts (chainable).

**Parameters:**
- `retries` - Number of retries

**Returns:** Self

**Example:**
```rust
let executor = ToolExecutor::new(registry)
    .with_max_retries(3);
```

#### `with_timeout(self, timeout_ms: u64) -> Self`
Set execution timeout (chainable).

**Parameters:**
- `timeout_ms` - Timeout in milliseconds

**Returns:** Self

**Example:**
```rust
let executor = ToolExecutor::new(registry)
    .with_timeout(5000);  // 5 second timeout
```

#### `execute(&mut self, invocation: &ToolInvocation) -> ToolExecutionResult`
Execute tool invocation.

**Parameters:**
- `invocation` - Tool invocation with arguments

**Returns:** ToolExecutionResult

**Example:**
```rust
let result = executor.execute(&invocation);
if result.success {
    println!("Success: {}", result.output);
} else {
    println!("Error: {}", result.error_message.unwrap());
}
```

---

## ToolExecutionResult

### Description
Result of executing a tool.

### Definition
```rust
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
}
```

### Methods

#### `ok(tool_name: &str, output: &str, time_ms: u64) -> Self`
Create successful result.

**Example:**
```rust
let result = ToolExecutionResult::ok("search", "Found 5 docs", 42);
```

#### `error(tool_name: &str, error: &str, time_ms: u64) -> Self`
Create error result.

**Example:**
```rust
let result = ToolExecutionResult::error("search", "Tool not found", 10);
```

---

## Examples

### Define & Register Tools

```rust
use killer_tool_use_dsl::*;

fn main() {
    let mut registry = ToolRegistry::new();
    
    // Define search tool
    let search = ToolDefinition::new("search_docs", "Find Killer documentation")
        .with_category("data")
        .with_parameter(
            ToolParameter::new("query", "string", "Search query")
        )
        .with_parameter(
            ToolParameter::new("limit", "number", "Max results").optional()
        )
        .with_return_type("array")
        .with_example("search_docs('Ghost Layer', 5)")
        .with_confidence(0.95);
    
    // Define optimize tool
    let optimize = ToolDefinition::new("optimize_code", "Apply optimizations")
        .with_category("ai")
        .with_parameter(
            ToolParameter::new("code", "string", "Killer code")
        )
        .with_parameter(
            ToolParameter::new("target", "string", "Optimization type")
                .with_enum(vec!["speed".to_string(), "memory".to_string()])
        )
        .with_return_type("string")
        .with_confidence(0.92);
    
    // Register tools
    registry.register(search);
    registry.register(optimize);
    
    // Generate schemas for LLMs
    let openai_schema = registry.to_openai_schema();
    let claude_schema = registry.to_claude_schema();
    
    println!("OpenAI: {}", openai_schema);
    println!("Claude: {}", claude_schema);
}
```

### Execute Tool Call

```rust
// Assume LLM made this call
let invocation = ToolInvocation::new("search_docs")
    .with_source("openai")
    .with_arg("query", ToolArgument::String("real-time".to_string()))
    .with_arg("limit", ToolArgument::Number(5.0));

// Execute it
let mut executor = ToolExecutor::new(registry)
    .with_max_retries(2)
    .with_timeout(5000);

let result = executor.execute(&invocation);

match result.success {
    true => println!("✓ {} in {}ms", result.output, result.execution_time_ms),
    false => println!("✗ Error: {}", result.error_message.unwrap()),
}
```

### Get Statistics

```rust
// Execute some tools
executor.execute(&invocation1);
executor.execute(&invocation2);
executor.execute(&bad_invocation);  // Fails

// Check stats
if let Some((count, success_rate)) = registry.get_stats("search_docs") {
    println!("Search tool: {} executions, {:.1}% success",
        count,
        success_rate * 100.0);
}
```

---

## Best Practices

1. **Use descriptive names** - "search_patterns" not "search"
2. **Clear descriptions** - Explain what the tool does
3. **Set confidence** - Tell LLM how well you understand (0-1)
4. **Validate inputs** - Check required parameters
5. **Use categories** - Organize tools by type (data, computation, etc.)
6. **Track statistics** - Monitor success rates
7. **Add examples** - Show LLM how to use the tool

---

**API Version**: Phase 5  
**Last Updated**: March 18, 2026
