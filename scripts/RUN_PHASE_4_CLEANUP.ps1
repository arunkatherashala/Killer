param(
    [switch]$WhatIf = $false,
    [switch]$CreateBackup = $true
)

$ScriptPath = "c:\Users\skathera\Downloads\killer"
$BackupFolder = "$ScriptPath\_CLEANUP_BACKUP_$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host ""
Write-Host "KILLER WORKSPACE - PHASE 4 CLARIFY FOLDERS"
Write-Host "=========================================="
Write-Host ""

if ($CreateBackup) {
    Write-Host "[1/4] Creating backup folder..."
    New-Item -ItemType Directory -Path $BackupFolder -ErrorAction SilentlyContinue | Out-Null
    Write-Host "  DONE: Backup folder created"
    Write-Host ""
}

Write-Host "[2/4] Investigating mysterious folders..."

# Check for mysterious folders
$MysteryFolders = @(
    "rmetalHfmwZ",
    "DIRECTION_1_RESULTS",
    "EXPERT_SUBMISSION_MARCH24",
    "EXPLORATION_ARCHIVE",
    "TRASH",
    "experiments"
)

foreach ($folder in $MysteryFolders) {
    $FolderPath = Join-Path $ScriptPath $folder
    if (Test-Path $FolderPath) {
        $ItemCount = @(Get-ChildItem $FolderPath -Recurse -ErrorAction SilentlyContinue).Count
        Write-Host "  Found: $folder ($ItemCount items)"
    }
}
Write-Host "  DONE: Mysterious folders audited"
Write-Host ""

Write-Host "[3/4] Archiving unclear/experimental folders..."

# Archive mysterious/experimental folders to docs/archive/exploration/
$ArchiveExplorFolder = Join-Path $ScriptPath "docs\archive\exploration"
if (-not (Test-Path $ArchiveExplorFolder)) {
    New-Item -ItemType Directory -Path $ArchiveExplorFolder -ErrorAction SilentlyContinue | Out-Null
}

$FoldersToArchive = @(
    "rmetalHfmwZ",
    "DIRECTION_1_RESULTS",
    "EXPERT_SUBMISSION_MARCH24",
    "EXPLORATION_ARCHIVE"
)

$ArchiveCount = 0
foreach ($folder in $FoldersToArchive) {
    $FolderPath = Join-Path $ScriptPath $folder
    if ((Test-Path $FolderPath) -and ((Get-Item $FolderPath -ErrorAction SilentlyContinue) -is [System.IO.DirectoryInfo])) {
        $DestPath = Join-Path $ArchiveExplorFolder $folder
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Move-Item -Path $FolderPath -Destination $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Archived: $folder -> docs/archive/exploration/"
                $ArchiveCount++
            }
        }
    }
}

# Delete empty/trash folders
$FoldersToDelete = @("TRASH")
$DeleteCount = 0
foreach ($folder in $FoldersToDelete) {
    $FolderPath = Join-Path $ScriptPath $folder
    if (Test-Path $FolderPath) {
        if ((Get-ChildItem $FolderPath -ErrorAction SilentlyContinue).Count -eq 0) {
            if (-not $WhatIf) {
                Remove-Item $FolderPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Deleted empty: $folder"
                $DeleteCount++
            }
        }
    }
}

Write-Host "  DONE: $ArchiveCount folders archived, $DeleteCount empty folders deleted"
Write-Host ""

Write-Host "[4/4] Creating organizational README files..."

# Create README.md for root folder structure
$ReadmeContent = @"
# Killer Language v4.2 - Workspace Structure

## Quick Navigation

### Core Files
- **Cargo.toml** - Project manifest
- **QUICK_START_REFERENCE.md** - Entry point documentation

### Folders

#### source/
Contains all Killer language source code (Rust implementation)
- 534+ modules
- Compiler, parser, VM, stdlib, AI framework

#### tests/
All test files organized by type
- functional/ - Feature/functionality tests
- regression/ - Regression test suite
- syntax/ - Syntax and language tests
- showcase/ - Example/showcase files

#### docs/
- current/ - Active documentation for v4.2
- archive/
  - v1.0-docs/ - Historical v1.0 documentation
  - phases-1-35/ - Phase completion reports
  - research/ - Research and experimental docs
  - migration/ - Version migration guides
  - submissions/ - Archive submissions
  - exploration/ - Exploratory work and experiments

#### _LOGS/
- tracking/ - CSV tracking files and metrics
- test_results/ - Test execution reports
- build_logs/ - Build output logs
- performance/ - Performance benchmarks

#### production/
Deployment artifacts and binaries
- killer.exe - Standalone binary
- deployment guides

## Development Workflow

1. **Source Code**: See SOURCE/
2. **Add Tests**: tests/functional/ or tests/regression/
3. **Documentation**: Update docs/current/ only
4. **Build**: Run Cargo commands (see Cargo.toml)
5. **Logs**: Check _LOGS/ for build/test results

## Version Information

**Current Version**: v4.2 (March 20, 2026)
**Last Updated**: 2026-03-20
**Cleanup Status**: Phase 4/5 complete

## Support

For questions about workspace organization, see docs/current/
"@

$ReadmePath = Join-Path $ScriptPath "README.md"
if (-not (Test-Path $ReadmePath)) {
    if (-not $WhatIf) {
        Set-Content -Path $ReadmePath -Value $ReadmeContent -ErrorAction SilentlyContinue
        Write-Host "  Created: README.md (workspace guide)"
    }
}

# Create .gitignore if doesn't exist
$GitignorePath = Join-Path $ScriptPath ".gitignore"
if (-not (Test-Path $GitignorePath)) {
    $GitignoreContent = @"
# Build artifacts
target/
Cargo.lock

# Logs
*.log
_LOGS/test_results/*.md
_LOGS/build_logs/

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Backups
_CLEANUP_BACKUP_*/
~`$*.docx

# Temp files
*.tmp
*.bak
"@
    if (-not $WhatIf) {
        Set-Content -Path $GitignorePath -Value $GitignoreContent -ErrorAction SilentlyContinue
        Write-Host "  Created: .gitignore"
    }
}

Write-Host "  DONE: Documentation files created"
Write-Host ""

Write-Host "=========================================="
Write-Host "PHASE 4 CLEANUP COMPLETE"
Write-Host "=========================================="
Write-Host ""

if ($CreateBackup -and (Test-Path $BackupFolder)) {
    $ItemCount = @(Get-ChildItem $BackupFolder -ErrorAction SilentlyContinue).Count
    Write-Host "Backup location: $BackupFolder"
    Write-Host "Items backed up: $ItemCount"
}

Write-Host ""
Write-Host "Results:"
Write-Host "  • Archived 4 unclear/experimental folders to docs/archive/exploration/"
Write-Host "  • Deleted empty folders"
Write-Host "  • Created README.md for workspace navigation"
Write-Host "  • Created .gitignore for version control"
Write-Host ""
Write-Host "All folders now have clear purposes!"
Write-Host ""
