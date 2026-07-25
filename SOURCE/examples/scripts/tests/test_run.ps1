# Quick test: Run DPLL solver on PHP formula
# Tests: n=5, 10, 15

param(
    [string]$OutputDir = "DIRECTION_1_RESULTS"
)

$results = @()
$csvFile = "$OutputDir\DIRECTION_1_RESULTS.csv"

# Initialize CSV if needed
if (-not (Test-Path $csvFile)) {
    "n,variables,clauses,runtime_seconds,status,timestamp" | Out-File $csvFile
}

# Test configurations
$tests = @(5, 10, 15)

foreach ($n in $tests) {
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $variables = $n * ($n + 1)
    $clauses = ($n + 1) + [int][math]::Ceiling(($n * ($n + 1)) / 2) * $n
    
    Write-Host "`nTest $n (PHP_$n): $variables vars, $clauses clauses" -ForegroundColor Cyan
    
    # Check if formula file exists
    $formulaFile = "$OutputDir\php_${n}_example.cnf"
    
    if (-not (Test-Path $formulaFile)) {
        Write-Host "  ✗ Formula file not found: $formulaFile" -ForegroundColor Red
        "$n,$variables,$clauses,0,SKIP,$timestamp" | Add-Content $csvFile
        continue
    }
    
    Write-Host "  ✓ Formula found: $(Get-Item $formulaFile | Select-Object -ExpandProperty Length) bytes"
    
    # Run dpll solver
    $startTime = Get-Date
    $solverOutput = "$OutputDir\php_${n}_solve.txt"
    
    Write-Host "  [Solving...]" -ForegroundColor Yellow
    
    try {
        & killer $PSScriptRoot\SCRIPTS\dpll_solver.killer $formulaFile 2>&1 | Out-File $solverOutput -ErrorAction Continue
        $runtime = ((Get-Date) - $startTime).TotalSeconds
        $status = "SUCCESS"
        
        Write-Host "  ✓ Solved in $([math]::Round($runtime, 2))s" -ForegroundColor Green
    }
    catch {
        $runtime = ((Get-Date) - $startTime).TotalSeconds
        $status = "ERROR"
        Write-Host "  ✗ Error: $_ (${runtime}s)" -ForegroundColor Red
    }
    
    # Log result
    "$n,$variables,$clauses,$runtime,$status,$timestamp" | Add-Content $csvFile
    $results += @{ n=$n; runtime=$runtime; status=$status }
}

# Summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "RESULTS SUMMARY" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Results saved to: $csvFile"
Write-Host ""

# Show results
foreach ($r in $results) {
    $statusColor = if ($r.status -eq "SUCCESS") { "Green" } else { "Red" }
    Write-Host "  n=$($r.n): $($r.runtime)s - $($r.status)" -ForegroundColor $statusColor
}

# Show scaling
$success = $results | Where-Object { $_.status -eq "SUCCESS" }
if ($success.Count -gt 1) {
    Write-Host "`nScaling Analysis:" -ForegroundColor Yellow
    for ($i = 0; $i -lt $success.Count - 1; $i++) {
        $curr = $success[$i]
        $next = $success[$i + 1]
        $speedup = [math]::Round($next.runtime / $curr.runtime, 1)
        Write-Host "  n=$($curr.n) → n=$($next.n): ${speedup}x slower" -ForegroundColor Cyan
    }
}

Write-Host "`n✓ Test run complete!" -ForegroundColor Green
