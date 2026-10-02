import { expect, test, type Page } from '@playwright/test';
import type { Estimate } from '../../src/showcase/contracts';
import { expectFixtureServer, storedView } from './fixtures';

const CHARTS = ['pareto', 'passk', 'envelope', 'fgr'];
const RATE_CHARTS = ['pareto', 'passk', 'fgr'];

async function openHarness(page: Page) {
  await page.goto('/__fixtures/charts?bundle=fx-golden', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('[data-harness="charts"]')).toBeVisible({ timeout: 15_000 });
}

/** Every `[data-metric-ref]` under `scope`, as [metric_ref, data-value] pairs. */
async function renderedValues(page: Page, scope: string): Promise<[string, string][]> {
  return page.locator(`${scope} [data-metric-ref][data-value]`).evaluateAll((nodes) => nodes.map(
    (node) => [node.getAttribute('data-metric-ref') ?? '', node.getAttribute('data-value') ?? ''],
  ));
}

test.describe('CI-aware charts with table fallbacks (S10 §4.1)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  test('every estimate is drawn with its interval', async ({ page }) => {
    await openHarness(page);
    const run = storedView('fx-golden', 'p1-head-to-head').arms.filter((a) => a.status === 'run');
    await expect(page.locator('[data-chart="pareto"] [data-ci="whisker"]'))
      .toHaveCount(run.length * 2);
    await expect(page.locator('[data-chart="passk"] [data-ci="whisker"]'))
      .toHaveCount(run.length * 3);
    await expect(page.locator('[data-chart="fgr"] [data-ci="band"]')).toHaveCount(1);
  });

  test('rate axes span [0,1]; dollars sit on a labelled log axis', async ({ page }) => {
    await openHarness(page);
    for (const chart of RATE_CHARTS) {
      await expect(page.locator(`[data-chart="${chart}"] [data-axis="y"]`))
        .toHaveAttribute('data-domain', '0,1');
    }
    const x = page.locator('[data-chart="pareto"] [data-axis="x"]');
    await expect(x).toHaveAttribute('data-scale', 'log');
    const labels = await x.locator('text').allTextContents();
    expect(labels.filter((label) => /^\$\d/.test(label)).length).toBeGreaterThanOrEqual(2);
  });

  test('the frontier line and the cost-source caveat render', async ({ page }) => {
    await openHarness(page);
    await expect(page.locator('[data-chart="pareto"] [data-frontier]')).toHaveCount(1);
    await expect(page.locator('[data-chart="pareto"] [data-caveat="cli_usage"]'))
      .toContainText('API-equivalent reported by the CLI');
  });

  test('golden values render exactly, in the charts and in their tables', async ({ page }) => {
    await openHarness(page);
    const h2h = storedView('fx-golden', 'p1-head-to-head');
    const m4 = storedView('fx-golden', 'm4-audits');
    const byRef = new Map([...h2h.metrics, ...m4.metrics].map((m) => [m.metric_ref, m.value]));

    const inCharts = await renderedValues(page, '[data-harness="charts"] svg');
    expect(inCharts.length).toBeGreaterThan(0);
    for (const [ref, value] of inCharts) expect(value, ref).toBe(String(byRef.get(ref)));

    for (const chart of RATE_CHARTS) {
      await page.locator(`[data-chart="${chart}"] .sc-chart__toggle`).click();
      await expect(page.locator(`[data-chart="${chart}"] table`)).toBeVisible();
    }
    const inTables = await renderedValues(page, '[data-harness="charts"] table');
    for (const [ref, value] of inTables) expect(value, ref).toBe(String(byRef.get(ref)));

    const shown = new Set(inTables.map(([ref]) => ref));
    const expected: (Estimate | null)[] = [];
    for (const arm of h2h.arms.filter((a) => a.status === 'run')) {
      expected.push(arm.resolve, arm.usd_per_verified);
      if (arm.pass_hat_k) expected.push(arm.pass_hat_k['1'], arm.pass_hat_k['3'], arm.pass_hat_k['5']);
    }
    for (const row of h2h.envelope) {
      expected.push(row.cheap_resolve, row.frontier_resolve, row.resolve_ratio, row.usd_ratio);
    }
    expected.push(...m4.series.map((point) => point.estimate));
    for (const estimate of expected) {
      if (estimate) expect(shown.has(estimate.metric_ref), estimate.metric_ref).toBe(true);
    }
  });

  test('at 390 px nothing scrolls sideways and every chart shows its table', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openHarness(page);
    for (const chart of CHARTS) {
      await expect(page.locator(`[data-chart="${chart}"] table`)).toHaveCount(1);
    }
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow).toBeLessThanOrEqual(0);
  });
});
