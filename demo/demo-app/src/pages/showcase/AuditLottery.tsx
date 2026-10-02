import BandLineChart from '../../components/Charts/BandLineChart';
import ChartTable, { MetricValue } from '../../components/Charts/ChartTable';
import NotMeasured from '../../components/NotMeasured';
import { PLANNED_IN, type AuditCheckKind, type M4AuditsView } from '../../showcase/contracts';
import {
  Badges,
  ClaimBanner,
  Count,
  Drawer,
  GuardedView,
  useBundleChoice,
  type BundleChoice,
} from './ViewFrame';

/** S05 §4.3 battery ids, as the page names them. */
const CHECKS: Record<AuditCheckKind, string> = {
  A1_tamper: 'A1 tamper',
  A2_rerun: 'A2 clean re-run',
  B1_hidden: 'B1 hidden tests',
  B2_mutation: 'B2 mutation',
  B3_review: 'B3 strong review',
};

/** Labels for the estimator tokens a bundle records; an unknown token shows as written. */
const ESTIMATORS: Record<string, string> = {
  hajek: 'Hájek (ratio HT)',
  ht_wilson_eff_n: 'Wilson on n_eff',
};

function label(token: string): string {
  return ESTIMATORS[token] ?? token;
}

function ByCheck({ view }: { view: M4AuditsView }) {
  const c = view.by_check;
  const rows = [
    [CHECKS.A1_tamper, <Count path="by_check.A1_tamper.run" value={c.A1_tamper.run} />,
      <span>failed <Count path="by_check.A1_tamper.failed" value={c.A1_tamper.failed} /></span>],
    [CHECKS.A2_rerun, <Count path="by_check.A2_rerun.run" value={c.A2_rerun.run} />,
      <span>failed <Count path="by_check.A2_rerun.failed" value={c.A2_rerun.failed} /></span>],
    [CHECKS.B1_hidden, <Count path="by_check.B1_hidden.run" value={c.B1_hidden.run} />,
      <span>failed <Count path="by_check.B1_hidden.failed" value={c.B1_hidden.failed} /></span>],
    [CHECKS.B2_mutation, <Count path="by_check.B2_mutation.run" value={c.B2_mutation.run} />,
      <span>mean score <MetricValue estimate={c.B2_mutation.mean_score} kind="score" /></span>],
    [CHECKS.B3_review, <Count path="by_check.B3_review.run" value={c.B3_review.run} />,
      <span>agree with B1 <Count path="by_check.B3_review.agree_with_B1" value={c.B3_review.agree_with_B1} /></span>],
  ];
  return (
    <ChartTable
      chart="by-check"
      title="Checks run"
      columns={['check', 'run', 'result']}
      rows={rows}
      provenance={<Drawer provenance={view.provenance} subject="checks" metrics={view.metrics} />}
    />
  );
}

function Draws({ view }: { view: M4AuditsView }) {
  return (
    <section className="sc-panel" data-panel="draws">
      <h2 className="sc-section-title">Recent draws</h2>
      <ul className="sc-draws">
        {view.draws.map((draw, i) => (
          <li key={draw.audit_id} data-audit={draw.audit_id} data-outcome={draw.outcome}>
            <code>{draw.audit_id}</code> · {'π'} <Count path={`draws.${i}.pi`} value={draw.pi} /> ·{' '}
            {draw.checks.map((check) => (
              <span key={check.kind} className={check.passed ? 'sc-ok' : 'sc-bad'}>
                {CHECKS[check.kind]} {check.passed ? '✓' : '✗'}{' '}
              </span>
            ))}
            · {draw.outcome}
            {draw.feedback_actions.map((action) => (
              <span key={action.target} className="sc-dim">
                {' '}{'→'} {action.target} {String(action.from)} {'→'} {String(action.to)}
              </span>
            ))}
          </li>
        ))}
      </ul>
    </section>
  );
}

/** The M4 audit lottery (S10 §4.3 H): FGR with its CI and estimator, checks, draws, series. */
export function AuditsBody({ view, choice }: { view: M4AuditsView; choice: BundleChoice }) {
  const live = choice.manifest.status === 'ready' && choice.manifest.data.live_enabled;
  const fgr = view.false_green;
  if (view.audited === 0 || !fgr) {
    return (
      <>
        <Badges choice={choice} provenance={view.provenance} />
        <ClaimBanner claim={view.claim} />
        <NotMeasured what="M4 audit lottery" plannedIn={[PLANNED_IN['m4-audits']]} />
      </>
    );
  }
  return (
    <>
      <Badges choice={choice} provenance={view.provenance} />
      <ClaimBanner claim={view.claim} />
      <section className="sc-panel" data-panel="fgr">
        <header className="sc-panel__head">
          <h2 className="sc-section-title">Random deep audits</h2>
          <Drawer provenance={view.provenance} subject="false-green rate" metrics={view.metrics} />
        </header>
        <p className="sc-counts">
          visible passes <Count path="visible_passes" value={view.visible_passes} /> · {'π'}{' '}
          base <Count path="policy.pi_base" value={view.policy.pi_base} /> · audited{' '}
          <Count path="audited" value={view.audited} /> · caught <Count path="caught" value={view.caught} />
        </p>
        <p className="sc-fgr">
          <span className="sc-fgr__label">false-green</span>{' '}
          <MetricValue estimate={fgr} kind="rate" />{' '}
          <span className="sc-dim" data-estimator={fgr.estimator}>
            {label(fgr.estimator)}, {label(fgr.ci_method)}; n_eff{' '}
            <Count path="false_green.n_eff" value={fgr.n_eff} />
            {fgr.bound !== null && (
              <>; S06 bound <Count path="false_green.bound" value={fgr.bound} /></>
            )}
          </span>
        </p>
        <p className="sc-counts">
          gaming <MetricValue estimate={view.gaming_rate} kind="rate" /> · detector recall{' '}
          <MetricValue estimate={view.detector.recall} kind="rate" /> / precision{' '}
          <MetricValue estimate={view.detector.precision} kind="rate" /> on{' '}
          <Count path="detector.honeypots" value={view.detector.honeypots} /> honeypots
        </p>
      </section>
      <ByCheck view={view} />
      <Draws view={view} />
      <BandLineChart
        chart="fgr-series"
        title="False-green rate over time"
        kind="rate"
        yLabel="false-green rate"
        series={[{ id: 'false_green', label: 'false-green', points: view.series }]}
        provenance={<Drawer provenance={view.provenance} subject="false-green series" metrics={view.metrics} />}
      />
      <section className="sc-panel" data-panel="draw-now">
        <button
          type="button"
          className="sc-action"
          disabled={!live}
          title={live ? 'Draw audits now (CPU-only, $0 LLM)' : 'replay-only: live draws are off here'}
          data-replay-only={String(!live)}
        >
          Draw now
        </button>
        {!live && <span className="sc-dim"> replay-only</span>}
      </section>
    </>
  );
}

/** `/demo/p2/audits`: the M4 audit lottery. */
export default function AuditLottery() {
  const choice = useBundleChoice();
  return (
    <section className="sc-page" data-showcase-page="audits">
      <header className="sc-hero">
        <h1 className="sc-hero__title">Random deep audits</h1>
        <p className="sc-hero__sub">How often a visible pass hides a failure, estimated from audits.</p>
      </header>
      <GuardedView choice={choice} view="m4-audits" subject="M4 audit lottery">
        {(view) => <AuditsBody view={view} choice={choice} />}
      </GuardedView>
    </section>
  );
}
