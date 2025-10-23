# WebAssembly Setup Instructions

This guide will help you compile Gnomics to WebAssembly and run it in your browser.

## Prerequisites

### 1. Install wasm-pack

**Option A: Using the installer (recommended)**
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

**Option B: Using cargo**
```bash
cargo install wasm-pack
```

### 2. Add WASM target
```bash
rustup target add wasm32-unknown-unknown
```

## Building

Use the provided build script:

```bash
./build_wasm.sh
```

Or build manually:

```bash
wasm-pack build \
    --target web \
    --out-dir visualization/pkg \
    --features wasm \
    --release
```

This will create:
- `visualization/pkg/gnomics_bg.wasm` - The compiled WebAssembly binary
- `visualization/pkg/gnomics.js` - JavaScript bindings
- `visualization/pkg/gnomics.d.ts` - TypeScript type definitions

## Running

### Start a local server

**Option A: Python**
```bash
cd visualization
python3 -m http.server 8000
```

**Option B: Node.js**
```bash
cd visualization
npx http-server -p 8000
```

### Open in browser

Navigate to: `http://localhost:8000/viewer_live.html`

## Testing

Once the page loads:

1. Click "Live Mode (WASM)" button
2. Click "Start Network" to begin execution
3. Watch the network execute in real-time
4. Use the speed slider to adjust execution rate
5. Click "Stop Network" to pause

## Troubleshooting

### Error: "wasm-pack not found"
- Install wasm-pack following the instructions above

### Error: "failed to download wasm-opt"
- On some systems, you may need to install binaryen:
  ```bash
  # macOS
  brew install binaryen

  # Ubuntu/Debian
  sudo apt-get install binaryen
  ```

### Error: Module not found
- Make sure you're serving from the `visualization/` directory
- Check that `pkg/gnomics.js` exists

### WASM module fails to load
- Ensure you're using a local server (not file://)
- Check browser console for specific errors
- Try a different browser (Chrome/Firefox/Safari)

## Browser Requirements

- Chrome 79+
- Firefox 79+
- Safari 14+
- Edge 79+

Mobile browsers are supported but may have reduced performance.

## Performance Tips

- Use Release mode (already default in build script)
- Start with small networks to test
- Adjust execution speed slider if visualization is laggy
- Close dev tools for better performance

## Next Steps

After successful setup:
- Modify `viewer_live.html` to create custom networks
- See `examples/` directory for network examples
- Read `.claude/WASM_VISUALIZATION_GUIDE.md` for detailed API documentation
