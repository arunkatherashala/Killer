# Killer installer for Windows:  irm <url>/install.ps1 | iex
# Downloads the release zip, verifies SHA256, installs to $HOME\.killer\bin.
#
# Integrity: the zip's .sha256 comes from the same place as the zip, so on its own it only catches corruption.
# For an independent check the installer also verifies the zip against a checksum that does NOT come from the release
# assets: -Sha256 HEX (copy it from the release notes or from checksums/<tag>.sha256 in the repository), or the
# repository's checksums/<tag>.sha256 file fetched from raw.githubusercontent.com. See
# SOURCE/docs/project/SECURITY.md ("Download and package integrity") for what this does and does not protect.
param(
  [string]$Version = $(if ($env:KILLER_VERSION) { $env:KILLER_VERSION } else { "latest" }),
  [string]$Prefix = $(if ($env:KILLER_HOME) { $env:KILLER_HOME } else { Join-Path $HOME ".killer" }),
  [string]$Sha256 = $(if ($env:KILLER_SHA256) { $env:KILLER_SHA256 } else { "" }),
  [string]$ChecksumsUrl = $(if ($env:KILLER_CHECKSUMS_URL) { $env:KILLER_CHECKSUMS_URL } else { "" }),
  [switch]$RequirePinned,
  [switch]$DryRun,
  [switch]$Help
)
$ErrorActionPreference = "Stop"
$Repo = if ($env:KILLER_REPO) { $env:KILLER_REPO } else { "arunkatherashala/Killer" }

if ($Help) {
  Write-Host "Usage: install.ps1 [-Version vX.Y.Z] [-Prefix DIR] [-Sha256 HEX] [-RequirePinned] [-DryRun] [-Help]"
  Write-Host "Installs killer.exe and killer_super.exe into DIR\bin (default: ~\.killer\bin)."
  Write-Host "  -Sha256 HEX      expected SHA-256 of the zip from an independent source (release notes or"
  Write-Host "                   checksums/<tag>.sha256 in the repository); the install fails if the zip differs"
  Write-Host "  -RequirePinned   fail unless an independent checksum (-Sha256 or the repository file) was checked"
  Write-Host "Environment: KILLER_VERSION, KILLER_HOME, KILLER_REPO override the defaults; KILLER_SHA256,"
  Write-Host "KILLER_REQUIRE_PINNED=1 and KILLER_CHECKSUMS_URL (a file of 'sha256  archive-name' lines) control the"
  Write-Host "independent checksum."
  return
}

if ($env:KILLER_REQUIRE_PINNED -eq "1") { $RequirePinned = [switch]::Present }
$Sha256 = $Sha256.Trim().ToLower()
if ($Sha256 -and $Sha256 -notmatch '^[0-9a-f]{64}$') { throw "invalid -Sha256: expected 64 hex digits" }

$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne "AMD64") { throw "no prebuilt binary for Windows $arch; build from source with cargo" }
$target = "windows-x64"

# Tags end up in URLs and file names: only [A-Za-z0-9._-] is accepted.
function Test-Tag([string]$t) { return ($t -match '^[A-Za-z0-9][A-Za-z0-9._-]*$') -and ($t -notmatch '\.\.') }
if ($Version -ne "latest" -and -not (Test-Tag $Version)) { throw "invalid version '$Version': only letters, digits, '.', '_' and '-' are allowed" }

if ($Version -eq "latest") {
  if ($DryRun) { $Tag = "<latest tag>" }
  else {
    try {
      $rel = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    } catch {
      # GitHub's "latest" skips pre-releases; fall back to the newest release of any kind.
      Write-Host "no full release yet; using the newest pre-release (pass -Version vX.Y.Z to choose)"
      $rel = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases?per_page=1") | Select-Object -First 1
    }
    if (-not $rel -or -not $rel.tag_name) { throw "could not determine a release tag (pass -Version vX.Y.Z)" }
    $Tag = $rel.tag_name
  }
} else { $Tag = $Version }

if (-not $DryRun -and -not (Test-Tag $Tag)) { throw "refusing release tag '$Tag': only letters, digits, '.', '_' and '-' are allowed" }
if ($env:KILLER_BASE_URL) {
  Write-Warning "KILLER_BASE_URL is set; downloading from $($env:KILLER_BASE_URL) instead of github.com/$Repo. The .sha256 file comes from the same place, so it detects corruption but does not prove the origin is trustworthy."
}
$name = "killer_super-$Tag-$target"
$base = if ($env:KILLER_BASE_URL) { $env:KILLER_BASE_URL } else { "https://github.com/$Repo/releases/download/$Tag" }
$binDir = Join-Path $Prefix "bin"
Write-Host "target:   $target"
Write-Host "tag:      $Tag"
Write-Host "archive:  $base/$name.zip"
Write-Host "checksum: $base/$name.zip.sha256"
# Independent checksum source. A mirror (KILLER_BASE_URL) has no default: pass -Sha256 or KILLER_CHECKSUMS_URL.
$explicitChecksums = [bool]$ChecksumsUrl
if (-not $Sha256 -and -not $ChecksumsUrl -and -not $env:KILLER_BASE_URL) {
  $ChecksumsUrl = "https://raw.githubusercontent.com/$Repo/main/checksums/$Tag.sha256"
}
if ($Sha256) { Write-Host "pinned:   sha256 $Sha256 (from -Sha256)" }
elseif ($ChecksumsUrl) { Write-Host "pinned:   $ChecksumsUrl" }
else { Write-Host "pinned:   none" }
Write-Host "install:  $binDir\killer.exe and $binDir\killer_super.exe"
if ($DryRun) { Write-Host "dry run: nothing downloaded or installed"; return }

$work = Join-Path ([IO.Path]::GetTempPath()) ("killer-install-" + [guid]::NewGuid())
New-Item -ItemType Directory $work | Out-Null
try {
  Invoke-WebRequest "$base/$name.zip" -OutFile "$work\$name.zip"
  Invoke-WebRequest "$base/$name.zip.sha256" -OutFile "$work\$name.zip.sha256"
  $want = ((Get-Content "$work\$name.zip.sha256" -Raw).Trim() -split "\s+")[0].ToLower()
  $got = (Get-FileHash "$work\$name.zip" -Algorithm SHA256).Hash.ToLower()
  if ($want -ne $got) { throw "checksum mismatch: expected $want, got $got" }
  Write-Host "checksum ok (release .sha256: detects corruption, same origin as the zip)"

  # Independent check: a checksum that was not served with the archive.
  $pinned = $Sha256
  if (-not $pinned -and $ChecksumsUrl) {
    $sums = $null
    try {
      Invoke-WebRequest $ChecksumsUrl -OutFile "$work\pinned.sums"
      $sums = @(Get-Content "$work\pinned.sums")
    } catch {
      if ($explicitChecksums -or $RequirePinned) { throw "could not fetch the pinned checksum file $ChecksumsUrl" }
      Write-Warning "no pinned checksum for $Tag at $ChecksumsUrl; the zip was only compared with the .sha256 from the same release. Pass -Sha256 <hash from the release notes> for an independent check, or -RequirePinned to refuse this."
    }
    if ($null -ne $sums) {
      foreach ($line in $sums) {
        $parts = ($line.Trim() -split "\s+")
        if ($parts.Count -ge 2 -and $parts[1].TrimStart("*") -eq "$name.zip") { $pinned = $parts[0].ToLower(); break }
      }
      if (-not $pinned) { throw "the pinned checksum file $ChecksumsUrl has no entry for $name.zip" }
    }
  }
  if ($pinned) {
    if ($pinned -ne $got) { throw "checksum mismatch: the pinned checksum is $pinned but the zip hashes to $got" }
    Write-Host "pinned checksum ok"
  } elseif ($RequirePinned) {
    throw "-RequirePinned: no independent checksum was available, refusing to install"
  }
  Expand-Archive "$work\$name.zip" -DestinationPath $work
  New-Item -ItemType Directory -Force $binDir | Out-Null
  foreach ($b in "killer", "killer_super") {
    Copy-Item "$work\$name\$b.exe" "$binDir\$b.exe" -Force
    Write-Host "installed $binDir\$b.exe"
  }
  if (-not (($env:Path -split ";") -contains $binDir)) {
    Write-Host "add it to your PATH:  setx PATH `"$binDir;%PATH%`""
  }
} finally { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
