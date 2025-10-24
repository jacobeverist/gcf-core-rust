/**
 * Playwright E2E Tests for Gnomics Live Visualizer
 *
 * Tests the interactive network editor UI and WASM integration.
 * Run with: npx playwright test
 */

import { test, expect, Page } from '@playwright/test';

const BASE_URL = 'http://localhost:8080/viewer_live.html';

// Helper to wait for WASM to be ready
async function waitForWasmReady(page: Page) {
  await expect(page.locator('text=WASM: Ready')).toBeVisible({ timeout: 10000 });
}

// Helper to initialize a demo network
async function initializeDemoNetwork(page: Page, demo: string) {
  await page.selectOption('select', demo);
  await page.click('button:has-text("Initialize Network")');
  await expect(page.locator('text=/Network: \\d+ blocks/')).toBeVisible({ timeout: 5000 });
}

test.describe('Gnomics Live Visualizer - Basic Functionality', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
  });

  test('should load page and initialize WASM', async ({ page }) => {
    // Check page title
    await expect(page).toHaveTitle(/Gnomics Live Visualizer/);

    // Wait for WASM to load
    await waitForWasmReady(page);

    // Verify initial status
    await expect(page.locator('text=Network: Not created')).toBeVisible();
  });

  test('should display all block palette items', async ({ page }) => {
    await waitForWasmReady(page);

    // Check block palette is visible
    await expect(page.locator('#block-palette')).toBeVisible();

    // Verify all 7 block types are present
    const blockTypes = [
      'ScalarTransformer',
      'DiscreteTransformer',
      'PersistenceTransformer',
      'PatternPooler',
      'PatternClassifier',
      'SequenceLearner',
      'ContextLearner'
    ];

    for (const blockType of blockTypes) {
      const item = page.locator(`.palette-item[data-block-type="${blockType}"]`);
      await expect(item).toBeVisible();
      await expect(item).toHaveAttribute('draggable', 'true');
    }
  });

  test('should display editor toolbar with all tools', async ({ page }) => {
    await waitForWasmReady(page);

    // Check all 7 tools are present
    const tools = await page.locator('.editor-tool').count();
    expect(tools).toBe(7);

    // Verify Select mode is active by default
    const selectTool = page.locator('.editor-tool').first();
    await expect(selectTool).toHaveClass(/active/);
  });
});

test.describe('Demo Network Loading', () => {

  test('should load Sequence Learning demo', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    await initializeDemoNetwork(page, 'sequence');

    // Verify network status
    await expect(page.locator('text=Network: 2 blocks')).toBeVisible();
    await expect(page.locator('text=Learns sequence [0→1→2→3]')).toBeVisible();

    // Verify blocks are visible in network graph
    await expect(page.locator('text=Discrete Encoder')).toBeVisible();
    await expect(page.locator('text=Sequence Learner')).toBeVisible();

    // Verify plots are displayed
    await expect(page.locator('text=Input/Output Values')).toBeVisible();
    await expect(page.locator('text=BitField States')).toBeVisible();
  });

  test('should load Classification demo', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    await initializeDemoNetwork(page, 'classification');

    await expect(page.locator('text=/Network: \\d+ blocks/')).toBeVisible();
    await expect(page.locator('text=3-class supervised learning')).toBeVisible();
  });

  test('should load Context Learning demo', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    await initializeDemoNetwork(page, 'context');

    await expect(page.locator('text=/Network: \\d+ blocks/')).toBeVisible();
    await expect(page.locator('text=Context-dependent pattern recognition')).toBeVisible();
  });

  test('should load Feature Pooling demo', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    await initializeDemoNetwork(page, 'pooling');

    await expect(page.locator('text=/Network: \\d+ blocks/')).toBeVisible();
    await expect(page.locator('text=Unsupervised feature extraction')).toBeVisible();
  });
});

test.describe('Editor Toolbar Interactions', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');
  });

  test('should switch to Connect mode', async ({ page }) => {
    const connectTool = page.locator('.editor-tool').nth(1); // Connect tool
    await connectTool.click();

    // Verify connect tool is now active
    await expect(connectTool).toHaveClass(/active/);

    // Verify other tools are inactive
    const selectTool = page.locator('.editor-tool').first();
    await expect(selectTool).not.toHaveClass(/active/);
  });

  test('should switch to Delete mode', async ({ page }) => {
    const deleteTool = page.locator('.editor-tool').nth(2); // Delete tool
    await deleteTool.click();

    await expect(deleteTool).toHaveClass(/active/);
  });

  test('should switch between modes using keyboard shortcuts', async ({ page }) => {
    // Press 'C' for connect mode
    await page.keyboard.press('c');
    const connectTool = page.locator('.editor-tool').nth(1);
    await expect(connectTool).toHaveClass(/active/);

    // Press 'V' for select mode
    await page.keyboard.press('v');
    const selectTool = page.locator('.editor-tool').first();
    await expect(selectTool).toHaveClass(/active/);

    // Press 'D' for delete mode
    await page.keyboard.press('d');
    const deleteTool = page.locator('.editor-tool').nth(2);
    await expect(deleteTool).toHaveClass(/active/);
  });
});

test.describe('Parameter Editor Modal', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');
  });

  test('should open parameter editor when clicking palette item', async ({ page }) => {
    // Click on ScalarTransformer palette item
    const scalarItem = page.locator('.palette-item[data-block-type="ScalarTransformer"]');
    await scalarItem.click();

    // Verify modal is visible
    const modal = page.locator('#param-modal');
    await expect(modal).toHaveClass(/visible/);

    // Verify modal title
    await expect(page.locator('.modal-header:has-text("Add ScalarTransformer")')).toBeVisible();
  });

  test('should display correct form fields for ScalarTransformer', async ({ page }) => {
    await page.locator('.palette-item[data-block-type="ScalarTransformer"]').click();

    // Check all form fields are present
    await expect(page.locator('#param-name')).toBeVisible();
    await expect(page.locator('#param-min')).toBeVisible();
    await expect(page.locator('#param-max')).toBeVisible();
    await expect(page.locator('#param-statelets')).toBeVisible();
    await expect(page.locator('#param-active')).toBeVisible();

    // Verify default values
    await expect(page.locator('#param-name')).toHaveValue('scalar_encoder');
    await expect(page.locator('#param-min')).toHaveValue('0');
    await expect(page.locator('#param-max')).toHaveValue('100');
    await expect(page.locator('#param-statelets')).toHaveValue('2048');
    await expect(page.locator('#param-active')).toHaveValue('256');
  });

  test('should display correct form fields for DiscreteTransformer', async ({ page }) => {
    await page.locator('.palette-item[data-block-type="DiscreteTransformer"]').click();

    await expect(page.locator('.modal-header:has-text("Add DiscreteTransformer")')).toBeVisible();
    await expect(page.locator('#param-name')).toBeVisible();
    await expect(page.locator('#param-categories')).toBeVisible();
    await expect(page.locator('#param-statelets')).toBeVisible();
  });

  test('should close modal when clicking Cancel', async ({ page }) => {
    await page.locator('.palette-item[data-block-type="ScalarTransformer"]').click();

    const modal = page.locator('#param-modal');
    await expect(modal).toHaveClass(/visible/);

    // Click Cancel
    await page.click('button:has-text("Cancel")');

    // Verify modal is hidden
    await expect(modal).not.toHaveClass(/visible/);
  });

  test('should close modal when clicking outside', async ({ page }) => {
    await page.locator('.palette-item[data-block-type="ScalarTransformer"]').click();

    const modal = page.locator('#param-modal');
    await expect(modal).toHaveClass(/visible/);

    // Click on the modal backdrop (outside the content)
    await modal.click({ position: { x: 10, y: 10 } });

    // Verify modal is hidden
    await expect(modal).not.toHaveClass(/visible/);
  });
});

test.describe('Block Creation', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');
  });

  test('should create a new ScalarTransformer block', async ({ page }) => {
    // Get initial block count
    const initialBlockCount = await page.locator('text=/Blocks: (\\d+)/')
      .textContent()
      .then(text => parseInt(text!.match(/\\d+/)![0]));

    // Open parameter editor
    await page.locator('.palette-item[data-block-type="ScalarTransformer"]').click();

    // Fill in parameters (use defaults)
    await page.fill('#param-name', 'test_scalar');

    // Click Apply
    await page.click('button:has-text("Apply")');

    // Wait for modal to close
    await expect(page.locator('#param-modal')).not.toHaveClass(/visible/, { timeout: 5000 });

    // Verify block count increased
    await expect(page.locator(`text=Blocks: ${initialBlockCount + 1}`))
      .toBeVisible({ timeout: 5000 });

    // Verify new block appears in network graph
    await expect(page.locator('text=test_scalar')).toBeVisible({ timeout: 5000 });
  });

  test('should create blocks of different types', async ({ page }) => {
    const blockTypes = [
      { type: 'DiscreteTransformer', name: 'test_discrete' },
      { type: 'PatternPooler', name: 'test_pooler' }
    ];

    for (const block of blockTypes) {
      await page.locator(`.palette-item[data-block-type="${block.type}"]`).click();
      await page.fill('#param-name', block.name);
      await page.click('button:has-text("Apply")');
      await page.waitForTimeout(1000); // Wait for network rebuild
    }

    // Verify both blocks were created
    await expect(page.locator('text=test_discrete')).toBeVisible();
    await expect(page.locator('text=test_pooler')).toBeVisible();
  });
});

test.describe('Context Menu', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');
  });

  test('should show context menu on right-click block', async ({ page }) => {
    // Find a block node in the network graph
    const blockNode = page.locator('text=Discrete Encoder').first();

    // Right-click to open context menu
    await blockNode.click({ button: 'right' });

    // Verify context menu appears
    const contextMenu = page.locator('#context-menu');
    await expect(contextMenu).toBeVisible();

    // Verify menu items
    await expect(page.locator('text=Add Input/Output Plot')).toBeVisible();
    await expect(page.locator('text=View BitField State')).toBeVisible();
    await expect(page.locator('text=Edit Parameters')).toBeVisible();
    await expect(page.locator('text=Delete Block')).toBeVisible();
  });

  test('should open parameter editor from context menu', async ({ page }) => {
    const blockNode = page.locator('text=Discrete Encoder').first();
    await blockNode.click({ button: 'right' });

    // Click Edit Parameters
    await page.click('text=Edit Parameters');

    // Verify parameter modal opens
    await expect(page.locator('#param-modal')).toHaveClass(/visible/);
    await expect(page.locator('.modal-header:has-text("Edit DiscreteTransformer")')).toBeVisible();
  });
});

test.describe('Network Simulation', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');
  });

  test('should start and stop simulation', async ({ page }) => {
    // Click Start button
    await page.click('button:has-text("Start")');

    // Wait for a few steps to execute
    await page.waitForTimeout(500);

    // Verify step counter increased
    const stepText = await page.locator('text=/Step: (\\d+)/').textContent();
    const stepCount = parseInt(stepText!.match(/\\d+/)![0]);
    expect(stepCount).toBeGreaterThan(0);

    // Click Stop button
    await page.click('button:has-text("Stop")');

    // Wait and verify step counter stopped increasing
    const stepBefore = parseInt((await page.locator('text=/Step: (\\d+)/').textContent())!.match(/\\d+/)![0]);
    await page.waitForTimeout(500);
    const stepAfter = parseInt((await page.locator('text=/Step: (\\d+)/').textContent())!.match(/\\d+/)![0]);

    expect(stepAfter).toBe(stepBefore);
  });

  test('should reset simulation', async ({ page }) => {
    // Run simulation for a few steps
    await page.click('button:has-text("Start")');
    await page.waitForTimeout(500);
    await page.click('button:has-text("Stop")');

    // Verify step counter is > 0
    const stepBefore = parseInt((await page.locator('text=/Step: (\\d+)/').textContent())!.match(/\\d+/)![0]);
    expect(stepBefore).toBeGreaterThan(0);

    // Click Reset
    await page.click('button:has-text("Reset")');

    // Verify step counter reset to 0
    await expect(page.locator('text=Step: 0')).toBeVisible();
  });

  test('should toggle learning mode', async ({ page }) => {
    const learningCheckbox = page.locator('input[type="checkbox"]').first();

    // Verify learning is enabled by default
    await expect(learningCheckbox).toBeChecked();

    // Disable learning
    await learningCheckbox.uncheck();
    await expect(learningCheckbox).not.toBeChecked();

    // Re-enable learning
    await learningCheckbox.check();
    await expect(learningCheckbox).toBeChecked();
  });

  test('should adjust simulation speed', async ({ page }) => {
    const speedSlider = page.locator('input[type="range"]');

    // Change speed to 500ms
    await speedSlider.fill('500');

    // Verify speed display updated
    await expect(page.locator('text=500ms')).toBeVisible();
  });
});

test.describe('Console Error Detection', () => {

  test('should not have console errors on page load', async ({ page }) => {
    const consoleErrors: string[] = [];

    page.on('console', msg => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    // Filter out known non-critical errors (like 404 for favicon)
    const criticalErrors = consoleErrors.filter(
      err => !err.includes('favicon') && !err.includes('404')
    );

    expect(criticalErrors).toHaveLength(0);
  });

  test('should not have console errors when initializing demo', async ({ page }) => {
    const consoleErrors: string[] = [];

    page.on('console', msg => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    await initializeDemoNetwork(page, 'sequence');

    const criticalErrors = consoleErrors.filter(
      err => !err.includes('favicon') && !err.includes('404')
    );

    expect(criticalErrors).toHaveLength(0);
  });
});

test.describe('Responsive Design', () => {

  test('should display correctly on desktop (1920x1080)', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    // Verify all major components are visible
    await expect(page.locator('#block-palette')).toBeVisible();
    await expect(page.locator('#network-svg')).toBeVisible();
    await expect(page.locator('text=Input/Output Values')).toBeVisible();
  });

  test('should display correctly on laptop (1366x768)', async ({ page }) => {
    await page.setViewportSize({ width: 1366, height: 768 });
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    await expect(page.locator('#block-palette')).toBeVisible();
    await expect(page.locator('#network-svg')).toBeVisible();
  });
});

test.describe('Performance', () => {

  test('should load page within acceptable time', async ({ page }) => {
    const startTime = Date.now();

    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    const loadTime = Date.now() - startTime;

    // Page should load and initialize WASM in < 5 seconds
    expect(loadTime).toBeLessThan(5000);
  });

  test('should initialize demo network quickly', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    const startTime = Date.now();

    await initializeDemoNetwork(page, 'sequence');

    const initTime = Date.now() - startTime;

    // Network should initialize in < 2 seconds
    expect(initTime).toBeLessThan(2000);
  });
});
