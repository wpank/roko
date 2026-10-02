/**
 * Refuse-to-render (S10 §4.5): every showcase view and chart passes `assertRenderable` before
 * it draws anything, and a refusal shows `RefusedPanel` instead. Simulated, fixture (outside
 * a test build), unattributed or tampered data is refused, not labelled.
 *
 * This is the client-side layer. The others are `verify_bundle.py` at staging and, from
 * R1-serve, the server's bundle loader. The module reads `import.meta.env` defensively so a
 * Playwright spec can import it in Node.
 */
import { PROVENANCE_SCHEMA, type RefusalReason, type ViewMetric } from './contracts';

/** Thrown by `assertRenderable`; `reason` is the code `RefusedPanel` shows. */
export class RenderRefusal extends Error {
  readonly reason: RefusalReason;

  constructor(reason: RefusalReason, detail: string) {
    super(`refused (${reason}): ${detail}`);
    this.name = 'RenderRefusal';
    this.reason = reason;
  }
}

export interface Refusal {
  reason: RefusalReason;
  detail: string;
}

export interface GuardOptions {
  /** Accept `kind: "fixture"`. Defaults to `fixturesAllowed()`. */
  allowFixtures?: boolean;
}

function viteEnv(name: string): string | undefined {
  const env = (import.meta as { env?: Record<string, unknown> }).env;
  const value = env?.[name];
  return typeof value === 'string' ? value : undefined;
}

/** Fixtures render only in a non-production build with `VITE_ALLOW_FIXTURES=1`. */
export function fixturesAllowed(): boolean {
  return viteEnv('MODE') !== 'production' && viteEnv('VITE_ALLOW_FIXTURES') === '1';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Every `metric_ref` a view displays outside its `metrics` index. */
function displayedRefs(node: unknown, refs: Set<string> = new Set()): Set<string> {
  if (Array.isArray(node)) {
    for (const item of node) displayedRefs(item, refs);
  } else if (isRecord(node)) {
    if (typeof node.metric_ref === 'string') refs.add(node.metric_ref);
    for (const [key, value] of Object.entries(node)) {
      if (key !== 'metrics' && key !== 'provenance') displayedRefs(value, refs);
    }
  }
  return refs;
}

/** Why `view` must not render, or `null` when it may. */
export function refusalOf(view: unknown, options: GuardOptions = {}): Refusal | null {
  const p = isRecord(view) ? view.provenance : undefined;
  if (!isRecord(p) || p.schema !== PROVENANCE_SCHEMA) {
    return { reason: 'missing_provenance', detail: 'the view carries no provenance envelope' };
  }
  const allowFixtures = options.allowFixtures ?? fixturesAllowed();
  const kind = p.kind;
  if (kind !== 'measured' && kind !== 'replay' && !(allowFixtures && kind === 'fixture')) {
    if (kind === 'fixture') {
      return { reason: 'fixture', detail: 'fixture data renders only in a test build' };
    }
    if (kind === 'simulated') {
      return { reason: 'simulated', detail: 'the view is marked simulated' };
    }
    return { reason: 'missing_provenance', detail: `unknown provenance kind ${String(kind)}` };
  }
  const sources = Array.isArray(p.sources) ? p.sources : [];
  if (p.simulated !== false || sources.some((s) => !isRecord(s) || s.simulated !== false)) {
    return { reason: 'simulated', detail: 'the view or one of its sources is simulated' };
  }
  if (!Array.isArray(p.run_ids) || p.run_ids.length === 0) {
    return { reason: 'no_run_ids', detail: 'no run ids back these numbers' };
  }
  if (sources.length === 0) {
    return { reason: 'missing_provenance', detail: 'the provenance lists no source files' };
  }
  const unverified = sources.find((s) => isRecord(s) && s.sha256_verified !== true);
  if (unverified !== undefined) {
    const path = isRecord(unverified) ? String(unverified.path) : '?';
    return { reason: 'integrity', detail: `${path} does not match the bundle's SHA256SUMS` };
  }
  const metrics = (isRecord(view) && Array.isArray(view.metrics) ? view.metrics : []) as unknown[];
  const indexed = new Set<string>();
  for (const entry of metrics) {
    const m = (isRecord(entry) ? entry : {}) as Partial<ViewMetric>;
    const ref = String(m.metric_ref);
    if (!(typeof m.n === 'number' && m.n >= 1)) {
      return { reason: 'n_missing', detail: `metric ${ref} has no sample size` };
    }
    if (m.kind === 'rate' && !m.ci && m.ci_method !== 'none') {
      return { reason: 'ci_missing', detail: `rate ${ref} has no confidence interval` };
    }
    indexed.add(ref);
  }
  for (const ref of displayedRefs(view)) {
    if (!indexed.has(ref)) {
      return { reason: 'n_missing', detail: `number ${ref} is not among the view's metrics` };
    }
  }
  return null;
}

/** Throw `RenderRefusal` unless `view` may render (S10 §4.5). */
export function assertRenderable(view: unknown, options: GuardOptions = {}): void {
  const refusal = refusalOf(view, options);
  if (refusal) throw new RenderRefusal(refusal.reason, refusal.detail);
}
