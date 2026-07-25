# Direction 1 Experiment Runner - Simple Version
# Generates formulas and collects metrics

param(
    [int[]]$nValues = @(5, 10, 15, 20, 25, 30),
    [string]$OutputDir = "DIRECTION_1_RESULTS"
)

$ErrorActionPreference = "Continue"
$ResultsFile = "$OutputDir\DIRECTION_1_RESULTS.csv"
$LogFile = "$OutputDir\experiment_log.txt"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Write CSV header
"n,variables,clauses,runtime_seconds,status,timestamp" | Out-File -FilePath $ResultsFile

# Initialize log
"Direction 1: Pigeonhole Formula Validation Experiment" | Out-File -FilePath $LogFile
"Started: $(Get-Date)" | Add-Content -Path $LogFile

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "STREAM A: EXPERIMENT EXECUTION" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan

foreach ($n in $nValues) {
    Write-Host "`nTesting n = $n..." -ForegroundColor Yellow
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $variables = $n * ($n + 1)
    $clauses = ($n + 1) + [int][math]::Ceiling(($n * ($n + 1)) / 2) * $n
    
    # Generate formula
    Write-Host "  [1/2] Generating PHP_$n..." -ForegroundColor Cyan
    $formulaFile = "$OutputDir\php_${n}.cnf"
    $genStart = Get-Date
    
    & killer $PSScriptRoot\SCRIPTS\pigeonhole_generator.killer $n 2>&1 | Out-File -FilePath $formulaFile
    
    if (Test-Path $formulaFile) {
        $genTime = ((Get-Date) - $genStart).TotalMilliseconds
        Write-Host "        ✓ Generated in $($genTime)ms"
    } else {
        Write-Host "        ✗ Generation failed" -ForegroundColor Red
        "n=$n,FAILED,GENERATION,$timestamp" | Add-Content -Path $ResultsFile
        continue
    }
    
    # Solve formula
    Write-Host "  [2/2] Solving with DPLL..." -ForegroundColor Cyan
    $solveStart = Get-Date
    $maxTime = 300  # 5 minute limit
    
    & killer $PSScriptRoot\SCRIPTS\dpll_solver.killer $formulaFile 2>&1 | Out-File -FilePath "$OutputDir\php_${n}_output.txt"
    
    $runtime = ((Get-Date) - $solveStart).TotalSeconds
    $status = "SUCCESS"
    
    if ($runtime -gt $maxTime) {
        $status = "TIMEOUT"
        Write-Host "        ⏱ Timeout: $($runtime)s" -ForegroundColor Yellow
    } else {
        Write-Host "        ✓ Solved in $($runtime)s"
    }
    
    # Log result
    "$n,$variables,$clauses,$runtime,$status,$timestamp" | Add-Content -Path $ResultsFile
    "n=$n : runtime=${runtime}s, status=$status" | Add-Content -Path $LogFile
    
    # Show scaling if we have previous results
    if ($n -gt $nValues[0]) {
        "  [Progress] $n variables, $clauses clauses"
    }
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "RESULTS SAVED TO: $ResultsFile" -ForegroundColor Green
Write-Host "LOG SAVED TO: $LogFile" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
