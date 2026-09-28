/**
 * alerts.ts — single-channel alert picker.
 *
 * One alert at a time. Candidates are evaluated in rank order; the first
 * candidate whose key is not `dismissedKey` is returned.
 *
 * Rank order:
 * 1. requestError (portal action rejected by the server)
 *    → newest run.errors entry (execution error)
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
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/**
 * Return the first non-empty line from `output` that does not start with
 * '$ ' (i.e. is not a shell-echo line).  Returns null when no such line exists.
 */
function firstCheckOutputLine(output: string): string | null {
  for (const raw of output.split('\n')) {
    const line = raw.trim();
    if (line.length > 0 && !raw.startsWith('$ ')) {
      return line;
    }
  }
  return null;
}

/**
 * Build the human-readable text for a failed task alert.
 *
 * Format: "<title> failed: <phase> — <first useful output line>"
 * The trailing " — <detail>" part is omitted when no useful line is found.
 */
function failedTaskText(task: TaskRun): string {
  const title = task.title.length > 0 ? task.title : task.taskId;
  const base = `${title} failed: ${task.phase}`;

  const failedCheck = task.checks.find((c) => c.status === 'failed');
  const detail = failedCheck ? firstCheckOutputLine(failedCheck.output) : null;

  return detail ? `${base} — ${detail}` : base;
}

// ── pickAlert ─────────────────────────────────────────────────────────────────

/**
 * Return the highest-priority alert that is not currently dismissed, or null
 * when no alert conditions are present.
 */
export function pickAlert(input: AlertInput): Alert | null {
  const { run, connection, selectedPlanId, validationErrors, requestError, requestNotice, dismissedKey } = input;

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
  if (run.errors.length > 0) {
    const newest = run.errors[run.errors.length - 1]!;
    const key = `run-error:${newest.atMs}`;
    const alert = tryAlert({ key, severity: 'error', text: newest.message, actions: [] });
    if (alert) return alert;
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
        text: failedTaskText(task),
        actions: [
          { kind: 'select-task', planId: task.planId, taskId: task.taskId },
          { kind: 'retry', planId: task.planId },
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
      text: `${validationErrors} validation errors`,
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
