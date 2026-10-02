/**
 * The showcase store (S10 §4.1, principle 5): the manifest and the loaded read models, keyed
 * by bundle and view. Replay and live share one renderer because both feed this store through
 * `showcase-event/1` events; a `metric.update` rewrites the estimates with its `metric_ref`.
 */
import { useEffect } from 'react';
import { create } from 'zustand';
import { ShowcaseLoadError, defaultSource, type ShowcaseSource } from './api';
import type {
  MetricUpdatePayload,
  ShowcaseEvent,
  ShowcaseManifest,
  ShowcaseView,
  ViewId,
  ViewTypes,
} from './contracts';

/**
 * How a load ended. `missing` is "this bundle has no such view" (shown as not yet measured);
 * `rejected` is the server's `409 bundle_rejected` (shown as refused).
 */
export type LoadFailure =
  | { status: 'missing'; message: string }
  | { status: 'rejected'; message: string }
  | { status: 'error'; message: string };

export type LoadState<T> = { status: 'loading' } | { status: 'ready'; data: T } | LoadFailure;

export function viewKey(bundleId: string, view: ViewId): string {
  return `${bundleId}/${view}`;
}

interface ShowcaseState {
  manifest: LoadState<ShowcaseManifest> | null;
  views: Record<string, LoadState<ShowcaseView>>;
  setManifest: (manifest: LoadState<ShowcaseManifest>) => void;
  setView: (bundleId: string, view: ViewId, state: LoadState<ShowcaseView>) => void;
  applyEvent: (bundleId: string, event: ShowcaseEvent) => void;
  reset: () => void;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseMetricUpdate(payload: unknown): MetricUpdatePayload | null {
  if (!isRecord(payload) || typeof payload.metric_ref !== 'string') return null;
  const value = payload.value;
  const ci = payload.ci;
  if (value !== null && typeof value !== 'number') return null;
  if (ci !== null && !(Array.isArray(ci) && ci.every((c) => typeof c === 'number'))) return null;
  if (typeof payload.n !== 'number') return null;
  return {
    metric_ref: payload.metric_ref as string,
    value: value as number | null,
    ci: ci as number[] | null,
    n: payload.n as number,
  };
}

function rewrite(node: unknown, update: MetricUpdatePayload): unknown {
  if (Array.isArray(node)) return node.map((item) => rewrite(item, update));
  if (!isRecord(node)) return node;
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(node)) {
    out[key] = key === 'provenance' ? value : rewrite(value, update);
  }
  if (node.metric_ref === update.metric_ref) {
    out.value = update.value;
    out.ci = update.ci;
    if ('n' in node) out.n = update.n;
  }
  return out;
}

/** `view` with every estimate of `update.metric_ref` (inline and indexed) set to the update. */
export function applyMetricUpdate<V extends ShowcaseView>(view: V, update: MetricUpdatePayload): V {
  return rewrite(view, update) as V;
}

/** The views after `event`; only `metric.update` changes them, and only in its bundle. */
export function reduceEvent(
  views: Record<string, LoadState<ShowcaseView>>,
  bundleId: string,
  event: ShowcaseEvent,
): Record<string, LoadState<ShowcaseView>> {
  if (event.type !== 'metric.update') return views;
  const update = parseMetricUpdate(event.payload);
  if (!update) return views;
  const out = { ...views };
  for (const [key, state] of Object.entries(views)) {
    if (key.startsWith(`${bundleId}/`) && state.status === 'ready') {
      out[key] = { status: 'ready', data: applyMetricUpdate(state.data, update) };
    }
  }
  return out;
}

export const useShowcaseStore = create<ShowcaseState>()((set) => ({
  manifest: null,
  views: {},
  setManifest: (manifest) => set({ manifest }),
  setView: (bundleId, view, state) => set((s) => ({
    views: { ...s.views, [viewKey(bundleId, view)]: state },
  })),
  applyEvent: (bundleId, event) => set((s) => ({ views: reduceEvent(s.views, bundleId, event) })),
  reset: () => set({ manifest: null, views: {} }),
}));

function failure(err: unknown): LoadFailure {
  const message = err instanceof Error ? err.message : String(err);
  if (err instanceof ShowcaseLoadError && err.kind === 'not_found') {
    return { status: 'missing', message };
  }
  if (err instanceof ShowcaseLoadError && err.kind === 'rejected') {
    return { status: 'rejected', message };
  }
  return { status: 'error', message };
}

const pending = new Set<string>();

/** Load the manifest once into the store. */
export function loadManifest(source: ShowcaseSource = defaultSource()): void {
  const store = useShowcaseStore.getState();
  if (store.manifest || pending.has('manifest')) return;
  pending.add('manifest');
  store.setManifest({ status: 'loading' });
  source.manifest()
    .then(
      (data) => useShowcaseStore.getState().setManifest({ status: 'ready', data }),
      (err: unknown) => useShowcaseStore.getState().setManifest(failure(err)),
    )
    .finally(() => pending.delete('manifest'));
}

/** Load one view of one bundle once into the store. */
export function loadView(
  bundleId: string,
  view: ViewId,
  source: ShowcaseSource = defaultSource(),
): void {
  const key = viewKey(bundleId, view);
  const store = useShowcaseStore.getState();
  if (store.views[key] || pending.has(key)) return;
  pending.add(key);
  store.setView(bundleId, view, { status: 'loading' });
  source.view(bundleId, view)
    .then(
      (data) => useShowcaseStore.getState().setView(bundleId, view, { status: 'ready', data }),
      (err: unknown) => useShowcaseStore.getState().setView(bundleId, view, failure(err)),
    )
    .finally(() => pending.delete(key));
}

/** The manifest, loading it on first use. */
export function useShowcaseManifest(): LoadState<ShowcaseManifest> {
  const manifest = useShowcaseStore((s) => s.manifest);
  useEffect(() => {
    loadManifest();
  }, []);
  return manifest ?? { status: 'loading' };
}

/** One view of one bundle, loading it on first use; `loading` until `bundleId` is known. */
export function useShowcaseView<V extends ViewId>(
  bundleId: string | null,
  view: V,
): LoadState<ViewTypes[V]> {
  const state = useShowcaseStore((s) => (bundleId ? s.views[viewKey(bundleId, view)] : undefined));
  useEffect(() => {
    if (bundleId) loadView(bundleId, view);
  }, [bundleId, view]);
  return (state ?? { status: 'loading' }) as LoadState<ViewTypes[V]>;
}
