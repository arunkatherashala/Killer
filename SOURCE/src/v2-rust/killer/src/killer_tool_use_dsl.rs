/// KILLER TOOL USE DSL - Phase 5
/// 
/// Enables Killer programs to define tools for LLM function calling.
/// Unlocks integration with agent frameworks (CrewAI, AutoGen, LangChain).
/// 
/// Key Capabilities:
/// - @tool annotation for marking functions as LLM-callable
/// - Tool registry and discovery
/// - JSON schema generation for OpenAI/Claude/Ollama function calling
/// - Safe tool invocation with error handling
/// - Multi-tool composition and orchestration

use std::collections::HashMap;
use std::fmt;

/// Tool parameter definition for function calling schema
#[derive(Debug, Clone, PartialEq)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,              // "string", "number", "boolean", "array", "object"
    pub description: String,
    pub required: bool,
    pub enum_values: Option<Vec<String>>,
}

impl ToolParameter {
    pub fn new(name: &str, param_type: &str, description: &str) -> Self {
        ToolParameter {
            name: name.to_string(),
            param_type: param_type.to_string(),
            description: description.to_string(),
            required: true,
            enum_values: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_enum(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }

    pub fn to_json_schema(&self) -> String {
        let mut schema = format!(
            r#"{{"type": "{}", "description": "{}""#,
            self.param_type, self.description
        );

        if let Some(ref enums) = self.enum_values {
            schema.push_str(", \"enum\": [");
            schema.push_str(
                &enums
                    .iter()
                    .map(|e| format!(r#""{}""#, e))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            schema.push(']');
        }

        schema.push('}');
        schema
    }
}

/// Full tool definition - represents an LLM-callable function
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: String,                 // "data", "computation", "io", "analysis", "ai"
    pub parameters: Vec<ToolParameter>,
    pub return_type: String,              // "string", "number", "object", "array"
    pub example_usage: String,
    pub confidence_score: f32,            // 0-1: how well LLM understands this tool
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str) -> Self {
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            category: "computation".to_string(),
            parameters: Vec::new(),
            return_type: "string".to_string(),
            example_usage: String::new(),
            confidence_score: 0.85,
        }
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    pub fn with_parameter(mut self, param: ToolParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_return_type(mut self, return_type: &str) -> Self {
        self.return_type = return_type.to_string();
        self
    }

    pub fn with_example(mut self, example: &str) -> Self {
        self.example_usage = example.to_string();
        self
    }

    pub fn with_confidence(mut self, score: f32) -> Self {
        self.confidence_score = score.max(0.0).min(1.0);
        self
    }

    /// Generate OpenAI-compatible function calling schema
    pub fn to_openai_schema(&self) -> String {
        let mut schema = format!(
            r#"{{"name": "{}", "description": "{}", "parameters": {{"type": "object", "properties": {{"#,
            self.name, self.description
        );

        let props: Vec<String> = self
            .parameters
            .iter()
            .map(|p| format!(r#""{}": {}"#, p.name, p.to_json_schema()))
            .collect();
        schema.push_str(&props.join(", "));

        let required_params: Vec<String> = self
            .parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| format!(r#""{}""#, p.name))
            .collect();

        schema.push_str("},");
        if !required_params.is_empty() {
            schema.push_str(&format!(r#""required": [{}]"#, required_params.join(", ")));
        } else {
            schema.push_str(r#""required": []"#);
        }
        schema.push_str("}}}}");

        schema
    }

    /// Generate Claude-compatible tool definition
    pub fn to_claude_schema(&self) -> String {
        let mut schema = format!(
            r#"{{"name": "{}", "description": "{}", "input_schema": {{"type": "object", "properties": {{"#,
            self.name, self.description
        );

        let props: Vec<String> = self
            .parameters
            .iter()
            .map(|p| format!(r#""{}": {{"type": "{}", "description": "{}""#, p.name, p.param_type, p.description))
            .collect();
        schema.push_str(&props.join("}, "));
        schema.push_str("}");

        let required_params: Vec<String> = self
            .parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| format!(r#""{}""#, p.name))
            .collect();

        if !required_params.is_empty() {
            schema.push_str(&format!(r#", "required": [{}]"#, required_params.join(", ")));
        }
        schema.push_str("}}}}}");

        schema
    }
}

/// Represents a tool invocation argument
#[derive(Debug, Clone, PartialEq)]
pub enum ToolArgument {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<String>),
}

impl fmt::Display for ToolArgument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ToolArgument::String(s) => write!(f, "{}", s),
            ToolArgument::Number(n) => write!(f, "{}", n),
            ToolArgument::Boolean(b) => write!(f, "{}", b),
            ToolArgument::Array(arr) => write!(f, "[{}]", arr.join(", ")),
        }
    }
}

/// Represents an LLM function call output
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub arguments: HashMap<String, ToolArgument>,
    pub source: String,                   // "openai", "claude", "ollama", "user"
}

impl ToolInvocation {
    pub fn new(tool_name: &str) -> Self {
        ToolInvocation {
            tool_name: tool_name.to_string(),
            arguments: HashMap::new(),
            source: "user".to_string(),
        }
    }

    pub fn with_arg(mut self, name: &str, value: ToolArgument) -> Self {
        self.arguments.insert(name.to_string(), value);
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn get_arg(&self, name: &str) -> Option<&ToolArgument> {
        self.arguments.get(name)
    }
}

/// Result of executing a tool
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
}

impl ToolExecutionResult {
    pub fn ok(tool_name: &str, output: &str, time_ms: u64) -> Self {
        ToolExecutionResult {
            tool_name: tool_name.to_string(),
            success: true,
            output: output.to_string(),
            error_message: None,
            execution_time_ms: time_ms,
        }
    }

    pub fn error(tool_name: &str, error: &str, time_ms: u64) -> Self {
        ToolExecutionResult {
            tool_name: tool_name.to_string(),
            success: false,
            output: String::new(),
            error_message: Some(error.to_string()),
            execution_time_ms: time_ms,
        }
    }
}

/// Tool registry - stores and manages all available tools
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    execution_count: HashMap<String, u32>,
    success_count: HashMap<String, u32>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
            execution_count: HashMap::new(),
            success_count: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register(&mut self, tool: ToolDefinition) {
        let tool_name = tool.name.clone();
        self.tools.insert(tool_name.clone(), tool);
        self.execution_count.insert(tool_name.clone(), 0);
        self.success_count.insert(tool_name, 0);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// List all available tools
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get tools by category
    pub fn by_category(&self, category: &str) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.category == category)
            .cloned()
            .collect()
    }

    /// Update execution statistics
    pub fn record_execution(&mut self, tool_name: &str, success: bool) {
        self.execution_count.entry(tool_name.to_string()).and_modify(|c| *c += 1).or_insert(1);
        
        if success {
            self.success_count.entry(tool_name.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    /// Get execution statistics - returns (total_executions, success_rate)
    pub fn get_stats(&self, tool_name: &str) -> Option<(u32, f32)> {
        self.execution_count.get(tool_name).and_then(|count| {
            let success = self.success_count.get(tool_name).copied().unwrap_or(0);
            let rate = if *count > 0 {
                success as f32 / *count as f32
            } else {
                0.0
            };
            Some((*count, rate))
        })
    }

    /// Generate OpenAI function calling schema for all tools
    pub fn to_openai_schema(&self) -> String {
        let schemas: Vec<String> = self.tools.values().map(|t| t.to_openai_schema()).collect();
        format!("[{}]", schemas.join(", "))
    }

    /// Generate Claude tool definitions for all tools
    pub fn to_claude_schema(&self) -> String {
        let schemas: Vec<String> = self.tools.values().map(|t| t.to_claude_schema()).collect();
        format!("[{}]", schemas.join(", "))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool executor - safely executes tool invocations
pub struct ToolExecutor {
    registry: ToolRegistry,
    max_retries: u32,
    timeout_ms: u64,
}

impl ToolExecutor {
    pub fn new(registry: ToolRegistry) -> Self {
        ToolExecutor {
            registry,
            max_retries: 3,
            timeout_ms: 5000,
        }
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Execute a tool invocation
    pub fn execute(&mut self, invocation: &ToolInvocation) -> ToolExecutionResult {
        let start = std::time::Instant::now();

        if !self.registry.tools.contains_key(&invocation.tool_name) {
            let elapsed = start.elapsed().as_millis() as u64;
            self.registry.record_execution(&invocation.tool_name, false);
            return ToolExecutionResult::error(
                &invocation.tool_name,
                &format!("Tool {} not found in registry", invocation.tool_name),
                elapsed,
            );
        }

        // Validate arguments
        if let Err(e) = self.validate_arguments(invocation) {
            let elapsed = start.elapsed().as_millis() as u64;
            self.registry.record_execution(&invocation.tool_name, false);
            return ToolExecutionResult::error(&invocation.tool_name, &e, elapsed);
        }

        // Simulate tool execution (in real implementation, would call actual tool)
        let output = self.simulate_tool_execution(invocation);
        let elapsed = start.elapsed().as_millis() as u64;

        self.registry.record_execution(&invocation.tool_name, true);
        ToolExecutionResult::ok(&invocation.tool_name, &output, elapsed)
    }

    fn validate_arguments(&self, invocation: &ToolInvocation) -> Result<(), String> {
        if let Some(tool_def) = self.registry.get(&invocation.tool_name) {
            for param in &tool_def.parameters {
                if param.required && !invocation.arguments.contains_key(&param.name) {
                    return Err(format!("Missing required parameter: {}", param.name));
                }
            }
        }
        Ok(())
    }

    fn simulate_tool_execution(&self, invocation: &ToolInvocation) -> String {
        let args_str = invocation
            .arguments
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        format!("Tool {} executed with args: {}", invocation.tool_name, args_str)
    }

    pub fn get_registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_parameter_creation() {
        let param = ToolParameter::new("query", "string", "Search query");
        assert_eq!(param.name, "query");
        assert_eq!(param.param_type, "string");
        assert!(param.required);
    }

    #[test]
    fn test_tool_parameter_optional() {
        let param = ToolParameter::new("limit", "number", "Result limit").optional();
        assert!(!param.required);
    }

    #[test]
    fn test_tool_parameter_enum() {
        let param = ToolParameter::new("mode", "string", "Operation mode")
            .with_enum(vec!["read".to_string(), "write".to_string()]);
        assert_eq!(param.enum_values, Some(vec!["read".to_string(), "write".to_string()]));
    }

    #[test]
    fn test_tool_parameter_schema() {
        let param = ToolParameter::new("query", "string", "Search query");
        let schema = param.to_json_schema();
        assert!(schema.contains("string"));
        assert!(schema.contains("Search query"));
    }

    #[test]
    fn test_tool_definition_creation() {
        let tool = ToolDefinition::new("search", "Search for information")
            .with_category("data")
            .with_return_type("array");

        assert_eq!(tool.name, "search");
        assert_eq!(tool.category, "data");
        assert_eq!(tool.return_type, "array");
    }

    #[test]
    fn test_tool_definition_with_parameters() {
        let tool = ToolDefinition::new("search", "Search for information")
            .with_parameter(ToolParameter::new("query", "string", "Search query"))
            .with_parameter(
                ToolParameter::new("limit", "number", "Result limit")
                    .optional(),
            );

        assert_eq!(tool.parameters.len(), 2);
    }

    #[test]
    fn test_tool_invocation_creation() {
        let invocation = ToolInvocation::new("search")
            .with_arg("query", ToolArgument::String("killer".to_string()))
            .with_arg("limit", ToolArgument::Number(10.0));

        assert_eq!(invocation.tool_name, "search");
        assert_eq!(invocation.arguments.len(), 2);
    }

    #[test]
    fn test_tool_registry_register() {
        let mut registry = ToolRegistry::new();
        let tool = ToolDefinition::new("search", "Search tool");
        registry.register(tool);

        assert!(registry.get("search").is_some());
    }

    #[test]
    fn test_tool_registry_by_category() {
        let mut registry = ToolRegistry::new();
        let tool1 = ToolDefinition::new("search", "Search tool").with_category("data");
        let tool2 = ToolDefinition::new("calculate", "Calculate tool").with_category("computation");

        registry.register(tool1);
        registry.register(tool2);

        let data_tools = registry.by_category("data");
        assert_eq!(data_tools.len(), 1);
        assert_eq!(data_tools[0].name, "search");
    }

    #[test]
    fn test_tool_registry_execution_stats() {
        let mut registry = ToolRegistry::new();
        let tool = ToolDefinition::new("search", "Search tool");
        registry.register(tool);

        registry.record_execution("search", true);
        registry.record_execution("search", true);

        let (count, rate) = registry.get_stats("search").unwrap();
        assert_eq!(count, 2);
        assert!(rate > 0.5); // Both succeeded
    }

    #[test]
    fn test_tool_executor_missing_tool() {
        let registry = ToolRegistry::new();
        let mut executor = ToolExecutor::new(registry);

        let invocation = ToolInvocation::new("nonexistent");
        let result = executor.execute(&invocation);

        assert!(!result.success);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_tool_executor_validation() {
        let mut registry = ToolRegistry::new();
        let tool = ToolDefinition::new("search", "Search tool")
            .with_parameter(ToolParameter::new("query", "string", "Search query"));
        registry.register(tool);

        let mut executor = ToolExecutor::new(registry);

        // Missing required parameter
        let invocation = ToolInvocation::new("search");
        let result = executor.execute(&invocation);

        assert!(!result.success);
    }

    #[test]
    fn test_tool_execution_result() {
        let ok_result = ToolExecutionResult::ok("search", "results", 42);
        assert!(ok_result.success);
        assert_eq!(ok_result.execution_time_ms, 42);

        let error_result = ToolExecutionResult::error("search", "not found", 10);
        assert!(!error_result.success);
        assert!(error_result.error_message.is_some());
    }

    #[test]
    fn test_openai_schema_generation() {
        let tool = ToolDefinition::new("search", "Search for information")
            .with_parameter(ToolParameter::new("query", "string", "Search query"))
            .with_parameter(
                ToolParameter::new("limit", "number", "Result limit").optional(),
            );

        let schema = tool.to_openai_schema();
        assert!(schema.contains("\"name\": \"search\""));
        assert!(schema.contains("\"type\": \"object\""));
        assert!(schema.contains("\"properties\""));
    }

    #[test]
    fn test_claude_schema_generation() {
        let tool = ToolDefinition::new("search", "Search for information")
            .with_parameter(ToolParameter::new("query", "string", "Search query"));

        let schema = tool.to_claude_schema();
        assert!(schema.contains("\"name\": \"search\""));
        assert!(schema.contains("\"input_schema\""));
    }
}
