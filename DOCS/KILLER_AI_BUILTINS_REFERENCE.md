# Killer Language — AI / LLM / RLM / KhLM Builtins Reference

**Version**: v2.1 Native Binary  
**Date**: 2026-03-24  
**Binary**: `SOURCE/src/v2-rust/killer/target/release/killer-native.exe`

---

## Overview

Killer has **14 AI builtins** built directly into the language runtime — no imports, no pip install, no API keys for local models. They fall into 5 groups:

| Group | Builtins | What it does |
|-------|----------|-------------|
| **LLM** | `llm_chat`, `llm_ask`, `llm_info`, `llm_reason`, `llm_reason_answer` | Local GGUF model inference |
| **RLM** | `rlm_think`, `rlm_answer`, `rlm_thinking` | Reasoning models (DeepSeek-R1, QwQ) |
| **KhLM** | `khlm_ask`, `khlm_ask_model`, `khlm_prefetch` | Smart hybrid router |
| **Web** | `ghost_ask`, `ghost_108`, `native_think` | Web search + deterministic engine |
| **Compose** | `khlm_classify`, `khlm_run`, `llm_parallel`, `rlm_synthesize` | Build your own AI systems |

All local models use the **GGUF format** (`.gguf`). Download from Hugging Face — any `Q4_K_M` or `Q8_0` quantization works.

---

## 1. LLM Builtins

### `llm_chat(model_path, question)`
### `llm_chat(model_path, question, max_tokens)`

Sends a question through the model's **native chat template** (auto-detected from GGUF metadata: ChatML for Qwen2, LLaMA-3 format, Phi-3, Gemma, Mistral, etc.).

**Parameters:**
- `model_path` — path to `.gguf` file (relative to binary, or absolute)
- `question` — user question string
- `max_tokens` — optional, default `512`

**Returns:** `String` — model's response

```killer
model = "models/qwen2.5-7b-instruct-q4_k_m.gguf"
answer = llm_chat(model, "What is the capital of France?")
print(answer)

// With token limit
answer = llm_chat(model, "Summarize quantum computing in 2 sentences", 150)
```

**Supported architectures:** LLaMA, LLaMA2, LLaMA3, Qwen2, Qwen2.5, Phi-3, Gemma, Mistral, DeepSeek, TinyLlama

---

### `llm_ask(model_path, raw_prompt)`
### `llm_ask(model_path, raw_prompt, max_tokens)`

Sends the prompt **directly** with no chat template wrapping. Use when you want full control over the prompt format.

```killer
model = "models/tinyllama-1.1b-chat-q4_k_m.gguf"

// Custom prompt — no template added
raw = "<|system|>You are a Haiku poet.</s><|user|>Write a haiku about code.</s><|assistant|>"
poem = llm_ask(model, raw, 100)
print(poem)
```

---

### `llm_info(model_path)`

Returns a human-readable summary of the model.

```killer
info = llm_info("models/deepseek-r1-7b-q4_k_m.gguf")
print(info)
// Output:
// Model: DeepSeek-R1-Distill-Qwen-7B-Q4_K_M
// Architecture: qwen2 | Layers: 28 | Heads: 28 | KV-heads: 4
// Embedding: 3584 | FFN: 18944 | Vocab: 152064
// Context: 131072 | rope_theta: 1,000,000
// Quantization: Q4_K_M | Parameters: ~7B
```

---

### `llm_reason(model_path, question)`
### `llm_reason(model_path, question, max_tokens)`

Turns **any standard LLM** into a pseudo-reasoning model using a chain-of-thought system prompt. Works with Qwen, TinyLlama, Mistral, Llama — any GGUF model. No DeepSeek-R1 needed.

Returns **full display**: thinking trace + final answer.

```killer
model = "models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
result = llm_reason(model, "What is 17 * 23?", 512)
print(result)
// Shows <think>...</think> reasoning then the answer
```

---

### `llm_reason_answer(model_path, question)`
### `llm_reason_answer(model_path, question, max_tokens)`

Same as `llm_reason` but returns **only the final answer** — the thinking trace is hidden.

```killer
answer = llm_reason_answer(model, "Is 997 a prime number?", 256)
print(answer)
// Output: "Yes, 997 is a prime number."
```

---

## 2. RLM Builtins (Reasoning Language Models)

RLMs are models trained with reinforcement learning to reason inside `<think>...</think>` tags before answering. **DeepSeek-R1** and **QwQ-32B** are the primary RLMs.

### `rlm_think(model_path, question)`
### `rlm_think(model_path, question, max_tokens)`

Runs the model with the DeepSeek-R1 / QwQ **reasoning template** — prefills `<think>` so the model starts its chain-of-thought immediately. Returns the **full formatted output**: thinking trace + answer.

**Parameters:**
- `model_path` — path to a DeepSeek-R1 or QwQ GGUF file
- `question` — question to reason about
- `max_tokens` — default `1024` (reasoning models need more tokens)

**Returns:** `String` — formatted thinking trace + answer

```killer
model = "C:/models/DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf"

result = rlm_think(model, "Solve: 2x + 5 = 13", 800)
print(result)
// ┌── Think ──────────────────────
// │ <think>
// │ Let me solve for x...
// │ 2x = 13 - 5 = 8
// │ x = 4
// │ </think>
// └───────────────────────────────
// Answer: x = 4
```

**Compatible models:**
- `DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf`
- `DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf`
- `DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf`
- `QwQ-32B-Q4_K_M.gguf`
- Any model with `deepseek`, `r1`, or `qwq` in the name

---

### `rlm_answer(model_path, question)`
### `rlm_answer(model_path, question, max_tokens)`

Like `rlm_think` but returns **only the final answer** — the `<think>` chain is stripped.

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"
answer = rlm_answer(model, "Prove that sqrt(2) is irrational", 2048)
print(answer)
// Output: "Proof by contradiction: assume sqrt(2) = p/q..."
```

---

### `rlm_thinking(model_path, question)`
### `rlm_thinking(model_path, question, max_tokens)`

Returns **only the thinking trace** — everything inside `<think>...</think>`. Useful for debugging the model's reasoning process or educational purposes.

```killer
trace = rlm_thinking(model, "Why is the sky blue?", 600)
print(K"Thinking process:\n{trace}")
```

---

## 3. KhLM Builtins (Killer Hybrid Learning Model)

KhLM is Killer's **self-thinking intelligent router**. It automatically decides the best engine for each question:

```
Question → Tier 1 (math/units, 0ms) ──→ HIT? return instantly
         → Tier 1.5 (classify: reasoning?) → RLM directly (skip web)  
         → Tier 2 (18 web agents parallel, ~400ms)
         → Tier 3 (local model, races Tier 2)
         → RLM Synthesis (merges fragments if no strong match)
```

### `khlm_ask(question)`

Unified smart query — **no model required**. Routes question to best engine automatically.

**Tier 1** handles: arithmetic, percentages, unit conversions, speed/distance/time  
**Tier 2** handles: factual questions, people, companies, news, prices

```killer
// Tier 1 — instant, no network
print(khlm_ask("What is 15% of 480?"))
// → 72

print(khlm_ask("Convert 100km to miles"))
// → 62.14 miles

// Tier 2 — web search via 18 parallel agents
print(khlm_ask("Who is Alan Turing?"))
// → ┌── KhLM ──────────────────
//   │  KhLM/Wikipedia  [★★★ 100%]  ⚡ 312ms
//   └──────────────────────────
//   Alan Turing was a British mathematician and computer scientist...
```

---

### `khlm_ask_model(model_path, question)`

KhLM with **Tier 3 Neural** engine added. For factual questions, fires web agents AND local model in parallel — fastest quality result wins. For reasoning questions, skips web entirely and uses the RLM.

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

// Reasoning question → skips web, goes straight to RLM
result = khlm_ask_model(model, "Explain the time complexity of merge sort")

// Factual question → web + model race
result = khlm_ask_model(model, "Who founded SpaceX?")
```

---

### `khlm_prefetch(question)`

**Fire-and-forget** background prefetch. Call at program start — by the time `khlm_ask()` runs, the answer is already cached → nanosecond return.

```killer
// At program start — fires 18 agents in background
khlm_prefetch("Who is Sai Arun Kumar Katherashala?")

// ... do other work for 1-2 seconds ...

// Now instant — cache hit
answer = khlm_ask("Who is Sai Arun Kumar Katherashala?")
print(answer)
```

---

## 4. Web / Search Builtins

### `ghost_ask(model_path, question)`
### `ghost_ask(model_path, question, max_tokens)`

Web-grounded LLM answer:
1. Math detected → compute natively (exact)
2. DuckDuckGo instant answer
3. Wikipedia fallback
4. Injects facts as context into the LLM prompt
5. Asks local model with grounded context

```killer
model = "models/tinyllama-1.1b-q4_k_m.gguf"
answer = ghost_ask(model, "What is the boiling point of water at high altitude?")
```

---

### `ghost_108(question)`

Launches **18 parallel search agents** simultaneously — fastest quality result wins. No model required. Pure web search.

Agents: Tofler-MCA, Zaubacorp, IndiaFilings, DDG, Yahoo, Wikipedia, Google HTML, GitHub API, Google News RSS, Economic Times, LinkedIn, Bing, OpenCorporates, CompanySeekers, MCA-Gov, UK Companies House, Crunchbase, GlobalRegistry

```killer
result = ghost_108("Who is Elon Musk?")
result = ghost_108("Deepthi Sudha Katherasala")  // India MCA director lookup
result = ghost_108("latest AI news 2026")
```

---

### `native_think(question)`

Killer's own 100% deterministic reasoning engine — **no model, no network, exact answers**.

Handles:
- Arithmetic: `"What is 2847 * 193?"`
- Unit conversions: `"How many km in 50 miles?"`
- Temperature: `"What is 98.6°F in Celsius?"`
- Speed/distance/time: `"How long to drive 300km at 90km/h?"`
- Percentage: `"What is 23% of 1500?"`

```killer
print(native_think("What is 15% of 480?"))        // → 72
print(native_think("100 kilometers in miles"))     // → 62.14 miles
print(native_think("How long to drive 250km at 100km/h?"))  // → 2 hours 30 minutes
```

---

## 5. Compose Builtins — Build Your Own AI System

These builtins expose KhLM's internals so you can compose **any AI pipeline** in Killer code.

### `khlm_classify(question)` → `String`

Returns question type — the brain of the smart router.

| Return value | Meaning | Best engine |
|---|---|---|
| `"math"` | Arithmetic, percentages, unit conversions | `native_think()` |
| `"factual"` | Who/what/when/where, people, companies | `ghost_108()` |
| `"reasoning"` | Explain/prove/implement/analyze/why/how | `rlm_answer()` |

```killer
kind = khlm_classify("Explain how quicksort works")
// kind == "reasoning"

kind = khlm_classify("Who is Alan Turing?")
// kind == "factual"

kind = khlm_classify("What is 15% of 480?")
// kind == "math"
```

---

### `khlm_run(model_path, question, pipeline)` → `String`

Run a **named pipeline** — full control over what engines fire.

| Pipeline | What happens |
|---|---|
| `"rlm"` | Pure RLM reasoning — no web at all |
| `"web"` | Ghost-108 web search — no model |
| `"web+rlm"` | Web search first, then RLM synthesizes results |
| `"rlm+web"` | RLM reasons, web fills in facts |
| `"auto"` | Full smart KhLM router (same as `khlm_ask_model`) |

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

// Only RLM — fast, no web latency
result = khlm_run(model, "Prove that sqrt(2) is irrational", "rlm")

// Web search only — no model needed
result = khlm_run(model, "Tesla stock price today", "web")

// Web context + RLM synthesis — best of both
result = khlm_run(model, "Who really founded Tesla and what happened?", "web+rlm")

// Full auto-routing
result = khlm_run(model, "What is 2+2?", "auto")
```

---

### `llm_parallel(model_path, questions, max_tokens)` → `Array`

Run **many questions in parallel** — all fired simultaneously in separate threads. Returns answers in the same order as the input list. Essential for multi-agent pipelines.

```killer
model = "models/qwen2.5-3b-instruct-q4_k_m.gguf"

questions = [
  "Explain BFS",
  "Explain DFS",
  "Explain Dijkstra's algorithm",
  "Compare BFS vs DFS vs Dijkstra"
]

answers = llm_parallel(model, questions, 300)

for i in 0..4 {
  print(K"--- Q{i+1} ---")
  print(answers[i])
}
```

**Performance:** 4 questions in parallel ≈ same time as 1 question sequentially.

---

### `rlm_synthesize(model_path, question, context)` → `String`

Give the RLM **your own context** and it reasons over it to produce a clean answer. This is the foundation for building **RAG (Retrieval-Augmented Generation)** pipelines.

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

// Gather context any way you want
web1 = ghost_108("Tesla founders history")
web2 = ghost_108("Elon Musk Tesla acquisition")
notes = "Tesla was founded in 2003 by Martin Eberhard and Marc Tarpenning."

context = web1 + "\n" + web2 + "\n" + notes

// RLM synthesizes one coherent answer from all fragments
answer = rlm_synthesize(model, "Who really founded Tesla?", context)
print(answer)
```

---

## Building a Complete Custom KhLM

With the compose builtins, users can implement their own full hybrid AI system in pure Killer code:

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

// Your custom AI assistant with full routing control
kfn my_ai(question) {
  kind = khlm_classify(question)

  if kind == "math" {
    // Exact — no model needed
    native_think(question)

  } elif kind == "reasoning" {
    // Deep thinking — RLM only, no web delay
    rlm_answer(model, question, 1200)

  } else {
    // Factual — get web context, synthesize with RLM
    web_context = ghost_108(question)
    
    if len(web_context) > 50 {
      rlm_synthesize(model, question, web_context)
    } else {
      khlm_ask(question)
    }
  }
}

// Usage
print(my_ai("What is 15% of 480?"))
print(my_ai("Explain binary search trees"))
print(my_ai("Who is Alan Turing?"))
```

### Custom Multi-Agent Debate System

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

kfn debate(topic, rounds) {
  khlm_prefetch(topic)  // Warm cache in background

  // Run N perspectives in parallel
  questions = [
    K"Argue FOR: {topic}",
    K"Argue AGAINST: {topic}",
    K"Find edge cases and flaws in: {topic}",
    K"Synthesize a balanced view of: {topic}"
  ]

  perspectives = llm_parallel(model, questions, 600)

  // RLM synthesizes all perspectives into final verdict
  context = join(perspectives, "\n\n---\n\n")
  rlm_synthesize(model, K"Final verdict on: {topic}", context)
}

verdict = debate("Is recursion better than iteration?", 4)
print(verdict)
```

### Custom RAG Pipeline

```killer
model = "models/deepseek-r1-7b-q4_k_m.gguf"

kfn rag_answer(question) {
  // Step 1: Classify
  kind = khlm_classify(question)

  // Step 2: Gather context with multiple sources in parallel
  sub_questions = [
    question,
    K"background context for: {question}",
    K"recent news about: {question}"
  ]
  web_results = []
  for q in sub_questions {
    web_results.push(ghost_108(q))
  }

  // Step 3: Synthesize with RLM
  context = join(web_results, "\n\n")
  rlm_synthesize(model, question, context)
}
```

---

## Quick Reference Table

| Builtin | Args | Returns | Network | Model |
|---------|------|---------|---------|-------|
| `llm_chat(m, q)` | model + question | String | No | Yes — any GGUF |
| `llm_ask(m, p)` | model + raw prompt | String | No | Yes — any GGUF |
| `llm_info(m)` | model | String | No | Yes — any GGUF |
| `llm_reason(m, q)` | model + question | String | No | Yes — any LLM |
| `llm_reason_answer(m, q)` | model + question | String | No | Yes — any LLM |
| `rlm_think(m, q)` | model + question | String | No | Yes — R1/QwQ only |
| `rlm_answer(m, q)` | model + question | String | No | Yes — R1/QwQ only |
| `rlm_thinking(m, q)` | model + question | String | No | Yes — R1/QwQ only |
| `khlm_ask(q)` | question | String | Yes | No |
| `khlm_ask_model(m, q)` | model + question | String | Yes | Yes — any GGUF |
| `khlm_prefetch(q)` | question | Nil | Yes (background) | No |
| `ghost_ask(m, q)` | model + question | String | Yes | Yes — any GGUF |
| `ghost_108(q)` | question | String | Yes | No |
| `native_think(q)` | question | String | No | No — pure Killer |
| `khlm_classify(q)` | question | String | No | No |
| `khlm_run(m, q, pipe)` | model + question + pipeline | String | Depends on pipeline | Yes — any GGUF |
| `llm_parallel(m, qs, n)` | model + Array + tokens | Array | No | Yes — any GGUF |
| `rlm_synthesize(m, q, ctx)` | model + question + context | String | No | Yes — any GGUF |

---

## Model Recommendations

| Purpose | Recommended Model | Size |
|---------|------------------|------|
| Fast LLM chat | `Qwen2.5-0.5B-Instruct-Q8_0.gguf` | 530MB |
| Balanced LLM | `Qwen2.5-3B-Instruct-Q4_K_M.gguf` | 2.0GB |
| Best LLM | `Qwen2.5-7B-Instruct-Q4_K_M.gguf` | 4.4GB |
| RLM reasoning | `DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf` | 4.4GB |
| Small RLM | `DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf` | 1.8GB |
| Best RLM | `QwQ-32B-Q4_K_M.gguf` | 20GB |

---

## Implementation Notes

### How the RLM template works (DeepSeek-R1)
```
<|im_start|>user
{question}<|im_end|>
<|im_start|>assistant
<think>
```
No system prompt by default — R1-Distill 7B was fine-tuned without system prompts. Including one causes confusing reasoning. Pass `rlm_think_with_system()` if you need domain-specific instructions.

### How KhLM classifies questions
30 keyword signatures split via two lists:
- **Reasoning keywords**: `"why "`, `"how does"`, `"explain"`, `"prove"`, `"implement"`, `"write a"`, `"step by step"`, `"analyze"`, `"difference between"`, ...  
- **Factual keywords**: `"who is"`, `"when was"`, `"ceo of"`, `"din "`, `"company"`, `"founded in"`, `"born in"`, ...
- Factual wins if present; reasoning wins otherwise; default = factual

### How `is_rlm_model()` works
Checks model path for: `deepseek`, `-r1`, `_r1`, `qwq`, `skywork-or`, `r1-distill`, `reasoning` — case-insensitive substring match.

### parallel matvec in the inference engine
`thread::scope` with 15 cores (no external crate — stdlib only). Each core handles a contiguous chunk of output rows. Zero-alloc `dot_q4k/dot_q6k/dot_q8_0/dot_q5k` functions process quantized blocks directly without expanding to f32 slice first.
