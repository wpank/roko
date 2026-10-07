import type { ReactNode } from 'react';
import type { ArmRow, PassHatK } from '../../showcase/contracts';
import ChartTable, { Legend, MetricValue } from './ChartTable';
import { RATE_DOMAIN, RATE_TICKS, armColor, formatCi, formatValue, linearScale } from './ciScales';

const W = 640;
const H = 280;
const M = { top: 16, right: 24, bottom: 44, left: 52 };
const KS = ['1', '3', '5'] as const;
const CAP = 4;
const DODGE = 8;

interface PassKChartProps {
  arms: ArmRow[];
  provenance?: ReactNode;
}

/**
 * pass^k for k ∈ {1, 3, 5} per arm (S10 §4.3 B, tau-bench's consistency measure): one 2px line
 * per arm on a fixed [0,1] axis, a CI whisker at every k, arms dodged so whiskers never sit on
 * top of each other.
 */
export default function PassKChart({ arms, provenance }: PassKChartProps) {
  const series = arms.filter((arm): arm is ArmRow & { pass_hat_k: PassHatK } => (
    arm.status === 'run' && arm.pass_hat_k !== null
  ));
  const step = (W - M.left - M.right) / KS.length;
  const kx = (index: number) => M.left + step * (index + 0.5);
  const y = linearScale(RATE_DOMAIN, [H - M.bottom, M.top]);
  const dodge = (i: number) => (i - (series.length - 1) / 2) * DODGE;

  const rows = series.map((arm) => [
    arm.label,
    ...KS.map((k) => <MetricValue key={k} estimate={arm.pass_hat_k[k]} kind="rate" />),
  ]);

  return (
    <ChartTable
      chart="passk"
      title="pass^k by k (consistency across repeated trials)"
      columns={['arm', 'pass^1 [95% CI]', 'pass^3 [95% CI]', 'pass^5 [95% CI]']}
      rows={rows}
      provenance={provenance}
    >
      <svg
        className="sc-svg"
        viewBox={`0 0 ${W} ${H}`}
        role="img"
        aria-label="pass^k at k = 1, 3 and 5 per arm, with 95% intervals"
      >
        <g className="sc-grid" aria-hidden="true">
          {RATE_TICKS.map((t) => <line key={t} x1={M.left} x2={W - M.right} y1={y(t)} y2={y(t)} />)}
        </g>
        <g className="sc-axis" data-axis="y" data-domain={RATE_DOMAIN.join(',')}>
          {RATE_TICKS.map((t) => (
            <text key={t} x={M.left - 8} y={y(t) + 3} textAnchor="end">{t}</text>
          ))}
          <text x={14} y={(M.top + H - M.bottom) / 2} transform={`rotate(-90 14 ${(M.top + H - M.bottom) / 2})`} textAnchor="middle">
            pass^k
          </text>
        </g>
        <g className="sc-axis" data-axis="x" data-domain="1,3,5">
          <line className="sc-axis__rule" x1={M.left} x2={W - M.right} y1={H - M.bottom} y2={H - M.bottom} />
          {KS.map((k, i) => (
            <text key={k} x={kx(i)} y={H - M.bottom + 17} textAnchor="middle">k = {k}</text>
          ))}
          <text x={(M.left + W - M.right) / 2} y={H - 8} textAnchor="middle">trials that must all pass</text>
        </g>
        {series.map((arm, i) => {
          const color = armColor(arm);
          const points = KS.map((k, index) => ({ k, index, estimate: arm.pass_hat_k[k] }))
            .filter((p) => p.estimate.value !== null);
          const path = points
            .map((p, j) => `${j === 0 ? 'M' : 'L'}${kx(p.index) + dodge(i)},${y(p.estimate.value as number)}`)
            .join(' ');
          return (
            <g key={arm.arm} className="sc-series" data-arm={arm.arm}>
              <path className="sc-line" d={path} stroke={color} />
              {points.map((p) => {
                const cx = kx(p.index) + dodge(i);
                const value = p.estimate.value as number;
                const ci = p.estimate.ci;
                return (
                  <g key={p.k} className="sc-mark" tabIndex={0}>
                    <title>
                      {`${arm.label}, pass^${p.k}: ${formatValue(value, 'rate')} ${formatCi(ci, 'rate')}`}
                    </title>
                    {ci && (
                      <g data-ci="whisker" data-ci-axis="y" data-metric-ref={p.estimate.metric_ref} data-value={String(value)}>
                        <line x1={cx} x2={cx} y1={y(ci[0])} y2={y(ci[1])} stroke={color} />
                        <line x1={cx - CAP} x2={cx + CAP} y1={y(ci[0])} y2={y(ci[0])} stroke={color} />
                        <line x1={cx - CAP} x2={cx + CAP} y1={y(ci[1])} y2={y(ci[1])} stroke={color} />
                      </g>
                    )}
                    <circle className="sc-dot" cx={cx} cy={y(value)} r={4} fill={color} />
                    <circle className="sc-hit" cx={cx} cy={y(value)} r={12} />
                  </g>
                );
              })}
            </g>
          );
        })}
      </svg>
      <Legend items={series.map((arm) => ({ label: arm.label, color: armColor(arm), mark: 'line' as const }))} />
    </ChartTable>
  );
}
