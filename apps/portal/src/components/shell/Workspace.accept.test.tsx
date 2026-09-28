// @vitest-environment jsdom
/**
 * Acceptance: the whole screen against canned server answers — the preview
 * findings of 2026-09-28 (Run locked forever after one run, "undefined/1" in
 * the rail, frozen clocks, a raw "HTTP 405" alert) stay fixed.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, screen } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { foldEvents, hasMissingValue, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const T0 = 1_780_000_000_000;

/** Plans as today's list route sends them: no tasks_done, group or estimate. */
const PLANS = [
  { id: 'a', title: 'Demo set: part A', task_count: 1 },
  { id: 'b', title: 'Demo set: part B', task_count: 1 },
  { id: 'hello', title: 'Hello world', task_count: 2 },
].map(
  (p) => ({ ...p, tasks_failed: 0, completed: false, status: 'pending', old_format: false }) as unknown as WirePlanSummary,
);

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold the hello project', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const SEED = [
  [queryKeys.plans, PLANS],
  [queryKeys.workspace, { name: 'preview-ws', path: '/tmp/preview-ws', branch: 'main' }],
  [queryKeys.planTasks('hello'), TASKS],
  [queryKeys.validation('hello'), VALID],
] as const;

const HELLO_RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
];
const HELLO_DONE: WireDashboardEvent[] = [
  ...HELLO_RUNNING,
  { type: 'plan_completed', plan_id: 'hello', success: true },
];

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(initialRunState());
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const runAll = () => document.querySelector('[data-action="run-all"]') as HTMLButtonElement;
const rail = () => document.querySelector('[data-region="rail"]');

describe('Workspace', () => {
  it('lays out header, rail and the selected plan without missing values', () => {
    stubFetch([]);
    renderWithClient(<Workspace />, { seed: SEED });
    expect(document.querySelector('header[data-region="header"]')).not.toBeNull();
    expect(rail()!.querySelectorAll('[data-plan-row]')).toHaveLength(3);
    expect(hasMissingValue(textOf(rail()))).toBe(false);
    expect(textOf(document.querySelector('[data-region="status-line"]'))).toContain('2 tasks');
  });

  it('explains a Run all this server cannot do instead of showing a raw status', async () => {
    stubFetch([{ method: 'POST', path: '/api/plans/execute', status: 405, statusText: 'Method Not Allowed' }]);
    renderWithClient(<Workspace />, { seed: SEED });
    fireEvent.click(runAll());
    const alert = await screen.findByText(
      'This roko serve does not support running all plans yet.',
      {},
      { timeout: 1_000 },
    );
    expect(alert.closest('[data-region="alert"]')).not.toBeNull();
    expect(textOf(document.body)).not.toMatch(/HTTP 405/);
  });

  it('locks Run all while a run is active and frees it, and Run again, once it is over', () => {
    stubFetch([]);
    setStore(foldEvents(HELLO_RUNNING, T0));
    const { unmount } = renderWithClient(<Workspace />, { seed: SEED });
    expect(runAll().disabled).toBe(true);
    unmount();

    setStore(foldEvents(HELLO_DONE, T0));
    renderWithClient(<Workspace />, { seed: SEED });
    expect(runAll().disabled).toBe(false);
    const again = document.querySelector('[data-action="run-again"]') as HTMLButtonElement;
    expect(again.disabled).toBe(false);
  });

  it('keeps the rail clock of a running plan moving between events', () => {
    vi.useFakeTimers();
    vi.setSystemTime(T0);
    stubFetch([]);
    setStore(foldEvents(HELLO_RUNNING, T0 - 5_000, 0));
    renderWithClient(<Workspace />, { seed: SEED });
    const time = () => textOf(rail()!.querySelector('[data-plan-row="hello"] [data-time]'));
    expect(time()).toBe('5s');
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(time()).toBe('8s');
  });
});
