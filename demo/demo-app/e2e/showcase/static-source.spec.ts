import { expect, test } from '@playwright/test';
import {
  ShowcaseLoadError,
  createApiSource,
  createStaticSource,
  type ShowcaseSource,
} from '../../src/showcase/api';
import type { HeadToHeadView, ShowcaseEvent } from '../../src/showcase/contracts';
import { refusalOf } from '../../src/showcase/guard';
import { reduceEvent, useShowcaseStore, viewKey } from '../../src/showcase/store';
import { RokoApi } from '../../src/transport/api';
import {
  POISONED_BUNDLES,
  bundleIndex,
  bundleManifest,
  expectFixtureServer,
  loadedView,
} from './fixtures';

const FIXTURES = { allowFixtures: true };

function staticSource(baseURL: string | undefined): ShowcaseSource {
  return createStaticSource({ base: `${baseURL ?? 'http://localhost:5173'}/bundles/` });
}

async function loadError(promise: Promise<unknown>): Promise<ShowcaseLoadError> {
  const error = await promise.then(() => null, (err: unknown) => err);
  expect(error).toBeInstanceOf(ShowcaseLoadError);
  return error as ShowcaseLoadError;
}

test.describe('showcase data client (S10 §4.8)', () => {
  test.beforeAll(async ({ request }) => {
    await expectFixtureServer(request);
  });

  test('the static source reads the bundle index', async ({ baseURL }) => {
    const manifest = await staticSource(baseURL).manifest();
    expect(manifest.featured_bundle).toBe('fx-golden');
    expect(manifest.bundles.map((b) => b.id)).toContain('fx-poison-tampered');
  });

  test('the golden bundle loads with every source verified', async ({ baseURL }) => {
    const source = staticSource(baseURL);
    for (const view of bundleManifest('fx-golden').views) {
      const loaded = await source.view('fx-golden', view);
      expect(loaded.provenance.sources.length).toBeGreaterThan(0);
      expect(loaded.provenance.sources.every((s) => s.sha256_verified)).toBe(true);
      expect(refusalOf(loaded, FIXTURES)).toBeNull();
    }
  });

  test('the tampered bundle loads unverified and the guard refuses it', async ({ baseURL }) => {
    const source = staticSource(baseURL);
    const tampered = await source.view(POISONED_BUNDLES.integrity, 'p1-head-to-head');
    expect(tampered.provenance.sources.map((s) => s.sha256_verified)).toEqual([false]);
    expect(refusalOf(tampered, FIXTURES)?.reason).toBe('integrity');
    const untouched = await source.view(POISONED_BUNDLES.integrity, 'overview');
    expect(refusalOf(untouched, FIXTURES)).toBeNull();
  });

  test('every bundle agrees with the loader rule the guard spec uses', async ({ baseURL }) => {
    const source = staticSource(baseURL);
    for (const { id } of bundleIndex().bundles) {
      for (const view of bundleManifest(id).views) {
        expect(await source.view(id, view), `${id} ${view}`).toEqual(loadedView(id, view));
      }
    }
  });

  test('a view a bundle lacks is not found; a bad bundle id is never fetched', async ({ baseURL }) => {
    const source = staticSource(baseURL);
    expect((await loadError(source.view('fx-p1-only', 'm4-audits'))).kind).toBe('not_found');
    expect((await loadError(source.view('../fx-golden', 'overview'))).kind).toBe('not_found');
  });

  test('the api source returns the same typed view, and 409 is a rejection', async ({ baseURL }) => {
    const verified = await staticSource(baseURL).view('fx-golden', 'p1-head-to-head');
    const original = globalThis.fetch;
    const requested: string[] = [];
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      const url = String(input);
      requested.push(url);
      if (url.includes('bundle%3Afx-poison-tampered')) {
        return new Response('{"error":"bundle_rejected"}', { status: 409 });
      }
      return new Response(JSON.stringify(verified), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }) as typeof fetch;
    try {
      const source = createApiSource(new RokoApi('http://serve.test'));
      expect(await source.view('fx-golden', 'p1-head-to-head')).toEqual(verified);
      expect(requested[0]).toBe(
        'http://serve.test/api/showcase/p1/head-to-head?source=bundle%3Afx-golden',
      );
      const rejected = await loadError(source.view(POISONED_BUNDLES.integrity, 'overview'));
      expect(rejected.kind).toBe('rejected');
    } finally {
      globalThis.fetch = original;
    }
  });

  test('metric.update events reduce into the store', async ({ baseURL }) => {
    const view = await staticSource(baseURL).view('fx-golden', 'p1-head-to-head');
    const store = useShowcaseStore.getState();
    store.reset();
    store.setView('fx-golden', 'p1-head-to-head', { status: 'ready', data: view });
    store.setView('fx-negatives', 'p1-head-to-head', { status: 'ready', data: view });
    const roko = view.arms.find((a) => a.arm === 'roko_fixed')?.resolve;
    if (!roko) throw new Error('the golden view has no cheap·roko resolve');
    const event: ShowcaseEvent = {
      schema: 'showcase-event/1',
      seq: 7,
      t_ms: 1200,
      source: 'replay',
      run_id: null,
      attempt_key: null,
      type: 'metric.update',
      payload: { metric_ref: roko.metric_ref, value: 0.5, ci: [0.4, 0.6], n: 10 },
    };
    useShowcaseStore.getState().applyEvent('fx-golden', event);

    const state = useShowcaseStore.getState().views[viewKey('fx-golden', 'p1-head-to-head')];
    if (state?.status !== 'ready') throw new Error('the view is not ready');
    const updated = state.data as HeadToHeadView;
    expect(updated.arms.find((a) => a.arm === 'roko_fixed')?.resolve)
      .toEqual({ value: 0.5, ci: [0.4, 0.6], metric_ref: roko.metric_ref });
    expect(updated.metrics.find((m) => m.metric_ref === roko.metric_ref))
      .toMatchObject({ value: 0.5, ci: [0.4, 0.6], n: 10 });
    // Another bundle and other event types are left alone.
    const other = useShowcaseStore.getState().views[viewKey('fx-negatives', 'p1-head-to-head')];
    expect(other).toEqual({ status: 'ready', data: view });
    const views = useShowcaseStore.getState().views;
    expect(reduceEvent(views, 'fx-golden', { ...event, type: 'gate.verdict' })).toBe(views);

    // Replaying the recorded value restores the static view exactly (S10 §4.7).
    useShowcaseStore.getState().applyEvent('fx-golden', {
      ...event,
      payload: { metric_ref: roko.metric_ref, value: roko.value, ci: roko.ci, n: 200 },
    });
    const replayed = useShowcaseStore.getState().views[viewKey('fx-golden', 'p1-head-to-head')];
    expect(replayed).toEqual({ status: 'ready', data: view });
    store.reset();
  });

  test('tampering is caught in the browser', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await page.addScriptTag({
      type: 'module',
      content: [
        "import { createStaticSource } from '/src/showcase/api.ts';",
        'const source = createStaticSource();',
        "const golden = await source.view('fx-golden', 'p1-head-to-head');",
        "const tampered = await source.view('fx-poison-tampered', 'p1-head-to-head');",
        'window.__showcaseDigests = {',
        '  golden: golden.provenance.sources.map((s) => s.sha256_verified),',
        '  tampered: tampered.provenance.sources.map((s) => s.sha256_verified),',
        '};',
      ].join('\n'),
    });
    const handle = await page.waitForFunction(
      () => (window as unknown as { __showcaseDigests?: unknown }).__showcaseDigests,
    );
    expect(await handle.jsonValue()).toEqual({ golden: [true], tampered: [false] });
  });
});
