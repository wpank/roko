/**
 * roko's words and number formats (the design system's §6): the same on every surface, so the portal reads like
 * roko's pages in Hermes and its terminal.
 */

/** A duration: `5.3s` under 10 s, `42s` under a minute, `4m 05s` under an hour, then `1h 12m`. */
export function duration(ms: number): string {
  const safe = Math.max(0, ms);
  if (safe < 10_000) return `${(Math.floor(safe / 100) / 10).toFixed(1)}s`;
  const seconds = Math.floor(safe / 1_000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${String(seconds % 60).padStart(2, '0')}s`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
}

/** A clock on the glass: `0:42`, `4:05`, `1:12:30`. */
export function clock(ms: number): string {
  const seconds = Math.floor(Math.max(0, ms) / 1_000);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = String(seconds % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}

/** The plain words a screen reader says for a clock: "4 minutes 5 seconds". */
export function clockLabel(ms: number): string {
  const seconds = Math.floor(Math.max(0, ms) / 1_000);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const parts: string[] = [];
  if (h) parts.push(`${h} ${h === 1 ? 'hour' : 'hours'}`);
  if (m) parts.push(`${m} ${m === 1 ? 'minute' : 'minutes'}`);
  if (s || parts.length === 0) parts.push(`${s} ${s === 1 ? 'second' : 'seconds'}`);
  return parts.join(' ');
}

/** Money, always two decimals: `$0.44`; `<$0.01` for less than a cent but not nothing; `$0.00` for nothing. */
export function money(usd: number): string {
  if (!Number.isFinite(usd) || usd <= 0) return '$0.00';
  if (usd < 0.005) return '<$0.01';
  return `$${usd.toFixed(2)}`;
}

/** A count with its noun: `1 task`, `3 tasks`. */
export function count(n: number, noun: string, plural = `${noun}s`): string {
  return `${n} ${n === 1 ? noun : plural}`;
}
