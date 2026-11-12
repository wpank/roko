// @vitest-environment jsdom
/**
 * Acceptance for gap-6c4d24: writing and fixing a plan speak in a person's words. While roko writes, the request
 * stays on screen with the time on glass. A failed write is one sentence, with what roko serve said folded under it
 * and the project's paths relative, and the notice band doesn't repeat it. A plan that can't run says so, one line
 * per problem, each on its task, with edit plan as the orange key; the checker's rule ids fold away. Esc closes the
 * open box before anything else.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { pickAlert } from '@/lib/alerts';
import { planProblems } from '@/lib/planProblems';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const WORKSPACE = [queryKeys.workspace, { name: 'greeting-app', path: '/work/greeting-app', branch: 'main' }] as const;
const SLUG = 'give-greeting-py-a-farewell';
const NO_PLAN =
  `plan generation failed for ${SLUG}: Plan generation failed after retries: no valid tasks.toml was produced. ` +
  'Last error: no TOML block found in agent output The agent output (2 bytes) did not contain a parseable ```toml ' +
  `block. hint: Try again, or create plans/${SLUG}/tasks.toml manually. The rejected drafts are in ` +
  `/work/greeting-app/.roko/plan-drafts/${SLUG}.`;

/** The draft plan as roko's checker reports it: one missing check, said twice. */
const DRAFT: WireValidation = {
  valid: false,
  errors: [],
  warnings: [],
  diagnostics: [
    { severity: 'error', rule_id: 'PLAN_035', message: "schema validation failed: T2: missing 'verify' (required for role 'implementer')" },
    {
      severity: 'error',
      rule_id: 'PLAN_037',
      task_id: 'T2',
      message:
        "task 'T2' has no verify steps: it can only end unverified, and then its plan does not succeed; give it a verify step, or set [meta] allow_unverified = true",
    },
  ],
};
const PLAN = planSummary('draft', 'Add a farewell', 2);
const TASKS = planTasks('draft', [
  ['T1', 'Add a farewell function', []],
  ['T2', 'Mention farewell in the README', ['T1']],
]);

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const ask = async (request: string) => {
  fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: request } });
  fireEvent.click(screen.getByRole('button', { name: /write plan/ }));
};

describe('writing a plan', () => {
  it('keeps the request on screen while roko writes, with the time on glass', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: { id: 'op-1', plan_id: SLUG } },
      { path: '/api/operations/op-1', body: { id: 'op-1', status: 'running' } },
    ]);
    renderWithClient(<PromptPanel mode="generate" workspace="greeting-app" firstRun={false} onDone={() => {}} />, {
      seed: [WORKSPACE],
    });
    await ask('Give greeting.py a farewell(name) function');
    const writing = await waitFor(() => document.querySelector('[data-region="writing"]')!);
    expect(textOf(writing.querySelector('[data-request]'))).toBe('“Give greeting.py a farewell(name) function”');
    expect(writing.querySelector('[data-reading="time"]')).not.toBeNull();
    expect(textOf(writing.querySelector('[data-waiting]'))).toContain('You’ll see the plan here before anything runs.');
    expect(document.querySelector('[data-prompt]')).toBeNull();
  });

  it('a generate failure is one sentence', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: { id: 'op-1', plan_id: SLUG } },
      { path: '/api/operations/op-1', body: { id: 'op-1', status: 'failed', error: NO_PLAN } },
    ]);
    renderWithClient(<PromptPanel mode="generate" workspace="greeting-app" firstRun={false} onDone={() => {}} />, {
      seed: [WORKSPACE],
    });
    await ask('Give greeting.py a farewell(name) function');
    const error = await screen.findByText(/the model’s answer had no plan in it/, {}, { timeout: 4_000 });
    expect(textOf(error)).toBe(
      'roko couldn’t write a plan for this request: the model’s answer had no plan in it. Try again, or say more about what you want.',
    );
    const said = textOf(document.querySelector('[data-fold="write-error"]'));
    expect(said).toContain(`The rejected drafts are in .roko/plan-drafts/${SLUG}.`);
    expect(said).not.toContain('/work/greeting-app');
    expect(said).not.toContain('plan generation failed for');
    // The request is still there to change and send again.
    expect((document.querySelector('[data-prompt]') as HTMLTextAreaElement).value).toBe('Give greeting.py a farewell(name) function');
  });

  it('is not repeated in the notice band', () => {
    const run = { ...initialRunState(), errors: [{ message: NO_PLAN, atMs: 1 }] };
    expect(
      pickAlert({ run, connection: 'connected', selectedPlanId: null, requestError: null, dismissedKey: null }),
    ).toBeNull();
  });
});

describe('a plan that can’t run', () => {
  it('one problem, one line', () => {
    const problems = planProblems(DRAFT.diagnostics, ['T1', 'T2']);
    expect(problems.map((p) => [p.taskId, p.text])).toEqual([
      ['T2', 'Task 2 has no check, so roko could never tell when it’s done. Give it a check: a command that fails until the task is done.'],
    ]);
    expect(problems[0]!.raw).toHaveLength(2);
  });

  it('says why on its page, outlines the task, and makes edit plan the orange key', async () => {
    const onSelectTask = vi.fn();
    renderPlanPage(PLAN, TASKS, [], { seed: [WORKSPACE, [queryKeys.validation('draft'), DRAFT]], onSelectTask });
    expect(textOf(document.querySelector('[data-region="plan"] .rk-pill'))).toBe('needs you');
    expect(textOf(document.querySelector('[data-region="run-summary"]'))).toBe(
      'This plan can’t run yet. Fix the problem below by editing the plan, or ask roko to revise it.',
    );
    const lines = [...document.querySelectorAll('[data-problem-line]')];
    expect(lines).toHaveLength(1);
    expect(textOf(lines[0]!)).toContain('Task 2 has no check, so roko could never tell when it’s done.');
    expect(textOf(lines[0]!)).not.toContain('PLAN_0');
    expect(textOf(document.querySelector('[data-fold="problems"]'))).toContain('PLAN_037');

    expect(document.querySelector('[data-pad="T2"]')!.getAttribute('data-problem')).toBe('no check');
    expect(textOf(document.querySelector('[data-pad="T2"]'))).toContain('no check');
    fireEvent.click(lines[0]!.querySelector('[data-action="show-problem"]')!);
    expect(onSelectTask).toHaveBeenCalledWith('T2');

    const orange = [...document.querySelectorAll('[data-region="status-line"] [data-you]')].map((k) => k.getAttribute('data-action'));
    expect(orange).toEqual(['edit-plan']);
    expect(document.querySelector('[data-action="run"]')).toBeNull();
    expect(document.querySelector('[data-reading]')).toBeNull();
  });
});

describe('Esc', () => {
  it('closes the open box before anything else', async () => {
    const elsewhere = vi.fn();
    window.addEventListener('keydown', elsewhere);
    const onCancel = vi.fn();
    renderWithClient(
      <PromptPanel mode="revise" planId="draft" workspace="Add a farewell" firstRun={false} onDone={() => {}} onCancel={onCancel} />,
      { seed: [WORKSPACE] },
    );
    fireEvent.keyDown(document.querySelector('[data-prompt]')!, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(elsewhere).not.toHaveBeenCalled();
    window.removeEventListener('keydown', elsewhere);
  });
});
