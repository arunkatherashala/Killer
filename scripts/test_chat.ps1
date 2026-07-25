$ErrorActionPreference = 'Continue'
$bin = "C:\Users\skathera\Downloads\killer\SOURCE\src\v2-rust\killer\target\debug\killer-native.exe"
$model = "C:\Users\skathera\Downloads\killer\SOURCE\src\v2-rust\killer\qwen2.5-0.5b-instruct-q4_k_m.gguf"

Write-Host "Binary exists: $(Test-Path $bin)"
Write-Host "Model exists:  $(Test-Path $model)"
Write-Host ""
Write-Host "=== Running --model-info ==="
& $bin "--model-info" $model
Write-Host "Exit: $LASTEXITCODE"
Write-Host ""
Write-Host "=== Running --chat (stderr to file) ==="
& $bin "--chat" $model "What is 2+2?" 2>"C:\Users\skathera\Downloads\killer\chat_err.log"
Write-Host "Exit: $LASTEXITCODE"
Write-Host ""
Write-Host "=== stderr output ==="
Get-Content "C:\Users\skathera\Downloads\killer\chat_err.log"
