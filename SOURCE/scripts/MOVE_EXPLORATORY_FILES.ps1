$SourceRoot = "c:\Users\skathera\Downloads\killer_V2_RS_M11"
$ArchiveBase = "$SourceRoot\EXPLORATION_ARCHIVE"

# Ensure directories exist
$dirs = @(
    "$ArchiveBase\phase-7-research",
    "$ArchiveBase\orchestration-experiments",
    "$ArchiveBase\build-logs",
    "$ArchiveBase\ai-integration-research",
    "$ArchiveBase\optimization-research"
)

foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
}

# Phase 7 Research Files
$phase7Files = @(
    "AI_INTEGRATION_FINAL_STATUS.md",
    "ALWAYS_BUILD_ANALYSIS.md",
    "COMPREHENSIVE_GAP_ANALYSIS_REPORT.md",
    "FIBONACCI_u64MAX_FINAL_REVIEW_v7_1.md",
    "FULL_KILLER_100_PERCENT_TEST_PLAN.md",
    "KILLER_AGENT_COMPLETE.md",
    "KILLER_AI_FIRST_PHASE1_COMPLETE.md",
    "KILLER_AI_INTEGRATION_COMPLETE.md",
    "KILLER_CLUSTER_DEMO_RELEASE.md",
    "KILLER_COMMAND_CENTER.md",
    "KILLER_ROADMAP_2026_STRATEGIC_ANALYSIS.md",
    "KILLER_THUMB_RULES.md",
    "KILLER_V2_COMPLETE_COMPREHENSIVE_TEST_REPORT.md",
    "KILLER_V2_COMPREHENSIVE_PERFORMANCE_TRACKING.md",
    "KILLER_V2_HISTORICAL_PERFORMANCE_DATA.md",
    "KILLER_V2_SPEED_TEST_COMPLETE.md",
    "MARCH_24_2026_SUBMISSION.md",
    "MASTER_STRATEGIES_REFERENCE_GUIDE.md",
    "PHASE_4_COMPLETION.md",
    "PHASE_7_ACHIEVEMENT_SUMMARY.md",
    "PHASE_7_ARU_PRINCIPLE_DOCUMENTATION.md",
    "PHASE_7_COMPLETE_PHASE_8_READY.md",
    "PHASE_7_FINAL_7_ROUNDS_COMPLETE.md",
    "PHASE_7_FULL_LOAD_PERFORMANCE_FINAL_REPORT.md",
    "PHASE_7_KILLER_PYTHON_ORCHESTRATION_FINAL.md",
    "PHASE_7_MASTER_INDEX.md",
    "PHASE_7_PURE_KILLER_COMPLETE.md",
    "PHASE_8_LLM_INTEGRATION_PLAN.md",
    "SPEED_TEST_REPORT_MARCH18.md",
    "ARU_MASTER_INDEX_AND_GUIDE.md",
    "ARU_STRATEGY_COMPARISON_AND_PLACEMENT.md",
    "ARU_STRATEGY_FRAMEWORK.md"
)

$moved = 0
Set-Location $SourceRoot
foreach ($file in $phase7Files) {
    if (Test-Path $file) {
        Move-Item -LiteralPath $file -Destination "$ArchiveBase\phase-7-research\" -Force
        $moved++
        Write-Host "✓ $file"
    }
}
Write-Host "Moved $moved Phase 7 research files"

# Orchestration Files
$orchFiles = @(
    "killer_orchestration_master.killer",
    "killer_orchestration_phase7.killer",
    "KILLER_ORCHESTRATION_PHASE7_PURE.killer",
    "killer_super_performance_test.ps1",
    "killer_super_quick_benchmark.ps1",
    "run_performance_test_full_load.py",
    "run_phase7_killer_orchestration.py",
    "run_phase7_orchestration_worldclass.py"
)

$moved2 = 0
foreach ($file in $orchFiles) {
    if (Test-Path $file) {
        Move-Item -LiteralPath $file -Destination "$ArchiveBase\orchestration-experiments\" -Force
        $moved2++
        Write-Host "✓ $file"
    }
}
Write-Host "Moved $moved2 orchestration files"

# Build & Log Files
$buildFiles = @(
    "build.log",
    "build_error.log",
    "build_errors.log",
    "build_final.log",
    "complete_build_log.txt",
    "full_build.log",
    "orchestration_output.txt",
    "orchestration_run.log",
    "performance_records_full_load.csv",
    "performance_test_full_load.log",
    "e0716.txt",
    "speed_test_final_results.txt",
    "Cargo.lock",
    "Cargo.toml"
)

$moved3 = 0
foreach ($file in $buildFiles) {
    if (Test-Path $file) {
        Move-Item -LiteralPath $file -Destination "$ArchiveBase\build-logs\" -Force
        $moved3++
        Write-Host "✓ $file"
    }
}
Write-Host "Moved $moved3 build/log files"

Write-Host ""
Write-Host "TOTAL MOVED: $($moved + $moved2 + $moved3) files"
Write-Host "✅ Cleanup complete!"
