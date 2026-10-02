import { expect, test } from '@playwright/test';
import type { ConfigEnv, UserConfig } from 'vite';
import viteConfig from '../../vite.config';
import type { RefusalReason, ViewId } from '../../src/showcase/contracts';
import {
  RenderRefusal,
  assertRenderable,
  fixturesAllowed,
  refusalOf,
  type GuardOptions,
} from '../../src/showcase/guard';
import {
  POISONED_BUNDLES,
  VALID_BUNDLES,
  bundleManifest,
  loadedView,
  storedView,
} from './fixtures';

const FIXTURES: GuardOptions = { allowFixtures: true };

/** The reason `assertRenderable` gives for `view`, or null when it renders. */
function thrownReason(view: unknown, options: GuardOptions): RefusalReason | null {
  try {
    assertRenderable(view, options);
    return null;
  } catch (error) {
    expect(error).toBeInstanceOf(RenderRefusal);
    return (error as RenderRefusal).reason;
  }
}

function views(bundleId: string): ViewId[] {
  return bundleManifest(bundleId).views;
}

test.describe('render guard (S10 §4.5)', () => {
  for (const bundleId of VALID_BUNDLES) {
    test(`${bundleId} renders in a fixture build`, () => {
      for (const view of views(bundleId)) {
        expect(thrownReason(loadedView(bundleId, view), FIXTURES), view).toBeNull();
      }
    });
  }

  test('outside a fixture build the default refuses fixtures', () => {
    // Node has no import.meta.env, so the defaults are a production build's.
    expect(fixturesAllowed()).toBe(false);
    expect(thrownReason(loadedView('fx-golden', 'overview'), {})).toBe('fixture');
  });

  test('a stored view is refused until a loader checks its digests', () => {
    expect(thrownReason(storedView('fx-golden', 'overview'), FIXTURES)).toBe('integrity');
  });

  const poisoned: [string, RefusalReason, GuardOptions][] = [
    [POISONED_BUNDLES.simulated, 'simulated', FIXTURES],
    [POISONED_BUNDLES.no_run_ids, 'no_run_ids', FIXTURES],
    [POISONED_BUNDLES.fixture, 'fixture', { allowFixtures: false }],
  ];
  for (const [bundleId, reason, options] of poisoned) {
    test(`${bundleId} is refused with ${reason}`, () => {
      for (const view of views(bundleId)) {
        expect(thrownReason(loadedView(bundleId, view), options), view).toBe(reason);
      }
    });
  }

  test(`${POISONED_BUNDLES.integrity} is refused where it was edited`, () => {
    const bundleId = POISONED_BUNDLES.integrity;
    expect(thrownReason(loadedView(bundleId, 'p1-head-to-head'), FIXTURES)).toBe('integrity');
    expect(thrownReason(loadedView(bundleId, 'overview'), FIXTURES)).toBeNull();
  });

  test('a simulated source, a missing n, a rate without a CI and a stray number', () => {
    const golden = () => loadedView('fx-golden', 'p1-head-to-head');

    const simulatedSource = golden();
    simulatedSource.provenance.sources[0].simulated = true;
    expect(refusalOf(simulatedSource, FIXTURES)?.reason).toBe('simulated');

    const noN = golden();
    noN.metrics[0].n = 0;
    expect(refusalOf(noN, FIXTURES)?.reason).toBe('n_missing');

    const noCi = golden();
    const rate = noCi.metrics.find((m) => m.kind === 'rate');
    if (!rate) throw new Error('the golden view has no rate');
    rate.ci = null;
    expect(refusalOf(noCi, FIXTURES)?.reason).toBe('ci_missing');
    rate.ci_method = 'none';
    expect(refusalOf(noCi, FIXTURES)).toBeNull();

    const stray = golden();
    stray.metrics = stray.metrics.slice(1);
    expect(refusalOf(stray, FIXTURES)?.reason).toBe('n_missing');
  });

  test('a view without provenance or sources is refused', () => {
    const view = loadedView('fx-golden', 'overview');
    expect(refusalOf({ ...view, provenance: undefined }, FIXTURES)?.reason)
      .toBe('missing_provenance');
    expect(refusalOf({ ...view, provenance: { ...view.provenance, sources: [] } }, FIXTURES)?.reason)
      .toBe('missing_provenance');
    expect(refusalOf(null, FIXTURES)?.reason).toBe('missing_provenance');
  });

  test('a production build with VITE_ALLOW_FIXTURES set fails at build time', () => {
    const config = viteConfig as (env: ConfigEnv) => UserConfig;
    const env = (command: 'build' | 'serve', mode: string): ConfigEnv => ({ command, mode });
    const saved = process.env.VITE_ALLOW_FIXTURES;
    try {
      process.env.VITE_ALLOW_FIXTURES = '1';
      expect(() => config(env('build', 'production'))).toThrow(/VITE_ALLOW_FIXTURES/);
      expect(() => config(env('serve', 'development'))).not.toThrow();
      delete process.env.VITE_ALLOW_FIXTURES;
      expect(() => config(env('build', 'production'))).not.toThrow();
    } finally {
      if (saved === undefined) delete process.env.VITE_ALLOW_FIXTURES;
      else process.env.VITE_ALLOW_FIXTURES = saved;
    }
  });
});
