#!/usr/bin/env bash
# Revalidate killer-native — same intent as revalidate_killer.ps1 (Linux/macOS/Git Bash).
# Usage:
#   cd SOURCE/src/v2-rust/killer && ./scripts/revalidate_killer.sh
#   ./scripts/revalidate_killer.sh --full
#   ./scripts/revalidate_killer.sh --release

set -euo pipefail

CRATE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$CRATE_ROOT"

RELEASE=""
FULL=""
for arg in "$@"; do
  case "$arg" in
    --release|-Release) RELEASE=1 ;;
    --full|-Full) FULL=1 ;;
  esac
done

if [[ -n "$RELEASE" ]]; then
  cargo build --release
else
  cargo build
fi

if [[ -n "$FULL" ]]; then
  echo "=== FULL: cargo test ==="
  cargo test
else
  echo "=== CI parity + language smoke ==="
  cargo test --lib
  cargo test --test pipeline_conformance
  cargo test --test trit_three_valued
  cargo test --test ai_integration_tests --test ai_annotations_tests
  cargo test --test builtin_pythonic
  cargo test --test parser_tests
fi

echo ""
echo "Revalidate OK."
