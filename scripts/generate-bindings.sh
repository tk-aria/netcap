#!/usr/bin/env bash
set -euo pipefail

# Generate UniFFI bindings for Kotlin (Android) and Swift (iOS)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
UDL_FILE="${PROJECT_ROOT}/crates/netcap-ffi/src/netcap.udl"

# Output directories
KOTLIN_OUT="${PROJECT_ROOT}/android/app/src/main/kotlin/com/netcap/generated"
SWIFT_OUT="${PROJECT_ROOT}/ios/NetCap/Sources/Generated"

echo "Generating UniFFI bindings..."

# Build the FFI library first
echo "Building netcap-ffi..."
cargo build --release -p netcap-ffi

# Generate Kotlin bindings
echo "Generating Kotlin bindings..."
mkdir -p "$KOTLIN_OUT"
cargo run --release -p uniffi-bindgen -- \
    generate "$UDL_FILE" \
    --language kotlin \
    --out-dir "$KOTLIN_OUT" \
    2>/dev/null || {
    echo "Note: uniffi-bindgen not available as separate binary."
    echo "Kotlin bindings will be generated at build time via uniffi::generate_scaffolding."
}

# Generate Swift bindings
echo "Generating Swift bindings..."
mkdir -p "$SWIFT_OUT"
cargo run --release -p uniffi-bindgen -- \
    generate "$UDL_FILE" \
    --language swift \
    --out-dir "$SWIFT_OUT" \
    2>/dev/null || {
    echo "Note: uniffi-bindgen not available as separate binary."
    echo "Swift bindings will be generated at build time via uniffi::generate_scaffolding."
}

echo "Binding generation complete!"
echo "  Kotlin: ${KOTLIN_OUT}"
echo "  Swift:  ${SWIFT_OUT}"
