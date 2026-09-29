// @vitest-environment jsdom
/**
 * A restarted `roko serve` no longer knows the portal's session and refuses
 * the event stream with a 401 that EventSource hides (bug-f47afb). The portal
 * asks GET /api/status why, and on a 401 asks for the new sign-in link instead
 * of saying it lost the server.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, waitFor } from '@testing-library/react';
import { ApiError } from '@/api/client';
import type { WireDashboardEvent, WirePlanSummary, WireStateHubSnapshotResponse } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { SIGN_IN_HINT, startLiveState, type ConnectionStatus } from '@/lib/bootstrap';
import { initialRunState } from '@/lib/runState';
import { useDashboardStore } from '@/stores/dashboard';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const PLANS = [
  { id: 'hello', title: 'Hello world', task_count: 1, tasks_failed: 0, completed: false, status: 'pending', old_format: false },
] as unknown as WirePlanSummary[];

const SEED = [
  [queryKeys.plans, PLANS],
  [queryKeys.workspace, { name: 'hello', path: '/tmp/hello' }],
  [queryKeys.planTasks('hello'), { plan_id: 'hello', task_count: 1, tasks: [] }],
  [queryKeys.validation('hello'), { valid: true, errors: [], warnings: [], diagnostics: [] }],
] as const;

/** The last run failed: a stale task failure that the alert shows while connected. */
const FAILED_RUN: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
  { type: 'plan_completed', plan_id: 'hello', success: false },
];

const SNAPSHOT: WireStateHubSnapshotResponse = {
  cursor: '0x1',
  state: {
    plans: {},
    tasks: {},
    agents: {},
    gates: [],
    errors: [],
    stats: { cost_usd_total: 0, total_input_tokens: 0, total_output_tokens: 0 },
  },
};

const settle = () =>
  act(async () => {
    for (let i = 0; i < 20; i++) await Promise.resolve();
  });

/**
 * Start the live state against the store with a fake stream; returns a
 * function that reports a stream status and lets the probe answer.
 */
async function live(opts: {
  probe: () => Promise<number>;
  fetchSnapshot?: () => Promise<WireStateHubSnapshotResponse>;
}): Promise<(status: ConnectionStatus) => Promise<void>> {
  let onStatus: ((s: ConnectionStatus) => void) | null = null;
  const handle = startLiveState({
    hash: '',
    postSession: async () => 204,
    dropFragment: () => {},
    setSession: () => {},
    fetchSnapshot: opts.fetchSnapshot ?? (async () => SNAPSHOT),
    probe: opts.probe,
    openStream: (_lastEventId, _onEvent, s) => {
      onStatus = s;
      return { close() {} };
    },
    // The test keeps its own run state.
    replace: () => {},
    apply: () => {},
    setStatus: (s) => useDashboardStore.getState().setConnection(s),
    sleep: () => new Promise(() => {}),
  });
  stops.push(handle.stop);
  await settle();
  return async (status) => {
    act(() => onStatus!(status));
    await settle();
  };
}

let stops: Array<() => void> = [];

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(foldEvents(FAILED_RUN));
  stubFetch([]);
});

afterEach(() => {
  stops.forEach((stop) => stop());
  stops = [];
  cleanup();
  vi.unstubAllGlobals();
});

const alert = () => document.querySelector('[data-region="alert"]');
const alertText = () => textOf(alert()?.querySelector('.rd-alert__text') ?? null);
const actions = () => [...(alert()?.querySelectorAll('.rd-alert__action') ?? [])].map((b) => textOf(b));
const dot = () => document.querySelector('[data-slot="connection"]') as HTMLElement;

describe('after roko serve restarts', () => {
  it('asks for the new sign-in link once the stream is refused and the probe answers 401', async () => {
    const probe = vi.fn(async () => 401);
    renderWithClient(<Workspace />, { seed: SEED });
    const stream = await live({ probe });
    await stream('connected');
    expect(alertText()).toBe('Hello world: T01 failed');

    await stream('disconnected');
    expect(probe).toHaveBeenCalledTimes(1);
    expect(alertText()).toBe(SIGN_IN_HINT);
    expect(actions()).toEqual([]);
    expect(dot().getAttribute('data-connection')).toBe('unauthorized');
    expect(dot().title).toBe('Not signed in');

    // A retry keeps asking, rather than flickering to another alert.
    await stream('connecting');
    expect(alertText()).toBe(SIGN_IN_HINT);

    // The new link was opened in another tab: the next stream opens.
    await stream('connected');
    expect(alertText()).toBe('Hello world: T01 failed');
  });

  it('asks for the new sign-in link when a reload finds the session gone', async () => {
    // A fresh page: nothing cached, and the API refuses every request.
    stubFetch([
      { path: '/api/plans', status: 401 },
      { path: '/api/status', status: 401 },
    ]);
    renderWithClient(<Workspace />);
    await live({
      probe: async () => 401,
      fetchSnapshot: async () => {
        throw new ApiError(401, 'Unauthorized', null);
      },
    });
    expect(alertText()).toBe(SIGN_IN_HINT);
    // The rail and the stage say so too, rather than "no plans" or a lost server.
    await waitFor(() => expect(textOf(document.querySelector('[data-region="rail"]'))).toContain(SIGN_IN_HINT));
    expect(textOf(document.querySelector('[data-region="stage"]'))).toContain(SIGN_IN_HINT);
    expect(textOf(document.body)).not.toMatch(/Lost the server|No plans in/);
  });

  it('still says it lost the server while the server is down', async () => {
    setStore(initialRunState());
    renderWithClient(<Workspace />, { seed: SEED });
    const stream = await live({ probe: async () => 0 });
    await stream('disconnected');
    expect(alertText()).toBe('Lost the server; reconnecting.');
    expect(actions()).toEqual(['Reconnect']);
  });
});
