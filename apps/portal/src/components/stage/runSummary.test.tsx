// @vitest-environment jsdom
/**
 * The plan view says what its run is doing or came to (design §7; gap-6ff814):
 * the all-dispatched, finished and stopped-at sentences sit under the status
 * line whatever task has focus. Before a run the status line says it alone.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanView } from '@/components/stage/PlanView';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

const PLAN: WirePlanSummary = {
  id: 'hello',
  title: 'Hello world',
  task_count: 2,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
};

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold the hello project', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** T01 passes and T02 starts: every task is dispatched. */
const SECOND_RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'implement' },
];

/** The whole run, T02 ending with `outcome`; plan_started to plan_completed is 5 steps. */
const finished = (outcome: string, success = true): WireDashboardEvent[] => [
  ...SECOND_RUNNING,
  ...(outcome === 'failed'
    ? [{ type: 'gate_result', plan_id: 'hello', task_id: 'T02', gate: 'verify[0:structural]', passed: false } as WireDashboardEvent]
    : []),
  { type: 'task_completed', plan_id: 'hello', task_id: 'T02', outcome },
  { type: 'plan_completed', plan_id: 'hello', success },
];

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

function renderView(events: WireDashboardEvent[], stepMs = 1, selectedTaskId: string | null = null) {
  setStore(foldEvents(events, 1_000, stepMs));
  return renderWithClient(
    <PlanView plan={PLAN} selectedTaskId={selectedTaskId} onSelectTask={() => {}} onRequestError={() => {}} />,
    { seed: [[queryKeys.planTasks('hello'), TASKS], [queryKeys.validation('hello'), VALID]] },
  );
}

const summary = () => document.querySelector('[data-region="run-summary"]');

describe('run summary', () => {
  it('shows the finished sentence for a completed plan, whatever task has focus', () => {
    // 5 steps of 50.4s from plan_started to plan_completed: 4m12s.
    renderView(finished('passed'), 50_400, 'T01');
    expect(textOf(summary())).toBe('Finished in 4m12s — 2 of 2 verified.');
  });

  it('says a task was accepted despite failing checks', () => {
    renderView(finished('accepted_with_failures'));
    expect(textOf(summary())).toBe('Finished; 1 task was accepted despite failing checks.');
  });

  it('shows the stopped-at sentence, naming the failed task and check', () => {
    renderView(finished('failed', false), 1, 'T01');
    expect(textOf(summary())).toBe('Stopped at T02 — structural failed. Retry resumes from T02.');
  });

  it('says every task is dispatched while the last one runs', () => {
    renderView(SECOND_RUNNING, 1, 'T02');
    expect(textOf(summary())).toBe('Every task is dispatched; checks are running.');
  });

  it('sits under the status line', () => {
    renderView(finished('passed'));
    const line = document.querySelector('[data-region="status-line"]')!;
    expect(line.nextElementSibling).toBe(summary());
    expect(summary()!.classList.contains('rd-meta')).toBe(true);
  });

  it('is absent before a run, where the status line says it', () => {
    renderView([]);
    expect(summary()).toBeNull();
    expect(textOf(document.querySelector('[data-region="status-line"]'))).toContain('2 waves');
  });
});
