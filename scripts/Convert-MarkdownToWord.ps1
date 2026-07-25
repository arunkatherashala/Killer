# Killer Markdown to Word Document Converter
# PowerShell Script for Windows
# Version 1.0 - March 20, 2026

param(
    [string]$MarkdownFile = "KILLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.md",
    [string]$OutputFile = "KILLER_COMPREHENSIVE_LEARNING_MANUAL_v4.2.docx"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Killer Language Manual - Word Converter" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if markdown file exists
if (-not (Test-Path $MarkdownFile)) {
    Write-Host "ERROR: File not found: $MarkdownFile" -ForegroundColor Red
    exit 1
}

Write-Host "[1/5] Reading markdown file..." -ForegroundColor Yellow
$markdownContent = Get-Content $MarkdownFile -Raw

# Check if Pandoc is installed
$pandocPath = Get-Command pandoc -ErrorAction SilentlyContinue

if ($null -eq $pandocPath) {
    Write-Host "[!] Pandoc not found. Attempting to install..." -ForegroundColor Magenta
    
    # Try with Chocolatey
    $chocoPath = Get-Command choco -ErrorAction SilentlyContinue
    if ($null -ne $chocoPath) {
        Write-Host "    Installing via Chocolatey..." -ForegroundColor Cyan
        & choco install pandoc -y | Out-Null
        
        $pandocPath = Get-Command pandoc -ErrorAction SilentlyContinue
    }
}

if ($null -eq $pandocPath) {
    Write-Host "[!] Pandoc installation failed. Using Word COM object method..." -ForegroundColor Magenta
    
    # Method 2: Use Word COM Object
    Write-Host "[2/5] Opening Microsoft Word..." -ForegroundColor Yellow
    
    try {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        
        Write-Host "[3/5] Creating new document..." -ForegroundColor Yellow
        $doc = $word.Documents.Add()
        
        Write-Host "[4/5] Processing markdown and converting to Word format..." -ForegroundColor Yellow
        
        # Split into lines
        $lines = $markdownContent -split "`n"
        $lastHeadingLevel = 0
        
        foreach ($line in $lines) {
            if ($line -match "^# (.+)$") {
                $text = $matches[1]
                $range = $doc.Content
                $range.InsertAfter($text)
                $range.ParagraphFormat.Style = 'Heading 1'
                $range.Font.Bold = $true
                $range.Font.Size = 28
                $range.InsertParagraphAfter()
            }
            elseif ($line -match "^## (.+)$") {
                $text = $matches[1]
                $range = $doc.Content
                $range.InsertAfter($text)
                $range.ParagraphFormat.Style = 'Heading 2'
                $range.Font.Bold = $true
                $range.Font.Size = 24
                $range.InsertParagraphAfter()
            }
            elseif ($line -match "^### (.+)$") {
                $text = $matches[1]
                $range = $doc.Content
                $range.InsertAfter($text)
                $range.ParagraphFormat.Style = 'Heading 3'
                $range.Font.Bold = $true
                $range.Font.Size = 22
                $range.InsertParagraphAfter()
            }
            elseif ($line -match "^```(.+)$") {
                # Start of code block
                $range = $doc.Content
                $range.InsertAfter($line)
                $range.Font.Name = 'Courier New'
                $range.Font.Size = 10
                $range.InsertParagraphAfter()
            }
            elseif ($line.Trim() -ne "") {
                # Regular paragraph
                $range = $doc.Content
                $range.InsertAfter($line.Trim())
                $range.InsertParagraphAfter()
            }
            else {
                # Empty line - add space
                $range = $doc.Content
                $range.InsertParagraphAfter()
            }
        }
        
        Write-Host "[5/5] Saving document as Word format..." -ForegroundColor Yellow
        
        # Save as DOCX
        $outputPath = (Resolve-Path .).Path + "\" + $OutputFile
        $doc.SaveAs([ref]$outputPath, [ref]16)  # 16 = wdFormatDocm (modern Word format)
        
        Write-Host ""
        Write-Host "✅ SUCCESS! Document created:" -ForegroundColor Green
        Write-Host "   $OutputFile" -ForegroundColor Cyan
        
        # Cleanup
        $doc.Close()
        $word.Quit()
    }
    catch {
        Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
}
else {
    # Method 1: Use Pandoc (Faster & Better)
    Write-Host "[2/5] Pandoc found. Using Pandoc for conversion..." -ForegroundColor Green
    Write-Host "[3/5] Converting markdown to DOCX..." -ForegroundColor Yellow
    
    # Run Pandoc conversion
    & pandoc $MarkdownFile `
        --from=markdown `
        --to=docx `
        --output=$OutputFile `
        --standalone `
        --toc `
        --toc-depth=3 `
        --number-sections `
        --reference-doc=None
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ SUCCESS! Document created:" -ForegroundColor Green
        Write-Host "   $OutputFile" -ForegroundColor Cyan
    }
    else {
        Write-Host "❌ FAILED: Pandoc conversion error" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "Document Details:" -ForegroundColor Yellow
$fileInfo = Get-Item $OutputFile
Write-Host "  File Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB"
Write-Host "  Created:   $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""
Write-Host "Ready for team distribution and market publication!" -ForegroundColor Green
Write-Host ""
