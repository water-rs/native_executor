#!/usr/bin/env bash
set -euo pipefail

# Run wasm tests in headless browser via wasm-pack.
cd "$(dirname "$0")/.."

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack not found; install with 'cargo install wasm-pack'" >&2
  exit 1
fi

BROWSER="${WASM_BROWSER:-chrome}"
case "$BROWSER" in
  chrome|firefox) ;;
  *)
    echo "Unsupported WASM_BROWSER value: $BROWSER (use chrome or firefox)" >&2
    exit 1
    ;;
esac

echo "==> wasm-pack test --headless --$BROWSER $*"
wasm-pack test --headless "--$BROWSER" "$@"
