#!/bin/bash
# Build script for compiling Gnomics to WebAssembly

set -e

echo "=== Gnomics WASM Build Script ==="
echo

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found!"
    echo
    echo "Please install wasm-pack:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    echo
    echo "Or with cargo:"
    echo "  cargo install wasm-pack"
    exit 1
fi

# Check if wasm32 target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "📦 Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

echo "✅ Prerequisites met"
echo

# Build for web
echo "🔨 Building WASM module..."
wasm-pack build \
    --target web \
    --out-dir web/pkg \
    --features wasm

echo
echo "✅ Build complete!"
echo
echo "📦 Output directory: web/pkg/"
echo "   - gnomics_bg.wasm (compiled binary)"
echo "   - gnomics.js (JavaScript bindings)"
echo "   - gnomics.d.ts (TypeScript definitions)"
echo
echo "🌐 To test in browser:"
echo "   cd web"
echo "   python3 -m http.server 8000"
echo "   # Then open: http://localhost:8000/viewer_live.html"
echo
