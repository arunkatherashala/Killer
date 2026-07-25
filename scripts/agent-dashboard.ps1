# Killer Status Dashboard — shows current state of your 24/7 agent
# Run anytime to see what happened while you were away

param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot),
    [int]$ShowLast = 20
)

$LogDir = Join-Path $ProjectRoot "_LOGS\autobuilder"
$StatusFile = Join-Path $LogDir "status.json"
$HistoryFile = Join-Path $LogDir "build_history.csv"

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║     KILLER 24/7 AGENT — STATUS DASHBOARD     ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# --- Live Status ---
if (Test-Path $StatusFile) {
    $status = Get-Content $StatusFile -Raw | ConvertFrom-Json
    $color = switch ($status.build_status) {
        "PASS"       { "Green" }
        "FAIL"       { "Red" }
        "TEST_FAIL"  { "Yellow" }
        "building"   { "Yellow" }
        "testing"    { "Yellow" }
        default      { "Gray" }
    }
    Write-Host "  STATUS: " -NoNewline; Write-Host "$($status.build_status)" -ForegroundColor $color
    Write-Host "  Last Update:   $($status.last_update)"
    Write-Host "  Running Since: $($status.running_since)"
    Write-Host "  Total Cycles:  $($status.cycles)"
    Write-Host "  Tests:         $($status.test_summary)"
    Write-Host "  Message:       $($status.message)"
} else {
    Write-Host "  STATUS: " -NoNewline; Write-Host "NOT RUNNING" -ForegroundColor Red
    Write-Host "  Start with: .\scripts\agent-autobuilder.ps1"
}

Write-Host ""
Write-Host "  ─── Recent Build History ───" -ForegroundColor Cyan

if (Test-Path $HistoryFile) {
    $history = Import-Csv $HistoryFile | Select-Object -Last $ShowLast

    $passCount = ($history | Where-Object BuildResult -eq "PASS").Count
    $failCount = ($history | Where-Object BuildResult -ne "PASS").Count
    $totalTests = ($history | Measure-Object -Property TestsPassed -Sum).Sum

    Write-Host ""
    Write-Host "  Last $($history.Count) builds: " -NoNewline
    Write-Host "$passCount PASS" -ForegroundColor Green -NoNewline
    Write-Host " / " -NoNewline
    Write-Host "$failCount FAIL" -ForegroundColor Red
    Write-Host "  Total tests run: $totalTests"
    Write-Host ""

    Write-Host "  Timestamp             Result     Passed  Failed  Duration" -ForegroundColor DarkGray
    Write-Host "  ─────────────────────────────────────────────────────────" -ForegroundColor DarkGray

    foreach ($row in $history) {
        $c = if ($row.BuildResult -eq "PASS") { "Green" } elseif ($row.BuildResult -eq "FAIL") { "Red" } else { "Yellow" }
        $ts = $row.Timestamp.PadRight(22)
        $res = $row.BuildResult.PadRight(10)
        $p = $row.TestsPassed.PadLeft(6)
        $f = $row.TestsFailed.PadLeft(6)
        $d = "$($row.Duration_ms)ms".PadLeft(10)
        Write-Host "  $ts" -NoNewline; Write-Host "$res" -ForegroundColor $c -NoNewline; Write-Host " $p  $f  $d"
    }
} else {
    Write-Host "  No build history yet."
}

Write-Host ""
Write-Host "  ─── Agent Processes ───" -ForegroundColor Cyan
$procs = Get-Process -Name "cargo","rustc" -ErrorAction SilentlyContinue
if ($procs) {
    $procs | Format-Table Name, Id, CPU, WorkingSet64 -AutoSize | Out-String | Write-Host
} else {
    Write-Host "  No active cargo/rustc processes"
}

Write-Host ""
