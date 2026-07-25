# Revalidate Killer (killer-native) — run after any language / VM / compiler change.
# Usage (from repo root or this crate):
#   pwsh -File SOURCE/src/v2-rust/killer/scripts/revalidate_killer.ps1
#   pwsh -File scripts/revalidate_killer.ps1   # if cwd is SOURCE/src/v2-rust/killer
#
# Parity with .github/workflows/killer-native.yml plus a few language-surface tests.
# Use -Full for entire integration suite (slow).

param(
    [switch] $Full,
    [switch] $Release
)

$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $PSScriptRoot
if (Test-Path (Join-Path $here "Cargo.toml")) {
    $crateRoot = $here
} else {
    $crateRoot = $PSScriptRoot
}

Push-Location $crateRoot
try {
    if ($Release) {
        cargo build --release
    } else {
        cargo build
    }

    if ($Full) {
        Write-Host "=== FULL: cargo test (all targets, may take many minutes) ===" -ForegroundColor Cyan
        cargo test
    } else {
        Write-Host "=== CI parity + language smoke ===" -ForegroundColor Cyan
        cargo test --lib
        cargo test --test pipeline_conformance
        cargo test --test trit_three_valued
        cargo test --test ai_integration_tests --test ai_annotations_tests
        cargo test --test builtin_pythonic
        cargo test --test parser_tests
    }
    Write-Host "`nRevalidate OK." -ForegroundColor Green
} finally {
    Pop-Location
}
