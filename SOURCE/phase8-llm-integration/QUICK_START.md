# PHASE 8 - QUICK START CHECKLIST

## Pre-Flight Checklist

- [ ] Phase 7 baseline complete (290.55s) ✓
- [ ] Phase 8 folder created ✓
- [ ] phase8_pure_killer_llm.killer written ✓
- [ ] Documentation complete ✓

## 5-Minute Setup

### 1️⃣ Install Ollama
```
Download from: https://ollama.ai/download
Windows: Download installer + run
macOS: brew install ollama
Linux: curl https://ollama.ai/install.sh | sh
```
**Time:** ~2 minutes (download) + 1 minute (install)

### 2️⃣ Start Ollama Service
```powershell
# Terminal 1 (keep running):
ollama serve

# Expected output:
# Listening on 127.0.0.1:11434
```
**Time:** Immediate

### 3️⃣ Download Model (in another terminal)
```powershell
# Terminal 2:
ollama pull mistral

# Expected output:
# pulling manifest... DONE
# (size: ~4GB, takes ~5-10 min on fast internet)
```
**Time:** 5-10 minutes (depends on internet)

### 4️⃣ Verify Connection
```powershell
curl http://localhost:11434/api/tags

# Expected output:
# {"models":[{"name":"mistral:latest",...}]}
```
**Time:** <1 second

### 5️⃣ Run Phase 8
```powershell
cd SOURCE/phase8-llm-integration/orchestration/
killer phase8_pure_killer_llm.killer
```
**Time:** ~5 minutes (execution)

### 6️⃣ Review Results
```powershell
cat phase8_llm_results.csv
```
**Time:** <1 second

---

## Total Setup Time
- **First Time:** ~10 minutes (Ollama install + model download)
- **Subsequent Runs:** ~5 minutes (orchestration only)

---

## Troubleshooting

| Issue | Fix |
|-------|-----|
| "ollama: command not found" | Restart terminal after install |
| "Connection refused" | Check `ollama serve` is running (Terminal 1) |
| "No models found" | Run `ollama pull mistral` in Terminal 2 |
| "HTTP timeout" | Model still downloading, wait & retry |
| Phase 8 slow | Normal — Ollama ~2-3s per query on CPU |

---

## File Locations
- **Main Code:** `SOURCE/phase8-llm-integration/orchestration/phase8_pure_killer_llm.killer`
- **Results:** `phase8_llm_results.csv` (generated after run)
- **Docs:** `SOURCE/phase8-llm-integration/PHASE_8_ORCHESTRATION_ROADMAP.md`

---

## Expected Outcome

After run completes:
```
Total Execution Time: ~320-350s (vs Phase 7: 290.55s)
Tests Completed: 7/7
Overhead: ~10-20% estimated
Status: Ready for analysis
```

---

**Ready to proceed? ➡️ Install Ollama and come back.**
