import { test, expect, type APIRequestContext } from '@playwright/test';

/** One suite, so the Bench page renders its configure tab while roko serve is offline. */
const SUITE = {
  id: 'no-simulated-fixture',
  name: 'Fixture suite',
  description: 'A suite for the strategy list',
  tasks: [],
  estimated_cost_usd: 0,
  difficulty_range: [1, 1],
};

/** Whether a roko serve answers behind the dev server's /api proxy. */
async function serveReachable(request: APIRequestContext): Promise<boolean> {
  try {
    return (await request.get('/api/health', { timeout: 3_000 })).ok();
  } catch {
    return false;
  }
}

/** The JSON paths in `value` where `simulated` is true. */
function simulatedPaths(value: unknown, path = '$'): string[] {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => simulatedPaths(item, `${path}[${index}]`));
  }
  if (value === null || typeof value !== 'object') return [];
  return Object.entries(value).flatMap(([key, item]) => [
    ...(key === 'simulated' && item === true ? [`${path}.${key}`] : []),
    ...simulatedPaths(item, `${path}.${key}`),
  ]);
}

test.describe('no simulated bench results (S10 §7 scenario 3)', () => {
  test('the strategy list has no Demo, even without API keys', async ({ page }) => {
    // Without API keys the page used to switch to the simulated Demo strategy.
    await page.route(/\/api\/bench\/provider-status(\?|$)/, (route) => route.fulfill({
      json: { has_providers: false, has_api_keys: false, demo_available: true },
    }));
    await page.route(/\/api\/bench\/suites(\?|$)/, (route) => route.fulfill({
      json: { suites: [SUITE] },
    }));
    await page.goto('/lab/bench', { waitUntil: 'domcontentloaded' });

    const cards = page.locator('.config-cards .config-card');
    await expect(cards.first()).toBeVisible({ timeout: 15_000 });
    const labels = await cards.locator('.card-label').allInnerTexts();
    expect(labels.length).toBeGreaterThan(0);
    expect(labels.filter((label) => /demo/i.test(label))).toEqual([]);
    const descriptions = await cards.locator('.card-desc').allInnerTexts();
    expect(descriptions.filter((desc) => /simulat/i.test(desc))).toEqual([]);

    // A real strategy stays picked, and the no-keys warning does not point at Demo.
    await expect(page.locator('.config-cards .config-card.selected')).toHaveCount(1);
    const warning = page.locator('.bench-demo-warn');
    await expect(warning).toBeVisible();
    await expect(warning).not.toContainText(/demo/i);
    await expect(page.getByText(/results are simulated/i)).toHaveCount(0);
  });

  test('cost-summary counts no simulated run', async ({ request }) => {
    test.skip(!(await serveReachable(request)), 'roko serve is not reachable behind the dev server');
    const response = await request.get('/api/bench/cost-summary');
    expect(response.ok()).toBe(true);
    expect(simulatedPaths(await response.json())).toEqual([]);
  });
});
