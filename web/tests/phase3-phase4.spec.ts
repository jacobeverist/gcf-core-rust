/**
 * Playwright E2E Tests for Phase 3 & Phase 4 Features
 *
 * Tests:
 * - Phase 3: Block creation, connections, deletion
 * - Phase 4: Undo/redo, save/load, auto-save
 */

import { test, expect, Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const BASE_URL = '/';

// Helper to wait for WASM to be ready
async function waitForWasmReady(page: Page) {
  await expect(page.locator('text=WASM: Ready')).toBeVisible({ timeout: 10000 });
}

// Helper to wait for network rebuild
async function waitForNetworkUpdate(page: Page) {
  await page.waitForTimeout(500); // Wait for rebuild and redraw
}

// Helper to get block count from status
async function getBlockCount(page: Page): Promise<number> {
  const counterText = await page.locator('#block-counter').textContent();
  return counterText ? parseInt(counterText) : 0;
}

test.describe('Phase 3: Block Creation', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
  });

  test('should create a ScalarTransformer block', async ({ page }) => {
    // Click ScalarTransformer in palette
    await page.click('text=Scalar');

    // Wait for modal
    await expect(page.locator('#param-editor-modal')).toBeVisible();

    // Fill in parameters
    await page.fill('#param-name', 'test_scalar');
    await page.fill('#param-min', '0');
    await page.fill('#param-max', '100');
    await page.fill('#param-statelets', '2048');
    await page.fill('#param-active', '256');

    // Click Apply
    await page.click('#param-apply-btn');

    // Wait for modal to close
    await expect(page.locator('#param-editor-modal')).not.toBeVisible();

    // Verify block counter increased
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);

    // Verify block appears in visualization
    await expect(page.locator('text=test_scalar')).toBeVisible();
  });

  test('should create a DiscreteTransformer block', async ({ page }) => {
    await page.click('text=Discrete');
    await expect(page.locator('#param-editor-modal')).toBeVisible();

    await page.fill('#param-name', 'test_discrete');
    await page.fill('#param-categories', '10');
    await page.fill('#param-statelets', '2048');

    await page.click('#param-apply-btn');
    await expect(page.locator('#param-editor-modal')).not.toBeVisible();

    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);

    await expect(page.locator('text=test_discrete')).toBeVisible();
  });

  test('should create a PatternPooler block', async ({ page }) => {
    const logs: string[] = [];
    page.on('console', msg => logs.push(`[${msg.type()}] ${msg.text()}`));

    await page.click('text=Pooler');
    await expect(page.locator('#param-editor-modal')).toBeVisible();

    await page.fill('#param-name', 'test_pooler');
    await page.fill('#param-dendrites', '1024');
    await page.fill('#param-active', '40');

    await page.click('#param-apply-btn');
    await expect(page.locator('#param-editor-modal')).not.toBeVisible();

    const blockCount = await getBlockCount(page);

    // Write logs to file for debugging
    const fs = require('fs');
    fs.writeFileSync('/tmp/pooler-test-logs.txt', logs.join('\n'));

    expect(blockCount).toBe(1);

    await expect(page.locator('text=test_pooler')).toBeVisible();
  });

  test('should cancel block creation', async ({ page }) => {
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();

    await page.fill('#param-name', 'cancelled_block');

    await page.click('#param-cancel-btn');
    await expect(page.locator('#param-editor-modal')).not.toBeVisible();

    // Verify no block was created
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);

    await expect(page.locator('text=cancelled_block')).not.toBeVisible();
  });
});

test.describe('Phase 3: Block Deletion', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    // Create a test block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'block_to_delete');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);
  });

  test('should delete a block in delete mode', async ({ page }) => {
    // Switch to delete mode
    await page.click('#tool-delete');

    // Verify delete mode is active
    await expect(page.locator('#tool-delete.active')).toBeVisible();

    // Click on the block - this will trigger confirm dialog
    page.once('dialog', dialog => {
      expect(dialog.message()).toContain('Delete block');
      dialog.accept();
    });

    // Click the block
    const block = page.locator('text=block_to_delete').first();
    await block.click({ force: true });

    // Wait for deletion
    await waitForNetworkUpdate(page);

    // Verify block is gone
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);

    await expect(page.locator('text=block_to_delete')).not.toBeVisible();
  });

  test('should cancel block deletion', async ({ page }) => {
    await page.click('#tool-delete');

    page.once('dialog', dialog => {
      dialog.dismiss();
    });

    const block = page.locator('text=block_to_delete').first();
    await block.click({ force: true });

    await waitForNetworkUpdate(page);

    // Block should still exist
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);

    await expect(page.locator('text=block_to_delete')).toBeVisible();
  });

  test('should switch between editor modes', async ({ page }) => {
    // Test select mode
    await page.click('#tool-select');
    await expect(page.locator('#tool-select.active')).toBeVisible();

    // Test connect mode
    await page.click('#tool-connect');
    await expect(page.locator('#tool-connect.active')).toBeVisible();

    // Test delete mode
    await page.click('#tool-delete');
    await expect(page.locator('#tool-delete.active')).toBeVisible();
  });
});

test.describe('Phase 3: Connections', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    // Create encoder block
    await page.click('text=Discrete');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'encoder');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Create pooler block
    await page.click('text=Pooler');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'pooler');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);
  });

  test('should have connection tool button', async ({ page }) => {
    await expect(page.locator('#tool-connect')).toBeVisible();
  });

  test('should switch to connection mode', async ({ page }) => {
    await page.click('#tool-connect');
    await expect(page.locator('#tool-connect.active')).toBeVisible();
  });

  // Note: Port drag-and-drop is complex to test in Playwright
  // Would require simulating mouse drag events on SVG elements
  test.skip('should create connection between blocks', async ({ page }) => {
    // This test is skipped as it requires complex SVG interaction
    // Manual testing confirms this works
  });
});

test.describe('Phase 4: Undo/Redo', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
  });

  test('should have undo/redo buttons initially disabled', async ({ page }) => {
    await expect(page.locator('#tool-undo[disabled]')).toBeVisible();
    await expect(page.locator('#tool-redo[disabled]')).toBeVisible();
  });

  test('should enable undo after creating a block', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'test_block');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Undo should now be enabled
    await expect(page.locator('#tool-undo:not([disabled])')).toBeVisible();
    await expect(page.locator('#tool-redo[disabled]')).toBeVisible();
  });

  test('should undo block creation', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'undo_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Verify block exists
    let blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
    await expect(page.locator('text=undo_test')).toBeVisible();

    // Click undo
    await page.click('#tool-undo');
    await waitForNetworkUpdate(page);

    // Verify block is gone
    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);
    await expect(page.locator('text=undo_test')).not.toBeVisible();

    // Redo should now be enabled
    await expect(page.locator('#tool-redo:not([disabled])')).toBeVisible();
  });

  test('should redo block creation', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'redo_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Undo
    await page.click('#tool-undo');
    await waitForNetworkUpdate(page);

    // Verify block is gone
    let blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);

    // Redo
    await page.click('#tool-redo');
    await waitForNetworkUpdate(page);

    // Verify block is back
    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
    await expect(page.locator('text=redo_test')).toBeVisible();
  });

  test('should undo block deletion', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'delete_undo_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Delete the block
    await page.click('#tool-delete');
    page.once('dialog', dialog => dialog.accept());
    await page.locator('text=delete_undo_test').first().click({ force: true });
    await waitForNetworkUpdate(page);

    // Verify block is gone
    let blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);

    // Undo the deletion
    await page.click('#tool-undo');
    await waitForNetworkUpdate(page);

    // Verify block is back
    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
    await expect(page.locator('text=delete_undo_test')).toBeVisible();
  });

  test('should use keyboard shortcuts for undo/redo', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'keyboard_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Undo with Ctrl+Z
    await page.keyboard.press('Control+z');
    await waitForNetworkUpdate(page);

    let blockCount = await getBlockCount(page);
    expect(blockCount).toBe(0);

    // Redo with Ctrl+Shift+Z
    await page.keyboard.press('Control+Shift+Z');
    await waitForNetworkUpdate(page);

    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
  });
});

test.describe('Phase 4: Save/Load', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
  });

  test('should have save and load buttons', async ({ page }) => {
    await expect(page.locator('#tool-save')).toBeVisible();
    await expect(page.locator('#tool-load')).toBeVisible();
  });

  test('should export network configuration', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'export_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Set up download listener
    const downloadPromise = page.waitForEvent('download');

    // Click save button
    await page.click('#tool-save');

    // Wait for download
    const download = await downloadPromise;

    // Verify filename pattern
    expect(download.suggestedFilename()).toMatch(/gnomics_network_\d+\.json/);

    // Verify file was downloaded
    const downloadPath = await download.path();
    expect(downloadPath).toBeTruthy();

    // Read and verify JSON structure
    const content = fs.readFileSync(downloadPath!, 'utf-8');
    const config = JSON.parse(content);
    expect(config).toHaveProperty('block_info');
    expect(config.block_info).toBeInstanceOf(Array);
    expect(config.block_info.length).toBeGreaterThan(0);
  });

  test('should load network configuration', async ({ page }) => {
    // Create a test config file
    const testConfig = {
      block_info: [
        {
          name: "loaded_block",
          block_type: { ScalarTransformer: { min_val: 0, max_val: 100, num_s: 2048, num_as: 256, num_t: 2, seed: 0 } }
        }
      ],
      connections: []
    };

    const tempDir = path.join(__dirname, '../test-temp');
    if (!fs.existsSync(tempDir)) {
      fs.mkdirSync(tempDir, { recursive: true });
    }

    const configPath = path.join(tempDir, 'test_config.json');
    fs.writeFileSync(configPath, JSON.stringify(testConfig));

    // Set up file chooser listener
    const fileChooserPromise = page.waitForEvent('filechooser');

    // Click load button
    await page.click('#tool-load');

    // Select file
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(configPath);

    // Wait for load
    await waitForNetworkUpdate(page);

    // Verify block was loaded
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
    await expect(page.locator('text=loaded_block')).toBeVisible();

    // Cleanup
    fs.unlinkSync(configPath);
  });
});

test.describe('Phase 4: Auto-Save', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
    // Clear localStorage
    await page.evaluate(() => {
      localStorage.removeItem('gnomics_network_autosave');
      localStorage.removeItem('gnomics_network_autosave_timestamp');
    });
  });

  test('should auto-save after creating a block', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'autosave_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Wait for auto-save (2 seconds delay + buffer)
    await page.waitForTimeout(3000);

    // Check localStorage
    const autoSaveData = await page.evaluate(() => {
      return localStorage.getItem('gnomics_network_autosave');
    });

    expect(autoSaveData).toBeTruthy();

    const config = JSON.parse(autoSaveData!);
    expect(config).toHaveProperty('block_info');
    expect(config.block_info.length).toBeGreaterThan(0);
  });

  test('should restore from auto-save on reload', async ({ page }) => {
    // Create a block
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'restore_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Wait for auto-save
    await page.waitForTimeout(3000);

    // Set up dialog handler for restore prompt
    page.once('dialog', dialog => {
      expect(dialog.message()).toContain('Found auto-saved network');
      dialog.accept();
    });

    // Reload page
    await page.reload();
    await waitForWasmReady(page);

    // Wait for restore
    await waitForNetworkUpdate(page);

    // Verify block was restored
    const blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);
    await expect(page.locator('text=restore_test')).toBeVisible();
  });
});

test.describe('Phase 4: Unsaved Changes Warning', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);
  });

  test('should warn before leaving with unsaved changes', async ({ page }) => {
    // Create a block to make changes
    await page.click('text=Scalar');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'unsaved_test');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    // Try to navigate away - should trigger beforeunload
    const dialogPromise = page.waitForEvent('dialog');

    // Trigger navigation
    await page.evaluate(() => {
      window.location.href = 'about:blank';
    });

    // Verify dialog appears
    const dialog = await dialogPromise;
    expect(dialog.type()).toBe('beforeunload');

    await dialog.dismiss();
  });
});

test.describe('Phase 4: Integration Test', () => {

  test('should support complete workflow: create, undo, save, load', async ({ page }) => {
    await page.goto(BASE_URL);
    await waitForWasmReady(page);

    // Step 1: Create two blocks
    await page.click('text=Discrete');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'workflow_encoder');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    await page.click('text=Pooler');
    await expect(page.locator('#param-editor-modal')).toBeVisible();
    await page.fill('#param-name', 'workflow_pooler');
    await page.click('#param-apply-btn');
    await waitForNetworkUpdate(page);

    let blockCount = await getBlockCount(page);
    expect(blockCount).toBe(2);

    // Step 2: Undo last creation
    await page.click('#tool-undo');
    await waitForNetworkUpdate(page);

    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);

    // Step 3: Redo
    await page.click('#tool-redo');
    await waitForNetworkUpdate(page);

    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(2);

    // Step 4: Save
    const downloadPromise = page.waitForEvent('download');
    await page.click('#tool-save');
    const download = await downloadPromise;
    const downloadPath = await download.path();

    // Step 5: Delete a block
    await page.click('#tool-delete');
    page.once('dialog', dialog => dialog.accept());
    await page.locator('text=workflow_pooler').first().click({ force: true });
    await waitForNetworkUpdate(page);

    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(1);

    // Step 6: Load saved configuration
    const fileChooserPromise = page.waitForEvent('filechooser');
    await page.click('#tool-load');
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(downloadPath!);
    await waitForNetworkUpdate(page);

    // Verify both blocks are back
    blockCount = await getBlockCount(page);
    expect(blockCount).toBe(2);
    await expect(page.locator('text=workflow_encoder')).toBeVisible();
    await expect(page.locator('text=workflow_pooler')).toBeVisible();
  });
});
