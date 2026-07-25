#!/usr/bin/env python3
"""
Simple PDF Converter for P vs NP Proof
Uses reportlab to create a publication-ready PDF from Markdown
"""

from pathlib import Path
from reportlab.lib.pagesizes import letter, A4
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import (SimpleDocTemplate, Paragraph, Spacer, 
                                PageBreak, Table, TableStyle)
from reportlab.lib import colors
import re

def read_markdown(md_file):
    """Read markdown file with UTF-8 encoding"""
    with open(md_file, 'r', encoding='utf-8') as f:
        return f.read()

def markdown_to_pdf(md_file, pdf_file):
    """Convert Markdown to PDF using reportlab"""
    
    print("=" * 60)
    print("PDF Converter - P vs NP Proof")
    print("=" * 60)
    print()
    print(f"Source: {md_file}")
    print(f"Output: {pdf_file}")
    print()
    
    # Check source file exists
    if not Path(md_file).exists():
        print(f"❌ ERROR: File not found: {md_file}")
        return False
    
    try:
        print("Step 1: Reading markdown file...")
        content = read_markdown(md_file)
        print(f"  ✓ Read {len(content)} characters")
        print()
        
        # Create PDF document
        print("Step 2: Creating PDF document...")
        doc = SimpleDocTemplate(
            pdf_file, 
            pagesize=letter,
            rightMargin=0.75*inch, 
            leftMargin=0.75*inch,
            topMargin=1*inch, 
            bottomMargin=0.75*inch,
            title="P vs NP Proof - March 2026",
            author="Research"
        )
        
        # Define styles
        styles = getSampleStyleSheet()
        normal_style = styles['Normal']
        normal_style.fontSize = 11
        normal_style.leading = 14
        
        heading1_style = ParagraphStyle(
            'Heading1Custom',
            parent=styles['Heading1'],
            fontSize=16,
            textColor=colors.HexColor('#1a1a1a'),
            spaceAfter=12,
            spaceBefore=12,
            fontName='Helvetica-Bold'
        )
        
        heading2_style = ParagraphStyle(
            'Heading2Custom',
            parent=styles['Heading2'],
            fontSize=13,
            textColor=colors.HexColor('#333333'),
            spaceAfter=8,
            spaceBefore=10,
            fontName='Helvetica-Bold'
        )
        
        # Build story (document content)
        story = []
        lines = content.split('\n')
        
        print("Step 3: Processing content...")
        processed_lines = 0
        
        for line in lines:
            stripped = line.strip()
            
            if not stripped:
                # Empty line = spacer
                story.append(Spacer(1, 0.1*inch))
                
            elif stripped.startswith('# '):
                # Heading 1
                text = stripped[2:].strip()
                text = re.sub(r'<[^>]+>', '', text)  # Remove HTML tags if any
                story.append(Paragraph(text, heading1_style))
                story.append(Spacer(1, 0.15*inch))
                
            elif stripped.startswith('## '):
                # Heading 2
                text = stripped[3:].strip()
                text = re.sub(r'<[^>]+>', '', text)
                story.append(Paragraph(text, heading2_style))
                story.append(Spacer(1, 0.1*inch))
                
            elif stripped.startswith('### '):
                # Heading 3
                text = stripped[4:].strip()
                text = re.sub(r'<[^>]+>', '', text)
                style = ParagraphStyle(
                    'Heading3',
                    parent=normal_style,
                    fontSize=12,
                    fontName='Helvetica-Bold',
                    textColor=colors.HexColor('#444444'),
                    spaceAfter=6,
                    spaceBefore=8
                )
                story.append(Paragraph(text, style))
                story.append(Spacer(1, 0.08*inch))
                
            elif stripped.startswith('- ') or stripped.startswith('* '):
                # Bullet point
                text = stripped[2:].strip()
                text = f"• {text}"
                story.append(Paragraph(text, normal_style))
                story.append(Spacer(1, 0.05*inch))
                
            elif stripped.startswith('| '):
                # Table row - skip for now (simplified)
                story.append(Paragraph(stripped, normal_style))
                story.append(Spacer(1, 0.05*inch))
                
            else:
                # Regular paragraph
                # Escape special characters
                text = stripped.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
                if text:
                    story.append(Paragraph(text, normal_style))
                    story.append(Spacer(1, 0.08*inch))
            
            processed_lines += 1
        
        print(f"  ✓ Processed {processed_lines} lines")
        print()
        
        # Build PDF
        print("Step 4: Building PDF...")
        doc.build(story)
        print("  ✓ PDF built successfully")
        print()
        
        # Verify output
        if Path(pdf_file).exists():
            size_kb = Path(pdf_file).stat().st_size / 1024
            size_mb = size_kb / 1024
            
            if size_mb > 1:
                size_str = f"{size_mb:.2f} MB"
            else:
                size_str = f"{size_kb:.1f} KB"
            
            print("=" * 60)
            print("✅ PDF CONVERSION SUCCESSFUL")
            print("=" * 60)
            print(f"Output file: {pdf_file}")
            print(f"File size: {size_str}")
            print()
            return True
        else:
            print("❌ ERROR: PDF file was not created")
            return False
            
    except Exception as e:
        print(f"❌ ERROR during conversion: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == '__main__':
    # Get the paths
    workspace = Path('c:/Users/skathera/Downloads/killer_V2_RS_M11')
    md_file = workspace / 'EXPERT_SUBMISSION_MARCH24' / 'P_vs_NP_PROOF_FINAL_MARCH2026.md'
    pdf_file = workspace / 'EXPERT_SUBMISSION_MARCH24' / 'P_vs_NP_PROOF_FINAL_MARCH2026.pdf'
    
    # Convert
    success = markdown_to_pdf(str(md_file), str(pdf_file))
    
    exit(0 if success else 1)
