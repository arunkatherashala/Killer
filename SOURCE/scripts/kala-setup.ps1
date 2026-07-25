#Requires -Version 5.1
<#
.SYNOPSIS
  Kala / KhLM setup: Ollama smoke test, optional advanced tier-2 + GGUF checklist.

.DESCRIPTION
  Default: verifies `ollama`, pulls a chat model, optional killer-native smoke.

  -Advanced: prints the full "most capable" checklist, creates %USERPROFILE%\.killer\models,
  suggests env vars (Tier-2 LLM + local GGUF + AI System synthesis), optional -Persist to
  store Tier-2 Ollama vars in your Windows **User** environment (new terminals pick them up).

.NOTES
  KALA_OLLAMA_MODEL   — default chat model (default: phi3:mini)
  KALA_OLLAMA_MODEL2  — second pull for Advanced (default: llama3.2)
  KILLER_NATIVE_EXE   — path to killer-native.exe if not auto-detected

  Rust runtime reads at **process start** (restart Kala / killer-native after changing env):
    KILLER_KHLM_LLM_PROVIDER   ollama | groq | openai | anthropic
    KILLER_KHLM_LLM_API_KEY     (empty for Ollama)
    KILLER_KHLM_LLM_MODEL       e.g. llama3.2, gpt-4o-mini, llama3-70b-8192
    KILLER_KHLM_LLM_MAX_TOKENS  optional, 64–32000
    KILLER_KHLM_GGUF            path or short name under .killer\models (KhLM + AI System + polyglot RLM)
    KILLER_KHLM_RLM             optional; explicit GGUF for polyglot Tier-3 if different from KILLER_KHLM_GGUF
#>
param(
    [switch] $Advanced,
    [switch] $Persist
)

$ErrorActionPreference = 'Stop'

function Test-Cmd([string] $Name) {
    $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

$modelsRoot = Join-Path $env:USERPROFILE '.killer\models'

Write-Host "== kala-setup: Ollama + optional killer-native smoke =="

if (-not (Test-Cmd 'ollama')) {
    Write-Host "ollama not found. Install from https://ollama.com then re-run this script."
    exit 1
}

Write-Host "-- ollama version --"
ollama --version

$model = if ($env:KALA_OLLAMA_MODEL) { $env:KALA_OLLAMA_MODEL } else { 'phi3:mini' }
Write-Host "-- pulling model: $model (no-op if already present) --"
ollama pull $model

if ($Advanced) {
    $m2 = if ($env:KALA_OLLAMA_MODEL2) { $env:KALA_OLLAMA_MODEL2 } else { 'llama3.2' }
    Write-Host "-- Advanced: second Ollama model: $m2 --"
    ollama pull $m2

    if (-not (Test-Path $modelsRoot)) {
        New-Item -ItemType Directory -Path $modelsRoot -Force | Out-Null
        Write-Host "-- created $modelsRoot (put .gguf files here, or set KILLER_KHLM_GGUF to full path) --"
    } else {
        Write-Host "-- GGUF registry exists: $modelsRoot --"
    }

    Write-Host ""
    Write-Host "======== ADVANCED KILLER / KALA SETUP (copy or use -Persist) ========"
    Write-Host "1) Tier-2 LLM (Think, Write, expert Ask, code assist) — pick ONE provider:"
    Write-Host "   Ollama (local):"
    Write-Host '     setx KILLER_KHLM_LLM_PROVIDER ollama'
    Write-Host '     setx KILLER_KHLM_LLM_API_KEY ""'
    Write-Host "     setx KILLER_KHLM_LLM_MODEL $m2"
    Write-Host "   Groq (fast cloud): setx KILLER_KHLM_LLM_PROVIDER groq && setx KILLER_KHLM_LLM_API_KEY <your_key> && setx KILLER_KHLM_LLM_MODEL llama3-70b-8192"
    Write-Host "   OpenAI: setx KILLER_KHLM_LLM_PROVIDER openai && setx KILLER_KHLM_LLM_API_KEY sk-... && setx KILLER_KHLM_LLM_MODEL gpt-4o-mini"
    Write-Host ""
    Write-Host "2) Local GGUF — use a reasoning-capable .gguf for best AI System synthesis:"
    Write-Host "   Download (e.g. DeepSeek-R1 distill, QwQ, or strong instruct) into:"
    Write-Host "   $modelsRoot"
    Write-Host "   Then:"
    Write-Host '     setx KILLER_KHLM_GGUF "<filename.gguf or short name>"'
    Write-Host ""
    Write-Host "3) Optional: larger cloud answers"
    Write-Host '     setx KILLER_KHLM_LLM_MAX_TOKENS 2048'
    Write-Host ""
    Write-Host "4) Restart terminal + Kala / killer-native after setx. Verify in Killer:"
    Write-Host "     println(khlm_status())"
    Write-Host ""
    Write-Host "5) Kala **AI System** mode — for HARD questions: router + search + neural + merge."
    Write-Host "   Prefer a reasoning GGUF; without GGUF you still get KhLM + Ghost-108 (no neural/merge)."
    Write-Host "   Positioning: advanced orchestration + merging — not AGI."
    Write-Host "===================================================================="

    if ($Persist) {
        Write-Host ""
        Write-Host "-- -Persist: writing User env for Ollama Tier-2 (provider/model; empty API key) --"
        [Environment]::SetEnvironmentVariable('KILLER_KHLM_LLM_PROVIDER', 'ollama', 'User')
        [Environment]::SetEnvironmentVariable('KILLER_KHLM_LLM_API_KEY', '', 'User')
        [Environment]::SetEnvironmentVariable('KILLER_KHLM_LLM_MODEL', $m2, 'User')
        Write-Host "Done. Open a **new** PowerShell window and restart Kala."
    }
}

Write-Host "-- ollama run smoke (one line) --"
try {
    $ollamaOut = "Reply with exactly: KALA_SETUP_OK" | & ollama run $model 2>&1
    Write-Host ($ollamaOut | Out-String).Substring(0, [Math]::Min(500, (($ollamaOut | Out-String).Length)))
} catch {
    Write-Warning "ollama run failed (model may still be usable): $_"
}

$killer = $env:KILLER_NATIVE_EXE
if (-not $killer) {
    $guess = Join-Path $PSScriptRoot '..\src\v2-rust\killer\target\release\killer-native.exe'
    if (Test-Path $guess) { $killer = $guess }
}
if ($killer -and (Test-Path $killer)) {
    Write-Host "-- killer-native kala_ask smoke --"
    $tmp = Join-Path $env:TEMP '_kala_setup_smoke.killer'
    @'
println(kala_ask("Say hello in one short sentence."))
'@ | Set-Content -Path $tmp -Encoding utf8
    & $killer $tmp
} else {
    Write-Host "(skip) killer-native not found. Build: cd SOURCE/src/v2-rust/killer ; cargo build --release"
}

Write-Host "== kala-setup: done =="
Write-Host "Tip: advanced setup:  .\kala-setup.ps1 -Advanced   (add -Persist to save Ollama env)"
Write-Host "Tip: GGUF smoke:        killer-native --model <path\to\file.gguf> `"hello`""
