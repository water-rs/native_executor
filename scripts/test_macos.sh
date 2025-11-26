#!/usr/bin/env bash
set -euo pipefail

# Run host (macOS + polyfill) tests.
cd "$(dirname "$0")/.."
echo "==> cargo test $*"
cargo test "$@"
