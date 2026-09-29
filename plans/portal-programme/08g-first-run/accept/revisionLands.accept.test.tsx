// @vitest-environment jsdom
/**
 * Acceptance: a revision's tasks show as soon as the revision lands. The
 * revise request answers 202 at once and the plan changes only when its
 * operation completes, so refreshing on the 202 alone left the old tasks on
 * screen (found by 09's browser smoke on the merged tree).
 * Copied verbatim from plans/portal-programme/08g-first-run/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanView } from '@/components/stage/PlanView';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore } from '@/test/dom';

const PLAN: WirePlanSummary = { id: 'hello', title: 'Hello world', task_count: 1, tasks_done: 0, tasks_failed: 0, completed: false, status: 'ready', old_format: false };

const T01 = { id: 'T01', title: 'Write the hello world program', tier: 'mechanical', status: 'pending', depends_on: [], files: ['hello/main.rs'], completed: false, verify_phases: ['build'] };
const T99 = { id: 'T99', title: 'Added by revision', tier: 'focused', status: 'pending', depends_on: [], files: ['out/revised.txt'], completed: false, verify_phases: ['structural'] };
const BEFORE: WirePlanTasks = { plan_id: 'hello', task_count: 1, max_parallel: 1, tasks: [T01] };
const AFTER: WirePlanTasks = { plan_id: 'hello', task_count: 2, max_parallel: 1, tasks: [T01, T99] };
const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** The plan changes only once the revision's operation completes; reads take a moment. */
function stubServer() {
  let revised = false;
  const json = (body: unknown, status = 200) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url, 'http://localhost');
      const route = `${(init?.method ?? 'GET').toUpperCase()} ${url.pathname}`;
      switch (route) {
        case 'GET /api/plans/hello/tasks':
          await new Promise((resolve) => setTimeout(resolve, 100));
          return json(revised ? AFTER : BEFORE);
        case 'POST /api/plans/hello/revise':
          return json({ id: 'op-r', plan_id: 'hello' }, 202);
        case 'GET /api/operations/op-r':
          revised = true;
          return json({ id: 'op-r', kind: 'plan_revise:hello', status: 'completed', result: { slug: 'hello', task_count: 2 } });
        case 'POST /api/plans/hello/validate':
          return json(VALID);
        case 'GET /api/plans':
          return json([PLAN]);
        default:
          throw new TypeError(`unexpected request: ${route}`);
      }
    }),
  );
}

beforeEach(() => setStore(initialRunState()));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('a revision', () => {
  it('shows its new task once it lands, and closes the prompt', async () => {
    stubServer();
    renderWithClient(<PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />, {
      seed: [
        [queryKeys.planTasks('hello'), BEFORE],
        [queryKeys.validation('hello'), VALID],
      ],
    });
    expect(document.querySelector('[data-task-row="T01"]')).not.toBeNull();
    fireEvent.click(document.querySelector('[data-action="revise"]')!);
    fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: 'add a closing task' } });
    fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);

    await waitFor(() => expect(document.querySelector('[data-task-row="T99"]')).not.toBeNull(), { timeout: 4_000 });
    expect(document.querySelector('[data-prompt]')).toBeNull();
  });
});
