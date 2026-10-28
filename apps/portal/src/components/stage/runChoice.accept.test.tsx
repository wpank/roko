// @vitest-environment jsdom
/**
 * gap-0eb763: Run and Run again keep contract P-7 (a body-less execute runs fresh), so the portal asks first when
 * the plan's last run can be resumed (Will's call of 2026-10-27, option B). It reads the plan's `resumable_run`
 * before running: with none it runs at once; with one it offers Resume (`{resume: true}`) or Start over (no body),
 * worded "only the delivery is left" when every task passed but a crash cut the delivery (bug-4fadf8); and after a
 * run that started over it says what that run set aside, when a resume could have continued it.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, waitFor } from '@testing-library/react';
import type { WireCheckpointRun, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanView } from '@/components/stage/PlanView';
import { initialRunState } from '@/lib/runState';
import { useDashboardStore } from '@/stores/dashboard';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

const PLAN: WirePlanSummary = {
  id: 'hello',
  title: 'Hello world',
  task_count: 3,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
};

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 3,
  max_parallel: 1,
  tasks: ['T01', 'T02', 'T03'].map((id, i) => ({
    id,
    title: `Task ${id}`,
    tier: 'focused',
    status: 'pending',
    depends_on: i === 0 ? [] : [`T0${i}`],
    files: [],
    completed: false,
    verify_phases: ['structural'],
  })),
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const STOPPED = 'graph-hello-8f3c2a1b-5d4e-4c1a-9f0e-2b7d6c5a4e3f';

/** A run that passed T01 and stopped with T02 and T03 left. */
const TASKS_LEFT: WireCheckpointRun = {
  run_id: STOPPED,
  status: 'interrupted',
  passed_tasks: ['T01'],
  tasks_left: ['T02', 'T03'],
  resumable: true,
};

/** A run that passed every task, but whose delivery a crash cut (bug-4fadf8). */
const DELIVERY_LEFT: WireCheckpointRun = {
  run_id: STOPPED,
  status: 'interrupted',
  passed_tasks: ['T01', 'T02', 'T03'],
  tasks_left: [],
  resumable: true,
  delivery_pending: true,
  spent_usd: 0.164,
};

/** A run that passed every task and delivered them. */
const DELIVERED: WireCheckpointRun = {
  run_id: STOPPED,
  status: 'succeeded',
  passed_tasks: ['T01', 'T02', 'T03'],
  tasks_left: [],
  resumable: false,
};

const EXECUTE = '/api/plans/hello/execute';

/** The server: the plan with `resumable` as its resumable run, and execute accepting with `accepted`. */
function serve(resumable: WireCheckpointRun | null, accepted: Record<string, unknown> = { id: 'run-2' }) {
  return stubFetch([
    { path: '/api/plans/hello', body: { ...PLAN, resumable_run: resumable } },
    { method: 'POST', path: EXECUTE, status: 202, body: accepted },
  ]);
}

/** The bodies of the execute calls made so far: `undefined` for a body-less one. */
function executeBodies(fetchMock: ReturnType<typeof stubFetch>): unknown[] {
  return fetchMock.mock.calls
    .filter(([input, init]) => (init as RequestInit | undefined)?.method === 'POST' && String(input).endsWith(EXECUTE))
    .map(([, init]) => {
      const body = (init as RequestInit).body;
      return body === undefined ? undefined : JSON.parse(String(body));
    });
}

function renderView() {
  return renderWithClient(
    <PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />,
    {
      seed: [
        [queryKeys.planTasks('hello'), TASKS],
        [queryKeys.validation('hello'), VALID],
      ],
    },
  );
}

const run = () => document.querySelector('[data-action="run"]') as HTMLButtonElement;
const question = () => document.querySelector('[data-region="run-choice"]');
const setAside = () => document.querySelector('[data-region="set-aside"]');
const choice = (which: 'resume' | 'start-over') =>
  document.querySelector(`[data-choice="${which}"]`) as HTMLButtonElement;
/** Let a run's answer reach the page. */
const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

beforeEach(() => {
  setStore(initialRunState());
  useDashboardStore.setState({ runChoice: null, startedOver: null });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('Run asks before a fresh run sets a resumable run aside', () => {
  it('runs at once, with no body, when the plan has no run to resume', async () => {
    // A delivered run that a fresh run sets aside held nothing a resume would continue: no notice.
    const fetchMock = serve(null, { id: 'run-2', set_aside: DELIVERED });
    renderView();
    await act(async () => {
      fireEvent.click(run());
    });
    await waitFor(() => expect(executeBodies(fetchMock)).toEqual([undefined]));
    await settle();
    expect(question()).toBeNull();
    expect(setAside()).toBeNull();
  });

  it('asks before setting aside a run with tasks left, and Resume continues it', async () => {
    const fetchMock = serve(TASKS_LEFT);
    renderView();
    await act(async () => {
      fireEvent.click(run());
    });
    await waitFor(() => expect(question()).not.toBeNull());
    expect(textOf(question())).toContain(
      "This plan's last run, 8f3c2a1b (interrupted), passed T01, and T02 and T03 are left.",
    );
    expect(textOf(question())).toContain('Resume runs only T02 and T03.');
    expect(executeBodies(fetchMock)).toEqual([]);

    await act(async () => {
      fireEvent.click(choice('resume'));
    });
    await waitFor(() => expect(executeBodies(fetchMock)).toEqual([{ resume: true }]));
    expect(question()).toBeNull();
  });

  it('says only the delivery is left when every task passed but a crash cut the delivery', async () => {
    serve(DELIVERY_LEFT);
    renderView();
    await act(async () => {
      fireEvent.click(run());
    });
    await waitFor(() => expect(question()).not.toBeNull());
    expect(textOf(question())).toContain(
      "This plan's last run, 8f3c2a1b (interrupted), passed T01, T02 and T03, and only the delivery is left.",
    );
    expect(textOf(question())).toContain('Resume runs no task again, only its delivery.');
    expect(textOf(question())).toContain('the $0.16 it spent stays spent');
  });

  it('starts over with no body when chosen, and says what it set aside', async () => {
    const fetchMock = serve(TASKS_LEFT, { id: 'run-2', set_aside: TASKS_LEFT });
    renderView();
    await act(async () => {
      fireEvent.click(run());
    });
    await waitFor(() => expect(question()).not.toBeNull());

    await act(async () => {
      fireEvent.click(choice('start-over'));
    });
    await waitFor(() => expect(executeBodies(fetchMock)).toEqual([undefined]));
    expect(question()).toBeNull();
    await waitFor(() => expect(setAside()).not.toBeNull());
    expect(textOf(setAside())).toContain(
      'Started over: set aside run 8f3c2a1b (interrupted; passed T01; left T02 and T03). ' +
        'In a terminal, roko resume 8f3c2a1b puts it back.',
    );
    expect(textOf(setAside()?.querySelector('code') ?? null)).toBe('roko resume 8f3c2a1b');
  });

  it('leaves Retry as it was: it resumes at once, without reading the plan or asking', async () => {
    setStore(
      foldEvents([
        { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 3 }] },
        { type: 'plan_started', plan_id: 'hello', tasks_total: 3 },
        { type: 'plan_completed', plan_id: 'hello', success: false },
      ]),
    );
    const fetchMock = serve(TASKS_LEFT);
    renderView();
    const retry = document.querySelector('[data-action="retry"]') as HTMLButtonElement;
    await act(async () => {
      fireEvent.click(retry);
    });
    await waitFor(() => expect(executeBodies(fetchMock)).toEqual([{ resume: true }]));
    expect(fetchMock.mock.calls.filter(([input]) => String(input).endsWith('/api/plans/hello'))).toEqual([]);
    expect(question()).toBeNull();
  });
});
