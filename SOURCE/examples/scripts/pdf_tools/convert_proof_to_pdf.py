#!/usr/bin/env python3
"""
Killer-inspired PDF Converter
Converts P vs NP proof markdown to publication-quality PDF
Uses pandoc via Python subprocess
"""

import os
import subprocess
import sys
from pathlib import Path

def convert_md_to_pdf():
    # File paths
    workspace = r"c:\Users\skathera\Downloads\killer_V2_RS_M11"
    md_file = os.path.join(workspace, "EXPERT_SUBMISSION_MARCH24", "P_vs_NP_PROOF_FINAL_MARCH2026.md")
    pdf_file = os.path.join(workspace, "EXPERT_SUBMISSION_MARCH24", "P_vs_NP_PROOF_FINAL_MARCH2026.pdf")
    
    print("Killer-inspired PDF Converter")
    print("=" * 50)
    print()
    
    # Check if markdown file exists
    if not Path(md_file).exists():
        print(f"ERROR: Markdown file not found: {md_file}")
        return False
    
    print(f"Source: {md_file}")
    print(f"Output: {pdf_file}")
    print()
    
    # Try pandoc first (best quality)
    print("Step 1: Checking for pandoc...")
    try:
        result = subprocess.run(["pandoc", "--version"], capture_output=True, text=True)
        if result.returncode == 0:
            print("  ✓ pandoc found")
            print()
            print("Step 2: Converting with pandoc (publication-quality)...")
            
            cmd = [
                "pandoc",
                md_file,
                "-o", pdf_file,
                "--pdf-engine=xelatex",
                "--toc",
                "--number-sections",
                "-V", "geometry:margin=1in",
                "-V", "fontsize:11pt",
                "-V", "linestretch:1.15",
                "--highlight-style=tango"
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode == 0:
                if Path(pdf_file).exists():
                    size_kb = Path(pdf_file).stat().st_size / 1024
                    print(f"  ✓ PDF created successfully")
                    print(f"  Size: {size_kb:.1f} KB")
                    print()
                    print("✅ CONVERSION COMPLETE")
                    return True
                else:
                    print("  ERROR: PDF file not created")
                    return False
            else:
                print(f"  pandoc error: {result.stderr}")
                raise Exception("pandoc failed")
                
    except (FileNotFoundError, Exception) as e:
        print(f"  pandoc not available, trying alternative...")
        print()
    
    # Try wkhtmltopdf
    print("Step 2: Checking for wkhtmltopdf...")
    try:
        result = subprocess.run(["wkhtmltopdf", "--version"], capture_output=True, text=True)
        if result.returncode == 0:
            print("  ✓ wkhtmltopdf found")
            print()
            print("Step 3: Converting to HTML first...")
            
            # First convert MD to HTML
            html_file = pdf_file.replace(".pdf", ".html")
            cmd = f'type "{md_file}" > "{html_file}"'
            os.system(cmd)
            
            print(f"  ✓ HTML intermediate created")
            print()
            print("Step 4: Converting HTML to PDF...")
            
            cmd = ["wkhtmltopdf", "--quiet", html_file, pdf_file]
            result = subprocess.run(cmd, capture_output=True, text=True)
            
            if Path(pdf_file).exists():
                size_kb = Path(pdf_file).stat().st_size / 1024
                print(f"  ✓ PDF created")
                print(f"  Size: {size_kb:.1f} KB")
                os.remove(html_file)
                print()
                print("✅ CONVERSION COMPLETE")
                return True
                
    except Exception as e:
        print(f"  wkhtmltopdf failed: {e}")
    
    # Fallback: Create using Python libraries
    print("Step 2: Using Python libraries (reportlab/pypdf)...")
    try:
        from reportlab.lib.pagesizes import letter
        from reportlab.lib.styles import getSampleStyleSheet
        from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
        from reportlab.lib.units import inch
        
        print("  ✓ reportlab found")
        print()
        print("Step 3: Building PDF...")
        
        # Read markdown
        with open(md_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Build PDF
        doc = SimpleDocTemplate(pdf_file, pagesize=letter,
                              rightMargin=0.75*inch, leftMargin=0.75*inch,
                              topMargin=1*inch, bottomMargin=1*inch)
        
        story = []
        styles = getSampleStyleSheet()
        
        # Simple paragraph from markdown
        for line in content.split('\n'):
            if line.strip():
                story.append(Paragraph(line, styles['Normal']))
                story.append(Spacer(1, 0.12*inch))
        
        doc.build(story)
        
        if Path(pdf_file).exists():
            size_kb = Path(pdf_file).stat().st_size / 1024
            print(f"  ✓ PDF created")
            print(f"  Size: {size_kb:.1f} KB")
            print()
            print("✅ CONVERSION COMPLETE")
            return True
            
    except ImportError:
        print("  reportlab not found, attempting weasyprint...")
        try:
            from weasyprint import HTML
            
            print("  ✓ weasyprint found")
            print()
            print("Step 3: Building PDF with weasyprint...")
            
            # Simple HTML wrapper
            with open(md_file, 'r', encoding='utf-8') as f:
                md_content = f.read()
            
            html_content = f"""
            <html>
            <head>
                <meta charset="utf-8">
                <style>
                    body {{ font-family: 'Liberation Serif', serif; margin: 1in; }}
                    h1 {{ page-break-before: always; font-size: 24pt; }}
                    h2 {{ font-size: 18pt; margin-top: 0.5in; }}
                    h3 {{ font-size: 14pt; }}
                    p {{ line-height: 1.15; font-size: 11pt; }}
                    code {{ font-family: 'Courier New', monospace; }}
                    pre {{ background: #f0f0f0; padding: 10px; }}
                </style>
            </head>
            <body>
                <pre>{md_content}</pre>
            </body>
            </html>
            """
            
            HTML(string=html_content).write_pdf(pdf_file)
            
            if Path(pdf_file).exists():
                size_kb = Path(pdf_file).stat().st_size / 1024
                print(f"  ✓ PDF created")
                print(f"  Size: {size_kb:.1f} KB")
                print()
                print("✅ CONVERSION COMPLETE")
                return True
                
        except ImportError:
            print("  No suitable PDF library found")
            return False
    
    return False

if __name__ == "__main__":
    success = convert_md_to_pdf()
    
    print()
    print("=" * 50)
    if success:
        print("PDF READY FOR EXPERT SUBMISSION")
        print("=" * 50)
        print("Next steps:")
        print("  1. Verify PDF opens correctly")
        print("  2. Check formatting and readability")
        print("  3. Send to expert reviewers (March 24)")
    else:
        print("PDF CONVERSION ENCOUNTERED ISSUES")
        print("=" * 50)
        print("Alternative: Use online converter or manual tool")
    
    sys.exit(0 if success else 1)
