param(
    [switch]$WhatIf = $false,
    [switch]$CreateBackup = $true
)

$ScriptPath = "c:\Users\skathera\Downloads\killer"
$BackupFolder = "$ScriptPath\_CLEANUP_BACKUP_$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host ""
Write-Host "KILLER WORKSPACE - PHASE 2 DOCUMENTATION CLEANUP"
Write-Host "=================================================="
Write-Host ""

if ($CreateBackup) {
    Write-Host "[1/4] Creating backup folder..."
    New-Item -ItemType Directory -Path $BackupFolder -ErrorAction SilentlyContinue | Out-Null
    Write-Host "  DONE: Backup folder created"
    Write-Host ""
}

Write-Host "[2/4] Creating docs folder structure..."
$DocFolders = @(
    "docs",
    "docs\current",
    "docs\archive",
    "docs\archive\v1.0-docs",
    "docs\archive\phases-1-35",
    "docs\archive\research",
    "docs\archive\migration",
    "docs\archive\submissions"
)

foreach ($folder in $DocFolders) {
    $FolderPath = Join-Path $ScriptPath $folder
    if (-not (Test-Path $FolderPath)) {
        New-Item -ItemType Directory -Path $FolderPath -ErrorAction SilentlyContinue | Out-Null
    }
}
Write-Host "  DONE: docs/ folder structure created"
Write-Host ""

Write-Host "[3/4] Moving documentation files..."

# Files to move to docs/current/
$CurrentDocs = @(
    "KILLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.md",
    "YOU_ARE_READY_TO_PRESENT.md",
    "KILLER_v1.0_PRODUCTION_RELEASE.md",
    "KILLER_v1.0_TEAM_PRESENTATION_DECK.md",
    "DEPLOYMENT_COMPLETE.md",
    "PROJECT_TRACKING_DASHBOARD.md",
    "KILLER_COMPLETE_TRACKING_SYSTEM.md",
    "CSV_TRACKING_UPDATE_GUIDE.md",
    "KILLER_ACCURACY_AUDIT_REPORT_MARCH_20_2026.md",
    "KILLER_CSV_ANALYSIS_REAL_METRICS.md",
    "VERSION_MANIFEST.md",
    "FINAL_DELIVERY_SUMMARY_TEAM_PRESENTATION.md",
    "IMPLEMENTATION_READY_SUMMARY.md"
)

$CurrentCount = 0
foreach ($file in $CurrentDocs) {
    $FilePath = Join-Path $ScriptPath $file
    if (Test-Path $FilePath) {
        $DestPath = Join-Path $ScriptPath "docs\current\$file"
        if (-not $WhatIf) {
            Copy-Item $FilePath $DestPath -Force -ErrorAction SilentlyContinue
            Write-Host "  Moved to docs/current/: $file"
            $CurrentCount++
        }
    }
}

# Move v1.0 docs to archive
$V1Docs = Get-ChildItem -Path $ScriptPath -Filter "KILLER_v1.0_*.md" -ErrorAction SilentlyContinue
$ArchiveCount = 0
foreach ($file in $V1Docs) {
    $DestPath = Join-Path $ScriptPath "docs\archive\v1.0-docs\$($file.Name)"
    if (-not (Test-Path $DestPath)) {
        if (-not $WhatIf) {
            Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
            Write-Host "  Moved to docs/archive/v1.0-docs/: $($file.Name)"
            $ArchiveCount++
        }
    }
}

# Move phase docs to archive
$PhaseFiles = Get-ChildItem -Path $ScriptPath -Filter "PHASE_*.md" -ErrorAction SilentlyContinue
foreach ($file in $PhaseFiles) {
    $DestPath = Join-Path $ScriptPath "docs\archive\phases-1-35\$($file.Name)"
    if (-not (Test-Path $DestPath)) {
        if (-not $WhatIf) {
            Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
            Write-Host "  Moved to docs/archive/phases-1-35/: $($file.Name)"
            $ArchiveCount++
        }
    }
}

# Move research/experimental docs
$ResearchPatterns = @("KILLER_SUPER_*.md", "KILLER_VS_*.md", "KILLER_HYBRID_*.md", "KILLER_INDENTATION_*.md", "KILLER_MANUAL_*.md", "KILLER_NEW_*.md")
foreach ($pattern in $ResearchPatterns) {
    $Files = Get-ChildItem -Path $ScriptPath -Filter $pattern -ErrorAction SilentlyContinue
    foreach ($file in $Files) {
        $DestPath = Join-Path $ScriptPath "docs\archive\research\$($file.Name)"
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved to docs/archive/research/: $($file.Name)"
                $ArchiveCount++
            }
        }
    }
}

# Move migration/conversion docs
$MigrationPatterns = @("CONVERSION_*.md", "K_STRING_*.md", "LOOP_*.md", "KILLER_TEST_*.md")
foreach ($pattern in $MigrationPatterns) {
    $Files = Get-ChildItem -Path $ScriptPath -Filter $pattern -ErrorAction SilentlyContinue
    foreach ($file in $Files) {
        $DestPath = Join-Path $ScriptPath "docs\archive\migration\$($file.Name)"
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved to docs/archive/migration/: $($file.Name)"
                $ArchiveCount++
            }
        }
    }
}

Write-Host "  DONE: $CurrentCount files to docs/current/, $ArchiveCount files to docs/archive/"
Write-Host ""

Write-Host "[4/4] Consolidating duplicate reference files..."

# Delete old QUICK_* files from root (keep only QUICK_START_REFERENCE.md)
$QuickFilesToDelete = @(
    "QUICK_INDEX_START_HERE.md",
    "QUICK_REFERENCE_CARD.md",
    "FILE_INDEX_AND_GUIDE.md",
    "KILLER_QUICK_START_REFERENCE.md"
)

foreach ($file in $QuickFilesToDelete) {
    $FilePath = Join-Path $ScriptPath $file
    if (Test-Path $FilePath) {
        if ($CreateBackup) {
            Copy-Item $FilePath $BackupFolder -ErrorAction SilentlyContinue
        }
        if (-not $WhatIf) {
            Remove-Item $FilePath -Force -ErrorAction SilentlyContinue
            Write-Host "  Deleted duplicate: $file"
        }
    }
}

Write-Host "  DONE: Duplicate files consolidated"
Write-Host ""

Write-Host "=================================================="
Write-Host "PHASE 2 CLEANUP COMPLETE"
Write-Host "=================================================="
Write-Host ""

if ($CreateBackup -and (Test-Path $BackupFolder)) {
    $ItemCount = @(Get-ChildItem $BackupFolder).Count
    Write-Host "Backup location: $BackupFolder"
    Write-Host "Items backed up: $ItemCount"
}

Write-Host ""
Write-Host "Results:"
Write-Host "  • Moved $CurrentCount key docs to docs/current/"
Write-Host "  • Archived $ArchiveCount old/research/migration docs"
Write-Host "  • Consolidated duplicate reference files"
Write-Host "  • Workspace is now cleaner and more organized"
Write-Host ""

Write-Host "Verification: Check that docs/current/ contains your active documentation"
Write-Host ""
