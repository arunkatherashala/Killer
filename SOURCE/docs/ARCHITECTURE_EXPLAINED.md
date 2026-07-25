# KILLER AI AGENTS - ARCHITECTURE CRASH COURSE
## Everything You Need to Know (30 minutes)

**Read this tonight** to understand what we're building

---

## THE GOAL (In Plain English)

Right now, Killer has:
- ✅ Actor model (message passing)
- ✅ Synchronous handlers (blocking)
- ❌ No way to call APIs without blocking
- ❌ No async/await

We're adding:
- ✅ Async handlers (non-blocking)
- ✅ HTTP client (GET/POST)
- ✅ LLM integration (call OpenAI natively)
- ✅ Await expression support

**Result** = Agents can ask questions to LLM while other actors keep working

---

## LAYER 1: CURRENT STATE (What Exists)

### Actor Model (killer_rcore v4.0)
```
Actor 1              Actor 2
  ┌─────┐               ┌──────┐
  │ Ask │──Message──→   │ Send │
  │     │←──Response─── │Response
  └─────┘               └──────┘
```

**How it works**:
1. Send message to actor mailbox
2. Actor receives it
3. Handler executes (BLOCKING - waits for response)
4. Actor sends response back
5. Caller continues

**Problem**: If handler calls HTTP or LLM, everything blocks waiting

---

## LAYER 2: WHAT WE'RE ADDING

### HTTP Client (Simple)
```rust
pub fn http_get(url: &str) -> Result<String> {
    // Connect to server
    let mut stream = TcpStream::connect(host)?;
    
    // Send GET request
    stream.write_all(request.as_bytes())?;
    
    // Read response
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    
    Ok(response)
}
```

**Usage in Killer**:
```killer
kfn main() {
  response = http_get("https://api.github.com/zen")
  println(response)
}
```

---

### LLM Integration (Wrapper)
```rust
pub fn llm_complete(prompt: &str, api_key: &str) -> Result<String> {
    // 1. Build JSON body
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
    });
    
    // 2. Call OpenAI API (uses http_post internally)
    let response = http_post("https://api.openai.com/v1/chat/completions", 
                            &body, api_key)?;
    
    // 3. Parse JSON response
    let assistant_msg = parse_response(response)?;
    
    Ok(assistant_msg)
}
```

**Usage in Killer**:
```killer
kfn main() {
  api_key = env("OPENAI_API_KEY")
  response = llm_complete("Hello, what is Killer?", api_key)
  println(response)
}
```

**Still blocking, but that's OK for now.**

---

### Async/Await (Latest Addition)
```killer
// WITHOUT async: blocks everything
kfn ask(q: String) -> String {
  response = llm_complete(q, api_key)  // WAITS HERE
  return response
}

// WITH async: doesn't block
async kfn ask(q: String) -> String {
  response = await llm_complete(q, api_key)  // Suspends, resumes later
  return response
}
```

**How async works**:

1. **Async function definition**
   ```killer
   async kfn ask(q: String) -> String {
     // ...
   }
   ```
   Parser sees `async` keyword → marks function as AsyncFunction

2. **Await expression**
   ```killer
   response = await llm_complete(q, api_key)
   ```
   - Calls function
   - Gets a Future
   - Awaits: "suspend me until this future completes"
   - Runtime polls future
   - When complete, resumes with result

3. **Event loop (runtime)**
   ```
   Loop {
     For each task in ready_queue:
       Run task until it hits await
       (task suspends, goes to waiting_queue)
     
     Check waiting_queue:
       Are any futures ready?
       If yes, move to ready_queue
     
     If nothing ready, exit
   }
   ```

---

## LAYER 3: HOW IT ALL CONNECTS

### The Flow
```
User Code (Killer language)
    ↓
Parser: async fn, await
    ↓
Evaluator: Execute async handler
    ↓
Async Runtime: Event loop polls futures
    ↓
LLM Client: Calls HTTP
    ↓
HTTP Client: TCP connection to OpenAI
    ↓
OpenAI API: Returns response
```

### Example: Full Flow
```killer
// Killer code
async kfn research(topic: String) -> String {
  q = "Research " + topic
  response = await llm_complete(q, api_key)
  return response
}

kfn main() {
  answer = research("Killer language").await
  println(answer)
}
```

**Step by step**:
1. `main()` calls `research("Killer language")`
2. `research` is async → returns Future
3. `main` hits `await` → "suspend until future completes"
4. Runtime starts event loop
5. Event loop runs `research` function
6. `research` calls `llm_complete` → hits await
7. `llm_complete` calls `http_post` (blocking)
8. HTTP connects to OpenAI
9. Response comes back
10. Future resolves
11. Event loop resumes `research`
12. `research` returns
13. Event loop resumes `main`
14. `main` prints answer

---

## LAYER 4: IMPLEMENTATION STRATEGY

### Week 1 Tasks (Ultra-Minimal)

#### Day 1 (Monday): Setup
- [ ] Read existing code: actor.rs, message.rs, handler.rs
- [ ] Understand message dispatch
- [ ] Write ARCHITECTURE.md (document findings)

#### Day 2 (Tuesday): HTTP Client
- [ ] Create `src/stdlib/http_client.rs`
- [ ] Implement `http_get()` - simple TCP connection
- [ ] Add to Killer: `http_get(url) -> String`
- [ ] Test: Real HTTP GET to GitHub API

#### Day 3 (Wednesday): LLM Integration
- [ ] Create `src/stdlib/llm_client.rs`
- [ ] Implement `llm_complete(prompt, api_key) -> String`
- [ ] Use http_post to call OpenAI
- [ ] Test: Real API call to GPT-3.5-turbo

#### Day 4 (Thursday): Async Support
- [ ] Parser: Add `async` keyword support
- [ ] Evaluator: Handle async function calls
- [ ] Runtime: Add await expression support
- [ ] Test: `async fn` + `await` syntax works

#### Day 5 (Friday): Demo
- [ ] Create example agent that uses all three
- [ ] Test end-to-end
- [ ] Document what works

---

## KEY FILES TO CREATE

### 1. HTTP Client: `src/stdlib/http_client.rs`
```rust
use std::net::TcpStream;
use std::io::{Read, Write};

pub fn http_get(url: &str) -> Result<String, String> {
    // Implementation: parse URL, connect, send GET, read response
}

pub fn http_post(url: &str, body: &str, api_key: &str) -> Result<String, String> {
    // Implementation: connect, send POST with auth header, read response
}
```

### 2. LLM Client: `src/stdlib/llm_client.rs`
```rust
use serde_json::{json, Value};
use crate::stdlib::http_client;

pub fn llm_complete(prompt: &str, api_key: &str) -> Result<String, String> {
    // Build JSON
    // Call http_post
    // Parse response
    // Extract message
}
```

### 3. Async Runtime: `src/runtime/async_runtime.rs`
```rust
pub struct AsyncRuntime {
    ready_queue: VecDeque<TaskId>,
    waiting_tasks: HashMap<TaskId, WaitingTask>,
}

pub fn run_forever(&mut self) {
    // Event loop: poll tasks, run ready ones
}
```

---

## WHAT WE'RE NOT DOING (YET)

❌ Full async/await like Tokio (too complex)  
❌ Streaming responses (can add later)  
❌ Multiple LLM providers (just OpenAI for now)  
❌ Tool calling (can add week 2)  
❌ Distributed agents (can add week 3)  
❌ GPU acceleration (can add week 4+)

**Just focus on**: HTTP → LLM → Async. One thing at a time.

---

## SUCCESS = THIS WORKS

```killer
async kfn ask(q: String) -> String {
  api_key = env("OPENAI_API_KEY")
  response = await llm_complete(q, api_key)
  return response
}

kfn main() {
  answer = ask("What is Killer?").await
  println(answer)  // Actually prints LLM response
}
```

By Friday, you'll run this and get a real answer from ChatGPT.

---

## READY TO BUILD?

Next step: `WEEK1_SOLO_EXECUTION.md` has your Monday tasks.

But if you're starting TONIGHT:

**Option A (Light)**: Read existing code in `src/runtime/` for 1-2 hours  
**Option B (Standard)**: Start with Monday's setup tasks  
**Option C (Full)**: Jump to Tuesday's HTTP client implementation  

What do you want to do?
