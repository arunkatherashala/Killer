# Killer File Watcher — triggers rebuild when .rs files change
# Uses FileSystemWatcher for instant response (no polling)

param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot),
    [int]$CooldownSeconds = 5
)

$RustSrc = Join-Path $ProjectRoot "SOURCE\src\v2-rust\killer\src"
$LogDir  = Join-Path $ProjectRoot "_LOGS\autobuilder"
$RustProject = Join-Path $ProjectRoot "SOURCE\src\v2-rust\killer"

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

$script:LastBuild = [DateTime]::MinValue

function Invoke-Build {
    $now = Get-Date
    if (($now - $script:LastBuild).TotalSeconds -lt $CooldownSeconds) { return }
    $script:LastBuild = $now

    $ts = Get-Date -Format "HH:mm:ss"
    Write-Host "[$ts] Change detected — rebuilding..." -ForegroundColor Yellow

    Push-Location $RustProject
    $buildOut = & cargo build --release 2>&1 | Out-String
    $buildOk = $LASTEXITCODE -eq 0

    if ($buildOk) {
        Write-Host "[$ts] Build OK" -ForegroundColor Green
        $testOut = & cargo test --lib 2>&1 | Out-String
        if ($testOut -match "test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored") {
            $color = if ([int]$Matches[2] -eq 0) { "Green" } else { "Red" }
            Write-Host "[$ts] Tests: $($Matches[1]) passed, $($Matches[2]) failed, $($Matches[3]) ignored" -ForegroundColor $color
        }
    } else {
        Write-Host "[$ts] BUILD FAILED" -ForegroundColor Red
        $lines = $buildOut -split "`n" | Where-Object { $_ -match "^error" } | Select-Object -First 5
        $lines | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    }
    Pop-Location
}

Write-Host "=== Killer File Watcher ==="
Write-Host "Watching: $RustSrc"
Write-Host "Cooldown: ${CooldownSeconds}s between builds"
Write-Host "Press Ctrl+C to stop."
Write-Host ""

$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = $RustSrc
$watcher.Filter = "*.rs"
$watcher.IncludeSubdirectories = $true
$watcher.EnableRaisingEvents = $true
$watcher.NotifyFilter = [System.IO.NotifyFilters]::LastWrite -bor [System.IO.NotifyFilters]::FileName

$action = {
    # Signal the main loop
    $Global:FileChanged = $true
}

Register-ObjectEvent $watcher "Changed" -Action $action | Out-Null
Register-ObjectEvent $watcher "Created" -Action $action | Out-Null
Register-ObjectEvent $watcher "Renamed" -Action $action | Out-Null

$Global:FileChanged = $false

try {
    while ($true) {
        if ($Global:FileChanged) {
            $Global:FileChanged = $false
            Start-Sleep -Milliseconds 500
            Invoke-Build
        }
        Start-Sleep -Milliseconds 200
    }
} finally {
    $watcher.EnableRaisingEvents = $false
    $watcher.Dispose()
    Get-EventSubscriber | Unregister-Event
    Write-Host "`nWatcher stopped."
}
