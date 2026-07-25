#!/usr/bin/env python3
import sys
sys.path.insert(0, r'c:\Users\skathera\Downloads\killer\src')

from lexer import Lexer
from parser import Parser, ImportStatement

code = 'import * as math from "./legacy-modules/math_utils";'

lexer = Lexer(code)
tokens = lexer.tokenize()

parser = Parser(tokens)
ast = parser.parse()

for stmt in ast.statements:
    print(f"Statement type: {type(stmt).__name__}")
    if isinstance(stmt, ImportStatement):
        print(f"  imports: {stmt.imports}")
        print(f"  module_path: {stmt.module_path}")
        print(f"  alias: {stmt.alias}")
