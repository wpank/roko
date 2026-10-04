import { expect, test, type Page } from '@playwright/test';
import { SHOWCASE_ROUTES, expectFixtureServer, openShowcaseRoute } from './fixtures';

/**
 * What S10 §4.4 retires from the showcase: they show invented activity, or read
 * `/api/bench/cost-summary`, which may count simulated runs. `module` matches the dev server's
 * module URL; each marker is a string of the module that minifying keeps, so the check holds for
 * a production build's hashed chunks too. The lab pages keep them.
 */
const RETIRED: { piece: string; module: RegExp; markers: string[] }[] = [
  {
    piece: 'the fake terminal lines (TerminalPreview FAKE_LINES)',
    module: /\/components\/TerminalPreview\./,
    markers: ['FAKE_LINES', 'roko prd plan system-prompt'],
  },
  {
    piece: 'the sample pipeline (lib/prd-pipeline-sample)',
    module: /\/lib\/prd-pipeline-sample\./,
    markers: ['roko_pipeline_release_watch'],
  },
  {
    piece: 'CostRace',
    module: /\/components\/CostRace\./,
    markers: ['/api/bench/cost-summary'],
  },
  {
    piece: 'the Demo scenario player (pages/Demo)',
    module: /\/pages\/Demo\//,
    markers: ['demo-topbar-playback'],
  },
];

interface Loaded {
  /** Every same-origin URL the page fetched. */
  urls: Set<string>;
  /** The scripts among them: modules in dev, chunks in a build. */
  scripts: Set<string>;
}

function watch(page: Page, origin: string): Loaded {
  const loaded: Loaded = { urls: new Set(), scripts: new Set() };
  page.on('response', (response) => {
    const url = response.url();
    if (!url.startsWith(origin)) return;
    loaded.urls.add(url);
    const script = response.request().resourceType() === 'script'
      || /\.(m?js|jsx|tsx?)$/.test(new URL(url).pathname);
    if (script) loaded.scripts.add(url);
  });
  return loaded;
}

/** Lazy chunks load while the page renders: wait for a second with no new script. */
async function settle(page: Page, loaded: Loaded): Promise<void> {
  let seen: number;
  do {
    seen = loaded.scripts.size;
    await page.waitForTimeout(1_000);
  } while (loaded.scripts.size !== seen);
}

test.describe('the retired pieces stay out of the showcase (S10 §4.4)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  for (const route of SHOWCASE_ROUTES) {
    test(`${route.path} loads no chunk holding a retired piece`, async ({ page, baseURL }) => {
      const loaded = watch(page, new URL(baseURL ?? 'http://localhost:5173').origin);
      await openShowcaseRoute(page, route);
      await settle(page, loaded);

      for (const { piece, module } of RETIRED) {
        const urls = [...loaded.urls].filter((url) => module.test(new URL(url).pathname));
        expect(urls, `${route.path} loads ${piece}`).toEqual([]);
      }
      let scanned = 0;
      for (const url of loaded.scripts) {
        const path = new URL(url).pathname;
        // Dependencies and the dev client hold no app code.
        if (path.includes('/node_modules/') || path.startsWith('/@')) continue;
        const body = await (await page.request.get(url)).text();
        scanned += 1;
        for (const { piece, markers } of RETIRED) {
          const found = markers.filter((marker) => body.includes(marker));
          expect(found, `${route.path} loads ${path}, which holds ${piece}`).toEqual([]);
        }
      }
      expect(scanned, `${route.path} loaded no app script`).toBeGreaterThan(0);
    });
  }

  test('no showcase route links to the Demo scenario player', async ({ page }) => {
    for (const route of SHOWCASE_ROUTES) {
      await openShowcaseRoute(page, route);
      await expect(page.locator('a[href$="/lab/demo"]'), route.path).toHaveCount(0);
      await expect(page.locator('nav.labnav'), route.path).toHaveCount(0);
    }
  });

  test('leaving the lab unmounts the Demo scenario player', async ({ page }) => {
    await page.goto('/lab/demo', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.demo-page')).toHaveCount(1, { timeout: 15_000 });
    await page.locator('nav.topnav .links .nav-link').filter({ hasText: 'OVERVIEW' }).click();
    const overview = page.locator('[data-showcase-page="overview"] [data-view="overview"]');
    await expect(overview).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('.demo-page')).toHaveCount(0);
  });
});
