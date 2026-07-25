# Killer Agent — Install as Windows Scheduled Task (runs 24/7)
# RUN THIS AS ADMINISTRATOR on your VM

param(
    [ValidateSet("install","uninstall","status")]
    [string]$Action = "install",
    [int]$IntervalMinutes = 5
)

$TaskName = "KillerAutoBuilder"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ScriptPath = Join-Path $PSScriptRoot "agent-autobuilder.ps1"

switch ($Action) {
    "install" {
        Write-Host "Installing Killer 24/7 Agent as Scheduled Task..." -ForegroundColor Cyan

        # Remove existing if any
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

        $pwsh = (Get-Command powershell.exe).Source
        $arg = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$ScriptPath`" -IntervalSeconds $($IntervalMinutes * 60)"

        $taskAction  = New-ScheduledTaskAction -Execute $pwsh -Argument $arg -WorkingDirectory $ProjectRoot
        $taskTrigger = New-ScheduledTaskTrigger -AtStartup
        $taskSettings = New-ScheduledTaskSettingsSet `
            -AllowStartIfOnBatteries `
            -DontStopIfGoingOnBatteries `
            -StartWhenAvailable `
            -RestartCount 3 `
            -RestartInterval (New-TimeSpan -Minutes 1) `
            -ExecutionTimeLimit (New-TimeSpan -Days 365)

        $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType S4U -RunLevel Highest

        Register-ScheduledTask `
            -TaskName $TaskName `
            -Action $taskAction `
            -Trigger $taskTrigger `
            -Settings $taskSettings `
            -Principal $principal `
            -Description "Killer Language 24/7 auto-builder and test runner"

        # Also start it immediately
        Start-ScheduledTask -TaskName $TaskName

        Write-Host ""
        Write-Host "INSTALLED!" -ForegroundColor Green
        Write-Host "  Task Name:  $TaskName"
        Write-Host "  Interval:   every ${IntervalMinutes} minutes"
        Write-Host "  Starts at:  system boot + right now"
        Write-Host "  Logs at:    _LOGS\autobuilder\"
        Write-Host ""
        Write-Host "Check status:  .\scripts\agent-dashboard.ps1"
        Write-Host "Uninstall:     .\scripts\agent-install-service.ps1 -Action uninstall"
    }

    "uninstall" {
        Write-Host "Removing Killer 24/7 Agent..." -ForegroundColor Yellow
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
        Write-Host "Removed." -ForegroundColor Green
    }

    "status" {
        $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        if ($task) {
            Write-Host "Task: $TaskName" -ForegroundColor Cyan
            Write-Host "State: $($task.State)"
            $info = Get-ScheduledTaskInfo -TaskName $TaskName
            Write-Host "Last Run: $($info.LastRunTime)"
            Write-Host "Next Run: $($info.NextRunTime)"
            Write-Host "Last Result: $($info.LastTaskResult)"
        } else {
            Write-Host "Task not installed." -ForegroundColor Red
            Write-Host "Install with: .\scripts\agent-install-service.ps1 -Action install"
        }
    }
}
