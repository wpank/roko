// @vitest-environment jsdom
/**
 * Acceptance: a failed run says, in the plan's words, what failed —
 * "Broken on purpose: T02 failed (structural check)" — instead of the server's
 * "plan broken completed with task-level failures", and the screen offers one
 * Retry: the plan header's. The alert offers Show; the task row none.
 * Copied verbatim from plans/portal-programme/08f-final-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { pickAlert } from '@/lib/alerts';
import type { AlertInput } from '@/lib/alerts';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const PLANS = [
  { id: 'broken', title: 'Broken on purpose', task_count: 2 },
  { id: 'hello', title: 'Hello world', task_count: 2 },
].map((p) => ({ ...p, tasks_failed: 0, completed: false, status: 'pending', old_format: false }) as WirePlanSummary);

const TASKS: WirePlanTasks = {
  plan_id: 'broken',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold the hello project', tier: 'focused', status: 'pending', depends_on: [], files: ['README.md'], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: ['README.md'], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const GENERIC = 'plan broken completed with task-level failures';

const FAILED_RUN: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'broken', title: 'Broken on purpose', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'broken', tasks_total: 2 },
  { type: 'task_started', plan_id: 'broken', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'broken', task_id: 'T01', outcome: 'passed' },
  { type: 'task_started', plan_id: 'broken', task_id: 'T02', phase: 'implement' },
  { type: 'gate_result', plan_id: 'broken', task_id: 'T02', gate: 'verify[0:structural]', passed: false, output_text: '$ test -f MISSING.md\n✗ exit status 1' },
  { type: 'task_completed', plan_id: 'broken', task_id: 'T02', outcome: 'failed' },
  { type: 'error', message: GENERIC },
  { type: 'plan_completed', plan_id: 'broken', success: false },
];

const input = (over: Partial<AlertInput> = {}): AlertInput => ({
  run: foldEvents(FAILED_RUN),
  connection: 'connected',
  selectedPlanId: 'broken',
  validationErrors: 0,
  requestError: null,
  planTitles: { broken: 'Broken on purpose' },
  dismissedKey: null,
  ...over,
});

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=broken&task=T02');
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('the failure alert', () => {
  it('names the plan, the task and the check that failed', () => {
    const alert = pickAlert(input())!;
    expect(alert.text).toBe('Broken on purpose: T02 failed (structural check)');
    expect(alert.severity).toBe('error');
  });

  it('offers Show, not a second Retry', () => {
    expect(pickAlert(input())!.actions).toEqual([{ kind: 'select-task', planId: 'broken', taskId: 'T02' }]);
  });

  it('takes the title from the plan list, else the run, else the id; no check, no brackets', () => {
    expect(pickAlert(input({ planTitles: {} }))!.text).toBe('Broken on purpose: T02 failed (structural check)');
    const untitled = foldEvents(FAILED_RUN.slice(1));
    expect(pickAlert(input({ run: untitled, planTitles: {} }))!.text).toBe('broken: T02 failed (structural check)');
    const run = foldEvents(FAILED_RUN.filter((e) => e.type !== 'gate_result'));
    expect(pickAlert(input({ run }))!.text).toBe('Broken on purpose: T02 failed');
  });

  it('never shows the server’s generic line: without a known task it reads “<title> failed”', () => {
    const run = { ...initialRunState(), errors: [{ message: GENERIC, atMs: 5 }] };
    expect(pickAlert(input({ run }))!.text).toBe('Broken on purpose failed');
  });

  it('passes any other run error through unchanged', () => {
    const run = { ...initialRunState(), errors: [{ message: 'no provider could run the task', atMs: 5 }] };
    expect(pickAlert(input({ run }))!.text).toBe('no provider could run the task');
  });
});

describe('the screen after a failed run', () => {
  function renderFailed() {
    stubFetch([]);
    setStore(foldEvents(FAILED_RUN));
    renderWithClient(<Workspace />, {
      seed: [
        [queryKeys.plans, PLANS],
        [queryKeys.workspace, { name: 'preview-ws', path: '/tmp/preview-ws', branch: 'main' }],
        [queryKeys.planTasks('broken'), TASKS],
        [queryKeys.validation('broken'), VALID],
      ],
    });
  }

  it('shows the plan title and the failed check in the alert', () => {
    renderFailed();
    expect(textOf(document.querySelector('[data-region="alert"]'))).toContain('Broken on purpose: T02 failed (structural check)');
  });

  it('offers exactly one Retry, in the plan header', () => {
    renderFailed();
    const retries = [...document.querySelectorAll('button')].filter((b) => /\bRetry\b/.test(textOf(b)));
    expect(retries).toHaveLength(1);
    expect(retries[0]!.getAttribute('data-action')).toBe('retry');
    expect(retries[0]!.closest('[data-task-row], [data-region="alert"]')).toBeNull();
  });
});
