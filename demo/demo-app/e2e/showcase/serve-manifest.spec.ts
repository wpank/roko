import { expect, test } from '@playwright/test';

/**
 * R1-serve (S10 §7, 9333) against `roko serve`: `npx playwright test -c
 * playwright.showcase-serve.config.ts`. The config stages a replay bundle and `b-tampered`.
 */

const BUNDLE = process.env.ROKO_SHOWCASE_SERVE_BUNDLE ?? 'b-fixture-p1';

interface Summary {
  id: string;
  status: string;
  reason?: string;
}

test('the manifest lists the replay bundle as verified and the tampered one as rejected', async ({
  request,
}) => {
  const res = await request.get('/api/showcase/manifest');
  expect(res.ok()).toBe(true);
  const manifest = (await res.json()) as {
    schema: string;
    featured_bundle: string | null;
    bundles: Summary[];
  };
  expect(manifest.schema).toBe('showcase-manifest/1');
  const byId = new Map(manifest.bundles.map((bundle) => [bundle.id, bundle]));
  expect(byId.get(BUNDLE)?.status).toBe('verified');
  expect(byId.get('b-tampered')).toMatchObject({ status: 'rejected', reason: 'integrity' });
  expect(manifest.featured_bundle).toBe(BUNDLE);
});

test("the tampered bundle's views answer 409 bundle_rejected", async ({ request }) => {
  for (const path of ['/api/showcase/overview', '/api/showcase/p1/head-to-head']) {
    const res = await request.get(`${path}?source=bundle:b-tampered`);
    expect(res.status(), path).toBe(409);
    expect(await res.json()).toMatchObject({ error: 'bundle_rejected', reason: 'integrity' });
  }
});

test("the replay bundle's views carry sources the server verified", async ({ request }) => {
  const res = await request.get(`/api/showcase/p1/head-to-head?source=bundle:${BUNDLE}`);
  expect(res.ok()).toBe(true);
  const view = (await res.json()) as {
    provenance: { kind: string; sources: { sha256_verified: boolean }[] };
  };
  expect(view.provenance.kind).toBe('replay');
  expect(view.provenance.sources.length).toBeGreaterThan(0);
  expect(view.provenance.sources.every((source) => source.sha256_verified)).toBe(true);
});

test('cost-summary carries no simulated run (S10 scenario 3)', async ({ request }) => {
  const res = await request.get('/api/bench/cost-summary');
  expect(res.ok()).toBe(true);
  expect(JSON.stringify(await res.json())).not.toContain('"simulated":true');
});

test('the showcase at /demo/ renders the replay bundle through the api source', async ({ page }) => {
  await page.goto('/demo/');
  const overview = page.locator('[data-showcase-page="overview"] [data-view="overview"]');
  await expect(overview).toBeVisible({ timeout: 20_000 });
});
