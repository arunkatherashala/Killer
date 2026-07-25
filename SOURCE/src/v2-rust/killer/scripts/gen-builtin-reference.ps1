# Regenerate BUILTIN_REFERENCE.md name index from builtin.rs dispatch table.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$builtin = Join-Path $root "src\builtin.rs"
# root = .../SOURCE/src/v2-rust/killer  →  SOURCE = three levels up
$sourceRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $root))
$outDoc = Join-Path $sourceRoot "docs\BUILTIN_REFERENCE.md"
if (-not (Test-Path $builtin)) { throw "Missing $builtin" }

$names = Select-String -Path $builtin -Pattern '^\s*"([a-zA-Z_][a-zA-Z0-9_]*)"\s*=>' -AllMatches |
    ForEach-Object { $_.Matches } | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique

$idx = ($names | ForEach-Object { "- ``$_``" }) -join "`n"
$header = @"
# Killer builtins - reference

**Auto-generated** from ``src/builtin.rs`` dispatch arms. Re-run:

``````powershell
cd SOURCE/src/v2-rust/killer
.\scripts\gen-builtin-reference.ps1
``````

## Name index ($($names.Count) entries)

$idx

## Notes

- Implementation lives in ``BuiltinFunctions`` in ``builtin.rs``.
- **killer_ui (native UI):** ``ui_core_version``, ``ui_headless_tick``, ``ui_headless_snapshot_json``, ``ui_health``, ``ui_help``, ``ui_native_window`` - see ``SOURCE/docs/KILLER_UI_ENGINE.md``. Optional **line sugar** maps e.g. ``ui snapshot``, ``ui tick``, ``ui health``, ``v = ui version`` to those calls.
"@

Set-Content -Path $outDoc -Value $header -Encoding utf8
Write-Host "Wrote $outDoc ($($names.Count) names)"
