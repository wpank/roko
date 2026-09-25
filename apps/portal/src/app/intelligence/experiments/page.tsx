'use client';

import React, { useState, useMemo } from 'react';
import { clsx } from 'clsx';
import { Badge } from '@/components/atoms/Badge';
import { Pill } from '@/components/atoms/Pill';
import { Spinner } from '@/components/atoms/Spinner';
import { useExperiments } from '@/api/hooks';
import type { ExperimentEntry, ExperimentStatus } from '@/api/types';

// ---------------------------------------------------------------------------
// Types and constants
// ---------------------------------------------------------------------------

type StatusFilter = 'active' | 'all' | 'concluded';

const STATUS_LABELS: Record<ExperimentStatus, string> = {
  collecting: 'Collecting',
  trending:   'Trending',
  significant:'Significant',
  concluded:  'Concluded',
};

const STATUS_DOT_COLORS: Record<ExperimentStatus, string> = {
  collecting:  'var(--dream)',
  trending:    'var(--rose)',
  significant: 'var(--sage)',
  concluded:   'var(--text-faint)',
};

const STATUS_BADGE_VARIANTS: Record<ExperimentStatus, 'dream' | 'default' | 'success' | 'info'> = {
  collecting:  'dream',
  trending:    'info',
  significant: 'success',
  concluded:   'default',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const t = new Date(iso).getTime();
  // Epoch sentinel (new Date(0)) means no timestamp was available from the server.
  if (t <= 0) return '—';
  const ms = Date.now() - t;
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

function formatPValue(pValue: number | null): string {
  if (pValue === null) return '—';
  if (pValue < 0.001)  return '<0.001';
  return pValue.toFixed(3);
}

function isActive(status: ExperimentStatus): boolean {
  return status === 'collecting' || status === 'trending';
}

function isConcluded(status: ExperimentStatus): boolean {
  return status === 'concluded';
}

// ---------------------------------------------------------------------------
// Placeholder data (used when the API returns an empty list)
// ---------------------------------------------------------------------------

const PLACEHOLDER_EXPERIMENTS: ExperimentEntry[] = [
  {
    id:            'exp-1',
    name:          'system-prompt-length',
    status:        'collecting',
    variantA:      'concise (200 tokens)',
    variantB:      'detailed (800 tokens)',
    sampleCount:   147,
    pValue:        null,
    winnerVariant: null,
    startedAt:     new Date(Date.now() - 86_400_000 * 3).toISOString(),
  },
  {
    id:            'exp-2',
    name:          'gate-threshold-adaptive',
    status:        'trending',
    variantA:      'fixed thresholds',
    variantB:      'EMA-adaptive thresholds',
    sampleCount:   312,
    pValue:        0.062,
    winnerVariant: null,
    startedAt:     new Date(Date.now() - 86_400_000 * 7).toISOString(),
  },
  {
    id:            'exp-3',
    name:          'cascade-ucb-vs-confidence',
    status:        'significant',
    variantA:      'confidence-interval routing',
    variantB:      'UCB routing',
    sampleCount:   891,
    pValue:        0.021,
    winnerVariant: 'b',
    startedAt:     new Date(Date.now() - 86_400_000 * 14).toISOString(),
  },
  {
    id:            'exp-4',
    name:          'dream-cycle-frequency',
    status:        'concluded',
    variantA:      '24h cycle interval',
    variantB:      '12h cycle interval',
    sampleCount:   1402,
    pValue:        0.004,
    winnerVariant: 'b',
    startedAt:     new Date(Date.now() - 86_400_000 * 30).toISOString(),
  },
  {
    id:            'exp-5',
    name:          'context-bidder-weights',
    status:        'collecting',
    variantA:      'equal weights',
    variantB:      'task-priority weighted',
    sampleCount:   83,
    pValue:        null,
    winnerVariant: null,
    startedAt:     new Date(Date.now() - 86_400_000 * 1).toISOString(),
  },
];

// ---------------------------------------------------------------------------
// Win rate bar — shows relative A vs B performance
// ---------------------------------------------------------------------------

function WinRateBar({
  experiment,
  expanded,
}: {
  experiment: ExperimentEntry;
  expanded: boolean;
}) {
  // Use real per-variant success rates when available; fall back to derived
  // placeholder values based on winner for experiments without rate data.
  const hasRealRates =
    experiment.variantARate !== undefined && experiment.variantBRate !== undefined;

  let aRate: number;
  let bRate: number;
  if (hasRealRates) {
    aRate = experiment.variantARate!;
    bRate = experiment.variantBRate!;
  } else {
    const hasWinner = experiment.winnerVariant !== null;
    const winnerIsB = experiment.winnerVariant === 'b';
    aRate = hasWinner ? (winnerIsB ? 0.42 : 0.61) : 0.50;
    bRate = hasWinner ? (winnerIsB ? 0.58 : 0.39) : 0.50;
  }

  const aColor = experiment.winnerVariant === 'a' ? 'var(--sage)' : 'var(--text-faint)';
  const bColor = experiment.winnerVariant === 'b' ? 'var(--rose-glow)' : 'var(--dream-bright)';

  // Normalise rates so the bar fills 100% (show relative split, not absolute)
  const total = aRate + bRate;
  const aPct = total > 0 ? (aRate / total) * 100 : 50;
  const bPct = total > 0 ? (bRate / total) * 100 : 50;

  return (
    <div className={clsx('flex flex-col gap-1', expanded ? '' : '')}>
      {/* Labels */}
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs" style={{ color: aColor }}>
          A&nbsp;
          <span className="num tabular-nums">{(aRate * 100).toFixed(1)}%</span>
        </span>
        <span className="font-mono text-xs" style={{ color: bColor }}>
          <span className="num tabular-nums">{(bRate * 100).toFixed(1)}%</span>
          &nbsp;B
        </span>
      </div>

      {/* Bar */}
      <div className="flex h-1.5 w-full bg-bg-highlight overflow-hidden">
        <div style={{ width: `${aPct}%`, backgroundColor: aColor, transition: 'width 300ms ease-out' }} />
        <div style={{ width: `${bPct}%`, backgroundColor: bColor, transition: 'width 300ms ease-out' }} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Experiment row (collapsed + expanded detail)
// ---------------------------------------------------------------------------

function ExperimentRow({ experiment }: { experiment: ExperimentEntry }) {
  const [expanded, setExpanded] = useState(false);

  const dotColor     = STATUS_DOT_COLORS[experiment.status];
  const badgeVariant = STATUS_BADGE_VARIANTS[experiment.status];

  return (
    <>
      {/* Main row */}
      <div
        role="row"
        tabIndex={0}
        onClick={() => setExpanded((v) => !v)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setExpanded((v) => !v);
          }
        }}
        className={clsx(
          'flex items-center gap-4 px-4 py-2.5',
          'border-b border-b-[var(--text-ghost)]',
          'cursor-pointer hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
          'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)] focus-visible:ring-inset',
          expanded && 'bg-[var(--bg-highlight)]',
        )}
        aria-expanded={expanded}
      >
        {/* Expand toggle */}
        <span
          className="shrink-0 font-mono text-xs text-text-ghost transition-transform duration-[150ms]"
          style={{ transform: expanded ? 'rotate(0deg)' : 'rotate(-90deg)' }}
        >
          ▾
        </span>

        {/* Name */}
        <span className="flex-1 min-w-0 font-mono text-xs text-text-strong truncate">
          {experiment.name}
        </span>

        {/* Win rate bar (compact) */}
        <div className="shrink-0 w-28 hidden sm:block">
          <WinRateBar experiment={experiment} expanded={false} />
        </div>

        {/* Status dot + badge */}
        <div className="shrink-0 flex items-center gap-1.5">
          <span
            className="w-1.5 h-1.5 shrink-0"
            style={{
              backgroundColor: dotColor,
              boxShadow: isActive(experiment.status) ? `0 0 5px ${dotColor}` : undefined,
            }}
          />
          <Badge variant={badgeVariant}>
            {STATUS_LABELS[experiment.status]}
          </Badge>
        </div>

        {/* Sample count */}
        <span className="shrink-0 w-14 font-mono text-xs num tabular-nums text-right text-text-faint">
          {(experiment.sampleCount ?? 0).toLocaleString()}
        </span>

        {/* p-value */}
        <span
          className={clsx(
            'shrink-0 w-12 font-mono text-xs num tabular-nums text-right',
            experiment.pValue !== null && experiment.pValue < 0.05
              ? 'text-sage'
              : 'text-text-ghost',
          )}
        >
          {formatPValue(experiment.pValue)}
        </span>

        {/* Started date */}
        <span className="shrink-0 w-16 font-mono text-xs num tabular-nums text-right text-text-ghost">
          {formatRelativeTime(experiment.startedAt)}
        </span>
      </div>

      {/* Expanded detail panel */}
      {expanded && (
        <div
          className={clsx(
            'border-b border-b-[var(--text-ghost)]',
            'bg-[var(--bg-raised)]',
          )}
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 px-8 py-4">
            {/* Variants */}
            <div className="flex flex-col gap-3">
              <span className="font-mono text-xs text-text-ghost uppercase tracking-widest">Variants</span>

              {/* Use allVariants when >2 exist, otherwise fall back to A/B pair */}
              {(experiment.allVariants && experiment.allVariants.length > 2
                ? experiment.allVariants.map((v, idx) => ({
                    key: v.id,
                    label: v.name,
                    isWinner: experiment.winnerVariant != null
                      ? (idx === 0 && experiment.winnerVariant === 'a') ||
                        (idx === 1 && experiment.winnerVariant === 'b')
                      : false,
                    successRate: v.successRate,
                    trials: v.trials,
                    tag: String.fromCharCode(65 + idx), // A, B, C…
                  }))
                : (['a', 'b'] as const).map((variant) => ({
                    key: variant,
                    label: variant === 'a' ? experiment.variantA : experiment.variantB,
                    isWinner: experiment.winnerVariant === variant,
                    successRate: variant === 'a' ? experiment.variantARate : experiment.variantBRate,
                    trials: undefined as number | undefined,
                    tag: variant.toUpperCase(),
                  }))
              ).map((v) => (
                <div key={v.key} className="flex items-start gap-2">
                  <span
                    className="shrink-0 w-5 h-5 flex items-center justify-center border font-mono text-xs font-medium"
                    style={{
                      borderColor: v.isWinner ? 'var(--sage)' : 'var(--text-ghost)',
                      color:       v.isWinner ? 'var(--sage)' : 'var(--text-ghost)',
                    }}
                  >
                    {v.tag}
                  </span>
                  <div className="flex flex-col gap-0.5 min-w-0">
                    <span className={clsx(
                      'font-mono text-xs leading-relaxed',
                      v.isWinner ? 'text-text-strong' : 'text-text-muted',
                    )}>
                      {v.label}
                      {v.isWinner && (
                        <span className="ml-1.5 font-mono text-xs" style={{ color: 'var(--sage)' }}>
                          ← winner
                        </span>
                      )}
                    </span>
                    {v.successRate !== undefined && (
                      <span className="font-mono text-xs text-text-ghost num tabular-nums">
                        {((v.successRate ?? 0) * 100).toFixed(1)}% success
                        {v.trials !== undefined && ` · ${v.trials} trials`}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Statistics */}
            <div className="flex flex-col gap-3">
              <span className="font-mono text-xs text-text-ghost uppercase tracking-widest">Statistics</span>

              <div className="grid grid-cols-2 gap-y-2 gap-x-4">
                {[
                  { label: 'Samples',   value: (experiment.sampleCount ?? 0).toLocaleString() },
                  { label: 'p-value',   value: formatPValue(experiment.pValue) },
                  { label: 'Status',    value: STATUS_LABELS[experiment.status] },
                  { label: 'Winner',    value: experiment.winnerVariant ? `Variant ${experiment.winnerVariant.toUpperCase()}` : 'TBD' },
                  { label: 'Started',   value: formatRelativeTime(experiment.startedAt) },
                ].map(({ label, value }) => (
                  <div key={label} className="flex flex-col gap-0.5">
                    <span className="font-mono text-xs text-text-ghost">{label}</span>
                    <span className="font-mono text-xs text-text-muted num tabular-nums">{value}</span>
                  </div>
                ))}
              </div>

              {/* Win rate bar */}
              <div className="mt-1">
                <WinRateBar experiment={experiment} expanded={true} />
              </div>

              {/* Significance indicator */}
              {experiment.pValue !== null && (
                <div className="flex items-center gap-1.5">
                  <span
                    className="w-1.5 h-1.5"
                    style={{ backgroundColor: experiment.pValue < 0.05 ? 'var(--sage)' : 'var(--warning)' }}
                  />
                  <span
                    className="font-mono text-xs"
                    style={{ color: experiment.pValue < 0.05 ? 'var(--sage)' : 'var(--warning)' }}
                  >
                    {experiment.pValue < 0.05
                      ? `Statistically significant (p=${formatPValue(experiment.pValue)})`
                      : `Not yet significant (p=${formatPValue(experiment.pValue)}, need <0.05)`}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

function EmptyState({ filter }: { filter: StatusFilter }) {
  const messages: Record<StatusFilter, string> = {
    active:    'No active experiments right now.',
    all:       'No experiments have been recorded yet.',
    concluded: 'No experiments have concluded yet.',
  };

  return (
    <div className="flex flex-col items-center gap-3 py-16 px-4">
      <span className="font-mono text-2xl text-text-ghost">◎</span>
      <span className="font-mono text-sm text-text-muted">{messages[filter]}</span>
      <span className="font-mono text-xs text-text-ghost">
        Run plans to generate experiment data.
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ExperimentsPage() {
  const [filter, setFilter] = useState<StatusFilter>('active');
  const { data, isLoading, isError } = useExperiments();

  // Use live data if available, otherwise fall back to placeholders for dev
  const allExperiments = useMemo(
    () => (data && data.length > 0 ? data : PLACEHOLDER_EXPERIMENTS),
    [data],
  );

  const displayExperiments = useMemo(() => {
    switch (filter) {
      case 'active':
        return allExperiments.filter((e) => isActive(e.status));
      case 'concluded':
        return allExperiments.filter((e) => isConcluded(e.status));
      default:
        return allExperiments;
    }
  }, [allExperiments, filter]);

  const activeCounts = useMemo(() => ({
    active:    allExperiments.filter((e) => isActive(e.status)).length,
    all:       allExperiments.length,
    concluded: allExperiments.filter((e) => isConcluded(e.status)).length,
  }), [allExperiments]);

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* ---- Toolbar ---- */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)] shrink-0">
        <span className="font-mono text-xs text-text-ghost">Filter:</span>

        {(['active', 'all', 'concluded'] as StatusFilter[]).map((f) => (
          <Pill key={f} active={filter === f} onClick={() => setFilter(f)}>
            {f === 'active' ? `Active (${activeCounts.active})` :
             f === 'all'    ? `All (${activeCounts.all})` :
                              `Concluded (${activeCounts.concluded})`}
          </Pill>
        ))}

        {isLoading && (
          <span className="ml-auto">
            <Spinner size="sm" />
          </span>
        )}
      </div>

      {/* ---- Table ---- */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">

        {/* Column headers */}
        <div
          role="row"
          className="flex items-center gap-4 px-4 py-1.5 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)] sticky top-0 z-10"
        >
          {/* expand toggle placeholder */}
          <span className="shrink-0 w-3" />

          <span className="flex-1 font-mono text-xs text-text-ghost">name</span>

          <span className="shrink-0 w-28 font-mono text-xs text-text-ghost hidden sm:block">
            variant A / B
          </span>

          <span className="shrink-0 font-mono text-xs text-text-ghost w-24">status</span>

          <span className="shrink-0 w-14 font-mono text-xs text-text-ghost text-right">samples</span>

          <span className="shrink-0 w-12 font-mono text-xs text-text-ghost text-right">p-val</span>

          <span className="shrink-0 w-16 font-mono text-xs text-text-ghost text-right">started</span>
        </div>

        {isLoading ? (
          <div className="flex items-center justify-center py-16">
            <Spinner size="md" />
          </div>
        ) : isError ? (
          <div className="flex items-center justify-center py-16">
            <span className="font-mono text-xs text-accent-error">
              Failed to load experiments.
            </span>
          </div>
        ) : displayExperiments.length === 0 ? (
          <EmptyState filter={filter} />
        ) : (
          <div role="table" aria-label="Experiments">
            {displayExperiments.map((experiment) => (
              <ExperimentRow key={experiment.id} experiment={experiment} />
            ))}
          </div>
        )}
      </div>

      {/* ---- Footer: summary stats ---- */}
      {!isLoading && !isError && allExperiments.length > 0 && (
        <div className="shrink-0 border-t border-t-[var(--text-ghost)] px-4 py-2 bg-[var(--bg-raised)] flex items-center gap-6">
          {[
            { label: 'Active',    value: activeCounts.active,    color: 'var(--dream)' },
            { label: 'Concluded', value: activeCounts.concluded, color: 'var(--text-faint)' },
            {
              label: 'Significant',
              value: allExperiments.filter((e) => e.pValue !== null && e.pValue < 0.05).length,
              color: 'var(--sage)',
            },
          ].map(({ label, value, color }) => (
            <div key={label} className="flex items-center gap-1.5">
              <span className="font-mono text-xs text-text-ghost">{label}:</span>
              <span className="font-mono text-xs num tabular-nums font-medium" style={{ color }}>
                {value}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
