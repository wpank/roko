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
import type { WirePlanSummary } from '@/api/contracts';
import {
  usePlanTasks,
  useRunPlan,
  useCancelPlan,
  useValidation,
} from '@/api/queries';
import { ApiError } from '@/api/client';
import { useDashboardStore } from '@/stores/dashboard';
import { buildTaskRows } from '@/lib/taskRows';
import { computeWaves } from '@/lib/waves';
import { formatCost, formatDuration } from '@/lib/formatters';
import { progressToken } from '@/lib/glyphs';
import { cn } from '@/lib/cn';
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
 *   • editing is active ("Save or discard your edits first")
 *   • validation reports errors ("Fix N validation errors first")
 *   • a plan-set run is active and this plan is not running in it
 *
 * The `r` key may call perform() for run / retry / run-again (T10 wires it).
 */
export function usePrimaryAction(
  planId: string | null,
  opts: { editing?: boolean; onError?: (msg: string) => void } = {},
): PrimaryAction {
  const { editing = false, onError } = opts;

  // Live run state
  const run = useDashboardStore((s) => s.run);

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
    if (editing) {
      disabled = true;
      reason = 'Save or discard your edits first';
    } else {
      const errorCount = validation
        ? validation.diagnostics.filter((d) => d.severity === 'error').length
        : 0;

      if (errorCount > 0) {
        disabled = true;
        reason = `Fix ${errorCount} validation error${errorCount === 1 ? '' : 's'} first`;
      } else if (run.planSet !== null && phase !== 'running') {
        // A plan-set run is active but this plan is not running in it.
        disabled = true;
        reason = 'Another run is already in progress';
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
      let msg = 'An unexpected error occurred.';
      if (err instanceof ApiError) {
        const b = err.body;
        if (b && typeof b === 'object') {
          const raw = (b as Record<string, unknown>)['message'];
          msg = typeof raw === 'string' ? raw : `Error ${err.status}`;
        } else {
          msg = `Error ${err.status}`;
        }
      } else if (err instanceof Error) {
        msg = err.message;
      }
      onError(msg);
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
    kind === 'cancel' ? 'Cancel' :
    kind === 'retry' ? 'Retry' :
    kind === 'run-again' ? 'Run again' :
    kind === 'run' ? 'Run' :
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
 *                  post-run: progress bar + done/total, elapsed, eta, cost, agents
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

  // ── Remote data ───────────────────────────────────────────────────────────

  const run = useDashboardStore((s) => s.run);
  const { data: tasksData } = usePlanTasks(plan.id);

  // ── Task rows + wave result ────────────────────────────────────────────────

  const { rows, waves } = useMemo(() => {
    const tasks = tasksData?.tasks;
    if (!tasks?.length) {
      return { rows: [], waves: computeWaves([]) };
    }
    return buildTaskRows(tasks, run, plan.id, Date.now());
  }, [tasksData, run, plan.id]);

  // ── Live plan run state ───────────────────────────────────────────────────

  const livePlan = run.plans[plan.id];
  const hasRun = livePlan !== undefined;
  const isRunning = livePlan?.phase === 'running';

  // ── Primary action ────────────────────────────────────────────────────────

  const primaryAction = usePrimaryAction(plan.id, {
    editing,
    onError: onRequestError,
  });

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

  // Progress bar color: green only when every task passed verification
  const allVerified =
    livePlan?.phase === 'completed' &&
    livePlan.tasksFailed === 0 &&
    livePlan.tasksAccepted === 0;

  const barColor = allVerified
    ? 'var(--state-done)'
    : livePlan?.phase === 'failed'
      ? 'var(--state-failed)'
      : livePlan?.phase === 'completed'
        ? 'var(--state-accepted)'
        : progressToken(fraction);

  // Elapsed time
  const nowMs = Date.now();
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

  const handleRetry = useCallback(() => {
    primaryAction.perform();
  }, [primaryAction]);

  const handleRevisionDone = useCallback(() => {
    // useRevisePlan already invalidates tasks/source/validation in onSuccess.
    setRevising(false);
  }, []);

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className={cn('flex flex-col gap-4')}>
      {/* ── 1. Header ─────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-start gap-3">
        {/* Title + group/id */}
        <div className="flex-1 min-w-0">
          <h2 className="font-mono text-sm font-semibold text-text-strong truncate leading-snug">
            {plan.title}
          </h2>
          <span className="font-mono text-xs text-text-ghost">
            {plan.group ? `${plan.group}/` : ''}
            {plan.id}
          </span>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-2 flex-shrink-0">
          {primaryAction.kind !== 'none' && (
            <Button
              data-action={primaryAction.kind}
              variant={primaryAction.kind === 'cancel' ? 'danger' : 'primary'}
              size="sm"
              disabled={primaryAction.disabled}
              title={primaryAction.reason ?? undefined}
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
              setRevising((r) => !r);
              if (editing) setEditing(false);
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
              setEditing((e) => !e);
              if (revising) setRevising(false);
            }}
          >
            ✎ Edit
          </Button>
        </div>
      </div>

      {/* ── 2. Status line ────────────────────────────────────────────────── */}
      {!hasRun ? (
        /* Pre-run: task count · waves · estimate · parallelism · validation */
        <div className="flex flex-wrap items-center gap-1 font-mono text-xs text-text-muted">
          <span>
            {plan.task_count} task{plan.task_count !== 1 ? 's' : ''}
          </span>
          <span className="text-text-ghost select-none">·</span>
          <span>
            {waveCount > 0
              ? `${waveCount} wave${waveCount !== 1 ? 's' : ''}`
              : '·'}
          </span>
          <span className="text-text-ghost select-none">·</span>
          <span>{estimatedMin !== null ? `~${estimatedMin} min` : '·'}</span>
          <span className="text-text-ghost select-none">·</span>
          <span>
            {effectiveParallel !== null ? `parallel ${effectiveParallel}` : '·'}
          </span>
          <span className="text-text-ghost select-none">·</span>
          <ValidationBadge planId={plan.id} onSelectTask={handleSelectTask} />
        </div>
      ) : (
        /* Post-run: progress bar + stats */
        <div className="flex flex-col gap-1.5">
          {/* Progress bar */}
          <div
            className="h-1 w-full rounded-full overflow-hidden bg-bg-highlight"
            role="progressbar"
            aria-valuenow={Math.round(fraction * 100)}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label={`${tasksDone} of ${tasksTotal} tasks complete`}
          >
            <div
              className="h-full transition-[width,background-color] duration-300"
              style={{
                width: `${Math.round(fraction * 100)}%`,
                backgroundColor: barColor,
              }}
            />
          </div>

          {/* Stats row */}
          <div className="flex flex-wrap items-center gap-1 font-mono text-xs text-text-muted tabular-nums">
            <span>
              {tasksDone}/{tasksTotal}
            </span>
            {elapsedMs !== null && (
              <>
                <span className="text-text-ghost select-none">·</span>
                <span>{formatDuration(elapsedMs)}</span>
              </>
            )}
            {etaMin !== null && (
              <>
                <span className="text-text-ghost select-none">·</span>
                <span>~{etaMin} min</span>
              </>
            )}
            {livePlan.costUsd > 0 && (
              <>
                <span className="text-text-ghost select-none">·</span>
                <span>{formatCost(livePlan.costUsd)}</span>
              </>
            )}
            {(busyAgents > 0 || isRunning) && (
              <>
                <span className="text-text-ghost select-none">·</span>
                <span>
                  {busyAgents} agent{busyAgents !== 1 ? 's' : ''} busy
                  {maxParallel !== null ? `/${maxParallel}` : ''}
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* ── 3. Content area ───────────────────────────────────────────────── */}
      {editing ? (
        /* SourceEditor replaces the list while open */
        <SourceEditor
          planId={plan.id}
          running={isRunning}
          onClose={() => setEditing(false)}
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
          <TaskList
            rows={rows}
            selectedTaskId={selectedTaskId}
            onSelectTask={handleSelectTask}
            onRetry={handleRetry}
          />
        </>
      )}
    </div>
  );
}
