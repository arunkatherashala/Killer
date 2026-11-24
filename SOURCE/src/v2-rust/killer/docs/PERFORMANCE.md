# Performance summary

An honest index of what has been measured. Nothing here is a new measurement: every number comes from
[`bench/RESULTS.md`](../bench/RESULTS.md) (rounds 1 to 7, 2026-10-08 to 2026-11-19 by git date) and is only as good as
that file says. All runs were on one Windows 11 x86-64 machine, CPython 3.12 against a Killer release build (LTO), usually while other builds
were running. **No Linux, macOS or ARM numbers exist.** Do not extrapolate.

## Methodology

* Early rounds: median wall time of `python bench/run.py`, including process start-up; ratio = Killer / CPython.
* Rounds 2 to 7: **CPU seconds** (kernel+user via `GetProcessTimes`), best of 3, later median of 5 to 15 alternating runs
  of base and new binary, about 0.03 s of start-up included. CPU time is less sensitive to a loaded machine.
* **Noise floor about +-10%** (+-8% on a quiet box in round 7). A change under that is called "noise" and was not kept
  unless it simplified the code. Several optimizations were tried and reverted for that reason (listed in RESULTS.md).
* Both implementations must print identical output; the benchmark pairs are `bench/*.killer` and `bench/*.py`.
* The only wall-clock ratio table (geometric mean 2.14 -> 1.81 versus CPython) predates rounds 2 to 7. A later loaded-box run in
  round 3 gave a geometric mean of 1.52 over four closure/object/list benchmarks only. There is no full, current ratio table.

## What is fast

* **Numeric code on x86-64** (the JIT covers pure numeric functions and loops; everything else is interpreted):
  `fib(32)` 0.094 s CPU and a 5M-iteration loop 0.55 s CPU in round 2; later rounds report them unchanged within noise.
  There is no recorded comparison with CPython for these in RESULTS.md, so no "x times faster" claim is made. On
  non-x86-64 CPUs the JIT is off and these numbers do not apply.
* **String building by repeated concatenation** (`s = s + x`): ratio 0.07 of CPython (CPython's `t = t + a + b` pattern
  is quadratic). 100k `x = x + "ab"` took 0.13 s CPU.
* Large `sorted` / list operations (ratio about 1.0 to 1.2), word counting (1.4), JSON (1.6), split/join (1.4) are
  near parity.

## What is slower than Python

Ratio = Killer / CPython, from the last wall-clock table (so before rounds 2 to 7, which improved several CPU times):

| Workload | Ratio then | Later CPU change | Status |
|---|---:|---|---|
| list comprehensions | 6.6 | -19% (round 2) | slower; round 6/7: noise |
| objects and method calls | 4.0 | -18% (round 2), no further gain | slower |
| string indexing on long strings | 4.8 | see below | slower, structural |
| nested data (list of dicts) | 3.8 | small dicts -12 to -18% CPU (round 4) | slower |
| closures | 4.4 | about 2.1x in the round-7 loaded-box run | roughly 2x Python at best measurement |
| dict ops | 2.0 | noise | slower |
| try/catch | 1.4 | | near parity |

**Long-string indexing.** `s[i]` and `len(s)` are O(n) per call; a 200k-iteration loop over an 852k-character string takes
about 0.7 s (fused `s[i]`) to 15.8 s (`s[i*4]`, which clones the string); `len(s)` 200k times took 6.9 s CPU. Fixing it needs a
different string representation (a repo-wide change); it has not been done.

## Not measured

Memory use, start-up time as a metric (about 0.03 s CPU is included in the numbers above), server throughput
(`serve` handles one request at a time), LLM and inference speed, I/O, multi-threaded scaling, Linux/macOS performance.

## Reproduce

`python bench/run.py` with `KILLER_BIN` pointing at a release `killer` binary; for CPU time follow the per-round
notes in RESULTS.md. Compiler switches `KILLER_NO_FUSION`, `KILLER_NO_PARAM_SLOTS`, `KILLER_NO_RO_CAPTURES`,
`KILLER_NO_CAP_SLOTS` allow same-binary A/B runs. `tests/perf_regress.rs` guards against gross regressions; it is not a benchmark.
