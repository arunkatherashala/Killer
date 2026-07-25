# Super Agent Status — see everything at a glance

param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

$AgentHome = Join-Path $ProjectRoot "_AGENT"
$HeartbeatFile = Join-Path $AgentHome "heartbeat.json"
$TaskFile = Join-Path $AgentHome "tasks.json"
$ConfigFile = Join-Path $AgentHome "config.json"
$LogDir = Join-Path $AgentHome "logs"
$CompletedDir = Join-Path $AgentHome "completed"

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║         KILLER SUPER AGENT — STATUS                  ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# ── Heartbeat ──
if (Test-Path $HeartbeatFile) {
    $hb = Get-Content $HeartbeatFile -Raw | ConvertFrom-Json
    $lastTick = [DateTime]::Parse($hb.last_tick)
    $age = (Get-Date) - $lastTick

    if ($age.TotalMinutes -lt 2) {
        Write-Host "  AGENT:   " -NoNewline; Write-Host "RUNNING" -ForegroundColor Green
    } elseif ($age.TotalMinutes -lt 10) {
        Write-Host "  AGENT:   " -NoNewline; Write-Host "SLOW (last tick $([math]::Round($age.TotalMinutes,1))m ago)" -ForegroundColor Yellow
    } else {
        Write-Host "  AGENT:   " -NoNewline; Write-Host "OFFLINE (last tick $([math]::Round($age.TotalMinutes,1))m ago)" -ForegroundColor Red
    }
    Write-Host "  PID:     $($hb.pid)"
    Write-Host "  Uptime:  $($hb.uptime_min) minutes"
    Write-Host "  Cycles:  $($hb.cycles)"
    Write-Host "  Activity: $($hb.activity)"
} else {
    Write-Host "  AGENT:   " -NoNewline; Write-Host "NEVER STARTED" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Start with: .\scripts\super-agent.ps1" -ForegroundColor Yellow
}

# ── Config ──
Write-Host ""
Write-Host "  ─── Configuration ───" -ForegroundColor Cyan
if (Test-Path $ConfigFile) {
    $cfg = Get-Content $ConfigFile -Raw | ConvertFrom-Json
    $features = @()
    if ($cfg.auto_build)     { $features += "Build(${($cfg.build_interval_min)}m)" }
    if ($cfg.system_monitor) { $features += "Monitor(${($cfg.monitor_interval_min)}m)" }
    if ($cfg.auto_cleanup)   { $features += "Cleanup(${($cfg.cleanup_interval_min)}m)" }
    if ($cfg.web_checks)     { $features += "WebCheck(${($cfg.web_check_interval_min)}m)" }
    Write-Host "  Active: $($features -join ' | ')" -ForegroundColor White
}

# ── Pending Tasks ──
Write-Host ""
Write-Host "  ─── Task Queue ───" -ForegroundColor Cyan
if (Test-Path $TaskFile) {
    try {
        $tasks = @(Get-Content $TaskFile -Raw | ConvertFrom-Json)
        $pending = @($tasks | Where-Object { $_.status -eq "pending" })
        if ($pending.Count -gt 0) {
            Write-Host "  Pending: $($pending.Count) task(s)" -ForegroundColor Yellow
            foreach ($t in $pending) {
                Write-Host "    - [$($t.type)] $($t.id)" -ForegroundColor Yellow
            }
        } else {
            Write-Host "  Queue: Empty (all done)" -ForegroundColor Green
        }
    } catch {
        Write-Host "  Queue: Empty" -ForegroundColor Green
    }
} else {
    Write-Host "  Queue: No task file yet"
}

# ── Completed Today ──
$today = Get-Date -Format "yyyy-MM-dd"
$archFile = Join-Path $CompletedDir "done_$today.json"
if (Test-Path $archFile) {
    $done = @(Get-Content $archFile -Raw | ConvertFrom-Json)
    $ok = @($done | Where-Object { $_.status -eq "completed" }).Count
    $fail = @($done | Where-Object { $_.status -eq "failed" }).Count
    Write-Host ""
    Write-Host "  ─── Today's Completed Tasks ───" -ForegroundColor Cyan
    Write-Host "  Done: $ok completed, $fail failed" -ForegroundColor $(if ($fail -gt 0) {"Yellow"} else {"Green"})
    foreach ($t in ($done | Select-Object -Last 10)) {
        $icon = if ($t.status -eq "completed") { "[OK]" } else { "[!!]" }
        $color = if ($t.status -eq "completed") { "Green" } else { "Red" }
        Write-Host "    $icon [$($t.type)] $($t.id)" -ForegroundColor $color
    }
}

# ── Recent Logs ──
Write-Host ""
Write-Host "  ─── Recent Activity ───" -ForegroundColor Cyan
$todayLog = Join-Path $LogDir "agent_$today.log"
if (Test-Path $todayLog) {
    Get-Content $todayLog | Select-Object -Last 10 | ForEach-Object {
        $color = "Gray"
        if ($_ -match "\[ERROR\]") { $color = "Red" }
        elseif ($_ -match "\[WARN\]")  { $color = "Yellow" }
        elseif ($_ -match "\[OK\]")    { $color = "Green" }
        Write-Host "  $_" -ForegroundColor $color
    }
} else {
    Write-Host "  No activity today yet."
}

Write-Host ""
Write-Host "  ─── Quick Commands ───" -ForegroundColor DarkGray
Write-Host "  Add task:    .\scripts\super-agent-add-task.ps1 -Type shell -Command 'dir'" -ForegroundColor DarkGray
Write-Host "  Full report: .\scripts\super-agent-add-task.ps1 -Type report" -ForegroundColor DarkGray
Write-Host "  Config:      notepad $ConfigFile" -ForegroundColor DarkGray
Write-Host ""
