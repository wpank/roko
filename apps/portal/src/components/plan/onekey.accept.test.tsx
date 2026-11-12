// @vitest-environment jsdom
/**
 * Acceptance for gap-a5b2e8: the portal follows the design system's one-key rule for a run (§4.8). A run that ran to
 * its end has one key, the orange review, and the plan's own keys (revise, run again) wait in its details. A merged
 * run says how many files went in, with no key. A run that wasn't fully checked, or holds only part of its work,
 * still gets review, with the reason in its hover and as a banner in the changes. A plan revised after a pass, with
 * tasks its run never ran, reads ready with an orange run again.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WireRunResult } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { initialRunState } from '@/lib/runState';
import { setStore, stubFetch, textOf } from '@/test/dom';
import type { FetchRoute } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const RUN = '69ef0a50-6238-41d9-8a32-67105107c628';
const PLAN = planSummary('greeting', 'Say hello from the command line', 1);
const ONE_TASK = planTasks('greeting', [['T1', 'Write the greeting module', []]]);
const WORKSPACE = [queryKeys.workspace, { name: 'greeting-app', path: '/work/greeting-app', branch: 'main' }] as const;

/** A one-task run that ended with T1 at `outcome` and the plan at `success`. */
const ran = (outcome: string, success = true): WireDashboardEvent[] => [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'greeting', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] }] },
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 1 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
  { type: 'gate_result', plan_id: 'greeting', task_id: 'T1', gate: 'verify[0:build]', passed: outcome === 'passed' },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T1', outcome },
  { type: 'plan_completed', plan_id: 'greeting', success },
  { type: 'run_completed', outcome: success ? 'succeeded' : 'failed', duration_ms: 20_000 },
];

const RESULT: WireRunResult = {
  branch: 'roko/work/greeting',
  commit: '2f1d0c3',
  diffstat: { files: 3, insertions: 14, deletions: 0 },
  merged: false,
  merge_command: 'git merge --ff-only roko/work/greeting',
};

const routes = (result: WireRunResult | null, status = 'succeeded', resumable = false): FetchRoute[] => {
  const last = { run_id: RUN, status, passed_tasks: ['T1'], tasks_left: [], resumable, continues: true };
  return [
    { path: '/api/plans/greeting', body: { ...PLAN, last_run: last, resumable_run: resumable ? last : null } },
    { path: `/api/runs/${RUN}/summary`, body: { result } },
    {
      path: `/api/runs/${RUN}/diff`,
      body: { run_id: RUN, branch: 'roko/work/greeting', files: [], totals: { files: 3, additions: 14, deletions: 0 }, truncated: false },
    },
  ];
};

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const statusKeys = () =>
  [...document.querySelectorAll('[data-region="status-line"] button')].map((k) => k.getAttribute('data-action'));
const orange = () =>
  [...document.querySelectorAll('[data-region="status-line"] [data-you]')].map((k) => k.getAttribute('data-action'));
const summary = () => textOf(document.querySelector('[data-region="run-summary"]'));
const reviewKey = () => document.querySelector('[data-region="status-line"] [data-action="review-run"]');

describe('a run that ran to its end', () => {
  it('has one key, the orange review, and keeps the plan’s own keys in its details', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, ONE_TASK, ran('passed'), { seed: [WORKSPACE] });
    await waitFor(() => expect(statusKeys()).toEqual(['review-run']));
    expect(orange()).toEqual(['review-run']);
    const fold = document.querySelector('[data-fold="plan"]')!;
    expect([...fold.querySelectorAll('button')].map((k) => k.getAttribute('data-action'))).toEqual([
      'revise',
      'run-again',
      'edit',
    ]);
    expect(fold.querySelector('[data-you]')).toBeNull();
  });

  it('when merged, says how many files went in, and has no key', async () => {
    stubFetch(routes({ ...RESULT, merged: true, merge_command: undefined }));
    renderPlanPage(PLAN, ONE_TASK, ran('passed'), { seed: [WORKSPACE] });
    await waitFor(() => expect(summary()).toBe('✓ Merged into main: 3 files changed.'));
    expect(statusKeys()).toEqual([]);
  });

  it('when not every task was checked, still offers review, saying why in its hover and in the changes', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, ONE_TASK, ran('unverified'), { seed: [WORKSPACE] });
    await waitFor(() => expect(statusKeys()).toEqual(['review-run']));
    expect(summary()).toContain('finished without a check that judged them');
    const why = 'Some tasks finished without a check that judged them: look at what they changed before you merge.';
    expect(reviewKey()!.getAttribute('title')).toBe(why);
    fireEvent.click(reviewKey()!);
    expect(textOf(await screen.findByText(why, { selector: '[data-banner]' }))).toBe(why);
  });

  it('when only part of its work is on its branch, says so on review', async () => {
    stubFetch(routes({ ...RESULT, partial: true }));
    renderPlanPage(PLAN, ONE_TASK, ran('passed'), { seed: [WORKSPACE] });
    await waitFor(() => expect(reviewKey()).not.toBeNull());
    expect(reviewKey()!.getAttribute('title')).toBe(
      'Only part of the run’s work is on this branch: the tasks that passed their checks.',
    );
    expect(summary()).toContain('Part of the work is ready to merge:');
  });
});

describe('a failed run that delivered what passed', () => {
  it('adds a plain review beside its orange key, and keeps revise in its details', async () => {
    stubFetch(routes({ ...RESULT, partial: true }, 'failed'));
    renderPlanPage(PLAN, ONE_TASK, ran('failed', false), { seed: [WORKSPACE] });
    await waitFor(() => expect(statusKeys()).toEqual(['review-run', 'run-again']));
    expect(orange()).toEqual(['run-again']);
    expect(document.querySelector('[data-fold="plan"] [data-action="revise"]')).not.toBeNull();
  });
});

describe('a plan revised after a pass', () => {
  it('reads ready, says what hasn’t run, and makes run again the orange key', async () => {
    stubFetch(routes(RESULT));
    const twoTasks = planTasks('greeting', [
      ['T1', 'Write the greeting module', []],
      ['T2', 'Say goodbye too', ['T1']],
    ]);
    renderPlanPage(planSummary('greeting', 'Say hello from the command line', 2), twoTasks, ran('passed'), {
      seed: [WORKSPACE],
    });
    expect(textOf(document.querySelector('[data-region="plan"] .rk-pill'))).toBe('ready');
    expect(summary()).toBe('The plan changed since its last run: 1 task hasn’t run.');
    await waitFor(() => expect(orange()).toEqual(['run-again']));
    expect(statusKeys()).toEqual(['revise', 'run-again']);
    expect(textOf(document.querySelector('[data-pad="T2"]'))).toContain('not run');
  });
});
