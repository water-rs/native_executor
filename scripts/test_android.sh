#!/usr/bin/env bash
set -euo pipefail

# Cross-compile the test binaries for a connected Android device or emulator and
# run them there. The ABI, the Rust target, the NDK toolchain and the location
# of the test binaries are all derived from the device, the SDK and cargo, so
# the same script works on a CI emulator and on a physical phone.
cd "$(dirname "$0")/.."

# The oldest Android API level this crate supports. The NDK ships one clang
# wrapper per level, so the toolchain binaries are named after it.
API_LEVEL="${ANDROID_API_LEVEL:-21}"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq not found; it is needed to read cargo's build output" >&2
  exit 1
fi

SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -z "$SDK_ROOT" ]; then
  # Android Studio's default install locations, used only as a last resort.
  case "$(uname -s)" in
    Darwin) SDK_ROOT="$HOME/Library/Android/sdk" ;;
    *) SDK_ROOT="$HOME/Android/Sdk" ;;
  esac
fi

if [ -n "${ADB:-}" ]; then
  ADB_BIN="$ADB"
elif command -v adb >/dev/null 2>&1; then
  ADB_BIN="$(command -v adb)"
else
  ADB_BIN="$SDK_ROOT/platform-tools/adb"
fi
if [ ! -x "$ADB_BIN" ]; then
  echo "adb not found at $ADB_BIN; set ADB or ANDROID_HOME" >&2
  exit 1
fi

NDK_ROOT="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
if [ -z "$NDK_ROOT" ]; then
  # Newest installed NDK wins; the version is not something to pin here.
  NDK_ROOT="$(find "$SDK_ROOT/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n1)"
fi
if [ -z "$NDK_ROOT" ] || [ ! -d "$NDK_ROOT" ]; then
  echo "no Android NDK found; set ANDROID_NDK_HOME or install one under $SDK_ROOT/ndk" >&2
  exit 1
fi

# An NDK ships exactly one prebuilt host toolchain; ask the filesystem which.
TOOLCHAIN="$(find "$NDK_ROOT/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n1)/bin"
if [ ! -d "$TOOLCHAIN" ]; then
  echo "no prebuilt host toolchain under $NDK_ROOT/toolchains/llvm/prebuilt" >&2
  exit 1
fi

DEVICE="${ANDROID_DEVICE:-}"
if [ -z "$DEVICE" ]; then
  DEVICES="$("$ADB_BIN" devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
  DEVICE_COUNT="$(printf '%s' "$DEVICES" | grep -c . || true)"
  if [ "$DEVICE_COUNT" != "1" ]; then
    echo "expected exactly one connected device, found $DEVICE_COUNT; set ANDROID_DEVICE" >&2
    "$ADB_BIN" devices >&2
    exit 1
  fi
  DEVICE="$DEVICES"
fi

ABI="$("$ADB_BIN" -s "$DEVICE" shell getprop ro.product.cpu.abi | tr -d '\r\n')"
case "$ABI" in
  arm64-v8a)
    TARGET=aarch64-linux-android
    CLANG_TRIPLE=aarch64-linux-android
    ;;
  armeabi-v7a)
    TARGET=armv7-linux-androideabi
    CLANG_TRIPLE=armv7a-linux-androideabi
    ;;
  x86_64)
    TARGET=x86_64-linux-android
    CLANG_TRIPLE=x86_64-linux-android
    ;;
  x86)
    TARGET=i686-linux-android
    CLANG_TRIPLE=i686-linux-android
    ;;
  *)
    echo "unsupported device ABI: $ABI" >&2
    exit 1
    ;;
esac

CLANG="$TOOLCHAIN/${CLANG_TRIPLE}${API_LEVEL}-clang"
if [ ! -x "$CLANG" ]; then
  echo "no clang wrapper for API $API_LEVEL at $CLANG; set ANDROID_API_LEVEL" >&2
  exit 1
fi

TARGET_ENV="$(printf '%s' "$TARGET" | tr 'a-z-' 'A-Z_')"
TARGET_VAR="$(printf '%s' "$TARGET" | tr '-' '_')"
export "CC_${TARGET_VAR}=$CLANG"
export "AR_${TARGET_VAR}=$TOOLCHAIN/llvm-ar"
export "CARGO_TARGET_${TARGET_ENV}_LINKER=$CLANG"

echo "==> Building Android tests for $DEVICE ($ABI -> $TARGET)"
EXECUTABLES="$(mktemp)"
trap 'rm -f "$EXECUTABLES"' EXIT
# Ask cargo where it put the test binaries instead of guessing a target layout.
cargo test --target "$TARGET" --no-run --message-format=json-render-diagnostics "$@" \
  | jq -r 'select(.reason == "compiler-artifact" and .profile.test == true and .executable != null) | .executable' \
  > "$EXECUTABLES"

if [ ! -s "$EXECUTABLES" ]; then
  echo "cargo reported no test executables for $TARGET" >&2
  exit 1
fi

# `adb shell` reads stdin, so the executable list is read on its own descriptor
# instead of the loop's stdin, which adb would otherwise drain.
while IFS= read -r BIN <&3; do
  DEST="/data/local/tmp/$(basename "$BIN")"
  echo "==> Running $(basename "$BIN") on $DEVICE"
  "$ADB_BIN" -s "$DEVICE" push "$BIN" "$DEST" >/dev/null
  "$ADB_BIN" -s "$DEVICE" shell chmod 755 "$DEST"
  "$ADB_BIN" -s "$DEVICE" shell "$DEST" --nocapture
  "$ADB_BIN" -s "$DEVICE" shell rm -f "$DEST"
done 3< "$EXECUTABLES"
