# CSV TRACKING GUIDE - HOW TO UPDATE & MAINTAIN
## Best Practices for Project Progress Tracking

**Last Updated:** March 20, 2026

---

## 📋 CSV FILES OVERVIEW

### Primary Tracking Files

1. **MASTER_KILLER_TRACKING_ENHANCED.csv**
   - Location: `c:\Users\skathera\Downloads\killer\`
   - Purpose: Single source of truth for all phases
   - Update Frequency: After each phase completion
   - Contains: 42+ phases with detailed metrics

2. **MASTER_KILLER_TRACKING.csv** 
   - Location: Same as enhanced (backup/historical)
   - Purpose: Legacy tracking (can deprecate)
   - Status: Superceded by _ENHANCED version

3. **PROJECT_TRACKING_DASHBOARD.md**
   - Location: `c:\Users\skathera\Downloads\killer\`
   - Purpose: Human-readable summary (auto-generated from CSV)
   - Update Frequency: After CSV updates

---

## 🎯 HOW TO UPDATE CSV (Step-by-Step)

### When Completing a New Phase:

#### Step 1: Run Tests
```bash
# Execute all tests for the phase
cargo test --release

# Capture output:
# - Tests passed: XXX
# - Tests failed: 0
# - Execution time: XXX ms
# - Coverage: XX.X%
```

#### Step 2: Record Raw Data
```
Keep this data handy:
- Phase name/number
- Current version (v4.2, v4.3, etc.)
- Date completed (YYYY-MM-DD format)
- Total tests run
- Tests passed
- Tests failed
- Performance time (in ms)
- Code coverage percentage
- List any new features
- Any critical issues found
- Overall code quality assessment
```

#### Step 3: Add Row to CSV
Open `MASTER_KILLER_TRACKING_ENHANCED.csv` and add:

```csv
Phase_X,v4.2,March_21_2026,50,50,0,COMPLETE,125.5,100%,99.8%,Feature_milestone,feature_list_here,0,EXCELLENT,139,YES,Brief_description
```

**Column Mapping:**
- `Phase`: Phase name (e.g., Phase_43)
- `Version`: Current version built
- `Date_Completed`: When tests passed
- `Tests_Total`: How many tests run
- `Tests_Passed`: Passing count
- `Failed`: Failing count (should be 0)
- `Status`: COMPLETE / IN_PROGRESS / BLOCKED
- `Performance_ms`: Execution time
- `Pass_Rate`: Pass percentage (100% target)
- `Code_Coverage`: Line coverage %
- `Key_Milestones`: What was delivered
- `Features_Implemented`: New functionality
- `Critical_Issues`: Blockers (0 = good)
- `Code_Quality`: EXCELLENT / GOOD / NEEDS_WORK
- `Binary_Size_KB`: Exe size (usually 139)
- `Deployment_Ready`: YES / BETA / NO
- `Notes`: Additional context

---

## 📊 SAMPLE CSV ENTRIES

### Example: New Phase Completion
```csv
Phase_43,v4.2,March_21_2026,35,35,0,COMPLETE,42.8,100%,99.9%,WebSocket_Support,WebSocket_server_client_message_handling,0,EXCELLENT,139,YES,Real_time_messaging_ready

Phase_44,v4.3,March_22_2026,28,28,0,COMPLETE,35.2,100%,99.8%,GraphQL_API,GraphQL_schema_resolver_batching,0,EXCELLENT,139,BETA,GraphQL_engine_alpha

Phase_45,v4.3,March_23_2026,22,22,0,COMPLETE,29.1,100%,99.7%,Rate_Limiting,Token_bucket_sliding_window_adaptive,0,EXCELLENT,139,COMPLETE,Production_ready
```

### Example: Performance Benchmark
```csv
PERF_BENCHMARK_Q2,v4.2,March_25_2026,10,10,0,COMPLETE,55,100%,N/A,Q2_performance_baseline,String_concat_regex_parsing_sorting,0,EXCELLENT,N/A,YES,Baseline_established_for_regression
```

### Example: Integration Test
```csv
INTEGRATION_TEST_WEBAPI,v4.2,March_26_2026,50,50,0,COMPLETE,78,100%,99.5%,End_to_end_HTTP,HTTP_REST_auth_caching_error_handling,0,EXCELLENT,139,YES,Full_API_stack_validated
```

---

## 🔄 WEEKLY UPDATE ROUTINE

### Monday Morning
1. Check last week's test results
2. Sum up passed/failed tests
3. Calculate cumulative metrics
4. Update TOTALS row
5. Generate dashboard report

### Before Each Release
1. Run full test suite
2. Record all metrics
3. Update CSV with complete phase info
4. Review for any regressions
5. Get sign-off from QA

### Monthly Status Update
1. Export CSV as PDF report
2. Create executive summary
3. Identify trends (passing rate, performance)
4. Plan next month's focus

---

## 📈 METRICS TO ALWAYS TRACK

### Essential (Never Skip)
- ✅ Phase name
- ✅ Version number
- ✅ Date completed
- ✅ Tests passed/failed
- ✅ Pass rate (%)
- ✅ Status

### Important (Track Each Phase)
- ✅ Execution time (performance)
- ✅ Code coverage %
- ✅ Critical issues count
- ✅ Code quality rating
- ✅ Deployment readiness

### Optional (Track When Available)
- ⚪ Binary size
- ⚪ Memory usage
- ⚪ Feature list
- ⚪ Milestone notes

---

## 🚨 RED FLAGS TO WATCH

### When Updating CSV, Flag These Issues:

**CRITICAL - Update Immediately:**
- [ ] Tests_Failed > 0 (any failures)
- [ ] Pass_Rate < 100%
- [ ] Status = "BLOCKED"
- [ ] Critical_Issues > 0
- [ ] Code_Quality = "NEEDS_WORK"
- [ ] Performance time **doubles** from previous phase

**HIGH - Report Next Day:**
- [ ] Performance time increased >50%
- [ ] Code_Coverage drops below 99%
- [ ] Status = "IN_PROGRESS" for >3 days
- [ ] New critical issue discovered

**MEDIUM - Monitor:**
- [ ] Performance time increased 20-50%
- [ ] Code_Coverage = 99.0-99.5% (still good)
- [ ] Deployment_Ready = "BETA"
- [ ] Notes indicate technical debt

---

## 📋 TEMPLATE FOR NEW PHASE ENTRY

Copy this template and fill in:

```csv
Phase_NEW,vX.X,YYYY-MM-DD,TESTS,PASSED,0,STATUS,TIME_MS,100%,COVERAGE,Milestone_Name,feature_one_feature_two,0,QUALITY,139,YES/BETA/NO,Notes_here
```

### Example to Copy:
```
Phase_50,v4.3,2026-03-28,45,45,0,COMPLETE,52.3,100%,99.9%,Monitoring_Tracing,Prometheus_integration_distributed_tracing,0,EXCELLENT,139,YES,Full_observability_stack
```

---

## 🔍 HOW TO ANALYZE CSV DATA

### Quick Start
```bash
# Open in Excel/Sheets for visualization
# Create pivot table by: Status, Date, Pass_Rate
# Create chart by: Phase vs Performance_ms
# Create chart by: Phase vs Code_Coverage
```

### Key Queries to Run

**1. Find any failing phases:**
```
Filter: Failed > 0
Result: Should be empty (all should pass)
```

**2. Find performance regressions:**
```
Sort by: Performance_ms (descending)
Look for: Any phase > 2x previous phase time
```

**3. Find coverage gaps:**
```
Filter: Code_Coverage < 99%
Result: Should be rare (most should be >99%)
```

**4. Track overall progress:**
```
Chart: Phases completed over time
Chart: Test count over time (should increase)
Chart: Pass rate (should stay 100%)
```

---

## 💾 BACKUP & VERSION CONTROL

### Daily Backup
```bash
# Before making changes, backup CSV
cp MASTER_KILLER_TRACKING_ENHANCED.csv MASTER_KILLER_TRACKING_ENHANCED.csv.backup

# After update, commit to git
git add MASTER_KILLER_TRACKING_ENHANCED.csv
git commit -m "Update: Add Phase_43 metrics (35/35 tests, 100%)"
```

### Monthly Archive
```bash
# End of month, save a snapshot
cp MASTER_KILLER_TRACKING_ENHANCED.csv "archive/tracking_march_2026.csv"

# In git:
git tag "tracking_march_2026"
```

### Quarterly Review
- Review trends over quarter
- Update projections for next quarter
- Clean up obsolete entries
- Generate executive summary

---

## 🎯 ADVANCED USAGE

### Generate Reports from CSV

**Excel/Google Sheets Formulas:**
```
=COUNTIF(B:B,"COMPLETE")           # Count complete phases
=AVERAGE(G:G)                       # Average performance time
=MIN(G:G)                           # Fastest phase
=MAX(G:G)                           # Slowest phase
=SUMIF(B:B,"COMPLETE",C:C)         # Total tests in complete phases
```

**Python Analysis:**
```python
import pandas as pd

df = pd.read_csv('MASTER_KILLER_TRACKING_ENHANCED.csv')

# Summary statistics
print(df['Tests_Passed'].sum())     # Total tests passed
print(df['Pass_Rate'].unique())     # Should be all 100%
print(df.groupby('Status').size())  # Tests by status
print(df['Performance_ms'].describe()) # Performance stats
```

### Create Dashboard Charts
- **Phase Completion:** Bar chart (X: Phase, Y: Tests_Passed)
- **Performance Trend:** Line chart (X: Date, Y: Performance_ms)
- **Coverage:** Line chart (X: Phase, Y: Code_Coverage)
- **Status:** Pie chart (X: Status, count)

---

## 📞 TROUBLESHOOTING

### "CSV won't open in Excel"
**Solution:** Make sure:
- File is saved as .csv (not .xlsx)
- No commas in quoted fields
- Line endings are consistent (CRLF)
- UTF-8 encoding

### "Numbers look wrong after import"
**Solution:**
- Format column as Text before import
- Or manually add ' prefix to numeric fields
- Example: '139 (not 139)

### "Can't find history of previous phases"
**Solution:**
- Check git history: `git log --oneline MASTER_KILLER_TRACKING_ENHANCED.csv`
- Or restore from backup: `git checkout HEAD~1 MASTER_KILLER_TRACKING_ENHANCED.csv`

---

## ✅ BEST PRACTICES CHECKLIST

- [ ] Update CSV **same day** phase completes
- [ ] Include ALL metrics (don't skip columns)
- [ ] Always set Pass_Rate to 100% (anything else is a blocker)
- [ ] Commit changes to git with clear message
- [ ] Run quick sanity check (no rows with Failed > 0)
- [ ] Review trends monthly
- [ ] Archive quarterly
- [ ] Share dashboard with team weekly

---

## 📊 CURRENT STATUS (For Reference)

```
Last Update: March 20, 2026
Total Phases Tracked: 42+
Total Tests: 5,604
Pass Rate: 100%
Coverage: 99.8%
Status: PRODUCTION READY
```

---

**Guide Created:** March 20, 2026  
**Next Review:** March 27, 2026  
**Questions?** Contact project lead
