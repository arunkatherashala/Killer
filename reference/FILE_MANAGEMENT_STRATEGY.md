# 📋 FILE MANAGEMENT STRATEGY - HOW TO MANAGE FILES
**Clear Rules for: Create New vs Update Old | Consolidate vs Delete | Track vs Archive**

**Version:** 1.0 | **Date:** March 20, 2026

---

## 🎯 THE CORE PRINCIPLE

### ONE FILE = ONE PURPOSE. NO DUPLICATES.

```
❌ WRONG:
  TRACKING_v1.csv
  TRACKING_v2.csv
  TRACKING_v3.csv
  (confusion: which is current?)

✅ RIGHT:
  MASTER_KILLER_TRACKING_ENHANCED.csv (always ONE file, kept updated)
  (everyone knows: THIS is the source of truth)
```

---

## 📊 WHEN TO: CREATE NEW vs UPDATE OLD

### Decision Tree

```
"I have data to add/update"
│
├─ "Is this type of data already tracked somewhere?"
│  │
│  ├─ YES → "Update that existing file" (go to RULE 2)
│  │
│  └─ NO → "Create NEW file" (go to RULE 1)
│
└─ "Should this be a separate file or combined?"
   │
   ├─ "Same category as existing?" → COMBINE into one
   │
   └─ "New category?" → Create new file in appropriate folder
```

---

## 📝 RULE 1: CREATE NEW FILE (When appropriate)

### ✅ DO Create New File When:

1. **NEW feature/module**
   ```
   Example: Added async/await support
   Action: Create SOURCE/async_await.rs
   ```

2. **NEW test category**
   ```
   Example: GPU tests for new functionality
   Action: Create tests/gpu/test_cuda.killer
   ```

3. **NEW report type (never seen before)**
   ```
   Example: First time tracking "AI accuracy"
   Action: Create docs/current/AI_ACCURACY_REPORT.md
   Then: Add reference to MASTER_INDEX.md
   ```

4. **Genuinely different purpose**
   ```
   Example: Performance profiling (vs testing)
   Action: Create _LOGS/performance/profile_report.txt
   (separate from test results)
   ```

### ✅ NEW FILE CHECKLIST:
- [ ] Serves purpose not covered by existing files
- [ ] Will be actively used/updated
- [ ] Not a duplicate or variant of existing file
- [ ] Clear naming (describes content)
- [ ] Goes in correct folder
- [ ] Add reference to MASTER_INDEX.md

### ❌ DON'T Create New If:
- Similar file already exists (UPDATE it instead)
- Just a different version/variant (CONSOLIDATE instead)
- Temporary/experimental (USE _LOGS/experiments/ instead)
- Will only be used once (ARCHIVE when done)

---

## ✏️ RULE 2: UPDATE EXISTING FILE (Most Common)

### ✅ DO Update Existing File When:

1. **Adding new data to same category**
   ```
   Example: New test results
   File: _LOGS/tracking/MASTER_KILLER_TRACKING_ENHANCED.csv
   Action: 
     • Open existing CSV
     • Add new row with data
     • Update "Date" column
     • Save (don't create new file)
   ```

2. **Updating status/progress**
   ```
   Example: Current deployment status changed
   File: docs/current/DEPLOYMENT_COMPLETE.md
   Action:
     • Open existing doc
     • Update status section
     • Change "Last Updated" date
     • Save (don't create new version)
   ```

3. **Adding to ongoing report**
   ```
   Example: New performance benchmark
   File: _LOGS/test_results/PERFORMANCE_REPORT.md
   Action:
     • Open existing report
     • Add new benchmark results
     • Update summary
     • Don't create PERFORMANCE_REPORT_v2.md
   ```

### ✅ UPDATE CHECKLIST:
- [ ] Same category as existing file
- [ ] File already exists and is being maintained
- [ ] Adding new data/info to same concept
- [ ] Keep file name SAME (no _v2, _new, _latest suffix)
- [ ] Update "Last Updated" date header
- [ ] Keep version history in one file (don't create duplicates)

### ❌ DON'T Update If:
- File served a one-time purpose (ARCHIVE it)
- Data fundamentally different from original (CREATE NEW)
- File is read-only/archived (COPY to current folder if changing)
- Multiple people editing simultaneously (use CONSOLIDATE strategy)

---

## 🔄 RULE 3: CONSOLIDATE (When duplicates exist)

### ✅ DO Consolidate When:

1. **Multiple versions of same tracking**
   ```
   Problem: 
     MASTER_KILLER_TRACKING.csv (old)
     MASTER_KILLER_TRACKING_ENHANCED.csv (new)
   
   Solution:
     1. Copy data from OLD into NEW
     2. Add new data to NEW
     3. Delete OLD file
     4. Keep only: MASTER_KILLER_TRACKING_ENHANCED.csv
   ```

2. **Multiple similar docs**
   ```
   Problem:
     QUICK_START_REFERENCE.md
     QUICK_INDEX_START_HERE.md
     QUICK_REFERENCE_CARD.md
   
   Solution:
     1. Keep BEST version as canonical
     2. Copy any unique info from others into canonical
     3. Delete all duplicate versions
     4. One file only: QUICK_START_REFERENCE.md
   ```

3. **Version proliferation**
   ```
   Problem:
     STATUS_REPORT.md (old, March 1)
     STATUS_REPORT_v2.md (March 10)
     STATUS_REPORT_UPDATED.md (March 15)
     FINAL_STATUS_REPORT.md (March 20)
   
   Solution:
     1. Keep ONLY: STATUS_REPORT.md (March 20)
     2. Merge all data: Keep best from each version
     3. Delete: v2, UPDATED, FINAL versions
     4. Update date: March 20, 2026
   ```

### ✅ CONSOLIDATION PROCESS:

```
Step 1: Identify duplicates
  └─ Find all versions (search QUICK_*, STATUS_*, etc.)

Step 2: Pick canonical version
  └─ Usually: the most recent or best-maintained one

Step 3: Merge data
  ├─ Copy unique info from OLD into NEW
  ├─ Remove duplicates
  └─ Keep best version of each section

Step 4: Update metadata
  ├─ Set date to TODAY
  ├─ Add consolidation note: "Consolidated March 20, 2026"
  └─ Keep version info in file (e.g., "v1 (consolidated)")

Step 5: Delete old versions
  ├─ Delete all duplicate/old versions
  ├─ Keep ONLY consolidated version
  └─ Verify: Exactly ONE file for this purpose

Step 6: Test
  ├─ Verify no data lost
  ├─ Check all links work
  └─ Update MASTER_INDEX.md reference
```

### ❌ DON'T Consolidate If:
- Files serve different purposes (keep both)
- Data conflicts and can't be merged (archive, keep current)
- Multiple people editing (wait for all edits complete)

---

## 🗑️ RULE 4: DELETE or ARCHIVE

### 🗑️ DELETE FILE If:

1. **Duplicate with no unique data**
   ```
   Example: QUICK_INDEX_START_HERE.md (duplicate of QUICK_START_REFERENCE.md)
   Action: DELETE (after ensuring no unique info)
   ```

2. **Completely superseded**
   ```
   Example: KILLER_v1.0_COMPREHENSIVE_TEST_REPORT.md (was for v1.0, now v4.2)
   Action: Check if need for history
     └─ If no: DELETE
     └─ If yes: ARCHIVE to docs/archive/
   ```

3. **Temporary/experimental file**
   ```
   Example: test_experiment.killer (one-off test)
   Action: DELETE after test complete (or archive if might reuse)
   ```

### 📦 ARCHIVE FILE If:

1. **Historical value**
   ```
   Example: PHASE_1_COMPLETION_REPORT.md (completed phase)
   Action: Move to docs/archive/phases-1-35/
   ```

2. **Old version still relevant**
   ```
   Example: KILLER_v1.0_MANUAL.md (v1 still referenced)
   Action: Move to docs/archive/v1.0-docs/
   ```

3. **Experimental but might reuse**
   ```
   Example: KILLER_HYBRID_INDENTATION_SPECIFICATION.md (experimental feature)
   Action: Move to docs/archive/research/
   ```

### ✅ DELETE vs ARCHIVE Decision:

```
"Should I delete or archive this file?"

├─ "Will anyone ever need this again?" 
│  ├─ "YES (historical/reference)" → ARCHIVE
│  └─ "NO (duplicate/old)" → DELETE
│
├─ "Is there unique data?"
│  ├─ "YES (different info)" → ARCHIVE (keep for reference)
│  └─ "NO (same as current)" → DELETE
│
└─ "Is file actively used?"
   ├─ "YES" → KEEP in docs/current/
   ├─ "NO (rarely referenced)" → ARCHIVE
   └─ "NEVER (obsolete)" → DELETE
```

---

## 📍 WHERE FILES GO

### docs/current/ (ACTIVE - 13 files max)
```
✅ Files that are actively used/updated
✅ Most recent versions only
✅ Current documentation
❌ Old versions
❌ Experimental docs
❌ Completed one-time reports
```

### docs/archive/ (HISTORICAL)
```
✅ v1.0-docs/ - Version 1.0 documentation
✅ phases-1-35/ - Completed phase reports
✅ research/ - Experimental/research docs
✅ migration/ - Version migration guides
✅ exploration/ - Exploratory work
```

### _LOGS/ (TRACKING & REPORTS)
```
✅ tracking/ - Active tracking CSVs (always current)
✅ test_results/ - Test reports (keep most recent)
✅ build_logs/ - Build output logs
✅ performance/ - Performance benchmark data
```

### _DELETE (Not archived anywhere)
```
✅ Duplicate files (old versions of current)
✅ Temporary files (no ongoing value)
✅ Test artifacts (one-time use)
✅ Redundant copies
```

---

## 📋 COMMON SCENARIOS

### Scenario 1: New Info Related to Existing Doc

**Situation:** You have new performance data to add to existing performance report

```
Current File: docs/current/PERFORMANCE_REPORT.md
New Data: Latest benchmark results

ACTION:
  1. Open: docs/current/PERFORMANCE_REPORT.md (existing)
  2. Add: New benchmark results
  3. Update: "Last Updated" date header
  4. Save: Same file (DON'T create _v2)
  5. Done! (only one file, always current)
```

### Scenario 2: Similar But Different Purpose

**Situation:** You created PERFORMANCE_REPORT.md but also have PERFORMANCE_DETAILS.md

```
Current Files: 
  - PERFORMANCE_REPORT.md (summary)
  - PERFORMANCE_DETAILS.md (detailed data)

DECISION:
  1. Ask: Do these serve different purposes?
     └─ Report (summary for execs)
     └─ Details (technical data for engineers)
     → Keep both (different purposes)
  
  2. Ask: Or are they just different versions?
     └─ Both trying to document same thing
     → CONSOLIDATE (keep one, delete other)
```

### Scenario 3: Tracking Metrics Over Time

**Situation:** New test results each week - create new file each time?

```
❌ WRONG - creates this mess:
  TEST_RESULTS_WEEK1.csv
  TEST_RESULTS_WEEK2.csv
  TEST_RESULTS_WEEK3.csv
  (10+ versions, confusing which is current)

✅ RIGHT - use ONE file:
  MASTER_KILLER_TRACKING_ENHANCED.csv
  ├─ Add row for Week 1
  ├─ Add row for Week 2
  ├─ Add row for Week 3
  └─ Always ONE file, always current (append data)
```

### Scenario 4: Different Status Reports

**Situation:** Many status reports exist (DEPLOYMENT_COMPLETE, STATUS_REPORT, PROGRESS, etc.)

```
ACTION:
  1. List all status files
  2. Pick CANONICAL name (best one)
  3. Merge data from others into canonical
  4. Delete duplicates
  5. Example:
     Keep: PROJECT_TRACKING_DASHBOARD.md
     Merge data from: STATUS_REPORT, PROGRESS, etc.
     Delete: Old versions
```

---

## 🎯 TEMPLATE: FILE DECISION CHECKLIST

Use this when deciding whether to create new file:

```
DECIDING WHETHER TO CREATE NEW FILE

Question 1: Does this purpose already exist?
  ☐ YES - go to Question 2
  ☐ NO - CREATE NEW FILE (appropriate)

Question 2: Can I add this data to existing file?
  ☐ YES - UPDATE EXISTING (not new)
  ☐ NO - go to Question 3

Question 3: Is this genuinely different purpose?
  ☐ YES - CREATE NEW FILE
  ☐ NO - CONSOLIDATE existing files

Question 4: Will this file be actively maintained?
  ☐ YES - CREATE NEW FILE (ongoing use)
  ☐ NO - Don't create (use existing or archive)

Question 5: Is there already a better/newer version?
  ☐ YES - CONSOLIDATE (merge and delete old)
  ☐ NO - CREATE NEW FILE
```

---

## 🚨 AVOID THESE MISTAKES

### ❌ Mistake 1: Version Proliferation
```
WRONG:
  report.md
  report_v2.md
  report_updated.md
  report_final.md
  
RIGHT:
  report.md (always one file, keep updated)
```

### ❌ Mistake 2: Unclear File Names
```
WRONG:
  data.csv
  info.txt
  stuff.md
  
RIGHT:
  MASTER_KILLER_TRACKING_ENHANCED.csv
  DEPLOYMENT_STATUS_REPORT.md
  PROJECT_STRUCTURE.md
```

### ❌ Mistake 3: Mixing Old & New
```
WRONG:
  docs/
  ├── MANUAL_v1.md (old)
  ├── MANUAL_v2.md (current)
  ├── MANUAL_LATEST.md (newest)
  All mixed together = CONFUSING
  
RIGHT:
  docs/current/
  ├── MANUAL.md (one file, always current)
  
  docs/archive/v1.0-docs/
  ├── MANUAL.md (old version for reference)
```

### ❌ Mistake 4: No Consolidation
```
WRONG:
  Multiple QUICK_START files
  Multiple TRACKING files
  Multiple STATUS reports
  = CONFUSION about which is real
  
RIGHT:
  One QUICK_START_REFERENCE.md in root
  One MASTER_KILLER_TRACKING_ENHANCED.csv in _LOGS/tracking/
  = CLEAR what's current
```

---

## ✅ AFTER YOU UNDERSTAND THIS

### Going Forward:
1. **Before creating file:** Check if it already exists
2. **Before updating:** Make sure updating correct version (the current one!)
3. **Before deleting:** Verify no unique data lost
4. **When confused:** Check MASTER_INDEX.md (bookmark it!)

### Your New Habit:
```
I have data → Check: Does file exist?
  ├─ YES → UPDATE (don't create new)
  └─ NO → CREATE (if truly new purpose)

When done → Check: Are there duplicates?
  ├─ YES → CONSOLIDATE (keep one, delete others)
  └─ NO → Done!
```

---

## 🎊 SUMMARY

**The System:**
- ✅ ONE file per purpose (no duplicates)
- ✅ UPDATE existing when adding data (don't create new)
- ✅ CONSOLIDATE when duplicates exist (keep one, delete others)
- ✅ ARCHIVE when done (preserve history)
- ✅ Only 13 active docs at once

**The Benefits:**
- 🎯 Clear which file is current
- 🎯 No duplicate data conflicts
- 🎯 Easy to maintain (fewer files to track)
- 🎯 Historical record preserved (archived)
- 🎯ectable and scalable

**The Golden Rule:**
```
CREATE NEW = rarely
UPDATE EXISTING = usually
CONSOLIDATE = when duplicates appear
ARCHIVE = when done
DELETE = only if no value
```

---

*File Management Strategy v1.0*  
*March 20, 2026*  
*Reference: MASTER_INDEX.md*
