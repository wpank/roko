/**
 * emptyState.ts — pure functions that describe what the system is doing.
 *
 * `describeEmpty` never returns an empty string or the phrase "no data".
 * It always describes the *current system state*, not the absence of content.
 * Its per-plan half, `describePlan`, is also the run summary the stage shows
 * whatever task has focus; `planState` gathers its input from run state.
 *
 * Mirrors the philosophy in mori's `widgets/agent_output.rs:327`.
 */

import type { ConnectionStatus } from '@/lib/bootstrap';
import type { PlanPhase, RunState } from '@/lib/runState';
import type { TaskRowModel } from '@/lib/taskRows';
import { compactDuration } from '@/lib/formatters';
import { queuePosition, waitReason } from '@/lib/planSet';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface EmptyStateInput {
  /** Folder name of the workspace (shown in the "no plans" message). */
  workspace: string;
  /** Current SSE connection status. */
  connection: ConnectionStatus;
  /** Total number of plans in the workspace (read only when no plan is selected). */
  planCount: number;
  /** The currently selected plan, if any. */
  plan?: {
    id: string;
    phase: PlanPhase | 'never_run';
    tasksTotal: number;
    tasksDone: number;
    tasksActive: number;
    tasksAccepted: number;
    /** Number of execution waves (optional — only set when the plan has wave metadata). */
    waves?: number;
    /** Task ID of the first failed task (for the failed-phase message). */
    failedTaskId?: string | null;
    /** Name of the failing check gate (for the failed-phase message). */
    failedCheck?: string | null;
    /** Elapsed wall-clock duration in milliseconds (for the completed message). */
    durationMs?: number | null;
    /** Human-readable reason why the plan is waiting in the queue (for the pending message). */
    waitReason?: string | null;
  };
}

/** One plan's state, as `describePlan` reads it and `planState` builds it. */
export type PlanState = NonNullable<EmptyStateInput['plan']>;

// ── describeEmpty ──────────────────────────────────────────────────────────────

/**
 * Return a human-readable sentence describing the current system state.
 *
 * Cases are evaluated in strict precedence order:
 * 1. Connection is lost → reconnecting message.
 * 2. No plan selected and none in the workspace → prompt to describe work.
 * 3. Plans exist but none selected → prompt to select.
 * Cases 4–10 are the selected plan's sentence, `describePlan`:
 * 4. Plan is never-run → show task / wave count.
 * 5. Plan is running:
 *    a. Nothing active, tasks queued → scheduler waiting message.
 *    b. Every task dispatched → dispatched message.
 *    c. Otherwise → generic running progress.
 * 6. Plan is pending (queued) → queued message with optional wait reason.
 * 7. Plan completed with accepted tasks → accepted message.
 * 8. Plan completed cleanly → finished + optional duration + verification count.
 * 9. Plan failed → stopped-at message with retry hint.
 * 10. Plan cancelled → cancelled message.
 */
export function describeEmpty(input: EmptyStateInput): string {
  const { connection, workspace, planCount, plan } = input;

  // ── 1. Connection problems ──────────────────────────────────────────────────
  if (connection === 'disconnected' || connection === 'error') {
    return 'Lost the server; reconnecting.';
  }

  // ── 2. No plans in workspace ────────────────────────────────────────────────
  if (!plan && planCount === 0) {
    return `No plans in ${workspace} yet. Describe what you want to build.`;
  }

  // ── 3. Plans exist but none selected ───────────────────────────────────────
  if (!plan) {
    return 'Select a plan, or press n to describe a new one.';
  }

  return describePlan(plan);
}

// ── describePlan ───────────────────────────────────────────────────────────────

/** Cases 4–10 above: one plan's sentence, which the stage also shows as its run summary. */
export function describePlan(plan: PlanState): string {
  const { phase, tasksTotal, tasksDone, tasksActive, tasksAccepted } = plan;

  // ── 4. Never run ────────────────────────────────────────────────────────────
  if (phase === 'never_run') {
    const taskWord = tasksTotal === 1 ? 'task' : 'tasks';
    if (plan.waves != null && plan.waves > 0) {
      const waveWord = plan.waves === 1 ? 'wave' : 'waves';
      return `Ready — ${tasksTotal} ${taskWord} in ${plan.waves} ${waveWord}.`;
    }
    return `Ready — ${tasksTotal} ${taskWord}.`;
  }

  // ── 5. Running ──────────────────────────────────────────────────────────────
  if (phase === 'running') {
    const queued = tasksTotal - tasksDone - tasksActive;

    // 5a. Scheduler waiting — nothing dispatched yet but tasks remain
    if (tasksActive === 0 && queued > 0) {
      const taskWord = queued === 1 ? 'queued task' : 'queued tasks';
      return `Scheduler has ${queued} ${taskWord} but no live agent yet.`;
    }

    // 5b. Every task is already dispatched to an agent
    if (tasksActive > 0 && tasksDone + tasksActive >= tasksTotal) {
      return 'Every task is dispatched; checks are running.';
    }

    // 5c. Running — generic progress
    return `Running — ${tasksActive} active, ${tasksDone} of ${tasksTotal} done.`;
  }

  // ── 6. Pending (queued) ─────────────────────────────────────────────────────
  if (phase === 'pending') {
    const reason = plan.waitReason;
    if (reason) return `Queued — ${reason}.`;
    return 'Queued — waiting to start.';
  }

  // ── 7. Completed with accepted tasks ───────────────────────────────────────
  if (phase === 'completed' && tasksAccepted > 0) {
    const taskWord = tasksAccepted === 1 ? 'task was' : 'tasks were';
    return `Finished; ${tasksAccepted} ${taskWord} accepted despite failing checks.`;
  }

  // ── 8. Completed cleanly ────────────────────────────────────────────────────
  if (phase === 'completed') {
    if (plan.durationMs != null) {
      const duration = compactDuration(plan.durationMs);
      return `Finished in ${duration} — ${tasksDone} of ${tasksTotal} verified.`;
    }
    return `Finished — ${tasksDone} of ${tasksTotal} verified.`;
  }

  // ── 9. Failed ───────────────────────────────────────────────────────────────
  if (phase === 'failed') {
    const { failedTaskId, failedCheck } = plan;
    if (failedTaskId) {
      const checkPart = failedCheck ? ` — ${failedCheck} failed` : '';
      return `Stopped at ${failedTaskId}${checkPart}. Retry resumes from ${failedTaskId}.`;
    }
    return 'Plan failed. Retry to resume from the last checkpoint.';
  }

  // ── 10. Cancelled ───────────────────────────────────────────────────────────
  return 'Plan was cancelled.';
}

// ── planState ──────────────────────────────────────────────────────────────────

/**
 * Gather `describePlan`'s input from live run state and the plan's task rows
 * (wave order). A plan with no live record, or left pending by a plan set that
 * has ended, never ran; a member still queued in the active set is pending.
 */
export function planState(
  run: RunState,
  planId: string,
  rows: readonly TaskRowModel[],
  waves: number,
): PlanState {
  const live = run.plans[planId];
  if (!live || (live.phase === 'pending' && queuePosition(run, planId) === null)) {
    const tasksTotal = live?.tasksTotal ?? rows.length;
    return { id: planId, phase: 'never_run', tasksTotal, tasksDone: 0, tasksActive: 0, tasksAccepted: 0, waves };
  }
  const failed = rows.find((r) => r.status === 'failed');
  const check = failed?.checks.find((c) => c.status === 'failed');
  return {
    id: planId,
    phase: live.phase,
    tasksTotal: live.tasksTotal,
    tasksDone: live.tasksDone,
    tasksActive: Object.values(run.tasks).filter((t) => t.planId === planId && t.status === 'active').length,
    tasksAccepted: live.tasksAccepted,
    waves,
    failedTaskId: failed?.id ?? null,
    failedCheck: check ? check.phase || check.name : null,
    durationMs:
      live.startedAtMs != null && live.finishedAtMs != null ? live.finishedAtMs - live.startedAtMs : null,
    waitReason: waitReason(run, planId),
  };
}
