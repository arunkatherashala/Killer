#!/usr/bin/env python3
"""
Generate PDF from Markdown - P vs NP Proof
Uses pypdf/reportlab for reliable PDF generation
"""

import sys
from pathlib import Path

def convert_html_to_pdf_simple():
    """Convert the existing HTML file to PDF using simple approach"""
    html_file = Path("EXPERT_SUBMISSION_MARCH24/P_vs_NP_PROOF_FINAL_MARCH2026.html")
    pdf_file = Path("EXPERT_SUBMISSION_MARCH24/P_vs_NP_PROOF_FINAL_MARCH2026.pdf")
    
    if not html_file.exists():
        print(f"❌ HTML file not found: {html_file}")
        return False
    
    # Try different PDF conversion methods
    methods = [
        ("fpdf", "from fpdf import FPDF"),
        ("PyPDF2", "import PyPDF2"),
        ("reportlab", "from reportlab.lib.pagesizes import letter; from reportlab.pdfgen import canvas"),
    ]
    
    for method_name, import_stmt in methods:
        try:
            exec(import_stmt)
            print(f"✅ Using {method_name} for PDF conversion")
            
            if method_name == "fpdf":
                # FPDF approach
                pdf = FPDF()
                pdf.add_page()
                pdf.set_font("Arial", size=11)
                
                # Read HTML and extract text
                with open(html_file, 'r', encoding='utf-8') as f:
                    html_content = f.read()
                
                # Simple HTML stripping
                import re
                text_content = re.sub('<[^<]+?>', '', html_content)
                text_content = text_content.replace('&nbsp;', ' ').replace('<br>', '\n')
                
                # Add text to PDF
                for line in text_content.split('\n')[:500]:  # First 500 lines
                    if line.strip():
                        pdf.multi_cell(0, 10, line[:100])
                
                pdf.output(str(pdf_file))
                print(f"✅ PDF created: {pdf_file}")
                print(f"   Size: {pdf_file.stat().st_size / 1024:.1f} KB")
                return True
                
        except ImportError:
            continue
        except Exception as e:
            print(f"❌ Error with {method_name}: {e}")
            continue
    
    print("\n⚠️  No Python PDF libraries available")
    print("\nRECOMMENDED: Convert using browser")
    print("1. Open the HTML file in Chrome/Firefox/Edge")
    print("2. Press Ctrl+P (or Cmd+P on Mac)")
    print("3. Save as PDF with settings:")
    print("   - Paper size: A4")
    print("   - Margins: Normal")
    print("   - Format: Save as PDF")
    print(f"4. Save to: {pdf_file}")
    return False

if __name__ == "__main__":
    print("=" * 60)
    print("P vs NP PROOF - PDF CONVERSION")
    print("=" * 60)
    print()
    
    convert_html_to_pdf_simple()
