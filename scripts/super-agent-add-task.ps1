# Add a task to the Super Agent queue
# The agent will pick it up and execute it automatically
#
# USAGE:
#   .\super-agent-add-task.ps1 -Type shell   -Command "Get-Date"
#   .\super-agent-add-task.ps1 -Type build
#   .\super-agent-add-task.ps1 -Type test
#   .\super-agent-add-task.ps1 -Type report
#   .\super-agent-add-task.ps1 -Type download -Url "https://example.com/file.zip"
#   .\super-agent-add-task.ps1 -Type remind   -Message "Deploy at 5pm"
#   .\super-agent-add-task.ps1 -Type copy     -Source "C:\a" -Dest "C:\b"
#   .\super-agent-add-task.ps1 -Type cleanup  -Path "C:\temp" [-Delete]

param(
    [Parameter(Mandatory)]
    [ValidateSet("shell","build","test","report","download","remind","cleanup","copy","watch_file","professor")]
    [string]$Type,

    [string]$Command,
    [string]$Url,
    [string]$Message,
    [string]$Source,
    [string]$Dest,
    [string]$Path,
    [switch]$Delete
)

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$TaskFile = Join-Path $ProjectRoot "_AGENT\tasks.json"

New-Item -ItemType Directory -Force -Path (Split-Path $TaskFile) | Out-Null

if (Test-Path $TaskFile) {
    try { $tasks = @(Get-Content $TaskFile -Raw | ConvertFrom-Json) } catch { $tasks = @() }
} else {
    $tasks = @()
}

$data = @{}
switch ($Type) {
    "shell"      { $data.command = $Command }
    "download"   { $data.url = $Url }
    "remind"     { $data.message = $Message }
    "copy"       { $data.source = $Source; $data.dest = $Dest }
    "cleanup"    { $data.path = $Path; $data.delete = [bool]$Delete }
    "watch_file" { $data.path = $Path }
}

$id = "task-$(Get-Date -Format 'yyyyMMdd-HHmmss')-$([guid]::NewGuid().ToString().Substring(0,4))"

$newTask = @{
    id      = $id
    type    = $Type
    data    = $data
    status  = "pending"
    created = (Get-Date -Format o)
}

$tasks += $newTask
$tasks | ConvertTo-Json -Depth 5 | Out-File $TaskFile -Encoding utf8

Write-Host ""
Write-Host "  Task added!" -ForegroundColor Green
Write-Host "  ID:   $id"
Write-Host "  Type: $Type"
Write-Host "  The Super Agent will pick this up on its next tick."
Write-Host ""
