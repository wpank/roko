/**
 * alerts.ts — single-channel alert picker.
 *
 * One alert at a time. Candidates are evaluated in rank order; the first
 * candidate whose key is not `dismissedKey` is returned.
 *
 * Rank order:
 * 1a. requestError (portal action rejected by the server)
 * 1b. requestNotice (server cannot do this — informational)
 * 1c. newest run.errors entry (execution error); the server's generic
 *     "plan <id> completed with task-level failures" yields to rank 2 when
 *     the plan has a known failed task, and otherwise reads "<title> failed".
 * 2. Failed task, preferring the selected plan
 * 3. validationErrors > 0 on the selected plan
 * 4. Connection disconnected or error
 */

import type { RunState, TaskRun } from './runState';
import type { ConnectionStatus } from './bootstrap';

// ── Exported types ────────────────────────────────────────────────────────────

export type AlertAction =
  | { kind: 'select-task'; planId: string; taskId: string }
  | { kind: 'retry'; planId: string }
  | { kind: 'reconnect' };

export interface Alert {
  key: string;
  severity: 'error' | 'warning' | 'info';
  text: string;
  actions: AlertAction[];
}

export interface AlertInput {
  run: RunState;
  connection: ConnectionStatus;
  selectedPlanId: string | null;
  validationErrors: number;
  requestError: string | null;
  /** A server notice about an unsupported operation — shown as an info alert. */
  requestNotice?: string | null;
  dismissedKey: string | null;
  /**
   * Map of plan id → display title from the plan list (GET /api/plans).
   * Provides the most up-to-date title; falls back to the run's plan record,
   * then to the bare plan id.
   */
  planTitles?: Readonly<Record<string, string>>;
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/**
 * Resolve a plan's display title: planTitles[id] → run.plans[id].title → id.
 */
function resolvePlanTitle(
  planId: string,
  planTitles: Readonly<Record<string, string>> | undefined,
  run: RunState,
): string {
  return planTitles?.[planId] ?? run.plans[planId]?.title ?? planId;
}

/**
 * Return the label for the first failed check: its phase if set, else its name.
 * Returns null when the task has no failed check.
 */
function failedCheckLabel(task: TaskRun): string | null {
  const check = task.checks.find((c) => c.status === 'failed');
  if (!check) return null;
  return check.phase || check.name;
}

/**
 * Build the human-readable text for a failed task alert.
 *
 * Format: "<plan title>: <task id> failed (<check label> check)"
 * The trailing " (<check label> check)" is omitted when no failed check exists.
 */
function failedTaskText(
  task: TaskRun,
  planTitles: Readonly<Record<string, string>> | undefined,
  run: RunState,
): string {
  const title = resolvePlanTitle(task.planId, planTitles, run);
  const checkLabel = failedCheckLabel(task);
  const base = `${title}: ${task.taskId} failed`;
  return checkLabel ? `${base} (${checkLabel} check)` : base;
}

// ── pickAlert ─────────────────────────────────────────────────────────────────

/**
 * Return the highest-priority alert that is not currently dismissed, or null
 * when no alert conditions are present.
 */
export function pickAlert(input: AlertInput): Alert | null {
  const { run, connection, selectedPlanId, validationErrors, requestError, requestNotice, dismissedKey, planTitles } = input;

  /** Return `alert` if it is not the dismissed candidate, otherwise null. */
  function tryAlert(alert: Alert): Alert | null {
    return alert.key === dismissedKey ? null : alert;
  }

  // ── Rank 1a: requestError ────────────────────────────────────────────────
  if (requestError !== null) {
    // Truncate to 120 chars to keep the key legible; the full text is in `text`.
    const key = `request-error:${requestError.slice(0, 120)}`;
    const alert = tryAlert({ key, severity: 'error', text: requestError, actions: [] });
    if (alert) return alert;
  }

  // ── Rank 1b: requestNotice (server cannot do this — informational) ───────
  if (requestNotice != null) {
    const key = `request-notice:${requestNotice.slice(0, 120)}`;
    const alert = tryAlert({ key, severity: 'info', text: requestNotice, actions: [] });
    if (alert) return alert;
  }

  // ── Rank 1c: newest run.errors entry ────────────────────────────────────
  // The server emits "plan <id> completed with task-level failures" as a
  // catch-all; hide it when rank 2 already has a specific task to show.
  // Without a known failed task, reword it to "<title> failed" (same key).
  // Any other run error passes through unchanged.
  if (run.errors.length > 0) {
    const newest = run.errors[run.errors.length - 1]!;
    const key = `run-error:${newest.atMs}`;

    const genericMatch = newest.message.match(/^plan (\S+) completed with task-level failures$/);
    if (genericMatch) {
      const planId = genericMatch[1]!;
      const hasFailed = Object.values(run.tasks).some(
        (t) => t.planId === planId && t.status === 'failed',
      );
      if (!hasFailed) {
        // No specific failed task — reword to a human-readable summary.
        const title = resolvePlanTitle(planId, planTitles, run);
        const alert = tryAlert({ key, severity: 'error', text: `${title} failed`, actions: [] });
        if (alert) return alert;
      }
      // else: yield to rank 2 — the specific task failure says it better.
    } else {
      const alert = tryAlert({ key, severity: 'error', text: newest.message, actions: [] });
      if (alert) return alert;
    }
  }

  // ── Rank 2: failed tasks ─────────────────────────────────────────────────
  {
    const allFailed = Object.values(run.tasks).filter((t) => t.status === 'failed');

    // Preferred plan first, then the rest (stable — preserves insertion order).
    const sorted = [
      ...allFailed.filter((t) => t.planId === selectedPlanId),
      ...allFailed.filter((t) => t.planId !== selectedPlanId),
    ];

    for (const task of sorted) {
      // The attempt count is embedded in the key so a retried failure
      // produces a fresh, non-dismissed alert.
      const key = `task-failed:${task.planId}:${task.taskId}:${task.attempts}`;
      const alert = tryAlert({
        key,
        severity: 'error',
        text: failedTaskText(task, planTitles, run),
        // Only Show (select-task) — the plan header's primary action is the one Retry.
        actions: [
          { kind: 'select-task', planId: task.planId, taskId: task.taskId },
        ],
      });
      if (alert) return alert;
    }
  }

  // ── Rank 3: validation errors on selected plan ───────────────────────────
  if (validationErrors > 0 && selectedPlanId !== null) {
    const key = `validation:${selectedPlanId}:${validationErrors}`;
    const alert = tryAlert({
      key,
      severity: 'warning',
      text: `${validationErrors} validation error${validationErrors === 1 ? '' : 's'}`,
      actions: [],
    });
    if (alert) return alert;
  }

  // ── Rank 4: connection issue ─────────────────────────────────────────────
  if (connection === 'disconnected' || connection === 'error') {
    const key = `connection:${connection}`;
    const alert = tryAlert({
      key,
      severity: 'error',
      text: 'Lost the server; reconnecting.',
      actions: [{ kind: 'reconnect' }],
    });
    if (alert) return alert;
  }

  return null;
}
