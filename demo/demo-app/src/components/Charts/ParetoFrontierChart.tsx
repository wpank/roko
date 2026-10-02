import type { ReactNode } from 'react';
import type { ArmRow, Estimate } from '../../showcase/contracts';
import ChartTable, { Legend, MetricValue } from './ChartTable';
import {
  RATE_DOMAIN,
  RATE_TICKS,
  armColor,
  formatCi,
  formatUsdTick,
  formatValue,
  linearScale,
  logDomain,
  logScale,
  logTicks,
} from './ciScales';

const W = 640;
const H = 320;
const M = { top: 16, right: 24, bottom: 48, left: 52 };
const CAP = 4;

interface Plotted {
  arm: ArmRow;
  resolve: Estimate & { value: number };
  usd: Estimate & { value: number };
}

function plotted(arms: ArmRow[]): Plotted[] {
  const out: Plotted[] = [];
  for (const arm of arms) {
    const resolve = arm.resolve;
    const usd = arm.usd_per_verified;
    if (arm.status !== 'run' || !resolve || !usd) continue;
    if (resolve.value === null || usd.value === null || usd.value <= 0) continue;
    out.push({
      arm,
      resolve: { ...resolve, value: resolve.value },
      usd: { ...usd, value: usd.value },
    });
  }
  return out;
}

/** S01 §4.4: a frontier-direct $ the CLI reported on a subscription is an API equivalent. */
function hasCaveat(arm: ArmRow): boolean {
  return arm.cost_source === 'cli_usage';
}

interface ParetoFrontierChartProps {
  arms: ArmRow[];
  /** The frontier computed offline (`pareto.frontier_arms`), joined in this order. */
  frontierArms: string[];
  provenance?: ReactNode;
}

/**
 * Resolve against $ per verified success (S10 §4.3 B): $ on a labelled log axis, resolve on
 * [0,1], a CI whisker on both axes of every arm, the frontier line from the bundle, and the
 * `cost_source` caveat on CLI-priced arms. Arms not run are listed in the table, not plotted.
 */
export default function ParetoFrontierChart({
  arms,
  frontierArms,
  provenance,
}: ParetoFrontierChartProps) {
  const points = plotted(arms);
  const xDomain = logDomain(points.flatMap((p) => [p.usd.value, ...(p.usd.ci ?? [])]));
  const x = logScale(xDomain, [M.left, W - M.right]);
  const y = linearScale(RATE_DOMAIN, [H - M.bottom, M.top]);
  const ticks = logTicks(xDomain, formatUsdTick);
  const frontier = frontierArms
    .map((id) => points.find((p) => p.arm.arm === id))
    .filter((p): p is Plotted => p !== undefined);
  const caveated = points.filter((p) => hasCaveat(p.arm)).map((p) => p.arm.label);

  const rows = arms.map((arm) => [
    <span key="arm">
      {arm.label}
      {arm.note && <span className="sc-dim"> · {arm.note}</span>}
    </span>,
    arm.status === 'run'
      ? <MetricValue key="r" estimate={arm.resolve} kind="rate" />
      : <span key="r" className="sc-dim">not run</span>,
    arm.status === 'run'
      ? <MetricValue key="u" estimate={arm.usd_per_verified} kind="usd" />
      : <span key="u" className="sc-dim">not run</span>,
    arm.cost_source ?? '—',
  ]);

  return (
    <ChartTable
      chart="pareto"
      title="Resolve vs $ per verified success"
      columns={['arm', 'resolve [95% CI]', '$/verified [95% CI]', 'cost source']}
      rows={rows}
      provenance={provenance}
      note={caveated.length > 0 && (
        <span data-caveat="cli_usage">
          {'⚠'} $ for {caveated.join(', ')} = API-equivalent reported by the CLI
          (subscription), not billed spend.
        </span>
      )}
    >
      <svg
        className="sc-svg"
        viewBox={`0 0 ${W} ${H}`}
        role="img"
        aria-label="Resolve rate against dollars per verified success, with 95% intervals"
      >
        <g className="sc-grid" aria-hidden="true">
          {RATE_TICKS.map((t) => <line key={`y${t}`} x1={M.left} x2={W - M.right} y1={y(t)} y2={y(t)} />)}
          {ticks.filter((t) => t.label).map((t) => (
            <line key={`x${t.value}`} x1={x(t.value)} x2={x(t.value)} y1={M.top} y2={H - M.bottom} />
          ))}
        </g>
        <g className="sc-axis" data-axis="y" data-domain={RATE_DOMAIN.join(',')}>
          {RATE_TICKS.map((t) => (
            <text key={t} x={M.left - 8} y={y(t) + 3} textAnchor="end">{t}</text>
          ))}
          <text x={14} y={(M.top + H - M.bottom) / 2} transform={`rotate(-90 14 ${(M.top + H - M.bottom) / 2})`} textAnchor="middle">
            resolve
          </text>
        </g>
        <g className="sc-axis" data-axis="x" data-scale="log" data-domain={xDomain.join(',')}>
          <line className="sc-axis__rule" x1={M.left} x2={W - M.right} y1={H - M.bottom} y2={H - M.bottom} />
          {ticks.map((t) => (
            <g key={t.value}>
              <line className="sc-axis__tick" x1={x(t.value)} x2={x(t.value)} y1={H - M.bottom} y2={H - M.bottom + (t.label ? 5 : 3)} />
              {t.label && <text x={x(t.value)} y={H - M.bottom + 17} textAnchor="middle">{t.label}</text>}
            </g>
          ))}
          <text x={(M.left + W - M.right) / 2} y={H - 8} textAnchor="middle">
            $ per verified success (log scale)
          </text>
        </g>
        {frontier.length > 1 && (
          <path
            className="sc-frontier"
            data-frontier="true"
            d={frontier.map((p, i) => `${i === 0 ? 'M' : 'L'}${x(p.usd.value)},${y(p.resolve.value)}`).join(' ')}
          />
        )}
        {points.map((p) => {
          const color = armColor(p.arm);
          const cx = x(p.usd.value);
          const cy = y(p.resolve.value);
          const right = cx < M.left + (W - M.left - M.right) * 0.7;
          const title = `${p.arm.label}: resolve ${formatValue(p.resolve.value, 'rate')} `
            + `${formatCi(p.resolve.ci, 'rate')}, $/verified ${formatValue(p.usd.value, 'usd')} `
            + `${formatCi(p.usd.ci, 'usd')}`;
          return (
            <g key={p.arm.arm} className="sc-mark" data-arm={p.arm.arm} tabIndex={0}>
              <title>{title}</title>
              {p.usd.ci && (
                <g data-ci="whisker" data-axis="x" data-metric-ref={p.usd.metric_ref} data-value={String(p.usd.value)}>
                  <line x1={x(p.usd.ci[0])} x2={x(p.usd.ci[1])} y1={cy} y2={cy} stroke={color} />
                  <line x1={x(p.usd.ci[0])} x2={x(p.usd.ci[0])} y1={cy - CAP} y2={cy + CAP} stroke={color} />
                  <line x1={x(p.usd.ci[1])} x2={x(p.usd.ci[1])} y1={cy - CAP} y2={cy + CAP} stroke={color} />
                </g>
              )}
              {p.resolve.ci && (
                <g data-ci="whisker" data-axis="y" data-metric-ref={p.resolve.metric_ref} data-value={String(p.resolve.value)}>
                  <line x1={cx} x2={cx} y1={y(p.resolve.ci[0])} y2={y(p.resolve.ci[1])} stroke={color} />
                  <line x1={cx - CAP} x2={cx + CAP} y1={y(p.resolve.ci[0])} y2={y(p.resolve.ci[0])} stroke={color} />
                  <line x1={cx - CAP} x2={cx + CAP} y1={y(p.resolve.ci[1])} y2={y(p.resolve.ci[1])} stroke={color} />
                </g>
              )}
              <circle className="sc-dot" cx={cx} cy={cy} r={5} fill={color} />
              <circle className="sc-hit" cx={cx} cy={cy} r={12} />
              <text className="sc-mark__label" x={right ? cx + 10 : cx - 10} y={cy - 8} textAnchor={right ? 'start' : 'end'}>
                {p.arm.label}{hasCaveat(p.arm) ? ' ⚠' : ''}
              </text>
            </g>
          );
        })}
      </svg>
      <Legend
        items={[
          ...points.map((p) => ({ label: p.arm.label, color: armColor(p.arm), mark: 'dot' as const })),
          ...(frontier.length > 1 ? [{ label: 'frontier (from the bundle)', color: '#9a8a98', mark: 'line' as const }] : []),
        ]}
      />
    </ChartTable>
  );
}
