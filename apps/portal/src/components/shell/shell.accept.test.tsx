// @vitest-environment jsdom
/**
 * Acceptance: the header is one row — workspace, what is running, connection —
 * with a live clock; the alert is one styled row. Copied verbatim from
 * plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { AlertBand } from '@/components/shell/AlertBand';
import { Header } from '@/components/shell/Header';
import { initialRunState } from '@/lib/runState';
import { foldEvents, hasMissingValue, renderWithClient, setStore, textOf } from '@/test/dom';

const T0 = 1_750_000_000_000;
const WORKSPACE = { name: 'preview-ws', path: '/tmp/preview-ws', branch: 'main' };
const SEED = [[queryKeys.workspace, WORKSPACE]] as const;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0);
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function renderHeader(runningPlanIds: string[], handlers: Partial<{ onSelectPlan: (id: string) => void; onCancelRun: (id: string) => void }> = {}) {
  return renderWithClient(
    <Header
      runningPlanIds={runningPlanIds}
      selectedPlanId={null}
      onSelectPlan={handlers.onSelectPlan ?? (() => {})}
      onCancelRun={handlers.onCancelRun ?? (() => {})}
    />,
    { seed: SEED },
  );
}

const header = () => document.querySelector('header[data-region="header"]');
const runSlot = () => document.querySelector('[data-slot="run"]');

const HELLO_RUN: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
];

const SET_RUN: WireDashboardEvent[] = [
  {
    type: 'plan_set_loaded',
    plans: [
      { plan_id: 'a', title: 'Part A', tasks_total: 2 },
      { plan_id: 'b', title: 'Part B', tasks_total: 2 },
      { plan_id: 'c', title: 'Part C', tasks_total: 3 },
    ],
  },
  { type: 'plan_started', plan_id: 'b', tasks_total: 2 },
  { type: 'task_started', plan_id: 'b', task_id: 'T1', phase: 'implement' },
  { type: 'task_completed', plan_id: 'b', task_id: 'T1', outcome: 'passed' },
  { type: 'task_started', plan_id: 'b', task_id: 'T2', phase: 'implement' },
  { type: 'task_completed', plan_id: 'b', task_id: 'T2', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'b', success: true },
  { type: 'plan_started', plan_id: 'a', tasks_total: 2 },
  { type: 'task_started', plan_id: 'a', task_id: 'T1', phase: 'implement' },
  { type: 'task_completed', plan_id: 'a', task_id: 'T1', outcome: 'passed' },
  { type: 'efficiency_event', plan_id: 'a', task_id: 'T1', metric: 'cost_usd', value: 0.21 },
];

describe('Header', () => {
  it('is one styled row with the workspace and the connection', () => {
    renderHeader([]);
    expect(header()?.classList.contains('rd-header')).toBe(true);
    expect(textOf(document.querySelector('[data-slot="workspace"]'))).toBe('preview-ws · main');
    expect(document.querySelector('[data-slot="connection"][data-connection="connected"]')).not.toBeNull();
    expect(runSlot()).toBeNull();
  });

  it('titles the document after the workspace', () => {
    renderHeader([]);
    expect(document.title).toBe('roko · preview-ws');
  });

  it('names a single running plan with its progress and a running clock', () => {
    setStore(foldEvents(HELLO_RUN, T0 - 5_000));
    renderHeader(['hello']);
    expect(textOf(runSlot())).toContain('Hello world 0/2 · 5s');
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(textOf(runSlot())).toContain('Hello world 0/2 · 8s');
  });

  it('summarises a plan set and shows the cost once there is one', () => {
    setStore(foldEvents(SET_RUN, T0 - 72_000));
    renderHeader(['a']);
    const text = textOf(runSlot());
    expect(text).toContain('1 running · 1/3 plans · 3/7 tasks · 1m12s');
    expect(text).toContain('$0.21');
  });

  it('leaves the cost out while nothing has been spent', () => {
    setStore(foldEvents(HELLO_RUN, T0 - 5_000));
    renderHeader(['hello']);
    expect(textOf(runSlot())).not.toContain('$');
  });

  it('shows the remaining estimate when the engine reports one', () => {
    setStore(
      foldEvents([...HELLO_RUN, { type: 'critical_path_eta_updated', plan_id: 'hello', eta_minutes: 4 }], T0 - 5_000),
    );
    renderHeader(['hello']);
    expect(textOf(runSlot())).toContain('· ~4m');
  });

  it('drops the run summary once nothing runs, though the plan set remains', () => {
    setStore(
      foldEvents([...HELLO_RUN, { type: 'plan_completed', plan_id: 'hello', success: true }], T0 - 5_000),
    );
    renderHeader([]);
    expect(runSlot()).toBeNull();
    expect(textOf(header())).not.toContain('running');
  });

  it('steps to the next running plan and cancels the run', () => {
    const onSelectPlan = vi.fn();
    const onCancelRun = vi.fn();
    setStore(foldEvents(SET_RUN, T0 - 1_000));
    renderHeader(['a', 'c'], { onSelectPlan, onCancelRun });
    fireEvent.click(document.querySelector('[data-action="next-running"]')!);
    expect(onSelectPlan).toHaveBeenCalledWith('a');
    fireEvent.click(document.querySelector('[data-action="cancel-run"]')!);
    expect(onCancelRun).toHaveBeenCalledWith('a');
  });

  it('never shows a missing value', () => {
    const cases: Array<[WireDashboardEvent[], string[]]> = [
      [[], []],
      [HELLO_RUN, ['hello']],
      [SET_RUN, ['a']],
    ];
    for (const [events, running] of cases) {
      setStore(foldEvents(events, T0 - 2_000));
      renderHeader(running);
      expect(hasMissingValue(textOf(header()))).toBe(false);
      cleanup();
    }
  });
});

describe('AlertBand', () => {
  it('renders one styled row carrying its severity, text and actions', () => {
    const onAction = vi.fn();
    const onDismiss = vi.fn();
    const { container } = render(
      <AlertBand
        alert={{
          key: 'k1',
          severity: 'error',
          text: 'T03 failed: test — tests/hello.rs:7',
          actions: [{ kind: 'select-task', planId: 'hello', taskId: 'T03' }, { kind: 'retry', planId: 'hello' }],
        }}
        onAction={onAction}
        onDismiss={onDismiss}
      />,
    );
    const band = container.querySelector('[data-region="alert"]') as HTMLElement;
    expect(band.classList.contains('rd-alert')).toBe(true);
    expect(band.getAttribute('data-severity')).toBe('error');
    expect(band.style.backgroundColor).toBe('');
    expect(textOf(band)).toContain('T03 failed: test — tests/hello.rs:7');
    const buttons = [...band.querySelectorAll('button')].map((b) => b.textContent?.trim());
    expect(buttons).toEqual(expect.arrayContaining(['Show', 'Retry']));
    fireEvent.click(band.querySelector('[aria-label="Dismiss alert"]')!);
    expect(onDismiss).toHaveBeenCalledWith('k1');
  });

  it('takes no space without an alert', () => {
    const { container } = render(<AlertBand alert={null} onAction={() => {}} onDismiss={() => {}} />);
    expect(container.innerHTML).toBe('');
  });
});
