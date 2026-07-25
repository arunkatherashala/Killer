<#
.SYNOPSIS
    Killer Professor Agent - Proactive code review and auto-correction
.DESCRIPTION
    Scans all Rust code, docs, tests for errors, quality, security.
    Grades your project A+ to F. Auto-fixes what it can.
#>

param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$AutoFix,
    [switch]$DeepScan
)

$RustProject = Join-Path $ProjectRoot 'SOURCE\src\v2-rust\killer'
$RustSrc     = Join-Path $RustProject 'src'
$DocsDir     = Join-Path $ProjectRoot 'DOCS'
$SrcDocs     = Join-Path $ProjectRoot 'SOURCE\docs'
$AgentHome   = Join-Path $ProjectRoot '_AGENT'
$ReviewDir   = Join-Path $AgentHome 'professor'
$timestamp   = Get-Date -Format 'yyyy-MM-dd_HH-mm-ss'
$ReviewFile  = Join-Path $ReviewDir "review_$timestamp.md"
$ScoreFile   = Join-Path $ReviewDir 'scores.csv'

New-Item -ItemType Directory -Force -Path $ReviewDir | Out-Null

$script:Findings    = @()
$script:Corrections = @()
$script:Score       = 100
$script:FilesScanned = 0

function Add-Finding {
    param([string]$Severity, [string]$Category, [string]$File, [int]$Line, [string]$Message, [string]$Suggestion)
    $script:Findings += [PSCustomObject]@{
        Severity=$Severity; Category=$Category; File=$File; Line=$Line; Message=$Message; Suggestion=$Suggestion
    }
    $deduction = switch ($Severity) { 'CRITICAL' { 5 } 'WARNING' { 2 } 'INFO' { 0.5 } default { 0 } }
    $script:Score = [math]::Max(0, $script:Score - $deduction)
}

# ====== ANALYSIS 1: RUST CODE QUALITY ======
function Invoke-RustCodeAnalysis {
    Write-Host '  [1/7] Analyzing Rust code quality...' -ForegroundColor Cyan
    $rsFiles = Get-ChildItem $RustSrc -Filter '*.rs' -Recurse
    foreach ($f in $rsFiles) {
        $script:FilesScanned++
        $content = Get-Content $f.FullName -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }
        $lines = $content -split "`n"
        $lineNum = 0

        foreach ($ln in $lines) {
            $lineNum++

            if ($ln -match '\.unwrap\(' -and $ln -notmatch '//.*unwrap' -and $f.Name -notmatch 'test') {
                Add-Finding 'WARNING' 'CODE' $f.Name $lineNum 'unwrap found - can panic at runtime' 'Use unwrap_or or match instead'
            }

            if ($ln -match '(TODO|FIXME|HACK|XXX|STUB)') {
                Add-Finding 'INFO' 'CODE' $f.Name $lineNum "Found $($Matches[1]) marker" 'Complete or remove before release'
            }

            if ($ln -match '\bunsafe\b' -and $ln -notmatch '^\s*//') {
                Add-Finding 'WARNING' 'SECURITY' $f.Name $lineNum 'unsafe block found' 'Verify memory safety and document why'
            }

            if ($ln.Length -gt 200) {
                Add-Finding 'INFO' 'CODE' $f.Name $lineNum "Line is $($ln.Length) chars long" 'Break into multiple lines'
            }

            if ($ln -match 'Err\(_\)\s*=>\s*\{' -and $ln -match '\{\s*\}') {
                Add-Finding 'WARNING' 'LOGIC' $f.Name $lineNum 'Silent error suppression' 'Log the error or return it'
            }

            if ($ln -match '\.clone\(' -and $ln -match '(Vec|String|HashMap|BTreeMap)') {
                Add-Finding 'INFO' 'PERFORMANCE' $f.Name $lineNum 'Cloning a large type' 'Consider borrowing or Rc/Arc'
            }

            if ($ln -match 'allow\(dead_code\)') {
                Add-Finding 'INFO' 'CODE' $f.Name $lineNum 'Dead code allowed' 'Remove dead code or explain why'
            }

            if ($ln -match 'panic!\(' -and $f.Name -notmatch 'test' -and $ln -notmatch '^\s*//') {
                Add-Finding 'WARNING' 'CODE' $f.Name $lineNum 'panic! in production code' 'Use Result/Option error handling'
            }
        }

        if ($lines.Count -gt 1000) {
            Add-Finding 'INFO' 'CODE' $f.Name 0 "File has $($lines.Count) lines - consider splitting" 'Break into smaller modules'
        }

        if ($content -notmatch '///') {
            Add-Finding 'INFO' 'DOC' $f.Name 0 'No doc comments found' 'Add /// doc comments to public items'
        }

        if ($content -match 'pub fn ' -and $content -notmatch 'cfg\(test\)' -and $f.Name -notmatch 'test|bench') {
            Add-Finding 'INFO' 'TEST' $f.Name 0 'Has public functions but no test module' 'Add unit tests'
        }
    }
}

# ====== ANALYSIS 2: CARGO BUILD + CLIPPY ======
function Invoke-BuildAnalysis {
    Write-Host '  [2/7] Running cargo check...' -ForegroundColor Cyan
    Push-Location $RustProject

    $checkOut = & cargo check 2>&1 | Out-String
    $checkOk = $LASTEXITCODE -eq 0

    if (-not $checkOk) {
        $errLines = $checkOut -split "`n" | Where-Object { $_ -match '^error' }
        foreach ($e in ($errLines | Select-Object -First 20)) {
            Add-Finding 'CRITICAL' 'CODE' 'cargo' 0 $e 'Fix compilation error'
        }
    } else {
        Add-Finding 'GOOD' 'CODE' 'cargo' 0 'Project compiles successfully' ''
    }

    $warnLines = $checkOut -split "`n" | Where-Object { $_ -match '^warning\[' }
    foreach ($w in ($warnLines | Select-Object -First 20)) {
        Add-Finding 'WARNING' 'CODE' 'cargo' 0 $w 'Fix compiler warning'
    }

    $clippyOut = & cargo clippy 2>&1 | Out-String
    if ($clippyOut) {
        $cWarns = $clippyOut -split "`n" | Where-Object { $_ -match '^warning:' -and $_ -notmatch 'generated' }
        foreach ($cw in ($cWarns | Select-Object -First 30)) {
            Add-Finding 'WARNING' 'CODE' 'clippy' 0 $cw 'Clippy suggestion'
        }
        if ($cWarns.Count -eq 0) {
            Add-Finding 'GOOD' 'CODE' 'clippy' 0 'Clippy clean - no suggestions' ''
        }
    }

    Pop-Location
}

# ====== ANALYSIS 3: TEST RESULTS ======
function Invoke-TestAnalysis {
    Write-Host '  [3/7] Running test suite...' -ForegroundColor Cyan
    Push-Location $RustProject

    $testOut = & cargo test --lib 2>&1 | Out-String

    $passedPattern = '(\d+) passed'
    $failedPattern = '(\d+) failed'
    $ignoredPattern = '(\d+) ignored'

    $passed = 0; $failed = 0; $ignored = 0
    if ($testOut -match $passedPattern) { $passed = [int]$Matches[1] }
    if ($testOut -match $failedPattern) { $failed = [int]$Matches[1] }
    if ($testOut -match $ignoredPattern) { $ignored = [int]$Matches[1] }

    $total = $passed + $failed + $ignored

    if ($total -gt 0) {
        if ($failed -gt 0) {
            Add-Finding 'CRITICAL' 'TEST' 'tests' 0 "$failed test(s) FAILING out of $total" 'Fix failing tests immediately'
            $failNames = $testOut -split "`n" | Where-Object { $_ -match 'FAILED' }
            foreach ($fn in ($failNames | Select-Object -First 10)) {
                Add-Finding 'CRITICAL' 'TEST' 'tests' 0 $fn 'Fix this test'
            }
        } else {
            Add-Finding 'GOOD' 'TEST' 'tests' 0 "All $passed tests passing, $ignored ignored" ''
        }

        if ($ignored -gt 10) {
            Add-Finding 'WARNING' 'TEST' 'tests' 0 "$ignored tests are ignored - too many" 'Review and re-enable or remove'
        }

        $coverage = [math]::Round(($passed / [math]::Max(1,$total)) * 100, 1)
        Add-Finding 'INFO' 'TEST' 'tests' 0 "Test pass rate: $coverage percent" ''
    }

    Pop-Location
}

# ====== ANALYSIS 4: DOCUMENTATION REVIEW ======
function Invoke-DocAnalysis {
    Write-Host '  [4/7] Reviewing documentation...' -ForegroundColor Cyan
    $mdFiles = @()
    if (Test-Path $DocsDir) { $mdFiles += Get-ChildItem $DocsDir -Filter '*.md' }
    if (Test-Path $SrcDocs) { $mdFiles += Get-ChildItem $SrcDocs -Filter '*.md' }

    foreach ($f in $mdFiles) {
        $script:FilesScanned++
        $content = Get-Content $f.FullName -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }

        if ($content.Length -lt 100) {
            Add-Finding 'WARNING' 'DOC' $f.Name 0 "Document is nearly empty ($($content.Length) chars)" 'Add content or remove'
        }

        if ($content -match 'v1\.\d' -and $f.Name -notmatch 'V1|HISTORY|LEGACY|MIGRATION') {
            Add-Finding 'INFO' 'DOC' $f.Name 0 'References v1.x - may be outdated' 'Update version references'
        }

        if ($f.Name -match 'GUIDE|TUTORIAL|QUICK.*START|REFERENCE') {
            if ($content -notmatch '``````') {
                Add-Finding 'INFO' 'DOC' $f.Name 0 'Guide has no code examples' 'Add code examples for clarity'
            }
        }
    }

    Add-Finding 'INFO' 'DOC' 'docs' 0 "Scanned $($mdFiles.Count) documentation files" ''
}

# ====== ANALYSIS 5: SECURITY REVIEW ======
function Invoke-SecurityAnalysis {
    Write-Host '  [5/7] Security review...' -ForegroundColor Cyan
    $rsFiles = Get-ChildItem $RustSrc -Filter '*.rs' -Recurse
    foreach ($f in $rsFiles) {
        $content = Get-Content $f.FullName -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }

        if ($content -match 'format!.*SELECT.*\{' -or $content -match 'format!.*INSERT.*\{') {
            Add-Finding 'CRITICAL' 'SECURITY' $f.Name 0 'Possible SQL injection - string formatting in query' 'Use parameterized queries'
        }

        if ($content -match '(password|secret|api_key|token)\s*=\s*"[^"]{8,}"' -and $content -notmatch 'test|example|demo') {
            Add-Finding 'CRITICAL' 'SECURITY' $f.Name 0 'Possible hardcoded secret' 'Move to environment variable'
        }

        if ($content -match 'Command::new.*format!') {
            Add-Finding 'WARNING' 'SECURITY' $f.Name 0 'Dynamic command construction - injection risk' 'Validate inputs before Command'
        }
    }
}

# ====== ANALYSIS 6: PERFORMANCE PATTERNS ======
function Invoke-PerformanceAnalysis {
    Write-Host '  [6/7] Performance analysis...' -ForegroundColor Cyan
    $rsFiles = Get-ChildItem $RustSrc -Filter '*.rs' -Recurse
    foreach ($f in $rsFiles) {
        $content = Get-Content $f.FullName -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }
        $lines = $content -split "`n"
        $loopDepth = 0; $lineNum = 0

        foreach ($ln in $lines) {
            $lineNum++
            if ($ln -match '\bfor\b.*\bin\b' -or $ln -match '\bwhile\b') { $loopDepth++ }
            if ($ln -match '^\s*\}') { $loopDepth = [math]::Max(0, $loopDepth - 1) }
            if ($loopDepth -ge 3) {
                Add-Finding 'WARNING' 'PERFORMANCE' $f.Name $lineNum 'Deeply nested loop 3+ levels' 'Consider algorithmic optimization'
                $loopDepth = 0
            }
        }
    }
}

# ====== ANALYSIS 7: RESEARCH CONSISTENCY ======
function Invoke-ResearchConsistency {
    Write-Host '  [7/7] Research consistency check...' -ForegroundColor Cyan

    $roadmap = Join-Path $ProjectRoot 'SOURCE\docs\KILLER_ROADMAP_TO_10.md'
    if (Test-Path $roadmap) {
        $rmContent = Get-Content $roadmap -Raw
        $incomplete = ([regex]::Matches($rmContent, '(?i)(TODO|NOT IMPLEMENTED|PLANNED|FUTURE|STUB)')).Count
        if ($incomplete -gt 0) {
            Add-Finding 'INFO' 'CODE' 'ROADMAP' 0 "$incomplete items still TODO/PLANNED in roadmap" 'Track progress'
        }
    }

    $tracker = Join-Path $ProjectRoot '_TOOLS\KILLER_MASTER_TRACKER.csv'
    if (Test-Path $tracker) {
        $csv = Import-Csv $tracker -ErrorAction SilentlyContinue
        if ($csv) {
            $inProgress = @($csv | Where-Object { $_.Status -eq 'In-Progress' })
            $complete = @($csv | Where-Object { $_.Status -eq 'Complete' })
            Add-Finding 'INFO' 'CODE' 'TRACKER' 0 "Master tracker: $($complete.Count) complete, $($inProgress.Count) in-progress" ''
            foreach ($item in $inProgress) {
                Add-Finding 'INFO' 'CODE' 'TRACKER' 0 "In-progress: $($item.Module_Name)" 'Complete or update status'
            }
        }
    }

    $testDir = Join-Path $ProjectRoot 'tests'
    if (Test-Path $testDir) {
        $tFiles = Get-ChildItem $testDir -Filter '*.killer' -Recurse -ErrorAction SilentlyContinue
        Add-Finding 'INFO' 'TEST' 'tests' 0 "$($tFiles.Count) .killer test files found" ''
    }

    $changelog = Join-Path $ProjectRoot 'CHANGELOG.md'
    if (Test-Path $changelog) {
        $clContent = Get-Content $changelog -Raw
        $thisMonth = Get-Date -Format 'yyyy-MM'
        if ($clContent -notmatch [regex]::Escape($thisMonth)) {
            Add-Finding 'INFO' 'DOC' 'CHANGELOG.md' 0 'No entries for this month' 'Add recent changes to CHANGELOG'
        }
    }
}

# ====== AUTO-CORRECTION ENGINE ======
function Invoke-AutoCorrections {
    if (-not $AutoFix) { return }
    Write-Host '  Running auto-corrections...' -ForegroundColor Yellow
    $rsFiles = Get-ChildItem $RustSrc -Filter '*.rs' -Recurse
    $fixCount = 0

    foreach ($f in $rsFiles) {
        $content = Get-Content $f.FullName -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }
        $original = $content
        $content = ($content -split "`n" | ForEach-Object { $_.TrimEnd() }) -join "`n"
        while ($content -match "`n`n`n") { $content = $content -replace "`n`n`n", "`n`n" }
        if ($content -and -not $content.EndsWith("`n")) { $content += "`n" }

        if ($content -ne $original) {
            $content | Set-Content $f.FullName -NoNewline -Encoding utf8
            $fixCount++
            $script:Corrections += "Fixed whitespace: $($f.Name)"
        }
    }
    if ($fixCount -gt 0) { Write-Host "  Auto-fixed $fixCount files" -ForegroundColor Green }
}

# ====== GENERATE REPORT ======
function Write-ReviewReport {
    $criticals = @($script:Findings | Where-Object Severity -eq 'CRITICAL')
    $warnings  = @($script:Findings | Where-Object Severity -eq 'WARNING')
    $infos     = @($script:Findings | Where-Object Severity -eq 'INFO')
    $goods     = @($script:Findings | Where-Object Severity -eq 'GOOD')

    $gradeNum = [math]::Floor($script:Score / 10)
    $grade = switch ($gradeNum) {
        { $_ -ge 9 } { 'A+' }
        { $_ -ge 8 } { 'A' }
        { $_ -ge 7 } { 'B' }
        { $_ -ge 6 } { 'C' }
        { $_ -ge 5 } { 'D' }
        default       { 'F' }
    }

    $goodLines = ($goods | ForEach-Object { "- [$($_.Category)] $($_.Message)" }) -join "`n"
    $critLines = if ($criticals.Count -eq 0) { 'None - great job!' } else {
        ($criticals | ForEach-Object { "- CRITICAL [$($_.Category)] $($_.File):$($_.Line) $($_.Message) -> $($_.Suggestion)" }) -join "`n"
    }
    $warnLines = if ($warnings.Count -eq 0) { 'None' } else {
        ($warnings | Select-Object -First 50 | ForEach-Object { "- WARNING [$($_.Category)] $($_.File):$($_.Line) $($_.Message)" }) -join "`n"
    }
    $infoLines = if ($infos.Count -eq 0) { 'None' } else {
        ($infos | Select-Object -First 30 | ForEach-Object { "- [$($_.Category)] $($_.File): $($_.Message)" }) -join "`n"
    }
    $fixLines = if ($script:Corrections.Count -eq 0) { 'None (run with -AutoFix to enable)' } else {
        ($script:Corrections | ForEach-Object { "- $_" }) -join "`n"
    }

    $report = @"
# Professor Agent Review Report

**Date:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
**Project:** Killer Language v2.1
**Files Scanned:** $($script:FilesScanned)
**Score:** $([math]::Round($script:Score, 1)) / 100 (Grade: $grade)

## Summary
| Category | Count |
|----------|-------|
| CRITICAL | $($criticals.Count) |
| WARNING  | $($warnings.Count) |
| INFO     | $($infos.Count) |
| GOOD     | $($goods.Count) |

## What is Good
$goodLines

## Critical Issues
$critLines

## Warnings
$warnLines

## Suggestions
$infoLines

## Auto-Corrections
$fixLines
"@

    $report | Out-File $ReviewFile -Encoding utf8

    "$timestamp,$([math]::Round($script:Score,1)),$grade,$($criticals.Count),$($warnings.Count),$($infos.Count),$($script:FilesScanned)" |
        Out-File $ScoreFile -Append -Encoding utf8

    return @{ Grade=$grade; Score=$script:Score; Criticals=$criticals.Count; Warnings=$warnings.Count; Infos=$infos.Count; Goods=$goods.Count; File=$ReviewFile }
}

# ====== MAIN ======
Write-Host ''
Write-Host '  === KILLER PROFESSOR AGENT ===' -ForegroundColor Magenta
Write-Host '  Proactive - Auto-Thinking - Research-Grade Review' -ForegroundColor Magenta
Write-Host ''

$sw = [System.Diagnostics.Stopwatch]::StartNew()

Invoke-RustCodeAnalysis
Invoke-BuildAnalysis
Invoke-TestAnalysis
Invoke-DocAnalysis
Invoke-SecurityAnalysis
Invoke-PerformanceAnalysis
Invoke-ResearchConsistency
Invoke-AutoCorrections

$sw.Stop()
$result = Write-ReviewReport

Write-Host ''
$gc = switch -Regex ($result.Grade) { 'A' { 'Green' } 'B|C' { 'Yellow' } default { 'Red' } }
Write-Host "  GRADE: $($result.Grade) (Score: $([math]::Round($result.Score,1))/100)" -ForegroundColor $gc
Write-Host "  Critical: $($result.Criticals) | Warnings: $($result.Warnings) | Info: $($result.Infos) | Good: $($result.Goods)" 
Write-Host "  Files scanned: $($script:FilesScanned) in $([math]::Round($sw.Elapsed.TotalSeconds,1))s"
Write-Host "  Report: $($result.File)" -ForegroundColor Cyan
Write-Host ''
