#!/usr/bin/env bash
set -euo pipefail

# Cross-compile and run Android tests on a connected emulator/device.
cd "$(dirname "$0")/.."

ADB_BIN="${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}"
DEVICE="${ANDROID_DEVICE:-emulator-5554}"

# If ADB is not in standard location, try to find it in PATH or ANDROID_HOME
if ! command -v "$ADB_BIN" >/dev/null 2>&1; then
    if [ -n "${ANDROID_HOME:-}" ] && [ -x "$ANDROID_HOME/platform-tools/adb" ]; then
        ADB_BIN="$ANDROID_HOME/platform-tools/adb"
    elif command -v adb >/dev/null 2>&1; then
        ADB_BIN=$(command -v adb)
    else
        echo "adb not found at $ADB_BIN; set ADB env var to your adb path" >&2
        exit 1
    fi
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

# Detect Host OS
HOST_OS=$(uname -s | tr '[:upper:]' '[:lower:]')
if [[ "$HOST_OS" == "linux" ]]; then
    TOOLCHAIN_HOST="linux-x86_64"
elif [[ "$HOST_OS" == "darwin" ]]; then
    TOOLCHAIN_HOST="darwin-x86_64"
else
    echo "Unsupported host OS: $HOST_OS"
    exit 1
fi

TOOLCHAIN="$NDK_ROOT/toolchains/llvm/prebuilt/$TOOLCHAIN_HOST/bin"
TARGET_ARCH="${TARGET_ARCH:-aarch64-linux-android}"

# Set environment variables based on target architecture
if [[ "$TARGET_ARCH" == "aarch64-linux-android" ]]; then
    API_LEVEL=21
    export CC_aarch64_linux_android="$TOOLCHAIN/aarch64-linux-android${API_LEVEL}-clang"
    export AR_aarch64_linux_android="$TOOLCHAIN/llvm-ar"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
elif [[ "$TARGET_ARCH" == "x86_64-linux-android" ]]; then
    API_LEVEL=21
    export CC_x86_64_linux_android="$TOOLCHAIN/x86_64-linux-android${API_LEVEL}-clang"
    export AR_x86_64_linux_android="$TOOLCHAIN/llvm-ar"
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"
else
    echo "Unsupported target architecture: $TARGET_ARCH"
    exit 1
fi

echo "==> Building Android tests ($TARGET_ARCH)"
cargo test --target "$TARGET_ARCH" --no-run "$@"

BIN=$(ls -t "target/$TARGET_ARCH/debug/deps"/android-* 2>/dev/null | head -n1 || true)
# Fallback to unit tests if specific android integration test not found
if [ -z "$BIN" ]; then
     BIN=$(ls -t "target/$TARGET_ARCH/debug/deps"/native_executor-* 2>/dev/null | head -n1 || true)
fi

if [ -z "$BIN" ]; then
  echo "Android test binary not found under target/$TARGET_ARCH/debug/deps/" >&2
  exit 1
fi

DEST="/data/local/tmp/native_executor_android_tests"
echo "==> Pushing test binary to $DEVICE"
"$ADB_BIN" -s "$DEVICE" push "$BIN" "$DEST" >/dev/null
"$ADB_BIN" -s "$DEVICE" shell chmod +x "$DEST"

echo "==> Running tests on $DEVICE"
"$ADB_BIN" -s "$DEVICE" shell "$DEST" --nocapture