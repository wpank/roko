/**
 * Acceptance: plan_set_loaded scheduling facts (wave, depends_on,
 * conflicts_with) are kept per member in RunState.planSet.members; a reload
 * keeps plan titles and what timing the snapshot allows; an agent's clock
 * starts when it spawns.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardSnapshot, WirePlanSetEntry } from '@/api/contracts';
import { applyEvent, fromSnapshot, initialRunState } from '@/lib/runState';

const ENTRIES: WirePlanSetEntry[] = [
  { plan_id: '01-a', title: 'Part A', tasks_total: 2, wave: 0, depends_on: [], conflicts_with: ['02-b'] },
  { plan_id: '02-b', title: 'Part B', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: ['01-a'] },
  { plan_id: '03-c', title: 'Part C', tasks_total: 3, wave: 1, depends_on: ['01-a'], conflicts_with: [] },
];

function snapshot(
  planSet: WireDashboardSnapshot['plan_set'],
  extra: Partial<WireDashboardSnapshot> = {},
): WireDashboardSnapshot {
  return {
    plans: {
      '01-a': { plan_id: '01-a', phase: 'started', active: true, tasks_total: 2, tasks_done: 0, tasks_failed: 0 },
      '02-b': { plan_id: '02-b', phase: 'pending', active: false, tasks_total: 1, tasks_done: 0, tasks_failed: 0 },
    },
    plan_set: planSet,
    tasks: {},
    agents: {},
    gates: [],
    errors: [],
    stats: {},
    ...extra,
  } as unknown as WireDashboardSnapshot;
}

describe('plan set members', () => {
  it('keeps position, wave, depends_on and conflicts_with per member', () => {
    const run = applyEvent(initialRunState(), { type: 'plan_set_loaded', plans: ENTRIES }, 1_000);
    expect(run.planSet?.members).toEqual({
      '01-a': { position: 0, wave: 0, dependsOn: [], conflictsWith: ['02-b'] },
      '02-b': { position: 1, wave: 0, dependsOn: [], conflictsWith: ['01-a'] },
      '03-c': { position: 2, wave: 1, dependsOn: ['01-a'], conflictsWith: [] },
    });
  });

  it('defaults the facts an older server omits', () => {
    const run = applyEvent(
      initialRunState(),
      { type: 'plan_set_loaded', plans: [{ plan_id: 'solo', title: 'Solo', tasks_total: 1 }] },
      1_000,
    );
    expect(run.planSet?.members?.['solo']).toEqual({
      position: 0,
      wave: 0,
      dependsOn: [],
      conflictsWith: [],
    });
  });

  it('keeps plan ids in execution order alongside the members', () => {
    const run = applyEvent(initialRunState(), { type: 'plan_set_loaded', plans: ENTRIES }, 1_000);
    expect(run.planSet?.planIds).toEqual(['01-a', '02-b', '03-c']);
    expect(run.planSet?.tasksTotal).toBe(6);
  });

  it('replaces the members when a new set loads', () => {
    let run = applyEvent(initialRunState(), { type: 'plan_set_loaded', plans: ENTRIES }, 1_000);
    run = applyEvent(
      run,
      { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello', tasks_total: 2 }] },
      2_000,
    );
    expect(Object.keys(run.planSet?.members ?? {})).toEqual(['hello']);
  });

  it('clears the previous outcome when a new set loads', () => {
    let run = applyEvent(
      initialRunState(),
      { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', tasks_total: 1 }] },
      1_000,
    );
    run = applyEvent(run, { type: 'run_completed', outcome: 'succeeded', duration_ms: 500 }, 1_500);
    expect(run.run.outcome).toBe('succeeded');
    run = applyEvent(run, { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', tasks_total: 1 }] }, 2_000);
    expect(run.run.outcome).toBeNull();
  });

  it('reads the members from a snapshot plan_set', () => {
    const run = fromSnapshot(
      snapshot({ plans: ENTRIES, tasks_total: 6, loaded_at_ms: 500 }),
      1_000,
    );
    expect(run.planSet?.members?.['03-c']).toEqual({
      position: 2,
      wave: 1,
      dependsOn: ['01-a'],
      conflictsWith: [],
    });
    expect(run.planSet?.loadedAtMs).toBe(500);
  });

  it('defaults members from a snapshot written by an older server', () => {
    const run = fromSnapshot(
      snapshot({ plans: [{ plan_id: '01-a' }, { plan_id: '02-b' }], tasks_total: 3, loaded_at_ms: 500 }),
      1_000,
    );
    expect(run.planSet?.members?.['02-b']).toEqual({
      position: 1,
      wave: 0,
      dependsOn: [],
      conflictsWith: [],
    });
  });

  it('has no members without a plan set', () => {
    expect(initialRunState().planSet).toBeNull();
    expect(fromSnapshot(snapshot(null), 1_000).planSet).toBeNull();
  });
});

describe('reload from a snapshot', () => {
  it('names plans from the plan_set entries', () => {
    const run = fromSnapshot(snapshot({ plans: ENTRIES, tasks_total: 6, loaded_at_ms: 500 }), 1_000);
    expect(run.plans['01-a']?.title).toBe('Part A');
    expect(run.plans['02-b']?.title).toBe('Part B');
  });

  it("starts the set's first member, while it runs, at the set's load time", () => {
    const run = fromSnapshot(snapshot({ plans: ENTRIES, tasks_total: 6, loaded_at_ms: 500 }), 1_000);
    expect(run.plans['01-a']?.phase).toBe('running');
    expect(run.plans['01-a']?.startedAtMs).toBe(500);
    expect(run.plans['02-b']?.startedAtMs).toBeNull();
  });

  it('times a finished one-plan set from the run duration', () => {
    const run = fromSnapshot(
      snapshot(
        { plans: [{ plan_id: '01-a', title: 'Part A', tasks_total: 2 }], tasks_total: 2, loaded_at_ms: 500 },
        {
          plans: {
            '01-a': { plan_id: '01-a', phase: 'completed', active: false, tasks_total: 2, tasks_done: 2, tasks_failed: 0 },
          },
          run_duration_ms: 96_000,
          run_outcome: 'succeeded',
        },
      ),
      200_000,
    );
    expect(run.plans['01-a']?.startedAtMs).toBe(500);
    expect(run.plans['01-a']?.finishedAtMs).toBe(96_500);
  });

  it('leaves the timing unknown when the snapshot cannot tell', () => {
    const run = fromSnapshot(
      snapshot(
        { plans: ENTRIES, tasks_total: 6, loaded_at_ms: 500 },
        {
          plans: {
            '01-a': { plan_id: '01-a', phase: 'completed', active: false, tasks_total: 2, tasks_done: 2, tasks_failed: 0 },
            '02-b': { plan_id: '02-b', phase: 'completed', active: false, tasks_total: 1, tasks_done: 1, tasks_failed: 0 },
          },
          run_duration_ms: 96_000,
        },
      ),
      200_000,
    );
    expect(run.plans['01-a']?.startedAtMs).toBeNull();
    expect(run.plans['01-a']?.finishedAtMs).toBeNull();
  });
});

describe('agent clock', () => {
  it('starts when the agent spawns', () => {
    const run = applyEvent(
      initialRunState(),
      { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p', task_id: 't', role: 'implementer', model: 'm' },
      4_000,
    );
    expect(run.agents['a1']?.spawnedAtMs).toBe(4_000);
  });

  it('keeps the spawn time through heartbeats', () => {
    let run = applyEvent(
      initialRunState(),
      { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p', task_id: 't', role: 'implementer' },
      4_000,
    );
    run = applyEvent(run, { type: 'agent_heartbeat', agent_id: 'a1', plan_id: 'p', task_id: 't', elapsed_ms: 5_200 }, 9_000);
    expect(run.agents['a1']?.spawnedAtMs).toBe(4_000);
  });

  it('restarts when a finished agent id spawns again', () => {
    let run = applyEvent(
      initialRunState(),
      { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p', task_id: 't', role: 'implementer' },
      4_000,
    );
    run = applyEvent(run, { type: 'agent_completed', agent_id: 'a1' }, 6_000);
    run = applyEvent(
      run,
      { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p', task_id: 't', role: 'implementer' },
      7_000,
    );
    expect(run.agents['a1']?.spawnedAtMs).toBe(7_000);
  });
});
