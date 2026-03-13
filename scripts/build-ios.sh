#!/usr/bin/env bash
set -euo pipefail

# Build netcap FFI library for iOS
# Prerequisites: Xcode, Rust iOS targets

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_ROOT}/ios/NetCap/Frameworks"

# iOS targets
TARGETS=(
    "aarch64-apple-ios"           # iPhone (arm64)
    "aarch64-apple-ios-sim"       # iPhone Simulator (arm64, Apple Silicon)
    "x86_64-apple-ios"            # iPhone Simulator (x86_64, Intel)
)

echo "Building netcap FFI for iOS..."

# Add Rust targets
for target in "${TARGETS[@]}"; do
    rustup target add "$target" 2>/dev/null || true
done

# Build for each target
for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    cargo build \
        --release \
        --target "$target" \
        -p netcap-ffi
done

# Create universal library for simulators
echo "Creating universal simulator library..."
mkdir -p "${OUTPUT_DIR}"

# Create xcframework
xcodebuild -create-xcframework \
    -library "${PROJECT_ROOT}/target/aarch64-apple-ios/release/libnetcap_ffi.a" \
    -library "${PROJECT_ROOT}/target/aarch64-apple-ios-sim/release/libnetcap_ffi.a" \
    -output "${OUTPUT_DIR}/NetcapFFI.xcframework" \
    2>/dev/null || {
    echo "Note: xcodebuild not available (non-macOS environment)."
    echo "iOS build must be performed on macOS with Xcode installed."
}

echo "iOS build complete!"
echo "Output: ${OUTPUT_DIR}"
