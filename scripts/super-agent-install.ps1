# Install Super Agent as a Windows Scheduled Task — runs 24/7 on VM
# RUN AS ADMINISTRATOR

param(
    [ValidateSet("install","uninstall","status")]
    [string]$Action = "install"
)

$TaskName = "KillerSuperAgent"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ScriptPath = Join-Path $PSScriptRoot "super-agent.ps1"

switch ($Action) {
    "install" {
        Write-Host ""
        Write-Host "  Installing Killer Super Agent as Windows Service..." -ForegroundColor Cyan
        Write-Host ""

        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

        $pwsh = (Get-Command powershell.exe).Source
        $arg = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$ScriptPath`""

        $taskAction  = New-ScheduledTaskAction -Execute $pwsh -Argument $arg -WorkingDirectory $ProjectRoot
        $taskTrigger = New-ScheduledTaskTrigger -AtStartup
        $taskSettings = New-ScheduledTaskSettingsSet `
            -AllowStartIfOnBatteries `
            -DontStopIfGoingOnBatteries `
            -StartWhenAvailable `
            -RestartCount 999 `
            -RestartInterval (New-TimeSpan -Minutes 1) `
            -ExecutionTimeLimit (New-TimeSpan -Days 9999)

        $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType S4U -RunLevel Highest

        Register-ScheduledTask `
            -TaskName $TaskName `
            -Action $taskAction `
            -Trigger $taskTrigger `
            -Settings $taskSettings `
            -Principal $principal `
            -Description "Killer Super Agent — 24/7 build, test, monitor, task processor"

        Start-ScheduledTask -TaskName $TaskName

        Write-Host "  ╔══════════════════════════════════════════════╗" -ForegroundColor Green
        Write-Host "  ║  SUPER AGENT INSTALLED & RUNNING!             ║" -ForegroundColor Green
        Write-Host "  ╚══════════════════════════════════════════════╝" -ForegroundColor Green
        Write-Host ""
        Write-Host "  It will:"
        Write-Host "    - Start automatically when VM boots"
        Write-Host "    - Restart if it crashes"
        Write-Host "    - Build & test your Killer language"
        Write-Host "    - Monitor CPU, RAM, disk"
        Write-Host "    - Check URLs you add to watchlist"
        Write-Host "    - Process any tasks you add to the queue"
        Write-Host "    - Clean up old files"
        Write-Host "    - Generate daily reports"
        Write-Host ""
        Write-Host "  Check status:  .\scripts\super-agent-status.ps1" -ForegroundColor Yellow
        Write-Host "  Add tasks:     .\scripts\super-agent-add-task.ps1" -ForegroundColor Yellow
        Write-Host "  Config:        _AGENT\config.json" -ForegroundColor Yellow
        Write-Host ""
    }

    "uninstall" {
        Write-Host "  Removing Super Agent..." -ForegroundColor Yellow
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
        Write-Host "  Removed." -ForegroundColor Green
    }

    "status" {
        $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        if ($task) {
            Write-Host ""
            Write-Host "  Task: $TaskName" -ForegroundColor Cyan
            Write-Host "  State: $($task.State)"
            $info = Get-ScheduledTaskInfo -TaskName $TaskName
            Write-Host "  Last Run: $($info.LastRunTime)"
            Write-Host "  Next Run: $($info.NextRunTime)"
            Write-Host "  Result:   $($info.LastTaskResult)"
            Write-Host ""
        } else {
            Write-Host "  Not installed." -ForegroundColor Red
            Write-Host "  Install: .\scripts\super-agent-install.ps1 -Action install"
        }
    }
}
