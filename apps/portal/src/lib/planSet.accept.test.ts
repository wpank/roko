/**
 * Acceptance: when a plan-set run counts as active, and why a queued member
 * waits. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WirePlanSetEntry } from '@/api/contracts';
import { applyEvent, initialRunState } from '@/lib/runState';
import type { RunState } from '@/lib/runState';
import { planSetActive, queuePosition, waitReason } from '@/lib/planSet';

function fold(events: WireDashboardEvent[]): RunState {
  return events.reduce((run, e, i) => applyEvent(run, e, 1_000 + i), initialRunState());
}

const HELLO: WirePlanSetEntry[] = [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }];

const SET: WirePlanSetEntry[] = [
  { plan_id: '01-a', tasks_total: 2, wave: 0, depends_on: [], conflicts_with: ['02-b'] },
  { plan_id: '02-b', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: ['01-a'] },
  { plan_id: '03-c', tasks_total: 1, wave: 1, depends_on: ['01-a', '02-b'], conflicts_with: [] },
  { plan_id: '04-d', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] },
];

describe('planSetActive', () => {
  it('is false without a plan set', () => {
    expect(planSetActive(initialRunState())).toBe(false);
  });

  it('is true while a member is pending', () => {
    expect(planSetActive(fold([{ type: 'plan_set_loaded', plans: HELLO }]))).toBe(true);
  });

  it('is true while a member is running', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: HELLO },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
    ]);
    expect(planSetActive(run)).toBe(true);
  });

  it('is false once every member finished, even though the server keeps plan_set', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: HELLO },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
      { type: 'plan_completed', plan_id: 'hello', success: true },
    ]);
    expect(run.planSet).not.toBeNull();
    expect(run.run.outcome).toBeNull();
    expect(planSetActive(run)).toBe(false);
  });

  it('is false when members failed or were cancelled', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET.slice(0, 2) },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '01-a', success: false },
      { type: 'plan_completed', plan_id: '02-b', success: false },
    ]);
    expect(planSetActive(run)).toBe(false);
  });

  it('is false once run_completed reports an outcome, even with a member still pending', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'run_completed', outcome: 'cancelled', duration_ms: 900 },
    ]);
    expect(run.plans['04-d']?.phase).toBe('pending');
    expect(planSetActive(run)).toBe(false);
  });

  it('ignores a member with no live record', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: HELLO },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
      { type: 'plan_completed', plan_id: 'hello', success: true },
    ]);
    const withGhost: RunState = {
      ...run,
      planSet: { ...run.planSet!, planIds: [...run.planSet!.planIds, 'ghost'] },
    };
    expect(planSetActive(withGhost)).toBe(false);
  });

  it('is true again when the next set loads', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: HELLO },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
      { type: 'plan_completed', plan_id: 'hello', success: true },
      { type: 'run_completed', outcome: 'succeeded', duration_ms: 500 },
      { type: 'plan_set_loaded', plans: HELLO },
    ]);
    expect(planSetActive(run)).toBe(true);
  });
});

describe('queuePosition', () => {
  it('is the 1-based execution position of a pending member of an active set', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(queuePosition(run, '02-b')).toBe(2);
    expect(queuePosition(run, '04-d')).toBe(4);
  });

  it('is null for running members, non-members and inactive sets', () => {
    let run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(queuePosition(run, '01-a')).toBeNull();
    expect(queuePosition(run, 'elsewhere')).toBeNull();
    run = applyEvent(run, { type: 'run_completed', outcome: 'failed', duration_ms: 10 }, 9_000);
    expect(queuePosition(run, '02-b')).toBeNull();
  });
});

describe('waitReason', () => {
  it('names the unfinished prerequisites, in depends_on order', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(waitReason(run, '03-c')).toBe('after 01-a, 02-b');
  });

  it('drops prerequisites that already completed', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '01-a', success: true },
      { type: 'plan_started', plan_id: '02-b', tasks_total: 1 },
    ]);
    expect(waitReason(run, '03-c')).toBe('after 02-b');
  });

  it('reports a failed prerequisite as blocking', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '01-a', success: false },
    ]);
    expect(waitReason(run, '03-c')).toBe('blocked: 01-a failed');
  });

  it('names a running plan it may not run beside', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(waitReason(run, '02-b')).toBe('after 01-a (shared files)');
  });

  it('otherwise waits for a free slot', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(waitReason(run, '04-d')).toBe('waiting for a free slot');
  });

  it('is null for running members, non-members and inactive sets', () => {
    let run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    expect(waitReason(run, '01-a')).toBeNull();
    expect(waitReason(run, 'elsewhere')).toBeNull();
    run = applyEvent(run, { type: 'run_completed', outcome: 'cancelled', duration_ms: 10 }, 9_000);
    expect(waitReason(run, '04-d')).toBeNull();
  });

  it('waits for a free slot when the server sent no scheduling facts', () => {
    const run = fold([
      {
        type: 'plan_set_loaded',
        plans: [
          { plan_id: 'one', tasks_total: 1 },
          { plan_id: 'two', tasks_total: 1 },
        ],
      },
      { type: 'plan_started', plan_id: 'one', tasks_total: 1 },
    ]);
    expect(waitReason(run, 'two')).toBe('waiting for a free slot');
  });
});
