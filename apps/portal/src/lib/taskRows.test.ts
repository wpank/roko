import { describe, it, expect } from 'vitest';
import { buildTaskRows, focusTaskId } from './taskRows';
import type { TaskRowModel } from './taskRows';
import { initialRunState, taskKey } from './runState';
import type { RunState, TaskRun, PlanRun } from './runState';
import type { WirePlanTask } from '@/api/contracts';

// ── Test helpers ───────────────────────────────────────────────────────────────

const PLAN_ID = 'plan1';
const NOW_MS = 1_000_000;

/** Build a minimal WirePlanTask, filling required fields with safe defaults. */
function makeWireTask(overrides: Partial<WirePlanTask> & Pick<WirePlanTask, 'id'>): WirePlanTask {
  return {
    tier: 'impl',
    status: 'pending',
    depends_on: [],
    files: [],
    completed: false,
    verify_phases: [],
    ...overrides,
    id: overrides.id,
    title: overrides.title ?? `Task ${overrides.id}`,
  };
}

/** Build a minimal TaskRun for insertion into run.tasks. */
function makeLiveTask(
  id: string,
  overrides: Partial<Omit<TaskRun, 'planId' | 'taskId'>>,
): TaskRun {
  return {
    planId: PLAN_ID,
    taskId: id,
    title: id,
    status: 'active',
    phase: 'impl',
    attempts: 1,
    startedAtMs: null,
    finishedAtMs: null,
    agentId: null,
    role: null,
    model: null,
    costUsd: 0,
    inputTokens: 0,
    outputTokens: 0,
    checks: [],
    ...overrides,
  };
}

/** Build a minimal PlanRun for insertion into run.plans. */
function makePlanRun(overrides: Partial<PlanRun> = {}): PlanRun {
  return {
    planId: PLAN_ID,
    title: null,
    phase: 'running',
    tasksTotal: 1,
    tasksDone: 0,
    tasksFailed: 0,
    tasksAccepted: 0,
    startedAtMs: null,
    finishedAtMs: null,
    etaMinutes: null,
    costUsd: 0,
    ...overrides,
  };
}

/** Overlay partial RunState on top of initialRunState(). */
function makeRunState(overrides: Partial<RunState> = {}): RunState {
  return { ...initialRunState(), ...overrides };
}

// ── buildTaskRows ──────────────────────────────────────────────────────────────

describe('buildTaskRows – status sources', () => {
  it('uses live task status when a run record exists', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      tasks: { [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', { status: 'active' }) },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.status).toBe('active');
    expect(rows[0]!.state).toBe('active');
  });

  it('uses passed when wire task is completed and no live record', () => {
    const tasks = [makeWireTask({ id: 'T01', completed: true })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.status).toBe('passed');
    expect(rows[0]!.state).toBe('done');
  });

  it('uses pending for an untouched task with no live record', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.status).toBe('pending');
    expect(rows[0]!.state).toBe('pending');
  });

  it('is skipped when the plan has failed and the task never started', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      plans: { [PLAN_ID]: makePlanRun({ phase: 'failed' }) },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.status).toBe('skipped');
    expect(rows[0]!.state).toBe('skipped');
  });

  it('is skipped when the plan has been cancelled and the task never started', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      plans: { [PLAN_ID]: makePlanRun({ phase: 'cancelled' }) },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.status).toBe('skipped');
  });

  it('does NOT mark tasks as skipped when plan is still running', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      plans: { [PLAN_ID]: makePlanRun({ phase: 'running' }) },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    // No live task and plan is running → pending, not skipped
    expect(rows[0]!.status).toBe('pending');
  });
});

describe('buildTaskRows – wave ordering', () => {
  it('returns rows in wave order with input order preserved within each wave', () => {
    // T01 is independent (wave 0)
    // T02 and T03 both depend on T01 (wave 1, input order: T02 then T03)
    // T04 depends on T02 (wave 2)
    const tasks = [
      makeWireTask({ id: 'T01' }),
      makeWireTask({ id: 'T02', depends_on: ['T01'] }),
      makeWireTask({ id: 'T03', depends_on: ['T01'] }),
      makeWireTask({ id: 'T04', depends_on: ['T02'] }),
    ];
    const { rows, waves } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows.map((r) => r.id)).toEqual(['T01', 'T02', 'T03', 'T04']);
    expect(rows.map((r) => r.wave)).toEqual([0, 1, 1, 2]);
    expect(waves.waves).toHaveLength(3);
  });
});

describe('buildTaskRows – waitingOn', () => {
  it('only lists deps that are not passed / accepted_with_failures / skipped', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }),              // pending → blocking
      makeWireTask({ id: 'T02', completed: true }), // passed → not blocking
      makeWireTask({ id: 'T03', depends_on: ['T01', 'T02'] }),
    ];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    const t03 = rows.find((r) => r.id === 'T03')!;
    expect(t03.waitingOn).toEqual(['T01']);
  });

  it('treats active and failed deps as blocking', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }), // will be set active
      makeWireTask({ id: 'T02' }), // will be set failed
      makeWireTask({ id: 'T03', depends_on: ['T01', 'T02'] }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', { status: 'active' }),
        [taskKey(PLAN_ID, 'T02')]: makeLiveTask('T02', { status: 'failed' }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    const t03 = rows.find((r) => r.id === 'T03')!;
    expect(t03.waitingOn).toContain('T01');
    expect(t03.waitingOn).toContain('T02');
  });
});

describe('buildTaskRows – time', () => {
  it('elapsed when task is active with a startedAtMs', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', {
          status: 'active',
          startedAtMs: NOW_MS - 4000,
        }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.time).toEqual({ kind: 'elapsed', ms: 4000 });
  });

  it('actual when task is finished with both startedAtMs and finishedAtMs', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', {
          status: 'passed',
          startedAtMs: NOW_MS - 8000,
          finishedAtMs: NOW_MS - 3000,
        }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.time).toEqual({ kind: 'actual', ms: 5000 });
  });

  it('estimate when no live record but estimated_minutes is set', () => {
    const tasks = [makeWireTask({ id: 'T01', estimated_minutes: 3 })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.time).toEqual({ kind: 'estimate', ms: 180_000 });
  });

  it('none when there is no timing information', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.time).toEqual({ kind: 'none', ms: null });
  });
});

describe('buildTaskRows – misc fields', () => {
  it('costUsd is live value when live record exists, else null', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }),
      makeWireTask({ id: 'T02' }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', { costUsd: 0.05 }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows.find((r) => r.id === 'T01')!.costUsd).toBe(0.05);
    expect(rows.find((r) => r.id === 'T02')!.costUsd).toBeNull();
  });

  it('verify: uses rich verify array when the server sends it', () => {
    const tasks = [
      makeWireTask({
        id: 'T01',
        verify: [{ phase: 'compile', command: 'cargo build' }],
        verify_phases: ['compile'],
      }),
    ];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.verify).toEqual([{ phase: 'compile', command: 'cargo build' }]);
  });

  it('verify: expands verify_phases with empty commands when verify is absent', () => {
    const tasks = [
      makeWireTask({ id: 'T01', verify_phases: ['compile', 'test'] }),
    ];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.verify).toEqual([
      { phase: 'compile', command: '' },
      { phase: 'test', command: '' },
    ]);
  });

  it('role and model fall back to wire fields when no live record', () => {
    const tasks = [
      makeWireTask({ id: 'T01', role: 'implementer', model_hint: 'claude-opus' }),
    ];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(rows[0]!.role).toBe('implementer');
    expect(rows[0]!.model).toBe('claude-opus');
  });

  it('role and model use live values over wire fields', () => {
    const tasks = [
      makeWireTask({ id: 'T01', role: 'researcher', model_hint: 'gpt-4' }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', {
          role: 'implementer',
          model: 'claude-3-5-sonnet',
        }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(rows[0]!.role).toBe('implementer');
    expect(rows[0]!.model).toBe('claude-3-5-sonnet');
  });
});

// ── focusTaskId ────────────────────────────────────────────────────────────────

describe('focusTaskId', () => {
  it('returns selected when it names an existing row', () => {
    const tasks = [makeWireTask({ id: 'T01' }), makeWireTask({ id: 'T02' })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(focusTaskId(rows, 'T02')).toBe('T02');
  });

  it('ignores selected when it does not match any row', () => {
    const tasks = [makeWireTask({ id: 'T01' })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    // 'UNKNOWN' is not a row, should fall through — all pending → null
    expect(focusTaskId(rows, 'UNKNOWN')).toBeNull();
  });

  it('returns first active row when selected is null', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }),
      makeWireTask({ id: 'T02' }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T02')]: makeLiveTask('T02', { status: 'active' }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(focusTaskId(rows, null)).toBe('T02');
  });

  it('returns first failed row when there is no active row', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }),
      makeWireTask({ id: 'T02' }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', { status: 'failed' }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(focusTaskId(rows, null)).toBe('T01');
  });

  it('returns last finished row when there is no active or failed row', () => {
    const tasks = [
      makeWireTask({ id: 'T01', completed: true }),
      makeWireTask({ id: 'T02', completed: true }),
    ];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    // Both are passed; last one in row order is T02
    expect(focusTaskId(rows, null)).toBe('T02');
  });

  it('returns null when all rows are pending', () => {
    const tasks = [makeWireTask({ id: 'T01' }), makeWireTask({ id: 'T02' })];
    const { rows } = buildTaskRows(tasks, initialRunState(), PLAN_ID, NOW_MS);
    expect(focusTaskId(rows, null)).toBeNull();
  });

  it('prefers active over failed when both exist', () => {
    const tasks = [
      makeWireTask({ id: 'T01' }),
      makeWireTask({ id: 'T02' }),
    ];
    const run = makeRunState({
      tasks: {
        [taskKey(PLAN_ID, 'T01')]: makeLiveTask('T01', { status: 'failed' }),
        [taskKey(PLAN_ID, 'T02')]: makeLiveTask('T02', { status: 'active' }),
      },
    });
    const { rows } = buildTaskRows(tasks, run, PLAN_ID, NOW_MS);
    expect(focusTaskId(rows, null)).toBe('T02');
  });
});
