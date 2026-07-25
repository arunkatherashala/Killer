# CSV to KORE-K converter
# Converts test_data_100records.csv -> 1st_data.kore

$csvPath  = "C:\Users\skathera\Downloads\test_data_100records.csv"
$korePath = "C:\Users\skathera\Downloads\kore\1st_data.kore"

$lines  = Get-Content $csvPath -Encoding UTF8
$header = $lines[0] -split ","
$nrows  = $lines.Count - 1
$ncols  = $header.Count

Write-Host "CSV: $nrows rows x $ncols cols"

# Parse all rows into a 2D array
$rows = for ($i = 1; $i -le $nrows; $i++) {
    ,($lines[$i] -split ",")
}

# Detect type per column: int -> delta, float -> plain, str -> dict
$types = @(); $algos = @()
for ($ci = 0; $ci -lt $ncols; $ci++) {
    $allInt = $true; $allFloat = $true
    for ($ri = 0; $ri -lt $nrows; $ri++) {
        $v = if ($ci -lt $rows[$ri].Count) { $rows[$ri][$ci].Trim() } else { "" }
        if ($v -eq "" -or $v -eq "null") { continue }
        $d = 0.0; $l = 0L
        if (-not [double]::TryParse($v, [System.Globalization.NumberStyles]::Any, [System.Globalization.CultureInfo]::InvariantCulture, [ref]$d)) {
            $allInt = $false; $allFloat = $false; break
        }
        if ($allInt -and -not [int64]::TryParse($v, [ref]$l)) { $allInt = $false }
    }
    if ($allInt)        { $types += "int";   $algos += "delta" }
    elseif ($allFloat)  { $types += "float"; $algos += "plain" }
    else                { $types += "str";   $algos += "dict"  }
}

# Sanitize: remove chars used as KORE-K separators (^  *  ~)
function Sanitize([string]$v) {
    if ([string]::IsNullOrWhiteSpace($v)) { return "EMPTY" }
    return $v.Trim() -replace '\^','-' -replace '\*','x' -replace '~','-' -replace '\|','/'
}

# Delta encode integers
function DeltaEncode([string[]]$colVals) {
    $nums = $colVals | ForEach-Object {
        $v = $_.Trim()
        if ($v -eq "" -or $v -eq "EMPTY") { 0L }
        else { try { [int64]$v } catch { 0L } }
    }
    $parts = @("$($nums[0])")
    for ($i = 1; $i -lt $nums.Count; $i++) {
        $parts += "$($nums[$i] - $nums[$i-1])"
    }
    return ($parts -join ",")
}

# Plain encode floats
function PlainEncode([string[]]$colVals) {
    return ($colVals | ForEach-Object {
        $v = $_.Trim()
        if ($v -eq "") { "0.0" } else { $v }
    }) -join ","
}

# Dict encode strings (dictionary + run indices)
function DictEncode([string[]]$colVals) {
    $dict = [System.Collections.Generic.List[string]]::new()
    $idxs = [System.Collections.Generic.List[int]]::new()
    foreach ($raw in $colVals) {
        $v = Sanitize $raw
        $idx = $dict.IndexOf($v)
        if ($idx -lt 0) { $dict.Add($v); $idx = $dict.Count - 1 }
        $idxs.Add($idx)
    }
    return ($dict -join "^") + "*" + ($idxs -join ",")
}

# Build SCHEMA line
$schemaParts = for ($i = 0; $i -lt $ncols; $i++) {
    "$($header[$i]):$($types[$i]):$($algos[$i])"
}
$schemaLine = "SCHEMA " + ($schemaParts -join " ")

# Build COL lines
Write-Host "Encoding columns..."
$colLines = for ($ci = 0; $ci -lt $ncols; $ci++) {
    $colVals = [string[]]($rows | ForEach-Object {
        if ($ci -lt $_.Count) { $_[$ci] } else { "" }
    })
    $encoded = switch ($algos[$ci]) {
        "delta" { DeltaEncode $colVals }
        "plain" { PlainEncode $colVals }
        "dict"  { DictEncode  $colVals }
    }
    "COL $($header[$ci]) $encoded"
    if ($ci % 10 -eq 0) { Write-Host "  col $ci / $ncols done" }
}

# Assemble and write with Unix line endings (LF only, not CRLF)
$allLines = @("KORE-K") + @($schemaLine) + @("ROWS $nrows") + $colLines + @("END")
$content  = $allLines -join "`n"
[System.IO.File]::WriteAllText($korePath, $content, [System.Text.UTF8Encoding]::new($false))

$size = (Get-Item $korePath).Length
Write-Host ""
Write-Host "Done! Written: $korePath"
Write-Host "File size: $([math]::Round($size/1KB,1)) KB"
Write-Host "Original CSV: $([math]::Round((Get-Item $csvPath).Length/1KB,1)) KB"
Write-Host "Compression : $([math]::Round($size * 100.0 / (Get-Item $csvPath).Length, 1))% of CSV"
