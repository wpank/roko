'use client';

/**
 * TaskList — renders the ordered list of task rows for a plan view.
 *
 * Each row shows: glyph · id · title · role·model · time · cost · retry count · check chips.
 * The selected row expands to show description, files, dependencies, and verify commands.
 * Failed rows show the first meaningful check output line and a Retry button.
 * accepted_with_failures rows are amber and list their failing checks.
 */

import React, { useCallback } from 'react';
import type { TaskRowModel } from '@/lib/taskRows';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { MetricCell } from '@/components/primitives/MetricCell';
import { cn } from '@/lib/cn';
import { formatSpan, shortModel, formatCost } from '@/lib/formatters';
import type { CheckRun } from '@/lib/runState';
import { GLYPHS } from '@/lib/glyphs';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface TaskListProps {
  rows: TaskRowModel[];
  selectedTaskId: string | null;
  onSelectTask(id: string): void;
  onRetry(): void;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Return the first output line of the first failed check that does not start
 * with "$ " (shell echo), or null if none exists.
 */
function firstFailedCheckLine(checks: CheckRun[]): string | null {
  for (const check of checks) {
    if (check.status !== 'failed') continue;
    const lines = check.output.split('\n');
    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed.length === 0) continue;
      if (trimmed.startsWith('$ ')) continue;
      return trimmed;
    }
  }
  return null;
}

// ── CheckChip ─────────────────────────────────────────────────────────────────

/** A single inline check chip: glyph + phase label. e.g. "✓ compile" or "✗ test" */
function CheckChip({ check }: { check: CheckRun }) {
  const glyphState =
    check.status === 'passed'
      ? 'done'
      : check.status === 'failed'
        ? 'failed'
        : 'active';
  const label = check.phase || check.name;
  return (
    <span
      className="inline-block text-xs font-mono"
      title={`${check.name}: ${check.status}`}
    >
      <StatusGlyph state={glyphState} title={`${check.name}: ${check.status}`} />
      {' '}
      <span style={{ color: GLYPHS[glyphState].token }}>{label}</span>
    </span>
  );
}

// ── ExpandedDetail ────────────────────────────────────────────────────────────

/** The expanded panel shown when a row is selected. */
function ExpandedDetail({
  row,
  onRetry,
}: {
  row: TaskRowModel;
  onRetry(): void;
}) {
  const failedChecks = row.checks.filter((c) => c.status === 'failed');
  const firstOutputLine = firstFailedCheckLine(row.checks);

  return (
    <div className="mt-2 pl-6 flex flex-col gap-3 text-sm font-mono">
      {/* ── Description ───────────────────────────────────────────────────── */}
      {row.description && (
        <pre className="whitespace-pre-wrap text-text-muted text-xs leading-relaxed">
          {row.description}
        </pre>
      )}

      {/* ── Failed: first output line + Retry button ───────────────────────── */}
      {row.status === 'failed' && (
        <div className="flex flex-col gap-2">
          {firstOutputLine && (
            <p
              className="text-xs font-mono text-accent-error truncate"
              title={firstOutputLine}
            >
              {firstOutputLine}
            </p>
          )}
          <div>
            <button
              type="button"
              data-action="retry"
              onClick={(e) => {
                e.stopPropagation();
                onRetry();
              }}
              className={cn(
                'inline-flex items-center gap-1 rounded px-2 py-1',
                'text-xs font-mono border border-border-default',
                'text-text-muted hover:text-text-strong hover:border-border-hover',
                'transition-[border-color,color] duration-[80ms]',
              )}
            >
              <span aria-hidden>↻</span> Retry
            </button>
          </div>
        </div>
      )}

      {/* ── accepted_with_failures: amber note + failing checks ───────────── */}
      {row.status === 'accepted_with_failures' && (
        <div className="flex flex-col gap-1">
          <p
            className="text-xs font-mono"
            style={{ color: 'var(--state-accepted)' }}
          >
            Forced through with failures — not a clean pass.
          </p>
          {failedChecks.length > 0 && (
            <ul className="flex flex-wrap gap-2">
              {failedChecks.map((c) => (
                <li key={c.name}>
                  <CheckChip check={c} />
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      {/* ── Files ────────────────────────────────────────────────────────────── */}
      {row.files.length > 0 && (
        <div className="flex flex-col gap-0.5">
          <span className="rd-section">files</span>
          <ul className="flex flex-col gap-0.5">
            {row.files.map((f) => (
              <li key={f} className="text-xs text-text-muted truncate" title={f}>
                {f}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* ── Depends on ────────────────────────────────────────────────────────── */}
      {row.dependsOn.length > 0 && (
        <div className="flex flex-col gap-0.5">
          <span className="rd-section">depends on</span>
          <ul className="flex flex-wrap gap-2">
            {row.dependsOn.map((depId) => {
              const isWaiting = row.waitingOn.includes(depId);
              return (
                <li
                  key={depId}
                  className={cn(
                    'text-xs font-mono',
                    isWaiting ? 'text-accent-warn' : 'text-text-muted',
                  )}
                  title={isWaiting ? 'still waiting on this dependency' : undefined}
                >
                  {depId}
                  {isWaiting && (
                    <span className="ml-0.5 text-xs" aria-label="waiting">
                      ⏳
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      )}

      {/* ── Verify commands ──────────────────────────────────────────────────── */}
      {row.verify.length > 0 && (
        <div className="flex flex-col gap-0.5">
          <span className="rd-section">verify</span>
          <ul className="flex flex-col gap-1">
            {row.verify.map((v, i) => (
              <li key={i} className="flex items-start gap-2 text-xs font-mono">
                <span className="text-text-ghost shrink-0 w-20 truncate" title={v.phase}>
                  {v.phase}
                </span>
                {v.command && (
                  <span className="text-text-muted break-all">{v.command}</span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

// ── TaskRow ───────────────────────────────────────────────────────────────────

function TaskRow({
  row,
  selected,
  onSelect,
  onRetry,
}: {
  row: TaskRowModel;
  selected: boolean;
  onSelect(): void;
  onRetry(): void;
}) {
  const timeStr = formatSpan(row.time);
  const costStr = row.costUsd != null ? formatCost(row.costUsd) : null;
  const isAccepted = row.status === 'accepted_with_failures';

  return (
    <li
      data-task-row={row.id}
      data-state={row.state}
      className={cn(
        'flex flex-col px-3 py-2 rounded cursor-pointer select-none',
        'border border-transparent',
        'transition-[background-color,border-color] duration-[80ms]',
        selected
          ? 'bg-bg-highlight border-border-default'
          : 'hover:bg-bg-subtle',
        // Amber tint for accepted_with_failures
        isAccepted && 'border-l-2',
      )}
      style={isAccepted ? { borderLeftColor: 'var(--state-accepted)' } : undefined}
      onClick={onSelect}
      role="option"
      aria-selected={selected}
    >
      {/* ── Summary row: eight-track grid, every track always present ────── */}
      <div className="rd-task-row">
        {/* 1. Glyph */}
        <span data-cell="glyph">
          <StatusGlyph state={row.state} />
        </span>

        {/* 2. ID */}
        <span data-cell="id" className="rd-meta font-mono" title={row.id}>
          {row.id}
        </span>

        {/* 3. Title */}
        <span data-cell="title" className="rd-row font-mono">
          {row.title}
        </span>

        {/* 4. Role · Model */}
        <span data-cell="role" className="rd-task-row__meta font-mono text-text-faint">
          {row.role && (
            <span
              data-role={row.role}
              style={{ color: `var(--role-${row.role}, var(--role-other))` }}
            >
              {row.role}
            </span>
          )}
          {row.role && row.model && '·'}
          {row.model && (
            <span title={row.model}>{shortModel(row.model)}</span>
          )}
        </span>

        {/* 5. Time — empty when unknown */}
        <span data-cell="time" className="rd-task-row__num">
          {timeStr != null && <MetricCell value={timeStr} />}
        </span>

        {/* 6. Cost — empty when unknown */}
        <span data-cell="cost" className="rd-task-row__num">
          {costStr != null && <MetricCell value={costStr} />}
        </span>

        {/* 7. Attempts (↻N when N > 1, else empty) */}
        <span
          data-cell="attempts"
          className="rd-task-row__num"
          style={row.attempts > 1 ? { color: 'var(--state-accepted)' } : undefined}
          title={row.attempts > 1 ? `${row.attempts} attempts` : undefined}
        >
          {row.attempts > 1 ? `↻${row.attempts}` : ''}
        </span>

        {/* 8. Checks (space-separated chips, no flex wrapper) */}
        <span data-cell="checks" className="rd-task-row__meta">
          {row.checks.map((c, i) => (
            <React.Fragment key={c.name}>
              {i > 0 && ' '}
              <CheckChip check={c} />
            </React.Fragment>
          ))}
        </span>
      </div>

      {/* ── Expanded detail ──────────────────────────────────────────────── */}
      {selected && (
        <ExpandedDetail row={row} onRetry={onRetry} />
      )}
    </li>
  );
}

// ── TaskList ──────────────────────────────────────────────────────────────────

/**
 * TaskList — ordered list of task rows for a plan view.
 *
 * Rows are rendered in the order provided (wave order, then input order within
 * each wave). Rows are never reordered here; that is the caller's responsibility.
 */
export function TaskList({ rows, selectedTaskId, onSelectTask, onRetry }: TaskListProps) {
  const handleSelect = useCallback(
    (id: string) => () => {
      onSelectTask(id);
    },
    [onSelectTask],
  );

  return (
    <ol
      data-region="tasks"
      role="listbox"
      aria-label="Plan tasks"
      className="flex flex-col gap-1"
    >
      {rows.map((row) => (
        <TaskRow
          key={row.id}
          row={row}
          selected={row.id === selectedTaskId}
          onSelect={handleSelect(row.id)}
          onRetry={onRetry}
        />
      ))}
    </ol>
  );
}
