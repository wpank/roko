import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router';
import NotMeasured from '../../components/NotMeasured';
import { defaultSource } from '../../showcase/api';
import type { BundleSummary } from '../../showcase/contracts';
import { useShowcaseManifest } from '../../showcase/store';
import { AuditsBody } from './AuditLottery';
import { HeadToHeadBody } from './HeadToHead';
import { OverviewBody } from './Overview';
import { GuardedView, useBundleChoice } from './ViewFrame';

/** Featured first, then the newest recording (S10 §4.7). */
function ordered(bundles: BundleSummary[]): BundleSummary[] {
  return [...bundles].sort((a, b) => (
    Number(b.featured) - Number(a.featured) || b.created_at.localeCompare(a.created_at)
  ));
}

/** `/demo/replays`: the recorded bundles this deployment can replay. */
export default function Replays() {
  const manifest = useShowcaseManifest();
  return (
    <section className="sc-page" data-showcase-page="replays">
      <header className="sc-hero">
        <h1 className="sc-hero__title">Replays</h1>
        <p className="sc-hero__sub">Recorded bundles: every number comes from one of these.</p>
      </header>
      {manifest.status === 'loading' && <p className="sc-loading">loading the bundle index…</p>}
      {(manifest.status === 'missing' || manifest.status === 'rejected') && (
        <NotMeasured what="Replays" plannedIn={['PILOT']} />
      )}
      {manifest.status === 'error' && (
        <p className="sc-error" role="alert">The bundle index could not be loaded: {manifest.message}</p>
      )}
      {manifest.status === 'ready' && (
        <ol className="sc-replays">
          {ordered(manifest.data.bundles).map((bundle) => (
            <li key={bundle.id} data-bundle={bundle.id} data-featured={String(bundle.featured)}>
              <Link to={`/replay/${encodeURIComponent(bundle.id)}`}>{bundle.title}</Link>
              <span className="sc-dim">
                {' '}<code>{bundle.id}</code> · recorded {bundle.created_at.slice(0, 10)} · {bundle.status}
                {bundle.featured && ' · featured'}
              </span>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

/**
 * `/demo/replay/:bundleId`: the bundle's static views under the REPLAY watermark. The timeline
 * (`ReplayClock`, scrubber, `?t=`) is R2.
 */
export function ReplayView() {
  const { bundleId = '' } = useParams();
  const choice = useBundleChoice(bundleId);
  const [commit, setCommit] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    defaultSource().bundle(bundleId).then(
      (bundle) => {
        if (live) setCommit(bundle.harness_commit);
      },
      () => {
        if (live) setCommit(null);
      },
    );
    return () => {
      live = false;
    };
  }, [bundleId]);

  const recorded = choice.summary?.created_at.slice(0, 10) ?? 'unknown date';
  return (
    <section className="sc-page sc-page--replay" data-showcase-page="replay" data-bundle={bundleId}>
      <div className="sc-watermark" data-watermark="replay">
        REPLAY of {choice.summary?.title ?? bundleId} · recorded {recorded}
        {commit && ` · commit ${commit.slice(0, 9)}`}
      </div>
      <GuardedView choice={choice} view="overview" subject="Overview">
        {(view) => <OverviewBody view={view} choice={choice} />}
      </GuardedView>
      <GuardedView choice={choice} view="p1-head-to-head" subject="P1 head-to-head">
        {(view) => <HeadToHeadBody view={view} choice={choice} />}
      </GuardedView>
      <GuardedView choice={choice} view="m4-audits" subject="M4 audit lottery">
        {(view) => <AuditsBody view={view} choice={choice} />}
      </GuardedView>
    </section>
  );
}
