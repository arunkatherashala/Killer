# Killer Architecture (Lightweight Core)

This project follows a **modular runtime architecture** so the interpreter and compiler stay lightweight.

## Design Goal

- Keep core engine files small and focused
- Move feature-specific logic into dedicated submodules
- Let the main pipeline orchestrate, not contain everything

## Ownership by Module

- `src/lexer.py`
  - Tokenization only
- `src/parser.py`
  - AST construction only
- `src/interpreter.py`
  - Runtime orchestration and execution flow
  - Delegates feature-specific logic to submodules
- `src/explain_engine.py`
  - `explain` keyword rendering and detailed reasoning output
- `src/python_generator.py`
  - Python transpilation
- `src/javascript_generator.py`
  - JavaScript transpilation

## Why This Is Better

- Easier maintenance: each file has one responsibility
- Safer changes: fewer side effects across unrelated features
- Better performance hygiene: no giant monolithic core file
- Faster onboarding: contributors can find ownership quickly

## Rule of Thumb for New Features

When adding a major feature, prefer a **new submodule** plus a thin integration point in `interpreter.py`/compiler entrypoints.

Example:
- Add `src/<feature>_engine.py`
- Wire it once in core
- Keep heavy logic in the feature module
