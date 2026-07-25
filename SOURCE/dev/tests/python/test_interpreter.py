#!/usr/bin/env python3
import sys
sys.path.insert(0, r'c:\Users\skathera\Downloads\killer\src')

from lexer import Lexer
from parser import Parser
from interpreter import Interpreter

code = 'import * as math from "./legacy-modules/math_utils"; print("Done!");'

try:
    lexer = Lexer(code)
    tokens = lexer.tokenize()
    
    parser = Parser(tokens)
    ast = parser.parse()
    
    interpreter = Interpreter()
    interpreter.interpret(ast)
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
