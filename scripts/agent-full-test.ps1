# Killer Full Test Suite Runner
# Runs all test categories and generates a report

param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

$RustProject = Join-Path $ProjectRoot "SOURCE\src\v2-rust\killer"
$ReportDir = Join-Path $ProjectRoot "test_reports"
$timestamp = Get-Date -Format "yyyy-MM-dd_HH-mm-ss"
$ReportFile = Join-Path $ReportDir "report_$timestamp.txt"

New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

function Log { param($msg) $msg | Tee-Object -FilePath $ReportFile -Append }

Push-Location $RustProject

Log "╔═══════════════════════════════════════════════╗"
Log "║        KILLER FULL TEST REPORT                ║"
Log "║        $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')                  ║"
Log "╚═══════════════════════════════════════════════╝"
Log ""

# 1. Build check
Log "--- RELEASE BUILD ---"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$out = & cargo build --release 2>&1 | Out-String
$sw.Stop()
$buildOk = $LASTEXITCODE -eq 0
Log "Result: $(if ($buildOk) {'PASS'} else {'FAIL'})"
Log "Duration: $($sw.ElapsedMilliseconds)ms"
if (-not $buildOk) { Log $out }
Log ""

# 2. Unit tests
Log "--- UNIT TESTS (cargo test --lib) ---"
$sw.Restart()
$out = & cargo test --lib 2>&1 | Out-String
$sw.Stop()
Log $out
Log "Duration: $($sw.ElapsedMilliseconds)ms"
Log ""

# 3. Check for warnings
Log "--- COMPILER WARNINGS ---"
$warnings = & cargo build --release 2>&1 | Out-String | Select-String "warning\[" | Measure-Object
Log "Warning count: $($warnings.Count)"
Log ""

# 4. Binary size
$binary = Get-ChildItem (Join-Path $RustProject "target\release") -Filter "killer*" -File -ErrorAction SilentlyContinue | Select-Object -First 1
if ($binary) {
    Log "--- BINARY SIZE ---"
    Log "File: $($binary.Name)"
    Log "Size: $([math]::Round($binary.Length / 1MB, 2)) MB"
    Log ""
}

# 5. Run .killer test files if binary exists
if ($binary) {
    $testFiles = Get-ChildItem (Join-Path $ProjectRoot "tests") -Filter "*.killer" -Recurse -ErrorAction SilentlyContinue
    if ($testFiles) {
        Log "--- .KILLER FILE TESTS ($($testFiles.Count) files) ---"
        $pass = 0; $fail = 0
        foreach ($tf in $testFiles | Select-Object -First 50) {
            $result = & $binary.FullName $tf.FullName 2>&1 | Out-String
            if ($LASTEXITCODE -eq 0) { $pass++ } else { $fail++; Log "  FAIL: $($tf.Name)" }
        }
        Log "Passed: $pass / $($pass + $fail)"
        Log ""
    }
}

Pop-Location

Log "═══════════════════════════════════════════════"
Log "Report saved: $ReportFile"

Write-Host ""
Write-Host "Full report saved to: $ReportFile" -ForegroundColor Green
