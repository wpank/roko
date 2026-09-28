import { describe, it, expect } from 'vitest';
import { describeEmpty } from './emptyState';
import type { EmptyStateInput } from './emptyState';

// ── Helpers ────────────────────────────────────────────────────────────────────

const BASE_PLAN: NonNullable<EmptyStateInput['plan']> = {
  id: 'plan-01',
  phase: 'pending',
  tasksTotal: 5,
  tasksDone: 0,
  tasksActive: 0,
  tasksAccepted: 0,
};

function input(overrides: Partial<EmptyStateInput> = {}): EmptyStateInput {
  return {
    workspace: 'my-project',
    connection: 'connected',
    planCount: 1,
    plan: BASE_PLAN,
    ...overrides,
  };
}

function plan(overrides: Partial<NonNullable<EmptyStateInput['plan']>> = {}): NonNullable<EmptyStateInput['plan']> {
  return { ...BASE_PLAN, ...overrides };
}

// ── Case 1: disconnected ───────────────────────────────────────────────────────

describe('case: disconnected', () => {
  it('returns reconnecting message when disconnected', () => {
    const result = describeEmpty(input({ connection: 'disconnected' }));
    expect(result).toBe('Lost the server; reconnecting.');
  });
});

// ── Case 2: error ──────────────────────────────────────────────────────────────

describe('case: error', () => {
  it('returns reconnecting message when connection errored', () => {
    const result = describeEmpty(input({ connection: 'error' }));
    expect(result).toBe('Lost the server; reconnecting.');
  });
});

// ── Case 3: no plans ──────────────────────────────────────────────────────────

describe('case: no plans', () => {
  it('prompts to describe work when there are no plans', () => {
    const result = describeEmpty(
      input({ planCount: 0, plan: undefined, workspace: 'roko-core' }),
    );
    expect(result).toBe('No plans in roko-core yet. Describe what you want to build.');
  });
});

// ── Case 4: plans but none selected ───────────────────────────────────────────

describe('case: plans but none selected', () => {
  it('prompts to select a plan when plan is undefined', () => {
    const result = describeEmpty(input({ planCount: 3, plan: undefined }));
    expect(result).toBe('Select a plan, or press n to describe a new one.');
  });
});

// ── Case 5: never run ─────────────────────────────────────────────────────────

describe('case: never run', () => {
  it('shows task and wave count when waves is set', () => {
    const result = describeEmpty(
      input({ plan: plan({ phase: 'never_run', tasksTotal: 3, waves: 3 }) }),
    );
    expect(result).toBe('Ready — 3 tasks in 3 waves.');
  });

  it('shows just task count when waves is not set', () => {
    const result = describeEmpty(
      input({ plan: plan({ phase: 'never_run', tasksTotal: 4 }) }),
    );
    expect(result).toBe('Ready — 4 tasks.');
  });

  it('uses singular task word', () => {
    const result = describeEmpty(
      input({ plan: plan({ phase: 'never_run', tasksTotal: 1 }) }),
    );
    expect(result).toContain('1 task');
    expect(result).not.toContain('tasks');
  });
});

// ── Case 6: running — nothing active, tasks queued ───────────────────────────

describe('case: running with tasks queued but no agent', () => {
  it('shows queued task count when nothing is active', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'running',
          tasksTotal: 3,
          tasksDone: 0,
          tasksActive: 0,
        }),
      }),
    );
    expect(result).toBe('Scheduler has 3 queued tasks but no live agent yet.');
  });

  it('uses singular when exactly one task queued', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'running',
          tasksTotal: 2,
          tasksDone: 1,
          tasksActive: 0,
        }),
      }),
    );
    expect(result).toContain('1 queued task ');
  });
});

// ── Case 7: running — every task dispatched ───────────────────────────────────

describe('case: running with every task dispatched', () => {
  it('reports all tasks dispatched when active + done = total', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'running',
          tasksTotal: 4,
          tasksDone: 2,
          tasksActive: 2,
        }),
      }),
    );
    expect(result).toBe('Every task is dispatched; checks are running.');
  });
});

// ── Case 8: running otherwise ─────────────────────────────────────────────────

describe('case: running otherwise', () => {
  it('shows active and done counts in the generic running state', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'running',
          tasksTotal: 6,
          tasksDone: 2,
          tasksActive: 1,
        }),
      }),
    );
    expect(result).toContain('Running');
    expect(result).toContain('1 active');
    expect(result).toContain('2 of 6');
  });
});

// ── Case 9: completed (clean) ─────────────────────────────────────────────────

describe('case: completed cleanly', () => {
  it('shows duration and verification count', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'completed',
          tasksTotal: 3,
          tasksDone: 3,
          tasksActive: 0,
          tasksAccepted: 0,
          durationMs: 252_000, // 4m12s
        }),
      }),
    );
    expect(result).toBe('Finished in 4m12s — 3 of 3 verified.');
  });

  it('omits duration when durationMs is null', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'completed',
          tasksTotal: 2,
          tasksDone: 2,
          tasksActive: 0,
          tasksAccepted: 0,
          durationMs: null,
        }),
      }),
    );
    expect(result).toBe('Finished — 2 of 2 verified.');
  });
});

// ── Case 10: completed with accepted tasks ────────────────────────────────────

describe('case: completed with accepted tasks', () => {
  it('reports accepted task count', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'completed',
          tasksTotal: 3,
          tasksDone: 3,
          tasksActive: 0,
          tasksAccepted: 1,
        }),
      }),
    );
    expect(result).toBe('Finished; 1 task was accepted despite failing checks.');
  });

  it('uses plural form for multiple accepted tasks', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'completed',
          tasksTotal: 5,
          tasksDone: 5,
          tasksActive: 0,
          tasksAccepted: 2,
        }),
      }),
    );
    expect(result).toContain('2 tasks were');
  });
});

// ── Case 11: failed ───────────────────────────────────────────────────────────

describe('case: failed', () => {
  it('shows failed task and check with retry hint', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'failed',
          failedTaskId: 'T03',
          failedCheck: 'test',
        }),
      }),
    );
    expect(result).toBe('Stopped at T03 — test failed. Retry resumes from T03.');
  });

  it('omits check name when failedCheck is not set', () => {
    const result = describeEmpty(
      input({
        plan: plan({
          phase: 'failed',
          failedTaskId: 'T07',
        }),
      }),
    );
    expect(result).toContain('T07');
    expect(result).not.toContain('—');
  });

  it('falls back when no failedTaskId', () => {
    const result = describeEmpty(
      input({ plan: plan({ phase: 'failed' }) }),
    );
    expect(result).toContain('failed');
  });
});

// ── Case 12: cancelled ────────────────────────────────────────────────────────

describe('case: cancelled', () => {
  it('reports cancellation', () => {
    const result = describeEmpty(
      input({ plan: plan({ phase: 'cancelled' }) }),
    );
    expect(result).toContain('cancelled');
  });
});

// ── Invariant: no case returns empty string or "no data" ──────────────────────

describe('invariant: no empty string or "no data"', () => {
  const allInputs: EmptyStateInput[] = [
    // connection states
    input({ connection: 'disconnected' }),
    input({ connection: 'error' }),
    input({ connection: 'connecting' }),
    input({ connection: 'connected' }),
    // no plans
    input({ planCount: 0, plan: undefined }),
    // no plan selected
    input({ planCount: 2, plan: undefined }),
    // never_run
    input({ plan: plan({ phase: 'never_run', tasksTotal: 5, waves: 2 }) }),
    input({ plan: plan({ phase: 'never_run', tasksTotal: 1 }) }),
    // running substates
    input({ plan: plan({ phase: 'running', tasksTotal: 4, tasksDone: 0, tasksActive: 0 }) }),
    input({ plan: plan({ phase: 'running', tasksTotal: 4, tasksDone: 2, tasksActive: 2 }) }),
    input({ plan: plan({ phase: 'running', tasksTotal: 6, tasksDone: 2, tasksActive: 1 }) }),
    // completed
    input({ plan: plan({ phase: 'completed', tasksTotal: 3, tasksDone: 3, tasksAccepted: 0, durationMs: 60_000 }) }),
    input({ plan: plan({ phase: 'completed', tasksTotal: 3, tasksDone: 3, tasksAccepted: 1 }) }),
    // failed
    input({ plan: plan({ phase: 'failed', failedTaskId: 'T01', failedCheck: 'compile' }) }),
    input({ plan: plan({ phase: 'failed' }) }),
    // cancelled
    input({ plan: plan({ phase: 'cancelled' }) }),
  ];

  it('no input produces an empty string', () => {
    for (const i of allInputs) {
      const result = describeEmpty(i);
      expect(result, `Input: ${JSON.stringify(i)}`).not.toBe('');
      expect(result.length, `Input: ${JSON.stringify(i)}`).toBeGreaterThan(0);
    }
  });

  it('no input produces the phrase "no data"', () => {
    for (const i of allInputs) {
      const result = describeEmpty(i);
      expect(result.toLowerCase(), `Input: ${JSON.stringify(i)}`).not.toContain('no data');
    }
  });
});
