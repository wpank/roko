/**
 * Scales, colours and number formats for the CI-aware showcase charts (S10 §4.1): rates on a
 * fixed [0,1] axis, dollars on a labelled log axis, and an interval beside every value. These
 * only place and print numbers that came from the bundle; nothing here computes a statistic.
 */
import type { ArmRow, MetricKind } from '../../showcase/contracts';

/**
 * Categorical slots for the arms, in fixed order: the dataviz reference palette's dark steps,
 * validated on the app's dark surfaces (#12101a, #08080c): adjacent pairs pass, and the first
 * three also pass all-pairs, so scatter plots use only those three plus muted gray.
 */
export const SERIES = ['#3987e5', '#d95926', '#199e70', '#c98500'] as const;

/** De-emphasised marks: extra frontier-direct rows and probes. */
export const MUTED = '#8b8794';

/** An arm keeps its colour whatever else is shown: colour follows the entity, never the rank. */
export function armColor(arm: Pick<ArmRow, 'tier' | 'harness' | 'role'>): string {
  if (arm.role !== 'arm') return MUTED;
  if (arm.tier === 'cheap' && arm.harness === 'roko') return SERIES[0];
  if (arm.tier === 'frontier' && arm.harness === 'direct') return SERIES[1];
  if (arm.tier === 'cheap' && arm.harness === 'direct') return SERIES[2];
  return SERIES[3];
}

/** The rate axis is always [0,1] (S10 §4.1). */
export const RATE_DOMAIN: [number, number] = [0, 1];
export const RATE_TICKS = [0, 0.25, 0.5, 0.75, 1];

export type Scale = (value: number) => number;

export function linearScale(domain: [number, number], range: [number, number]): Scale {
  const [d0, d1] = domain;
  const [r0, r1] = range;
  const span = d1 - d0 || 1;
  return (value) => r0 + ((value - d0) / span) * (r1 - r0);
}

/** Whole decades around the positive `values`, e.g. [0.01, 10]; [0.01, 1] when there are none. */
export function logDomain(values: number[]): [number, number] {
  const positive = values.filter((v) => Number.isFinite(v) && v > 0);
  if (positive.length === 0) return [0.01, 1];
  const lo = 10 ** Math.floor(Math.log10(Math.min(...positive)));
  const hi = 10 ** Math.ceil(Math.log10(Math.max(...positive)));
  return hi > lo ? [lo, hi] : [lo, lo * 10];
}

export function logScale(domain: [number, number], range: [number, number]): Scale {
  const inner = linearScale([Math.log10(domain[0]), Math.log10(domain[1])], range);
  return (value) => inner(Math.log10(Math.max(value, domain[0] / 10)));
}

export interface Tick {
  value: number;
  label: string | null;
}

/** 1-2-5 ticks over a log domain; every tick is labelled up to two decades, else only the 1s. */
export function logTicks(domain: [number, number], format: (v: number) => string): Tick[] {
  const first = Math.round(Math.log10(domain[0]));
  const last = Math.round(Math.log10(domain[1]));
  const ticks: Tick[] = [];
  for (let exponent = first; exponent <= last; exponent += 1) {
    for (const mantissa of [1, 2, 5]) {
      const value = Number((mantissa * 10 ** exponent).toPrecision(1));
      if (value < domain[0] * 0.999 || value > domain[1] * 1.001) continue;
      const labelled = last - first <= 2 || mantissa === 1;
      ticks.push({ value, label: labelled ? format(value) : null });
    }
  }
  return ticks;
}

function decimals(value: number): number {
  const magnitude = Math.abs(value);
  return magnitude !== 0 && magnitude < 0.1 ? 3 : 2;
}

/** A bundle value as text: `—` when undefined; dollars with `$`; counts whole. */
export function formatValue(value: number | null, kind: MetricKind): string {
  if (value === null) return '—';
  switch (kind) {
    case 'usd':
      return `$${value.toFixed(value < 1 ? 3 : 2)}`;
    case 'count':
      return String(Math.round(value));
    default:
      return value.toFixed(decimals(value));
  }
}

/** `[low, high]`, or an empty string for a value without an interval. */
export function formatCi(ci: number[] | null, kind: MetricKind): string {
  if (!ci || ci.length !== 2) return '';
  return `[${formatValue(ci[0], kind)}, ${formatValue(ci[1], kind)}]`;
}

/** A dollar tick label: `$0.05`, `$1`, `$20`. */
export function formatUsdTick(value: number): string {
  return `$${Number(value.toPrecision(2))}`;
}

/** A timeline label: `HH:MM` (UTC) for an ISO timestamp, the text otherwise. */
export function formatTimeTick(x: string): string {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(x) ? x.slice(11, 16) : x;
}
