# Update Killer Manual - Convert fn to kfn
$filePath = "KILLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.md"

Write-Host "Reading file..." -ForegroundColor Cyan
$content = Get-Content $filePath -Raw

Write-Host "Replacing 'fn ' with 'kfn '..." -ForegroundColor Yellow

# Count before
$countBefore = ($content | Select-String -Pattern '\bfn ' -AllMatches).Matches.Count
Write-Host "Found $countBefore instances of 'fn '" -ForegroundColor Gray

# Replace fn with kfn
$updated = $content -replace '\bfn ', 'kfn '

# Count after
$countAfter = ($updated | Select-String -Pattern '\bkfn ' -AllMatches).Matches.Count
Write-Host "Updated to $countAfter instances of 'kfn '" -ForegroundColor Green

# Save
Set-Content -Path $filePath -Value $updated -Encoding UTF8

Write-Host ""
Write-Host "✅ Update Complete!" -ForegroundColor Green
Write-Host "File: $filePath" -ForegroundColor Cyan
Write-Host ""

# Verify
Write-Host "Verification - showing updated 'kfn' examples:" -ForegroundColor Yellow
Get-Content $filePath | Select-String 'kfn' | Select-Object -First 5
