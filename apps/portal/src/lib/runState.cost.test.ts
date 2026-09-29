/**
 * bug-b69a47: a run's cost is what that run spent — not everything the server
 * has recorded since it started (`totals.costUsd`).
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import { applyEvent, fromSnapshot, initialRunState, runCostUsd } from '@/lib/runState';
import type { RunState } from '@/lib/runState';

function fold(events: WireDashboardEvent[], run: RunState = initialRunState()): RunState {
  return events.reduce((state, event) => applyEvent(state, event, 1_000), run);
}

function cost(planId: string, value: number): WireDashboardEvent {
  return { type: 'efficiency_event', plan_id: planId, task_id: 'T1', metric: 'cost_usd', value };
}

function runOf(planId: string, spend: number[]): WireDashboardEvent[] {
  return [
    { type: 'plan_set_loaded', plans: [{ plan_id: planId, title: planId, tasks_total: 1 }] },
    { type: 'plan_started', plan_id: planId, tasks_total: 1 },
    { type: 'task_started', plan_id: planId, task_id: 'T1', phase: 'implement' },
    ...spend.map((value) => cost(planId, value)),
  ];
}

const END_HELLO: WireDashboardEvent[] = [
  { type: 'task_completed', plan_id: 'hello', task_id: 'T1', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'hello', success: true },
  { type: 'run_completed', outcome: 'succeeded', duration_ms: 1_000 },
];

describe('runCostUsd', () => {
  it('cost restarts with each run of the same plan', () => {
    const first = fold([...runOf('hello', [4, 6]), ...END_HELLO]);
    expect(runCostUsd(first)).toBe(10);

    const second = fold(runOf('hello', []), first);
    expect(runCostUsd(second)).toBe(0);
    expect(runCostUsd(fold([cost('hello', 0.25)], second))).toBe(0.25);
    // The lifetime total still has both runs.
    expect(fold([cost('hello', 0.25)], second).totals.costUsd).toBe(10.25);
  });

  it('cost restarts with each run of another plan', () => {
    const first = fold([...runOf('hello', [10]), ...END_HELLO]);
    const second = fold(runOf('notes', [0.5]), first);
    expect(runCostUsd(second)).toBe(0.5);
  });

  it('sums every plan of a set run, finished ones included', () => {
    const run = fold([
      {
        type: 'plan_set_loaded',
        plans: [
          { plan_id: 'a', title: 'A', tasks_total: 1 },
          { plan_id: 'b', title: 'B', tasks_total: 1 },
        ],
      },
      { type: 'plan_started', plan_id: 'a', tasks_total: 1 },
      cost('a', 1),
      { type: 'plan_completed', plan_id: 'a', success: true },
      { type: 'plan_started', plan_id: 'b', tasks_total: 1 },
      cost('b', 2),
    ]);
    expect(runCostUsd(run)).toBe(3);
  });

  it('leaves out spend outside the run, such as generating or revising the plan', () => {
    const generate: WireDashboardEvent = {
      type: 'efficiency_event',
      plan_id: 'hello',
      task_id: 'generate',
      metric: 'cost_usd',
      value: 0.5,
    };
    const revise: WireDashboardEvent = { ...generate, task_id: 'revise', value: 1 };
    const [setLoaded, ...rest] = runOf('hello', [0.25]);
    const run = fold([setLoaded!, generate, ...rest, ...END_HELLO, revise]);
    expect(run.plans['hello']?.costUsd).toBe(0.25);
    expect(runCostUsd(run)).toBe(0.25);
    expect(run.totals.costUsd).toBe(1.75);
  });

  it('counts the running plans when the plan set is from an earlier run', () => {
    let run = fold([...runOf('hello', [10]), ...END_HELLO]);
    // A run that announces no plan set.
    run = fold([{ type: 'plan_started', plan_id: 'notes', tasks_total: 1 }, cost('notes', 0.5)], run);
    expect(runCostUsd(run)).toBe(0.5);
  });

  it("cost restarts with each run after a reload: the snapshot's plan costs, not its lifetime total", () => {
    const snapshot: WireDashboardSnapshot = {
      plans: {
        hello: {
          plan_id: 'hello',
          phase: 'started',
          active: true,
          tasks_total: 2,
          tasks_done: 1,
          tasks_failed: 0,
          started_at_ms: 500,
          finished_at_ms: null,
          cost_usd: 0.21,
        },
      },
      plan_set: { plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 2 }], tasks_total: 2, loaded_at_ms: 400 },
      tasks: {},
      agents: {},
      gates: [],
      errors: [],
      stats: { cost_usd_total: 10.21, total_input_tokens: 0, total_output_tokens: 0 },
    };
    expect(runCostUsd(fromSnapshot(snapshot, 1_000))).toBe(0.21);
  });
});
