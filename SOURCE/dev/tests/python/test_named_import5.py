#!/usr/bin/env python3
import sys
sys.path.insert(0, r'c:\Users\skathera\Downloads\killer\src')

from lexer import Lexer
from parser import Parser, ImportStatement

code = 'import { square, cube, factorial, isPrime, PI } from "./legacy-modules/math_utils";'

try:
    lexer = Lexer(code)
    tokens = lexer.tokenize()
    
    print("Tokens:")
    for token in tokens:
        print(f"  {token.type.name}: {token.value}")
    
    parser = Parser(tokens)
    ast = parser.parse()
    
    for stmt in ast.statements:
        print(f"\nStatement type: {type(stmt).__name__}")
        if isinstance(stmt, ImportStatement):
            print(f"  imports: {stmt.imports}")
            print(f"  module_path: {stmt.module_path}")
            print(f"  alias: {stmt.alias}")
except Exception as e:
    print(f"\nError: {e}")
    import traceback
    traceback.print_exc()
