// @vitest-environment jsdom
/**
 * Acceptance for gap-f8c57a: while roko writes a plan, the glass shows SPENT against the write's cap as its calls
 * settle, and a stop key stops the write. The box comes back with the request in it and says what the write spent.
 * A write that had already written its plan shows the plan, and an older roko serve that can't stop a write says so,
 * with no key. A plan being written has a row at the top of the plan list, which opens its writing view.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanList } from '@/components/plans/PlanList';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { Stage } from '@/components/stage/Stage';
import { NO_STOP } from '@/components/stage/Writing';
import { WriteStopped, waitForOperation } from '@/lib/operation';
import { buildPlanRows } from '@/lib/planRows';
import { initialRunState } from '@/lib/runState';
import { isWriteEvent, splitWrites } from '@/lib/writing';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

const WORKSPACE = [queryKeys.workspace, { name: 'greeting-app', path: '/work/greeting-app', branch: 'main' }] as const;
const SLUG = 'give-greeting-py-a-farewell';
const REQUEST = 'Give greeting.py a farewell(name) function';
const ACCEPTED = { id: 'op-1', plan_id: SLUG, max_usd: 2, max_usd_source: 'config' };
const PROGRESS = { stage: 'planning', attempt: 1, attempts: 1, model: 'gpt-5.4', calls: 1, cost_usd: 0.0375, max_usd: 2 };
const RUNNING = { id: 'op-1', kind: `plan_generate:${SLUG}`, status: 'running', elapsed_secs: 3, progress: PROGRESS };
const CANCELLED = { cancelled: true, run_id: 'op-1', plan_id: SLUG, state: 'cancelled', cost_usd: 0.0375 };
const LISTED: WirePlanSummary[] = [
  { id: 'hello', title: 'Hello', task_count: 1, tasks_done: 0, tasks_failed: 0, completed: false, status: 'ready', old_format: false },
  {
    id: SLUG,
    title: REQUEST,
    task_count: 0,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'writing',
    old_format: false,
    writing: { run_id: 'op-1', stage: 'planning', elapsed_secs: 12, calls: 1, cost_usd: 0.0375, max_usd: 2 },
  },
];

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const ask = () => {
  fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: REQUEST } });
  fireEvent.click(screen.getByRole('button', { name: /write plan/ }));
};
const label = (reading: string) =>
  document.querySelector(`[data-reading="${reading}"] svg`)?.getAttribute('aria-label') ?? '';
/** The stop key, once the write's id is in: before that it is off. */
const stopKey = () =>
  waitFor(() => {
    const key = document.querySelector<HTMLButtonElement>('[data-action="stop-write"]');
    expect(key?.disabled).toBe(false);
    return key!;
  });
const renderBox = (onDone = vi.fn()) =>
  renderWithClient(<PromptPanel mode="generate" workspace="greeting-app" firstRun={false} onDone={onDone} />, {
    seed: [WORKSPACE],
  });

describe('writing a plan', () => {
  it('shows what the write has spent against its cap, as its calls settle', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: ACCEPTED },
      { path: '/api/operations/op-1', body: RUNNING },
    ]);
    renderBox();
    ask();
    // Until roko serve answers, the cap isn't known, and the glass doesn't guess.
    expect(label('spent')).toBe('spent: $0.00 spent');
    expect(textOf(document.querySelector('[data-reading="spent"] .rk-cell-note'))).toBe('');
    await waitFor(() => expect(label('spent')).toBe('spent: $0.04 of $2.00'), { timeout: 3_000 });
    expect(textOf(document.querySelector('[data-reading="spent"] .rk-cell-note'))).toBe('of $2.00');
    expect(label('time')).toMatch(/^time: /);
  });

  it('stop ends the write: the box comes back with the request in it, and says what the write spent', async () => {
    const fetch = stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: ACCEPTED },
      { path: '/api/operations/op-1', body: RUNNING },
      { method: 'POST', path: '/api/plans/op-1/cancel', body: CANCELLED },
    ]);
    const onDone = vi.fn();
    renderBox(onDone);
    ask();
    fireEvent.click(await stopKey());
    await waitFor(() => expect(textOf(document.querySelector('[data-stopped]'))).toBe('Stopped. Writing the plan spent $0.04.'));
    expect((document.querySelector('[data-prompt]') as HTMLTextAreaElement).value).toBe(REQUEST);
    expect(document.querySelector('[data-region="writing"]')).toBeNull();
    expect(onDone).not.toHaveBeenCalled();
    const cancels = fetch.mock.calls.filter(
      ([url, init]) => String(url).endsWith('/api/plans/op-1/cancel') && init?.method === 'POST',
    );
    expect(cancels).toHaveLength(1);
  });

  it('a write that had already written its plan shows the plan', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: ACCEPTED },
      { path: '/api/operations/op-1', body: { id: 'op-1', status: 'completed', result: { slug: SLUG, task_count: 2 } } },
      {
        method: 'POST',
        path: '/api/plans/op-1/cancel',
        status: 409,
        body: { code: 'conflict', message: `the write of plan '${SLUG}' had already written it` },
      },
    ]);
    const onDone = vi.fn();
    renderBox(onDone);
    ask();
    fireEvent.click(await stopKey());
    await waitFor(() => expect(onDone).toHaveBeenCalledWith(SLUG), { timeout: 3_000 });
    expect(document.querySelector('[data-stopped]')).toBeNull();
    expect(document.querySelector('[data-error]')).toBeNull();
  });

  it('an older roko serve that can’t stop a write says so, with no stop key', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/generate', status: 202, body: ACCEPTED },
      { path: '/api/operations/op-1', body: RUNNING },
      {
        method: 'POST',
        path: '/api/plans/op-1/cancel',
        status: 404,
        body: { code: 'not_found', message: 'no active execution for this plan' },
      },
    ]);
    renderBox();
    ask();
    fireEvent.click(await stopKey());
    await waitFor(() => expect(textOf(document.querySelector('[data-region="writing"] [data-notice]'))).toContain(NO_STOP));
    expect(document.querySelector('[data-action="stop-write"]')).toBeNull();
    expect(document.querySelector('[data-region="writing"]')).not.toBeNull();
  });

  it('a stopped write ends the wait with what it spent', async () => {
    const stopped = waitForOperation(
      ACCEPTED,
      {
        planExists: async () => false,
        fetchOperation: async () => ({ ...RUNNING, status: 'cancelled' }),
        sleep: async () => {},
        now: () => 0,
      },
      { expect: 'new-plan' },
    );
    await expect(stopped).rejects.toBeInstanceOf(WriteStopped);
    await expect(stopped).rejects.toMatchObject({ costUsd: 0.0375 });
  });
});

describe('a plan being written', () => {
  it('has a row at the top of the plan list, which the count leaves out', () => {
    const { plans, writes } = splitWrites(LISTED);
    const onSelect = vi.fn();
    renderWithClient(
      <PlanList
        result={buildPlanRows(plans!, initialRunState(), { filter: '', nowMs: 0 })}
        writes={writes}
        costs={{}}
        selectedPlanId={null}
        filter=""
        onFilterChange={() => {}}
        filterInputRef={{ current: null }}
        onSelect={onSelect}
        onNewPlan={() => {}}
        onRunPlans={() => {}}
        emptySentence="No plans yet."
      />,
    );
    const rows = [...document.querySelectorAll('[data-plan-row]')].map((row) => row.getAttribute('data-plan-row'));
    expect(rows).toEqual([SLUG, 'hello']);
    const row = document.querySelector(`[data-plan-row="${SLUG}"]`)!;
    expect(row.getAttribute('data-state')).toBe('writing');
    expect(textOf(row.querySelector('.pt-plan-title'))).toBe(REQUEST);
    expect(textOf(row.querySelector('.pt-plan-line'))).toBe('writing the plan · $0.04');
    expect(textOf(document.querySelector('.pt-plans-head .rk-label'))).toBe('plans · 1');
    fireEvent.click(row);
    expect(onSelect).toHaveBeenCalledWith(SLUG);
  });

  it('opens its writing view, whose stop gives the request back to the box', async () => {
    stubFetch([
      { path: '/api/operations/op-1', body: RUNNING },
      { method: 'POST', path: '/api/plans/op-1/cancel', body: CANCELLED },
    ]);
    const { plans, writes } = splitWrites(LISTED);
    const onReopenPrompt = vi.fn();
    renderWithClient(
      <Stage
        plans={plans}
        writes={writes}
        loaded
        selection={{ plan: SLUG, task: null }}
        promptOpen={false}
        workspace="greeting-app"
        onSelect={() => {}}
        onClosePrompt={() => {}}
        onReopenPrompt={onReopenPrompt}
        onRequestError={() => {}}
      />,
      { seed: [WORKSPACE] },
    );
    const writing = document.querySelector('[data-region="writing"]')!;
    expect(textOf(writing.querySelector('[data-request]'))).toBe(`“${REQUEST}”`);
    expect(label('time')).toBe('time: 12 seconds');
    expect(label('spent')).toBe('spent: $0.00 of $2.00');
    fireEvent.click(await stopKey());
    await waitFor(() => expect(onReopenPrompt).toHaveBeenCalledTimes(1));
    expect(onReopenPrompt.mock.calls[0]![0]).toMatchObject({
      prompt: REQUEST,
      note: 'Stopped. Writing the plan spent $0.04.',
    });
  });

  it('a write that starts or ends is what refreshes the plan list', () => {
    const entry = (event_type: string): WireDashboardEvent => ({ type: 'event_log_entry', event_type, plan_id: SLUG });
    expect(isWriteEvent(entry('plan_generate.started'))).toBe(true);
    expect(isWriteEvent(entry('plan_generate.cancelled'))).toBe(true);
    expect(isWriteEvent(entry('plan_revise.completed'))).toBe(true);
    expect(isWriteEvent(entry('plan_run.started'))).toBe(false);
    expect(isWriteEvent({ type: 'plan_completed', plan_id: SLUG, success: true } as WireDashboardEvent)).toBe(false);
  });
});
