Write-Host "========================================" -ForegroundColor Cyan
Write-Host "PHP Formula Scaling Experiments" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$tests = @(
    @{n = 5;  pigeons = 6;  holes = 5;  vars = 30;  clauses = 81;      file = 'php_5_example.cnf'},
    @{n = 10; pigeons = 11; holes = 10; vars = 110; clauses = 1110;    file = 'php_10_example.cnf'},
    @{n = 15; pigeons = 16; holes = 15; vars = 240; clauses = 3640;    file = 'php_15_example.cnf'}
)

foreach ($test in $tests) {
    $n = $test.n
    $file = $test.file
    
    Write-Host "Testing PHP_$n ($($test.pigeons) pigeons, $($test.holes) holes)..." -ForegroundColor Yellow
    Write-Host "  Variables: $($test.vars), Clauses: $($test.clauses)"
    
    if (Test-Path $file) {
        $actualSize = (Get-Item $file).Length / 1024
        Write-Host "  ✓ File found - Size: $([math]::Round($actualSize, 1)) KB" -ForegroundColor Green
    } else {
        Write-Host "  ✗ File NOT FOUND" -ForegroundColor Red
    }
    
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Test Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
