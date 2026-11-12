// @vitest-environment jsdom
/**
 * Acceptance for gap-6d2dd2: after a run the page says where the work went. A passed run's work sits on its own
 * branch, and the page says what it changed, gives the command that takes it with a copy key, and makes review the
 * orange key, which opens every changed file with its patch. A failed run names its check's command and the line of
 * its output that says why. After a reload the plan's detail names its last run, so none of this is lost.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WireRunResult } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { initialRunState } from '@/lib/runState';
import { useDashboardStore } from '@/stores/dashboard';
import { setStore, stubFetch, textOf } from '@/test/dom';
import type { FetchRoute } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const RUN = '69ef0a50-6238-41d9-8a32-67105107c628';
const PLAN = planSummary('greeting', 'Say hello from the command line', 1);
const TASKS = planTasks('greeting', [['T1', 'Document how to run it', []]]);
const WORKSPACE = [queryKeys.workspace, { name: 'greeting-app', path: '/work/greeting-app', branch: 'main' }] as const;

const ran = (outcome: 'passed' | 'failed', output = ''): WireDashboardEvent[] => [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'greeting', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] }] },
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 1 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'greeting/T1', plan_id: 'greeting', task_id: 'T1', attempt: 1, role: 'implementer', model: 'glm-4.7' },
  { type: 'gate_result', plan_id: 'greeting', task_id: 'T1', gate: 'verify[0:build]', passed: outcome === 'passed', output_text: output },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T1', outcome },
  { type: 'plan_completed', plan_id: 'greeting', success: outcome === 'passed' },
  { type: 'run_completed', outcome: outcome === 'passed' ? 'succeeded' : 'failed', duration_ms: 20_000 },
];

const RESULT: WireRunResult = {
  branch: `roko/batch/${RUN}`,
  commit: '2f1d0c3',
  diffstat: { files: 3, insertions: 14, deletions: 0 },
  files_changed: ['README.md', 'cli.py', 'greeting.py'],
  merged: false,
  merge_command: `git merge --ff-only roko/batch/${RUN}`,
};

const README_PATCH = '@@ -0,0 +1,3 @@\n+# greeting\n+\n+Usage: python3 cli.py <name>\n';

const routes = (result: WireRunResult | null, lastRun = true): FetchRoute[] => [
  {
    path: '/api/plans/greeting',
    body: {
      ...PLAN,
      resumable_run: null,
      last_run: lastRun
        ? { run_id: RUN, status: 'succeeded', passed_tasks: ['T1'], tasks_left: [], resumable: false, continues: true }
        : null,
    },
  },
  { path: `/api/runs/${RUN}/summary`, body: { result } },
  {
    path: `/api/runs/${RUN}/diff`,
    body: {
      run_id: RUN,
      branch: `roko/batch/${RUN}`,
      files: [{ path: 'README.md', status: 'added', additions: 3, deletions: 0, binary: false, patch: README_PATCH }],
      totals: { files: 1, additions: 3, deletions: 0 },
      truncated: false,
    },
  },
  {
    path: '/api/plans/greeting/tasks/T1/diff',
    body: {
      task_id: 'T1',
      file_count: 1,
      total_additions: 3,
      total_deletions: 0,
      files: [{ path: 'README.md', status: 'added', additions: 3, deletions: 0, patch: README_PATCH }],
    },
  },
];

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const summaryText = () => textOf(document.querySelector('[data-region="run-summary"]'));
const orange = () => [...document.querySelectorAll('[data-region="status-line"] [data-you]')].map((k) => k.getAttribute('data-action'));

describe('a passed run', () => {
  it('a passed run says where its work is', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, TASKS, ran('passed'), { seed: [WORKSPACE] });
    await waitFor(() => expect(summaryText()).toContain('Ready to merge:'));
    expect(summaryText()).toBe('✓ Ready to merge: 3 files changed, +14 lines, on its own branch. Your checkout hasn’t changed.');
    expect(textOf(document.querySelector('[data-merge-command]'))).toBe(`git merge --ff-only roko/batch/${RUN}`);
    expect(document.querySelector('[data-action="copy-merge"]')).not.toBeNull();
    expect(orange()).toEqual(['review-run']);
    expect(document.querySelector('[data-action="run-again"]')).not.toBeNull();
  });

  it('opens every file the run changed from the review key, each with its patch folded', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, TASKS, ran('passed'), { seed: [WORKSPACE] });
    fireEvent.click(await screen.findByRole('button', { name: 'review' }));
    const changes = await waitFor(() => {
      const region = document.querySelector('[data-region="changes"]');
      expect(region?.querySelector('[data-file="README.md"]')).toBeTruthy();
      return region!;
    });
    expect(textOf(changes)).toContain('README.md +3 new file');
    expect([...changes.querySelectorAll('[data-patch] [data-line="add"]')].map((l) => textOf(l).trim())).toEqual([
      '+# greeting',
      '+',
      '+Usage: python3 cli.py <name>',
    ]);
  });

  it('says a run whose work is in the checkout already was merged, with no command to copy', async () => {
    stubFetch(routes({ ...RESULT, merged: true, merge_command: undefined }));
    renderPlanPage(PLAN, TASKS, ran('passed'), { seed: [WORKSPACE] });
    await waitFor(() => expect(summaryText()).toBe('✓ Merged into main.'));
    expect(document.querySelector('[data-region="merge"]')).toBeNull();
    expect(orange()).toEqual([]);
  });

  it('shows what a task that passed changed', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, TASKS, ran('passed'), { seed: [WORKSPACE], selected: 'T1' });
    await waitFor(() => expect(textOf(document.querySelector('[data-region="task-detail"]'))).toContain('changedREADME.md +3 new file'));
    expect(document.querySelector('[data-region="task-detail"] [data-fold="change"]')).not.toBeNull();
  });

  it('reads the run’s summary again when the run ends: its work is on record only then', async () => {
    let runEnded = false;
    const answer = (body: unknown) =>
      new Response(JSON.stringify(body), { status: 200, headers: { 'content-type': 'application/json' } });
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const path = new URL(String(input), 'http://localhost').pathname;
      if (path === '/api/plans/greeting') return answer(routes(RESULT)[0]!.body);
      if (path === `/api/runs/${RUN}/summary`) return answer({ result: runEnded ? RESULT : null });
      throw new TypeError(`unexpected request: ${path}`);
    });
    vi.stubGlobal('fetch', fetchMock);
    renderPlanPage(PLAN, TASKS, ran('passed').slice(0, -1), { seed: [WORKSPACE] });
    await waitFor(() => expect(fetchMock.mock.calls.some(([input]) => String(input).endsWith('/summary'))).toBe(true));
    expect(summaryText()).toBe('The task passed its checks.');

    runEnded = true;
    act(() => {
      useDashboardStore.getState().applyEvent({ type: 'run_completed', outcome: 'succeeded', duration_ms: 20_000 });
    });
    await waitFor(() => expect(summaryText()).toContain('Ready to merge: 3 files changed'));
  });

  it('finds the last run after a reload, from the plan’s detail', async () => {
    stubFetch(routes(RESULT));
    renderPlanPage(PLAN, TASKS, [], { seed: [WORKSPACE] });
    await waitFor(() => expect(summaryText()).toContain('Its last run passed. ✓ Ready to merge: 3 files changed'));
    expect(orange()).toEqual(['review-run']);
  });
});

describe('a failed run', () => {
  it('a failed run names the check’s real error line', async () => {
    stubFetch(routes(null));
    const output = [
      '$ make t1',
      '---stderr---',
      'Traceback (most recent call last):',
      '  File "<string>", line 1, in <module>',
      '  File "/work/greeting-app/.roko/worktrees/attempt-4f1c/settings.py", line 2',
      '    return dict(',
      "SyntaxError: '(' was never closed",
      '✗ exit status 1',
    ].join('\n');
    renderPlanPage(PLAN, TASKS, ran('failed', output), { seed: [WORKSPACE] });
    expect(summaryText()).toBe("Task 1’s check failed: make t1 stopped at settings.py, line 2: '(' was never closed.");
    expect(textOf(document.querySelector('[data-region="run-summary"] code'))).toBe('make t1');
    await waitFor(() => expect(orange()).toEqual(['run-again']));
    expect(textOf(document.querySelector('[data-action="run-again"]'))).toContain('run again');
  });
});
