import { test, expect, Page } from '@playwright/test';

test.describe('WASM Diagnostic', () => {
  test('should capture WASM loading errors', async ({ page }) => {
    const consoleLogs: string[] = [];
    const consoleErrors: string[] = [];

    // Capture all console messages
    page.on('console', msg => {
      const text = msg.text();
      consoleLogs.push(`[${msg.type()}] ${text}`);
      if (msg.type() === 'error') {
        consoleErrors.push(text);
      }
    });

    // Capture page errors
    page.on('pageerror', error => {
      consoleErrors.push(`PAGE ERROR: ${error.message}`);
    });

    // Navigate to the page
    await page.goto('/');

    // Wait a bit longer for WASM to load
    await page.waitForTimeout(15000);

    // Check what status we have
    const wasmStatus = await page.locator('#wasm-status-text').textContent();

    console.log('\n===== WASM STATUS =====');
    console.log('Status:', wasmStatus);

    console.log('\n===== CONSOLE LOGS =====');
    consoleLogs.forEach(log => console.log(log));

    console.log('\n===== CONSOLE ERRORS =====');
    consoleErrors.forEach(err => console.log(err));

    // Take a screenshot for debugging
    await page.screenshot({ path: 'test-results/wasm-diagnostic.png' });

    // Fail the test to see the output
    expect(wasmStatus).toBe('WASM: Ready');
  });
});
