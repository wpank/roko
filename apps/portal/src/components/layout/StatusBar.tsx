'use client';

import { useEffect, useState } from 'react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface StatusBarProps {
  /** Git branch name, or `null` when not in a repo / info unavailable. */
  gitBranch?: string | null;
  /** Number of live agents. */
  agentCount?: number;
  /** Total number of configured providers. */
  providerCount?: number;
  /** Current API round-trip latency in milliseconds.  `null` = unknown. */
  latencyMs?: number | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTime(d: Date): string {
  return d.toLocaleTimeString('en-GB', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
}

// ---------------------------------------------------------------------------
// StatusBar
// ---------------------------------------------------------------------------

export default function StatusBar({
  gitBranch = null,
  agentCount = 0,
  providerCount = 0,
  latencyMs = null,
}: StatusBarProps) {
  const [clock, setClock] = useState<string>(() => formatTime(new Date()));

  // Update clock every second on the client.
  useEffect(() => {
    const id = setInterval(() => {
      setClock(formatTime(new Date()));
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const baseCls = clsx(
    'font-[var(--font-mono)]',
    'text-[11px]',
    'text-[var(--text-faint)]',
    'leading-none',
    'whitespace-nowrap',
  );

  return (
    <footer
      className={clsx(
        'flex items-center justify-between',
        'h-6 px-3 shrink-0',
        'bg-[var(--bg-secondary)]',
        'border-t border-t-[var(--text-ghost)]',
        // Pin at the very bottom of the layout
        'relative z-[var(--z-sticky)]',
      )}
      aria-label="Status bar"
    >
      {/* ---- Left: Git branch ---- */}
      <div className="flex items-center gap-1.5 min-w-0">
        {gitBranch ? (
          <>
            {/* Git branch glyph (using unicode, no icon import needed) */}
            <span className={clsx(baseCls, 'opacity-50')} aria-hidden>
              ⎇
            </span>
            <span className={clsx(baseCls, 'truncate max-w-[160px]')} title={gitBranch}>
              {gitBranch}
            </span>
          </>
        ) : (
          <span className={baseCls}>no git info</span>
        )}
      </div>

      {/* ---- Center: Counts ---- */}
      <div className="flex items-center">
        <span className={baseCls}>
          {agentCount} {agentCount === 1 ? 'agent' : 'agents'}
          {' · '}
          {providerCount} {providerCount === 1 ? 'provider' : 'providers'}
        </span>
      </div>

      {/* ---- Right: Latency + Clock ---- */}
      <div className="flex items-center gap-3">
        {latencyMs !== null ? (
          <span
            className={clsx(
              baseCls,
              // Colour-code latency: green <100ms, amber 100–500ms, red >500ms
              latencyMs < 100
                ? 'text-[var(--sage)]'
                : latencyMs < 500
                  ? 'text-[var(--warning)]'
                  : 'text-[var(--accent-error)]',
            )}
            title="API round-trip latency"
          >
            {latencyMs}ms
          </span>
        ) : (
          <span className={clsx(baseCls, 'opacity-40')}>—ms</span>
        )}

        {/* Clock — suppress hydration mismatch by not rendering dateTime server-side */}
        <span
          className={baseCls}
          aria-label="Current time"
        >
          {clock}
        </span>
      </div>
    </footer>
  );
}
