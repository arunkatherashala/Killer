# KILLER v2.0: FEATURE #2 - LLM INTEGRATION
## Native Language Support for OpenAI, Claude, Ollama, Local Models

**Status**: Initial Specification (ready for implementation weeks 4-8)  
**Dependency**: Killer v1.1+  
**Enables**: NLP applications, agents, tool calling  
**Timeline**: 4-5 weeks to production  
**Tier**: Tier 1 (Foundation - enables Features #3, #6)

---

## OVERVIEW: Why LLM Integration?

Killer's unique vision: **Make LLMs first-class citizens in the language itself**, not an afterthought library.

```
KILLER PHILOSOPHY:
┌─────────────────────────────────────────┐
│  Data (SQL) + AI (LLMs) + Concurrency   │
│         = ONE LANGUAGE                  │
└─────────────────────────────────────────┘

Traditional: Python + OpenAI library (clunky)
vs.
Killer: LLM types built into language (elegant)
```

**Key Goals**:
✅ Native types: `Message`, `ChatResponse`, `EmbeddingResponse`  
✅ Multi-provider: OpenAI, Claude, Ollama, Local  
✅ Streaming: Real-time token generation  
✅ Error handling: Automatic retries, fallback models  
✅ Async-first: Works seamlessly with Feature #1

---

## ARCHITECTURE: 4-Layer Model

```
Layer 4: High-Level (What users write)
┌─────────────────────────────────────────┐
│  let response = llm.ask("What is AI?")  │
│  let agent = AIAgent::new(llm)          │
└──────────────┬──────────────────────────┘
               ▲
Layer 3: Provider Abstraction
┌──────────────┼──────────────────────────┐
│  OpenAI      │ Claude      │ Ollama     │
│  Provider    │ Provider    │ Provider   │
└──────────────┼──────────────────────────┘
               ▲
Layer 2: Protocol (HTTP, async)
┌──────────────┼──────────────────────────┐
│  HTTP POST   │ Streaming   │ Retry      │
│  Serialization│ Handling   │ Logic      │
└──────────────┼──────────────────────────┘
               ▲
Layer 1: Killer Runtime
┌──────────────┴──────────────────────────┐
│  Async/Await (#1), Actor Model          │
│  Network I/O, JSON parsing              │
└─────────────────────────────────────────┘
```

---

## PART 1: NATIVE LLM TYPES

### 1.1 Core Type Definitions

```killer
// Message: Single message in conversation
record Message {
  role: String          // "system", "user", "assistant"
  content: String       // Message text
  timestamp: Int        // Unix timestamp (optional)
}

// ChatResponse: Response from LLM
record ChatResponse {
  message: String       // Assistant's response
  model: String         // Which model was used
  tokens_used: Int      // Total tokens (prompt + completion)
  prompt_tokens: Int    // Input tokens
  completion_tokens: Int  // Output tokens
  finish_reason: String // "stop", "max_tokens", "error"
  latency_ms: Int       // Response time in milliseconds
}

// EmbeddingResponse: Vector embedding for text
record EmbeddingResponse {
  text: String
  embedding: Vector<Float>  // 384, 768, or 1536 dims depending on model
  model: String
  dimension: Int
}

// ModelConfig: Configuration for an LLM provider
record ModelConfig {
  provider: String      // "openai", "claude", "ollama", "local"
  model_name: String    // "gpt-4", "claude-3-opus", "mistral", etc.
  api_key: String       // API key (if needed)
  api_base_url: String  // Custom endpoint (for self-hosted)
  temperature: Float    // 0.0-1.0, controls randomness
  max_tokens: Int       // Max response length
  top_p: Float          // Nucleus sampling
  timeout_sec: Int      // HTTP timeout
}

// LLMError: Structured error handling
enum LLMError {
  RateLimited,
  AuthenticationFailed(reason: String),
  ModelNotFound(model: String),
  InvalidRequest(reason: String),
  ServiceUnavailable,
  TimeoutError,
  NetworkError(reason: String),
  UnknownError(reason: String)
}
```

### 1.2 LLM Client Actor

```killer
actor LLMClient {
  config: ModelConfig
  conversation_history: List<Message>
  total_tokens: Int
  
  // Initialize client with config
  handle initialize(cfg: ModelConfig) {
    this.config = cfg
    this.conversation_history = []
    this.total_tokens = 0
  }
  
  // Send single message to LLM
  handle send_message(user_message: String) -> ChatResponse | LLMError {
    // Add to history
    msg = Message {
      role: "user",
      content: user_message,
      timestamp: current_time()
    }
    this.conversation_history.push(msg)
    
    // Call provider
    result = match this.config.provider {
      "openai" -> call_openai(this.config, this.conversation_history)
      "claude" -> call_claude(this.config, this.conversation_history)
      "ollama" -> call_ollama(this.config, this.conversation_history)
      "local" -> call_local(this.config, this.conversation_history)
      _ -> return LLMError::InvalidRequest("Unknown provider")
    }
    
    match result {
      Ok(response) -> {
        // Update history
        assistant_msg = Message {
          role: "assistant",
          content: response.message,
          timestamp: current_time()
        }
        this.conversation_history.push(assistant_msg)
        this.total_tokens = this.total_tokens + response.tokens_used
        response
      }
      Err(e) -> e
    }
  }
  
  // Clear conversation history
  handle clear_history() {
    this.conversation_history = []
  }
  
  // Get full conversation
  handle get_history() -> List<Message> {
    this.conversation_history
  }
  
  // Get token usage
  handle get_token_usage() -> Int {
    this.total_tokens
  }
}

// Helper: Get current Unix timestamp
kfn current_time() -> Int {
  // In production: call system time
  0  // Placeholder
}
```

---

## PART 2: PROVIDER IMPLEMENTATIONS

### 2.1 OpenAI Provider

```killer
kfn call_openai(
  config: ModelConfig,
  messages: List<Message>
) -> ChatResponse | LLMError {
  
  // Build request
  request_body = build_openai_request(config, messages)
  
  // Make API call
  response = http_post(
    "https://api.openai.com/v1/chat/completions",
    request_body,
    map {
      "Authorization": "Bearer " + config.api_key,
      "Content-Type": "application/json"
    },
    timeout: config.timeout_sec
  )
  
  match response.status_code {
    200 -> {
      // Success
      parsed = parse_json(response.body)
      ChatResponse {
        message: parsed["choices"][0]["message"]["content"],
        model: config.model_name,
        tokens_used: parsed["usage"]["total_tokens"],
        prompt_tokens: parsed["usage"]["prompt_tokens"],
        completion_tokens: parsed["usage"]["completion_tokens"],
        finish_reason: parsed["choices"][0]["finish_reason"],
        latency_ms: response.latency_ms
      }
    }
    401 -> {
      LLMError::AuthenticationFailed("Invalid OpenAI API key")
    }
    429 -> {
      LLMError::RateLimited
    }
    503 -> {
      LLMError::ServiceUnavailable
    }
    _ -> {
      LLMError::UnknownError(response.body)
    }
  }
}

kfn build_openai_request(
  config: ModelConfig,
  messages: List<Message>
) -> String {
  // Build JSON request
  // POST body format:
  // {
  //   "model": "gpt-4",
  //   "messages": [...],
  //   "temperature": 0.7,
  //   "max_tokens": 2000
  // }
  
  // In actual implementation: use JSON builder
  "{\"model\": \"" + config.model_name + 
  "\", \"temperature\": " + config.temperature.to_string() +
  ", \"max_tokens\": " + config.max_tokens.to_string() + "}"
}
```

### 2.2 Claude (Anthropic) Provider

```killer
kfn call_claude(
  config: ModelConfig,
  messages: List<Message>
) -> ChatResponse | LLMError {
  
  // Build request for Anthropic API
  request_body = build_claude_request(config, messages)
  
  response = http_post(
    "https://api.anthropic.com/v1/messages",
    request_body,
    map {
      "x-api-key": config.api_key,
      "anthropic-version": "2023-06-01"
    },
    timeout: config.timeout_sec
  )
  
  match response.status_code {
    200 -> {
      parsed = parse_json(response.body)
      ChatResponse {
        message: parsed["content"][0]["text"],
        model: config.model_name,
        tokens_used: parsed["usage"]["input_tokens"] + 
                     parsed["usage"]["output_tokens"],
        prompt_tokens: parsed["usage"]["input_tokens"],
        completion_tokens: parsed["usage"]["output_tokens"],
        finish_reason: parsed["stop_reason"],
        latency_ms: response.latency_ms
      }
    }
    401 -> {
      LLMError::AuthenticationFailed("Invalid Claude API key")
    }
    429 -> {
      LLMError::RateLimited
    }
    _ -> {
      LLMError::UnknownError(response.body)
    }
  }
}

kfn build_claude_request(
  config: ModelConfig,
  messages: List<Message>
) -> String {
  // Claude API format: system message separate
  "{\"model\": \"" + config.model_name + "\"}"
}
```

### 2.3 Ollama (Local Models) Provider

```killer
kfn call_ollama(
  config: ModelConfig,
  messages: List<Message>
) -> ChatResponse | LLMError {
  
  api_base = config.api_base_url  // localhost:11434 typically
  request_body = build_ollama_request(config, messages)
  
  // Ollama runs locally, no auth needed
  response = http_post(
    api_base + "/api/chat",
    request_body,
    map { "Content-Type": "application/json" },
    timeout: config.timeout_sec
  )
  
  match response.status_code {
    200 -> {
      parsed = parse_json(response.body)
      ChatResponse {
        message: parsed["message"]["content"],
        model: config.model_name,
        tokens_used: parsed["eval_count"],
        prompt_tokens: parsed["prompt_eval_count"],
        completion_tokens: parsed["eval_count"],
        finish_reason: "stop",
        latency_ms: response.latency_ms
      }
    }
    404 -> {
      LLMError::ModelNotFound(config.model_name)
    }
    _ -> {
      LLMError::ServiceUnavailable
    }
  }
}

kfn build_ollama_request(
  config: ModelConfig,
  messages: List<Message>
) -> String {
  "{\"model\": \"" + config.model_name + "\"}"
}
```

---

## PART 3: FEATURE #1 INTEGRATION (ASYNC/AWAIT)

### 3.1 Concurrent Message Processing

```killer
actor AsyncLLMClient {
  base_client: LLMClient
  batch_size: Int
  
  handle initialize(client: LLMClient, batch: Int) {
    this.base_client = client
    this.batch_size = batch
  }
  
  // Process multiple messages concurrently
  handle process_messages_async(
    messages: List<String>
  ) -> List<ChatResponse> {
    
    tasks = []
    
    i = 0
    loop {
      if i >= messages.len() { break }
      
      msg = messages[i]
      task = spawn_task {
        await this.base_client.send_message(msg)
      }
      tasks.push(task)
      
      i = i + 1
    }
    
    // Wait for all tasks
    results = await join_all(tasks)
    results
  }
  
  // Process messages with sliding window
  // (Feature #1 enables this)
  handle process_streaming_messages(
    message_stream: (String) -> String
  ) {
    tasks = []
    
    loop {
      msg = message_stream()
      if msg.len() == 0 { break }
      
      task = spawn_task {
        await this.base_client.send_message(msg)
      }
      tasks.push(task)
      
      // Keep queue bounded to batch_size
      if tasks.len() >= this.batch_size {
        completed = await select_first_async(tasks)
        // Remove completed task, continue
      }
    }
  }
}
```

### 3.2 Parallel Inference (Multiple Models)

```killer
actor EnsembleLLMInference {
  models: List<LLMClient>
  
  handle initialize(model_configs: List<ModelConfig>) {
    this.models = []
    i = 0
    loop {
      if i >= model_configs.len() { break }
      client = LLMClient::spawn()
      await client.initialize(model_configs[i])
      this.models.push(client)
      i = i + 1
    }
  }
  
  // Get response from all models concurrently
  handle ensemble_response(user_message: String) -> List<ChatResponse> {
    tasks = []
    
    i = 0
    loop {
      if i >= this.models.len() { break }
      
      model = this.models[i]
      task = spawn_task {
        await model.send_message(user_message)
      }
      tasks.push(task)
      
      i = i + 1
    }
    
    results = await join_all(tasks)
    results
  }
  
  // Vote/consensus from ensemble
  handle ensemble_consensus(user_message: String) -> String {
    responses = await this.ensemble_response(user_message)
    
    // In production: implement majority voting or semantic similarity
    // For now: return first response
    responses[0].message
  }
}
```

---

## PART 4: ERROR HANDLING & RETRIES

### 4.1 Automatic Retry Logic

```killer
actor ResilientLLMClient {
  base_client: LLMClient
  max_retries: Int
  backoff_base: Int  // milliseconds
  
  handle initialize(client: LLMClient, retries: Int) {
    this.base_client = client
    this.max_retries = retries
    this.backoff_base = 1000  // 1 second
  }
  
  // Send message with automatic retries
  handle send_message_with_retry(
    user_message: String
  ) -> ChatResponse | LLMError {
    
    attempt = 0
    loop {
      if attempt >= this.max_retries {
        return LLMError::ServiceUnavailable
      }
      
      result = await this.base_client.send_message(user_message)
      
      match result {
        // Success
        ChatResponse -> return result
        
        // Retryable errors
        LLMError::RateLimited -> {
          wait_time = this.backoff_base * (2 ^ attempt)
          sleep(wait_time)
          attempt = attempt + 1
        }
        
        LLMError::ServiceUnavailable -> {
          wait_time = this.backoff_base * (2 ^ attempt)
          sleep(wait_time)
          attempt = attempt + 1
        }
        
        // Non-retryable errors
        _ -> return result
      }
    }
  }
}
```

### 4.2 Fallback Chain

```killer
actor FallbackLLMProvider {
  providers: List<LLMClient>
  fallback_index: Int
  
  handle initialize(configs: List<ModelConfig>) {
    this.providers = []
    i = 0
    loop {
      if i >= configs.len() { break }
      client = LLMClient::spawn()
      await client.initialize(configs[i])
      this.providers.push(client)
      i = i + 1
    }
    this.fallback_index = 0
  }
  
  // Try providers in order until one works
  handle send_with_fallback(
    user_message: String
  ) -> ChatResponse | LLMError {
    
    i = this.fallback_index
    loop {
      if i >= this.providers.len() { break }
      
      result = await this.providers[i].send_message(user_message)
      
      match result {
        ChatResponse -> {
          this.fallback_index = i  // Remember successful provider
          return result
        }
        _ -> {
          i = i + 1
        }
      }
    }
    
    LLMError::ServiceUnavailable
  }
}
```

---

## PART 5: USAGE EXAMPLES

### 5.1 Simple Chat

```killer
kfn main() {
  println("=== KILLER LLM INTEGRATION (Feature #2) ===")
  println("")
  
  // Configuration
  config = ModelConfig {
    provider: "openai",
    model_name: "gpt-4",
    api_key: "sk-...",
    api_base_url: "https://api.openai.com",
    temperature: 0.7,
    max_tokens: 2000,
    top_p: 0.9,
    timeout_sec: 30
  }
  
  // Create client
  client = LLMClient::spawn()
  await client.initialize(config)
  
  // Send message
  println("User: What is machine learning?")
  response = await client.send_message("What is machine learning?")
  
  match response {
    ChatResponse -> {
      println("Assistant: " + response.message)
      println("Tokens used: " + response.tokens_used.to_string())
      println("Latency: " + response.latency_ms.to_string() + "ms")
    }
    LLMError -> {
      println("Error: " + response.to_string())
    }
  }
}
```

### 5.2 Concurrent Queries (Feature #1 Integration)

```killer
kfn concurrent_queries_example() {
  config = ModelConfig {
    provider: "openai",
    model_name: "gpt-4",
    api_key: "sk-...",
    temperature: 0.7,
    max_tokens: 2000,
    timeout_sec: 30
  }
  
  client = LLMClient::spawn()
  await client.initialize(config)
  
  async_client = AsyncLLMClient::spawn()
  await async_client.initialize(client, 10)
  
  // Send 5 messages concurrently
  queries = [
    "What is AI?",
    "What is ML?",
    "What is DL?",
    "What is NLP?",
    "What is CV?"
  ]
  
  println("Sending 5 concurrent queries...")
  start_time = current_time()
  
  responses = await async_client.process_messages_async(queries)
  
  end_time = current_time()
  total_time = end_time - start_time
  
  println("Total time: " + total_time.to_string() + "ms")
  println("Responses received: " + responses.len().to_string())
  println("Speedup: ~5x from parallelism")
}
```

---

## PERFORMANCE TARGETS

| Metric | Target | Achieved |
|--------|--------|----------|
| Single message latency | <200ms | N/A (waiting) |
| Concurrent 10 messages | <250ms (vs 2s sequential) | N/A (waiting) |
| Retry overhead | <5% | N/A (waiting) |
| Error detection | <50ms | N/A (waiting) |
| Memory per client | <5 MB | N/A (waiting) |

**First deployed**: Week 8, 2026  
**Production ready**: Week 8, 2026  
**Market ready**: Week 26, 2026 (v2.0 launch)

---

## NEXT STEPS

✅ **Week 4**: Specification complete (THIS DOCUMENT)  
→ **Week 5**: Implement OpenAI provider + basic client  
→ **Week 6**: Add Claude, Ollama providers + retry logic  
→ **Week 7**: Async integration + ensemble inference  
→ **Week 8**: Performance optimization + production polish  

**Then**: Build Example #2 (NLP Pipeline) using Feature #2! 🚀
