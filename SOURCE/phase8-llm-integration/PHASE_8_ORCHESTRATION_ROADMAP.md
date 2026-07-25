# PHASE 8 ORCHESTRATION - PURE KILLER + OLLAMA LLM INTEGRATION

## Overview

**Purpose:** Extend Phase 7 Killer performance testing with Ollama LLM queries
**Architecture:** Killer VM executes 7 tests → queries Ollama for optimization suggestions → measures total latency
**Baseline Comparison:** Phase 7: 290.55s (pure Killer) → Phase 8: TBD (with LLM reasoning)

---

## Phase 8 Code Structure

### file: `phase8_pure_killer_llm.killer`

#### Test Functions (Identical to Phase 7)
All 7 test functions copied from phase7_pure_killer.killer:
- `test_round_1_arithmetic()` - 1M iterations
- `test_round_2_nested_loops()` - 100K x 10 nested
- `test_round_3_fibonacci()` - 100 recursive computations
- `test_round_4_modulo_full_load()` - 100K x 100 modulo (FULL LOAD)
- `test_round_5_division_full_load()` - 100K x 100 division (FULL LOAD)
- `test_round_6_conditional_branching()` - 100K conditionals
- `test_round_7_power_operations()` - 10K power calculations

#### New: Ollama Integration
```killer
kfn query_ollama(prompt) {
    // Placeholder (simulates latency until Ollama ready)
    // Real implementation:
    // POST http://localhost:11434/api/generate
    // {
    //   "model": "mistral",
    //   "prompt": prompt,
    //   "stream": false
    // }
    
    response = "AI: Optimized algorithm reduces complexity"
    return response
}
```

**How It Works:**
1. Each test executes
2. After test completes, query Ollama with prompt like "Analyze [test_name] performance"
3. Ollama returns optimization suggestion
4. Capture both Killer time + Ollama latency
5. Record in CSV: `killer_ms, ollama_response, total_ms`

#### Orchestration Flow
```
Round 1:  start → test_arithmetic() → query_ollama() → record CSV
Round 2:  start → test_nested_loops() → query_ollama() → record CSV
...
Round 7:  start → test_power() → query_ollama() → record CSV

CSV Output: phase8_llm_results.csv
```

---

## Current Status

### ✅ Completed
- [x] Code structure created
- [x] 7 test functions implemented (copy from Phase 7)
- [x] Timing infrastructure added (time::now_milliseconds)
- [x] CSV generation framework added
- [x] Ollama function placeholder added
- [x] Phase 8 folder structure ready

### ⏳ Waiting on User
- [ ] Install Ollama (`ollama.ai`)
- [ ] Start Ollama service (`ollama serve`)
- [ ] Download model (`ollama pull mistral`)
- [ ] Verify connection (`curl http://localhost:11434/api/tags`)

### ⏳ After Ollama Ready
- [ ] Re-run `phase8_pure_killer_llm.killer`
- [ ] Replace `query_ollama()` function with HTTP calls (requires Killer HTTP library)
- [ ] Capture real Ollama latency measurements
- [ ] Generate Phase 8 results CSV with full data

---

## Expected Results

### Phase 7 Baseline (for reference)
```
Total Execution Time: 290.55s
Tests Completed: 7/7

Round Breakdown:
  1. Arithmetic: ~20s
  2. Nested Loops: ~40s
  3. Fibonacci: ~75s
  4. Modulo (FULL): ~45s
  5. Division (FULL): ~50s
  6. Branching: ~35s
  7. Power Ops: ~26s
```

### Phase 8 Expected (with Ollama)
```
Total Execution Time: 315-350s (estimated)
Tests Completed: 7/7
LLM Overhead: ~25-60s (depends on Ollama model speed)

Round Breakdown:
  1. Arithmetic + LLM Query: ~22-24s (2-4s LLM)
  2. Nested Loops + LLM Query: ~42-44s (2-4s LLM)
  ...
  7. Power Ops + LLM Query: ~28-30s (2-4s LLM)

Key Measurement: Ollama latency per query (~2-4s for mistral, <1s for neural-chat)
```

---

## CSV Output Format

**File:** `phase8_llm_results.csv`

**Columns:**
```
timestamp,round,test_name,killer_ms,ollama_response,status,total_ms
2026-03-18T00:00:00,1,Baseline Arithmetic (1M),[killer_time],[llm_response],COMPLETE,[total]
2026-03-18T00:00:00,2,Nested Loops (100K x 10),[killer_time],[llm_response],COMPLETE,[total]
...
```

**Example Entry:**
```
2026-03-18T12:34:56,1,Baseline Arithmetic (1M),21450,"AI: Consider loop unrolling for better CPU cache utilization",COMPLETE,23500
```

---

## Next Steps to Run Phase 8

### Step 1: Install Ollama
```bash
# Windows:
# Download from https://ollama.ai/download
# Or: winget install Ollama.Ollama

# macOS:
# brew install ollama

# Linux:
# curl https://ollama.ai/install.sh | sh
```

### Step 2: Start Ollama Service
```bash
ollama serve
# Output: Listening on 127.0.0.1:11434
```

### Step 3: Download Model
```bash
ollama pull mistral
# Or: ollama pull neural-chat
# (in another terminal while ollama serve running)
```

### Step 4: Verify Connection
```bash
curl http://localhost:11434/api/tags
# Should return: { "models": [ { "name": "mistral:latest", ... } ] }
```

### Step 5: Run Phase 8 Orchestration
```bash
killer SOURCE/phase8-llm-integration/orchestration/phase8_pure_killer_llm.killer
```

### Step 6: Check Results
```bash
cat phase8_llm_results.csv
# Should show 7 rounds with Killer times + LLM responses
```

---

## Implementation Details (For When HTTP Ready)

### Ollama API Call Pattern
```killer
// Pseudo-code: real implementation requires HTTP library in Killer

kfn query_ollama(prompt) {
    request = {
        "model": "mistral",
        "prompt": prompt,
        "stream": false,
        "temperature": 0.7
    };
    
    // HTTP POST to localhost:11434/api/generate
    start_ollama = time::now_milliseconds();
    response = http::post("http://localhost:11434/api/generate", request);
    end_ollama = time::now_milliseconds();
    
    ollama_time = end_ollama - start_ollama;
    
    return {
        "response": response.text,
        "latency_ms": ollama_time
    };
}
```

### Timing Measurement
```killer
// For each test:
start_killer = time::now_milliseconds();
result = test_function();
end_killer = time::now_milliseconds();
killer_time = end_killer - start_killer;

start_ollama = time::now_milliseconds();
ai_response = query_ollama(prompt);
end_ollama = time::now_milliseconds();
ollama_time = end_ollama - start_ollama;

total_time = killer_time + ollama_time;

// Record to CSV
csv_row = timestamp + "," + round + "," + test_name + "," + killer_time + "," + ai_response + "," + total_time;
```

---

## Models Available via Ollama

| Model | Size | Speed | Accuracy | Recommended |
|-------|------|-------|----------|-------------|
| **mistral** | 7B | Medium | Very High | ✅ Best for Phase 8 |
| **neural-chat** | 7B | Fast | High | Good, faster |
| **llama2** | 7B | Medium | High | General purpose |
| **orca-mini** | 3B | Very Fast | Medium | Lightweight |

**Recommendation for Phase 8:** Use `mistral` (good balance of speed & quality for analysis prompts)

---

## Troubleshooting

### "Connection refused: localhost:11434"
- **Problem:** Ollama not running
- **Solution:** Run `ollama serve` in terminal

### "No models found"
- **Problem:** Model not downloaded
- **Solution:** Run `ollama pull mistral` (while ollama serve running)

### "HTTP timeout" (when Killer tries to call Ollama)
- **Problem:** Query too complex or Ollama overloaded
- **Solution:** Simplify prompts, check Ollama logs

### Phase 8 runs slower than expected
- **Problem:** LLM queries are slow (network latency + model inference)
- **Normal:** Ollama runs on CPU by default, expect 2-5s per query
- **Optimization:** If GPU available, configure CUDA/Metal in Ollama

---

## Phase 7 vs Phase 8 Comparison Framework

**Metric:** `Overhead = (Phase8_Total - Phase7_Total) / Phase7_Total * 100%`

**Example:**
- Phase 7: 290.55s
- Phase 8 (with LLM): 340s
- Overhead: (340 - 290.55) / 290.55 * 100% = **16.9%**

**Interpretation:**
- < 10% overhead = LLM integration is efficient
- 10-20% overhead = Acceptable, LLM benefits worth cost
- > 20% overhead = May need optimization (batching, caching, etc.)

---

## Follow-up Phases (Planned)

### Phase 9: Distributed LLM Queries
- Multiple Test Runners (Killer 1, 2, 3, ...) → One Ollama Service
- Measure: Ollama queue latency, concurrent request handling

### Phase 10: LLM Caching
- Cache Ollama responses for identical prompts
- Measure: Hit rate, latency improvement

### Phase 11: Multiple LLM Services
- Killer → Ollama (optimization analysis) → Hugging Face (classification)
- Measure: Multi-model orchestration overhead

---

## Key Files

```
SOURCE/phase8-llm-integration/
├── PHASE_8_SETUP.md (installation & troubleshooting)
├── PHASE_8_ORCHESTRATION_ROADMAP.md (this file)
├── orchestration/
│   ├── phase8_pure_killer_llm.killer (main code)
│   └── [phase8_llm_results.csv] (output, generated after run)
└── [documentation/] (to be created if needed)
```

---

## Success Criteria (Checklist)

- [ ] Ollama installed and running on localhost:11434
- [ ] Phase 8 code executes without errors
- [ ] 7/7 tests complete successfully
- [ ] phase8_llm_results.csv generated with all 7 rounds
- [ ] CSV contains: killer_ms, ollama_response, total_ms for each round
- [ ] Total execution time measured (expected ~320-350s)
- [ ] Phase 7 vs Phase 8 comparison calculated
- [ ] Documentation updated with results

**Estimated Time to Complete Phase 8:** 2-3 hours (includes Ollama setup + initial runs)

---

**Last Updated:** March 18, 2026 | **Phase 8 Status:** Ready for Ollama Integration
