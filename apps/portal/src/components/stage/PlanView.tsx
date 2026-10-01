'use client';

/**
 * PlanView — the main plan detail view.
 *
 * Exports:
 *   usePrimaryAction — derives the primary CTA from live plan phase + validation
 *   PlanView         — top-to-bottom plan view: header, status line,
 *                      WaveStrip + TaskList (or SourceEditor / PromptPanel overlay).
 */

import React, { useCallback, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { WirePlanSummary } from '@/api/contracts';
import {
  usePlanTasks,
  useRunPlan,
  useCancelPlan,
  useValidation,
  queryKeys,
} from '@/api/queries';
import { confirmDiscard, useDashboardStore } from '@/stores/dashboard';
import { buildTaskRows } from '@/lib/taskRows';
import { computeWaves } from '@/lib/waves';
import { progressSegments } from '@/lib/planRows';
import { cn } from '@/lib/cn';
import { describeRequestError } from '@/lib/apiErrors';
import { describePlan, planState } from '@/lib/emptyState';
import { planSetActive, queuePosition, waitReason } from '@/lib/planSet';
import { statusFields } from '@/lib/statusLine';
import { useNow } from '@/lib/useNow';
import { Button } from '@/components/atoms/Button';
import { WaveStrip } from '@/components/stage/WaveStrip';
import { TaskList } from '@/components/stage/TaskList';
import { ValidationBadge } from '@/components/stage/ValidationBadge';
import { SourceEditor } from '@/components/stage/SourceEditor';
import { PromptPanel } from '@/components/stage/PromptPanel';

// ── usePrimaryAction ───────────────────────────────────────────────────────────

export interface PrimaryAction {
  kind: 'run' | 'cancel' | 'retry' | 'run-again' | 'none';
  label: string;
  disabled: boolean;
  reason: string | null;
  perform(): void;
}

/**
 * Derive the primary CTA for a plan from its live execution phase and
 * the current validation state.
 *
 * - running        → cancel  (window.confirm first; stops the whole run)
 * - failed/cancelled → retry (resume: true — skips completed tasks)
 * - completed      → run-again (fresh run)
 * - otherwise      → run
 *
 * Disabled while:
 *   • the plan's editor holds unsaved text ("Save or discard your edits first")
 *   • validation reports errors ("Fix N validation errors first")
 *   • a plan-set run is active and this plan is not running in it
 *
 * The header button and the `r` key each call this hook; both read the unsaved
 * text from the store, so neither runs a plan other than the one on screen.
 */
export function usePrimaryAction(
  planId: string | null,
  opts: { onError?: (msg: string) => void } = {},
): PrimaryAction {
  const { onError } = opts;

  // Live run state
  const run = useDashboardStore((s) => s.run);
  const unsaved = useDashboardStore((s) => planId !== null && s.unsavedPlan === planId);

  // Validation (disabled when planId is null)
  const { data: validation } = useValidation(planId ?? undefined);

  // Mutations
  const runPlan = useRunPlan();
  const cancelPlan = useCancelPlan();

  // ── Phase ─────────────────────────────────────────────────────────────────

  const livePlan = planId ? run.plans[planId] : undefined;
  const phase = livePlan?.phase ?? null;

  // ── Kind ──────────────────────────────────────────────────────────────────

  let kind: PrimaryAction['kind'];
  if (!planId) {
    kind = 'none';
  } else if (phase === 'running') {
    kind = 'cancel';
  } else if (phase === 'failed' || phase === 'cancelled') {
    kind = 'retry';
  } else if (phase === 'completed') {
    kind = 'run-again';
  } else {
    // pending or no live record (never run)
    kind = 'run';
  }

  // ── Disabled / reason ─────────────────────────────────────────────────────

  let disabled = !planId;
  let reason: string | null = null;

  if (planId) {
    if (unsaved) {
      disabled = true;
      reason = 'Save or discard your edits first';
    } else {
      const errorCount = validation
        ? validation.diagnostics.filter((d) => d.severity === 'error').length
        : 0;

      if (errorCount > 0) {
        disabled = true;
        reason = `Fix ${errorCount} validation error${errorCount === 1 ? '' : 's'} first`;
      } else if (planSetActive(run) && phase !== 'running') {
        // A plan-set run is active but this plan is not running in it.
        disabled = true;
        const pos = queuePosition(run, planId);
        const wait = waitReason(run, planId);
        if (pos !== null && wait !== null) {
          reason = `Queued #${pos} — ${wait}`;
        } else {
          reason = 'Another run is already in progress';
        }
      }
    }
  }

  // ── Running plan count for cancel confirmation ────────────────────────────

  const runningPlanCount = useMemo(
    () =>
      Object.values(run.plans).filter((p) => p.phase === 'running').length,
    [run.plans],
  );

  // ── perform ───────────────────────────────────────────────────────────────

  const perform = useCallback(() => {
    if (!planId || disabled) return;

    /** Surface mutation errors to the caller. */
    const handleError = (err: unknown): void => {
      if (!onError) return;
      const action = kind === 'cancel' ? 'cancelling runs' : 'running plans';
      onError(describeRequestError(err, action));
    };

    switch (kind) {
      case 'cancel': {
        const n = runningPlanCount;
        const planWord = n === 1 ? 'the running plan' : `${n} running plans`;
        if (
          !window.confirm(
            `Cancel this run? This will stop ${planWord}. The entire run will be cancelled.`,
          )
        )
          return;
        cancelPlan.mutate({ id: planId }, { onError: handleError });
        break;
      }
      case 'retry': {
        // resume: true — completed tasks are skipped; plan re-runs fresh if edited
        runPlan.mutate(
          { id: planId, resume: true },
          { onError: handleError },
        );
        break;
      }
      case 'run':
      case 'run-again': {
        runPlan.mutate({ id: planId }, { onError: handleError });
        break;
      }
      default:
        break;
    }
  }, [planId, disabled, kind, runningPlanCount, cancelPlan, runPlan, onError]);

  // ── Label ─────────────────────────────────────────────────────────────────

  const label =
    kind === 'cancel' ? '■ Cancel' :
    kind === 'retry' ? '↻ Retry' :
    kind === 'run-again' ? '▶ Run again' :
    kind === 'run' ? '▶ Run' :
    '';

  return { kind, label, disabled, reason, perform };
}

// ── PlanView ───────────────────────────────────────────────────────────────────

export interface PlanViewProps {
  plan: WirePlanSummary;
  selectedTaskId: string | null;
  onSelectTask(id: string | null): void;
  onRequestError(message: string): void;
}

/**
 * Top-to-bottom plan detail view:
 *
 * 1. Header — title, group/id, primary CTA, ✦ Revise, ✎ Edit
 * 2. Status line — pre-run: task/wave/estimate/parallel counts + ValidationBadge;
 *                  post-run: progress bar + done/total, elapsed, eta, cost, agents,
 *                  then the run summary sentence
 * 3. WaveStrip + TaskList — or SourceEditor / PromptPanel(revise) while open
 */
export function PlanView({
  plan,
  selectedTaskId,
  onSelectTask,
  onRequestError,
}: PlanViewProps) {
  // ── Panel toggle state ────────────────────────────────────────────────────

  const [editing, setEditing] = useState(false);
  const [revising, setRevising] = useState(false);
  // The editor's unsaved text is marked in the store, where Run, the `r` key,
  // Run all and the selection all see it. An open editor that holds none (or
  // shows the "not supported" notice) blocks nothing.
  const dirty = useDashboardStore((s) => s.unsavedPlan === plan.id);
  const setUnsaved = useDashboardStore((s) => s.setUnsaved);
  const markDirty = useCallback((d: boolean) => setUnsaved(plan.id, d), [setUnsaved, plan.id]);

  // ── Remote data ───────────────────────────────────────────────────────────

  const queryClient = useQueryClient();
  const run = useDashboardStore((s) => s.run);
  const { data: tasksData } = usePlanTasks(plan.id);

  // ── Live plan run state (derived before hooks that depend on isRunning) ───

  const livePlan = run.plans[plan.id];
  // hasRun: plan has live state and its phase is not 'pending' (queued plans
  // in a plan-set have phase 'pending' and show pre-run facts, no progress bar).
  const hasRun = livePlan !== undefined && livePlan.phase !== 'pending';
  const isRunning = livePlan?.phase === 'running';

  // ── Ticking clock — drives elapsed time; costs nothing while idle ─────────

  const nowMs = useNow(isRunning);

  // ── Task rows + wave result ────────────────────────────────────────────────

  const { rows, waves } = useMemo(() => {
    const tasks = tasksData?.tasks;
    if (!tasks?.length) {
      return { rows: [], waves: computeWaves([]) };
    }
    return buildTaskRows(tasks, run, plan.id, nowMs);
  }, [tasksData, run, plan.id, nowMs]);

  // ── Primary action ────────────────────────────────────────────────────────

  const primaryAction = usePrimaryAction(plan.id, { onError: onRequestError });

  // ── Derived metrics ───────────────────────────────────────────────────────

  const maxParallel = tasksData?.max_parallel ?? null;
  const waveCount = waves.waves.length;

  // Pre-run: estimate and effective parallelism
  const estimatedMin = plan.estimated_minutes ?? null;
  const maxWaveWidth = waves.waves.reduce((m, w) => Math.max(m, w.length), 0);
  const effectiveParallel =
    maxParallel ?? (maxWaveWidth > 1 ? maxWaveWidth : null);

  // Post-run: progress fraction
  const tasksDone = livePlan?.tasksDone ?? 0;
  const tasksTotal = livePlan?.tasksTotal ?? plan.task_count;
  const fraction = tasksTotal > 0 ? tasksDone / tasksTotal : 0;

  // Progress bar segments
  // Unverified tasks share the amber segment: only verified passes are green.
  const barAccepted = (livePlan?.tasksAccepted ?? 0) + (livePlan?.tasksUnverified ?? 0);
  const barFailed = livePlan?.tasksFailed ?? 0;
  const barActive = isRunning
    ? Object.values(run.tasks).filter(
        (t) => t.planId === plan.id && t.status === 'active',
      ).length
    : 0;
  const barDone = Math.max(0, tasksDone - barAccepted);
  const barSegments = progressSegments(
    { done: barDone, accepted: barAccepted, failed: barFailed, active: barActive },
    tasksTotal,
  );

  // Elapsed time (nowMs comes from useNow above — no read-time clock calls)
  const elapsedMs =
    livePlan?.startedAtMs != null
      ? (livePlan.finishedAtMs ?? nowMs) - livePlan.startedAtMs
      : null;

  const etaMin = livePlan?.etaMinutes ?? null;

  // Active agent count for this plan
  const busyAgents = useMemo(
    () =>
      Object.values(run.agents).filter(
        (a) => a.active && a.planId === plan.id,
      ).length,
    [run.agents, plan.id],
  );

  // ── Callbacks ─────────────────────────────────────────────────────────────

  /** Coerce string to the parent's nullable signature. */
  const handleSelectTask = useCallback(
    (id: string) => onSelectTask(id),
    [onSelectTask],
  );

  const handleRevisionDone = useCallback(async () => {
    // A revision lands only when its operation completes — after the 202 that
    // useRevisePlan's onSuccess fired on. Refreshing on the 202 alone left the
    // old tasks on screen because the server had not yet written the new tasks.
    // Awaiting all four invalidations here ensures the cache holds the post-
    // operation data before we close the prompt and the task list re-renders.
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.plans }),
      queryClient.invalidateQueries({ queryKey: queryKeys.planTasks(plan.id) }),
      queryClient.invalidateQueries({ queryKey: queryKeys.planSource(plan.id) }),
      queryClient.invalidateQueries({ queryKey: queryKeys.validation(plan.id) }),
    ]);
    setRevising(false);
  }, [queryClient, plan.id]);

  // ── Render ────────────────────────────────────────────────────────────────

  // While editing, the view fills the stage and the editor takes what the
  // header leaves (design §2, §4a).
  return (
    <div className={cn('flex flex-col gap-4', editing && 'min-h-full')}>
      {/* ── 1. Header ─────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-start gap-3">
        {/* Title + group/id */}
        <div className="flex-1 min-w-0">
          <h2 className="rd-title font-mono font-semibold text-text-strong truncate leading-snug">
            {plan.title}
          </h2>
          <span className="rd-meta font-mono text-text-ghost">
            {plan.group ? `${plan.group}/` : ''}
            {plan.id}
          </span>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-2 flex-shrink-0">
          {/* Why the primary action is disabled, as text rather than a tooltip
              (design §4.1: "with the reason shown"). */}
          {primaryAction.disabled && primaryAction.reason !== null && (
            <span data-reason className="rd-reason font-mono text-xs text-text-muted">
              {primaryAction.reason}
            </span>
          )}
          {primaryAction.kind !== 'none' && (
            <Button
              data-action={primaryAction.kind}
              variant={primaryAction.kind === 'cancel' ? 'danger' : 'primary'}
              size="sm"
              disabled={primaryAction.disabled}
              onClick={primaryAction.perform}
            >
              {primaryAction.label}
            </Button>
          )}

          {/* ✦ Revise — toggles PromptPanel in revise mode */}
          <Button
            data-action="revise"
            variant={revising ? 'secondary' : 'ghost'}
            size="sm"
            disabled={isRunning}
            title={
              isRunning
                ? 'Cannot revise while the plan is running'
                : undefined
            }
            onClick={() => {
              if (!confirmDiscard(dirty)) return;
              setRevising((r) => !r);
              setEditing(false);
            }}
          >
            ✦ Revise
          </Button>

          {/* ✎ Edit — toggles SourceEditor */}
          <Button
            data-action="edit"
            variant={editing ? 'secondary' : 'ghost'}
            size="sm"
            disabled={isRunning}
            title={
              isRunning
                ? 'Cannot edit while the plan is running'
                : undefined
            }
            onClick={() => {
              if (!confirmDiscard(dirty)) return;
              setEditing((e) => !e);
              setRevising(false);
            }}
          >
            ✎ Edit
          </Button>
        </div>
      </div>

      {/* ── 2. Status line ────────────────────────────────────────────────── */}

      {/* Progress bar — only when the plan has actually run (phase ≠ pending) */}
      {hasRun && (
        <div
          role="progressbar"
          className="rd-progress rounded-full"
          aria-valuenow={Math.round(fraction * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={`${tasksDone} of ${tasksTotal} tasks complete`}
        >
          {barSegments.map((seg) => (
            <span
              key={seg.state}
              className="rd-seg"
              data-segment={seg.state}
              style={{ width: `${(seg.share * 100).toFixed(1)}%` }}
            />
          ))}
        </div>
      )}

      {/* Status line: only known fields, no placeholder dots */}
      <div
        data-region="status-line"
        className="rd-status flex flex-wrap items-center gap-1 font-mono text-xs text-text-muted tabular-nums"
      >
        {statusFields({
          hasRun,
          running: isRunning,
          taskCount: plan.task_count,
          waveCount,
          estimatedMinutes: estimatedMin,
          parallel: effectiveParallel,
          tasksDone,
          tasksTotal,
          elapsedMs,
          etaMinutes: etaMin,
          costUsd: livePlan?.costUsd ?? 0,
          busyAgents,
          maxParallel,
        }).map((field, idx) => (
          <React.Fragment key={field.key}>
            {idx > 0 && (
              <span className="rd-status__sep text-text-ghost select-none" aria-hidden="true">·</span>
            )}
            <span data-field={field.key} className="rd-status__field">
              {field.text}
            </span>
          </React.Fragment>
        ))}

        {/* ValidationBadge — only before a run, after one more separator */}
        {!hasRun && (
          <>
            <span className="rd-status__sep text-text-ghost select-none" aria-hidden="true">·</span>
            <ValidationBadge planId={plan.id} onSelectTask={handleSelectTask} />
          </>
        )}
      </div>

      {/* Run summary (design §7) — what the run is doing or came to, whatever
          task has focus. Before a run the status line says it. */}
      {hasRun && (
        <p data-region="run-summary" className="rd-meta font-mono">
          {describePlan(planState(run, plan.id, rows, waveCount))}
        </p>
      )}

      {/* ── 3. Content area ───────────────────────────────────────────────── */}
      {editing ? (
        /* SourceEditor replaces the list while open */
        <SourceEditor
          planId={plan.id}
          running={isRunning}
          onClose={() => setEditing(false)}
          onDirtyChange={markDirty}
        />
      ) : revising ? (
        /* PromptPanel in revise mode replaces the list while open */
        <PromptPanel
          mode="revise"
          planId={plan.id}
          workspace={plan.id}
          firstRun={false}
          onDone={handleRevisionDone}
          onCancel={() => setRevising(false)}
        />
      ) : (
        <>
          {waves.waves.length > 0 && (
            <WaveStrip
              waves={waves}
              rows={rows}
              maxParallel={maxParallel}
              selectedTaskId={selectedTaskId}
              onSelectTask={handleSelectTask}
            />
          )}
          {/* onRetry is intentionally omitted: the plan header's primary
              action button is the one Retry for this plan. A second Retry
              in every failed task row would duplicate the action and confuse
              which button to use. */}
          <TaskList
            rows={rows}
            selectedTaskId={selectedTaskId}
            onSelectTask={handleSelectTask}
          />
        </>
      )}
    </div>
  );
}
