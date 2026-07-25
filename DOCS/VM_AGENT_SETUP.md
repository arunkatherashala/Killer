# Killer 24/7 Agent — VM Setup Guide

## What You Get

| Script | What It Does |
|--------|-------------|
| `agent-autobuilder.ps1` | Builds & tests every N seconds in a loop |
| `agent-watcher.ps1` | Watches .rs files, rebuilds instantly on save |
| `agent-install-service.ps1` | Installs as Windows Scheduled Task (survives reboots) |
| `agent-dashboard.ps1` | Shows build history and current status |
| `agent-full-test.ps1` | Runs full test suite and saves a report |

## Quick Start (on your VM)

### Step 1: Copy the project to your VM

Copy the entire `killer` folder to your VM.

### Step 2: Make sure Rust is installed

```powershell
rustup --version
cargo --version
```

If not installed: https://rustup.rs/

### Step 3: Start the auto-builder (manual)

```powershell
cd C:\path\to\killer
.\scripts\agent-autobuilder.ps1
```

This runs forever, building and testing every 60 seconds. Leave this terminal open.

### Step 4: Install as a Windows Service (runs even after reboot)

Open **PowerShell as Administrator**:

```powershell
cd C:\path\to\killer
.\scripts\agent-install-service.ps1 -Action install -IntervalMinutes 5
```

Now it runs at boot, every 5 minutes, even when you're not logged in.

### Step 5: Check status anytime

```powershell
.\scripts\agent-dashboard.ps1
```

## How It Works

```
You sleep → VM stays on → auto-builder runs every N minutes
                         → detects .rs changes → rebuilds
                         → runs cargo test → logs results
                         → you wake up → run dashboard → see what happened
```

## File Locations

| What | Where |
|------|-------|
| Build logs | `_LOGS/autobuilder/cycle_*.log` |
| Status file | `_LOGS/autobuilder/status.json` |
| Build history | `_LOGS/autobuilder/build_history.csv` |
| Test reports | `test_reports/report_*.txt` |

## Commands Reference

```powershell
# Run auto-builder manually (60s interval)
.\scripts\agent-autobuilder.ps1

# Run with custom interval (5 minutes)
.\scripts\agent-autobuilder.ps1 -IntervalSeconds 300

# Watch files for instant rebuild
.\scripts\agent-watcher.ps1

# Install as 24/7 service
.\scripts\agent-install-service.ps1 -Action install

# Check service status
.\scripts\agent-install-service.ps1 -Action status

# Remove service
.\scripts\agent-install-service.ps1 -Action uninstall

# Run full test suite
.\scripts\agent-full-test.ps1

# Check dashboard
.\scripts\agent-dashboard.ps1
```

## Troubleshooting

**"Script not allowed to run"**
```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

**"cargo not found"**
Make sure Rust is installed and in PATH. Restart terminal after installing.

**Dashboard shows "NOT RUNNING"**
The auto-builder hasn't been started yet. Run `agent-autobuilder.ps1` or install the service.
