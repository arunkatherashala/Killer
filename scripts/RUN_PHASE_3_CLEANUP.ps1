param(
    [switch]$WhatIf = $false,
    [switch]$CreateBackup = $true
)

$ScriptPath = "c:\Users\skathera\Downloads\killer"
$BackupFolder = "$ScriptPath\_CLEANUP_BACKUP_$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host ""
Write-Host "KILLER WORKSPACE - PHASE 3 LOGS CLEANUP"
Write-Host "========================================"
Write-Host ""

if ($CreateBackup) {
    Write-Host "[1/5] Creating backup folder..."
    New-Item -ItemType Directory -Path $BackupFolder -ErrorAction SilentlyContinue | Out-Null
    Write-Host "  DONE: Backup folder created"
    Write-Host ""
}

Write-Host "[2/5] Creating _LOGS folder structure..."
$LogFolders = @(
    "_LOGS",
    "_LOGS\tracking",
    "_LOGS\test_results",
    "_LOGS\build_logs",
    "_LOGS\performance"
)

foreach ($folder in $LogFolders) {
    $FolderPath = Join-Path $ScriptPath $folder
    if (-not (Test-Path $FolderPath)) {
        New-Item -ItemType Directory -Path $FolderPath -ErrorAction SilentlyContinue | Out-Null
    }
}
Write-Host "  DONE: _LOGS/ folder structure created"
Write-Host ""

Write-Host "[3/5] Moving CSV tracking files..."
$TrackingFiles = @(
    "MASTER_KILLER_TRACKING_ENHANCED.csv",
    "MASTER_KILLER_TRACKING.csv"
)

$TrackingCount = 0
foreach ($file in $TrackingFiles) {
    $FilePath = Join-Path $ScriptPath $file
    if (Test-Path $FilePath) {
        $DestPath = Join-Path $ScriptPath "_LOGS\tracking\$file"
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Copy-Item $FilePath $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved to _LOGS/tracking/: $file"
                $TrackingCount++
            }
        }
    }
}
Write-Host "  DONE: $TrackingCount tracking files moved"
Write-Host ""

Write-Host "[4/5] Moving performance and test result files..."

# Move performance CSV
$PerfFiles = Get-ChildItem -Path $ScriptPath -Filter "performance_*.csv" -ErrorAction SilentlyContinue
foreach ($file in $PerfFiles) {
    $DestPath = Join-Path $ScriptPath "_LOGS\performance\$($file.Name)"
    if (-not (Test-Path $DestPath)) {
        if (-not $WhatIf) {
            Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
            Write-Host "  Moved to _LOGS/performance/: $($file.Name)"
        }
    }
}

# Move test report files
$ReportPatterns = @("*_REPORT_*.md", "*_REPORT.md", "TEST_EXECUTION_*.md")
$ReportCount = 0
foreach ($pattern in $ReportPatterns) {
    $Reports = Get-ChildItem -Path $ScriptPath -Filter $pattern -ErrorAction SilentlyContinue
    foreach ($file in $Reports) {
        $DestPath = Join-Path $ScriptPath "_LOGS\test_results\$($file.Name)"
        if (-not (Test-Path $DestPath) -and $file.Name -notmatch "PHASE_\d_CLEANUP") {
            if (-not $WhatIf) {
                Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved to _LOGS/test_results/: $($file.Name)"
                $ReportCount++
            }
        }
    }
}

# Move deployment/dry-run reports
$DeployPatterns = @("DEPLOYMENT_*.md", "DEPLOYMENT_*.txt", "*DRY_RUN*.md", "*DRY_RUN*.txt")
foreach ($pattern in $DeployPatterns) {
    $Files = Get-ChildItem -Path $ScriptPath -Filter $pattern -ErrorAction SilentlyContinue
    foreach ($file in $Files) {
        $DestPath = Join-Path $ScriptPath "_LOGS\test_results\$($file.Name)"
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Copy-Item $file.FullName $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Moved to _LOGS/test_results/: $($file.Name)"
                $ReportCount++
            }
        }
    }
}

Write-Host "  DONE: $ReportCount test/deployment reports moved"
Write-Host ""

Write-Host "[5/5] Organizing existing _LOGS/ contents..."

# If _LOGS/ already exists at root with files, move them to appropriate subfolders
$ExistingLogs = Get-ChildItem -Path "$ScriptPath\_LOGS" -ErrorAction SilentlyContinue | Where-Object {-not $_.PSIsContainer}
$ExistingCount = 0
foreach ($file in $ExistingLogs) {
    if ($file.Name -match "\.csv$") {
        $DestPath = Join-Path $ScriptPath "_LOGS\tracking\$($file.Name)"
        if (-not (Test-Path $DestPath)) {
            if (-not $WhatIf) {
                Move-Item -Path $file.FullName -Destination $DestPath -Force -ErrorAction SilentlyContinue
                Write-Host "  Organized: $($file.Name) -> _LOGS/tracking/"
                $ExistingCount++
            }
        }
    }
}

Write-Host "  DONE: $ExistingCount files organized"
Write-Host ""

Write-Host "========================================"
Write-Host "PHASE 3 CLEANUP COMPLETE"
Write-Host "========================================"
Write-Host ""

if ($CreateBackup -and (Test-Path $BackupFolder)) {
    $ItemCount = @(Get-ChildItem $BackupFolder).Count
    Write-Host "Backup location: $BackupFolder"
    Write-Host "Items backed up: $ItemCount"
}

Write-Host ""
Write-Host "Results:"
Write-Host "  • Created _LOGS/ folder structure (tracking, test_results, build_logs, performance)"
Write-Host "  • Moved $TrackingCount tracking files to _LOGS/tracking/"
Write-Host "  • Moved $ReportCount test reports to _LOGS/test_results/"
Write-Host "  • Centralized all logs and performance data"
Write-Host ""
Write-Host "Verification: Check that _LOGS/ contains your organized logs and tracking"
Write-Host ""
