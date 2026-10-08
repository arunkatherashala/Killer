# Killer installer for Windows:  irm <url>/install.ps1 | iex
# Downloads the release zip, verifies SHA256, installs to $HOME\.killer\bin.
param(
  [string]$Version = $(if ($env:KILLER_VERSION) { $env:KILLER_VERSION } else { "latest" }),
  [string]$Prefix = $(if ($env:KILLER_HOME) { $env:KILLER_HOME } else { Join-Path $HOME ".killer" }),
  [switch]$DryRun,
  [switch]$Help
)
$ErrorActionPreference = "Stop"
$Repo = if ($env:KILLER_REPO) { $env:KILLER_REPO } else { "arunkatherashala/Killer" }

if ($Help) {
  Write-Host "Usage: install.ps1 [-Version vX.Y.Z] [-Prefix DIR] [-DryRun] [-Help]"
  Write-Host "Installs killer_super.exe into DIR\bin (default: ~\.killer\bin)."
  Write-Host "Environment: KILLER_VERSION, KILLER_HOME, KILLER_REPO override the defaults."
  return
}

$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne "AMD64") { throw "no prebuilt binary for Windows $arch; build from source with cargo" }
$target = "windows-x64"

if ($Version -eq "latest") {
  if ($DryRun) { $Tag = "<latest tag>" }
  else {
    $rel = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Tag = $rel.tag_name
  }
} else { $Tag = $Version }

$name = "killer_super-$Tag-$target"
$base = "https://github.com/$Repo/releases/download/$Tag"
$binDir = Join-Path $Prefix "bin"
Write-Host "target:   $target"
Write-Host "tag:      $Tag"
Write-Host "archive:  $base/$name.zip"
Write-Host "checksum: $base/$name.zip.sha256"
Write-Host "install:  $binDir\killer_super.exe"
if ($DryRun) { Write-Host "dry run: nothing downloaded or installed"; return }

$work = Join-Path ([IO.Path]::GetTempPath()) ("killer-install-" + [guid]::NewGuid())
New-Item -ItemType Directory $work | Out-Null
try {
  Invoke-WebRequest "$base/$name.zip" -OutFile "$work\$name.zip"
  Invoke-WebRequest "$base/$name.zip.sha256" -OutFile "$work\$name.zip.sha256"
  $want = ((Get-Content "$work\$name.zip.sha256" -Raw).Trim() -split "\s+")[0].ToLower()
  $got = (Get-FileHash "$work\$name.zip" -Algorithm SHA256).Hash.ToLower()
  if ($want -ne $got) { throw "checksum mismatch: expected $want, got $got" }
  Write-Host "checksum ok"
  Expand-Archive "$work\$name.zip" -DestinationPath $work
  New-Item -ItemType Directory -Force $binDir | Out-Null
  Copy-Item "$work\$name\killer_super.exe" "$binDir\killer_super.exe" -Force
  Write-Host "installed $binDir\killer_super.exe"
  if (-not (($env:Path -split ";") -contains $binDir)) {
    Write-Host "add it to your PATH:  setx PATH `"$binDir;%PATH%`""
  }
} finally { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
