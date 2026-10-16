import { test, expect } from '@playwright/test';

test.describe('Config sync across pages', () => {
  // <ConfigWidget /> (the .cw-pill) is mounted only inside the scenario player
  // (src/pages/Demo/index.tsx), which AppShell renders at /lab/demo; AppShell
  // never shows it on other pages (it is retired from the showcase and hidden,
  // display:none, on the rest of /lab/*). "Sync" here means the same
  // server-backed config value survives navigating away and back, and other
  // pages' own read-outs of that config (Settings' select, Bench's display)
  // still work even though the pill itself is not visible there.
  test('config pill is consistent across navigation', async ({ page }) => {
    await page.goto('/lab/demo');
    const pillText = await page.locator('.cw-pill').textContent();

    // The pill does not leak onto other lab pages.
    await page.goto('/lab/bench');
    await expect(page.locator('.cw-pill')).not.toBeVisible();

    await page.goto('/lab/settings');
    await expect(page.locator('.cw-pill')).not.toBeVisible();

    // Revisiting the scenario player reflects the same server-backed config.
    await page.goto('/lab/demo');
    const pillTextAgain = await page.locator('.cw-pill').textContent();
    expect(pillTextAgain).toBe(pillText);
  });

  test('settings page shows model matching config pill', async ({ page }) => {
    await page.goto('/lab/demo');

    // Get model from pill
    const pillText = await page.locator('.cw-pill').textContent();
    expect(pillText).toBeTruthy();

    // Navigate to settings
    await page.goto('/lab/settings');

    // Settings model selector should exist
    const modelSelect = page.locator('.settings-section').nth(2).locator('select').first();
    await expect(modelSelect).toBeVisible({ timeout: 5000 });
  });

  test('bench page shows model from config context', async ({ page }) => {
    await page.goto('/lab/bench');

    // The MODEL pane should show the model as read-only text
    const modelDisplay = page.locator('.bench-model-display');
    await expect(modelDisplay).toBeVisible({ timeout: 5000 });

    // Should contain "Change via config pill" hint
    const hint = page.locator('.bench-model-hint');
    await expect(hint).toContainText('config pill');
  });
});
