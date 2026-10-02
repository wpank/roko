import type { ReactNode } from 'react';
import type { Estimate, MetricKind } from '../../showcase/contracts';
import ChartTable, { Legend, MetricValue } from './ChartTable';
import {
  RATE_DOMAIN,
  RATE_TICKS,
  SERIES,
  formatCi,
  formatTimeTick,
  formatValue,
  linearScale,
} from './ciScales';

const W = 640;
const H = 260;
const M = { top: 16, right: 24, bottom: 44, left: 52 };

export interface BandSeries {
  id: string;
  label: string;
  /** A colour from `SERIES` or `MUTED`; defaults to the series' slot. */
  color?: string;
  points: { t: string; estimate: Estimate }[];
}

interface BandLineChartProps {
  /** `data-chart` id. */
  chart: string;
  title: string;
  kind: MetricKind;
  series: BandSeries[];
  /** The y-axis title, e.g. "false-green rate". */
  yLabel: string;
  provenance?: ReactNode;
}

function niceMax(values: number[]): number {
  const top = Math.max(...values, 0);
  if (top <= 0) return 1;
  const magnitude = 10 ** Math.floor(Math.log10(top));
  return Math.ceil(top / magnitude) * magnitude;
}

/**
 * Series over time with their 95% bands (S10 §4.3 E, G, H, I): a 2px line through the values
 * and the interval as a 10% wash behind it. Rates sit on the fixed [0,1] axis; other kinds
 * start at 0. Points are evenly spaced in time order.
 */
export default function BandLineChart({
  chart,
  title,
  kind,
  series,
  yLabel,
  provenance,
}: BandLineChartProps) {
  const times = [...new Set(series.flatMap((s) => s.points.map((p) => p.t)))].sort();
  const step = (W - M.left - M.right) / Math.max(times.length - 1, 1);
  const x = (t: string) => M.left + step * times.indexOf(t);
  const bounds = series.flatMap((s) => s.points.flatMap((p) => [
    p.estimate.value ?? 0,
    ...(p.estimate.ci ?? []),
  ]));
  const domain: [number, number] = kind === 'rate' ? RATE_DOMAIN : [0, niceMax(bounds)];
  const y = linearScale(domain, [H - M.bottom, M.top]);
  const yTicks = kind === 'rate' ? RATE_TICKS : [0, 0.25, 0.5, 0.75, 1].map((f) => f * domain[1]);
  const colorOf = (s: BandSeries, i: number) => s.color ?? SERIES[i % SERIES.length];

  const rows = times.map((t) => [
    formatTimeTick(t),
    ...series.map((s) => (
      <MetricValue key={s.id} estimate={s.points.find((p) => p.t === t)?.estimate ?? null} kind={kind} />
    )),
  ]);

  return (
    <ChartTable
      chart={chart}
      title={title}
      columns={['time', ...series.map((s) => `${s.label} [95% CI]`)]}
      rows={rows}
      provenance={provenance}
    >
      <svg className="sc-svg" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`${title}, with 95% bands`}>
        <g className="sc-grid" aria-hidden="true">
          {yTicks.map((t) => <line key={t} x1={M.left} x2={W - M.right} y1={y(t)} y2={y(t)} />)}
        </g>
        <g className="sc-axis" data-axis="y" data-domain={domain.join(',')}>
          {yTicks.map((t) => (
            <text key={t} x={M.left - 8} y={y(t) + 3} textAnchor="end">{formatValue(t, kind)}</text>
          ))}
          <text x={14} y={(M.top + H - M.bottom) / 2} transform={`rotate(-90 14 ${(M.top + H - M.bottom) / 2})`} textAnchor="middle">
            {yLabel}
          </text>
        </g>
        <g className="sc-axis" data-axis="x">
          <line className="sc-axis__rule" x1={M.left} x2={W - M.right} y1={H - M.bottom} y2={H - M.bottom} />
          {times.map((t) => (
            <text key={t} x={x(t)} y={H - M.bottom + 17} textAnchor="middle">{formatTimeTick(t)}</text>
          ))}
        </g>
        {series.map((s, i) => {
          const color = colorOf(s, i);
          const valued = s.points.filter((p) => p.estimate.value !== null);
          const banded = s.points.filter((p) => p.estimate.ci && p.estimate.ci.length === 2);
          const upper = banded.map((p) => `${x(p.t)},${y((p.estimate.ci as number[])[1])}`);
          const lower = banded.map((p) => `${x(p.t)},${y((p.estimate.ci as number[])[0])}`).reverse();
          const line = valued
            .map((p, j) => `${j === 0 ? 'M' : 'L'}${x(p.t)},${y(p.estimate.value as number)}`)
            .join(' ');
          return (
            <g key={s.id} className="sc-series" data-series={s.id}>
              {banded.length > 1 && (
                <polygon data-ci="band" className="sc-band" points={[...upper, ...lower].join(' ')} fill={color} />
              )}
              <path className="sc-line" d={line} stroke={color} />
              {valued.map((p) => (
                <g key={p.t} className="sc-mark" tabIndex={0} data-metric-ref={p.estimate.metric_ref} data-value={String(p.estimate.value)}>
                  <title>
                    {`${s.label} at ${formatTimeTick(p.t)}: ${formatValue(p.estimate.value, kind)} `
                      + formatCi(p.estimate.ci, kind)}
                  </title>
                  <circle className="sc-dot" cx={x(p.t)} cy={y(p.estimate.value as number)} r={4} fill={color} />
                  <circle className="sc-hit" cx={x(p.t)} cy={y(p.estimate.value as number)} r={12} />
                </g>
              ))}
            </g>
          );
        })}
      </svg>
      <Legend items={series.map((s, i) => ({ label: s.label, color: colorOf(s, i), mark: 'line' as const }))} />
    </ChartTable>
  );
}
