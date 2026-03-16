#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CORE_PKG="$SCRIPT_DIR/.."
RUST_PKG="$CORE_PKG/../core-rust"
NATIVE_DIR="$CORE_PKG/native"

mkdir -p "$NATIVE_DIR"

cargo build --release --manifest-path "$RUST_PKG/Cargo.toml"

TARGET_DIR="$RUST_PKG/target/release"

if [[ "$(uname)" == "Darwin" ]]; then
  cp "$TARGET_DIR/libkittyui_core.dylib" "$NATIVE_DIR/"
elif [[ "$(uname)" == "Linux" ]]; then
  cp "$TARGET_DIR/libkittyui_core.so" "$NATIVE_DIR/"
else
  cp "$TARGET_DIR/kittyui_core.dll" "$NATIVE_DIR/"
fi

echo "Native library copied to $NATIVE_DIR"
