import { describe, it, expect } from 'vitest';
import { buildRoster } from './agentRoster';
import type { RunState, AgentRun } from './runState';

// ── Helpers ────────────────────────────────────────────────────────────────────

function makeRun(agents: Record<string, AgentRun>): RunState {
  return {
    planSet: null,
    plans: {},
    tasks: {},
    agents,
    transcripts: {},
    errors: [],
    run: { startedAtMs: null, durationMs: null, outcome: null },
    totals: { costUsd: 0, inputTokens: 0, outputTokens: 0 },
    usage: [],
  };
}

function agent(
  agentId: string,
  planId: string | null,
  taskId: string | null,
  active: boolean,
  spawnedAtMs: number | null = null,
  inputTokens = 0,
  outputTokens = 0,
): AgentRun {
  return {
    agentId,
    planId,
    taskId,
    role: 'impl',
    model: 'claude',
    active,
    spawnedAtMs,
    costUsd: 0,
    inputTokens,
    outputTokens,
  };
}

// ── Tests ──────────────────────────────────────────────────────────────────────

describe('buildRoster', () => {
  // 1. Two plans' agents ordered by plan position, then task id (numeric-aware),
  //    then agent id.
  it('orders agents by plan position in runningPlanIds, then task id (numeric), then agent id', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-b', 'task-2', true),
      a2: agent('a2', 'plan-a', 'task-10', true),
      a3: agent('a3', 'plan-a', 'task-2', true),
      a4: agent('a4', 'plan-b', 'task-1', true),
    });
    const { rows } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a', 'plan-b'],
      maxParallelByPlan: {},
    });
    // plan-a first: task-2 (a3) < task-10 (a2); plan-b next: task-1 (a4) < task-2 (a1)
    expect(rows.map((r) => r.agentId)).toEqual(['a3', 'a2', 'a4', 'a1']);
  });

  // 2. Finished agents are counted but not listed in rows.
  it('finished agents are counted but not listed in rows', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
      a2: agent('a2', 'plan-a', 'task-2', false),
      a3: agent('a3', 'plan-a', 'task-3', false),
    });
    const roster = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(roster.rows).toHaveLength(1);
    expect(roster.rows[0]!.agentId).toBe('a1');
    expect(roster.finished).toBe(2);
  });

  // 3. elapsedMs = nowMs − spawnedAtMs.
  it('computes elapsedMs as nowMs − spawnedAtMs', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true, 800),
    });
    const { rows } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(rows[0]!.elapsedMs).toBe(200);
  });

  // 4. elapsedMs is null when spawnedAtMs is unknown.
  it('elapsedMs is null when spawnedAtMs is unknown', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true, null),
    });
    const { rows } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(rows[0]!.elapsedMs).toBeNull();
  });

  // 5. tokens is inputTokens + outputTokens.
  it('tokens is inputTokens + outputTokens', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true, null, 300, 150),
    });
    const { rows } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(rows[0]!.tokens).toBe(450);
  });

  // 6. idle is summed across two plans with known max_parallel.
  it('idle sums open slots across two plans with known limits', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
      a2: agent('a2', 'plan-b', 'task-1', true),
      a3: agent('a3', 'plan-b', 'task-2', true),
    });
    // plan-a: max=3, active=1 → 2 slots; plan-b: max=4, active=2 → 2 slots; total=4
    const { idle } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a', 'plan-b'],
      maxParallelByPlan: { 'plan-a': 3, 'plan-b': 4 },
    });
    expect(idle).toBe(4);
  });

  // 7. A plan with more active agents than its limit contributes 0 (not negative).
  it('overcrowded plan contributes 0 to idle, not a negative number', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
      a2: agent('a2', 'plan-a', 'task-2', true),
      a3: agent('a3', 'plan-a', 'task-3', true),
    });
    // max=2, active=3 → max(0, 2-3) = 0
    const { idle } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: { 'plan-a': 2 },
    });
    expect(idle).toBe(0);
  });

  // 8. idle is null when no running plan has a known limit.
  it('idle is null when no running plan has a known limit', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
    });
    const { idle } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(idle).toBeNull();
  });

  // 9. Idle run: no agents, empty rows, zero finished, null idle (no limits known).
  it('returns empty rows and zero finished for an idle run with no agents', () => {
    const run = makeRun({});
    const roster = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(roster.rows).toHaveLength(0);
    expect(roster.finished).toBe(0);
    expect(roster.idle).toBeNull();
  });

  // 10. Agents belonging to non-running plans are excluded from both rows and
  //     finished.
  it('agents of non-running plans are excluded from rows and finished', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
      a2: agent('a2', 'plan-b', 'task-1', true),
      a3: agent('a3', 'plan-b', 'task-2', false),
    });
    // Only plan-a is in runningPlanIds
    const roster = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(roster.rows).toHaveLength(1);
    expect(roster.rows[0]!.agentId).toBe('a1');
    expect(roster.finished).toBe(0);
  });

  // 11. idle remains null when only some plans have unknown limits and none have
  //     known limits — partial knowledge scenario.
  it('idle is still null when only the unrecognised plan has an unknown limit', () => {
    const run = makeRun({
      a1: agent('a1', 'plan-a', 'task-1', true),
      a2: agent('a2', 'plan-b', 'task-1', true),
    });
    // plan-a limit unknown, plan-b limit unknown
    const { idle } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a', 'plan-b'],
      maxParallelByPlan: {},
    });
    expect(idle).toBeNull();
  });

  // 12. Ties on same plan + task are broken by agent id (numeric-aware).
  it('breaks ties within the same plan and task by agent id', () => {
    const run = makeRun({
      agent2: agent('agent2', 'plan-a', 'task-1', true),
      agent10: agent('agent10', 'plan-a', 'task-1', true),
      agent1: agent('agent1', 'plan-a', 'task-1', true),
    });
    const { rows } = buildRoster(run, {
      nowMs: 1000,
      runningPlanIds: ['plan-a'],
      maxParallelByPlan: {},
    });
    expect(rows.map((r) => r.agentId)).toEqual(['agent1', 'agent2', 'agent10']);
  });
});
