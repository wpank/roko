import type { ReactNode } from 'react';
import ChartTable, { MetricValue } from '../../components/Charts/ChartTable';
import EnvelopeTable from '../../components/Charts/EnvelopeTable';
import ParetoFrontierChart from '../../components/Charts/ParetoFrontierChart';
import PassKChart from '../../components/Charts/PassKChart';
import type { HeadToHeadView } from '../../showcase/contracts';
import {
  Badges,
  ClaimBanner,
  Drawer,
  GuardedView,
  useBundleChoice,
  type BundleChoice,
} from './ViewFrame';

const CAVEAT = 'API-equivalent reported by the CLI (subscription), not billed spend';

function notRun() {
  return <span className="sc-dim" data-arm-status="not_run">not run</span>;
}

/** Every arm, the probe and extra rows included; an arm not run says so instead of a blank. */
function ArmsTable({ view }: { view: HeadToHeadView }) {
  const rows = view.arms.map((arm) => {
    const run = arm.status === 'run';
    const cell = (node: ReactNode) => (run ? node : notRun());
    return [
      <span key="arm" data-arm={arm.arm} data-arm-role={arm.role}>
        {arm.label}
        {arm.note && <span className="sc-dim"> · {arm.note}</span>}
        {!run && arm.reason && <span className="sc-arm-reason">{arm.reason}</span>}
      </span>,
      cell(<MetricValue estimate={arm.resolve} kind="rate" />),
      cell(<MetricValue estimate={arm.usd_per_verified} kind="usd" />),
      cell(<MetricValue estimate={arm.pass_hat_k?.['1'] ?? null} kind="rate" showCi={false} />),
      cell(<MetricValue estimate={arm.pass_hat_k?.['3'] ?? null} kind="rate" showCi={false} />),
      cell(<MetricValue estimate={arm.pass_hat_k?.['5'] ?? null} kind="rate" showCi={false} />),
      cell(<MetricValue estimate={arm.outcome_sd} kind="score" />),
      run ? `${arm.n_tasks} · ${arm.n_trials}` : '—',
      arm.cost_source === 'cli_usage' ? (
        <span className="sc-caveat" title={CAVEAT} data-caveat="cli_usage">
          cli_usage {'⚠'}
        </span>
      ) : (arm.cost_source ?? '—'),
    ];
  });
  const caveated = view.arms.filter((arm) => arm.cost_source === 'cli_usage');
  return (
    <ChartTable
      chart="arms"
      title="Arms"
      columns={[
        'arm',
        'resolve [95% CI]',
        '$/verified [95% CI]',
        'pass^1',
        'pass^3',
        'pass^5',
        'outcome sd',
        'tasks · trials',
        'cost source',
      ]}
      rows={rows}
      provenance={<Drawer provenance={view.provenance} subject="arms" metrics={view.metrics} />}
      note={caveated.length > 0 && (
        <span>
          {'⚠'} $ for {caveated.map((arm) => arm.label).join(', ')} = {CAVEAT}.
        </span>
      )}
    />
  );
}

/** The P1 head-to-head (S10 §4.3 B): the claim, the arms, Pareto, pass^k and the envelope. */
export function HeadToHeadBody({ view, choice }: { view: HeadToHeadView; choice: BundleChoice }) {
  return (
    <>
      <Badges choice={choice} provenance={view.provenance} />
      <ClaimBanner claim={view.claim} />
      <ArmsTable view={view} />
      <div className="sc-pair">
        <ParetoFrontierChart
          arms={view.arms}
          frontierArms={view.pareto.frontier_arms}
          provenance={<Drawer provenance={view.provenance} subject="Pareto chart" metrics={view.metrics} />}
        />
        <PassKChart
          arms={view.arms}
          provenance={<Drawer provenance={view.provenance} subject="pass^k chart" metrics={view.metrics} />}
        />
      </div>
      <EnvelopeTable
        rows={view.envelope}
        provenance={<Drawer provenance={view.provenance} subject="envelope" metrics={view.metrics} />}
      />
    </>
  );
}

/** `/demo/p1/head-to-head`: three arms plus the frontier·roko probe (D1, D2). */
export default function HeadToHead() {
  const choice = useBundleChoice();
  return (
    <section className="sc-page" data-showcase-page="head-to-head">
      <header className="sc-hero">
        <h1 className="sc-hero__title">Cheap + harness vs frontier-direct</h1>
        <p className="sc-hero__sub">
          Three arms plus the frontier·roko probe, with 95% intervals on every number.
        </p>
      </header>
      <GuardedView choice={choice} view="p1-head-to-head" subject="P1 head-to-head">
        {(view) => <HeadToHeadBody view={view} choice={choice} />}
      </GuardedView>
    </section>
  );
}
