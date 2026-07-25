# HOW TO RUN PHASE 1 CLEANUP - QUICK GUIDE

## ⚡ FASTEST WAY (30 seconds)

**Open PowerShell in this folder and run:**
```powershell
.\RUN_PHASE_1_CLEANUP.ps1
```

That's it. Script will:
- ✅ Delete clutter (6 files)
- ✅ Create tests/ folder structure
- ✅ Move 25+ .killer files to organized subfolders
- ✅ Consolidate CSV files
- ✅ Delete duplicate QUICK_START files
- ✅ Create automatic backup (if something breaks, easily restore)

---

## 🔍 PREVIEW MODE (if you want to see what happens first)

**Open PowerShell and run:**
```powershell
.\RUN_PHASE_1_CLEANUP.ps1 -WhatIf
```

Shows exactly what will happen WITHOUT making changes:
```
[PREVIEW] Would delete: build_log.txt
[PREVIEW] Would move: test_round1.killer → tests\regression\
[PREVIEW] Would delete: test_results.txt
...
```

---

## ⚙️ ADVANCED OPTIONS

**Disable backup (runs faster, but no safety net):**
```powershell
.\RUN_PHASE_1_CLEANUP.ps1 -CreateBackup:$false
```

**Run in preview mode without creating backup:**
```powershell
.\RUN_PHASE_1_CLEANUP.ps1 -WhatIf -CreateBackup:$false
```

---

## 📊 WHAT WILL HAPPEN

### Files Deleted:
```
❌ build_log.txt
❌ test_results.txt
❌ ~$LLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.docx (Word temp file)
❌ ConvertToWord.bat
❌ ConvertToWord.sh
