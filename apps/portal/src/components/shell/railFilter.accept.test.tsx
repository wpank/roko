// @vitest-environment jsdom
/**
 * Acceptance: the rail filter (`/`) is a view (design §3). It narrows the rows
 * the rail shows and nothing else: a group ▶ and Run all run, and confirm,
 * every plan they cover; the header reports and cancels a run the filter
 * hides; and the selection stays on a plan the filter hides (§9).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

// As in Next 15, replaceState re-renders every reader of the search params.
vi.mock('next/navigation', async () => {
  const React = await import('react');
  const listeners = new Set<() => void>();
  const replaceState = window.history.replaceState.bind(window.history);
  window.history.replaceState = (...args: Parameters<History['replaceState']>) => {
    replaceState(...args);
    listeners.forEach((notify) => notify());
  };
  const subscribe = (notify: () => void) => {
    listeners.add(notify);
    return () => listeners.delete(notify);
  };
  return {
    useSearchParams: () => {
      const search = React.useSyncExternalStore(subscribe, () => window.location.search);
      return React.useMemo(() => new URLSearchParams(search), [search]);
    },
  };
});

const plan = (id: string, title: string, group?: string): WirePlanSummary => ({
  id,
  title,
  task_count: 2,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
  ...(group ? { group } : {}),
});

const PLANS = [
  plan('hello', 'Hello world'),
  plan('pp-01', 'Backend plans', 'portal-programme'),
  plan('pp-02', 'Frontend plans', 'portal-programme'),
  plan('pp-03', 'Acceptance', 'portal-programme'),
];

const tasksOf = (id: string): WirePlanTasks => ({
  plan_id: id,
  task_count: 2,
  max_parallel: 1,
  tasks: ['T01', 'T02'].map((t) => ({
    id: t, title: `Task ${t}`, tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'],
  })),
});

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const SEED = [
  [queryKeys.plans, PLANS],
  [queryKeys.workspace, { name: 'ws', path: '/tmp/ws', branch: 'main' }],
  ...PLANS.flatMap((p) => [
    [queryKeys.planTasks(p.id), tasksOf(p.id)] as const,
    [queryKeys.validation(p.id), VALID] as const,
  ]),
] as const;

const running = (id: string, title: string): WireDashboardEvent[] => [
  { type: 'plan_set_loaded', plans: [{ plan_id: id, title, tasks_total: 2 }] },
  { type: 'plan_started', plan_id: id, tasks_total: 2 },
];

const ACCEPTED = { id: 'run-1', order: [], max_parallel_plans: 1 };

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(initialRunState());
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const filterTo = (value: string) =>
  fireEvent.change(document.querySelector('input[aria-label="Filter plans"]')!, { target: { value } });
const shownRows = () =>
  [...document.querySelectorAll('[data-region="rail"] [data-plan-row]')].map((r) => r.getAttribute('data-plan-row'));
const selectedPlan = () => new URLSearchParams(window.location.search).get('plan');
const stageTitle = () => textOf(document.querySelector('[data-region="stage"] h2'));

/** The server as these tests need it: the plan list (refetched after a run) plus `routes`. */
const serve = (...routes: Parameters<typeof stubFetch>[0]) =>
  stubFetch([{ path: '/api/plans', body: PLANS }, ...routes]);

/** The JSON bodies POSTed to `path`, in order (undefined for none). */
function posted(fetchMock: ReturnType<typeof stubFetch>, path: string): unknown[] {
  return fetchMock.mock.calls
    .filter(([input, init]) => init?.method === 'POST' && new URL(String(input), 'http://localhost').pathname === path)
    .map(([, init]) => (init?.body === undefined ? undefined : JSON.parse(String(init.body))));
}

describe('the rail filter', () => {
  it('the filter does not change what a group ▶ runs, or its confirm', async () => {
    const fetchMock = serve({ method: 'POST', path: '/api/plans/execute', status: 202, body: ACCEPTED });
    renderWithClient(<Workspace />, { seed: SEED });
    filterTo('front');
    expect(shownRows()).toEqual(['pp-02']);

    fireEvent.click(document.querySelector('[data-action="run-group"]')!);
    expect(window.confirm).toHaveBeenCalledWith('Run the 3 plans in portal-programme in dependency order?');
    await waitFor(() =>
      expect(posted(fetchMock, '/api/plans/execute')).toEqual([{ plans: ['pp-01', 'pp-02', 'pp-03'] }]),
    );
  });

  it('the filter does not change the Run all confirm count', async () => {
    const fetchMock = serve({ method: 'POST', path: '/api/plans/execute', status: 202, body: ACCEPTED });
    renderWithClient(<Workspace />, { seed: SEED });
    filterTo('front');
    expect(shownRows()).toEqual(['pp-02']);
    expect(textOf(document.querySelector('[data-region="rail"]'))).toContain('PLANS 4');

    fireEvent.click(document.querySelector('[data-action="run-all"]')!);
    expect(window.confirm).toHaveBeenCalledWith('Run all 4 plans in dependency order?');
    await waitFor(() => expect(posted(fetchMock, '/api/plans/execute')).toEqual([{}]));
  });

  it('the filter does not change the header’s running summary or its ■', async () => {
    const fetchMock = serve({ method: 'POST', path: '/api/plans/hello/cancel', body: {} });
    window.history.replaceState(null, '', '/?plan=pp-02');
    setStore(foldEvents(running('hello', 'Hello world')));
    renderWithClient(<Workspace />, { seed: SEED });
    filterTo('front');
    expect(shownRows()).toEqual(['pp-02']);

    const run = document.querySelector('[data-slot="run"]');
    expect(textOf(run)).toContain('Hello world 0/2');
    expect(document.querySelector('[data-region="run-band"]')).not.toBeNull();
    fireEvent.click(run!.querySelector('[data-action="cancel-run"]')!);
    expect(window.confirm).toHaveBeenCalledWith('Stop the running plan "hello"?');
    await waitFor(() => expect(posted(fetchMock, '/api/plans/hello/cancel')).toHaveLength(1));

    // The running part still steps to the running plan the filter hides.
    fireEvent.click(run!.querySelector('[data-action="next-running"]')!);
    expect(selectedPlan()).toBe('hello');
  });

  it('the filter does not change the selection, even with a running plan in view', () => {
    serve();
    setStore(foldEvents(running('pp-02', 'Frontend plans')));
    renderWithClient(<Workspace />, { seed: SEED });
    expect(selectedPlan()).toBe('hello');

    filterTo('front');
    expect(shownRows()).toEqual(['pp-02']);
    expect(selectedPlan()).toBe('hello');
    expect(stageTitle()).toBe('Hello world');

    filterTo('');
    expect(selectedPlan()).toBe('hello');
  });
});
