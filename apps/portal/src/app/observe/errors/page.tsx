'use client';

import React, { useState, useMemo } from 'react';
import { clsx } from 'clsx';
import { Badge, Button } from '@/components/atoms';
import { useDashboardStore } from '@/stores/dashboard';
import type { ErrorEntry, Severity } from '@/api/types';

// ---------------------------------------------------------------------------
// Mock errors (used when the store is empty, i.e. no live connection yet)
// ---------------------------------------------------------------------------

const MOCK_ERRORS: ErrorEntry[] = [
  {
    id: 'err-001',
    source: 'gate/rung-2',
    message: 'test suite failed: 1 test failed in roko-agent — thread "test_dispatch_with_mock_provider" panicked at "assertion failed: result.is_ok()"',
    severity: 'high',
    timestamp: new Date(Date.now() - 4 * 60_000).toISOString(),
    planId: 'auth-layer',
    taskId: 'task-implement-jwt',
  },
  {
    id: 'err-002',
    source: 'agent/reviewer-01',
    message: 'Context window exhausted mid-turn. Turn truncated at 196,608 tokens. Agent could not complete review of diff.',
    severity: 'medium',
    timestamp: new Date(Date.now() - 12 * 60_000).toISOString(),
    planId: 'auth-layer',
    taskId: 'task-review-auth',
  },
  {
    id: 'err-003',
    source: 'provider/anthropic',
    message: 'Rate limit exceeded (429). Retried 3 times with exponential back-off. Circuit breaker half-open.',
    severity: 'critical',
    timestamp: new Date(Date.now() - 18 * 60_000).toISOString(),
  },
  {
    id: 'err-004',
    source: 'system/disk-gc',
    message: 'Disk admission check failed: workspace has < 500 MB free. Worktree creation blocked until GC completes.',
    severity: 'high',
    timestamp: new Date(Date.now() - 31 * 60_000).toISOString(),
  },
  {
    id: 'err-005',
    source: 'gate/rung-1',
    message: 'clippy --workspace -- -D warnings: 3 unused import warnings promoted to errors',
    severity: 'medium',
    timestamp: new Date(Date.now() - 45 * 60_000).toISOString(),
    planId: 'cache-impl',
    taskId: 'task-add-redis',
  },
  {
    id: 'err-006',
    source: 'agent/planner-01',
    message: 'PRD plan generation timed out after 120 s. No tasks.toml was written.',
    severity: 'medium',
    timestamp: new Date(Date.now() - 58 * 60_000).toISOString(),
  },
  {
    id: 'err-007',
    source: 'system/budget',
    message: 'Plan "big-refactor" exceeded $5.00 budget cap. All pending tasks cancelled.',
    severity: 'critical',
    timestamp: new Date(Date.now() - 73 * 60_000).toISOString(),
    planId: 'big-refactor',
  },
  {
    id: 'err-008',
    source: 'system/mcp',
    message: 'MCP server "roko-mcp-code" exited unexpectedly (exit code 1). Tools will be unavailable until restart.',
    severity: 'low',
    timestamp: new Date(Date.now() - 2 * 3_600_000).toISOString(),
  },
];

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SEVERITY_ORDER: Severity[] = ['critical', 'high', 'medium', 'low'];

const SEVERITY_BADGE_VARIANT: Record<Severity, 'error' | 'warning' | 'info' | 'default'> = {
  critical: 'error',
  high:     'warning',
  medium:   'info',
  low:      'default',
};

const SEVERITY_BG: Record<Severity, string> = {
  critical: 'bg-[rgba(196,31,31,0.08)]',
  high:     'bg-[rgba(192,160,64,0.06)]',
  medium:   'bg-transparent',
  low:      'bg-transparent',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

function groupBySeverity(errors: ErrorEntry[]): Map<Severity, ErrorEntry[]> {
  const map = new Map<Severity, ErrorEntry[]>();
  for (const s of SEVERITY_ORDER) map.set(s, []);
  for (const e of errors) {
    const bucket = map.get(e.severity as Severity) ?? map.get('low')!;
    bucket.push(e);
  }
  return map;
}

// ---------------------------------------------------------------------------
// Error row
// ---------------------------------------------------------------------------

interface ErrorRowProps {
  error: ErrorEntry;
}

function ErrorRow({ error }: ErrorRowProps) {
  const [expanded, setExpanded] = useState(false);
  const badgeVariant = SEVERITY_BADGE_VARIANT[error.severity as Severity] ?? 'default';

  return (
    <div
      className={clsx(
        'border-b border-b-[var(--text-ghost)]',
        SEVERITY_BG[error.severity as Severity],
      )}
    >
      {/* Summary row */}
      <div
        role="button"
        tabIndex={0}
        onClick={() => setExpanded((x) => !x)}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') setExpanded((x) => !x); }}
        className={clsx(
          'flex items-start gap-3 px-4 py-2.5',
          'cursor-pointer',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms]',
          'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
        )}
      >
        {/* Severity badge */}
        <div className="shrink-0 pt-px">
          <Badge variant={badgeVariant}>
            {error.severity}
          </Badge>
        </div>

        {/* Source */}
        <span className="shrink-0 font-mono text-xs text-[var(--text-muted)] leading-snug w-36 truncate pt-px">
          {error.source}
        </span>

        {/* Message — truncated in collapsed state */}
        <span
          className={clsx(
            'flex-1 font-mono text-xs text-[var(--text-strong)] leading-snug min-w-0',
            !expanded && 'truncate',
          )}
        >
          {error.message}
        </span>

        {/* Timestamp */}
        <span className="shrink-0 font-mono text-xs text-[var(--text-ghost)] tabular-nums leading-snug pt-px ml-2">
          {formatRelativeTime(error.timestamp)}
        </span>

        {/* Expand chevron */}
        <span
          className={clsx(
            'shrink-0 font-mono text-xs text-[var(--text-faint)] leading-none select-none pt-0.5',
            'transition-transform duration-[80ms]',
            expanded && 'rotate-90',
          )}
          aria-hidden
        >
          ›
        </span>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div
          className={clsx(
            'px-4 pb-4 pt-1',
            'bg-[var(--bg-secondary)]',
            'border-t border-t-[var(--text-ghost)]',
          )}
        >
          {/* Full message */}
          <pre
            className={clsx(
              'font-mono text-xs text-[var(--text-muted)] whitespace-pre-wrap break-words',
              'leading-relaxed mb-3',
            )}
          >
            {error.message}
          </pre>

          {/* Metadata grid */}
          <div className="flex flex-wrap gap-x-6 gap-y-1.5">
            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Source
              </span>
              <span className="font-mono text-xs text-[var(--text-faint)]">{error.source}</span>
            </div>

            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Time
              </span>
              <span className="font-mono text-xs text-[var(--text-faint)] tabular-nums">
                {new Date(error.timestamp).toLocaleString()}
              </span>
            </div>

            {error.planId && (
              <div className="flex items-center gap-2">
                <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                  Plan
                </span>
                <span className="font-mono text-xs text-[var(--rose)]">{error.planId}</span>
              </div>
            )}

            {error.taskId && (
              <div className="flex items-center gap-2">
                <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                  Task
                </span>
                <span className="font-mono text-xs text-[var(--accent-cyan)]">{error.taskId}</span>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section header
// ---------------------------------------------------------------------------

function SectionHeader({ severity, count }: { severity: Severity; count: number }) {
  const variant = SEVERITY_BADGE_VARIANT[severity];

  return (
    <div
      className={clsx(
        'flex items-center gap-2 px-4 py-1.5',
        'bg-[var(--bg-secondary)]',
        'border-b border-b-[var(--text-ghost)]',
        'sticky top-0 z-10',
      )}
    >
      <Badge variant={variant}>{severity}</Badge>
      <span className="font-mono text-xs text-[var(--text-ghost)] tabular-nums">
        {count} {count === 1 ? 'error' : 'errors'}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveErrorsPage() {
  const storeErrors = useDashboardStore((s) => s.recentErrors);

  // Use live store data when available, fall back to mock data for UI dev
  const errors: ErrorEntry[] = storeErrors.length > 0 ? storeErrors : MOCK_ERRORS;

  const grouped = useMemo(() => groupBySeverity(errors), [errors]);

  const totalCritical = grouped.get('critical')?.length ?? 0;
  const totalHigh     = grouped.get('high')?.length     ?? 0;
  const totalErrors   = errors.length;

  return (
    <div className="flex flex-col min-h-0">

      {/* ---------------------------------------------------------------- */}
      {/* Page header / summary strip                                       */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'flex items-center gap-6 px-4 py-3 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Total
          </span>
          <span className="font-mono text-lg tabular-nums text-[var(--text-strong)]">
            {totalErrors}
          </span>
        </div>

        <span className="w-px h-8 bg-[var(--text-ghost)]" aria-hidden />

        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Critical
          </span>
          <span
            className="font-mono text-lg tabular-nums"
            style={{ color: totalCritical > 0 ? 'var(--accent-error)' : 'var(--text-ghost)' }}
          >
            {totalCritical}
          </span>
        </div>

        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            High
          </span>
          <span
            className="font-mono text-lg tabular-nums"
            style={{ color: totalHigh > 0 ? 'var(--warning)' : 'var(--text-ghost)' }}
          >
            {totalHigh}
          </span>
        </div>

        <div className="ml-auto flex items-center gap-2">
          <Button variant="ghost" size="sm">
            Clear all
          </Button>
        </div>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Error groups                                                       */}
      {/* ---------------------------------------------------------------- */}
      {totalErrors === 0 ? (
        <div className="flex flex-col items-center justify-center gap-3 py-24">
          <span
            className="font-mono text-3xl"
            style={{ color: 'var(--sage)' }}
            aria-hidden
          >
            ✓
          </span>
          <span className="font-mono text-sm text-[var(--text-muted)]">
            No errors recorded.
          </span>
        </div>
      ) : (
        SEVERITY_ORDER.map((severity) => {
          const bucket = grouped.get(severity) ?? [];
          if (bucket.length === 0) return null;
          return (
            <div key={severity}>
              <SectionHeader severity={severity} count={bucket.length} />
              {bucket.map((error) => (
                <ErrorRow key={error.id} error={error} />
              ))}
            </div>
          );
        })
      )}
    </div>
  );
}
