/**
 * What every showcase page shares (S10 §4.1): the bundle a page shows (`?bundle=<id>`, else the
 * featured one), a loader that runs the render guard before anything draws, the claim banner,
 * and the REPLAY and LIVE badges.
 */
import type { ReactNode } from 'react';
import { useSearchParams } from 'react-router';
import NotMeasured from '../../components/NotMeasured';
import ProvenanceDrawer from '../../components/ProvenanceDrawer';
import RefusedPanel from '../../components/RefusedPanel';
import { defaultSource } from '../../showcase/api';
import {
  PLANNED_IN,
  type BundleSummary,
  type Claim,
  type Provenance,
  type ShowcaseManifest,
  type ViewId,
  type ViewMetric,
  type ViewTypes,
} from '../../showcase/contracts';
import { refusalOf } from '../../showcase/guard';
import { useShowcaseManifest, useShowcaseView, type LoadState } from '../../showcase/store';
import '../../showcase/showcase.css';

export interface BundleChoice {
  bundleId: string | null;
  manifest: LoadState<ShowcaseManifest>;
  summary: BundleSummary | null;
  /** `?bundle=<id>` when the bundle was picked explicitly, so links keep it; else empty. */
  query: string;
}

/** The bundle to show: `explicit`, else `?bundle=<id>`, else the manifest's featured one. */
export function useBundleChoice(explicit?: string): BundleChoice {
  const manifest = useShowcaseManifest();
  const [params] = useSearchParams();
  const ready = manifest.status === 'ready' ? manifest.data : null;
  const picked = explicit ?? params.get('bundle');
  const bundleId = picked ?? ready?.featured_bundle ?? null;
  const summary = ready?.bundles.find((bundle) => bundle.id === bundleId) ?? null;
  const query = picked ? `?bundle=${encodeURIComponent(picked)}` : '';
  return { bundleId, manifest, summary, query };
}

/** Where each view lives in the showcase (S10 §4.2). */
export const VIEW_PATHS: Readonly<Record<ViewId, string>> = {
  overview: '/',
  'p1-head-to-head': '/p1/head-to-head',
  'm4-audits': '/p2/audits',
};

/** A plain number copied from the view (a count or a policy setting), with its JSON path. */
export function Count({ path, value }: { path: string; value: number | string }) {
  return <span className="sc-count" data-view-path={path}>{String(value)}</span>;
}

/** The badges above a view: what it replays, and whether live actions are on. */
export function Badges({ choice, provenance }: { choice: BundleChoice; provenance: Provenance }) {
  return (
    <div className="sc-badges">
      <ReplayBadge summary={choice.summary} provenance={provenance} />
      <LiveBadge manifest={choice.manifest} />
    </div>
  );
}

interface GuardedViewProps<V extends ViewId> {
  choice: BundleChoice;
  view: V;
  /** What the view is, for messages, e.g. "P1 head-to-head". */
  subject: string;
  children: (data: ViewTypes[V]) => ReactNode;
}

/**
 * Load one view and render it only if the guard accepts it (S10 §4.5): a refusal shows
 * `RefusedPanel` and nothing of the view; a bundle without the view shows "not yet measured".
 */
export function GuardedView<V extends ViewId>({ choice, view, subject, children }: GuardedViewProps<V>) {
  const state = useShowcaseView(choice.bundleId, view);
  if (!choice.bundleId) {
    if (choice.manifest.status === 'loading') return <Loading subject={subject} view={view} />;
    return <NotMeasured what={subject} plannedIn={[PLANNED_IN[view]]} />;
  }
  switch (state.status) {
    case 'loading':
      return <Loading subject={subject} view={view} />;
    case 'missing':
      return <NotMeasured what={subject} plannedIn={[PLANNED_IN[view]]} />;
    case 'rejected':
      return <RefusedPanel reason="integrity" subject={subject} detail={state.message} />;
    case 'error':
      return (
        <p className="sc-error" role="alert" data-load-error={view}>
          {subject} could not be loaded: {state.message}
        </p>
      );
    case 'ready': {
      const refusal = refusalOf(state.data);
      if (refusal) {
        return <RefusedPanel reason={refusal.reason} subject={subject} detail={refusal.detail} />;
      }
      return <div data-view={view}>{children(state.data)}</div>;
    }
  }
}

function Loading({ subject, view }: { subject: string; view: ViewId }) {
  return <p className="sc-loading" data-loading={view}>loading {subject}…</p>;
}

/** The ⓘ for a panel of `provenance`, with download links to the bundle's files. */
export function Drawer({
  provenance,
  subject,
  metrics,
}: {
  provenance: Provenance;
  subject: string;
  metrics?: ViewMetric[];
}) {
  return (
    <ProvenanceDrawer
      provenance={provenance}
      subject={subject}
      metrics={metrics}
      fileHref={(path) => defaultSource().fileHref(provenance.bundle_id, path)}
    />
  );
}

/** The claim state, copied verbatim from the bundle (S09's test record), and its sentence. */
export function ClaimBanner({ claim }: { claim: Claim }) {
  return (
    <div className="sc-claim" data-claim-state={claim.state} data-hypothesis={claim.hypothesis}>
      <div className="sc-claim__head">
        <span className="sc-claim__hypothesis">{claim.hypothesis}</span>
        <span className={`sc-claim__state sc-claim__state--${claim.state}`}>
          {claim.state.replace(/_/g, ' ')}
        </span>
        <span className="sc-claim__prereg">
          {claim.prereg_id ? `S09 pre-registration ${claim.prereg_id}` : 'descriptive, not pre-registered'}
        </span>
      </div>
      <p className="sc-claim__text">{claim.text}</p>
      {claim.state === 'NOT_YET_MEASURED' && <NotMeasured plannedIn={claim.planned_in} />}
    </div>
  );
}

/** `REPLAY ● <bundle> · recorded <date> · commit <sha>` (S10 §4.1, two badges only). */
export function ReplayBadge({ summary, provenance }: {
  summary: BundleSummary | null;
  provenance: Provenance;
}) {
  const recorded = (summary?.created_at ?? provenance.generated_at).slice(0, 10);
  return (
    <span className="sc-badge sc-badge--replay" data-badge="replay">
      REPLAY {'●'} {summary?.title ?? provenance.bundle_id} · recorded {recorded} · commit{' '}
      {provenance.harness_commit.slice(0, 9)}
    </span>
  );
}

/** Live actions are off in R1 (replay-only deployments). */
export function LiveBadge({ manifest }: { manifest: LoadState<ShowcaseManifest> }) {
  const live = manifest.status === 'ready' && manifest.data.live_enabled;
  return (
    <span className={`sc-badge ${live ? 'sc-badge--live' : 'sc-badge--live-off'}`} data-badge="live">
      LIVE {live ? '● on' : '○ off'}
    </span>
  );
}
