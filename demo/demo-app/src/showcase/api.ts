/**
 * The showcase data client (S10 §4.8). One typed interface over two sources, chosen at build
 * time by `VITE_SHOWCASE_SOURCE`:
 *
 * - `static` (MS1, the default): reads `${BASE_URL}bundles/index.json`, then a bundle's
 *   `bundle.json`, `SHA256SUMS` and `views/<view>.json`, hashes every fetched file with
 *   WebCrypto, and marks a source `sha256_verified` only on a `SHA256SUMS` match, so the render
 *   guard works unchanged. No server is needed (`vite preview`).
 * - `api` (MS2): the `/api/showcase/*` routes, whose bundle loader verifies on the server.
 *
 * A checksum is not a statistic: the browser still computes none.
 */
import { api as defaultApi, type RokoApi } from '../transport/api';
import {
  MANIFEST_SCHEMA,
  type BundleManifest,
  type Provenance,
  type ShowcaseManifest,
  type ViewId,
  type ViewTypes,
} from './contracts';

export type SourceKind = 'static' | 'api';

export type LoadErrorKind = 'not_found' | 'rejected' | 'http' | 'parse' | 'network';

/** Why a bundle file or view could not be loaded. `not_found` means "not in this bundle". */
export class ShowcaseLoadError extends Error {
  readonly kind: LoadErrorKind;
  readonly status: number;

  constructor(kind: LoadErrorKind, status: number, message: string) {
    super(message);
    this.name = 'ShowcaseLoadError';
    this.kind = kind;
    this.status = status;
  }
}

export interface ShowcaseSource {
  readonly kind: SourceKind;
  manifest(): Promise<ShowcaseManifest>;
  bundle(bundleId: string): Promise<BundleManifest>;
  view<V extends ViewId>(bundleId: string, view: V): Promise<ViewTypes[V]>;
  /** The URL of a raw bundle file, for download links. */
  fileHref(bundleId: string, path: string): string;
}

function viteEnv(name: string): string | undefined {
  const env = (import.meta as { env?: Record<string, unknown> }).env;
  const value = env?.[name];
  return typeof value === 'string' ? value : undefined;
}

/** The source this build reads: `api` only when `VITE_SHOWCASE_SOURCE=api`. */
export function sourceKind(): SourceKind {
  return viteEnv('VITE_SHOWCASE_SOURCE') === 'api' ? 'api' : 'static';
}

const SAFE_ID = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const SAFE_PATH = /^[A-Za-z0-9_][A-Za-z0-9._-]*(\/[A-Za-z0-9_][A-Za-z0-9._-]*)*$/;

function checkBundleId(bundleId: string): void {
  if (!SAFE_ID.test(bundleId) || bundleId.includes('..')) {
    throw new ShowcaseLoadError('not_found', 0, `not a bundle id: ${bundleId}`);
  }
}

/** Lowercase hex SHA-256 of `data`, with WebCrypto (a secure context in the browser). */
export async function sha256Hex(data: ArrayBuffer): Promise<string> {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) throw new ShowcaseLoadError('network', 0, 'WebCrypto is unavailable here');
  const digest = new Uint8Array(await subtle.digest('SHA-256', data));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** `SHA256SUMS` (`sha256sum` format, `<hex>  <path>`) as path → digest. */
export function parseSha256Sums(text: string): Map<string, string> {
  const sums = new Map<string, string>();
  for (const line of text.split('\n')) {
    const match = /^([0-9a-f]{64}) [ *](.+)$/.exec(line.trim());
    if (match) sums.set(match[2], match[1]);
  }
  return sums;
}

/**
 * Set every source's `sha256_verified` from what the loader saw, never from the file: true only
 * when the view itself matched `SHA256SUMS` and the source file's digest equals both its
 * `SHA256SUMS` entry and the digest the provenance records.
 */
export function markVerified<V extends { provenance: Provenance }>(
  view: V,
  viewVerified: boolean,
  digests: Map<string, string | null>,
  sums: Map<string, string>,
): V {
  const sources = Array.isArray(view.provenance?.sources) ? view.provenance.sources : [];
  for (const source of sources) {
    const digest = digests.get(source.path) ?? null;
    source.sha256_verified = viewVerified
      && digest !== null
      && sums.get(source.path) === digest
      && source.sha256 === digest;
  }
  return view;
}

export interface StaticSourceOptions {
  /** Where the bundles are, ending in `/`; default `${BASE_URL}bundles/`. */
  base?: string;
  fetch?: typeof fetch;
}

interface Fetched {
  bytes: ArrayBuffer;
  digest: string;
}

/** The static source of R1-static (S10 §4.8). */
export function createStaticSource(options: StaticSourceOptions = {}): ShowcaseSource {
  const base = options.base ?? `${viteEnv('BASE_URL') ?? '/'}bundles/`;
  const fetchImpl = options.fetch ?? ((input: RequestInfo | URL, init?: RequestInit) => fetch(input, init));
  const files = new Map<string, Promise<Fetched>>();
  const decoder = new TextDecoder();

  async function download(path: string): Promise<Fetched> {
    let response: Response;
    try {
      response = await fetchImpl(base + path);
    } catch (err) {
      throw new ShowcaseLoadError('network', 0, `${path}: ${String(err)}`);
    }
    if (response.status === 404) throw new ShowcaseLoadError('not_found', 404, `${path}: not found`);
    if (!response.ok) {
      throw new ShowcaseLoadError('http', response.status, `${path}: HTTP ${response.status}`);
    }
    const bytes = await response.arrayBuffer();
    return { bytes, digest: await sha256Hex(bytes) };
  }

  function file(path: string): Promise<Fetched> {
    let pending = files.get(path);
    if (!pending) {
      pending = download(path);
      files.set(path, pending);
      pending.catch(() => files.delete(path));
    }
    return pending;
  }

  function json<T>(fetched: Fetched, path: string): T {
    try {
      return JSON.parse(decoder.decode(fetched.bytes)) as T;
    } catch {
      throw new ShowcaseLoadError('parse', 0, `${path}: not JSON`);
    }
  }

  async function sums(bundleId: string): Promise<Map<string, string>> {
    const fetched = await file(`${bundleId}/SHA256SUMS`);
    return parseSha256Sums(decoder.decode(fetched.bytes));
  }

  return {
    kind: 'static',
    async manifest() {
      const manifest = json<ShowcaseManifest>(await file('index.json'), 'index.json');
      if (manifest?.schema !== MANIFEST_SCHEMA) {
        throw new ShowcaseLoadError('parse', 0, 'index.json: not a showcase-manifest/1');
      }
      return manifest;
    },
    async bundle(bundleId) {
      checkBundleId(bundleId);
      const path = `${bundleId}/bundle.json`;
      return json<BundleManifest>(await file(path), path);
    },
    async view<V extends ViewId>(bundleId: string, view: V): Promise<ViewTypes[V]> {
      checkBundleId(bundleId);
      const path = `${bundleId}/views/${view}.json`;
      const fetched = await file(path);
      const parsed = json<ViewTypes[V]>(fetched, path);
      const checksums = await sums(bundleId).catch(() => new Map<string, string>());
      const manifest = await file(`${bundleId}/bundle.json`).catch(() => null);
      const viewVerified = checksums.get(`views/${view}.json`) === fetched.digest
        && manifest !== null
        && checksums.get('bundle.json') === manifest.digest;
      const digests = new Map<string, string | null>();
      const sources = Array.isArray(parsed?.provenance?.sources) ? parsed.provenance.sources : [];
      for (const source of sources) {
        const safe = typeof source.path === 'string' && SAFE_PATH.test(source.path);
        const digest = safe
          ? await file(`${bundleId}/${source.path}`).then((f) => f.digest, () => null)
          : null;
        digests.set(String(source.path), digest);
      }
      return markVerified(parsed, viewVerified, digests, checksums);
    },
    fileHref(bundleId, path) {
      return `${base}${bundleId}/${path}`;
    },
  };
}

/** The serve routes of each R1 view (S10 §5.2). */
export const VIEW_ROUTES: Readonly<Record<ViewId, string>> = {
  overview: '/api/showcase/overview',
  'p1-head-to-head': '/api/showcase/p1/head-to-head',
  'm4-audits': '/api/showcase/m4/audits',
};

/**
 * The api source of R1-serve (S10 §5.2): the server's loader verifies bundles and answers
 * `409 bundle_rejected` for a bad one, which surfaces as a `rejected` load error.
 */
export function createApiSource(client: RokoApi = defaultApi): ShowcaseSource {
  async function get<T>(path: string): Promise<T> {
    const result = await client.get<T>(path);
    if (result.ok) return result.data;
    const { status, statusText, body } = result.error;
    const message = `${path}: ${body ?? statusText}`;
    if (status === 409) throw new ShowcaseLoadError('rejected', status, message);
    if (status === 404) throw new ShowcaseLoadError('not_found', status, message);
    throw new ShowcaseLoadError(status === 0 ? 'network' : 'http', status, message);
  }
  const bundlePath = (bundleId: string) => `/api/showcase/bundles/${encodeURIComponent(bundleId)}`;
  return {
    kind: 'api',
    manifest: () => get<ShowcaseManifest>('/api/showcase/manifest'),
    bundle: (bundleId) => get<BundleManifest>(bundlePath(bundleId)),
    view: <V extends ViewId>(bundleId: string, view: V) => get<ViewTypes[V]>(
      `${VIEW_ROUTES[view]}?source=${encodeURIComponent(`bundle:${bundleId}`)}`,
    ),
    fileHref: (bundleId, path) => `${client.baseUrl}${bundlePath(bundleId)}/files/${path}`,
  };
}

let shared: ShowcaseSource | null = null;

/** The source of this build, created once. */
export function defaultSource(): ShowcaseSource {
  shared ??= sourceKind() === 'api' ? createApiSource() : createStaticSource();
  return shared;
}
