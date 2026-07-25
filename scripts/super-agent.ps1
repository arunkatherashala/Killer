# ╔══════════════════════════════════════════════════════════════╗
# ║           KILLER SUPER AGENT — 24/7 ASSISTANT              ║
# ║  Build · Test · Monitor · Clean · Check · Report · Guard   ║
# ╚══════════════════════════════════════════════════════════════╝
#
# This is your always-on assistant. It runs on your VM and does
# everything: builds code, monitors your system, cleans up files,
# checks websites, processes your task queue, and reports back.

param(
    [int]$TickSeconds = 30,
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

# ── Paths ──
$AgentHome    = Join-Path $ProjectRoot "_AGENT"
$LogDir       = Join-Path $AgentHome "logs"
$TaskFile     = Join-Path $AgentHome "tasks.json"
$CompletedDir = Join-Path $AgentHome "completed"
$ReportDir    = Join-Path $AgentHome "reports"
$ConfigFile   = Join-Path $AgentHome "config.json"
$HeartbeatFile = Join-Path $AgentHome "heartbeat.json"
$RustProject  = Join-Path $ProjectRoot "SOURCE\src\v2-rust\killer"

foreach ($d in @($AgentHome, $LogDir, $CompletedDir, $ReportDir)) {
    New-Item -ItemType Directory -Force -Path $d | Out-Null
}

# ── Default Config ──
if (-not (Test-Path $ConfigFile)) {
    @{
        auto_build        = $true
        build_interval_min = 5
        system_monitor    = $true
        monitor_interval_min = 2
        auto_cleanup      = $true
        cleanup_interval_min = 60
        web_checks        = $true
        web_check_interval_min = 10
        professor_review  = $true
        professor_interval_min = 30
        professor_autofix = $true
        max_log_age_days  = 7
        notify_on_failure = $true
    } | ConvertTo-Json -Depth 5 | Out-File $ConfigFile -Encoding utf8
}

# ── Default Task Queue ──
if (-not (Test-Path $TaskFile)) {
    @(
        @{ id = "example-1"; type = "remind"; data = @{ message = "Super Agent is alive!" }; status = "pending"; created = (Get-Date -Format o) }
    ) | ConvertTo-Json -Depth 5 | Out-File $TaskFile -Encoding utf8
}

$Config = Get-Content $ConfigFile -Raw | ConvertFrom-Json

# ── Logging ──
function Write-Log {
    param([string]$Category, [string]$Message, [string]$Level = "INFO")
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $line = "[$ts] [$Level] [$Category] $Message"
    $dailyLog = Join-Path $LogDir "agent_$(Get-Date -Format 'yyyy-MM-dd').log"
    $line | Out-File $dailyLog -Append -Encoding utf8
    $color = switch ($Level) { "ERROR" { "Red" } "WARN" { "Yellow" } "OK" { "Green" } default { "Gray" } }
    Write-Host $line -ForegroundColor $color
}

function Update-Heartbeat {
    param([string]$Activity)
    @{
        alive      = $true
        pid        = $PID
        last_tick  = (Get-Date -Format o)
        activity   = $Activity
        uptime_min = [math]::Round(((Get-Date) - $script:BootTime).TotalMinutes, 1)
        cycles     = $script:TickCount
    } | ConvertTo-Json | Out-File $HeartbeatFile -Encoding utf8
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 1: BUILD & TEST
# ══════════════════════════════════════════════════════════════
$script:LastBuildTime = [DateTime]::MinValue

function Invoke-AutoBuild {
    if (-not $Config.auto_build) { return }
    if (((Get-Date) - $script:LastBuildTime).TotalMinutes -lt $Config.build_interval_min) { return }
    $script:LastBuildTime = Get-Date

    Write-Log "BUILD" "Starting release build..."
    Push-Location $RustProject
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $out = & cargo build --release 2>&1 | Out-String
    $buildOk = $LASTEXITCODE -eq 0
    $sw.Stop()

    if ($buildOk) {
        Write-Log "BUILD" "Build OK ($($sw.ElapsedMilliseconds)ms)" "OK"
        $testOut = & cargo test --lib 2>&1 | Out-String
        if ($testOut -match "(\d+) passed; (\d+) failed; (\d+) ignored") {
            $p = $Matches[1]; $f = $Matches[2]; $ig = $Matches[3]
            $lvl = if ([int]$f -eq 0) { "OK" } else { "WARN" }
            Write-Log "TEST" "$p passed, $f failed, $ig ignored" $lvl
        }
    } else {
        Write-Log "BUILD" "BUILD FAILED" "ERROR"
        $errLines = ($out -split "`n" | Where-Object { $_ -match "^error" }) -join "; "
        Write-Log "BUILD" $errLines "ERROR"
    }
    Pop-Location
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 2: SYSTEM MONITOR
# ══════════════════════════════════════════════════════════════
$script:LastMonitorTime = [DateTime]::MinValue

function Invoke-SystemMonitor {
    if (-not $Config.system_monitor) { return }
    if (((Get-Date) - $script:LastMonitorTime).TotalMinutes -lt $Config.monitor_interval_min) { return }
    $script:LastMonitorTime = Get-Date

    # Disk
    $drives = Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Used -gt 0 }
    foreach ($drv in $drives) {
        $total = $drv.Used + $drv.Free
        $pctFree = [math]::Round(($drv.Free / $total) * 100, 1)
        $freeGB = [math]::Round($drv.Free / 1GB, 1)
        if ($pctFree -lt 10) {
            Write-Log "DISK" "CRITICAL: Drive $($drv.Name): only ${freeGB}GB free (${pctFree}%)" "ERROR"
        } elseif ($pctFree -lt 25) {
            Write-Log "DISK" "WARNING: Drive $($drv.Name): ${freeGB}GB free (${pctFree}%)" "WARN"
        }
    }

    # Memory
    $os = Get-CimInstance Win32_OperatingSystem
    $totalMem = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
    $freeMem = [math]::Round($os.FreePhysicalMemory / 1MB, 1)
    $usedPct = [math]::Round((1 - $freeMem / $totalMem) * 100, 1)
    if ($usedPct -gt 90) {
        Write-Log "MEM" "CRITICAL: ${usedPct}% memory used (${freeMem}GB free of ${totalMem}GB)" "ERROR"
    } elseif ($usedPct -gt 75) {
        Write-Log "MEM" "High memory: ${usedPct}% used" "WARN"
    }

    # CPU
    $cpu = (Get-CimInstance Win32_Processor).LoadPercentage
    if ($cpu -gt 90) {
        Write-Log "CPU" "CRITICAL: CPU at ${cpu}%" "ERROR"
    } elseif ($cpu -gt 70) {
        Write-Log "CPU" "High CPU: ${cpu}%" "WARN"
    }

    # Top processes by memory
    $topProcs = Get-Process | Sort-Object WorkingSet64 -Descending | Select-Object -First 3
    $procStr = ($topProcs | ForEach-Object { "$($_.Name)=$([math]::Round($_.WorkingSet64/1MB))MB" }) -join ", "
    Write-Log "PROCS" "Top 3: $procStr"

    # Save snapshot
    $snapshot = @{
        timestamp = (Get-Date -Format o)
        cpu_pct   = $cpu
        mem_used_pct = $usedPct
        mem_free_gb = $freeMem
        drives = @($drives | ForEach-Object { @{ name = $_.Name; free_gb = [math]::Round($_.Free/1GB,1) } })
    }
    $snapshotFile = Join-Path $ReportDir "system_$(Get-Date -Format 'yyyy-MM-dd').json"
    if (Test-Path $snapshotFile) {
        $existing = Get-Content $snapshotFile -Raw | ConvertFrom-Json
        $arr = @($existing) + @($snapshot)
    } else {
        $arr = @($snapshot)
    }
    $arr | ConvertTo-Json -Depth 5 | Out-File $snapshotFile -Encoding utf8
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 3: WEB / URL HEALTH CHECKER
# ══════════════════════════════════════════════════════════════
$script:LastWebCheckTime = [DateTime]::MinValue

function Invoke-WebChecks {
    if (-not $Config.web_checks) { return }
    if (((Get-Date) - $script:LastWebCheckTime).TotalMinutes -lt $Config.web_check_interval_min) { return }
    $script:LastWebCheckTime = Get-Date

    $urlFile = Join-Path $AgentHome "watchlist_urls.txt"
    if (-not (Test-Path $urlFile)) {
        @(
            "# Add URLs to monitor (one per line)",
            "# Example:",
            "# https://github.com"
        ) | Out-File $urlFile -Encoding utf8
        return
    }

    $urls = Get-Content $urlFile | Where-Object { $_ -match "^https?://" }
    foreach ($url in $urls) {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $resp = Invoke-WebRequest -Uri $url -TimeoutSec 10 -UseBasicParsing -ErrorAction Stop
            $sw.Stop()
            $code = $resp.StatusCode
            if ($code -eq 200) {
                Write-Log "WEB" "$url -> $code ($($sw.ElapsedMilliseconds)ms)" "OK"
            } else {
                Write-Log "WEB" "$url -> $code ($($sw.ElapsedMilliseconds)ms)" "WARN"
            }
        } catch {
            Write-Log "WEB" "$url -> FAILED: $($_.Exception.Message)" "ERROR"
        }
    }
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 4: AUTO CLEANUP
# ══════════════════════════════════════════════════════════════
$script:LastCleanupTime = [DateTime]::MinValue

function Invoke-AutoCleanup {
    if (-not $Config.auto_cleanup) { return }
    if (((Get-Date) - $script:LastCleanupTime).TotalMinutes -lt $Config.cleanup_interval_min) { return }
    $script:LastCleanupTime = Get-Date

    $removed = 0

    # Old agent logs
    $cutoff = (Get-Date).AddDays(-$Config.max_log_age_days)
    Get-ChildItem $LogDir -Filter "*.log" | Where-Object { $_.LastWriteTime -lt $cutoff } | ForEach-Object {
        Remove-Item $_.FullName -Force; $removed++
    }

    # Rust build cache older than 7 days
    $targetDir = Join-Path $RustProject "target\debug\incremental"
    if (Test-Path $targetDir) {
        $oldDirs = Get-ChildItem $targetDir -Directory | Where-Object { $_.LastWriteTime -lt $cutoff }
        foreach ($d in $oldDirs) {
            Remove-Item $d.FullName -Recurse -Force -ErrorAction SilentlyContinue; $removed++
        }
    }

    # Temp files in project root
    Get-ChildItem $ProjectRoot -Filter "_tmp_*" -File | Where-Object { $_.LastWriteTime -lt $cutoff } | ForEach-Object {
        Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue; $removed++
    }

    if ($removed -gt 0) {
        Write-Log "CLEAN" "Removed $removed old files/dirs" "OK"
    }
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 5: TASK QUEUE PROCESSOR
# ══════════════════════════════════════════════════════════════

function Invoke-TaskQueue {
    if (-not (Test-Path $TaskFile)) { return }

    try {
        $tasks = Get-Content $TaskFile -Raw | ConvertFrom-Json
    } catch { return }

    if (-not $tasks) { return }
    $changed = $false

    foreach ($task in $tasks) {
        if ($task.status -ne "pending") { continue }
        $changed = $true
        $task.status = "running"

        Write-Log "TASK" "Processing task $($task.id) [$($task.type)]"

        try {
            switch ($task.type) {
                "shell" {
                    $result = Invoke-Expression $task.data.command 2>&1 | Out-String
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue $result -Force
                    Write-Log "TASK" "Shell task done: $($task.id)" "OK"
                }
                "build" {
                    Push-Location $RustProject
                    $result = & cargo build --release 2>&1 | Out-String
                    Pop-Location
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue $(if ($LASTEXITCODE -eq 0) {"OK"} else {"FAIL: $result"}) -Force
                    Write-Log "TASK" "Build task: $(if ($LASTEXITCODE -eq 0) {'OK'} else {'FAILED'})" $(if ($LASTEXITCODE -eq 0) {"OK"} else {"ERROR"})
                }
                "test" {
                    Push-Location $RustProject
                    $result = & cargo test --lib 2>&1 | Out-String
                    Pop-Location
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue $result -Force
                    Write-Log "TASK" "Test task done" "OK"
                }
                "download" {
                    $dest = Join-Path $AgentHome "downloads"
                    New-Item -ItemType Directory -Force -Path $dest | Out-Null
                    $outFile = Join-Path $dest (Split-Path $task.data.url -Leaf)
                    Invoke-WebRequest -Uri $task.data.url -OutFile $outFile -UseBasicParsing
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Downloaded to $outFile" -Force
                    Write-Log "TASK" "Downloaded $($task.data.url)" "OK"
                }
                "remind" {
                    Write-Log "REMIND" $task.data.message "OK"
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Displayed" -Force
                }
                "cleanup" {
                    $target = $task.data.path
                    if (Test-Path $target) {
                        $count = (Get-ChildItem $target -Recurse -File).Count
                        if ($task.data.delete -eq $true) {
                            Remove-Item $target -Recurse -Force
                            $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Deleted $count files" -Force
                        } else {
                            $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Found $count files (dry run)" -Force
                        }
                    }
                    Write-Log "TASK" "Cleanup task done: $($task.id)" "OK"
                }
                "copy" {
                    Copy-Item -Path $task.data.source -Destination $task.data.dest -Recurse -Force
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Copied" -Force
                    Write-Log "TASK" "Copy: $($task.data.source) -> $($task.data.dest)" "OK"
                }
                "report" {
                    & (Join-Path $PSScriptRoot "agent-full-test.ps1") -ProjectRoot $ProjectRoot
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Report generated" -Force
                    Write-Log "TASK" "Full report generated" "OK"
                }
                "professor" {
                    $profScript = Join-Path $PSScriptRoot "professor-agent.ps1"
                    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $profScript -ProjectRoot $ProjectRoot -AutoFix 2>&1 | Out-Null
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Professor review completed" -Force
                    Write-Log "TASK" "Professor review triggered" "OK"
                }
                "watch_file" {
                    $exists = Test-Path $task.data.path
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Exists: $exists" -Force
                    Write-Log "TASK" "File watch: $($task.data.path) exists=$exists"
                }
                default {
                    Write-Log "TASK" "Unknown task type: $($task.type)" "WARN"
                    $task | Add-Member -NotePropertyName "result" -NotePropertyValue "Unknown type" -Force
                }
            }
            $task.status = "completed"
        } catch {
            $task.status = "failed"
            $task | Add-Member -NotePropertyName "error" -NotePropertyValue $_.Exception.Message -Force
            Write-Log "TASK" "Task $($task.id) failed: $($_.Exception.Message)" "ERROR"
        }

        $task | Add-Member -NotePropertyName "completed_at" -NotePropertyValue (Get-Date -Format o) -Force
    }

    if ($changed) {
        $completed = @($tasks | Where-Object { $_.status -in @("completed","failed") })
        $remaining = @($tasks | Where-Object { $_.status -eq "pending" })

        # Archive completed
        $archFile = Join-Path $CompletedDir "done_$(Get-Date -Format 'yyyy-MM-dd').json"
        if (Test-Path $archFile) {
            $existing = Get-Content $archFile -Raw | ConvertFrom-Json
            $completed = @($existing) + $completed
        }
        $completed | ConvertTo-Json -Depth 5 | Out-File $archFile -Encoding utf8

        # Keep only pending in queue
        if ($remaining.Count -eq 0) {
            "[]" | Out-File $TaskFile -Encoding utf8
        } else {
            $remaining | ConvertTo-Json -Depth 5 | Out-File $TaskFile -Encoding utf8
        }
    }
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 6: PROFESSOR REVIEW (PROACTIVE AUTO-THINKING)
# ══════════════════════════════════════════════════════════════
$script:LastProfessorTime = [DateTime]::MinValue

function Invoke-ProfessorReview {
    if (-not $Config.professor_review) { return }
    if (((Get-Date) - $script:LastProfessorTime).TotalMinutes -lt $Config.professor_interval_min) { return }
    $script:LastProfessorTime = Get-Date

    Write-Log "PROFESSOR" "Starting proactive code review..."
    $profScript = Join-Path $PSScriptRoot "professor-agent.ps1"
    if (Test-Path $profScript) {
        $args = @("-File", $profScript, "-ProjectRoot", $ProjectRoot)
        if ($Config.professor_autofix) { $args += "-AutoFix" }
        $out = & powershell.exe -NoProfile -ExecutionPolicy Bypass @args 2>&1 | Out-String

        if ($out -match "GRADE:\s*(\S+)") {
            $grade = $Matches[1]
            Write-Log "PROFESSOR" "Review complete — Grade: $grade" $(if ($grade -match "A") {"OK"} elseif ($grade -match "B|C") {"WARN"} else {"ERROR"})
        } else {
            Write-Log "PROFESSOR" "Review completed" "OK"
        }
    } else {
        Write-Log "PROFESSOR" "professor-agent.ps1 not found" "ERROR"
    }
}

# ══════════════════════════════════════════════════════════════
# CAPABILITY 7: DAILY SUMMARY REPORT
# ══════════════════════════════════════════════════════════════
$script:LastReportDate = ""

function Invoke-DailyReport {
    $today = Get-Date -Format "yyyy-MM-dd"
    if ($script:LastReportDate -eq $today) { return }
    if ((Get-Date).Hour -lt 1) { return }
    $script:LastReportDate = $today

    $report = @()
    $report += "╔═══════════════════════════════════════════════╗"
    $report += "║  KILLER SUPER AGENT — DAILY REPORT            ║"
    $report += "║  $today                                       ║"
    $report += "╚═══════════════════════════════════════════════╝"
    $report += ""

    # System
    $os = Get-CimInstance Win32_OperatingSystem
    $uptime = (Get-Date) - $os.LastBootUpTime
    $report += "SYSTEM: Up $([math]::Round($uptime.TotalHours,1))h | RAM $([math]::Round(($os.TotalVisibleMemorySize - $os.FreePhysicalMemory)/1MB,1))GB used"

    # Disk
    Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Used -gt 0 } | ForEach-Object {
        $report += "DISK $($_.Name): $([math]::Round($_.Free/1GB,1))GB free"
    }

    # Tasks completed today
    $archFile = Join-Path $CompletedDir "done_$today.json"
    if (Test-Path $archFile) {
        $done = Get-Content $archFile -Raw | ConvertFrom-Json
        $report += "TASKS: $($done.Count) completed today"
    }

    $report += ""
    $reportFile = Join-Path $ReportDir "daily_$today.txt"
    $report | Out-File $reportFile -Encoding utf8
    Write-Log "REPORT" "Daily report saved: $reportFile" "OK"
}

# ══════════════════════════════════════════════════════════════
# MAIN LOOP
# ══════════════════════════════════════════════════════════════
$script:BootTime = Get-Date
$script:TickCount = 0

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║         KILLER SUPER AGENT — ONLINE                  ║" -ForegroundColor Cyan
Write-Host "  ║  Build · Test · Monitor · Professor · Clean · Report  ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Project:  $ProjectRoot" -ForegroundColor White
Write-Host "  Agent:    $AgentHome" -ForegroundColor White
Write-Host "  Tick:     every ${TickSeconds}s" -ForegroundColor White
Write-Host "  Tasks:    $TaskFile" -ForegroundColor White
Write-Host "  Config:   $ConfigFile" -ForegroundColor White
Write-Host ""
Write-Host "  Add tasks to $TaskFile — agent picks them up automatically." -ForegroundColor Yellow
Write-Host "  Press Ctrl+C to stop." -ForegroundColor DarkGray
Write-Host ""

try {
    while ($true) {
        $script:TickCount++
        Update-Heartbeat "tick $($script:TickCount)"

        Invoke-TaskQueue
        Invoke-AutoBuild
        Invoke-SystemMonitor
        Invoke-ProfessorReview
        Invoke-WebChecks
        Invoke-AutoCleanup
        Invoke-DailyReport

        Start-Sleep -Seconds $TickSeconds
    }
} finally {
    Update-Heartbeat "stopped"
    Write-Log "AGENT" "Super Agent stopped" "WARN"
    Write-Host "`nSuper Agent stopped." -ForegroundColor Yellow
}
