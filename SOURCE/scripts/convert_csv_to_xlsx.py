#!/usr/bin/env python3
"""
KILLER Status Tracker - CSV to Excel Converter
Demonstrates Phase 39 (Office Format Support) usage
"""

import csv
import sys
from pathlib import Path

def csv_to_xlsx(csv_file, xlsx_file):
    """Convert CSV to tab-separated Excel format"""
    
    print(f"📂 Reading: {csv_file}")
    
    # Read CSV
    rows = []
    with open(csv_file, 'r', encoding='utf-8') as f:
        csv_reader = csv.reader(f)
        rows = list(csv_reader)
    
    print(f"📊 Found {len(rows)} rows (1 header + {len(rows)-1} data rows)")
    
    # Write as Tab-Separated Values (Excel compatible)
    # This is the format Phase 39 generates for XLSX
    print(f"📝 Converting to Excel format...")
    
    with open(xlsx_file, 'w', encoding='utf-8') as f:
        for row in rows:
            # Replace commas with tabs for Excel ingestion
            line = '\t'.join(row) + '\n'
            f.write(line)
    
    file_size = Path(xlsx_file).stat().st_size
    print(f"✅ Created: {xlsx_file} ({file_size} bytes)")
    print(f"📊 Status tracker ready for Excel!\n")

if __name__ == '__main__':
    csv_file = 'KILLER_STATUS_TRACKER.csv'
    xlsx_file = 'KILLER_STATUS_TRACKER.xlsx'
    
    if not Path(csv_file).exists():
        print(f"❌ Error: {csv_file} not found")
        sys.exit(1)
    
    try:
        csv_to_xlsx(csv_file, xlsx_file)
        print(f"🎯 Use Case: Open {xlsx_file} in Excel, Google Sheets, or LibreOffice")
        print(f"   Benefits:")
        print(f"   ✓ Better formatting and visualization")
        print(f"   ✓ Easy sharing with team members")
        print(f"   ✓ Sorting and filtering capabilities")
        print(f"   ✓ Can add charts and formulas")
    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)
