#!/usr/bin/env python3
"""
Convert Markdown file to HTML (for PDF print from browser)
"""

import markdown
from pathlib import Path

def md_to_html(md_file, html_file, title="Document"):
    """Convert markdown file to HTML"""
    
    # Read markdown file
    with open(md_file, 'r', encoding='utf-8') as f:
        md_content = f.read()
    
    # Convert markdown to HTML
    html_content = markdown.markdown(md_content, extensions=['tables', 'fenced_code', 'toc', 'sane_lists'])
    
    # Create full HTML document with print-friendly styling
    full_html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Georgia', 'Times New Roman', serif;
            line-height: 1.7;
            color: #222;
            background-color: #fff;
            padding: 20px;
        }}
        
        @media print {{
            body {{
                padding: 0;
                margin: 0;
            }}
            .page-break {{
                page-break-after: always;
            }}
            h1 {{
                page-break-after: avoid;
            }}
            h2, h3, h4, h5, h6 {{
                page-break-after: avoid;
                page-break-inside: avoid;
            }}
        }}
        
        .container {{
            max-width: 900px;
            margin: 0 auto;
            background-color: white;
        }}
        
        h1 {{
            font-size: 2.4em;
            font-weight: bold;
            margin-top: 1.2em;
            margin-bottom: 0.8em;
            color: #000;
            border-bottom: 3px solid #1a73e8;
            padding-bottom: 0.4em;
            page-break-after: avoid;
        }}
        
        h2 {{
            font-size: 1.8em;
            font-weight: bold;
            margin-top: 1.4em;
            margin-bottom: 0.6em;
            color: #000;
            border-bottom: 2px solid #dadce0;
            padding-bottom: 0.3em;
            page-break-after: avoid;
        }}
        
        h3 {{
            font-size: 1.4em;
            font-weight: bold;
            margin-top: 1.2em;
            margin-bottom: 0.5em;
            color: #1a73e8;
            page-break-after: avoid;
        }}
        
        h4, h5, h6 {{
            font-size: 1.1em;
            font-weight: bold;
            margin-top: 1em;
            margin-bottom: 0.5em;
            page-break-after: avoid;
        }}
        
        p {{
            margin-bottom: 1em;
            text-align: justify;
        }}
        
        a {{
            color: #1a73e8;
            text-decoration: none;
        }}
        
        a:hover {{
            text-decoration: underline;
        }}
        
        code {{
            background-color: #f5f5f5;
            color: #d32f2f;
            padding: 2px 6px;
            border-radius: 2px;
            font-family: 'Courier New', monospace;
            font-size: 0.95em;
        }}
        
        pre {{
            background-color: #f5f5f5;
            border: 1px solid #dadce0;
            border-radius: 3px;
            padding: 15px;
            overflow-x: auto;
            margin: 1em 0;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            line-height: 1.4;
        }}
        
        pre code {{
            background-color: transparent;
            color: #222;
            padding: 0;
            border-radius: 0;
        }}
        
        blockquote {{
            border-left: 4px solid #1a73e8;
            padding-left: 16px;
            margin: 1em 0;
            color: #555;
            font-style: italic;
        }}
        
        ul, ol {{
            margin: 1em 0 1em 2em;
        }}
        
        li {{
            margin-bottom: 0.5em;
        }}
        
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 1.5em 0;
        }}
        
        table th {{
            background-color: #f0f0f0;
            border: 1px solid #dadce0;
            padding: 10px;
            text-align: left;
            font-weight: bold;
            font-size: 0.95em;
        }}
        
        table td {{
            border: 1px solid #dadce0;
            padding: 10px;
            font-size: 0.95em;
        }}
        
        table tr:nth-child(even) {{
            background-color: #f9f9f9;
        }}
        
        strong {{
            font-weight: bold;
        }}
        
        em {{
            font-style: italic;
        }}
        
        hr {{
            border: none;
            border-top: 1px solid #dadce0;
            margin: 2em 0;
        }}
        
        .header-info {{
            background-color: #f9f9f9;
            border: 1px solid #dadce0;
            border-radius: 3px;
            padding: 12px;
            margin-bottom: 2em;
            font-size: 0.95em;
        }}
        
        .header-info strong {{
            color: #1a73e8;
        }}
    </style>
</head>
<body>
    <div class="container">
        {html_content}
    </div>
    
    <script>
        // Helper: Print to PDF
        function printToPDF() {{
            window.print();
        }}
        
        // Add print button
        document.addEventListener('DOMContentLoaded', function() {{
            const printBtn = document.createElement('button');
            printBtn.textContent = '📄 Print to PDF';
            printBtn.style.cssText = 'position: fixed; top: 10px; right: 10px; padding: 8px 12px; background: #1a73e8; color: white; border: none; border-radius: 3px; cursor: pointer; z-index: 1000; font-size: 0.9em;';
            printBtn.onclick = printToPDF;
            document.body.appendChild(printBtn);
        }});
    </script>
</body>
</html>"""
    
    # Write HTML file
    with open(html_file, 'w', encoding='utf-8') as f:
        f.write(full_html)
    
    print(f"✓ HTML file created: {html_file}")
    print(f"  File size: {Path(html_file).stat().st_size:,} bytes")
    print("\nTo convert to PDF:")
    print("  1. Open the HTML file in a web browser")
    print("  2. Click 'Print to PDF' button (top-right)")
    print("  3. Or: Ctrl+P → Save as PDF")

if __name__ == "__main__":
    md_file = r"_CURRENT_WORK\P_vs_NP_SOLUTION\P_vs_NP_PROOF_FINAL_MARCH2026.md"
    html_file = r"_CURRENT_WORK\P_vs_NP_SOLUTION\P_vs_NP_PROOF_FINAL_MARCH2026.html"
    
    print(f"Converting: {md_file}")
    print(f"Output HTML: {html_file}")
    print("Processing...")
    
    try:
        md_to_html(md_file, html_file, title="P vs NP: A Complete Proof via Resolution Proof Complexity - Katherashala Sai Arun Kumar")
        print("\n✓ Conversion complete!")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()
