# Killer language parity (generated)

Every probe below was executed against the real `killer_super` binary. A probe passes only if the
output matches what the equivalent Python/JS/Java program prints. Regenerate with
`python parity/run_parity.py --md`. Source of truth: `parity/probes.py`.

**Overall: 76 of 166 probes pass (46%).**

| Area | Pass | Total |
|------|-----:|------:|
| Basics | 8 | 21 |
| Control flow | 8 | 14 |
| Functions | 4 | 22 |
| OOP | 5 | 15 |
| Collections | 13 | 24 |
| Strings | 9 | 19 |
| Math | 6 | 10 |
| Errors | 2 | 9 |
| Iterators | 4 | 5 |
| Modules & I/O | 3 | 9 |
| Concurrency | 3 | 5 |
| Types | 5 | 6 |
| Killer-specific | 6 | 7 |

Status legend: PASS = identical output; WRONG = ran but printed something different; ERROR = the program failed to parse or run.

## Basics

| Probe | Status | Detail |
|-------|--------|--------|
| arithmetic precedence | PASS |  |
| power operator ** | PASS |  |
| modulo | PASS |  |
| floor division // | ERROR | ERROR: Parse error: Line 1: unclosed `(` or `[` in function signature |
| float sum prints like IEEE | PASS |  |
| string repetition | ERROR | ERROR: Runtime error: Type error in '*': left operand must be a number, got string |
| boolean && || ! | PASS |  |
| boolean keywords and/or/not | ERROR | ERROR: Parse error: Line 1: unsupported expression `true and false` |
| chained comparison | WRONG | got 'false', expected 'true' |
| ternary expression | ERROR | ERROR: Parse error: Line 2: unsupported expression `2 ? "big" : "small"` |
| python-style conditional expr | ERROR | ERROR: Parse error: Line 2: unsupported expression `"big" if x` |
| multiple assignment | ERROR | ERROR: Parse error: Line 1: unsupported Killer subset statement `a, b = 1, 2` |
| swap via tuple | ERROR | ERROR: Parse error: Line 3: unsupported Killer subset statement `a, b = b, a` |
| augmented assignment | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `x += 5` |
| increment ++ | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `x++` |
| null handling | PASS |  |
| array equality | PASS |  |
| string equality and ordering | ERROR | ERROR: Runtime error: Cannot convert value to number |
| bitwise operators | ERROR | ERROR: Parse error: Line 1: unsupported expression `6 & 3` |
| big integers | PASS |  |
| hex literal | ERROR | ERROR: Parse error: Line 1: unsupported expression `0xFF` |

## Control flow

| Probe | Status | Detail |
|-------|--------|--------|
| if / else if / else | PASS |  |
| elif keyword | ERROR | ERROR: Parse error: Line 4: unsupported Killer subset statement `elif x < 10` |
| while loop | PASS |  |
| for over range | PASS |  |
| range with step | PASS |  |
| break and continue | PASS |  |
| nested loops | PASS |  |
| do-while | ERROR | ERROR: Parse error: Line 4: expected `{` after condition (found `println(i)`) |
| C-style for | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `for (i = 0` |
| for over dict keys | PASS |  |
| enumerate | ERROR | ERROR: Runtime error: Undefined variable `i` |
| match statement | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `match x` |
| switch statement | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `switch x` |
| while true + break | PASS |  |

## Functions

| Probe | Status | Detail |
|-------|--------|--------|
| define and call | PASS |  |
| recursion | PASS |  |
| deep recursion 5000 | PASS |  |
| default argument | WRONG | got 'null Sai\nYo Sai', expected 'Hi Sai\nYo Sai' |
| variadic arguments | ERROR | ERROR: Parse error: Line 1: Parse error: Line 1: invalid parameter name `...xs` |
| return multiple values | ERROR | ERROR: Parse error: Line 4: unsupported Killer subset statement `a, b = two()` |
| no explicit return | ERROR | ERROR: Runtime error: Stack underflow |
| function stored in variable | ERROR | ERROR: Parse error: Line 5: unknown function `f` |
| lambda expression | ERROR | ERROR: Parse error: Line 2: `return` is only valid inside a function |
| arrow lambda | ERROR | ERROR: Parse error: Line 1: unsupported expression `(a) =` |
| closure keeps state | ERROR | ERROR: Parse error: Line 10: unknown function `c` |
| function passed as argument | ERROR | ERROR: Parse error: Line 2: unknown function `f` |
| map with lambda | ERROR | ERROR: Parse error: Line 1: unsupported expression `fn(x) { return x * 10 }` |
| map with named function | ERROR | ERROR: Runtime error: Undefined variable `d` |
| filter and reduce | ERROR | ERROR: Runtime error: Undefined variable `even` |
| decorator | ERROR | ERROR: Parse error: Line 3: unknown function `f` |
| function in dict | ERROR | ERROR: Parse error: Line 5: unsupported expression `d["say"]()` |
| read global from function | ERROR | ERROR: Runtime error: Undefined variable `g` |
| write global from function | ERROR | ERROR: Runtime error: Undefined variable `total` |
| global keyword | ERROR | ERROR: Parse error: Line 2: Parse error: Line 3: unsupported Killer subset statement `global total` |
| keyword arguments | ERROR | ERROR: Parse error: Line 4: unsupported expression `b = 1` |
| mutual recursion | PASS |  |

## OOP

| Probe | Status | Detail |
|-------|--------|--------|
| class, init and method | PASS |  |
| state mutation through methods | PASS |  |
| inheritance and override | WRONG | got 'null barks', expected 'Rex barks' |
| inherited method + init | WRONG | got 'null makes a sound\nnull fetches', expected 'Rex makes a sound\nRex fetches' |
| super method call | ERROR | ERROR: Runtime error: Undefined variable `super` |
| super in init | ERROR | ERROR: Runtime error: Undefined variable `super` |
| objects are references | PASS |  |
| list of objects | PASS |  |
| polymorphism | WRONG | got 'null\nmeow', expected 'a makes a sound\nmeow' |
| operator overloading __add__ | WRONG | got 'null', expected '3' |
| custom toString when printed | WRONG | got '<P instance>', expected 'P(1)' |
| instanceof / type check | ERROR | ERROR: Parse error: Line 10: unsupported expression `a instanceof Animal` |
| static / class method | ERROR | ERROR: Parse error: Line 1: Parse error: Line 2: expected method definition (kfn) inside class, got `static fn twice(x)` |
| method chaining | ERROR | ERROR: Parse error: Line 11: unsupported Killer subset statement `b.add("a").add("b")` |
| field default and missing field | PASS |  |

## Collections

| Probe | Status | Detail |
|-------|--------|--------|
| list basics | PASS |  |
| negative index | WRONG | got '1', expected '3' |
| slicing | ERROR | ERROR: Parse error: Line 2: unsupported expression `1:3` |
| slice function | PASS |  |
| pop and insert | ERROR | ERROR: Runtime error: (at instruction 11): Runtime error: hash_map_insert: first arg must be a Dict |
| sort and reverse | PASS |  |
| membership with in | ERROR | ERROR: Parse error: Line 1: unsupported expression `2 in` |
| contains function | PASS |  |
| list comprehension | PASS |  |
| list comprehension with filter | PASS |  |
| nested lists | PASS |  |
| list concatenation | ERROR | ERROR: Runtime error: Cannot add these types |
| sum min max | PASS |  |
| dict basics | PASS |  |
| dict missing key is null | PASS |  |
| dict membership | ERROR | ERROR: Parse error: Line 2: unsupported expression `"a" in d` |
| dict delete | ERROR | ERROR: Parse error: Line 2: unknown function `delete` |
| dict values and items | PASS |  |
| dict comprehension | ERROR | ERROR: Parse error: Line 1: unsupported expression `2 for k in range(3)` |
| nested dict | PASS |  |
| set operations | ERROR | ERROR: Parse error: Line 1: unknown function `set` |
| tuple literal | ERROR | ERROR: Parse error: Line 1: unsupported expression `1, 2, 3` |
| sorted with key function | ERROR | ERROR: Runtime error: Undefined variable `neg` |
| zip | PASS |  |

## Strings

| Probe | Status | Detail |
|-------|--------|--------|
| length upper lower | PASS |  |
| string indexing | ERROR | ERROR: Runtime error: Cannot index hello with 1 |
| string slicing | ERROR | ERROR: Parse error: Line 2: unsupported expression `1:3` |
| split and join | PASS |  |
| replace and strip | PASS |  |
| find / index_of | PASS |  |
| startswith endswith | PASS |  |
| string contains | PASS |  |
| str() and number parse | PASS |  |
| f-string interpolation | ERROR | ERROR: Parse error: Line 2: unsupported expression `f"n is {n}"` |
| k-string interpolation | PASS |  |
| escape sequences | PASS |  |
| single-quoted strings | ERROR | ERROR: Parse error: Line 1: unsupported expression `'hi'` |
| multiline string | ERROR | ERROR: Parse error: Line 1: unsupported expression `"""a` |
| reverse a string | ERROR | ERROR: Runtime error: (at instruction 1): Runtime error: reverse() expects an array |
| character codes | ERROR | ERROR: Parse error: Line 1: unknown function `ord` |
| unicode | WRONG | got '6', expected '5' |
| format with padding | ERROR | ERROR: Parse error: Line 1: unknown function `pad_left` |
| iterate characters | ERROR | ERROR: Runtime error: Cannot index abc with 0 |

## Math

| Probe | Status | Detail |
|-------|--------|--------|
| sqrt pow abs | PASS |  |
| floor ceil round | PASS |  |
| int and float conversion | ERROR | ERROR: Parse error: Line 2: unknown function `float` |
| trig | PASS |  |
| constants pi and e | ERROR | ERROR: Runtime error: Undefined variable `PI` |
| random in range | PASS |  |
| log and exp | ERROR | ERROR: Parse error: Line 1: unknown function `log` |
| gcd | ERROR | ERROR: Parse error: Line 1: unknown function `gcd` |
| division by zero is an error | PASS |  |
| integer vs float printing | PASS |  |

## Errors

| Probe | Status | Detail |
|-------|--------|--------|
| try / catch / throw | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `throw "boom"` |
| catch a runtime error | ERROR | ERROR: Parse error: Line 3: unsupported Killer subset statement `catch e` |
| finally always runs | ERROR | ERROR: Runtime error: Undefined variable `try` |
| catch then finally | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `throw "x"` |
| custom error value | ERROR | ERROR: Parse error: Line 2: unsupported Killer subset statement `throw {"code": 42}` |
| error propagates through calls | ERROR | ERROR: Parse error: Line 1: Parse error: Line 2: unsupported Killer subset statement `throw "deep"` |
| assert passes | ERROR | ERROR: Parse error: Line 1: unknown function `assert` |
| undefined variable is an error | PASS |  |
| script exits non-zero on error | PASS |  |

## Iterators

| Probe | Status | Detail |
|-------|--------|--------|
| generator with yield | ERROR | ERROR: Parse error: Line 1: Parse error: Line 2: unsupported Killer subset statement `yield 1` |
| range materialises | PASS |  |
| zip in for | PASS |  |
| iterate dict items | PASS |  |
| reversed iteration | PASS |  |

## Modules & I/O

| Probe | Status | Detail |
|-------|--------|--------|
| import another file | ERROR | ERROR: Parse error: Line 2: unknown function `helper_add` |
| write and read a file | PASS |  |
| file exists | PASS |  |
| json round trip | ERROR | ERROR: Parse error: Line 1: unsupported expression `'{"a": 1, "b": [1, 2]}'` |
| read environment variable | ERROR | ERROR: Parse error: Line 1: unknown function `env` |
| current time is a number | PASS |  |
| regex match | WRONG | got 'false', expected 'true' |
| command line args available | ERROR | ERROR: Parse error: Line 1: unknown function `args` |
| sleep | ERROR | ERROR: Runtime error: (at instruction 1): Runtime error: unknown function 'sleep' -- did you mean 'len'? |

## Concurrency

| Probe | Status | Detail |
|-------|--------|--------|
| async function and await | PASS |  |
| spawn and await | PASS |  |
| async_spawn / async_await builtins | ERROR | ERROR: Runtime error: Undefined variable `w` |
| channels | PASS |  |
| mutex | ERROR | ERROR: Parse error: Line 1: unknown function `mutex_new` |

## Types

| Probe | Status | Detail |
|-------|--------|--------|
| type annotations run | PASS |  |
| type error is caught early | PASS |  |
| typeof / type() | PASS |  |
| conversion between types | ERROR | ERROR: Parse error: Line 2: unknown function `bool` |
| null coalescing | PASS |  |
| optional chaining | PASS |  |

## Killer-specific

| Probe | Status | Detail |
|-------|--------|--------|
| uncertain arithmetic | PASS |  |
| three-valued comparison | PASS |  |
| gauss quadrature | PASS |  |
| trit constants | PASS |  |
| native JIT result matches | PASS |  |
| C FFI call | PASS |  |
| statistics builtins | ERROR | ERROR: Parse error: Line 1: unknown function `mean` |
