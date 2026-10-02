/**
 * Read the fixture bundles under `e2e/showcase/bundles/` and the showcase JSON Schemas from
 * disk, for specs that check contracts without a browser. Fixture bundles carry
 * `kind: "fixture"`, so a production build refuses every one of them (S10 §4.5).
 */
import { expect, type APIRequestContext } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type {
  BundleManifest,
  ShowcaseManifest,
  ViewId,
  ViewTypes,
} from '../../src/showcase/contracts';
import { checkSchema, type JsonSchema } from './schema';

export const BUNDLE_ROOT = fileURLToPath(new URL('./bundles/', import.meta.url));
export const SCHEMA_ROOT = fileURLToPath(new URL('../../src/showcase/schemas/', import.meta.url));

/** Bundles that conform: the golden one, the negative results (S10 SC4), and P1 without audits. */
export const VALID_BUNDLES = ['fx-golden', 'fx-negatives', 'fx-p1-only'] as const;

/** One poisoned bundle per refusal the client must make (S10 §7 scenario 2). */
export const POISONED_BUNDLES = {
  simulated: 'fx-poison-simulated',
  fixture: 'fx-poison-fixture-kind',
  no_run_ids: 'fx-poison-no-run-ids',
  integrity: 'fx-poison-tampered',
} as const;

export function readBundleBytes(bundleId: string, path: string): Buffer {
  return readFileSync(`${BUNDLE_ROOT}${bundleId}/${path}`);
}

export function readBundleJson<T = unknown>(bundleId: string, path: string): T {
  return JSON.parse(readBundleBytes(bundleId, path).toString('utf8')) as T;
}

/** A showcase schema by file stem (`provenance`, `overview`, ...), checked against the subset. */
export function loadSchema(name: string): JsonSchema {
  const text = readFileSync(`${SCHEMA_ROOT}${name}.schema.json`, 'utf8');
  const schema = JSON.parse(text) as JsonSchema;
  checkSchema(schema);
  return schema;
}

export function bundleIndex(): ShowcaseManifest {
  return JSON.parse(readFileSync(`${BUNDLE_ROOT}index.json`, 'utf8')) as ShowcaseManifest;
}

export function bundleManifest(bundleId: string): BundleManifest {
  return readBundleJson<BundleManifest>(bundleId, 'bundle.json');
}

export function sha256Hex(data: Buffer | string): string {
  return createHash('sha256').update(data).digest('hex');
}

/** `SHA256SUMS` as path → digest (`sha256sum` format: `<hex>  <path>`). */
export function sha256Sums(bundleId: string): Map<string, string> {
  const sums = new Map<string, string>();
  for (const line of readBundleBytes(bundleId, 'SHA256SUMS').toString('utf8').split('\n')) {
    const match = /^([0-9a-f]{64}) [ *](.+)$/.exec(line.trim());
    if (match) sums.set(match[2], match[1]);
  }
  return sums;
}

/** The files whose digest differs from their `SHA256SUMS` entry. */
export function digestMismatches(bundleId: string): string[] {
  const bad: string[] = [];
  for (const [path, digest] of sha256Sums(bundleId)) {
    if (sha256Hex(readBundleBytes(bundleId, path)) !== digest) bad.push(path);
  }
  return bad.sort();
}

/** A view exactly as stored: every source has `sha256_verified: false`. */
export function storedView<V extends ViewId>(bundleId: string, view: V): ViewTypes[V] {
  return readBundleJson<ViewTypes[V]>(bundleId, `views/${view}.json`);
}

/**
 * A view as a loader hands it to the guard: a source is verified only when the view file, the
 * source file and the source's recorded digest all match `SHA256SUMS` (S10 §4.8).
 */
export function loadedView<V extends ViewId>(bundleId: string, view: V): ViewTypes[V] {
  const sums = sha256Sums(bundleId);
  const viewPath = `views/${view}.json`;
  const viewOk = sums.get(viewPath) === sha256Hex(readBundleBytes(bundleId, viewPath));
  const loaded = storedView(bundleId, view);
  for (const source of loaded.provenance.sources) {
    const digest = sha256Hex(readBundleBytes(bundleId, source.path));
    source.sha256_verified = viewOk && sums.get(source.path) === digest && source.sha256 === digest;
  }
  return loaded;
}

/** The `metric_ref` of a `data/metrics.jsonl` line: `m-` plus 16 hex digits of its SHA-256. */
export function metricRef(line: string): string {
  return `m-${sha256Hex(line).slice(0, 16)}`;
}

/** The metric records of a bundle, by `metric_ref`. */
export function metricRows(bundleId: string): Map<string, Record<string, unknown>> {
  const rows = new Map<string, Record<string, unknown>>();
  const text = readBundleBytes(bundleId, 'data/metrics.jsonl').toString('utf8');
  for (const line of text.split('\n')) {
    if (line.trim()) rows.set(metricRef(line), JSON.parse(line) as Record<string, unknown>);
  }
  return rows;
}

/**
 * Fail fast, with the fix, when the dev server on :5173 was started without
 * `VITE_ALLOW_FIXTURES=1` (Playwright reuses a running server).
 */
export async function expectFixtureServer(request: APIRequestContext): Promise<void> {
  const response = await request.get('/bundles/index.json');
  const body = response.ok() ? await response.text() : '';
  expect(
    body.includes('"showcase-manifest/1"'),
    'the dev server does not serve the fixture bundles: stop it, or start it with '
      + 'VITE_ALLOW_FIXTURES=1 (npm run dev:fixtures)',
  ).toBe(true);
}
