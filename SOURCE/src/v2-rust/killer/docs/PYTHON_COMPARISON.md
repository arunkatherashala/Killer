# Killer compared with Python

Measured on 2026-10-25 by running small Python-style programs through `killer run` (debug build, Windows).
99 probes: **95 behave like Python, 4 differ.** Only what a probe ran is claimed here; anything not
probed is listed at the bottom as unknown. Re-run with `python parity/python_gap/probe.py` and
`probe2.py` (set `KILLER_BIN` to the `killer` binary). The first measurement on 2026-10-24 found 12
differences; eight were fixed the next day (see "Closed since the first measurement").

## Where Killer still differs from Python

| # | Python feature | What Killer does | Workaround |
|---|----------------|------------------|------------|
| 1 | reading an unassigned variable is an error | reads `null` (slots start as `null`; telling "never assigned" from "assigned null" needs a new value kind) | check for `null` |
| 2 | `open(path)` returns a file object | no file objects; `with` works on any class that defines `__enter__` / `__exit__` | `file_read`, `file_write`, `file_append` |
| 3 | `sqlite3` | no SQL database | none built in |
| 4 | assigning in a nested function creates a local | assigns the enclosing variable (JavaScript-like closure sharing) | by design; use a different name for a local |

Also: an integer literal above 2^53 is read as a float before any arithmetic (`9007199254740993` is
`9007199254740992`); write `9007199254740993n`. Python has no such limit. `__exit__` cannot swallow an
exception (Python's can by returning true).

## Closed since the first measurement

| Python feature | Now |
|----------------|-----|
| `2 ** 100`, `10 ** 20 + 1`, factorial loops | exact: whole-number `+ - * **` results that reach 2^53 become big integers automatically (`type` is `bigint`); native-compiled functions and hot loops fall back to exact arithmetic |
| `round(2.567, 2)` | `round(x, digits)`, negative digits too |
| `hex(255)`, `int("ff", 16)` | `hex`, `bin`, `oct` (text like `"0xff"`), `int(text, base)` for bases 2 to 36 |
| `uuid.uuid4()` | `uuid()` |
| `"%d items" % 3` | printf-style `%` with `d i s f e x X o c %%`, flags, width, precision |
| `format("{:x}")`, `{:5d}` | full Python-style specs: fill, align, sign, `#`, `0`, width, `,`, precision, types `d f e x X o b s %` |
| `@property` getters and setters | `get name() { }` and `set name(v) { }` in a class; inherited; a real field of the same name wins |
| `with ... as x:` | `with expr as x { }` calls `__enter__` / `__exit__` on every path (end, `return`, `break`, `continue`, error) |
| type hints `x: int` | `int`, `integer`, `float`, `double`, `str`, `boolean`, `list` are aliases of Killer's names and are still checked |

## Same name, different spelling (not gaps)

| Python | Killer |
|--------|--------|
| `type(1)` is `int`, `type([1])` is `list` | `number`, `array` (no int/float distinction in `type`) |
| `s.upper()` | works as a method and as `upper(s)` |
| `//` integer division | `//` starts a comment; use `floor(a / b)` or `idiv(a, b)` |
| `-7 % 3` is `2` | same (floor modulo); `rem(a, b)` is the C-style remainder |
| exceptions: `try/except` | `try/catch`, `throw` |
| `round(2.5)` is `2` (banker's rounding) | `3` (halves round away from zero) |

## What matched Python (95 probes)

Numbers: float arithmetic prints identically, floor modulo, exact big integers (automatic past 2^53, or
with `n` / `big()`), factorial, min/max/sum/abs, sqrt/pow/floor/ceil/log, bit operations, `round` with digits,
`hex` / `bin` / `oct`, `int` with a base.
Strings: slicing and negative indexes, f-strings, split/join/replace/trim, unicode length, `format` with
specs, `%` formatting, startswith/endswith/find, multi-line strings.
Collections: list, dict and set comprehensions, set operations, sorting with a key or reversed, tuple
unpacking and swap, enumerate/zip/range with step, dict iteration, any/all/map/filter/reduce, `xs.append`,
push/pop, dict get with default, keys/values, element assignment.
Functions: default and keyword arguments, `...rest`, closures, generators, decorators, recursion 5000
deep, `match`, ternary, chained comparison, lambdas, `global`, `??` default, type hints.
Classes: inheritance and `super`, operator overloading, static methods, properties, `toString`, records.
Errors: try/catch/finally, throw, catchable division by zero, custom error classes, `assert`, `with`.
Standard library: regex (text first, pattern second), JSON round trip, date and time, random, `uuid`,
SHA-256, base64, file read/write, command-line arguments, environment variables, CSV (returns a list of
dicts keyed by the header), compression, running an external command, `sleep`, `exit` codes.
Concurrency: `async` / `await`, channels. Network: `http_request`, `serve`, `route` exist.

## Not probed (unknown)

Python features I did not run: `dataclasses`, `typing` generics, `itertools` / `functools` equivalents,
`collections` (`Counter`, `deque`), `pathlib`, `subprocess` options, `argparse`, `logging` formats,
decimal / fraction types, `struct`, `socket`, `threading` locks beyond `mutex_*`, `multiprocessing`,
`asyncio` event-loop details, and packages (NumPy, pandas, ...), which Killer does not have.
