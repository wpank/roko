/**
 * gap-bfd447: every page load rebuilds the run from GET /api/statehub/snapshot,
 * so a plan's times, cost and accepted count must come back from the snapshot
 * — and reach the rail's time and colour and the status line's cost.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WireDashboardSnapshot, WirePlanSummary } from '@/api/contracts';
import { buildPlanRows } from '@/lib/planRows';
import { applyEvent, fromSnapshot, initialRunState } from '@/lib/runState';
import { statusFields } from '@/lib/statusLine';

function snapshot(
  plans: WireDashboardSnapshot['plans'],
  extra: Partial<WireDashboardSnapshot> = {},
): WireDashboardSnapshot {
  return {
    plans,
    tasks: {},
    agents: {},
    gates: [],
    errors: [],
    stats: { cost_usd_total: 12.5, total_input_tokens: 0, total_output_tokens: 0 },
    ...extra,
  };
}

function disk(id: string): WirePlanSummary {
  return {
    id,
    title: id,
    task_count: 3,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'ready',
    old_format: false,
    estimated_minutes: 4,
  };
}

/** A finished plan with one task accepted with failures, and a running one. */
const PLANS: WireDashboardSnapshot['plans'] = {
  hello: {
    plan_id: 'hello',
    phase: 'completed',
    active: false,
    tasks_total: 3,
    tasks_done: 3,
    tasks_failed: 0,
    tasks_accepted_with_failures: 1,
    started_at_ms: 10_000,
    finished_at_ms: 82_000,
    cost_usd: 0.21,
  },
  notes: {
    plan_id: 'notes',
    phase: 'started',
    active: true,
    tasks_total: 1,
    tasks_done: 0,
    tasks_failed: 0,
    tasks_accepted_with_failures: 0,
    started_at_ms: 90_000,
    finished_at_ms: null,
    cost_usd: 0.04,
  },
};

describe('reload from a snapshot', () => {
  it('keeps plan times, cost and accepted counts', () => {
    const run = fromSnapshot(snapshot(PLANS), 100_000);

    expect(run.plans['hello']).toMatchObject({
      phase: 'completed',
      tasksDone: 3,
      tasksAccepted: 1,
      startedAtMs: 10_000,
      finishedAtMs: 82_000,
      costUsd: 0.21,
    });
    expect(run.plans['notes']).toMatchObject({
      phase: 'running',
      startedAtMs: 90_000,
      finishedAtMs: null,
      costUsd: 0.04,
    });
  });

  it('keeps plan times, cost and accepted counts where the rail and status line show them', () => {
    const nowMs = 100_000;
    const run = fromSnapshot(snapshot(PLANS), nowMs);
    const rows = buildPlanRows([disk('hello'), disk('notes')], run, { filter: '', nowMs });
    const row = (id: string) => rows.groups.flatMap((g) => g.rows).find((r) => r.id === id)!;

    // Actual time once finished, elapsed while running — not the ~4m estimate.
    expect(row('hello').time).toEqual({ kind: 'actual', ms: 72_000 });
    expect(row('notes').time).toEqual({ kind: 'elapsed', ms: 10_000 });
    // Green means verified: a plan with an accepted task stays amber.
    expect(row('hello').state).toBe('accepted');

    const hello = run.plans['hello']!;
    const cost = statusFields({
      hasRun: true,
      running: false,
      taskCount: 3,
      waveCount: 1,
      estimatedMinutes: null,
      parallel: null,
      tasksDone: hello.tasksDone,
      tasksTotal: hello.tasksTotal,
      elapsedMs: hello.finishedAtMs! - hello.startedAtMs!,
      etaMinutes: null,
      costUsd: hello.costUsd,
      busyAgents: 0,
      maxParallel: null,
    }).find((f) => f.key === 'cost');
    expect(cost?.text).toBe('$0.21');
  });

  it('keeps plan times, cost and accepted counts that the live events built', () => {
    const events: WireDashboardEvent[] = [
      { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 2 }] },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
      { type: 'task_started', plan_id: 'hello', task_id: 'T1', phase: 'implement' },
      { type: 'efficiency_event', plan_id: 'hello', task_id: 'T1', metric: 'cost_usd', value: 0.25 },
      { type: 'task_completed', plan_id: 'hello', task_id: 'T1', outcome: 'passed' },
      { type: 'task_started', plan_id: 'hello', task_id: 'T2', phase: 'implement' },
      { type: 'efficiency_event', plan_id: 'hello', task_id: 'T2', metric: 'cost_usd', value: 0.5 },
      { type: 'task_completed', plan_id: 'hello', task_id: 'T2', outcome: 'accepted_with_failures' },
      { type: 'plan_completed', plan_id: 'hello', success: true },
    ];
    // One event per second from t=1s.
    const live = events.reduce((run, event, i) => applyEvent(run, event, 1_000 * (i + 1)), initialRunState());

    // The server's materialization of the same events (dashboard_snapshot.rs).
    const reloaded = fromSnapshot(
      snapshot(
        {
          hello: {
            plan_id: 'hello',
            phase: 'completed',
            active: false,
            tasks_total: 2,
            tasks_done: 2,
            tasks_failed: 0,
            tasks_accepted_with_failures: 1,
            started_at_ms: 2_000,
            finished_at_ms: 9_000,
            cost_usd: 0.75,
          },
        },
        {
          plan_set: {
            plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 2 }],
            tasks_total: 2,
            loaded_at_ms: 1_000,
          },
          run_duration_ms: 8_500,
          run_outcome: 'succeeded',
        },
      ),
      20_000,
    );

    const fields = ['phase', 'tasksDone', 'tasksFailed', 'tasksAccepted', 'startedAtMs', 'finishedAtMs', 'costUsd'] as const;
    for (const field of fields) {
      expect(reloaded.plans['hello']?.[field], field).toBe(live.plans['hello']?.[field]);
    }
  });

  it('keeps the first end time when a plan completes twice, as the snapshot does', () => {
    const events: WireDashboardEvent[] = [
      { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 1 }] },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
      { type: 'plan_completed', plan_id: 'hello', success: true },
      { type: 'run_completed', outcome: 'succeeded', duration_ms: 2_000 },
      { type: 'plan_completed', plan_id: 'hello', success: true },
    ];
    const run = events.reduce((state, event, i) => applyEvent(state, event, 1_000 * (i + 1)), initialRunState());
    expect(run.plans['hello']).toMatchObject({ phase: 'completed', startedAtMs: 2_000, finishedAtMs: 3_000 });
  });

  it("prefers a plan's own times to what its one-plan set implies", () => {
    const run = fromSnapshot(
      snapshot(
        { hello: PLANS['hello']! },
        {
          plan_set: {
            plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 3 }],
            tasks_total: 3,
            loaded_at_ms: 9_000,
          },
          run_duration_ms: 80_000,
          run_outcome: 'succeeded',
        },
      ),
      100_000,
    );
    expect(run.plans['hello']?.startedAtMs).toBe(10_000);
    expect(run.plans['hello']?.finishedAtMs).toBe(82_000);
  });
});
