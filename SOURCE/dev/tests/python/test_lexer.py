#!/usr/bin/env python3
import sys
sys.path.insert(0, r'c:\Users\skathera\Downloads\killer\src')

from lexer import Lexer, TokenType

code = 'import * as math from "./legacy-modules/math_utils";'

lexer = Lexer(code)
tokens = lexer.tokenize()

for token in tokens:
    print(f"{token.type.name}: {token.value}")
