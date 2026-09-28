import { describe, it, expect } from 'vitest';
import { buildBurn } from './burn';
import type { RunState, AgentRun, PlanRun } from './runState';
import { initialRunState } from './runState';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeRun(partial: Partial<RunState>): RunState {
  return { ...initialRunState(), ...partial };
}

function makeAgent(
  id: string,
  planId: string | null,
  role: string,
  input: number,
  output: number,
): AgentRun {
  return {
    agentId: id,
    planId,
    taskId: null,
    role,
    model: 'test',
    active: true,
    spawnedAtMs: null,
    costUsd: 0,
    inputTokens: input,
    outputTokens: output,
  };
}

function makePlan(id: string, phase: PlanRun['phase']): PlanRun {
  return {
    planId: id,
    title: null,
    phase,
    tasksTotal: 0,
    tasksDone: 0,
    tasksFailed: 0,
    tasksAccepted: 0,
    startedAtMs: null,
    finishedAtMs: null,
    etaMinutes: null,
    costUsd: 0,
  };
}

const NOW = 100_000;

// ── Tests ──────────────────────────────────────────────────────────────────────

describe('buildBurn', () => {
  it('returns zero tokens, empty byRole, and null rate for empty state', () => {
    const result = buildBurn(makeRun({}), { nowMs: NOW });
    expect(result.tokens).toBe(0);
    expect(result.byRole).toEqual([]);
    expect(result.tokensPerMin).toBeNull();
  });

  it('set scope: includes only agents whose planId is in planSet.planIds', () => {
    const run = makeRun({
      planSet: { planIds: ['plan-A'], tasksTotal: 2, loadedAtMs: 0 },
      plans: {
        'plan-A': makePlan('plan-A', 'running'),
        'plan-B': makePlan('plan-B', 'running'),
      },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 100, 50),
        a2: makeAgent('a2', 'plan-B', 'impl', 200, 100), // excluded
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(150); // a1 only: 100+50
    expect(result.byRole).toHaveLength(1);
    expect(result.byRole[0]).toMatchObject({ role: 'impl', tokens: 150 });
  });

  it('running-plan scope when no planSet: only agents of running plans', () => {
    const run = makeRun({
      planSet: null,
      plans: {
        'plan-A': makePlan('plan-A', 'running'),
        'plan-B': makePlan('plan-B', 'completed'),
      },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 100, 50),
        a2: makeAgent('a2', 'plan-B', 'impl', 200, 100), // excluded: plan-B completed
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(150); // a1 only
    expect(result.byRole).toHaveLength(1);
  });

  it('set scope takes priority over running-plan scope', () => {
    // plan-A is completed but in planSet; plan-B is running but NOT in planSet
    const run = makeRun({
      planSet: { planIds: ['plan-A'], tasksTotal: 1, loadedAtMs: 0 },
      plans: {
        'plan-A': makePlan('plan-A', 'completed'),
        'plan-B': makePlan('plan-B', 'running'),
      },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 500, 0),
        a2: makeAgent('a2', 'plan-B', 'impl', 100, 0),
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(500); // a1 only (plan-A in planSet)
  });

  it('groups tokens by role, sorted largest first', () => {
    const run = makeRun({
      planSet: { planIds: ['plan-A'], tasksTotal: 3, loadedAtMs: 0 },
      plans: { 'plan-A': makePlan('plan-A', 'running') },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 300, 0),
        a2: makeAgent('a2', 'plan-A', 'reviewer', 50, 0),
        a3: makeAgent('a3', 'plan-A', 'impl', 100, 0), // same role as a1
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(450);
    expect(result.byRole).toHaveLength(2);
    expect(result.byRole[0]).toMatchObject({ role: 'impl', tokens: 400 }); // 300+100
    expect(result.byRole[1]).toMatchObject({ role: 'reviewer', tokens: 50 });
  });

  it('byRole shares sum to 1 when total > 0', () => {
    const run = makeRun({
      planSet: { planIds: ['plan-A'], tasksTotal: 3, loadedAtMs: 0 },
      plans: { 'plan-A': makePlan('plan-A', 'running') },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 300, 0),
        a2: makeAgent('a2', 'plan-A', 'reviewer', 100, 0),
        a3: makeAgent('a3', 'plan-A', 'scribe', 100, 0),
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    const sum = result.byRole.reduce((s, r) => s + r.share, 0);
    expect(sum).toBeCloseTo(1, 10);
  });

  it('zero totals: byRole is empty, tokens is 0', () => {
    // Agents with zero tokens are omitted from byRole
    const run = makeRun({
      planSet: { planIds: ['plan-A'], tasksTotal: 1, loadedAtMs: 0 },
      plans: { 'plan-A': makePlan('plan-A', 'running') },
      agents: {
        a1: makeAgent('a1', 'plan-A', 'impl', 0, 0),
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(0);
    expect(result.byRole).toHaveLength(0);
    // Share would be 0 — but byRole is empty so nothing to assert on shares
  });

  it('tokensPerMin: sums samples within window and scales to one minute', () => {
    const windowMs = 60_000;
    const run = makeRun({
      usage: [
        { atMs: NOW - 10_000, tokens: 100 },
        { atMs: NOW - 20_000, tokens: 200 },
        { atMs: NOW - windowMs - 1, tokens: 999 }, // outside window — excluded
      ],
    });
    const result = buildBurn(run, { nowMs: NOW, windowMs });
    // 300 tokens over 60 000 ms → 300/60000 * 60000 = 300 tok/min
    expect(result.tokensPerMin).toBeCloseTo(300, 5);
  });

  it('tokensPerMin is null when no samples fall inside the window', () => {
    const run = makeRun({
      usage: [
        { atMs: NOW - 200_000, tokens: 100 }, // 200 s ago, outside 60 s window
      ],
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokensPerMin).toBeNull();
  });

  it('tokensPerMin is null when the usage ring is empty (after reload)', () => {
    const run = makeRun({ usage: [] });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokensPerMin).toBeNull();
  });

  it('tokensPerMin scales correctly with a custom windowMs', () => {
    const windowMs = 30_000;
    const run = makeRun({
      usage: [{ atMs: NOW - 10_000, tokens: 600 }],
    });
    const result = buildBurn(run, { nowMs: NOW, windowMs });
    // 600 tokens over 30 000 ms → 600/30000 * 60000 = 1200 tok/min
    expect(result.tokensPerMin).toBeCloseTo(1200, 5);
  });

  it('agents with null planId are always excluded', () => {
    const run = makeRun({
      planSet: null,
      plans: { 'plan-A': makePlan('plan-A', 'running') },
      agents: {
        a1: makeAgent('a1', null, 'impl', 500, 0), // planId null
        a2: makeAgent('a2', 'plan-A', 'impl', 100, 0),
      },
    });
    const result = buildBurn(run, { nowMs: NOW });
    expect(result.tokens).toBe(100); // a1 excluded (null planId)
  });
});
