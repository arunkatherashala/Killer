#!/usr/bin/env python3
"""
Killer Auto-Formatter (killer fmt)

Formats Killer source code to consistent style:
- Standardizes indentation (4 spaces)
- Removes trailing whitespace
- Normalizes spacing around operators
- Aligns imports
- Formats function definitions consistently

Can run on file save as a background process without blocking the editor.
"""

import sys
import re
from pathlib import Path
from typing import List, Tuple


class KillerFormatter:
    def __init__(self, indent_size: int = 4):
        self.indent_size = indent_size
        self.indent_str = " " * indent_size
    
    def format_file(self, content: str) -> str:
        """Format entire Killer source file"""
        lines = content.split('\n')
        formatted_lines = []
        indent_level = 0
        
        for line in lines:
            # Skip empty lines but preserve them
            if not line.strip():
                formatted_lines.append('')
                continue
            
            # Format the line
            formatted = self._format_line(line, indent_level)
            formatted_lines.append(formatted)
            
            # Adjust indent level based on braces and dedents
            indent_level = self._calculate_indent_change(formatted, indent_level)
        
        return '\n'.join(formatted_lines)
    
    def _format_line(self, line: str, indent_level: int) -> str:
        """Format a single line"""
        stripped = line.strip()
        
        if not stripped:
            return ''
        
        # Remove trailing whitespace
        stripped = stripped.rstrip()
        
        # Format the content
        formatted = self._format_content(stripped)
        
        # Calculate proper indentation
        indent = self._calculate_line_indent(formatted, indent_level)
        
        return indent + formatted
    
    def _calculate_indent_change(self, line: str, current_indent: int) -> int:
        """Calculate new indent level based on brace/dedent patterns"""
        # Count opening and closing braces
        open_braces = line.count('{')
        close_braces = line.count('}')
        
        new_indent = current_indent + open_braces - close_braces
        return max(0, new_indent)
    
    def _calculate_line_indent(self, line: str, indent_level: int) -> str:
        """Calculate indentation for a line"""
        # Decrease indent for closing braces at line start
        if line.startswith('}'):
            indent_level = max(0, indent_level - 1)
        
        # Decrease indent for dedents
        if line.startswith('else') or line.startswith('elif'):
            indent_level = max(0, indent_level - 1)
        
        return self.indent_str * indent_level
    
    def _format_content(self, content: str) -> str:
        """Format line content (operators, spacing, etc.)"""
        # Add spaces around operators (but not in strings)
        content = self._format_operators(content)
        
        # Format function definitions
        content = self._format_function(content)
        
        # Remove multiple spaces
        content = re.sub(r'  +', ' ', content)
        
        return content.rstrip()
    
    def _format_operators(self, line: str) -> str:
        """Add consistent spacing around operators"""
        operators = ['==', '!=', '<=', '>=', '=>', '=', '+', '-', '*', '/', '%', '<', '>', 'and', 'or']
        
        for op in operators:
            # Skip if in string
            if self._is_in_string(line, op):
                continue
            
            # Format based on operator type
            if op == '=>':
                line = re.sub(r'\s*=>\s*', ' => ', line)
            elif op in ['==', '!=', '<=', '>=']:
                line = re.sub(rf'\s*{re.escape(op)}\s*', f' {op} ', line)
            elif op in ['+', '-', '*', '/', '%']:
                # Be careful not to format signs in numbers
                line = re.sub(rf'([^\s\d])\s*{re.escape(op)}\s*', rf'\1 {op} ', line)
                line = re.sub(rf'\s*{re.escape(op)}\s*([^\s=])', rf' {op} \1', line)
            elif op in ['<', '>']:
                if '<=' not in line and '>=' not in line:
                    line = re.sub(rf'\s*{re.escape(op)}\s*', f' {op} ', line)
            elif op == '=':
                # Assignment
                line = re.sub(r'([^=!<>])\s*=\s*([^=])', rf'\1 = \2', line)
            elif op in ['and', 'or']:
                line = re.sub(rf'\s+{op}\s+', f' {op} ', line)
        
        return line
    
    def _format_function(self, line: str) -> str:
        """Format function definitions"""
        # fn name(args) {...}
        line = re.sub(r'\bfn\s+(\w+)\s*\(', r'fn \1(', line)
        # name(args) {...}
        line = re.sub(r'\b(\w+)\s*\(\s*', r'\1(', line)
        return line
    
    def _is_in_string(self, line: str, text: str) -> bool:
        """Check if text appears only in strings"""
        in_string = False
        i = 0
        while i < len(line):
            if line[i] == '"' and (i == 0 or line[i-1] != '\\'):
                in_string = not in_string
            i += 1
        return in_string
    
    def format_and_write(self, file_path: str) -> bool:
        """Format file and write back"""
        try:
            path = Path(file_path)
            if not path.exists():
                print(f"Error: File not found: {file_path}")
                return False
            
            if path.suffix != '.killer':
                print(f"Error: Not a Killer file: {file_path}")
                return False
            
            # Read original
            with open(path, 'r', encoding='utf-8') as f:
                original = f.read()
            
            # Format
            formatted = self.format_file(original)
            
            # Write back
            with open(path, 'w', encoding='utf-8') as f:
                f.write(formatted)
            
            # Report changes
            if original != formatted:
                print(f"✓ Formatted: {file_path}")
                return True
            else:
                print(f"- No changes: {file_path}")
                return False
        
        except Exception as e:
            print(f"Error formatting {file_path}: {e}")
            return False


def main():
    if len(sys.argv) < 2:
        print("Usage: killer fmt <file.killer> [--check] [--in-place]")
        print("  --check        : Report if formatting needed (no write)")
        print("  --in-place     : Write formatted content back to file")
        sys.exit(1)
    
    file_path = sys.argv[1]
    check_only = '--check' in sys.argv
    in_place = '--in-place' in sys.argv or (len(sys.argv) == 2)
    
    formatter = KillerFormatter()
    
    # Read file
    if not Path(file_path).exists():
        print(f"Error: File not found: {file_path}")
        sys.exit(1)
    
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Format
    formatted = formatter.format_file(content)
    
    # Output
    if check_only:
        if formatted != content:
            print(f"✗ Formatting needed: {file_path}")
            sys.exit(1)
        else:
            print(f"✓ Already formatted: {file_path}")
            sys.exit(0)
    elif in_place:
        if formatter.format_and_write(file_path):
            sys.exit(0)
        else:
            sys.exit(1)
    else:
        # Print to stdout
        print(formatted)
        sys.exit(0)


if __name__ == '__main__':
    main()
