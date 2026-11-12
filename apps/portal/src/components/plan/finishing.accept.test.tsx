// @vitest-environment jsdom
/**
 * Acceptance for bug-33f5a6: a plan's own end comes about a second before the end of the run that holds it, and
 * until then the server refuses to revise the plan or run it again. The page waits for the run: revise and run again
 * are held, saying why, and come back when the run ends, or after a while if it never says so. A 409 from revise
 * says what the server said.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { ApiError } from '@/api/client';
import { toRevise409Error } from '@/components/stage/PromptPanel';
import { FINISHING_MS } from '@/lib/planSet';
import { initialRunState } from '@/lib/runState';
import { useDashboardStore } from '@/stores/dashboard';
import { setStore, textOf } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const T0 = 1_780_000_000_000;
const PLAN = planSummary('greeting', 'Say hello from the command line', 1);
const TASKS = planTasks('greeting', [['T1', 'Write the greeting module', []]]);

/** A one-plan run whose plan has passed; its run_completed is still to come. */
const PLAN_PASSED: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'greeting', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] }] },
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 1 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T1', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'greeting', success: true },
];

beforeEach(() => {
  vi.useFakeTimers();
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const key = (action: string) => document.querySelector<HTMLButtonElement>(`[data-action="${action}"]`)!;

describe('while the run that holds the plan finishes', () => {
  it('holds revise and run again, says why, and lets them go when the run ends', () => {
    vi.setSystemTime(T0 + 600);
    renderPlanPage(PLAN, TASKS, PLAN_PASSED, { startMs: T0 });
    expect(textOf(document.querySelector('[data-region="plan"] .rk-pill'))).toBe('passed');
    expect(key('revise').disabled).toBe(true);
    expect(key('run-again').disabled).toBe(true);
    expect(textOf(document.querySelector('[data-reason]'))).toBe('roko is finishing the run');

    act(() => {
      useDashboardStore.getState().applyEvent({ type: 'run_completed', outcome: 'succeeded', duration_ms: 900 });
    });
    expect(key('revise').disabled).toBe(false);
    expect(key('run-again').disabled).toBe(false);
    expect(document.querySelector('[data-reason]')).toBeNull();
  });

  it('stops waiting when a run never says it ended', () => {
    vi.setSystemTime(T0 + FINISHING_MS + 1_000);
    renderPlanPage(PLAN, TASKS, PLAN_PASSED, { startMs: T0 });
    expect(key('revise').disabled).toBe(false);
    expect(key('run-again').disabled).toBe(false);
  });
});

describe('a 409 from revise', () => {
  const conflict = (message: string) => new ApiError(409, 'Conflict', { code: 'conflict', message });

  it('says the plan is in a run that hasn’t finished when the server says so', () => {
    expect(
      toRevise409Error(
        conflict("plan 'greeting' is part of an active run (run key: r1); finish or cancel the run before revising the plan"),
      ),
    ).toBe('This plan is part of a run that hasn’t finished; revise it when the run ends.');
  });

  it('says the plan is already being revised only when another revision is in progress', () => {
    expect(toRevise409Error(conflict("a revision of plan 'greeting' is already in progress"))).toBe(
      'This plan is already being revised.',
    );
  });
});
