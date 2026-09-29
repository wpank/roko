// @vitest-environment jsdom
/**
 * bug-b69a47: the header shows what the running run has cost (design §2, §12),
 * not every cost the server has recorded since it started.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Header } from '@/components/shell/Header';
import { fromSnapshot, initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

const T0 = 1_750_000_000_000;
const SEED = [[queryKeys.workspace, { name: 'ws', path: '/tmp/ws', branch: 'main' }]] as const;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0);
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function renderHeader(runningPlanIds: string[]) {
  return renderWithClient(
    <Header
      runningPlanIds={runningPlanIds}
      selectedPlanId={null}
      onSelectPlan={() => {}}
      onCancelRun={() => {}}
    />,
    { seed: SEED },
  );
}

const runText = () => textOf(document.querySelector('[data-slot="run"]'));

function helloRun(spend: number[]): WireDashboardEvent[] {
  return [
    { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 1 }] },
    { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
    { type: 'task_started', plan_id: 'hello', task_id: 'T1', phase: 'implement' },
    ...spend.map(
      (value): WireDashboardEvent => ({
        type: 'efficiency_event',
        plan_id: 'hello',
        task_id: 'T1',
        metric: 'cost_usd',
        value,
      }),
    ),
  ];
}

const FINISH: WireDashboardEvent[] = [
  { type: 'task_completed', plan_id: 'hello', task_id: 'T1', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'hello', success: true },
  { type: 'run_completed', outcome: 'succeeded', duration_ms: 60_000 },
];

describe('Header cost', () => {
  it("cost restarts with each run: a second run's header starts at its own cost", () => {
    const first = [...helloRun([6, 4.2]), ...FINISH];

    setStore(foldEvents([...first, ...helloRun([])], T0 - 5_000));
    renderHeader(['hello']);
    expect(runText()).toContain('Hello world 0/1');
    expect(runText()).not.toContain('$');
    cleanup();

    setStore(foldEvents([...first, ...helloRun([0.21])], T0 - 5_000));
    renderHeader(['hello']);
    expect(runText()).toContain('$0.21');
    expect(runText()).not.toContain('$10.41');
  });

  it('cost restarts with each run after a reload, which reads the plan costs', () => {
    const snapshot: WireDashboardSnapshot = {
      plans: {
        hello: {
          plan_id: 'hello',
          phase: 'started',
          active: true,
          tasks_total: 1,
          tasks_done: 0,
          tasks_failed: 0,
          started_at_ms: T0 - 5_000,
          finished_at_ms: null,
          cost_usd: 0.21,
        },
      },
      plan_set: {
        plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 1 }],
        tasks_total: 1,
        loaded_at_ms: T0 - 5_000,
      },
      tasks: {},
      agents: {},
      gates: [],
      errors: [],
      // Everything the server has spent since it started.
      stats: { cost_usd_total: 10.41, total_input_tokens: 0, total_output_tokens: 0 },
    };
    setStore(fromSnapshot(snapshot, T0));
    renderHeader(['hello']);
    expect(runText()).toContain('Hello world 0/1 · 5s · $0.21');
  });
});
