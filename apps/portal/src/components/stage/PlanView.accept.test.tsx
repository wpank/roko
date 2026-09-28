// @vitest-environment jsdom
/**
 * Acceptance: the plan view can always run again once a run is over, says why
 * Run is disabled, keeps its clock moving, and its status line lists only what
 * it knows. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanView } from '@/components/stage/PlanView';
import { initialRunState } from '@/lib/runState';
import { foldEvents, hasMissingValue, renderWithClient, setStore, textOf } from '@/test/dom';

const T0 = 1_770_000_000_000;

const PLAN: WirePlanSummary = {
  id: 'hello',
  title: 'Hello world',
  task_count: 2,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
};

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

const HELLO_SET: WireDashboardEvent = {
  type: 'plan_set_loaded',
  plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }],
};

const DONE: WireDashboardEvent[] = [
  HELLO_SET,
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T02', outcome: 'passed' },
  { type: 'plan_completed', plan_id: 'hello', success: true },
];

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0);
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function renderView(opts: { plan?: WirePlanSummary; validation?: WireValidation } = {}) {
  return renderWithClient(
    <PlanView plan={opts.plan ?? PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />,
    {
      seed: [
        [queryKeys.planTasks('hello'), TASKS],
        [queryKeys.validation('hello'), opts.validation ?? VALID],
      ],
    },
  );
}

const primary = () =>
  document.querySelector('[data-action="run"], [data-action="run-again"], [data-action="retry"], [data-action="cancel"]') as HTMLButtonElement | null;
const reason = () => textOf(document.querySelector('[data-reason]'));
const statusLine = () => document.querySelector('[data-region="status-line"]');
const fields = () =>
  [...(statusLine()?.querySelectorAll('[data-field]') ?? [])].map((f) => [f.getAttribute('data-field'), textOf(f)]);

describe('primary action', () => {
  it('offers Run again once a run finished, though the server keeps the plan set', () => {
    setStore(foldEvents(DONE, T0 - 20_000));
    renderView();
    const button = primary()!;
    expect(button.getAttribute('data-action')).toBe('run-again');
    expect(textOf(button)).toBe('▶ Run again');
    expect(button.disabled).toBe(false);
  });

  it('offers Retry after a failed run', () => {
    setStore(
      foldEvents(
        [
          HELLO_SET,
          { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
          { type: 'plan_completed', plan_id: 'hello', success: false },
          { type: 'run_completed', outcome: 'failed', duration_ms: 900 },
        ],
        T0 - 5_000,
      ),
    );
    renderView();
    expect(primary()!.getAttribute('data-action')).toBe('retry');
    expect(textOf(primary())).toBe('↻ Retry');
    expect(primary()!.disabled).toBe(false);
  });

  it('offers Run for a plan that never ran', () => {
    renderView();
    expect(primary()!.getAttribute('data-action')).toBe('run');
    expect(textOf(primary())).toBe('▶ Run');
    expect(primary()!.disabled).toBe(false);
  });

  it('offers Cancel while the plan runs', () => {
    setStore(foldEvents([HELLO_SET, { type: 'plan_started', plan_id: 'hello', tasks_total: 2 }], T0 - 1_000));
    renderView();
    expect(primary()!.getAttribute('data-action')).toBe('cancel');
    expect(textOf(primary())).toBe('■ Cancel');
    expect(primary()!.disabled).toBe(false);
  });

  it('is disabled, and says so, while another plan’s run is active', () => {
    setStore(
      foldEvents(
        [
          { type: 'plan_set_loaded', plans: [{ plan_id: 'other', tasks_total: 3 }] },
          { type: 'plan_started', plan_id: 'other', tasks_total: 3 },
        ],
        T0 - 1_000,
      ),
    );
    renderView();
    expect(primary()!.disabled).toBe(true);
    expect(reason()).toBe('Another run is already in progress');
  });

  it('says where a queued plan stands and what it waits for', () => {
    setStore(
      foldEvents(
        [
          {
            type: 'plan_set_loaded',
            plans: [
              { plan_id: 'a', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] },
              { plan_id: 'hello', tasks_total: 2, wave: 1, depends_on: ['a'], conflicts_with: [] },
            ],
          },
          { type: 'plan_started', plan_id: 'a', tasks_total: 1 },
        ],
        T0 - 1_000,
      ),
    );
    renderView();
    expect(primary()!.disabled).toBe(true);
    expect(reason()).toBe('Queued #2 — after a');
  });

  it('is disabled by validation errors, with the count', () => {
    renderView({
      validation: {
        valid: false,
        errors: ['T02: depends on a missing task'],
        warnings: [],
        diagnostics: [{ severity: 'error', rule_id: 'PLAN_010', task_id: 'T02', message: 'depends on a missing task' }],
      },
    });
    expect(primary()!.disabled).toBe(true);
    expect(reason()).toBe('Fix 1 validation error first');
  });

  it('shows no reason while Run is available', () => {
    renderView();
    expect(document.querySelector('[data-reason]')).toBeNull();
  });
});

describe('status line', () => {
  it('lists tasks, waves and parallelism before a run, then the validation badge', () => {
    renderView();
    const line = statusLine()!;
    expect(line.classList.contains('rd-status')).toBe(true);
    expect(fields()).toEqual([
      ['tasks', '2 tasks'],
      ['waves', '2 waves'],
      ['parallel', 'parallel 1'],
    ]);
    expect(textOf(line)).toContain('valid ✓');
  });

  it('adds the estimate when the plan has one', () => {
    renderView({ plan: { ...PLAN, estimated_minutes: 9 } });
    expect(fields()).toContainEqual(['estimate', '~9 min']);
  });

  it('has no placeholder dots', () => {
    renderView();
    const text = textOf(statusLine()).trim();
    expect(text).not.toMatch(/·\s*·/);
    expect(text.startsWith('·')).toBe(false);
    expect(text.endsWith('·')).toBe(false);
  });

  it('shows a queued plan its pre-run facts, not an empty progress count', () => {
    setStore(
      foldEvents(
        [
          {
            type: 'plan_set_loaded',
            plans: [
              { plan_id: 'a', tasks_total: 1 },
              { plan_id: 'hello', tasks_total: 2 },
            ],
          },
          { type: 'plan_started', plan_id: 'a', tasks_total: 1 },
        ],
        T0 - 1_000,
      ),
    );
    renderView();
    expect(fields().map(([k]) => k)).toEqual(['tasks', 'waves', 'parallel']);
    expect(document.querySelector('[role="progressbar"]')).toBeNull();
  });

  it('keeps the elapsed time moving while the plan runs', () => {
    setStore(foldEvents([HELLO_SET, { type: 'plan_started', plan_id: 'hello', tasks_total: 2 }], T0 - 5_000, 0));
    renderView();
    expect(fields()).toContainEqual(['elapsed', '5s']);
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(fields()).toContainEqual(['elapsed', '8s']);
  });

  it('shows progress and elapsed after a run, without an estimate or a zero cost', () => {
    setStore(foldEvents(DONE, T0 - 20_000, 2_000));
    renderView();
    const keys = fields().map(([k]) => k);
    expect(fields()[0]).toEqual(['progress', '2/2']);
    expect(keys).toContain('elapsed');
    expect(keys).not.toContain('eta');
    expect(keys).not.toContain('cost');
  });
});

describe('plan view', () => {
  it('never shows a missing value', () => {
    for (const events of [[], DONE, [HELLO_SET, { type: 'plan_started', plan_id: 'hello', tasks_total: 2 } as WireDashboardEvent]]) {
      setStore(foldEvents(events, T0 - 3_000));
      const { container } = renderView();
      expect(hasMissingValue(textOf(container))).toBe(false);
      cleanup();
    }
  });
});
