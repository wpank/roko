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
 * Format a Unix timestamp (milliseconds) as a human-readable relative time.
 *
 * - Under 60 s ago: "just now"
 * - Under 60 min ago: "5m ago"
 * - Under 24 h ago: "3h ago"
 * - Under 48 h ago: "yesterday"
 * - Otherwise: a locale date string
 *
 * @param timestamp — Unix timestamp in milliseconds.
 *
 * @example
 * formatRelativeTime(Date.now() - 90_000)        // "1m ago"
 * formatRelativeTime(Date.now() - 7_200_000)     // "2h ago"
 */
export function formatRelativeTime(timestamp: number): string {
  const diffMs = Date.now() - timestamp;
  const diffSeconds = Math.floor(diffMs / 1_000);

  if (diffSeconds < 60) return 'just now';

  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return `${diffMinutes}m ago`;

  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;

  if (diffHours < 48) return 'yesterday';

  return new Date(timestamp).toLocaleDateString();
}

/**
 * Format a number as a percentage string.
 *
 * @param value    — Numeric value in the range [0, 1] or [0, 100].
 *                   Values > 1 are treated as already-percentage (e.g. 42.1 → "42.1%").
 *                   Values <= 1 are multiplied by 100 first (e.g. 0.421 → "42.1%").
 * @param decimals — Number of decimal places (default 1).
 *
 * @example
 * formatPercentage(0.421)     // "42.1%"
 * formatPercentage(42.1)      // "42.1%"
 * formatPercentage(0.5, 0)    // "50%"
 */
export function formatPercentage(value: number, decimals = 1): string {
  const pct = value <= 1 ? value * 100 : value;
  return `${pct.toFixed(decimals)}%`;
}

/**
 * Truncate a hex hash string to `length` characters, returning just the
 * prefix (no ellipsis).  Useful for displaying commit SHAs or content hashes.
 *
 * @param hash   — Full hash string.
 * @param length — Number of characters to keep (default 8).
 *
 * @example
 * truncateHash('a1b2c3d4e5f6')        // "a1b2c3d4"
 * truncateHash('a1b2c3d4e5f6', 6)     // "a1b2c3"
 */
export function truncateHash(hash: string, length = 8): string {
  return hash.slice(0, length);
}

/**
 * Format a number with comma-separated thousands groups.
 *
 * Uses `Intl.NumberFormat` when available, falling back to a simple regex
 * replacement for environments where `Intl` is unavailable.
 *
 * @example
 * formatNumber(1234567) // "1,234,567"
 * formatNumber(42)      // "42"
 */
export function formatNumber(n: number): string {
  try {
    return new Intl.NumberFormat('en-US').format(n);
  } catch {
    return n.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  }
}
