# Flash Functions

Flash Functions are Killer's built-in functions available instantly without writing `fn` definitions.

## Why Flash Functions

- Fast usage for common tasks
- Consistent runtime behavior
- No boilerplate for one-line operations

## Current Flash Functions

## Core Utility

- `print(...)`
- `parseInt(value, radix=10)`
- `parseFloat(value)`
- `String(value)`
- `Number(value)`
- `Boolean(value)`
- `isNaN(value)`
- `isFinite(value)`

## New Convenience

- `sum(n)`
  - Returns arithmetic series sum from `1..n` using `n * (n + 1) / 2`
  - Example: `sum(10)` -> `55`
- `sum(array)`
  - Returns numeric sum of array elements
  - Example: `sum([1, 2, 3, 4])` -> `10`
- `factorial(n)`
  - Global alias of `Math.factorial(n)`
- `gcd(a, b)`
  - Global alias of `Math.gcd(a, b)`
- `lcm(a, b)`
  - Global alias of `Math.lcm(a, b)`
- `force(mass, acceleration)`
  - Global alias of `Physics.force(mass, acceleration)`
- `acceleration(velocityFinal, velocityInitial, timeSeconds)`
  - Global alias of `Physics.acceleration(...)`
- `velocity(velocityInitial, acceleration, timeSeconds)`
  - Global alias of `Physics.velocity(...)`
- `kineticEnergy(mass, velocity)`
  - Global alias of `Physics.kineticEnergy(...)`
- `potentialEnergy(mass, height)`
  - Global alias of `Physics.potentialEnergy(...)` (uses internal `g=9.81`)
- `ohmsLawCurrent(voltage, resistance)`
  - Global alias of `Physics.ohmsLawCurrent(...)`
- `ohmsLawVoltage(current, resistance)`
  - Global alias of `Physics.ohmsLawVoltage(...)`
- `ohmsLawResistance(voltage, current)`
  - Global alias of `Physics.ohmsLawResistance(...)`
- `gravity`
  - Global alias of `Physics.g`

## File Flash Functions

- `readFile(path)`
  - Reads UTF-8 text file content
- `writeFile(path, content)`
  - Writes UTF-8 text content (overwrite)
- `appendFile(path, content)`
  - Appends UTF-8 text content
- `exists(path)`
  - Returns `true` if path exists, else `false`

## Iterator Flash Functions

- `iter(value)`
  - Creates iterator from list/tuple/set/string/dictionary (dict iterates keys)
- `next(iterator)`
  - Returns next value, errors when exhausted
- `next(iterator, default)`
  - Returns `default` instead of error when exhausted

## Generator Flash Functions

- `rangeGenerator(start, end, step=1)`
  - Creates lazy iterator values from start to end (end exclusive)
  - Use with `next(...)` to consume values

## Logging Flash Functions

- `logInfo(...values)`
  - Prints message with `[INFO]` prefix
- `logWarn(...values)`
  - Prints message with `[WARN]` prefix
- `logError(...values)`
  - Prints message with `[ERROR]` prefix

## Context + Debug Flash Functions

- `withFile(path, mode, callback?)`
  - Context-style helper for file workflows
  - `mode`: `read`, `write`, `append`
  - `read` returns file content or callback result
  - `write/append` call callback with existing content and persist returned text
- `debug(value)`
  - Prints `[DEBUG]` with type and value, returns original value
- `trace(label, value)`
  - Prints `[TRACE] label: value`, returns original value
- `debugOn()` / `debugOff()`
  - Enables/disables `debug(...)` and `trace(...)` output

## Decorator Ergonomics

- `@logCalls`
  - Prints call and return traces for decorated function
- `@time`
  - Prints execution time in milliseconds for decorated function
- Existing decorators continue to work (`@deprecated`, `@memoized`, `@readonly`, `@override`)

## Math / Array Static Objects

- `Math.sqrt(x)`, `Math.abs(x)`, `Math.pow(x, y)`, `Math.floor(x)`, `Math.ceil(x)`, `Math.round(x)`, `Math.min(...)`, `Math.max(...)`, `Math.random()`, `Math.factorial(n)`, `Math.gcd(a, b)`, `Math.lcm(a, b)`, `Math.PI`, `Math.E`
- `Array.isArray(value)`

## Physics Static Object

- `Physics.force(mass, acceleration)`
- `Physics.acceleration(velocityFinal, velocityInitial, timeSeconds)`
- `Physics.velocity(velocityInitial, acceleration, timeSeconds)`
- `Physics.kineticEnergy(mass, velocity)`
- `Physics.potentialEnergy(mass, height, gravity=9.81)`
- `Physics.ohmsLawCurrent(voltage, resistance)`
- `Physics.ohmsLawVoltage(current, resistance)`
- `Physics.ohmsLawResistance(voltage, current)`
- `Physics.g`

## Explain Keyword (AI-style learning mode)

`explain` is a language keyword that prints backend reasoning only when explicitly used.

Example:

```killer
explain fibonacci(10)
```

This prints:

- expression/call breakdown
- evaluation steps
- Fibonacci-specific step table for `fibonacci(n)` / `fib(n)`
- final result

For Flash Functions and Physics calls, `explain` now includes formula-level details.

Examples:

```killer
explain sum(10)
explain factorial(6)
explain force(10, 2)
explain potentialEnergy(2, 10)
```

Typical detailed output includes:

- detected mode (for example `sum(n)`)
- formula used (for example `F = m * a`, `PE = m * g * h`)
- substituted values
- step-by-step arithmetic where relevant
- final computed result

Normal execution remains unchanged when `explain` is not used.
