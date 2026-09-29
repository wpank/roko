// @vitest-environment jsdom
/**
 * Generating a plan waits on its operation, not on the plan: GET /api/plans/{id}
 * answers 404 until the plan is written, and a real run logged one 404 a second
 * (bug-64fb48). The plan is asked for only once the server no longer knows the
 * operation.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, waitFor } from '@testing-library/react';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { renderWithClient } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const RUNNING = { id: 'op-g', kind: 'plan_generate:hello', status: 'running' };
const COMPLETED = { ...RUNNING, status: 'completed', result: { slug: 'hello', task_count: 1 } };

/** Answers each operation poll from `polls` in turn; `null` is a 404 (swept). */
function stubServer(polls: ReadonlyArray<object | null>, planWritten: boolean) {
  let poll = 0;
  const json = (body: unknown, status = 200) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
  const requests: string[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url, 'http://localhost');
      const route = `${(init?.method ?? 'GET').toUpperCase()} ${url.pathname}`;
      requests.push(route);
      switch (route) {
        case 'POST /api/plans/generate':
          return json({ id: 'op-g', plan_id: 'hello' }, 202);
        case 'GET /api/operations/op-g': {
          const op = polls[Math.min(poll++, polls.length - 1)];
          return op ? json(op) : json({ error: 'not_found' }, 404);
        }
        case 'GET /api/plans/hello':
          return planWritten ? json({ id: 'hello' }) : json({ error: 'not_found' }, 404);
        default:
          throw new TypeError(`unexpected request: ${route}`);
      }
    }),
  );
  return requests;
}

function generate(onDone: (slug: string) => void) {
  renderWithClient(<PromptPanel mode="generate" workspace="hello" firstRun={false} onDone={onDone} />);
  fireEvent.change(document.querySelector('[data-prompt]')!, { target: { value: 'a rust app that prints hello world' } });
  fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);
}

describe('PromptPanel generating a plan', () => {
  it('waits for the operation to complete and never asks for the unwritten plan', async () => {
    const requests = stubServer([RUNNING, COMPLETED], false);
    const onDone = vi.fn();
    generate(onDone);

    await waitFor(() => expect(onDone).toHaveBeenCalledWith('hello'), { timeout: 4_000 });
    expect(requests.filter((r) => r === 'GET /api/operations/op-g')).toHaveLength(2);
    expect(requests).not.toContain('GET /api/plans/hello');
  });

  it('asks for the plan once the server no longer knows the operation', async () => {
    const requests = stubServer([null], true);
    const onDone = vi.fn();
    generate(onDone);

    await waitFor(() => expect(onDone).toHaveBeenCalledWith('hello'), { timeout: 4_000 });
    expect(requests.filter((r) => r === 'GET /api/plans/hello')).toHaveLength(1);
  });
});
