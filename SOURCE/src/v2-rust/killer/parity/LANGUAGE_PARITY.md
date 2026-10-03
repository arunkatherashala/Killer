# Killer language parity (generated)

Every probe below was executed against the real `killer_super` binary. A probe passes only if the
output matches what the equivalent Python/JS/Java program prints. Regenerate with
`python parity/run_parity.py --md`. Source of truth: `parity/probes.py`.

**Overall: 166 of 166 probes pass (100%).**

| Area | Pass | Total |
|------|-----:|------:|
| Basics | 21 | 21 |
| Control flow | 14 | 14 |
| Functions | 22 | 22 |
| OOP | 15 | 15 |
| Collections | 24 | 24 |
| Strings | 19 | 19 |
| Math | 10 | 10 |
| Errors | 9 | 9 |
| Iterators | 5 | 5 |
| Modules & I/O | 9 | 9 |
| Concurrency | 5 | 5 |
| Types | 6 | 6 |
| Killer-specific | 7 | 7 |

Status legend: PASS = identical output; WRONG = ran but printed something different; ERROR = the program failed to parse or run.

## Basics

| Probe | Status | Detail |
|-------|--------|--------|
| arithmetic precedence | PASS |  |
| power operator ** | PASS |  |
| modulo | PASS |  |
| integer division via floor | PASS |  |
| float sum prints like IEEE | PASS |  |
| string repetition | PASS |  |
| boolean && || ! | PASS |  |
| boolean keywords and/or/not | PASS |  |
| chained comparison | PASS |  |
| ternary expression | PASS |  |
| python-style conditional expr | PASS |  |
| multiple assignment | PASS |  |
| swap via tuple | PASS |  |
| augmented assignment | PASS |  |
| increment ++ | PASS |  |
| null handling | PASS |  |
| array equality | PASS |  |
| string equality and ordering | PASS |  |
| bitwise operators | PASS |  |
| big integers | PASS |  |
| hex literal | PASS |  |

## Control flow

| Probe | Status | Detail |
|-------|--------|--------|
| if / else if / else | PASS |  |
| elif keyword | PASS |  |
| while loop | PASS |  |
| for over range | PASS |  |
| range with step | PASS |  |
| break and continue | PASS |  |
| nested loops | PASS |  |
| do-while | PASS |  |
| C-style for | PASS |  |
| for over dict keys | PASS |  |
| enumerate | PASS |  |
| match statement | PASS |  |
| switch statement | PASS |  |
| while true + break | PASS |  |

## Functions

| Probe | Status | Detail |
|-------|--------|--------|
| define and call | PASS |  |
| recursion | PASS |  |
| deep recursion 5000 | PASS |  |
| default argument | PASS |  |
| variadic arguments | PASS |  |
| return multiple values | PASS |  |
| no explicit return | PASS |  |
| function stored in variable | PASS |  |
| lambda expression | PASS |  |
| arrow lambda | PASS |  |
| closure keeps state | PASS |  |
| function passed as argument | PASS |  |
| map with lambda | PASS |  |
| map with named function | PASS |  |
| filter and reduce | PASS |  |
| decorator | PASS |  |
| function in dict | PASS |  |
| read global from function | PASS |  |
| assignment without global stays local | PASS |  |
| global keyword | PASS |  |
| keyword arguments | PASS |  |
| mutual recursion | PASS |  |

## OOP

| Probe | Status | Detail |
|-------|--------|--------|
| class, init and method | PASS |  |
| state mutation through methods | PASS |  |
| inheritance and override | PASS |  |
| inherited method + init | PASS |  |
| super method call | PASS |  |
| super in init | PASS |  |
| objects are references | PASS |  |
| list of objects | PASS |  |
| polymorphism | PASS |  |
| operator overloading __add__ | PASS |  |
| custom toString when printed | PASS |  |
| instanceof / type check | PASS |  |
| static / class method | PASS |  |
| method chaining | PASS |  |
| field default and missing field | PASS |  |

## Collections

| Probe | Status | Detail |
|-------|--------|--------|
| list basics | PASS |  |
| negative index | PASS |  |
| slicing | PASS |  |
| slice function | PASS |  |
| pop and insert | PASS |  |
| sort and reverse | PASS |  |
| membership with in | PASS |  |
| contains function | PASS |  |
| list comprehension | PASS |  |
| list comprehension with filter | PASS |  |
| nested lists | PASS |  |
| list concatenation | PASS |  |
| sum min max | PASS |  |
| dict basics | PASS |  |
| dict missing key is null | PASS |  |
| dict membership | PASS |  |
| dict delete | PASS |  |
| dict values and items | PASS |  |
| dict comprehension | PASS |  |
| nested dict | PASS |  |
| set operations | PASS |  |
| tuple literal | PASS |  |
| sorted with key function | PASS |  |
| zip | PASS |  |

## Strings

| Probe | Status | Detail |
|-------|--------|--------|
| length upper lower | PASS |  |
| string indexing | PASS |  |
| string slicing | PASS |  |
| split and join | PASS |  |
| replace and strip | PASS |  |
| find / index_of | PASS |  |
| startswith endswith | PASS |  |
| string contains | PASS |  |
| str() and number parse | PASS |  |
| f-string interpolation | PASS |  |
| k-string interpolation | PASS |  |
| escape sequences | PASS |  |
| single-quoted strings | PASS |  |
| multiline string | PASS |  |
| reverse a string | PASS |  |
| character codes | PASS |  |
| unicode | PASS |  |
| format with padding | PASS |  |
| iterate characters | PASS |  |

## Math

| Probe | Status | Detail |
|-------|--------|--------|
| sqrt pow abs | PASS |  |
| floor ceil round | PASS |  |
| int and float conversion | PASS |  |
| trig | PASS |  |
| constants pi and e | PASS |  |
| random in range | PASS |  |
| log and exp | PASS |  |
| gcd | PASS |  |
| division by zero is an error | PASS |  |
| integer vs float printing | PASS |  |

## Errors

| Probe | Status | Detail |
|-------|--------|--------|
| try / catch / throw | PASS |  |
| catch a runtime error | PASS |  |
| finally always runs | PASS |  |
| catch then finally | PASS |  |
| custom error value | PASS |  |
| error propagates through calls | PASS |  |
| assert passes | PASS |  |
| undefined variable is an error | PASS |  |
| script exits non-zero on error | PASS |  |

## Iterators

| Probe | Status | Detail |
|-------|--------|--------|
| generator with yield | PASS |  |
| range materialises | PASS |  |
| zip in for | PASS |  |
| iterate dict items | PASS |  |
| reversed iteration | PASS |  |

## Modules & I/O

| Probe | Status | Detail |
|-------|--------|--------|
| import another file | PASS |  |
| write and read a file | PASS |  |
| file exists | PASS |  |
| json round trip | PASS |  |
| read environment variable | PASS |  |
| current time is a number | PASS |  |
| regex match | PASS |  |
| command line args available | PASS |  |
| sleep | PASS |  |

## Concurrency

| Probe | Status | Detail |
|-------|--------|--------|
| async function and await | PASS |  |
| spawn and await | PASS |  |
| async_spawn / async_await builtins | PASS |  |
| channels | PASS |  |
| mutex | PASS |  |

## Types

| Probe | Status | Detail |
|-------|--------|--------|
| type annotations run | PASS |  |
| type error is caught early | PASS |  |
| typeof / type() | PASS |  |
| conversion between types | PASS |  |
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
| statistics builtins | PASS |  |
