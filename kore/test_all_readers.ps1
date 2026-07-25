# KORE Reader Test Script — Run in PowerShell
# cd C:\Users\skathera\Downloads\killer_M29\killer\kore
# .\test_all_readers.ps1

$ErrorActionPreference = "Continue"
$base = $PSScriptRoot

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " KORE Multi-Language Reader Tests" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Python
Write-Host "--- [1/5] Python ---" -ForegroundColor Yellow
Push-Location "$base\readers\python"
try { python test_kore_reader.py 2>&1 | Write-Host }
catch { Write-Host "SKIP: Python not found" -ForegroundColor Red }
Pop-Location
Write-Host ""

# 2. Go
Write-Host "--- [2/5] Go ---" -ForegroundColor Yellow
Push-Location "$base\readers\go"
if (Test-Path "kore_test.exe") {
    .\kore_test.exe "..\..\test\test_v2.kore" 2>&1 | Write-Host
} else {
    Write-Host "Building Go..."
    go build -o kore_test.exe . 2>&1 | Write-Host
    if (Test-Path "kore_test.exe") {
        .\kore_test.exe "..\..\test\test_v2.kore" 2>&1 | Write-Host
    } else { Write-Host "SKIP: Go build failed" -ForegroundColor Red }
}
Pop-Location
Write-Host ""

# 3. TypeScript (Node.js)
Write-Host "--- [3/5] TypeScript/Node.js ---" -ForegroundColor Yellow
Push-Location "$base\readers\typescript"
try { node kore_test.mjs "..\..\test\test_v2.kore" 2>&1 | Write-Host }
catch { Write-Host "SKIP: Node.js not found" -ForegroundColor Red }
Pop-Location
Write-Host ""

# 4. C# (.NET)
Write-Host "--- [4/5] C# ---" -ForegroundColor Yellow
Push-Location "$base\readers\csharp"
try { dotnet-script KoreReader.cs -- "..\..\test\test_v2.kore" 2>&1 | Write-Host }
catch { Write-Host "SKIP: dotnet-script not found" -ForegroundColor Red }
Pop-Location
Write-Host ""

# 5. Java
Write-Host "--- [5/5] Java ---" -ForegroundColor Yellow
Push-Location "$base\readers\java"
if (Get-Command javac -ErrorAction SilentlyContinue) {
    javac kore/KoreReader.java 2>&1 | Write-Host
    java -cp . kore.KoreReader "..\..\test\test_v2.kore" 2>&1 | Write-Host
} else { Write-Host "SKIP: Java not installed" -ForegroundColor Red }
Pop-Location
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " ALL TESTS COMPLETE" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
