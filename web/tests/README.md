# Gnomics Live Visualizer - E2E Tests

End-to-end tests for the Gnomics Live Visualizer using Playwright.

## Setup

### Prerequisites
- Node.js 18+ and npm
- WASM package built and available in `pkg/` (sibling to this directory)

### Installation

```bash
# From the web/ directory
cd web

# Install dependencies
npm install

# Install Playwright browsers
npx playwright install
```

## Running Tests

### Run all tests (headless)
```bash
npm test
```

### Run tests with browser UI visible
```bash
npm run test:headed
```

### Run tests in interactive UI mode
```bash
npm run test:ui
```

### Run tests in specific browser
```bash
npm run test:chromium
npm run test:firefox
npm run test:webkit
```

### Debug mode (step through tests)
```bash
npm run test:debug
```

### View test report
```bash
npm run show-report
```

## Test Structure

### Test Suites

1. **Basic Functionality** (`viewer.spec.ts`)
   - WASM loading and initialization
   - Block palette display
   - Editor toolbar display

2. **Demo Network Loading**
   - Sequence Learning demo
   - Classification demo
   - Context Learning demo
   - Feature Pooling demo

3. **Editor Toolbar Interactions**
   - Mode switching (Select, Connect, Delete)
   - Keyboard shortcuts (V, C, D)

4. **Parameter Editor Modal**
   - Opening modal from palette
   - Form field validation
   - Default values
   - Cancel/Apply actions

5. **Block Creation**
   - Creating new blocks via UI
   - Block count verification
   - Network graph updates

6. **Context Menu**
   - Right-click interactions
   - Menu item actions
   - Parameter editing

7. **Network Simulation**
   - Start/Stop simulation
   - Reset simulation
   - Learning mode toggle
   - Speed adjustment

8. **Console Error Detection**
   - No critical errors on load
   - No critical errors during operations

9. **Responsive Design**
   - Desktop viewports (1920x1080, 1366x768)

10. **Performance**
    - Page load time (<5s)
    - Network initialization time (<2s)

## Test Coverage

### UI Components Tested
- ✅ Block Palette (7 block types)
- ✅ Editor Toolbar (7 tools)
- ✅ Parameter Editor Modal
- ✅ Network Visualization (SVG)
- ✅ Context Menu
- ✅ Plots (Input/Output, BitField)
- ✅ Status Indicators
- ✅ Controls (Start/Stop/Reset)

### User Interactions Tested
- ✅ Click events
- ✅ Right-click (context menu)
- ✅ Keyboard shortcuts
- ✅ Form input
- ✅ Dropdown selection
- ✅ Checkbox toggle
- ✅ Slider adjustment

### WASM Integration Tested
- ✅ Module loading
- ✅ Initialization
- ✅ Demo network creation
- ✅ Block addition (via UI)
- ✅ Network execution

## CI/CD Integration

The tests are configured to run in CI environments with:
- Automatic retry on failure (2 retries)
- Sequential test execution (no parallel)
- HTML and list reporters
- Trace collection on failure
- Screenshots and videos on failure

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: npm ci

      - name: Install Playwright browsers
        run: npx playwright install --with-deps

      - name: Build WASM
        run: |
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
          source $HOME/.cargo/env
          cargo install wasm-pack
          wasm-pack build --target web --dev -- --features wasm
          cp -r pkg web/

      - name: Run Playwright tests
        run: npm test

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

## Debugging Tests

### Visual Debugging
```bash
# Run with headed browsers
npm run test:headed

# Run in UI mode (interactive)
npm run test:ui

# Run in debug mode with inspector
npm run test:debug
```

### Inspecting Test Results
```bash
# View HTML report
npm run show-report
```

The report includes:
- Test results summary
- Screenshots on failure
- Videos of failed tests
- Trace viewer for step-by-step debugging

### Common Issues

**Issue**: "Target page, context or browser has been closed"
- **Solution**: Increase timeout in `playwright.config.ts`

**Issue**: "Timeout 5000ms exceeded"
- **Solution**: Check if HTTP server is running and WASM is built

**Issue**: "Cannot find element"
- **Solution**: Update selectors in test or check if UI changed

**Issue**: WASM not loading
- **Solution**: Rebuild WASM package and ensure it's in `pkg/` (sibling to web/ directory)

## Test Development

### Adding New Tests

1. Create or update test file in `tests/`
2. Follow naming convention: `*.spec.ts`
3. Use descriptive test names
4. Group related tests in `test.describe()` blocks
5. Use helper functions for common operations

### Best Practices

- ✅ Use meaningful selector strategies (data attributes, text content)
- ✅ Wait for elements explicitly with `expect().toBeVisible()`
- ✅ Use `beforeEach()` for test setup
- ✅ Clean up state between tests
- ✅ Keep tests independent and isolated
- ✅ Use page object model for complex flows
- ✅ Document test intent with clear descriptions

### Selectors Priority

1. Data attributes (`data-testid`, `data-block-type`)
2. Text content (`text=Submit`, `has-text("Save")`)
3. Unique IDs (`#param-modal`)
4. CSS classes (`.palette-item`)
5. XPath (last resort)

## Maintenance

### Updating Tests After UI Changes

1. Run tests to identify failures
2. Update selectors to match new UI
3. Verify tests pass with new changes
4. Update test documentation

### Performance Benchmarks

Current benchmarks (as of 2025-10-24):
- Page load + WASM init: < 5 seconds
- Demo network initialization: < 2 seconds
- Block creation: < 1 second
- Modal open/close: < 100ms

If tests fail these benchmarks, investigate:
- WASM build optimization
- Network request optimization
- JavaScript bundle size
- DOM rendering performance

## Resources

- [Playwright Documentation](https://playwright.dev/)
- [Best Practices](https://playwright.dev/docs/best-practices)
- [Selectors](https://playwright.dev/docs/selectors)
- [Test Fixtures](https://playwright.dev/docs/test-fixtures)
- [Assertions](https://playwright.dev/docs/test-assertions)

## License

MIT License - See [LICENSE](../../LICENSE) file
