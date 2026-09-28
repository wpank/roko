import { describe, it, expect } from 'vitest';
import { buildRungs, checkFocusTask } from './rungs';
import type { CheckRun, RunState } from './runState';
import { initialRunState } from './runState';

// ── helpers ────────────────────────────────────────────────────────────────────

function makeCheck(
  index: number | null,
  phase: string,
  status: CheckRun['status'],
): CheckRun {
  const name = index !== null ? `verify[${index}:${phase}]` : phase;
  return { name, index, phase, status, output: '' };
}

function makeTask(
  planId: string,
  taskId: string,
  status: 'active' | 'passed' | 'failed',
  checks: CheckRun[] = [],
) {
  return {
    planId,
    taskId,
    title: taskId,
    status,
    phase: 'impl',
    attempts: 1,
    startedAtMs: null,
    finishedAtMs: null,
    agentId: null,
    role: null,
    model: null,
    costUsd: 0,
    inputTokens: 0,
    outputTokens: 0,
    checks,
  };
}

function stateWithTasks(
  tasks: Record<string, ReturnType<typeof makeTask>>,
): RunState {
  return { ...initialRunState(), tasks } as RunState;
}

// ── buildRungs ─────────────────────────────────────────────────────────────────

describe('buildRungs', () => {
  it('partly-reached ladder: reached rungs show their state, unreached are pending', () => {
    const declared = [
      { phase: 'compile' },
      { phase: 'unit' },
      { phase: 'integration' },
      { phase: 'e2e' },
    ];
    const checks: CheckRun[] = [
      makeCheck(0, 'compile', 'passed'),
      makeCheck(1, 'unit', 'passed'),
    ];
    const rungs = buildRungs(checks, declared);
    expect(rungs).toHaveLength(4);
    expect(rungs[0]).toMatchObject({ index: 0, label: 'compile', state: 'passed' });
    expect(rungs[1]).toMatchObject({ index: 1, label: 'unit', state: 'passed' });
    expect(rungs[2]).toMatchObject({ index: 2, label: 'integration', state: 'pending' });
    expect(rungs[3]).toMatchObject({ index: 3, label: 'e2e', state: 'pending' });
  });

  it('failed step maps to failed rung state', () => {
    const declared = [{ phase: 'lint' }, { phase: 'test' }];
    const checks: CheckRun[] = [
      makeCheck(0, 'lint', 'passed'),
      makeCheck(1, 'test', 'failed'),
    ];
    const rungs = buildRungs(checks, declared);
    expect(rungs[1]).toMatchObject({ index: 1, label: 'test', state: 'failed' });
  });

  it('running step maps to running rung state', () => {
    const declared = [{ phase: 'build' }, { phase: 'check' }];
    const checks: CheckRun[] = [
      makeCheck(0, 'build', 'passed'),
      makeCheck(1, 'check', 'running'),
    ];
    const rungs = buildRungs(checks, declared);
    expect(rungs[0]).toMatchObject({ state: 'passed' });
    expect(rungs[1]).toMatchObject({ state: 'running' });
  });

  it('extra unindexed checks are appended after declared rungs', () => {
    const declared = [{ phase: 'typecheck' }];
    const checks: CheckRun[] = [
      makeCheck(0, 'typecheck', 'passed'),
      // index null — no structured name
      { name: 'custom-gate', index: null, phase: 'custom-gate', status: 'failed', output: '' },
      // index beyond declared length
      makeCheck(2, 'extra', 'running'),
    ];
    const rungs = buildRungs(checks, declared);
    // declared rung first
    expect(rungs[0]).toMatchObject({ index: 0, label: 'typecheck', state: 'passed' });
    // null-index extra appended
    expect(rungs[1]).toMatchObject({ index: null, label: 'custom-gate', state: 'failed' });
    // beyond-range index extra appended
    expect(rungs[2]).toMatchObject({ index: 2, label: 'extra', state: 'running' });
    expect(rungs).toHaveLength(3);
  });

  it('no declared list: one rung per check in order', () => {
    const checks: CheckRun[] = [
      makeCheck(0, 'compile', 'passed'),
      makeCheck(1, 'test', 'failed'),
      { name: 'orphan', index: null, phase: '', status: 'running', output: '' },
    ];
    const rungs = buildRungs(checks, null);
    expect(rungs).toHaveLength(3);
    expect(rungs[0]).toMatchObject({ index: 0, label: 'compile', state: 'passed' });
    expect(rungs[1]).toMatchObject({ index: 1, label: 'test', state: 'failed' });
    // phase is empty, index is null → falls back to name
    expect(rungs[2]).toMatchObject({ index: null, label: 'orphan', state: 'running' });
  });

  it('declared step with empty phase falls back to verify[i] label', () => {
    const declared = [{ phase: '' }, { phase: 'test' }];
    const checks: CheckRun[] = [makeCheck(0, '', 'passed')];
    const rungs = buildRungs(checks, declared);
    expect(rungs[0]).toMatchObject({ label: 'verify[0]', state: 'passed' });
    expect(rungs[1]).toMatchObject({ label: 'test', state: 'pending' });
  });

  it('empty checks with declared list produces all-pending rungs', () => {
    const declared = [{ phase: 'a' }, { phase: 'b' }, { phase: 'c' }];
    const rungs = buildRungs([], declared);
    expect(rungs).toHaveLength(3);
    expect(rungs.every((r) => r.state === 'pending')).toBe(true);
  });
});

// ── checkFocusTask ─────────────────────────────────────────────────────────────

describe('checkFocusTask', () => {
  it('returns the selected task when run.tasks knows it', () => {
    const run = stateWithTasks({
      'plan-a/task-1': makeTask('plan-a', 'task-1', 'passed'),
    });
    const result = checkFocusTask(
      run,
      { planId: 'plan-a', taskId: 'task-1' },
      [],
    );
    expect(result).toEqual({ planId: 'plan-a', taskId: 'task-1' });
  });

  it('returns first active task of selected plan when selected task is unknown', () => {
    const run = stateWithTasks({
      'plan-a/task-2': makeTask('plan-a', 'task-2', 'active'),
      'plan-a/task-3': makeTask('plan-a', 'task-3', 'active'),
    });
    const result = checkFocusTask(
      run,
      { planId: 'plan-a', taskId: 'task-X' },
      [],
    );
    // task-2 < task-3 lexicographically
    expect(result).toEqual({ planId: 'plan-a', taskId: 'task-2' });
  });

  it('returns first active task across running plans when no selected plan match', () => {
    const run = stateWithTasks({
      'plan-b/task-1': makeTask('plan-b', 'task-1', 'active'),
      'plan-c/task-1': makeTask('plan-c', 'task-1', 'active'),
    });
    const result = checkFocusTask(
      run,
      { planId: null, taskId: null },
      ['plan-c', 'plan-b'],
    );
    // plan-c is first in runningPlanIds order
    expect(result).toEqual({ planId: 'plan-c', taskId: 'task-1' });
  });

  it('returns null when no active tasks exist anywhere', () => {
    const run = stateWithTasks({
      'plan-a/task-1': makeTask('plan-a', 'task-1', 'passed'),
    });
    const result = checkFocusTask(
      run,
      { planId: null, taskId: null },
      ['plan-a'],
    );
    expect(result).toBeNull();
  });

  it('skips non-active tasks when finding first active for selected plan', () => {
    const run = stateWithTasks({
      'plan-a/task-done': makeTask('plan-a', 'task-done', 'passed'),
      'plan-a/task-live': makeTask('plan-a', 'task-live', 'active'),
    });
    const result = checkFocusTask(
      run,
      { planId: 'plan-a', taskId: 'missing' },
      [],
    );
    expect(result).toEqual({ planId: 'plan-a', taskId: 'task-live' });
  });
});
