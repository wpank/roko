/**
 * Display-formatting utilities for the roko portal.
 *
 * All functions are pure and have no side-effects, making them safe to call
 * during server-side rendering.
 */

/**
 * Format a USD cost value into a compact human-readable string.
 *
 * Values >= $0.01 render with two decimal places ("$1.23").
 * Values below $0.01 render with four decimal places to preserve resolution
 * on very cheap operations ("$0.0012").
 *
 * @example
 * formatCost(1.234)   // "$1.23"
 * formatCost(0.00123) // "$0.0012"
 * formatCost(0)       // "$0.00"
 */
export function formatCost(usd: number): string {
  if (usd === 0) return '$0.00';
  if (Math.abs(usd) < 0.01) {
    return `$${usd.toFixed(4)}`;
  }
  return `$${usd.toFixed(2)}`;
}

/**
 * Format a token count into a compact human-readable string.
 *
 * Values < 1 000 render as integers ("42").
 * Values in the thousands render as "1.2k".
 * Values >= 1 000 000 render as "1.2M".
 *
 * @example
 * formatTokens(42)        // "42"
 * formatTokens(1234)      // "1.2k"
 * formatTokens(1_500_000) // "1.5M"
 */
export function formatTokens(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return `${Math.round(count)}`;
}

/**
 * Format a duration in milliseconds into a compact human-readable string.
 *
 * - Under 60 000 ms: "1.2s"
 * - Under 3 600 000 ms: "2m 30s"
 * - Otherwise: "1h 15m"
 *
 * @example
 * formatDuration(1234)        // "1.2s"
 * formatDuration(150_000)     // "2m 30s"
 * formatDuration(4_500_000)   // "1h 15m"
 */
export function formatDuration(ms: number): string {
  const totalSeconds = Math.round(ms / 1_000);
  if (totalSeconds < 60) {
    // Sub-minute: show one decimal in seconds.
    return `${(ms / 1_000).toFixed(1)}s`;
  }
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) {
    const secs = totalSeconds % 60;
    return secs > 0 ? `${totalMinutes}m ${secs}s` : `${totalMinutes}m`;
  }
  const hours = Math.floor(totalMinutes / 60);
  const mins = totalMinutes % 60;
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
}

/**
 * Truncate a long string in the middle, keeping both the head and the tail
 * visible around a single "…" character.  Preserves identifier tails so that
 * plan names like "portal-…-shell" remain distinguishable.
 *
 * - text.length <= max → returned unchanged.
 * - max < 3 → first max characters (no room for "…" plus two sides).
 * - Otherwise the result is exactly max characters.  The tail receives the
 *   extra character when (max - 1) is odd.
 *
 * @example
 * middleEllipsis('portal-plan-implementer-shell', 20) // "portal-pl…nter-shell"
 * middleEllipsis('short', 10)                         // "short"
 */
export function middleEllipsis(text: string, max: number): string {
  if (max < 3) return text.slice(0, max);
  if (text.length <= max) return text;
  const available = max - 1; // 1 char reserved for "…"
  const headLen = Math.floor(available / 2);
  const tailLen = Math.ceil(available / 2);
  return text.slice(0, headLen) + '…' + text.slice(-tailLen);
}

/**
 * Format a duration in milliseconds in a compact form inspired by mori's UI.
 *
 * - "45s"   — under a minute
 * - "2m14s" — minutes with seconds
 * - "1h05m" — hours with zero-padded minutes (seconds dropped)
 * - "·"     — null, undefined, NaN, or negative (placeholder proves cell exists)
 *
 * @example
 * compactDuration(45_000)       // "45s"
 * compactDuration(134_000)      // "2m14s"
 * compactDuration(3_900_000)    // "1h05m"
 * compactDuration(null)         // "·"
 */
export function compactDuration(ms: number | null | undefined): string {
  if (ms == null || isNaN(ms) || ms < 0) return '·';
  const totalSeconds = Math.floor(ms / 1_000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) {
    const secs = totalSeconds % 60;
    return `${totalMinutes}m${secs}s`;
  }
  const hours = Math.floor(totalMinutes / 60);
  const mins = totalMinutes % 60;
  return `${hours}h${String(mins).padStart(2, '0')}m`;
}

/**
 * Unified time-cell formatter.  Returns null for unknown/invalid time so
 * callers can suppress the cell entirely instead of showing a placeholder.
 *
 * - kind 'estimate': rounded up to whole minutes (min 1), prefixed with "~".
 *   Under an hour: "~9m".  One hour or more: "~1h35m".
 * - kind 'elapsed' | 'actual': exact compact form via compactDuration ("12s",
 *   "1m12s", "1h05m").
 * - kind 'none', or any kind with null / negative / non-finite ms: null.
 *
 * @example
 * formatSpan({ kind: 'estimate', ms: 9 * 60_000 })   // "~9m"
 * formatSpan({ kind: 'estimate', ms: 95 * 60_000 })  // "~1h35m"
 * formatSpan({ kind: 'elapsed',  ms: 12_000 })        // "12s"
 * formatSpan({ kind: 'none',     ms: null })           // null
 */
export function formatSpan(time: {
  kind: 'estimate' | 'elapsed' | 'actual' | 'none';
  ms: number | null;
}): string | null {
  if (time.kind === 'none') return null;
  const { ms } = time;
  if (ms == null || !isFinite(ms) || ms < 0) return null;

  if (time.kind === 'estimate') {
    const minutes = Math.max(1, Math.ceil(ms / 60_000));
    if (minutes < 60) return `~${minutes}m`;
    return `~${compactDuration(minutes * 60_000)}`;
  }

  // 'elapsed' | 'actual': ms is already validated as finite and non-negative.
  return compactDuration(ms);
}

/**
 * Shorten a model identifier for display in a compact table cell.
 *
 * Transformations applied in order:
 *  1. Strip a provider path prefix ("openai/" → "").
 *  2. Strip a leading "claude-" vendor prefix.
 *  3. Strip a trailing date suffix ("-20250514" or "-2025-05-14").
 *
 * @example
 * shortModel('claude-sonnet-4-20250514') // "sonnet-4"
 * shortModel('claude-opus-4-6')          // "opus-4-6"
 * shortModel('openai/gpt-5.1-codex')     // "gpt-5.1-codex"
 * shortModel('kimi-k2')                  // "kimi-k2"
 */
export function shortModel(model: string): string {
  // 1. Drop provider path (e.g. "openai/")
  const slashIdx = model.lastIndexOf('/');
  let name = slashIdx >= 0 ? model.slice(slashIdx + 1) : model;
  // 2. Drop leading "claude-"
  if (name.startsWith('claude-')) name = name.slice('claude-'.length);
  // 3. Drop trailing date suffix: -YYYYMMDD or -YYYY-MM-DD
  name = name.replace(/-\d{8}$/, '').replace(/-\d{4}-\d{2}-\d{2}$/, '');
  return name;
}
