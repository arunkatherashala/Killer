# Killer compared with Python

Measured on 2026-10-24 by running small Python-style programs through `killer run` (debug build, Windows).
99 probes: **87 behave like Python, 12 differ.** Only what a probe ran is claimed here; anything not
probed is listed at the bottom as unknown. Re-run with `python parity/python_gap/probe.py` and
`probe2.py` (set `KILLER_BIN` to the `killer` binary).

## Where Killer differs from Python

| # | Python feature | What Killer does | Workaround |
|---|----------------|------------------|------------|
| 1 | `2 ** 100` is an exact integer | prints `1267650600228229400000000000000` (a float, precision lost) | `big(2) ** 100` or `2n ** 100` |
| 2 | `9223372036854775807 + 1` is exact | prints `9223372036854776000` (float) | use `n` literals / `big()` |
| 3 | `round(2.567, 2)` | `round()` takes one argument | `round(x * 100) / 100` |
| 4 | `hex(255)`, `int("ff", 16)` | `hex` is not a function; `int()` takes one argument | `parse_int` exists (base argument not probed), `hex_encode` is for bytes |
| 5 | `@property` getters | `get name()` inside a class is a parse error | write a method `fn name()` |
| 6 | reading an unassigned variable is an error | reads `null` | none; check for `null` |
| 7 | `with open(...) as f:` | unsupported statement | call `file_read` / `file_write` |
| 8 | `uuid.uuid4()` | no `uuid` function | build one from `random` / `secure_random_bytes` |
| 9 | `sqlite3` | no SQL database | none built in |
| 10 | type hints `x: int` | accepted only with Killer's type names (`x: number`); `int` is rejected | use Killer names or omit hints |
| 11 | `"%d items" % 3` | runtime error | `format("{} items", 3)` or `f"{n} items"` |
| 12 | assigning in a nested function creates a local | assigns the enclosing variable (JavaScript-like closure sharing) | by design; use a different name for a local |

Also found: `format("{:x}", 255)` ignores the `x` and prints `255`, and `format("{:5d}", 42)` ignores the
width. `format("{:.2f}", 3.14159)` works (`3.14`).

## Same name, different spelling (not gaps)

| Python | Killer |
|--------|--------|
| `type(1)` is `int`, `type([1])` is `list` | `number`, `array` (no int/float distinction in `type`) |
| `s.upper()` | works as a method and as `upper(s)` |
| `//` integer division | `//` starts a comment; use `floor(a / b)` or `idiv(a, b)` |
| `-7 % 3` is `2` | same (floor modulo); `rem(a, b)` is the C-style remainder |
| exceptions: `try/except` | `try/catch`, `throw` |

## What matched Python (87 probes)

Numbers: float arithmetic prints identically, floor modulo, big integers with `n` / `big()`, factorial,
min/max/sum/abs, sqrt/pow/floor/ceil/log, bit operations.
Strings: slicing and negative indexes, f-strings, split/join/replace/trim, unicode length, `format`,
startswith/endswith/find, multi-line strings.
Collections: list, dict and set comprehensions, set operations, sorting with a key, tuple unpacking and
swap, enumerate/zip/range with step, dict iteration, any/all/map/filter/reduce, `xs.append`, push/pop,
dict get with default, keys/values, element assignment.
Functions: default and keyword arguments, `...rest`, closures, generators, decorators, recursion 5000
deep, `match`, ternary, chained comparison, lambdas, `global`, `??` default.
Classes: inheritance and `super`, operator overloading, static methods, `toString`, records.
Errors: try/catch/finally, throw, catchable division by zero, custom error classes, `assert`.
Standard library: regex (text first, pattern second), JSON round trip, date and time, random, SHA-256,
base64, file read/write, command-line arguments, environment variables, CSV (returns a list of dicts
keyed by the header), compression, running an external command, `sleep`, `exit` codes.
Concurrency: `async` / `await`, channels. Network: `http_request`, `serve`, `route` exist.

## Not probed (unknown)

Python features I did not run: `dataclasses`, `typing` generics, `itertools` / `functools` equivalents,
`collections` (`Counter`, `deque`), `pathlib`, `subprocess` options, `argparse`, `logging` formats,
decimal / fraction types, `struct`, `socket`, `threading` locks beyond `mutex_*`, `multiprocessing`,
`asyncio` event-loop details, f-string format specs beyond `.2f`, and packages (NumPy, pandas, ...),
which Killer does not have.
