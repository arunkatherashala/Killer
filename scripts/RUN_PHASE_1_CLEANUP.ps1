param(
    [switch]$WhatIf = $false,
    [switch]$CreateBackup = $true
)

$ScriptPath = "c:\Users\skathera\Downloads\killer"
$BackupFolder = "$ScriptPath\_CLEANUP_BACKUP_$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host ""
Write-Host "KILLER WORKSPACE - PHASE 1 CLEANUP"
Write-Host "======================================"
Write-Host ""

if ($CreateBackup) {
    Write-Host "[1/5] Creating backup folder..."
    New-Item -ItemType Directory -Path $BackupFolder -ErrorAction SilentlyContinue | Out-Null
    Write-Host "  DONE: Backup folder created"
    Write-Host ""
}

Write-Host "[2/5] Deleting clutter files..."
$FilesToDelete = @(
    "build_log.txt",
    "test_results.txt",
    "ConvertToWord.bat",
    "ConvertToWord.sh"
)

foreach ($file in $FilesToDelete) {
    $FilePath = Join-Path $ScriptPath $file
    if (Test-Path $FilePath) {
        if ($CreateBackup) {
            Copy-Item $FilePath $BackupFolder -ErrorAction SilentlyContinue
        }
        if (-not $WhatIf) {
            Remove-Item $FilePath -Force -ErrorAction SilentlyContinue
            Write-Host "  Deleted: $file"
        }
    }
}
Write-Host "  DONE: Clutter files deleted"
Write-Host ""

Write-Host "[3/5] Creating tests folder structure..."
$TestFolders = @(
    "tests",
    "tests\functional",
    "tests\regression",
    "tests\syntax",
    "tests\showcase"
)

foreach ($folder in $TestFolders) {
    $FolderPath = Join-Path $ScriptPath $folder
    if (-not (Test-Path $FolderPath)) {
        New-Item -ItemType Directory -Path $FolderPath -ErrorAction SilentlyContinue | Out-Null
        Write-Host "  Created: $folder"
    }
}
Write-Host "  DONE: Test folder structure created"
Write-Host ""

Write-Host "[4/5] Moving test files to tests/ folder..."
$TestPatterns = @(
    "test_*.killer",
    "TEST_*.killer",
    "K_STRING_TESTS.killer",
    "MASTER_TEST_SUITE_COMPREHENSIVE.killer",
    "KILLER_SYNTAX_*.killer"
)

$MoveCount = 0
foreach ($pattern in $TestPatterns) {
    $Files = Get-ChildItem -Path $ScriptPath -Filter $pattern -ErrorAction SilentlyContinue
    foreach ($file in $Files) {
        if ($file.DirectoryName -ne "$ScriptPath\tests") {
            $Destination = "$ScriptPath\tests\$($file.Name)"
            if (-not $WhatIf) {
                Move-Item -Path $file.FullName -Destination $Destination -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved: $($file.Name)"
                $MoveCount++
            }
        }
    }
}
Write-Host "  DONE: $MoveCount test files moved"
Write-Host ""

Write-Host "[5/5] Consolidating tracking files..."
$OldCSV = "c:\Users\skathera\Downloads\killer\MASTER_KILLER_TRACKING.csv"
if (Test-Path $OldCSV) {
    if ($CreateBackup) {
        Copy-Item $OldCSV $BackupFolder -ErrorAction SilentlyContinue
    }
    if (-not $WhatIf) {
        Remove-Item $OldCSV -Force -ErrorAction SilentlyContinue
        Write-Host "  Deleted: MASTER_KILLER_TRACKING.csv (duplicate)"
    }
}
Write-Host "  Keeping: MASTER_KILLER_TRACKING_ENHANCED.csv"
Write-Host "  DONE: Tracking files consolidated"
Write-Host ""

Write-Host "======================================"
Write-Host "CLEANUP COMPLETE"
Write-Host "======================================"
Write-Host ""

if ($CreateBackup -and (Test-Path $BackupFolder)) {
    $ItemCount = @(Get-ChildItem $BackupFolder).Count
    Write-Host "Backup location: $BackupFolder"
    Write-Host "Backup items: $ItemCount"
}

Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Review that tests/ folder has your .killer files"
Write-Host "  2. Run: git add -A"
Write-Host "  3. Run: git commit -m 'Phase 1: Cleanup workspace'"
Write-Host ""
