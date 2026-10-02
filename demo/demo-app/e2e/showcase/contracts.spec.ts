import { expect, test } from '@playwright/test';
import {
  CLAIM_STATES,
  R1_VIEWS,
  VIEW_SCHEMAS,
  type ShowcaseView,
} from '../../src/showcase/contracts';
import {
  POISONED_BUNDLES,
  VALID_BUNDLES,
  bundleIndex,
  bundleManifest,
  digestMismatches,
  loadSchema,
  metricRows,
  storedView,
} from './fixtures';
import { errorPaths, schemaErrors, type JsonSchema } from './schema';

const SCHEMA_NAMES = [
  'provenance',
  'bundle',
  'manifest',
  'event',
  'overview',
  'p1-head-to-head',
  'm4-audits',
];

/** Every failing path of a bundle: its manifest, then each of its views. */
function bundlePaths(bundleId: string): string[] {
  const errors = schemaErrors(bundleManifest(bundleId), loadSchema('bundle'));
  for (const view of bundleManifest(bundleId).views) {
    errors.push(...schemaErrors(storedView(bundleId, view), loadSchema(view)));
  }
  return errorPaths(errors);
}

/** Production rules on top of the schemas: only measured or replay data (S10 §4.5). */
function productionPaths(bundleId: string): string[] {
  const paths: string[] = [];
  const isReal = (kind: string) => kind === 'measured' || kind === 'replay';
  if (!isReal(bundleManifest(bundleId).kind)) paths.push('$.kind');
  for (const view of bundleManifest(bundleId).views) {
    if (!isReal(storedView(bundleId, view).provenance.kind)) paths.push('$.provenance.kind');
  }
  return [...new Set(paths)].sort();
}

/** Every inline estimate of a view (an object with `metric_ref` and `value`), with its path. */
function inlineEstimates(node: unknown, path = '$'): [string, Record<string, unknown>][] {
  if (Array.isArray(node)) return node.flatMap((item, i) => inlineEstimates(item, `${path}[${i}]`));
  if (typeof node !== 'object' || node === null) return [];
  const record = node as Record<string, unknown>;
  const found: [string, Record<string, unknown>][] = [];
  if (typeof record.metric_ref === 'string' && 'value' in record) found.push([path, record]);
  for (const [key, value] of Object.entries(record)) {
    if (key !== 'metrics') found.push(...inlineEstimates(value, `${path}.${key}`));
  }
  return found;
}

test.describe('showcase contracts (S10 §5)', () => {
  test('every schema uses only the subset validate.py implements', () => {
    for (const name of SCHEMA_NAMES) expect(() => loadSchema(name), name).not.toThrow();
  });

  test('the TypeScript contracts name the same schemas and claim states', () => {
    for (const view of R1_VIEWS) {
      const properties = loadSchema(view).properties as Record<string, JsonSchema>;
      expect(properties.schema.const).toBe(VIEW_SCHEMAS[view]);
    }
    const claim = (loadSchema('p1-head-to-head').properties as Record<string, JsonSchema>).claim;
    const state = (claim.properties as Record<string, JsonSchema>).state;
    expect(state.enum).toEqual(CLAIM_STATES);
  });

  test('the fixture index validates and lists every bundle', () => {
    const index = bundleIndex();
    expect(schemaErrors(index, loadSchema('manifest'))).toEqual([]);
    const ids = index.bundles.map((bundle) => bundle.id);
    expect(ids).toEqual(expect.arrayContaining([
      ...VALID_BUNDLES,
      ...Object.values(POISONED_BUNDLES),
    ]));
    expect(index.featured_bundle).toBe('fx-golden');
    expect(index.live_enabled).toBe(false);
  });

  for (const bundleId of VALID_BUNDLES) {
    test(`${bundleId}: manifest and views validate, digests match`, () => {
      expect(bundlePaths(bundleId)).toEqual([]);
      expect(digestMismatches(bundleId)).toEqual([]);
      // Fixtures are never real results, so production rules refuse them.
      expect(productionPaths(bundleId)).toEqual(['$.kind', '$.provenance.kind']);
    });

    test(`${bundleId}: every number is a metric record of the bundle`, () => {
      const rows = metricRows(bundleId);
      for (const viewId of bundleManifest(bundleId).views) {
        const view: ShowcaseView = storedView(bundleId, viewId);
        const byRef = new Map(view.metrics.map((metric) => [metric.metric_ref, metric]));
        for (const metric of view.metrics) {
          const row = rows.get(metric.metric_ref);
          expect(row, `${viewId} ${metric.metric_ref}`).toBeDefined();
          expect(row?.metric).toBe(metric.metric);
          expect(row?.value).toBe(metric.value);
          expect(row?.ci ?? null).toEqual(metric.ci);
          expect(row?.n).toBe(metric.n);
        }
        for (const [path, estimate] of inlineEstimates(view)) {
          const metric = byRef.get(estimate.metric_ref as string);
          expect(metric, `${viewId} ${path}`).toBeDefined();
          expect(estimate.value, `${viewId} ${path}`).toBe(metric?.value);
          expect(estimate.ci, `${viewId} ${path}`).toEqual(metric?.ci);
        }
        for (const metric of view.metrics) {
          if (metric.ci !== null) expect(metric.ci).toHaveLength(2);
        }
      }
    });
  }

  test('the negatives bundle holds the three negative results (SC4)', () => {
    const kinds = storedView('fx-negatives', 'overview').negatives.map((n) => n.kind);
    expect(kinds).toEqual(['frontier_wins', 'loop_dormant', 'fgr_above_bound']);
    const envelope = storedView('fx-negatives', 'p1-head-to-head').envelope;
    expect(envelope.map((row) => row.verdict)).toContain('frontier_wins');
    const audits = storedView('fx-negatives', 'm4-audits');
    expect(audits.false_green?.value).toBeGreaterThan(audits.false_green?.bound ?? 1);
  });

  test('a simulated bundle fails on simulated', () => {
    expect(bundlePaths(POISONED_BUNDLES.simulated)).toEqual([
      '$.provenance.simulated',
      '$.simulated',
    ]);
    expect(digestMismatches(POISONED_BUNDLES.simulated)).toEqual([]);
  });

  test('a bundle without run ids fails on run_ids', () => {
    expect(bundlePaths(POISONED_BUNDLES.no_run_ids)).toEqual([
      '$.provenance.run_ids',
      '$.run_ids',
    ]);
    expect(digestMismatches(POISONED_BUNDLES.no_run_ids)).toEqual([]);
  });

  test('a tampered bundle fails on the digest of the edited view', () => {
    expect(bundlePaths(POISONED_BUNDLES.integrity)).toEqual([]);
    expect(digestMismatches(POISONED_BUNDLES.integrity)).toEqual(['views/p1-head-to-head.json']);
  });

  test('a fixture-kind bundle fails the production rules on kind', () => {
    expect(bundlePaths(POISONED_BUNDLES.fixture)).toEqual([]);
    expect(digestMismatches(POISONED_BUNDLES.fixture)).toEqual([]);
    expect(productionPaths(POISONED_BUNDLES.fixture)).toEqual(['$.kind', '$.provenance.kind']);
  });

  test('showcase-event/1 accepts a metric update and refuses a bad one', () => {
    const schema = loadSchema('event');
    const event = {
      schema: 'showcase-event/1',
      seq: 42,
      t_ms: 183000,
      source: 'replay',
      run_id: 'run-fx-golden-01',
      attempt_key: null,
      type: 'metric.update',
      payload: { metric_ref: 'm-0123456789abcdef', value: 0.5, ci: [0.4, 0.6], n: 10 },
    };
    expect(schemaErrors(event, schema)).toEqual([]);
    expect(errorPaths(schemaErrors({ ...event, type: 'metric.guess', seq: 1.5 }, schema)))
      .toEqual(['$.seq', '$.type']);
  });
});
