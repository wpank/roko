import { test, expect } from '@playwright/test';

test.describe('Bench config integration', () => {
  test('configure tab has no standalone ModelPicker', async ({ page }) => {
    await page.goto('/lab/bench');

    // The MODEL pane should show read-only display, not a ModelPicker component
    const modelDisplay = page.locator('.bench-model-display');
    await expect(modelDisplay).toBeVisible({ timeout: 5000 });

    // There should be no model-picker component (class from old ModelPicker)
    const modelPicker = page.locator('.model-picker');
    await expect(modelPicker).toHaveCount(0);
  });

  // <ConfigWidget /> (the .cw-pill) is mounted only inside the scenario player
  // (src/pages/Demo/index.tsx), which AppShell renders at /lab/demo; AppShell
  // keeps it hidden (display:none) on every other /lab/* page, including
  // /lab/bench, and it is retired from the showcase.
  test('config pill is not visible on bench page', async ({ page }) => {
    await page.goto('/lab/bench');
    await expect(page.locator('.cw-pill')).not.toBeVisible();
  });

  test('bench model display shows current model', async ({ page }) => {
    await page.goto('/lab/bench');

    // Model display should have some text (even if "--" for offline)
    const modelDisplay = page.locator('.bench-model-display .param-value').first();
    await expect(modelDisplay).toBeVisible({ timeout: 5000 });
    const text = await modelDisplay.textContent();
    expect(text).toBeTruthy();
  });
});
