# Killer 24/7 Auto-Builder & Test Runner
# Runs continuously on your VM — builds, tests, logs results

param(
    [int]$IntervalSeconds = 60,
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$LogDir = $null
)

$RustProject = Join-Path $ProjectRoot "SOURCE\src\v2-rust\killer"
if (-not $LogDir) { $LogDir = Join-Path $ProjectRoot "_LOGS\autobuilder" }

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

$StatusFile = Join-Path $LogDir "status.json"
$HistoryFile = Join-Path $LogDir "build_history.csv"

if (-not (Test-Path $HistoryFile)) {
    "Timestamp,BuildResult,TestsPassed,TestsFailed,TestsIgnored,Duration_ms,Commit" | Out-File $HistoryFile -Encoding utf8
}

function Write-Status {
    param($Build, $Tests, $Message)
    $status = @{
        last_update   = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
        build_status  = $Build
        test_summary  = $Tests
        message       = $Message
        running_since = $script:StartTime
        cycles        = $script:CycleCount
    }
    $status | ConvertTo-Json | Out-File $StatusFile -Encoding utf8
}

function Run-BuildCycle {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $timestamp = Get-Date -Format "yyyy-MM-dd_HH-mm-ss"
    $cycleLog = Join-Path $LogDir "cycle_$timestamp.log"

    "=== Build Cycle $($script:CycleCount) at $timestamp ===" | Out-File $cycleLog -Encoding utf8

    # --- Build ---
    Write-Status "building" "" "Compiling..."
    $buildOutput = & cargo build --release 2>&1 | Out-String
    $buildOutput | Out-File $cycleLog -Append -Encoding utf8
    $buildOk = $LASTEXITCODE -eq 0

    if (-not $buildOk) {
        Write-Status "FAILED" "" "Build failed — see $cycleLog"
        $sw.Stop()
        "$timestamp,FAIL,0,0,0,$($sw.ElapsedMilliseconds),N/A" | Out-File $HistoryFile -Append -Encoding utf8
        return
    }

    # --- Tests ---
    Write-Status "testing" "" "Running cargo test..."
    $testOutput = & cargo test --lib 2>&1 | Out-String
    $testOutput | Out-File $cycleLog -Append -Encoding utf8
    $testOk = $LASTEXITCODE -eq 0

    $passed = 0; $failed = 0; $ignored = 0
    if ($testOutput -match "test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored") {
        $passed = [int]$Matches[1]; $failed = [int]$Matches[2]; $ignored = [int]$Matches[3]
    }

    $sw.Stop()

    $buildResult = if ($testOk) { "PASS" } else { "TEST_FAIL" }
    $summary = "$passed passed, $failed failed, $ignored ignored"

    Write-Status $buildResult $summary "Cycle $($script:CycleCount) done in $($sw.ElapsedMilliseconds)ms"

    "$timestamp,$buildResult,$passed,$failed,$ignored,$($sw.ElapsedMilliseconds),N/A" | Out-File $HistoryFile -Append -Encoding utf8

    # Keep only last 50 cycle logs to save disk
    $oldLogs = Get-ChildItem $LogDir -Filter "cycle_*.log" | Sort-Object LastWriteTime -Descending | Select-Object -Skip 50
    $oldLogs | Remove-Item -Force -ErrorAction SilentlyContinue

    Write-Host "[$timestamp] $buildResult | $summary | $($sw.ElapsedMilliseconds)ms"
}

# --- Main Loop ---
$script:StartTime = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$script:CycleCount = 0

Write-Host "=== Killer Auto-Builder Started ==="
Write-Host "Project: $RustProject"
Write-Host "Logs:    $LogDir"
Write-Host "Interval: ${IntervalSeconds}s"
Write-Host "Press Ctrl+C to stop."
Write-Host ""

Push-Location $RustProject
try {
    while ($true) {
        $script:CycleCount++
        Run-BuildCycle
        Start-Sleep -Seconds $IntervalSeconds
    }
} finally {
    Pop-Location
    Write-Status "stopped" "" "Auto-builder stopped"
    Write-Host "`nAuto-builder stopped."
}
