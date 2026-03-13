#!/usr/bin/env bash
set -euo pipefail

# Build netcap FFI library for Android using cargo-ndk
# Prerequisites: cargo-ndk, Android NDK

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_ROOT}/android/app/src/main/jniLibs"

# Supported Android targets
TARGETS=(
    "aarch64-linux-android"    # arm64-v8a
    "armv7-linux-androideabi"  # armeabi-v7a
    "x86_64-linux-android"    # x86_64
)

# Map Rust target to Android ABI directory
target_to_abi() {
    case "$1" in
        aarch64-linux-android)   echo "arm64-v8a" ;;
        armv7-linux-androideabi) echo "armeabi-v7a" ;;
        x86_64-linux-android)   echo "x86_64" ;;
        *)                       echo "unknown" ;;
    esac
}

echo "Building netcap FFI for Android..."

# Check for cargo-ndk
if ! command -v cargo-ndk &>/dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Add Rust targets
for target in "${TARGETS[@]}"; do
    rustup target add "$target" 2>/dev/null || true
done

# Build for each target
for target in "${TARGETS[@]}"; do
    abi=$(target_to_abi "$target")
    echo "Building for $target ($abi)..."

    cargo ndk \
        --target "$target" \
        --platform 26 \
        build \
        --release \
        -p netcap-ffi

    # Copy .so to jniLibs
    mkdir -p "${OUTPUT_DIR}/${abi}"
    cp "${PROJECT_ROOT}/target/${target}/release/libnetcap_ffi.so" \
       "${OUTPUT_DIR}/${abi}/libnetcap_ffi.so"

    echo "  -> ${OUTPUT_DIR}/${abi}/libnetcap_ffi.so"
done

echo "Android build complete!"
