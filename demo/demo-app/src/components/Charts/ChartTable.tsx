import { useState, useSyncExternalStore, type ReactNode } from 'react';
import type { Estimate, MetricKind } from '../../showcase/contracts';
import { formatCi, formatValue } from './ciScales';
import './CiCharts.css';

const NARROW_QUERY = '(max-width: 480px)';

function subscribeNarrow(onChange: () => void): () => void {
  if (typeof window === 'undefined' || !window.matchMedia) return () => {};
  const query = window.matchMedia(NARROW_QUERY);
  query.addEventListener('change', onChange);
  return () => query.removeEventListener('change', onChange);
}

function isNarrow(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia
    && window.matchMedia(NARROW_QUERY).matches;
}

/** True under 480 px, where every chart shows its table first (S10 SC7). */
export function useNarrowViewport(): boolean {
  return useSyncExternalStore(subscribeNarrow, isNarrow, () => false);
}

interface MetricValueProps {
  estimate: Estimate | null;
  kind: MetricKind;
  /** Print the interval after the value (default true). */
  showCi?: boolean;
}

/**
 * One bundle number with its interval. `data-metric-ref` and `data-value` carry the record
 * and the exact value, so specs compare what renders with `views/*.json`.
 */
export function MetricValue({ estimate, kind, showCi = true }: MetricValueProps) {
  if (!estimate) return <span className="sc-value sc-value--none">{'—'}</span>;
  return (
    <span
      className="sc-value"
      data-metric-ref={estimate.metric_ref}
      data-value={String(estimate.value)}
    >
      {formatValue(estimate.value, kind)}
      {showCi && estimate.ci && <span className="sc-ci"> {formatCi(estimate.ci, kind)}</span>}
    </span>
  );
}

export interface LegendItem {
  label: string;
  color: string;
  /** How the series is drawn, mirrored by its key. */
  mark: 'dot' | 'line';
}

/** The legend every chart with two or more series carries; text stays in text colours. */
export function Legend({ items }: { items: LegendItem[] }) {
  if (items.length < 2) return null;
  return (
    <ul className="sc-legend">
      {items.map((item) => (
        <li key={item.label}>
          <span
            className={`sc-legend__key sc-legend__key--${item.mark}`}
            style={{ background: item.color }}
            aria-hidden="true"
          />
          {item.label}
        </li>
      ))}
    </ul>
  );
}

interface ChartTableProps {
  /** `data-chart` id, e.g. `pareto`. */
  chart: string;
  title: string;
  /** The table fallback: column headers, then one row of cells per mark or series. */
  columns: string[];
  rows: ReactNode[][];
  /** The ⓘ, usually a `ProvenanceDrawer`. */
  provenance?: ReactNode;
  /** Shown under the chart or table, e.g. a cost caveat. */
  note?: ReactNode;
  /** The chart; a panel without one is a table only. */
  children?: ReactNode;
}

/**
 * The frame of every showcase chart: a title, the ⓘ, a chart/table toggle, and the accessible
 * table that replaces the chart on request and by default under 480 px.
 */
export default function ChartTable({
  chart,
  title,
  columns,
  rows,
  provenance,
  note,
  children,
}: ChartTableProps) {
  const narrow = useNarrowViewport();
  const [choice, setChoice] = useState<'chart' | 'table' | null>(null);
  const tableOnly = children === undefined;
  const mode = tableOnly ? 'table' : (choice ?? (narrow ? 'table' : 'chart'));
  return (
    <figure className="sc-chart" data-chart={chart} data-mode={mode}>
      <figcaption className="sc-chart__head">
        <span className="sc-chart__title">{title}</span>
        <span className="sc-chart__tools">
          {!tableOnly && (
            <button
              type="button"
              className="sc-chart__toggle"
              aria-pressed={mode === 'table'}
              onClick={() => setChoice(mode === 'table' ? 'chart' : 'table')}
            >
              {mode === 'table' ? 'show chart' : 'show table'}
            </button>
          )}
          {provenance}
        </span>
      </figcaption>
      {mode === 'chart' ? (
        <div className="sc-chart__plot">{children}</div>
      ) : (
        <div className="sc-chart__table">
          <table className="sc-table">
            <caption className="sc-visually-hidden">{title}</caption>
            <thead>
              <tr>
                {columns.map((column) => <th key={column} scope="col">{column}</th>)}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, i) => (
                <tr key={i}>
                  {row.map((cell, j) => (j === 0
                    ? <th key={j} scope="row">{cell}</th>
                    : <td key={j}>{cell}</td>))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {note && <p className="sc-chart__note">{note}</p>}
    </figure>
  );
}
