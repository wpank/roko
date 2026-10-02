'use client';

/**
 * TaskList — renders the ordered list of task rows for a plan view.
 *
 * Each row shows: glyph · id · title · role·model · time · cost · retry count · check chips.
 * The selected row expands to show description, files, dependencies, and verify commands.
 * A failed row shows the first line of its failed check's digest under the row, expanded
 * or not (design §11); the Retry button appears only when onRetry is provided (the plan
 * header is the one Retry — see PlanView.tsx).
 * accepted_with_failures rows are amber and list their failing checks.
 * Given its plan id, a task a Graph run holds for review gets a Review action that opens its
 * ReviewPane.
 */

import React, { useCallback, useMemo, useState } from 'react';
import { useReviews } from '@/api/queries';
import { ReviewPane } from '@/components/stage/ReviewPane';
import { useDashboardStore } from '@/stores/dashboard';
import type { TaskRowModel } from '@/lib/taskRows';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { MetricCell } from '@/components/primitives/MetricCell';
import { cn } from '@/lib/cn';
import { formatSpan, shortModel, formatCost } from '@/lib/formatters';
import type { CheckRun } from '@/lib/runState';
import { GLYPHS, glyphStateForCheck } from '@/lib/glyphs';
import { digestHeadline, digestOutput } from '@/lib/checks';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface TaskListProps {
  rows: TaskRowModel[];
  selectedTaskId: string | null;
  onSelectTask(id: string): void;
  /** When provided, a Retry button appears in the expanded detail of failed rows. */
  onRetry?(): void;
  /** The rows' plan: with it, a task held for review gets a Review action. */
  planId?: string;
}

/** A held task's Review action: whether its pane is open, and the toggle. */
interface RowReview {
  planId: string;
  open: boolean;
  onToggle(): void;
}

/** The phase of a task whose verified attempt waits for a review (roko-cli's AWAITING_APPROVAL_PHASE). */
const AWAITING_APPROVAL = 'awaiting_approval';

const NONE_HELD: ReadonlySet<string> = new Set();

// ── Helpers ────────────────────────────────────────────────────────────────────

/** The first line of the first failed check's digest, or null when no check failed. */
function failureHeadline(checks: CheckRun[]): string | null {
  const failed = checks.find((c) => c.status === 'failed');
  return failed ? digestHeadline(digestOutput(failed.output)) : null;
}

// ── CheckChip ─────────────────────────────────────────────────────────────────

/** A single inline check chip: glyph + phase label. e.g. "✓ compile" or "✗ test" */
function CheckChip({ check }: { check: CheckRun }) {
  const glyphState = glyphStateForCheck(check.status);
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
  onRetry?(): void;
}) {
  const failedChecks = row.checks.filter((c) => c.status === 'failed');

  return (
    <div className="mt-2 pl-6 flex flex-col gap-3 text-sm font-mono">
      {/* ── Description ───────────────────────────────────────────────────── */}
      {row.description && (
        <pre className="whitespace-pre-wrap text-text-muted text-xs leading-relaxed">
          {row.description}
        </pre>
      )}

      {/* ── Blocked: the task that blocked it, and why ────────────────────── */}
      {row.blocked !== null && (
        <div className="flex flex-col gap-0.5">
          <span className="rd-section">blocked</span>
          <p className="text-xs text-text-muted whitespace-pre-wrap">{row.blocked}</p>
        </div>
      )}

      {/* ── Failed: optional Retry button (the digest line is on the row) ── */}
      {row.status === 'failed' && (
        <div className="flex flex-col gap-2">
          {onRetry && (
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
          )}
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
  review,
}: {
  row: TaskRowModel;
  selected: boolean;
  onSelect(): void;
  onRetry?(): void;
  review?: RowReview;
}) {
  const timeStr = formatSpan(row.time);
  const costStr = row.costUsd != null ? formatCost(row.costUsd) : null;
  const isAccepted = row.status === 'accepted_with_failures';
  const headline = row.status === 'failed' ? failureHeadline(row.checks) : null;

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

      {/* ── Failed: the digest's first line, on the row itself ───────────── */}
      {headline !== null && (
        <p
          data-digest=""
          className="pl-6 text-xs font-mono text-accent-error truncate"
          title={headline}
        >
          {headline}
        </p>
      )}

      {/* ── Blocked: which task blocked it, on the row itself ────────────── */}
      {row.blocked !== null && (
        <p
          data-blocked=""
          className="pl-6 text-xs font-mono text-text-muted truncate"
          title={row.blocked}
        >
          {row.blocked}
        </p>
      )}

      {/* ── Held for review: the Review action, and its pane ─────────────── */}
      {review && (
        <div className="pl-6">
          <button
            type="button"
            data-action="review"
            onClick={(e) => {
              e.stopPropagation();
              review.onToggle();
            }}
            className={cn(
              'mt-1 inline-flex items-center rounded px-2 py-1 text-xs font-mono',
              'border border-border-default text-text-muted hover:text-text-strong',
            )}
          >
            Review: waits for approval
          </button>
          {review.open && (
            <ReviewPane planId={review.planId} taskId={row.id} onDone={review.onToggle} />
          )}
        </div>
      )}

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
export function TaskList(props: TaskListProps) {
  return props.planId ? (
    <ReviewableTaskList {...props} planId={props.planId} />
  ) : (
    <TaskRows {...props} held={NONE_HELD} />
  );
}

/** Held tasks: those the live run shows awaiting approval, and those the reviews list. */
function ReviewableTaskList(props: TaskListProps & { planId: string }) {
  const { planId } = props;
  const tasks = useDashboardStore((s) => s.run.tasks);
  const running = useDashboardStore((s) => s.run.plans[planId]?.phase === 'running');
  // Holds exist only while a run waits on them, so only a running plan asks.
  const { data } = useReviews(planId, running);
  const held = useMemo(() => {
    const ids = new Set<string>();
    for (const task of Object.values(tasks)) {
      if (task.planId === planId && task.phase === AWAITING_APPROVAL) ids.add(task.taskId);
    }
    for (const review of data?.reviews ?? []) {
      if (review.status === AWAITING_APPROVAL) ids.add(review.task_id);
    }
    return ids;
  }, [tasks, data, planId]);
  return <TaskRows {...props} held={held} />;
}

function TaskRows({
  rows,
  selectedTaskId,
  onSelectTask,
  onRetry,
  planId,
  held,
}: TaskListProps & { held: ReadonlySet<string> }) {
  const [reviewing, setReviewing] = useState<string | null>(null);
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
          review={
            planId && held.has(row.id)
              ? {
                  planId,
                  open: reviewing === row.id,
                  onToggle: () => setReviewing((open) => (open === row.id ? null : row.id)),
                }
              : undefined
          }
        />
      ))}
    </ol>
  );
}
