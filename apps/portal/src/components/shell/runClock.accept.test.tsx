// @vitest-environment jsdom
/**
 * Acceptance for bug-b6d5cb: every reading of a run on the page follows the definitions the run card's glass uses
 * (gap-c1726f), not only the glass.
 * - The top bar's clock, the plan's row in the list and the header's "started" count from when roko accepted the
 *   run, as TIME does, not from when its tasks began.
 * - Once the run has ended, the row reads roko's own duration, as TIME does.
 * - The row's spend is SPENT's: a resume's earlier spend counts.
 * Phase 7.2's shape: the tasks began about a minute after roko accepted the run.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, waitFor } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';
import { VALID, planSummary, planTasks } from '@/test/plan';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const PLAN = planSummary('p7-2', 'P7.2 · The window as designed', 2);
const TASKS = planTasks('p7-2', [
  ['T1', 'Redraw the knobs and switches', []],
  ['T2', 'Redraw the console', ['T1']],
]);
const RUNNING: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'p7-2', tasks_total: 2 },
  { type: 'task_started', plan_id: 'p7-2', task_id: 'T1', phase: 'implement' },
];
const PASSED: WireDashboardEvent[] = [
  // roko says what the run's earlier processes spent just before the plan starts again.
  { type: 'budget_carried', plan_id: 'p7-2', carried_usd: 1, ceiling_usd: 5, detail: '' },
  ...RUNNING,
  { type: 'efficiency_event', plan_id: 'p7-2', task_id: 'T1', metric: 'cost_usd', value: 0.32 },
  { type: 'task_completed', plan_id: 'p7-2', task_id: 'T1', outcome: 'passed' },
  { type: 'task_started', plan_id: 'p7-2', task_id: 'T2', phase: 'implement' },
  { type: 'task_completed', plan_id: 'p7-2', task_id: 'T2', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'p7-2', success: true },
  { type: 'run_completed', outcome: 'succeeded', duration_ms: 1_333_000 },
];

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=p7-2');
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

/** Render the workspace on p7-2 with `events`, the tasks having begun at `tasksBeganMs`, roko having accepted the
 * run at `acceptedMs`. */
function renderRun(events: readonly WireDashboardEvent[], tasksBeganMs: number, acceptedMs: number, status: string) {
  const detail = {
    ...PLAN,
    last_run: {
      run_id: '583b4be5-87a7-463c-b2e1-7beac0395b56',
      status,
      passed_tasks: [],
      tasks_left: [],
      resumable: false,
      max_usd: 5,
      started_at: new Date(acceptedMs).toISOString(),
    },
  };
  stubFetch([{ path: '/api/plans/p7-2', body: detail }]);
  setStore(foldEvents(events, tasksBeganMs));
  renderWithClient(<Workspace />, {
    seed: [
      [queryKeys.plans, [PLAN]],
      [queryKeys.workspace, { name: 'undertow', path: '/tmp/undertow', branch: 'main' }],
      [queryKeys.planTasks('p7-2'), TASKS],
      [queryKeys.validation('p7-2'), VALID],
      [queryKeys.gateRungs, []],
      [queryKeys.planCosts('p7-2'), 5],
    ],
  });
}

const timeLabel = () => document.querySelector('[data-reading="time"] svg')?.getAttribute('aria-label') ?? '';
const timeNote = () => textOf(document.querySelector('[data-reading="time"] .rk-cell-note'));
const chipLabel = () => document.querySelector('[data-slot="run"] svg')?.getAttribute('aria-label') ?? '';
const row = () => textOf(document.querySelector('[data-plan-row="p7-2"]'));
const when = () => textOf(document.querySelector('.rk-card-when'));
const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

describe('a running run', () => {
  it('reads one time everywhere: the glass, the top bar, the list and the header count from roko accepting it', async () => {
    const now = Date.now();
    // roko accepted the run 200 s ago; its tasks began 100 s later, so the two starts are minutes apart.
    renderRun(RUNNING, now - 100_000, now - 200_000, 'running');
    await waitFor(() => expect(timeLabel()).toMatch(/^time: 3 minutes (19|20|21) seconds$/));
    expect(chipLabel()).toMatch(/^3 minutes (19|20|21) seconds$/);
    expect(row()).toMatch(/3:(19|20|21)/);
    expect(timeNote()).toBe(`started ${hhmm(now - 200_000)}`);
    expect(when()).toBe(timeNote());
  });
});

describe('an ended run', () => {
  it('reads roko’s own duration and the whole run’s spend in the list, as on the glass', async () => {
    const now = Date.now();
    renderRun(PASSED, now - 1_272_000, now - 1_333_000, 'succeeded');
    await waitFor(() => expect(document.querySelector('[data-reading="spent"] svg')?.getAttribute('aria-label')).toBe('spent: $1.32 of $5.00'));
    expect(timeLabel()).toBe('time: 22 minutes 13 seconds');
    expect(row()).toContain('22m 13s');
    expect(row()).toContain('$1.32');
  });
});
