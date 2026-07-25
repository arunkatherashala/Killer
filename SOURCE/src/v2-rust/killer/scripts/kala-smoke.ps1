#Requires -Version 5.1
# Kala HTTP smoke test — requires a running server (see ..\KALA_SMOKE.md).
$ErrorActionPreference = 'Stop'
$port = if ($env:KALA_PORT) { $env:KALA_PORT } else { '8080' }
$base = "http://127.0.0.1:$port"

Write-Host "Kala smoke: GET $base/"
try {
    $page = Invoke-WebRequest -Uri $base -UseBasicParsing -TimeoutSec 10
} catch {
    Write-Error "GET failed: $_"
}
if ($page.StatusCode -ne 200) { throw "Home status $($page.StatusCode)" }
if ($page.Content -notmatch 'Kala|<!DOCTYPE') { throw 'Home page missing expected HTML' }

Write-Host "Kala smoke: POST $base/api/kala"
$bodyObj = @{
    mode     = 'ask'
    question = 'Reply with exactly: SMOKE_OK'
    style    = 'essay'
    lang     = 'killer'
    history  = @()
    uname    = ''
}
$body = $bodyObj | ConvertTo-Json -Compress
try {
    $resp = Invoke-RestMethod -Uri "$base/api/kala" -Method Post -Body $body `
        -ContentType 'application/json; charset=utf-8' -TimeoutSec 120
} catch {
    Write-Error "POST /api/kala failed: $_"
}
if (-not $resp.PSObject.Properties['response']) { throw 'JSON missing response' }
if ($resp.response -isnot [string]) { throw 'response is not a string' }

Write-Host "Kala smoke(optional): POST clear-session"
try {
    $cl = Invoke-RestMethod -Uri "$base/api/kala/clear-session" -Method Post `
        -Body '{}' -ContentType 'application/json; charset=utf-8' -TimeoutSec 10
    if (-not $cl.ok) { Write-Warning "clear-session did not return ok" }
} catch {
    Write-Warning "clear-session failed (optional): $_"
}

Write-Host 'PASS.'
