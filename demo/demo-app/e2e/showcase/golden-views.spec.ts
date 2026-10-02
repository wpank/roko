import { expect, test, type Locator } from '@playwright/test';
import type { ViewId } from '../../src/showcase/contracts';
import { expectFixtureServer, storedView } from './fixtures';

/** The R1 pages and the view each renders (S10 §4.2). */
const PAGES: { path: string; page: string; view: ViewId }[] = [
  { path: '/', page: 'overview', view: 'overview' },
  { path: '/p1/head-to-head', page: 'head-to-head', view: 'p1-head-to-head' },
  { path: '/p2/audits', page: 'audits', view: 'm4-audits' },
];

/** The value at a dotted path (`by_check.A1_tamper.run`, `draws.0.pi`) of a view. */
function at(value: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>(
    (node, key) => (node as Record<string, unknown> | undefined)?.[key],
    value,
  );
}

async function pairs(scope: Locator, selector: string, attribute: string, valueOf: 'attr' | 'text') {
  return scope.locator(selector).evaluateAll(
    (nodes, [name, mode]) => nodes.map((node) => [
      node.getAttribute(name) ?? '',
      mode === 'attr' ? (node.getAttribute('data-value') ?? '') : (node.textContent ?? ''),
    ]),
    [attribute, valueOf] as const,
  );
}

test.describe('R1 pages render the golden bundle exactly (S10 §7 scenario 4)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  for (const { path, page: name, view } of PAGES) {
    test(`${name}: every number equals views/${view}.json`, async ({ page }) => {
      await page.goto(path, { waitUntil: 'domcontentloaded' });
      const root = page.locator(`[data-showcase-page="${name}"] [data-view="${view}"]`);
      await expect(root).toBeVisible({ timeout: 15_000 });
      const json = storedView('fx-golden', view);

      const byRef = new Map(json.metrics.map((m) => [m.metric_ref, m.value]));
      const values = await pairs(root, '[data-metric-ref][data-value]', 'data-metric-ref', 'attr');
      expect(values.length).toBeGreaterThan(0);
      for (const [ref, value] of values) expect(value, ref).toBe(String(byRef.get(ref)));
      const shown = new Set(values.map(([ref]) => ref));
      for (const metric of json.metrics) {
        expect(shown.has(metric.metric_ref), `${metric.metric} ${metric.metric_ref}`).toBe(true);
      }

      const counts = await pairs(root, '[data-view-path]', 'data-view-path', 'text');
      for (const [jsonPath, text] of counts) expect(text, jsonPath).toBe(String(at(json, jsonPath)));
    });
  }

  test('claim states are copied verbatim', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    const overview = storedView('fx-golden', 'overview');
    for (const tile of overview.tiles) {
      await expect(page.locator(`[data-tile="${tile.id}"]`))
        .toHaveAttribute('data-claim-state', tile.claim_state, { timeout: 15_000 });
    }
    await page.goto('/p1/head-to-head', { waitUntil: 'domcontentloaded' });
    const claim = storedView('fx-golden', 'p1-head-to-head').claim;
    const banner = page.locator('[data-showcase-page="head-to-head"] .sc-claim');
    await expect(banner).toHaveAttribute('data-claim-state', claim.state, { timeout: 15_000 });
    await expect(banner).toContainText(claim.text);
  });

  test('head-to-head: three arms plus the probe, never "2×2" (scenario 5)', async ({ page }) => {
    await page.goto('/p1/head-to-head', { waitUntil: 'domcontentloaded' });
    const root = page.locator('[data-showcase-page="head-to-head"]');
    await expect(root.locator('[data-view="p1-head-to-head"]')).toBeVisible({ timeout: 15_000 });
    const probe = root.locator('[data-arm="fr_claude"]');
    await expect(probe).toContainText('probe (48 H3 tasks)');
    await expect(root.locator('[data-arm-status="not_run"]').first()).toHaveText('not run');
    await expect(root).not.toContainText('2×2');
    await expect(root.locator('[data-chart="pareto"] [data-frontier]')).toHaveCount(1);
    await expect(root.locator('[data-chart="passk"] [data-axis="x"]')).toContainText('k = 5');
    const levels = storedView('fx-golden', 'p1-head-to-head').envelope.length;
    await expect(root.locator('[data-chart="envelope"] [data-verdict]')).toHaveCount(levels);
    await expect(root.locator('[data-caveat="cli_usage"]').first()).toBeVisible();
  });

  test('audits: FGR with its estimator, and Draw now is replay-only (scenario 6)', async ({ page }) => {
    await page.goto('/p2/audits', { waitUntil: 'domcontentloaded' });
    const root = page.locator('[data-showcase-page="audits"]');
    await expect(root.locator('[data-panel="fgr"]')).toContainText('Hájek (ratio HT)', { timeout: 15_000 });
    const draw = root.locator('[data-panel="draw-now"] button');
    await expect(draw).toBeDisabled();
    await expect(draw).toHaveAttribute('title', /replay-only/);
  });

  test('negative results render in the same slots (SC4, scenario 8)', async ({ page }) => {
    await page.goto('/?bundle=fx-negatives', { waitUntil: 'domcontentloaded' });
    const negatives = page.locator('[data-negatives] [data-negative]');
    await expect(negatives).toHaveCount(3, { timeout: 15_000 });
    for (const kind of ['frontier_wins', 'loop_dormant', 'fgr_above_bound']) {
      await expect(page.locator(`[data-negative="${kind}"]`)).toBeVisible();
    }
    await page.goto('/p1/head-to-head?bundle=fx-negatives', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('[data-verdict="frontier_wins"]')).toBeVisible({ timeout: 15_000 });
  });

  test('a bundle without audits says not yet measured', async ({ page }) => {
    await page.goto('/p2/audits?bundle=fx-p1-only', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('[data-showcase-page="audits"] [data-not-measured]'))
      .toContainText('not yet measured · planned in E-H5-live', { timeout: 15_000 });
  });

  test('every tile opens a provenance drawer (SC1)', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    const tiles = page.locator('[data-tile]');
    await expect(tiles.first()).toBeVisible({ timeout: 15_000 });
    const count = await tiles.count();
    await expect(page.locator('[data-tile] .sc-prov-btn')).toHaveCount(count);
    await tiles.first().locator('.sc-prov-btn').click();
    const drawer = page.locator('.sc-drawer[role="dialog"]');
    await expect(drawer).toBeVisible();
    for (const field of ['kind', 'sources', 'harness_commit', 'n', 'estimator', 'ci', 'reproduce']) {
      await expect(drawer.locator(`[data-prov-field="${field}"]`)).toBeVisible();
    }
    await expect(drawer.locator('[data-sha256-verified="true"]')).toHaveCount(1);
    await page.keyboard.press('Escape');
    await expect(drawer).toHaveCount(0);
  });

  test('replays list the bundles, featured first, and replay one under the watermark', async ({ page }) => {
    await page.goto('/replays', { waitUntil: 'domcontentloaded' });
    const items = page.locator('.sc-replays li');
    await expect(items.first()).toHaveAttribute('data-bundle', 'fx-golden', { timeout: 15_000 });
    await expect(items.nth(1)).toHaveAttribute('data-bundle', 'fx-negatives');
    await items.first().locator('a').click();
    await expect(page).toHaveURL(/\/replay\/fx-golden$/);
    await expect(page.locator('[data-watermark="replay"]')).toContainText('REPLAY of');
    for (const view of ['overview', 'p1-head-to-head', 'm4-audits']) {
      await expect(page.locator(`[data-showcase-page="replay"] [data-view="${view}"]`))
        .toBeVisible({ timeout: 15_000 });
    }
  });
});
