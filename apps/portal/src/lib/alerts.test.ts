import { describe, it, expect } from 'vitest';
import { pickAlert } from './alerts';
import type { AlertInput } from './alerts';
import { initialRunState } from './runState';
import type { RunState, TaskRun } from './runState';

// ── Test helpers ──────────────────────────────────────────────────────────────

/** Return a baseline AlertInput with no alert conditions. */
function baseInput(overrides: Partial<AlertInput> = {}): AlertInput {
  return {
    run: initialRunState(),
    connection: 'connected',
    selectedPlanId: null,
    validationErrors: 0,
    requestError: null,
    dismissedKey: null,
    ...overrides,
  };
}

/** Return a RunState with one extra failed task appended. */
function withFailedTask(
  run: RunState,
  planId: string,
  taskId: string,
  title: string,
  phase: string,
  attempts: number,
  checkOutput = '',
): RunState {
  const key = `${planId}/${taskId}`;
  const task: TaskRun = {
    planId,
    taskId,
    title,
    status: 'failed',
    phase,
    attempts,
    startedAtMs: null,
    finishedAtMs: null,
    agentId: null,
    role: null,
    model: null,
    costUsd: 0,
    inputTokens: 0,
    outputTokens: 0,
    checks: checkOutput
      ? [{ name: 'verify', index: null, phase, status: 'failed', output: checkOutput }]
      : [],
  };
  return { ...run, tasks: { ...run.tasks, [key]: task } };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('pickAlert', () => {
  // 1. Null when everything is fine
  it('returns null when there are no alert conditions', () => {
    expect(pickAlert(baseInput())).toBeNull();
  });

  // 2. Rank 1a — requestError
  it('rank 1a: emits an error for a server-rejected portal action', () => {
    const alert = pickAlert(
      baseInput({ requestError: 'Server rejected action: plan-xyz not found' }),
    );
    expect(alert).not.toBeNull();
    expect(alert!.severity).toBe('error');
    expect(alert!.text).toBe('Server rejected action: plan-xyz not found');
    expect(alert!.key).toContain('request-error:');
    expect(alert!.actions).toEqual([]);
  });

  // 3. Rank 1b — run.errors (newest entry)
  it('rank 1b: emits an error for the newest run.errors entry', () => {
    const run = {
      ...initialRunState(),
      errors: [
        { message: 'Old error in plan-old', atMs: 500 },
        { message: 'Agent timed out in plan-abc', atMs: 1000 },
      ],
    };
    const alert = pickAlert(baseInput({ run }));
    expect(alert).not.toBeNull();
    expect(alert!.severity).toBe('error');
    expect(alert!.text).toBe('Agent timed out in plan-abc');
    expect(alert!.key).toBe('run-error:1000');
  });

  // 4. Rank 2 — failed task alert names plan, task and failed check
  it('rank 2: names the plan, task and failed check in the alert', () => {
    const run = withFailedTask(
      initialRunState(),
      'plan-1',
      'task-compile',
      'Build binary',
      'compile',
      1,
      '$ cargo build\nerror[E0432]: unresolved import `foo`\n',
    );
    const alert = pickAlert(baseInput({ run }));
    expect(alert).not.toBeNull();
    expect(alert!.severity).toBe('error');
    expect(alert!.text).toBe('plan-1: task-compile failed (compile check)');
    expect(alert!.key).toBe('task-failed:plan-1:task-compile:1');
    expect(alert!.actions).toEqual([
      { kind: 'select-task', planId: 'plan-1', taskId: 'task-compile' },
    ]);
  });

  // 5. Rank 3 — validation errors
  it('rank 3: emits a warning for validation errors on the selected plan', () => {
    const alert = pickAlert(
      baseInput({ selectedPlanId: 'plan-1', validationErrors: 3 }),
    );
    expect(alert).not.toBeNull();
    expect(alert!.severity).toBe('warning');
    expect(alert!.text).toBe('3 validation errors');
    expect(alert!.actions).toEqual([]);
  });

  // 6. Rank 4 — connection disconnected
  it('rank 4: emits a reconnect alert when connection is disconnected', () => {
    const alert = pickAlert(baseInput({ connection: 'disconnected' }));
    expect(alert).not.toBeNull();
    expect(alert!.severity).toBe('error');
    expect(alert!.text).toBe('Lost the server; reconnecting.');
    expect(alert!.actions).toEqual([{ kind: 'reconnect' }]);
  });

  // 7. Rank 4 — connection error (same alert for both disconnected/error)
  it('rank 4: emits a reconnect alert when connection is error', () => {
    const alert = pickAlert(baseInput({ connection: 'error' }));
    expect(alert).not.toBeNull();
    expect(alert!.text).toBe('Lost the server; reconnecting.');
    expect(alert!.key).toBe('connection:error');
  });

  // 8. Precedence — rank 1 beats rank 2 beats rank 3
  it('precedence: requestError takes priority over a failed task and validation errors', () => {
    let run = withFailedTask(initialRunState(), 'plan-1', 'task-1', 'Build', 'compile', 1);
    run = { ...run, errors: [{ message: 'Exec error', atMs: 100 }] };
    const alert = pickAlert(
      baseInput({
        run,
        requestError: 'Server rejected: forbidden',
        selectedPlanId: 'plan-1',
        validationErrors: 5,
      }),
    );
    expect(alert!.key).toContain('request-error:');
  });

  // 9. Dismissal falls through to the next candidate
  it('dismissal falls through: dismissed requestError reveals run.errors', () => {
    const requestError = 'Server rejected action: plan-xyz not found';
    const run = {
      ...initialRunState(),
      errors: [{ message: 'Agent timed out in plan-abc', atMs: 2000 }],
    };
    const dismissedKey = `request-error:${requestError.slice(0, 120)}`;
    const alert = pickAlert(baseInput({ run, requestError, dismissedKey }));
    expect(alert).not.toBeNull();
    expect(alert!.key).toBe('run-error:2000');
    expect(alert!.text).toBe('Agent timed out in plan-abc');
  });

  // 10. Attempt-keyed re-alert: a retried failure generates a new, non-dismissed alert
  it('attempt key: a retried task failure alerts again after the first was dismissed', () => {
    const run1 = withFailedTask(
      initialRunState(),
      'plan-1',
      'task-1',
      'Lint',
      'verify',
      1,
    );
    const alert1 = pickAlert(baseInput({ run: run1 }));
    expect(alert1!.key).toBe('task-failed:plan-1:task-1:1');

    // User dismisses the first-attempt alert; task retries and fails again (attempt 2)
    const run2 = withFailedTask(
      initialRunState(),
      'plan-1',
      'task-1',
      'Lint',
      'verify',
      2,
    );
    const alert2 = pickAlert(
      baseInput({ run: run2, dismissedKey: 'task-failed:plan-1:task-1:1' }),
    );
    expect(alert2).not.toBeNull();
    expect(alert2!.key).toBe('task-failed:plan-1:task-1:2');
  });

  // 11. Rank 2 prefers selected plan over other plans
  it('rank 2: prefers the failed task in the selected plan', () => {
    let run = withFailedTask(
      initialRunState(),
      'plan-other',
      'task-other',
      'Other task',
      'compile',
      1,
    );
    run = withFailedTask(run, 'plan-selected', 'task-sel', 'Selected task', 'verify', 1);

    const alert = pickAlert(baseInput({ run, selectedPlanId: 'plan-selected' }));
    expect(alert).not.toBeNull();
    expect(alert!.key).toBe('task-failed:plan-selected:task-sel:1');
  });

  // 12. Validation alert suppressed when no plan is selected
  it('rank 3: no validation alert when selectedPlanId is null', () => {
    const alert = pickAlert(baseInput({ validationErrors: 5, selectedPlanId: null }));
    expect(alert).toBeNull();
  });
});
