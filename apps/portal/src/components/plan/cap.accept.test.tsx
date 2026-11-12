// @vitest-environment jsdom
/**
 * Acceptance for gap-acfe4b: SPENT on the glass is measured against the cap the run's request set, which roko serve
 * names in the plan's detail (`last_run.max_usd`, as Hermes shows "of $9.00"), else against the plan's budget. When
 * neither is known, it shows the spend alone: unknown isn't unlimited, so it never says "no cap set".
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, waitFor } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { initialRunState } from '@/lib/runState';
import { setStore, stubFetch, textOf } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const PLAN = planSummary('greeting', 'Say hello from the command line', 2);
const TASKS = planTasks('greeting', [
  ['T1', 'Write the greeting module', []],
  ['T2', 'Add the command-line entry point', ['T1']],
]);
const RUNNING: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 2 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'greeting/T1', plan_id: 'greeting', task_id: 'T1', attempt: 1, role: 'implementer', model: 'glm-4.7' },
  { type: 'efficiency_event', plan_id: 'greeting', task_id: 'T1', metric: 'cost_usd', value: 0.1007 },
  { type: 'attempt_settled', plan_id: 'greeting', task_id: 'T1', attempt: 1, verdict: 'passed', model: 'glm-4.7', cost_usd: 0.1007 },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T1', outcome: 'passed' },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T2', phase: 'implement' },
];
const lastRun = (maxUsd?: number) => ({
  run_id: '23837ede-1d23-4d53-b8a9-fbfa362760d0',
  status: 'running',
  passed_tasks: ['T1'],
  tasks_left: ['T2'],
  resumable: false,
  ...(maxUsd === undefined ? {} : { max_usd: maxUsd }),
});

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const spent = () => document.querySelector('[data-reading="spent"]')!;
const spentLabel = () => spent().querySelector('svg')!.getAttribute('aria-label');

describe('SPENT on the glass', () => {
  it('is measured against the cap the run’s request set, over the plan’s budget', async () => {
    stubFetch([{ path: '/api/plans/greeting', body: { ...PLAN, last_run: lastRun(9) } }]);
    renderPlanPage(PLAN, TASKS, RUNNING, { seed: [[queryKeys.planCosts('greeting'), 3]] });
    await waitFor(() => expect(spentLabel()).toBe('spent: $0.10 of $9.00'));
    expect(textOf(spent().querySelector('.rk-cell-note'))).toBe('of $9.00');
    expect(spent().querySelector('.rk-meter')).not.toBeNull();
  });

  it('is measured against the plan’s budget when the run set no cap', async () => {
    stubFetch([{ path: '/api/plans/greeting', body: { ...PLAN, last_run: lastRun() } }]);
    renderPlanPage(PLAN, TASKS, RUNNING, { seed: [[queryKeys.planCosts('greeting'), 3]] });
    await waitFor(() => expect(spentLabel()).toBe('spent: $0.10 of $3.00'));
  });

  it('shows the spend alone when no cap is known, and never says “no cap set”', async () => {
    const fetch = stubFetch([{ path: '/api/plans/greeting', body: { ...PLAN, last_run: lastRun() } }]);
    renderPlanPage(PLAN, TASKS, RUNNING, { seed: [[queryKeys.planCosts('greeting'), null]] });
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    expect(spentLabel()).toBe('spent: $0.10 spent');
    expect(textOf(spent())).not.toContain('no cap set');
    expect(textOf(spent().querySelector('.rk-cell-note'))).toBe('');
    expect(spent().querySelector('.rk-meter')).toBeNull();
  });
});
