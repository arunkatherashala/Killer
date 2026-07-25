# LLM INTEGRATION SPECIFICATION
## How Killer Talks to AI

**Design Date**: March 17, 2026  
**Implementation**: This week (Wednesday)  

---

## SIMPLE API

### Basic Call
```killer
fn main() {
  api_key = env("OPENAI_API_KEY")
  response = llm_complete("Hello, what is Killer?", api_key)
  println(response)
}
```

### Async Version
```killer
async fn ask_llm(q: String) -> String {
  api_key = env("OPENAI_API_KEY")
  response = await llm_complete_async(q, api_key)
  return response
}

fn main() {
  answer = ask_llm("What is Killer?").await
  println(answer)
}
```

---

## IMPLEMENTATION

### HTTP POST to OpenAI
```rust
pub fn llm_complete(prompt: &str, api_key: &str) -> Result<String, String> {
    // 1. Build JSON body
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 1024,
    }).to_string();
    
    // 2. POST to OpenAI
    let response = http_post(
        "https://api.openai.com/v1/chat/completions",
        &body,
        api_key,
    )?;
    
    // 3. Parse JSON response
    let json: Value = serde_json::from_str(&response)?;
    
    // 4. Extract assistant message
    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response")
        .to_string())
}
```

---

## SUPPORTED PROVIDERS

- **OpenAI**: gpt-3.5-turbo, gpt-4
- **Claude**: claude-3-opus, claude-3-sonnet
- **Ollama**: Local (llama2, mistral, etc.)

---

## KEY FILES TO CREATE

1. `src/stdlib/http_client.rs` - HTTP GET/POST
2. `src/stdlib/llm_client.rs` - LLM API wrapper
3. `examples/llm_hello.killer` - Demo

---

## SUCCESS CRITERIA

✅ Can call OpenAI API  
✅ Gets real response back  
✅ Parses JSON correctly  
✅ Returns assistant message  
✅ <3 second response time  

---

## REFERENCE

See `WEEK1_SOLO_EXECUTION.md` for step-by-step instructions
