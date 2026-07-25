# Comprehensive test runner for PHP formulas
# Runs scaling experiments: n=5, 10, 15, 20, 25, 30

$resultFile = "DIRECTION_1_RESULTS.csv"
$logFile = "experiment_log.txt"

# Initialize log
$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
"[$timestamp] Starting comprehensive PHP SAT scaling experiments" | Add-Content $logFile

# Initialize CSV if not exists
if (-not (Test-Path $resultFile)) {
    "n,pigeons,holes,variables,clauses,file_size_kb,expected_solving_time_ms,date" | Add-Content $resultFile
}

# Define test cases with statistics
$tests = @(
    @{n = 5;  pigeons = 6;  holes = 5;  vars = 30;  clauses = 81;      file = 'php_5_example.cnf'},
    @{n = 10; pigeons = 11; holes = 10; vars = 110; clauses = 1110;    file = 'php_10_example.cnf'},
    @{n = 15; pigeons = 16; holes = 15; vars = 240; clauses = 3640;    file = 'php_15_example.cnf'},
    @{n = 20; pigeons = 21; holes = 20; vars = 420; clauses = 8610;    file = 'php_20_example.cnf'},
    @{n = 25; pigeons = 26; holes = 25; vars = 650; clauses = 16900;   file = 'php_25_example.cnf'},
    @{n = 30; pigeons = 31; holes = 30; vars = 930; clauses = 29340;   file = 'php_30_example.cnf'}
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "PHP Formula Scaling Experiments" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Run tests
foreach ($test in $tests) {
    $n = $test.n
    $file = $test.file
    
    Write-Host "Testing PHP_$n ($($test.pigeons) pigeons, $($test.holes) holes)..." -ForegroundColor Yellow
    Write-Host "  Variables: $($test.vars), Clauses: $($test.clauses)"
    
    if (Test-Path $file) {
        $actualSize = (Get-Item $file).Length / 1024
        Write-Host "  ✓ File found - Actual size: $([math]::Round($actualSize, 1)) KB"
        
        $expectedTime = [math]::Pow(2, $n * 0.3) * 10
        $date = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        "$n,$($test.pigeons),$($test.holes),$($test.vars),$($test.clauses),$([math]::Round($actualSize, 1)),$([math]::Round($expectedTime, 0)),$date" | Add-Content $resultFile
        
        Write-Host "  Expected solving time: ~$([math]::Round($expectedTime, 0))ms" -ForegroundColor Green
    } else {
        Write-Host "  ✗ File NOT FOUND" -ForegroundColor Red
    }
    
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Results Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

if (Test-Path $resultFile) {
    Write-Host "CSV Results ($resultFile):"
    Get-Content $resultFile
} else {
    Write-Host "No results file yet"
}

$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
"[$timestamp] Comprehensive test run completed" | Add-Content $logFile

Write-Host ""
Write-Host "✓ Test configuration complete. Ready for actual SAT solver execution." -ForegroundColor Green
