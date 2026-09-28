// @vitest-environment jsdom
/**
 * Acceptance: the component-test harness (jsdom + Testing Library + helpers).
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import { useQuery } from '@tanstack/react-query';
import { api, ApiError } from '@/api/client';
import { useDashboardStore } from '@/stores/dashboard';
import {
  foldEvents,
  hasMissingValue,
  renderWithClient,
  setStore,
  stubFetch,
  testQueryClient,
  textOf,
} from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function Probe() {
  const { data } = useQuery<string>({ queryKey: ['probe'], queryFn: () => 'fetched' });
  return <p data-probe>{data ?? 'loading'}</p>;
}

describe('component test harness', () => {
  it('runs in a DOM environment', () => {
    expect(typeof document).toBe('object');
    expect(document.createElement('div').tagName).toBe('DIV');
  });

  it('renders with seeded query data and no fetch', () => {
    const fetchMock = stubFetch([]);
    renderWithClient(<Probe />, { seed: [[['probe'], 'seeded']] });
    expect(textOf(document.querySelector('[data-probe]'))).toBe('seeded');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('never retries a failed query', () => {
    const client = testQueryClient();
    expect(client.getDefaultOptions().queries?.retry).toBe(false);
  });

  it('folds events into run state', () => {
    const run = foldEvents([
      { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
      { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
    ]);
    expect(run.plans['hello']?.phase).toBe('running');
    expect(run.planSet?.planIds).toEqual(['hello']);
  });

  it('writes the dashboard store', () => {
    const run = foldEvents([{ type: 'plan_started', plan_id: 'p', tasks_total: 1 }]);
    setStore(run, { connection: 'connecting' });
    expect(useDashboardStore.getState().run.plans['p']?.phase).toBe('running');
    expect(useDashboardStore.getState().connection).toBe('connecting');
  });

  it('answers requests from stubbed routes', async () => {
    stubFetch([
      { path: '/api/status', body: { workdir: '/tmp/ws', git_branch: 'main' } },
      { method: 'POST', path: '/api/plans/execute', status: 405, statusText: 'Method Not Allowed' },
    ]);
    await expect(api.get('/api/status')).resolves.toEqual({ workdir: '/tmp/ws', git_branch: 'main' });
    const err = await api.post('/api/plans/execute', {}).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(405);
  });

  it('rejects unexpected requests', async () => {
    stubFetch([]);
    await expect(api.get('/api/nowhere')).rejects.toThrow(/unexpected request/);
  });

  it('flags missing values in rendered text', () => {
    expect(hasMissingValue('undefined/2')).toBe(true);
    expect(hasMissingValue('NaN tok')).toBe(true);
    expect(hasMissingValue('0/2 · 5s')).toBe(false);
  });
});
