'use client';

import React, { useState, useMemo } from 'react';
import { clsx } from 'clsx';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import { Badge } from '@/components/atoms/Badge';
import { Button } from '@/components/atoms/Button';
import { ProgressBar } from '@/components/atoms/ProgressBar';
import { Spinner } from '@/components/atoms/Spinner';
import { StatusLED } from '@/components/atoms/StatusLED';
import type { DreamPhase, DreamJournalEntry } from '@/api/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface DreamTrigger {
  id: string;
  type: 'idle' | 'cron' | 'episode-count';
  schedule: string;
  lastRunAt: string | null;
  enabled: boolean;
}

interface DreamStatus {
  activeCycle: {
    id:        string;
    phase:     DreamPhase;
    startedAt: string;
  } | null;
  lastCycleAt:      string | null;
  archivedCount:    number;
  activeCount:      number;
  totalCycles:      number;
  triggers:         DreamTrigger[];
}

interface DreamJournal {
  entries:    DreamJournalEntry[];
  totalCount: number;
}

type SortField = 'timestamp' | 'promoted' | 'demoted' | 'duration';
type SortDir   = 'asc' | 'desc';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PHASES: DreamPhase[] = ['hypnagogia', 'imagination', 'consolidation'];

const PHASE_LABEL: Record<DreamPhase, string> = {
  hypnagogia:    'Hypnagogia',
  imagination:   'Imagination',
  consolidation: 'Consolidation',
};

const PHASE_DESC: Record<DreamPhase, string> = {
  hypnagogia:    'Scanning active signals for consolidation candidates',
  imagination:   'Generating synthetic cross-domain associations',
  consolidation: 'Persisting promoted entries, demoting stale ones',
};

const PHASE_COLOR: Record<DreamPhase, string> = {
  hypnagogia:    'var(--warning)',
  imagination:   'var(--dream)',
  consolidation: 'var(--rose-dim)',
};

const TRIGGER_TYPE_ICON: Record<DreamTrigger['type'], string> = {
  idle:          '◷',
  cron:          '⏱',
  'episode-count': '#',
};

// ---------------------------------------------------------------------------
// Mock / placeholder factories (used when the API is unavailable)
// ---------------------------------------------------------------------------

function makeMockStatus(): DreamStatus {
  return {
    activeCycle: null,
    lastCycleAt: new Date(Date.now() - 3_600_000).toISOString(),
    archivedCount: 1_204,
    activeCount:   3_891,
    totalCycles:   47,
    triggers: [
      {
        id:        'trig-1',
        type:      'idle',
        schedule:  '30m idle',
        lastRunAt: new Date(Date.now() - 3_600_000).toISOString(),
        enabled:   true,
      },
      {
        id:        'trig-2',
        type:      'cron',
        schedule:  '0 3 * * *',
        lastRunAt: new Date(Date.now() - 86_400_000).toISOString(),
        enabled:   true,
      },
      {
        id:        'trig-3',
        type:      'episode-count',
        schedule:  'every 50 episodes',
        lastRunAt: null,
        enabled:   false,
      },
    ],
  };
}

function makeMockJournal(): DreamJournal {
  const now = Date.now();
  const entries: DreamJournalEntry[] = Array.from({ length: 12 }, (_, i) => ({
    id:             `dream-${i + 1}`,
    phase:          PHASES[i % 3],
    promotedCount:  Math.floor(Math.random() * 20),
    demotedCount:   Math.floor(Math.random() * 5),
    createdCount:   Math.floor(Math.random() * 8),
    insightsGained: Math.floor(Math.random() * 4),
    durationMs:     Math.floor(Math.random() * 120_000) + 5_000,
    timestamp:      new Date(now - i * 3_600_000 * 4).toISOString(),
  }));
  return { entries, totalCount: entries.length };
}

// ---------------------------------------------------------------------------
// React Query hooks
//
// The server exposes:
//   GET  /api/dream/journal  → { last_cycle, cycle_count, phases[] }
//   POST /api/dream/run      → { id: string }
//
// There is no /api/dream/status endpoint — we derive DreamStatus from the
// journal response, and no /api/dream/triggers endpoint exists in roko-serve.
// ---------------------------------------------------------------------------

/** Raw shape returned by GET /api/dream/journal */
interface RawDreamJournal {
  last_cycle:  string;
  cycle_count: number;
  phases: Array<{
    name:                    string;
    status:                  string;
    episodes_processed:      number;
    clusters_formed:         number;
    knowledge_entries_written: number;
    playbooks_created:       number;
    duration_secs:           number;
    trend:                   unknown[];
  }>;
}

/** Map /api/dream/journal → DreamStatus (synthesised) */
function mapJournalToStatus(raw: RawDreamJournal): DreamStatus {
  const totalCycles  = raw.cycle_count ?? 0;
  const lastCycleAt  = raw.last_cycle ? raw.last_cycle : null;

  // Derive archived / active counts from the phase stats
  const totalPromoted = (raw.phases ?? []).reduce(
    (s, p) => s + (p.knowledge_entries_written ?? 0), 0,
  );

  return {
    activeCycle:   null,  // SSE stream drives live phase; journal is historical
    lastCycleAt,
    archivedCount: totalPromoted,
    activeCount:   0,
    totalCycles,
    // No trigger data available from this endpoint
    triggers: [],
  };
}

/** Map /api/dream/journal → DreamJournal */
function mapJournalToJournal(raw: RawDreamJournal): DreamJournal {
  const phases = raw.phases ?? [];
  const totalCycles = raw.cycle_count ?? 0;

  // Each phase in the latest cycle becomes one DreamJournalEntry so that the
  // history table has real data to show.
  const entries: DreamJournalEntry[] = phases.map((p, i) => {
    const phaseLabel = p.name?.toLowerCase() ?? 'hypnagogia';
    const phase: DreamPhase =
      phaseLabel.includes('nrem') || phaseLabel.includes('hypnagogia') ? 'hypnagogia' :
      phaseLabel.includes('rem')  || phaseLabel.includes('imagination')  ? 'imagination' :
      'consolidation';

    return {
      id:             `phase-${i}-${p.name}`,
      phase,
      promotedCount:  p.knowledge_entries_written ?? 0,
      demotedCount:   0,
      createdCount:   p.playbooks_created ?? 0,
      insightsGained: p.clusters_formed ?? 0,
      durationMs:     (p.duration_secs ?? 0) * 1_000,
      timestamp:      raw.last_cycle || new Date().toISOString(),
    };
  });

  return {
    entries,
    totalCount: totalCycles,
  };
}

function useDreamStatus() {
  return useQuery<DreamStatus>({
    queryKey:        ['dreams', 'status'],
    queryFn:         async () => {
      const raw = await api.get<RawDreamJournal>('/api/dream/journal');
      return mapJournalToStatus(raw);
    },
    staleTime:       15_000,
    refetchInterval: 30_000,
    placeholderData: makeMockStatus(),
  });
}

function useDreamJournal() {
  return useQuery<DreamJournal>({
    queryKey:        ['dreams', 'journal'],
    queryFn:         async () => {
      const raw = await api.get<RawDreamJournal>('/api/dream/journal');
      return mapJournalToJournal(raw);
    },
    staleTime:       30_000,
    placeholderData: makeMockJournal(),
  });
}

function useRunDreamCycle() {
  const qc = useQueryClient();
  return useMutation<{ cycleId: string }, Error, void>({
    // POST /api/dream/run returns { id: string } — we normalise to cycleId
    mutationFn: async () => {
      const res = await api.post<{ id: string }>('/api/dream/run');
      return { cycleId: res.id ?? '' };
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['dreams', 'status']  });
      void qc.invalidateQueries({ queryKey: ['dreams', 'journal'] });
    },
  });
}

function useToggleTrigger() {
  const qc = useQueryClient();
  // No trigger-management endpoint exists in roko-serve; this is a no-op stub
  // that keeps the UI functional without throwing.
  return useMutation<unknown, Error, { triggerId: string; enabled: boolean }>({
    mutationFn: (_params) => Promise.resolve(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['dreams', 'status'] });
    },
  });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTs(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString('en-US', {
      month:  'short',
      day:    'numeric',
      hour:   '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return iso;
  }
}

function formatRelativeTime(iso: string | null): string {
  if (!iso) return '—';
  try {
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 60_000) return 'just now';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
  } catch {
    return iso;
  }
}

function formatDuration(ms: number): string {
  if (ms < 1_000)     return `${ms}ms`;
  if (ms < 60_000)    return `${(ms / 1_000).toFixed(1)}s`;
  const m = Math.floor(ms / 60_000);
  const s = Math.floor((ms % 60_000) / 1_000);
  return `${m}m${s.toString().padStart(2, '0')}s`;
}

// ---------------------------------------------------------------------------
// Phase stepper
// ---------------------------------------------------------------------------

function PhaseStepper({
  activePhase,
}: {
  activePhase: DreamPhase | null;
}) {
  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)] bg-[var(--bg-raised)]',
      )}
    >
      {/* Header */}
      <div
        className={clsx(
          'flex items-center gap-2 px-4 py-2',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
          Dream Phase
        </span>
        {activePhase && (
          <div className="ml-auto flex items-center gap-1.5">
            <StatusLED status="active" pulse size="sm" />
            <span className="font-mono text-[10px] text-[var(--rose)]">
              Running
            </span>
          </div>
        )}
      </div>

      {/* Steps */}
      <div className="flex items-stretch p-4 gap-0">
        {PHASES.map((phase, idx) => {
          const isActive    = phase === activePhase;
          const isCompleted = activePhase !== null &&
            PHASES.indexOf(activePhase) > idx;
          const phaseColor  = PHASE_COLOR[phase];
          const isLast      = idx === PHASES.length - 1;

          return (
            <React.Fragment key={phase}>
              {/* Step */}
              <div className="flex flex-col items-center gap-2 flex-1 min-w-0">
                {/* Circle / indicator */}
                <div className="relative flex items-center justify-center">
                  <div
                    className={clsx(
                      'w-8 h-8 flex items-center justify-center border-2',
                      'font-mono text-xs font-medium leading-none',
                      'transition-[border-color,background-color,color] duration-300',
                    )}
                    style={{
                      borderColor: isActive || isCompleted ? phaseColor : 'var(--text-ghost)',
                      backgroundColor: isActive
                        ? `color-mix(in srgb, ${phaseColor} 15%, transparent)`
                        : 'transparent',
                      color: isActive || isCompleted ? phaseColor : 'var(--text-ghost)',
                      ...(isActive && {
                        animation: 'rd-led-pulse 2s ease-in-out infinite',
                        boxShadow: `0 0 6px 2px ${phaseColor}`,
                      }),
                    }}
                  >
                    {isCompleted ? '✓' : idx + 1}
                  </div>
                </div>

                {/* Label + description */}
                <div className="text-center px-1">
                  <div
                    className="font-mono text-[10px] font-medium uppercase tracking-widest leading-none mb-1"
                    style={{
                      color: isActive || isCompleted
                        ? phaseColor
                        : 'var(--text-ghost)',
                    }}
                  >
                    {PHASE_LABEL[phase]}
                  </div>
                  <div className="font-mono text-[10px] text-[var(--text-faint)] leading-snug hidden sm:block">
                    {PHASE_DESC[phase]}
                  </div>
                </div>
              </div>

              {/* Connector */}
              {!isLast && (
                <div className="flex items-start pt-4 shrink-0 px-2">
                  <div
                    className="w-8 h-px mt-px"
                    style={{
                      backgroundColor: isCompleted
                        ? phaseColor
                        : 'var(--text-ghost)',
                    }}
                  />
                </div>
              )}
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Trigger table
// ---------------------------------------------------------------------------

function TriggerTable({ triggers }: { triggers: DreamTrigger[] }) {
  const toggleMut = useToggleTrigger();

  return (
    <div className="border border-[var(--text-ghost)] bg-[var(--bg-raised)]">
      {/* Header */}
      <div
        className={clsx(
          'flex items-center gap-2 px-4 py-2',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
          Trigger Configuration
        </span>
      </div>

      {/* Table */}
      {triggers.length === 0 ? (
        <div className="flex items-center justify-center py-6">
          <span className="font-mono text-xs text-[var(--text-ghost)]">
            No triggers configured.
          </span>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[480px] border-collapse">
            <thead>
              <tr className="border-b border-b-[var(--text-ghost)]">
                {(['Type', 'Schedule / Threshold', 'Last Run', 'Enabled'] as const).map((col) => (
                  <th
                    key={col}
                    className={clsx(
                      'px-4 py-2 text-left',
                      'font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]',
                      col === 'Enabled' && 'text-right',
                    )}
                  >
                    {col}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {triggers.map((trig) => (
                <tr
                  key={trig.id}
                  className={clsx(
                    'border-b border-b-[var(--text-ghost)]',
                    'hover:bg-[var(--bg-highlight)]',
                    'transition-[background-color] duration-[80ms]',
                  )}
                >
                  {/* Type */}
                  <td className="px-4 py-2">
                    <div className="flex items-center gap-2">
                      <span
                        className="font-mono text-base text-[var(--text-ghost)]"
                        aria-hidden
                      >
                        {TRIGGER_TYPE_ICON[trig.type]}
                      </span>
                      <span className="font-mono text-xs text-[var(--text-faint)] uppercase">
                        {trig.type}
                      </span>
                    </div>
                  </td>

                  {/* Schedule */}
                  <td className="px-4 py-2 font-mono text-xs text-[var(--text-muted)]">
                    {trig.schedule}
                  </td>

                  {/* Last run */}
                  <td className="px-4 py-2 font-mono text-[10px] text-[var(--text-ghost)]">
                    {formatRelativeTime(trig.lastRunAt)}
                  </td>

                  {/* Toggle */}
                  <td className="px-4 py-2 text-right">
                    <button
                      type="button"
                      onClick={() =>
                        toggleMut.mutate({ triggerId: trig.id, enabled: !trig.enabled })
                      }
                      disabled={toggleMut.isPending}
                      aria-pressed={trig.enabled}
                      aria-label={`${trig.enabled ? 'Disable' : 'Enable'} ${trig.type} trigger`}
                      className={clsx(
                        'inline-flex items-center gap-1.5',
                        'font-mono text-[10px] px-2 py-1 border',
                        'transition-[color,border-color,opacity] duration-[80ms]',
                        'disabled:opacity-40 disabled:cursor-not-allowed',
                        trig.enabled
                          ? 'text-[var(--sage)] border-[var(--sage)]'
                          : 'text-[var(--text-ghost)] border-[var(--text-ghost)]',
                      )}
                    >
                      <span
                        className="w-1.5 h-1.5"
                        style={{
                          backgroundColor: trig.enabled
                            ? 'var(--sage)'
                            : 'var(--text-ghost)',
                        }}
                        aria-hidden
                      />
                      {trig.enabled ? 'On' : 'Off'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Journal table
// ---------------------------------------------------------------------------

type SortHeaderProps = {
  field:   SortField;
  label:   string;
  current: SortField;
  dir:     SortDir;
  onSort:  (f: SortField) => void;
};

function SortHeader({ field, label, current, dir, onSort }: SortHeaderProps) {
  const active = field === current;
  return (
    <button
      type="button"
      onClick={() => onSort(field)}
      className={clsx(
        'font-mono text-[10px] uppercase tracking-widest leading-none',
        'transition-colors duration-[80ms]',
        active
          ? 'text-[var(--text-strong)]'
          : 'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
      )}
    >
      {label}
      {active && (dir === 'desc' ? ' ↓' : ' ↑')}
    </button>
  );
}

function JournalRow({
  entry,
  expanded,
  onToggle,
}: {
  entry:    DreamJournalEntry;
  expanded: boolean;
  onToggle: () => void;
}) {
  const phaseColor = PHASE_COLOR[entry.phase];

  return (
    <>
      <tr
        className={clsx(
          'border-b border-b-[var(--text-ghost)]',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms]',
          'cursor-pointer',
        )}
        onClick={onToggle}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') onToggle();
        }}
      >
        {/* Expand chevron */}
        <td className="px-4 py-2 w-6">
          <span
            className={clsx(
              'font-mono text-[10px] text-[var(--text-ghost)]',
              'inline-block transition-transform duration-[80ms]',
              expanded ? 'rotate-90' : 'rotate-0',
            )}
            aria-hidden
          >
            ▶
          </span>
        </td>

        {/* Time */}
        <td className="px-2 py-2 font-mono text-[10px] text-[var(--text-faint)] whitespace-nowrap">
          {formatTs(entry.timestamp)}
        </td>

        {/* Phase */}
        <td className="px-2 py-2">
          <span
            className="font-mono text-[10px] uppercase tracking-widest px-1.5 py-0.5 border"
            style={{ color: phaseColor, borderColor: phaseColor }}
          >
            {PHASE_LABEL[entry.phase]}
          </span>
        </td>

        {/* Duration */}
        <td className="px-2 py-2 font-mono text-[10px] tabular-nums text-[var(--text-ghost)]">
          {formatDuration(entry.durationMs)}
        </td>

        {/* Promoted */}
        <td className="px-2 py-2 font-mono text-[10px] tabular-nums">
          <span style={{ color: 'var(--dream-bright)' }}>
            +{entry.promotedCount}
          </span>
        </td>

        {/* Demoted */}
        <td className="px-2 py-2 font-mono text-[10px] tabular-nums">
          <span style={{ color: 'var(--warning)' }}>
            -{entry.demotedCount}
          </span>
        </td>

        {/* Created */}
        <td className="px-2 py-2 font-mono text-[10px] tabular-nums text-[var(--text-faint)]">
          {entry.createdCount}
        </td>

        {/* Insights */}
        <td className="px-2 py-2 font-mono text-[10px] tabular-nums">
          <span style={{ color: 'var(--sage)' }}>
            {entry.insightsGained}
          </span>
        </td>
      </tr>

      {/* Expanded row */}
      {expanded && (
        <tr
          className="border-b border-b-[var(--text-ghost)] bg-[var(--bg-secondary)]"
        >
          <td colSpan={8} className="px-8 py-3">
            <div className="flex flex-wrap gap-8">
              <div>
                <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
                  Cycle ID
                </div>
                <div className="font-mono text-xs text-[var(--text-faint)]">
                  {entry.id}
                </div>
              </div>
              <div>
                <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
                  Phase
                </div>
                <div className="font-mono text-xs" style={{ color: phaseColor }}>
                  {PHASE_LABEL[entry.phase]}
                </div>
              </div>
              <div>
                <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
                  Net Knowledge Delta
                </div>
                <div className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                  {entry.promotedCount - entry.demotedCount >= 0 ? '+' : ''}
                  {entry.promotedCount - entry.demotedCount}
                </div>
              </div>
              <div>
                <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
                  Throughput
                </div>
                <div className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                  {entry.durationMs > 0
                    ? (
                        ((entry.promotedCount + entry.demotedCount + entry.createdCount) /
                          (entry.durationMs / 1_000))
                      ).toFixed(2)
                    : '—'}
                  {' '}ops/s
                </div>
              </div>
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

function JournalTable({ entries }: { entries: DreamJournalEntry[] }) {
  const [sortField,   setSortField]   = useState<SortField>('timestamp');
  const [sortDir,     setSortDir]     = useState<SortDir>('desc');
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const handleSort = (field: SortField) => {
    if (field === sortField) {
      setSortDir((d) => (d === 'desc' ? 'asc' : 'desc'));
    } else {
      setSortField(field);
      setSortDir('desc');
    }
  };

  const sorted = useMemo(() => {
    const copy = [...entries];
    const mult = sortDir === 'desc' ? -1 : 1;
    copy.sort((a, b) => {
      switch (sortField) {
        case 'timestamp': return mult * (new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
        case 'promoted':  return mult * (a.promotedCount - b.promotedCount);
        case 'demoted':   return mult * (a.demotedCount  - b.demotedCount);
        case 'duration':  return mult * (a.durationMs    - b.durationMs);
        default: return 0;
      }
    });
    return copy;
  }, [entries, sortField, sortDir]);

  const toggleRow = (id: string) =>
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  if (entries.length === 0) {
    return (
      <div
        className={clsx(
          'border border-[var(--text-ghost)] bg-[var(--bg-raised)]',
          'flex items-center justify-center py-16',
        )}
      >
        <div className="text-center">
          <div className="font-mono text-sm text-[var(--text-ghost)] mb-2">
            No dream cycles recorded yet.
          </div>
          <div className="font-mono text-xs text-[var(--text-faint)]">
            Run a cycle to start consolidating knowledge.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="border border-[var(--text-ghost)] bg-[var(--bg-raised)]">
      {/* Header */}
      <div
        className={clsx(
          'flex items-center gap-2 px-4 py-2',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
          Dream Journal
        </span>
        <span className="font-mono text-[10px] text-[var(--text-faint)]">
          ({sorted.length} cycles)
        </span>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full min-w-[680px] border-collapse">
          <thead>
            <tr className="border-b border-b-[var(--text-ghost)]">
              {/* Expand */}
              <th className="px-4 py-2 w-6" aria-label="Expand" />

              <th className="px-2 py-2 text-left">
                <SortHeader
                  field="timestamp"
                  label="Time"
                  current={sortField}
                  dir={sortDir}
                  onSort={handleSort}
                />
              </th>

              <th className="px-2 py-2 text-left">
                <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
                  Phase
                </span>
              </th>

              <th className="px-2 py-2 text-left">
                <SortHeader
                  field="duration"
                  label="Duration"
                  current={sortField}
                  dir={sortDir}
                  onSort={handleSort}
                />
              </th>

              <th className="px-2 py-2 text-left">
                <SortHeader
                  field="promoted"
                  label="Promoted"
                  current={sortField}
                  dir={sortDir}
                  onSort={handleSort}
                />
              </th>

              <th className="px-2 py-2 text-left">
                <SortHeader
                  field="demoted"
                  label="Demoted"
                  current={sortField}
                  dir={sortDir}
                  onSort={handleSort}
                />
              </th>

              <th className="px-2 py-2 text-left">
                <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
                  Created
                </span>
              </th>

              <th className="px-2 py-2 text-left">
                <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
                  Insights
                </span>
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((entry) => (
              <JournalRow
                key={entry.id}
                entry={entry}
                expanded={expandedIds.has(entry.id)}
                onToggle={() => toggleRow(entry.id)}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Archive coverage strip
// ---------------------------------------------------------------------------

function ArchiveCoverageStrip({
  archivedCount,
  activeCount,
}: {
  archivedCount: number;
  activeCount:   number;
}) {
  const total = archivedCount + activeCount;
  const pct   = total > 0 ? Math.round((archivedCount / total) * 100) : 0;

  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)] bg-[var(--bg-raised)]',
        'px-4 py-3',
      )}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-4">
          <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
            Archive Coverage
          </span>
          <span className="font-mono text-xs text-[var(--text-faint)]">
            <span style={{ color: 'var(--rose-dim)' }}>
              {archivedCount.toLocaleString()}
            </span>
            {' '}archived
          </span>
          <span className="font-mono text-xs text-[var(--text-faint)]">
            <span style={{ color: 'var(--dream)' }}>
              {activeCount.toLocaleString()}
            </span>
            {' '}active
          </span>
        </div>
        <span className="font-mono text-[10px] tabular-nums text-[var(--text-ghost)]">
          {pct}%
        </span>
      </div>
      <ProgressBar value={pct} height={3} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Run cycle button
// ---------------------------------------------------------------------------

function RunCycleButton({
  hasActiveCycle,
  onRun,
  loading,
}: {
  hasActiveCycle: boolean;
  onRun: () => void;
  loading: boolean;
}) {
  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)] bg-[var(--bg-raised)]',
        'p-5',
        'flex flex-col items-center gap-4',
      )}
    >
      {hasActiveCycle ? (
        <div className="flex flex-col items-center gap-3">
          <div className="flex items-center gap-3">
            <Spinner size="md" />
            <span className="font-mono text-sm text-[var(--text-muted)]">
              Dream cycle running…
            </span>
          </div>
          <span className="font-mono text-xs text-[var(--text-ghost)]">
            The cycle will complete when consolidation finishes.
          </span>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-3">
          <div className="flex items-center gap-4">
            <Button
              size="lg"
              variant="primary"
              loading={loading}
              onClick={onRun}
            >
              Run Dream Cycle
            </Button>
          </div>
          <span className="font-mono text-xs text-[var(--text-ghost)] text-center max-w-xs">
            Runs all three phases: Hypnagogia → Imagination → Consolidation.
            Knowledge entries are promoted, demoted, or created based on evidence.
          </span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function DreamsPage() {
  const { data: status,  isLoading: statusLoading  } = useDreamStatus();
  const { data: journal, isLoading: journalLoading } = useDreamJournal();
  const runMut = useRunDreamCycle();

  const activePhase    = status?.activeCycle?.phase ?? null;
  const hasActiveCycle = Boolean(status?.activeCycle);

  return (
    <div className="flex flex-col h-full min-h-0 overflow-y-auto overflow-x-hidden">
      <div className="p-5 max-w-5xl w-full mx-auto flex flex-col gap-5">

        {/* ------------------------------------------------------------------ */}
        {/* Phase stepper                                                        */}
        {/* ------------------------------------------------------------------ */}
        {statusLoading ? (
          <div className="flex items-center justify-center py-8">
            <Spinner size="md" />
          </div>
        ) : (
          <PhaseStepper activePhase={activePhase} />
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Two-col row: triggers + archive                                      */}
        {/* ------------------------------------------------------------------ */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
          {/* Trigger panel */}
          {status ? (
            <TriggerTable triggers={status.triggers} />
          ) : (
            <div className="border border-[var(--text-ghost)] flex items-center justify-center py-8">
              <Spinner size="sm" />
            </div>
          )}

          {/* Right column: archive strip + run button */}
          <div className="flex flex-col gap-5">
            {status && (
              <ArchiveCoverageStrip
                archivedCount={status.archivedCount}
                activeCount={status.activeCount}
              />
            )}

            <RunCycleButton
              hasActiveCycle={hasActiveCycle}
              onRun={() => runMut.mutate()}
              loading={runMut.isPending}
            />
          </div>
        </div>

        {/* ------------------------------------------------------------------ */}
        {/* Journal table                                                        */}
        {/* ------------------------------------------------------------------ */}
        {journalLoading ? (
          <div className="flex items-center justify-center py-8">
            <Spinner size="md" />
          </div>
        ) : (
          <JournalTable entries={journal?.entries ?? []} />
        )}

      </div>
    </div>
  );
}
