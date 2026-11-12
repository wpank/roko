// @vitest-environment jsdom
/**
 * Acceptance for gap-c1726f: the glass's TIME and CHECKS follow the one definition every surface uses.
 * - TIME runs from when roko accepted the run (`last_run.started_at`) and, once it has ended, reads roko's own
 *   `run_completed.duration_ms`.
 * - CHECKS is "x of y": y counts each task's steps, the workspace's rungs when the plan runs them and the plan's
 *   own checks, from the start. x counts those that passed, each once.
 * Phase 7's shape: steps that share a phase word are matched by their place, not counted twice.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { initialRunState } from '@/lib/runState';
import { setStore, stubFetch } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const PLAN = planSummary('p7-ship', 'P7 · Ship', 2);
/** Phase 7's plan: T1 has four `test` steps, T2 three, the plan one check of its own, and no workspace rungs. */
const TASKS: WirePlanTasks = (() => {
  const tasks = planTasks('p7-ship', [
    ['T1', 'Split the Fast attack times', []],
    ['T2', 'Keep the strict pluginval report', ['T1']],
  ]);
  const steps = (n: number) => Array.from({ length: n }, (_, i) => ({ phase: 'test', command: `tools/spec.sh ${i}` }));
  return {
    ...tasks,
    tasks: [
      { ...tasks.tasks[0]!, verify: steps(4), verify_phases: Array(4).fill('test') },
      { ...tasks.tasks[1]!, verify: steps(3), verify_phases: Array(3).fill('test') },
    ],
    plan_checks: [{ phase: 'plan', command: 'tools/check.sh --plugin' }],
    workspace_rungs: false,
  };
})();
const gate = (task: string, i: number): WireDashboardEvent => ({
  type: 'gate_result',
  plan_id: 'p7-ship',
  task_id: task,
  gate: `verify[${i}:test]`,
  passed: true,
});
const T1_PASSED: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'p7-ship', tasks_total: 2 },
  { type: 'task_started', plan_id: 'p7-ship', task_id: 'T1', phase: 'implement' },
  gate('T1', 0), gate('T1', 1), gate('T1', 2), gate('T1', 3),
  { type: 'task_completed', plan_id: 'p7-ship', task_id: 'T1', outcome: 'passed' },
  { type: 'task_started', plan_id: 'p7-ship', task_id: 'T2', phase: 'implement' },
];
const PASSED: WireDashboardEvent[] = [
  ...T1_PASSED,
  gate('T2', 0), gate('T2', 1), gate('T2', 2),
  { type: 'task_completed', plan_id: 'p7-ship', task_id: 'T2', outcome: 'passed' },
  { type: 'plan_check_finished', plan_id: 'p7-ship', passed: true, name: 'check.sh', step: 1, steps: 1 },
  { type: 'plan_completed', plan_id: 'p7-ship', success: true },
  { type: 'run_completed', outcome: 'succeeded', duration_ms: 379_014 },
];

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const label = (name: string) =>
  document.querySelector(`[data-reading="${name}"] svg`)?.getAttribute('aria-label') ?? '';
const detail = (startedAtMs: number) => ({
  ...PLAN,
  last_run: {
    run_id: '31e52490-4366-4ca8-9aa5-7e1e81b45fc1',
    status: 'running',
    passed_tasks: [],
    tasks_left: ['T1', 'T2'],
    resumable: false,
    max_usd: 1,
    started_at: new Date(startedAtMs).toISOString(),
  },
});
const seed = { seed: [[queryKeys.planTasks('p7-ship'), TASKS] as const] };

describe('CHECKS', () => {
  it('counts each step once, by its place, and the plan’s own check, from the start', async () => {
    stubFetch([{ path: '/api/plans/p7-ship', body: detail(Date.now() - 60_000) }]);
    renderPlanPage(PLAN, TASKS, T1_PASSED, seed);
    expect(label('checks')).toBe('checks: 4 of 8 checks passed');
  });

  it('reads 8 of 8 once the run has passed, as roko counted it', () => {
    stubFetch([{ path: '/api/plans/p7-ship', body: detail(Date.now() - 400_000) }]);
    renderPlanPage(PLAN, TASKS, PASSED, seed);
    expect(label('checks')).toBe('checks: 8 of 8 checks passed');
  });
});

describe('TIME', () => {
  it('runs from when roko accepted the run, not from when its tasks began', async () => {
    const now = Date.now();
    stubFetch([{ path: '/api/plans/p7-ship', body: detail(now - 128_000) }]);
    // The tasks began 28 s after roko accepted the run: its spec gate ran first.
    renderPlanPage(PLAN, TASKS, T1_PASSED, { ...seed, startMs: now - 100_000 });
    await waitFor(() => expect(label('time')).toMatch(/^time: 2 minutes (8|9) seconds$/));
  });

  it('reads roko’s own duration once the run has ended', () => {
    stubFetch([{ path: '/api/plans/p7-ship', body: detail(Date.now() - 400_000) }]);
    renderPlanPage(PLAN, TASKS, PASSED, { ...seed, startMs: Date.now() - 351_000 });
    expect(label('time')).toBe('time: 6 minutes 19 seconds');
  });
});
