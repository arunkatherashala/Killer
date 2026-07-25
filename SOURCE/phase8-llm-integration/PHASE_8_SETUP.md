# Phase 8 - LLM Integration Setup
**Status:** Ready to Start | **Date:** March 18, 2026

---

## 🚀 Quick Start

### Step 1: Install Ollama
1. Download: https://ollama.ai/
2. Install for your OS (Windows/Mac/Linux)
3. Restart computer after install

### Step 2: Start Ollama & Download Model
```bash
# Start Ollama service
ollama serve

# In another terminal, download a fast model
ollama pull mistral
# OR for smaller model:
# ollama pull neural-chat
```

### Step 3: Verify Ollama is Running
```bash
curl http://localhost:11434/api/tags
# Should show: {"models":[{"name":"mistral:latest",...}]}
```

### Step 4: Run Phase 8 Orchestration
```bash
killer SOURCE/phase8-llm-integration/orchestration/phase8_pure_killer_llm.killer
```

---

## 📊 What Phase 8 Measures

| Metric | Phase 7 | Phase 8 |
|--------|---------|---------|
| **Test Type** | Arithmetic only | Arithmetic + AI |
| **Latency** | Direct execution | + Ollama queries |
| **Output** | CSV: timing only | CSV: timing + AI responses |
| **Baseline** | 290.55s (7 rounds) | ?s (same 7 rounds) |

---

## 🎯 Phase 8 Goals

1. ✅ Killer makes HTTP calls to Ollama
2. ✅ Measure latency: request → Ollama → response
3. ✅ Compare Phase 7 vs Phase 8 baseline
4. ✅ Record both timing and AI insights
5. ✅ Identify bottleneck (Killer speed vs LLM latency)

---

## 📁 Phase 8 Structure

```
SOURCE/phase8-llm-integration/
├── PHASE_8_SETUP.md (this file)
├── PHASE_8_README.md (implementation guide)
├── orchestration/
│   └── phase8_pure_killer_llm.killer (main code)
└── results/
    └── phase8_llm_results.csv (auto-generated)
```

---

## ✅ Checklist Before Running

- [ ] Ollama installed locally
- [ ] Ollama running (`ollama serve`)
- [ ] Model downloaded (`ollama pull mistral`)
- [ ] Test connection: `curl http://localhost:11434/api/tags`
- [ ] Killer executable in PATH
- [ ] Phase 8 folder structure created ✅

---

## 🔧 Troubleshooting

**Ollama not found:**
```bash
# Check if installed
which ollama
# Or on Windows:
where ollama
```

**Connection refused (localhost:11434):**
- Make sure `ollama serve` is running in another terminal
- Windows: Check Task Manager for ollama process

**Model not found:**
```bash
ollama list  # See downloaded models
ollama pull mistral  # Download if missing
```

---

## 📈 Expected Results

After Phase 8 completes:
- CSV with 7 rounds
- Each round includes:
  - Killer execution time (ms)
  - Ollama query time (ms)
  - AI response
  - Total latency

**Comparison:** Phase 7 baseline vs Phase 8 with LLM overhead
