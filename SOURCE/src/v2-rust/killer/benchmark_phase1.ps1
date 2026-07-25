# Week 6 Phase 1 Benchmark: Variable Caching Integration
# Compares performance before and after variable cache implementation

$bench_file = 'C:\Users\skathera\Downloads\killer_V2_RS_M11\examples\arithmetic_bench_week5.killer'
$binary = '.\target\release\killer-native.exe'

$results = @()

Write-Host "=== Week 6 Phase 1 Benchmark ===" -ForegroundColor Cyan
Write-Host "Running 5 iterations of 20M arithmetic operations..."  -ForegroundColor Cyan
Write-Host ""

for ($i = 1; $i -le 5; $i++) {
    Write-Host "Run $i..." -NoNewline
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $binary $bench_file 2>&1
    $timer.Stop()
    
    $ms = $timer.ElapsedMilliseconds
    $ops_per_sec = [math]::Round(20000000.0 / ($ms / 1000.0) / 1000000, 3)
    
    $results += @{ 
        Run = $i
        TimeMs = $ms
        OpsPerSec = $ops_per_sec
        Result = $output
    }
    
    Write-Host "  $ms ms | $ops_per_sec M ops/sec"
}

# Calculate statistics
$times = $results | ForEach-Object { $_.TimeMs }
$avg_ms = $times | Measure-Object -Average | Select-Object -ExpandProperty Average
$min_ms = $times | Measure-Object -Minimum | Select-Object -ExpandProperty Minimum
$max_ms = $times | Measure-Object -Maximum | Select-Object -ExpandProperty Maximum
$avg_ops = [math]::Round(20000000.0 / ($avg_ms / 1000.0) / 1000000, 3)

Write-Host ""
Write-Host "=== PHASE 1 RESULTS ===" -ForegroundColor Green
Write-Host "Average Time: $([int]$avg_ms) ms"
Write-Host "Average Speed: $avg_ops M ops/sec"
Write-Host "Range: $([int]$min_ms) - $([int]$max_ms) ms"
Write-Host ""
Write-Host "Baseline (before Phase 1): 20,250 ms | 0.988 M ops/sec" -ForegroundColor Gray
Write-Host "Expected improvement: 1.3-1.5x (13.5-15.6s)" -ForegroundColor Gray
Write-Host "Actual improvement: $([math]::Round(20250.0 / $avg_ms, 2))x" -ForegroundColor Yellow
Write-Host ""
Write-Host "Correct result: $($results[0].Result)" -ForegroundColor Gray

# Analysis
if ($avg_ms -le 15600) {
    Write-Host "`n✅ EXCELLENT: Achieved target improvement!" -ForegroundColor Green
} elseif ($avg_ms -le 17000) {
    Write-Host "`n✅ GOOD: Within expected range" -ForegroundColor Green
} elseif ($avg_ms -le 20000) {
    Write-Host "`n⚠️  PARTIAL: Some improvement but less than expected" -ForegroundColor Yellow
} else {
    Write-Host "`n❌ NO IMPROVEMENT: Cache not providing benefit" -ForegroundColor Red
}
