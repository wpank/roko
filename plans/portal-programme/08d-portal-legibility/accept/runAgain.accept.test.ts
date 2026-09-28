/**
 * Acceptance: running a plan again starts a new run — no "4/2", no "↻2" on
 * tasks that ran once, no "— attempt 2 —" — while a retry after a failure keeps
 * the tasks that passed, and a task retried inside one run counts once.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent } from '@/api/contracts';
import { applyEvent, initialRunState, taskKey } from '@/lib/runState';
import type { RunState } from '@/lib/runState';

const SET: WireDashboardEvent = {
  type: 'plan_set_loaded',
  plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }],
};
const STARTED: WireDashboardEvent = { type: 'plan_started', plan_id: 'hello', tasks_total: 2 };
const start = (task: string): WireDashboardEvent => ({ type: 'task_started', plan_id: 'hello', task_id: task, phase: 'implement' });
const done = (task: string, outcome = 'passed'): WireDashboardEvent => ({ type: 'task_completed', plan_id: 'hello', task_id: task, outcome });
const cost = (task: string, value: number): WireDashboardEvent => ({ type: 'efficiency_event', plan_id: 'hello', task_id: task, metric: 'cost_usd', value });

const FIRST_RUN: WireDashboardEvent[] = [
  SET,
  STARTED,
  start('T01'),
  cost('T01', 0.5),
  done('T01'),
  start('T02'),
  done('T02'),
  { type: 'plan_completed', plan_id: 'hello', success: true },
  { type: 'run_completed', outcome: 'succeeded', duration_ms: 1_000 },
];

const FAILED_RUN: WireDashboardEvent[] = [
  SET,
  STARTED,
  start('T01'),
  done('T01'),
  start('T02'),
  done('T02', 'failed'),
  { type: 'plan_completed', plan_id: 'hello', success: false },
  { type: 'run_completed', outcome: 'failed', duration_ms: 1_000 },
];

function fold(events: WireDashboardEvent[]): RunState {
  return events.reduce((run, e, i) => applyEvent(run, e, 1_000 + i * 10), initialRunState());
}

const plan = (run: RunState) => run.plans['hello']!;
const task = (run: RunState, id: string) => run.tasks[taskKey('hello', id)];
const kinds = (run: RunState, id: string) => (run.transcripts[taskKey('hello', id)]?.entries ?? []).map((e) => e.kind);

describe('Run again after a successful run', () => {
  it('starts from 0/2 with the server’s extra pre-run plan_started', () => {
    const run = fold([...FIRST_RUN, STARTED, SET, STARTED, start('T01'), done('T01')]);
    expect(plan(run).phase).toBe('running');
    expect([plan(run).tasksDone, plan(run).tasksTotal]).toEqual([1, 2]);
    expect(task(run, 'T01')?.attempts).toBe(1);
    expect(task(run, 'T02')).toBeUndefined();
  });

  it('starts from 0/2 without it', () => {
    const run = fold([...FIRST_RUN, SET, STARTED, start('T01'), done('T01')]);
    expect([plan(run).tasksDone, plan(run).tasksTotal]).toEqual([1, 2]);
    expect(task(run, 'T01')?.attempts).toBe(1);
  });

  it('adds no attempt divider to a task that ran once per run', () => {
    const run = fold([
      ...FIRST_RUN,
      { type: 'agent_output', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', content: 'first run' },
      SET,
      STARTED,
      start('T01'),
    ]);
    expect(kinds(run, 'T01')).not.toContain('divider');
  });

  it('counts two runs to 2/2, never 4/2', () => {
    const run = fold([...FIRST_RUN, SET, STARTED, start('T01'), done('T01'), start('T02'), done('T02')]);
    expect([plan(run).tasksDone, plan(run).tasksTotal]).toEqual([2, 2]);
    expect(plan(run).tasksFailed).toBe(0);
  });

  it('resets the cost of the run', () => {
    const run = fold([...FIRST_RUN, SET, STARTED]);
    expect(plan(run).costUsd).toBe(0);
  });
});

describe('Retry after a failed run', () => {
  it('keeps the task that passed and clears the failure', () => {
    const run = fold([...FAILED_RUN, SET, STARTED]);
    expect(task(run, 'T01')?.status).toBe('passed');
    expect(task(run, 'T02')).toBeUndefined();
    expect([plan(run).tasksDone, plan(run).tasksFailed]).toEqual([1, 0]);
  });

  it('re-runs the failed task as a first attempt and finishes at 2/2', () => {
    const run = fold([...FAILED_RUN, SET, STARTED, start('T02'), done('T02')]);
    expect(task(run, 'T02')?.attempts).toBe(1);
    expect(kinds(run, 'T02')).not.toContain('divider');
    expect([plan(run).tasksDone, plan(run).tasksFailed, plan(run).tasksTotal]).toEqual([2, 0, 2]);
  });

  it('does not double-count a passed task that runs again', () => {
    const run = fold([...FAILED_RUN, SET, STARTED, start('T01'), done('T01'), start('T02'), done('T02')]);
    expect([plan(run).tasksDone, plan(run).tasksFailed]).toEqual([2, 0]);
    expect(task(run, 'T01')?.attempts).toBe(1);
  });
});

describe('a retry inside one run', () => {
  it('counts the task once, shows the second attempt and its divider', () => {
    const run = fold([SET, STARTED, start('T01'), done('T01', 'failed'), start('T01'), done('T01')]);
    expect([plan(run).tasksDone, plan(run).tasksFailed]).toEqual([1, 0]);
    expect(task(run, 'T01')?.attempts).toBe(2);
    expect(kinds(run, 'T01')).toContain('divider');
  });

  it('never lets a counter go below zero', () => {
    let run = fold([SET, STARTED, start('T01'), done('T01')]);
    run = { ...run, plans: { ...run.plans, hello: { ...plan(run), tasksDone: 0 } } };
    run = applyEvent(run, start('T01'), 9_000);
    expect(plan(run).tasksDone).toBe(0);
  });
});

describe('a plan that is still running', () => {
  it('keeps its progress when the plan set is announced again', () => {
    const run = fold([SET, STARTED, start('T01'), done('T01'), SET]);
    expect(plan(run).phase).toBe('running');
    expect(plan(run).tasksDone).toBe(1);
  });
});
