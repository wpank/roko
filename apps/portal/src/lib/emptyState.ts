/**
 * emptyState.ts — pure function that describes what the system is doing.
 *
 * `describeEmpty` never returns an empty string or the phrase "no data".
 * It always describes the *current system state*, not the absence of content.
 *
 * Mirrors the philosophy in mori's `widgets/agent_output.rs:327`.
 */

import type { ConnectionStatus } from '@/lib/bootstrap';
import type { PlanPhase } from '@/lib/runState';
import { compactDuration } from '@/lib/formatters';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface EmptyStateInput {
  /** Folder name of the workspace (shown in the "no plans" message). */
  workspace: string;
  /** Current SSE connection status. */
  connection: ConnectionStatus;
  /** Total number of plans in the workspace. */
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
  };
}

// ── describeEmpty ──────────────────────────────────────────────────────────────

/**
 * Return a human-readable sentence describing the current system state.
 *
 * Cases are evaluated in strict precedence order:
 * 1. Connection is lost → reconnecting message.
 * 2. No plans in workspace → prompt to describe work.
 * 3. Plans exist but none selected → prompt to select.
 * 4. Plan is never-run → show task / wave count.
 * 5. Plan is running:
 *    a. Nothing active, tasks queued → scheduler waiting message.
 *    b. Every task dispatched → dispatched message.
 *    c. Otherwise → generic running progress.
 * 6. Plan completed with accepted tasks → accepted message.
 * 7. Plan completed cleanly → finished + duration + verification count.
 * 8. Plan failed → stopped-at message with retry hint.
 * 9. Plan cancelled → cancelled message.
 */
export function describeEmpty(input: EmptyStateInput): string {
  const { connection, workspace, planCount, plan } = input;

  // ── 1. Connection problems ──────────────────────────────────────────────────
  if (connection === 'disconnected' || connection === 'error') {
    return 'Lost the server; reconnecting.';
  }

  // ── 2. No plans in workspace ────────────────────────────────────────────────
  if (planCount === 0) {
    return `No plans in ${workspace} yet. Describe what you want to build.`;
  }

  // ── 3. Plans exist but none selected ───────────────────────────────────────
  if (!plan) {
    return 'Select a plan, or press n to describe a new one.';
  }

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

  // ── 6. Completed with accepted tasks ───────────────────────────────────────
  if (phase === 'completed' && tasksAccepted > 0) {
    const taskWord = tasksAccepted === 1 ? 'task was' : 'tasks were';
    return `Finished; ${tasksAccepted} ${taskWord} accepted despite failing checks.`;
  }

  // ── 7. Completed cleanly ────────────────────────────────────────────────────
  if (phase === 'completed') {
    const duration = compactDuration(plan.durationMs);
    return `Finished in ${duration} — ${tasksDone} of ${tasksTotal} verified.`;
  }

  // ── 8. Failed ───────────────────────────────────────────────────────────────
  if (phase === 'failed') {
    const { failedTaskId, failedCheck } = plan;
    if (failedTaskId) {
      const checkPart = failedCheck ? ` — ${failedCheck} failed` : '';
      return `Stopped at ${failedTaskId}${checkPart}. Retry resumes from ${failedTaskId}.`;
    }
    return 'Plan failed. Retry to resume from the last checkpoint.';
  }

  // ── 9. Cancelled ────────────────────────────────────────────────────────────
  return 'Plan was cancelled.';
}
