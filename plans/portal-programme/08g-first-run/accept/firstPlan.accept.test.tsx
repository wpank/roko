// @vitest-environment jsdom
/**
 * Acceptance: the first plan of a workspace with none opens as soon as it is
 * generated. The workspace clears a selected plan id its loaded list does not
 * know, and the list it had loaded was empty — so selecting the new plan before
 * the refreshed list arrived lost the selection, and the operator was left at
 * "Select a plan" (found by 09's browser smoke on the merged tree).
 * Copied verbatim from plans/portal-programme/08g-first-run/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, textOf } from '@/test/dom';

// As in Next 15, replaceState re-renders every reader of the search params.
vi.mock('next/navigation', async () => {
  const React = await import('react');
  const listeners = new Set<() => void>();
  const replaceState = window.history.replaceState.bind(window.history);
  window.history.replaceState = (...args: Parameters<History['replaceState']>) => {
    replaceState(...args);
    listeners.forEach((notify) => notify());
  };
  const subscribe = (notify: () => void) => {
    listeners.add(notify);
    return () => listeners.delete(notify);
  };
  return {
    useSearchParams: () => {
      const search = React.useSyncExternalStore(subscribe, () => window.location.search);
      return React.useMemo(() => new URLSearchParams(search), [search]);
    },
  };
});

const PLAN: WirePlanSummary = { id: 'hello', title: 'Hello world', task_count: 1, tasks_done: 0, tasks_failed: 0, completed: false, status: 'ready', old_format: false };

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 1,
  max_parallel: 1,
  tasks: [{ id: 'T01', title: 'Write the hello world program', tier: 'mechanical', status: 'pending', depends_on: [], files: ['hello/main.rs'], completed: false, verify_phases: ['build'] }],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/**
 * A server with no plans until generation finishes: the plan exists once its
 * operation reports completed, and listing takes a moment, as over a network.
 */
function stubServer() {
  let generated = false;
  const json = (body: unknown, status = 200) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url, 'http://localhost');
    const route = `${(init?.method ?? 'GET').toUpperCase()} ${url.pathname}`;
    switch (route) {
      case 'GET /api/plans':
        await new Promise((resolve) => setTimeout(resolve, 150));
        return json(generated ? [PLAN] : []);
      case 'POST /api/plans/generate':
        return json({ id: 'op-1', plan_id: 'hello' }, 202);
      case 'GET /api/operations/op-1':
        generated = true;
        return json({ id: 'op-1', kind: 'plan_generate:hello', status: 'completed', result: { slug: 'hello', task_count: 1 } });
      case 'GET /api/plans/hello':
        return generated ? json(PLAN) : json({ error: 'not_found' }, 404);
      case 'GET /api/plans/hello/tasks':
        return json(TASKS);
      case 'POST /api/plans/hello/validate':
        return json(VALID);
      default:
        throw new TypeError(`unexpected request: ${route}`);
    }
  });
  vi.stubGlobal('fetch', fetchMock);
}

beforeEach(() => {
  window.history.replaceState(null, '', '/');
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('the first plan', () => {
  it('opens as soon as it is generated', async () => {
    stubServer();
    renderWithClient(<Workspace />, {
      seed: [
        [queryKeys.plans, []],
        [queryKeys.workspace, { name: 'hello-ws', path: '/tmp/hello-ws', branch: 'main' }],
      ],
    });
    const prompt = document.querySelector('[data-prompt]')!;
    expect(prompt).not.toBeNull();
    fireEvent.change(prompt, { target: { value: 'a rust app that prints hello world' } });
    fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);

    await waitFor(() => expect(document.querySelector('[data-plan-row="hello"][aria-current="true"]')).not.toBeNull(), {
      timeout: 4_000,
    });
    expect(new URLSearchParams(window.location.search).get('plan')).toBe('hello');
    await waitFor(() => expect(document.querySelector('[data-action="run"]')).not.toBeNull());
    expect(textOf(document.querySelector('[data-region="stage"]'))).not.toContain('Select a plan');
  });
});
