import type { ReactNode } from 'react';
import type { EnvelopeRow, EnvelopeVerdict } from '../../showcase/contracts';
import ChartTable, { MetricValue } from './ChartTable';

/** The bundle's verdict per level, with a mark so it never relies on colour alone. */
const VERDICTS: Record<EnvelopeVerdict, string> = {
  within_target: 'within target ✓',
  frontier_wins: 'frontier wins ✗',
  inconclusive: 'inconclusive',
  not_measured: 'not yet measured',
};

interface EnvelopeTableProps {
  rows: EnvelopeRow[];
  provenance?: ReactNode;
}

/**
 * Where cheap·roko stops being enough (S10 §4.3 B): per cumulative envelope level j (tasks with
 * ℓ ≤ j, S09 §4.2) the two resolve rates, the resolve and $ ratios with their intervals, and the
 * verdict copied from the bundle. A level not measured says so instead of showing numbers.
 */
export default function EnvelopeTable({ rows, provenance }: EnvelopeTableProps) {
  const cells = rows.map((row) => {
    const measured = row.verdict !== 'not_measured';
    const value = (cell: ReactNode) => (measured ? cell : <span className="sc-dim">{'—'}</span>);
    return [
      row.label,
      measured ? row.n_tasks : <span className="sc-dim">{'—'}</span>,
      value(<MetricValue estimate={row.cheap_resolve} kind="rate" />),
      value(<MetricValue estimate={row.frontier_resolve} kind="rate" />),
      value(<MetricValue estimate={row.resolve_ratio} kind="ratio" />),
      value(<MetricValue estimate={row.usd_ratio} kind="ratio" />),
      <span className={`sc-verdict sc-verdict--${row.verdict}`} data-verdict={row.verdict}>
        {VERDICTS[row.verdict]}
      </span>,
    ];
  });
  return (
    <ChartTable
      chart="envelope"
      title="Envelope by difficulty: where cheap·roko stops being enough"
      columns={[
        'level',
        'n tasks',
        'cheap·roko resolve',
        'frontier·direct resolve',
        'resolve ratio',
        '$ ratio',
        'verdict',
      ]}
      rows={cells}
      provenance={provenance}
    />
  );
}
