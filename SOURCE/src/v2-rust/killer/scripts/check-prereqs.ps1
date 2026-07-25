$ErrorActionPreference = "SilentlyContinue"

Write-Host "Killer Native VM - Prerequisite Check"
Write-Host "-------------------------------------"

$rustupPath = "$env:USERPROFILE\.cargo\bin\rustup.exe"
$cargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"

if (Test-Path $cargoPath) {
    Write-Host "[OK] cargo found at $cargoPath"
    & $cargoPath --version
} else {
    Write-Host "[FAIL] cargo not found"
    Write-Host "Install Rustup: winget install Rustlang.Rustup"
}

if (Test-Path $rustupPath) {
    Write-Host "[OK] rustup found at $rustupPath"
    & $rustupPath show active-toolchain
} else {
    Write-Host "[FAIL] rustup not found"
}

$dbghelpX64 = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\Lib" -Recurse -Filter dbghelp.lib |
    Where-Object { $_.FullName -match "\\um\\x64\\" } |
    Select-Object -First 1 -ExpandProperty FullName

if ($dbghelpX64) {
    Write-Host "[OK] x64 dbghelp.lib found at $dbghelpX64"
} else {
    Write-Host "[FAIL] x64 dbghelp.lib not found"
    Write-Host "Install/Modify Visual Studio Build Tools and include:"
    Write-Host "  - Desktop development with C++"
    Write-Host "  - Windows 10 SDK or Windows 11 SDK"
}

Write-Host ""
Write-Host "If cargo is installed but command is not recognized in current terminal:"
Write-Host "  Restart terminal, or run:"
Write-Host "  `$env:Path += ';' + "$env:USERPROFILE\.cargo\bin""
