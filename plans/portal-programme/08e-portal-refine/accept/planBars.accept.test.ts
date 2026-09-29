/**
 * Acceptance: a plan's bar takes its state's colour, not its fraction's — a
 * running bar is the running colour at 10% and at 90%, a failed bar red, a
 * partial (accepted-with-failures) bar amber, and only a verified bar green.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WirePlanSummary } from '@/api/contracts';
import { buildPlanRows } from '@/lib/planRows';
import { initialRunState } from '@/lib/runState';
import type { PlanRun } from '@/lib/runState';

const DISK: WirePlanSummary = { id: 'hello', title: 'Hello', task_count: 10, tasks_done: 0, tasks_failed: 0, completed: false, status: 'pending', old_format: false };

function bar(live: Partial<PlanRun> | null, disk: Partial<WirePlanSummary> = {}): string {
  const run = initialRunState();
  const plans: Record<string, PlanRun> = live
    ? {
        hello: {
          planId: 'hello', title: null, phase: 'running', tasksTotal: 10, tasksDone: 0, tasksFailed: 0, tasksAccepted: 0,
          startedAtMs: 1_000, finishedAtMs: null, etaMinutes: null, costUsd: 0, ...live,
        } as PlanRun,
      }
    : {};
  const rows = buildPlanRows([{ ...DISK, ...disk }], { ...run, plans }, { filter: '', nowMs: 10_000 });
  return rows.groups[0]!.rows[0]!.barToken;
}

describe('plan bars', () => {
  it('keeps a running bar the running colour at any fraction', () => {
    expect(bar({ tasksDone: 1 })).toBe('var(--state-active)');
    expect(bar({ tasksDone: 9 })).toBe('var(--state-active)');
  });

  it('draws a verified plan green, a partial one amber and a failed one red', () => {
    expect(bar({ phase: 'completed', tasksDone: 10, finishedAtMs: 5_000 })).toBe('var(--state-done)');
    expect(bar({ phase: 'completed', tasksDone: 10, tasksAccepted: 2, finishedAtMs: 5_000 })).toBe('var(--state-accepted)');
    expect(bar({ phase: 'failed', tasksDone: 3, tasksFailed: 1, finishedAtMs: 5_000 })).toBe('var(--state-failed)');
  });

  it('colours a plan known only from disk by its state too', () => {
    expect(bar(null, { completed: true, tasks_done: 10 })).toBe('var(--state-done)');
    expect(bar(null, { tasks_done: 4, tasks_failed: 1 })).toBe('var(--state-failed)');
    expect(bar(null, { tasks_done: 4 })).toBe('var(--state-pending)');
  });
});
