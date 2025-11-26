#!/usr/bin/env bash
set -euo pipefail

# Cross-compile and run Android tests on a connected emulator/device.
cd "$(dirname "$0")/.."

ADB_BIN="${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}"
DEVICE="${ANDROID_DEVICE:-emulator-5554}"

if ! command -v "$ADB_BIN" >/dev/null 2>&1; then
  echo "adb not found at $ADB_BIN; set ADB env var to your adb path" >&2
  exit 1
fi

NDK_ROOT="${ANDROID_NDK_HOME:-$HOME/Library/Android/sdk/ndk/26.1.10909125}"
if [ ! -d "$NDK_ROOT" ]; then
  # pick the newest NDK under the default location
  if [ -d "$HOME/Library/Android/sdk/ndk" ]; then
    NDK_ROOT="$(ls -1d "$HOME/Library/Android/sdk/ndk"/* 2>/dev/null | sort | tail -n1)"
  fi
fi

if [ ! -d "$NDK_ROOT" ]; then
  echo "ANDROID_NDK_HOME not set and no NDK found under ~/Library/Android/sdk/ndk" >&2
  exit 1
fi

TOOLCHAIN="$NDK_ROOT/toolchains/llvm/prebuilt/darwin-x86_64/bin"
export CC_aarch64_linux_android="$TOOLCHAIN/aarch64-linux-android21-clang"
export AR_aarch64_linux_android="$TOOLCHAIN/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"

echo "==> Building Android tests (aarch64-linux-android)"
cargo test --target aarch64-linux-android --no-run "$@"

BIN=$(ls -t target/aarch64-linux-android/debug/deps/android-* 2>/dev/null | head -n1 || true)
if [ -z "$BIN" ]; then
  echo "Android test binary not found under target/aarch64-linux-android/debug/deps/" >&2
  exit 1
fi

DEST=/data/local/tmp/native_executor_android_tests
echo "==> Pushing test binary to $DEVICE"
"$ADB_BIN" -s "$DEVICE" push "$BIN" "$DEST" >/dev/null
"$ADB_BIN" -s "$DEVICE" shell chmod +x "$DEST"

echo "==> Running tests on $DEVICE"
"$ADB_BIN" -s "$DEVICE" shell "$DEST" --nocapture
