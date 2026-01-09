#!/bin/bash

# Exit immediately if any command fails
set -e

# Default values
OUTPUT_DIR="./wasm-demo/src/assets/wasm"
OUTPUT_NAME="data-viewer-3d"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --output-name)
            OUTPUT_NAME="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--output-dir <dir>] [--output-name <name>] [--no-embed]"
            exit 1
            ;;
    esac
done

echo "🦀 Building Rust project for WebAssembly..."
echo "📁 Output directory: $OUTPUT_DIR"
echo "📝 Output name: $OUTPUT_NAME"

# Build the Rust project for WebAssembly
if ! cargo build --target wasm32-unknown-unknown --release; then
    echo "❌ Rust compilation failed!"
    exit 1
fi

echo "✅ Rust compilation successful!"

echo "🔗 Generating TypeScript bindings with web target..."

# Generate TypeScript bindings with web target
if ! wasm-bindgen --out-dir "$OUTPUT_DIR" --out-name "$OUTPUT_NAME" --target web --typescript target/wasm32-unknown-unknown/release/data-viewer-3d.wasm; then
    echo "❌ wasm-bindgen failed!"
    exit 1
fi

echo "✅ WebAssembly build complete! Files generated in $OUTPUT_DIR"

