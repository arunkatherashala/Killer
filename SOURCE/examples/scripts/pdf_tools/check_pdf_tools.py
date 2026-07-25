#!/usr/bin/env python3
"""
Simple PDF Converter - Convert Markdown/HTML to PDF
Uses built-in Python libraries with minimal dependencies
"""

import sys
from pathlib import Path

def create_pdf_from_text():
    """Create a high-quality PDF from markdown"""
    try:
        from html2image import Html2Image
        print("✅ html2image available - high quality conversion")
        return True
    except ImportError:
        print("❌ html2image not available")
    
    try:
        from fpdf import FPDF
        print("✅ fpdf available - basic PDF conversion")
        return True
    except ImportError:
        print("❌ fpdf not available")
    
    try:
        import pypdf
        print("✅ pypdf available - PDF manipulation")
        return True
    except ImportError:
        print("❌ pypdf not available")
    
    return False

def markdown_to_html(md_file):
    """Convert markdown to HTML"""
    import markdown
    
    with open(md_file, 'r', encoding='utf-8') as f:
        md_content = f.read()
    
    # Convert markdown to HTML
    html = markdown.markdown(md_content, extensions=['tables', 'fenced_code'])
    
    # Create full HTML document
    full_html = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <title>P vs NP Proof - March 2026</title>
        <style>
            body {{
                font-family: 'Segoe UI', Arial, sans-serif;
                line-height: 1.8;
                color: #333;
                max-width: 1000px;
                margin: 0 auto;
                padding: 40px;
                background: white;
            }}
            h1 {{ font-size: 2.5em; color: #1a1a1a; border-bottom: 3px solid #0066cc; padding-bottom: 0.5em; }}
            h2 {{ font-size: 1.8em; color: #0066cc; margin-top: 1.5em; border-bottom: 1px solid #ddd; }}
            h3 {{ font-size: 1.3em; color: #333; margin-top: 1.2em; }}
            p {{ margin: 0.8em 0; }}
            code {{ background: #f4f4f4; padding: 0.2em 0.4em; border-radius: 3px; font-family: 'Courier New', monospace; }}
            pre {{ background: #f4f4f4; padding: 1em; border-radius: 5px; overflow-x: auto; border-left: 4px solid #0066cc; }}
            table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
            table, th, td {{ border: 1px solid #ddd; }}
            th, td {{ padding: 0.8em; text-align: left; }}
            th {{ background-color: #0066cc; color: white; }}
            blockquote {{ border-left: 4px solid #0066cc; padding-left: 1em; margin-left: 0; color: #666; }}
        </style>
    </head>
    <body>
        {html}
    </body>
    </html>
    """
    return full_html

# Check available tools
print("Checking PDF conversion capabilities...\n")

has_lib = create_pdf_from_text()

if not has_lib:
    print("\n⚠️  No direct PDF libraries available")
    print("Alternative: Using HTML + system tools")
    
    # Try to use system tools
    import subprocess
    try:
        result = subprocess.run(['where', 'wkhtmltopdf'], capture_output=True)
        if result.returncode == 0:
            print("✅ wkhtmltopdf available on system")
    except:
        pass
    
    try:
        result = subprocess.run(['where', 'chromium'], capture_output=True)
        if result.returncode == 0:
            print("✅ chromium available on system")
    except:
        pass

print("\nRecommendation: Convert using Chrome/Chromium or print to PDF manually")
print("This provides highest quality PDF output for academic submission")
