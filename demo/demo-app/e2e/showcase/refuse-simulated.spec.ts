import { expect, test } from '@playwright/test';
import type { RefusalReason, ViewId } from '../../src/showcase/contracts';
import { refusalOf } from '../../src/showcase/guard';
import { POISONED_BUNDLES, expectFixtureServer, loadedView } from './fixtures';

/** Poisoned bundles and the views each one poisons (S10 §7 scenario 2). */
const CASES: { bundle: string; reason: RefusalReason; views: ViewId[] }[] = [
  {
    bundle: POISONED_BUNDLES.simulated,
    reason: 'simulated',
    views: ['overview', 'p1-head-to-head', 'm4-audits'],
  },
  {
    bundle: POISONED_BUNDLES.no_run_ids,
    reason: 'no_run_ids',
    views: ['overview', 'p1-head-to-head', 'm4-audits'],
  },
  { bundle: POISONED_BUNDLES.integrity, reason: 'integrity', views: ['p1-head-to-head'] },
];

test.describe('poisoned bundles are refused, not drawn (S10 SC2)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  for (const { bundle, reason, views } of CASES) {
    test(`${bundle}: RefusedPanel names ${reason} and no chart renders`, async ({ page }) => {
      await page.goto(`/replay/${bundle}`, { waitUntil: 'domcontentloaded' });
      const replay = page.locator('[data-showcase-page="replay"]');
      const refused = replay.locator(`[data-refused-reason="${reason}"]`);
      await expect(refused).toHaveCount(views.length, { timeout: 15_000 });
      await expect(refused.locator('svg')).toHaveCount(0);
      for (const view of views) {
        await expect(replay.locator(`[data-view="${view}"]`)).toHaveCount(0);
      }

      await page.goto(`/p1/head-to-head?bundle=${bundle}`, { waitUntil: 'domcontentloaded' });
      const h2h = page.locator('[data-showcase-page="head-to-head"]');
      await expect(h2h.locator(`[data-refused-reason="${reason}"]`)).toBeVisible({ timeout: 15_000 });
      await expect(h2h.locator('svg')).toHaveCount(0);
      await expect(h2h.locator('[data-metric-ref]')).toHaveCount(0);
    });
  }

  test('the fixture-kind bundle is refused under production rules', () => {
    // A test build allows fixtures; the production-build page check is showcase-static's.
    for (const view of ['overview', 'p1-head-to-head', 'm4-audits'] as const) {
      const loaded = loadedView(POISONED_BUNDLES.fixture, view);
      expect(refusalOf(loaded, { allowFixtures: false })?.reason).toBe('fixture');
      expect(refusalOf(loaded, { allowFixtures: true })).toBeNull();
    }
  });
});
