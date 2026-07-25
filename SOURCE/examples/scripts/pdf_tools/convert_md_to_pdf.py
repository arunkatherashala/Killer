#!/usr/bin/env python3
"""
Convert Markdown file to PDF using weasyprint and markdown
"""

import markdown
from weasyprint import HTML, CSS
from pathlib import Path
import sys

def md_to_pdf(md_file, pdf_file, title="Document"):
    """Convert markdown file to PDF"""
    
    # Read markdown file
    with open(md_file, 'r', encoding='utf-8') as f:
        md_content = f.read()
    
    # Convert markdown to HTML
    html_content = markdown.markdown(md_content, extensions=['tables', 'fenced_code', 'codehilite', 'toc', 'sane_lists'])
    
    # Create full HTML document with styling
    full_html = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <title>{title}</title>
        <style>
            body {{
                font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                line-height: 1.6;
                color: #333;
                margin: 40px;
                background-color: #fff;
            }}
            h1, h2, h3, h4, h5, h6 {{
                color: #1a1a1a;
                margin-top: 1.5em;
                margin-bottom: 0.5em;
            }}
            h1 {{
                font-size: 2.2em;
                border-bottom: 2px solid #007bff;
                padding-bottom: 0.3em;
            }}
            h2 {{
                font-size: 1.8em;
                border-bottom: 1px solid #ddd;
                padding-bottom: 0.2em;
            }}
            h3 {{
                font-size: 1.4em;
            }}
            code {{
                background-color: #f4f4f4;
                padding: 2px 4px;
                border-radius: 3px;
                font-family: 'Courier New', monospace;
            }}
            pre {{
                background-color: #f5f5f5;
                border: 1px solid #ddd;
                border-radius: 4px;
                padding: 12px;
                overflow-x: auto;
                font-family: 'Courier New', monospace;
                line-height: 1.4;
            }}
            pre code {{
                background-color: transparent;
                padding: 0;
            }}
            blockquote {{
                border-left: 4px solid #007bff;
                padding-left: 15px;
                color: #666;
                margin: 1em 0;
            }}
            table {{
                border-collapse: collapse;
                width: 100%;
                margin: 1em 0;
            }}
            table th {{
                background-color: #f0f0f0;
                border: 1px solid #ddd;
                padding: 10px;
                text-align: left;
                font-weight: bold;
            }}
            table td {{
                border: 1px solid #ddd;
                padding: 8px;
            }}
            table tr:nth-child(even) {{
                background-color: #f9f9f9;
            }}
            strong {{
                color: #000;
            }}
            em {{
                color: #555;
            }}
            a {{
                color: #007bff;
                text-decoration: none;
            }}
            a:hover {{
                text-decoration: underline;
            }}
        </style>
    </head>
    <body>
        {html_content}
    </body>
    </html>
    """
    
    # Convert HTML to PDF
    HTML(string=full_html).write_pdf(pdf_file)
    print(f"✓ PDF created successfully: {pdf_file}")
    print(f"  File size: {Path(pdf_file).stat().st_size:,} bytes")

if __name__ == "__main__":
    md_file = r"_CURRENT_WORK\P_vs_NP_SOLUTION\P_vs_NP_PROOF_FINAL_MARCH2026.md"
    pdf_file = r"_CURRENT_WORK\P_vs_NP_SOLUTION\P_vs_NP_PROOF_FINAL_MARCH2026.pdf"
    
    print(f"Converting: {md_file}")
    print(f"Output PDF: {pdf_file}")
    print("Processing...")
    
    try:
        md_to_pdf(md_file, pdf_file, title="P vs NP: A Complete Proof via Resolution Proof Complexity")
        print("\n✓ Conversion complete!")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        sys.exit(1)
