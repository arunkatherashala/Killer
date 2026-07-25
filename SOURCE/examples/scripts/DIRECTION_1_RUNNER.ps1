# Direction 1 Experiment Runner - Pigeonhole Validation
# Generates formulas and collects metrics systematically

param(
    [int[]]$nValues = @(5, 10, 15, 20, 25, 30),
    [int]$TimeoutSeconds = 300,
    [string]$OutputDir = "DIRECTION_1_RESULTS",
    [bool]$Verbose = $true
)

# Setup
$ErrorActionPreference = "Stop"
$ScriptsDir = ".\SCRIPTS"
$ResultsFile = "$OutputDir\DIRECTION_1_RESULTS.csv"
$LogFile = "$OutputDir\experiment_log.txt"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Initialize CSV header
$csvHeader = "n,variables,clauses,runtime_seconds,nodes_visited,memory_mb,formula_size_bytes,status,timestamp"
$csvHeader | Out-File -FilePath $ResultsFile -Encoding UTF8
if ($Verbose) { Write-Host "✓ Created results file: $ResultsFile" }

# Initialize log
"Direction 1: Pigeonhole Formula Validation Experiment" | Out-File -FilePath $LogFile
"Started: $(Get-Date)" | Add-Content -Path $LogFile
"" | Add-Content -Path $LogFile

# Test parameters
function Get-FormulaStats ($n) {
    $variables = $n * ($n + 1)
    $clauses = ($n + 1) + [int][math]::Ceiling(($n * ($n + 1)) / 2) * $n  # Covering + uniqueness
    @{
        Variables = $variables
        Clauses = $clauses
    }
}

# Execution loop
$results = @()

foreach ($n in $nValues) {
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "Testing Pigeonhole formula: n = $n" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $stats = Get-FormulaStats $n
    
    Write-Host "  Variables: $($stats.Variables)"
    Write-Host "  Clauses: $($stats.Clauses)"
    
    # Phase 1: Generate formula
    Write-Host "  [1/3] Generating formula..." -ForegroundColor Yellow
    $formulaFile = "$OutputDir\php_${n}.cnf"
    $genStart = Get-Date
    
    try {
        & killer pigeonhole_generator.killer $n | Out-File -FilePath $formulaFile
        $genTime = ((Get-Date) - $genStart).TotalSeconds
        Write-Host "        ✓ Generated in $($genTime)s"
    }
    catch {
        Write-Host "        ✗ Generation failed: $_" -ForegroundColor Red
        "n=$n : Generation failed - $_" | Add-Content -Path $LogFile
        continue
    }
    
    # Get formula size
    $formulaSize = (Get-Item $formulaFile).Length
    
    # Phase 2: Solve formula
    Write-Host "  [2/3] Solving formula (timeout: ${TimeoutSeconds}s)..." -ForegroundColor Yellow
    $solveStart = Get-Date
    $status = "TIMEOUT"
    $runtime = $null
    $nodes = $null
    
    try
    {
        # Run killer dpll_solver with timeout
        $proc = Start-Process -FilePath "killer" `
            -ArgumentList "dpll_solver.killer $formulaFile" `
            -NoNewWindow -PassThru -RedirectStandardOutput "$OutputDir\php_${n}_output.txt"
        
        $procExited = $proc | Wait-Process -Timeout $TimeoutSeconds -ErrorAction Stop
        $runtime = ((Get-Date) - $solveStart).TotalSeconds
        $status = "SUCCESS"
        
        Write-Host "        ✓ Solved in $($runtime)s"
        
        # Parse output for metrics
        $output = Get-Content "$OutputDir\php_${n}_output.txt" -Raw
        
        # Extract nodes visited (if logged by solver)
        if ($output -match "nodes.*?(\d+)") {
            $nodes = [int]$matches[1]
            Write-Host "        ✓ Decision nodes: $nodes"
        } else {
            Write-Host "        ⚠ Node count not found in output"
        }
    }
    catch [System.TimeoutException]
    {
        $runtime = $TimeoutSeconds
        $status = "TIMEOUT"
        Write-Host "        ⏱ Timeout after ${TimeoutSeconds}s" -ForegroundColor Yellow
        try { $proc.Kill() } catch { }
    }
    catch
    {
        $status = "ERROR"
        Write-Host "        ✗ Solver error: $_" -ForegroundColor Red
        $runtime = ((Get-Date) - $solveStart).TotalSeconds
        try { $proc.Kill() } catch { }
    }
    
    # Phase 3: Collect memory (approximate from process)
    Write-Host "  [3/3] Recording metrics..." -ForegroundColor Yellow
    $memory = "N/A"  # Would need to monitor during execution
    
    # Build result row
    $resultRow = "$n,$($stats.Variables),$($stats.Clauses),$runtime,$nodes,$memory,$formulaSize,$status,$timestamp"
    $resultRow | Add-Content -Path $ResultsFile
    $results += @{ n = $n; runtime = $runtime; nodes = $nodes; status = $status }
    
    # Log
    "n=$n : runtime=${runtime}s, nodes=$nodes, status=$status" | Add-Content -Path $LogFile
    
    # Display summary
    Write-Host "  ✓ Complete: runtime=${runtime}s, status=$status" -ForegroundColor Green
    
    # Pause between tests if getting slow
    if ($runtime -gt 60) {
        Write-Host "  ⏸ Long runtime detected, pausing before next test..." -ForegroundColor Yellow
        Start-Sleep -Seconds 5
    }
}

# Summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "EXPERIMENT SUMMARY" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan

$successCount = ($results | Where-Object { $_.status -eq "SUCCESS" }).Count
Write-Host "Successful runs: $successCount / $($nValues.Count)"
Write-Host "Results saved to: $ResultsFile"
Write-Host "Log saved to: $LogFile"

# Analyze scaling
if ($results.Count -gt 1) {
    Write-Host "`nScaling Analysis:" -ForegroundColor Yellow
    $sortedResults = $results | Sort-Object { [int]$_.n }
    
    for ($i = 0; $i -lt $sortedResults.Count - 1; $i++) {
        $current = $sortedResults[$i]
        $next = $sortedResults[$i + 1]
        
        if ($current.runtime -gt 0 -and $next.runtime -gt 0 -and $current.status -eq "SUCCESS" -and $next.status -eq "SUCCESS") {
            $speedup = [math]::Round($next.runtime / $current.runtime, 2)
            Write-Host "  n=$($current.n) to n=$($next.n): $speedup x speedup" -ForegroundColor Cyan
        }
    }
}

"Completed: $(Get-Date)" | Add-Content -Path $LogFile
Write-Host "`n✓ Experiment complete!" -ForegroundColor Green
