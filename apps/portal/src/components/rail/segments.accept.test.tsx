// @vitest-environment jsdom
/**
 * Acceptance: a plan's bar shows what its tasks came to — verified green,
 * accepted amber, failed red, running indigo, one segment each — so a failed
 * plan whose first task passed draws that task green and only the failure red.
 * Copied verbatim from plans/portal-programme/08f-final-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanRow } from '@/components/rail/PlanRow';
import { PlanView } from '@/components/stage/PlanView';
import { buildPlanRows, progressSegments } from '@/lib/planRows';
import { initialRunState } from '@/lib/runState';
import type { PlanRun } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore } from '@/test/dom';

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

const PLAN: WirePlanSummary = { id: 'hello', title: 'Hello world', task_count: 2, tasks_done: 0, tasks_failed: 0, completed: false, status: 'pending', old_format: false };

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** hello: T01 passed, T02 failed its structural check. */
const HALF_FAILED: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'implement' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T02', gate: 'verify[0:structural]', passed: false, output_text: '$ test -f MISSING.md\n✗ exit status 1' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T02', outcome: 'failed' },
  { type: 'plan_completed', plan_id: 'hello', success: false },
];

const segs = (root: ParentNode | null) =>
  [...(root?.querySelectorAll<HTMLElement>('[data-segment]') ?? [])].map(
    (s) => `${s.dataset.segment} ${Number.parseFloat(s.style.width).toFixed(1)}%`,
  );

function rowFor(live: Partial<PlanRun> | null, disk: Partial<WirePlanSummary> = {}) {
  const run = initialRunState();
  const plans: Record<string, PlanRun> = live
    ? {
        hello: {
          planId: 'hello', title: null, phase: 'running', tasksTotal: 10, tasksDone: 0, tasksFailed: 0, tasksAccepted: 0,
          startedAtMs: 1_000, finishedAtMs: null, etaMinutes: null, costUsd: 0, ...live,
        },
      }
    : {};
  return buildPlanRows([{ ...PLAN, task_count: 10, ...disk }], { ...run, plans }, { filter: '', nowMs: 10_000 }).groups[0]!.rows[0]!;
}

describe('progress segments', () => {
  it('split progress into verified, accepted, failed and running parts, leaving empty parts out', () => {
    expect(progressSegments({ done: 1, accepted: 0, failed: 1, active: 0 }, 2)).toEqual([
      { state: 'done', share: 0.5 },
      { state: 'failed', share: 0.5 },
    ]);
    expect(progressSegments({ done: 2, accepted: 1, failed: 0, active: 1 }, 8)).toEqual([
      { state: 'done', share: 0.25 },
      { state: 'accepted', share: 0.125 },
      { state: 'active', share: 0.125 },
    ]);
    expect(progressSegments({ done: 0, accepted: 0, failed: 0, active: 0 }, 0)).toEqual([]);
  });

  it('never add up to more than the whole bar', () => {
    const shares = progressSegments({ done: 3, accepted: 0, failed: 2, active: 1 }, 4).map((s) => s.share);
    expect(shares.reduce((a, b) => a + b, 0)).toBeCloseTo(1);
    expect(shares).toEqual([0.75, 0.25]);
  });

  it('give each rail row its segments', () => {
    expect(rowFor({ phase: 'failed', tasksDone: 3, tasksFailed: 1, finishedAtMs: 5_000 }).segments).toEqual([
      { state: 'done', share: 0.3 },
      { state: 'failed', share: 0.1 },
    ]);
    expect(rowFor({ phase: 'completed', tasksDone: 10, tasksAccepted: 2, finishedAtMs: 5_000 }).segments).toEqual([
      { state: 'done', share: 0.8 },
      { state: 'accepted', share: 0.2 },
    ]);
    expect(rowFor(null, { tasks_done: 4, tasks_failed: 1 }).segments).toEqual([
      { state: 'done', share: 0.4 },
      { state: 'failed', share: 0.1 },
    ]);
  });
});

describe('bars', () => {
  it('draw a half-failed plan green then red in the rail', () => {
    const run = foldEvents(HALF_FAILED);
    const row = buildPlanRows([PLAN], run, { filter: '', nowMs: 10_000 }).groups[0]!.rows[0]!;
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    expect(segs(container.querySelector('.rd-plan-row__bar'))).toEqual(['done 50.0%', 'failed 50.0%']);
  });

  it('draw the same segments in the plan header', () => {
    setStore(foldEvents(HALF_FAILED));
    renderWithClient(<PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />, {
      seed: [
        [queryKeys.planTasks('hello'), TASKS],
        [queryKeys.validation('hello'), VALID],
      ],
    });
    const bar = document.querySelector('[role="progressbar"]');
    expect(segs(bar)).toEqual(['done 50.0%', 'failed 50.0%']);
    expect(bar!.getAttribute('aria-label')).toBe('1 of 2 tasks complete');
  });
});
