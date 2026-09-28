/**
 * dom.tsx — shared helpers for jsdom component tests.
 *
 * A component test file starts with the line `// @vitest-environment jsdom`
 * and calls `cleanup()` from @testing-library/react after each test: vitest
 * globals are off, so Testing Library cannot register that hook itself.
 *
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 * The acceptance tests import it; do not change its exports.
 */

import React from 'react';
import { render } from '@testing-library/react';
import type { RenderResult } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { QueryKey } from '@tanstack/react-query';
import { vi } from 'vitest';
import type { WireDashboardEvent } from '@/api/contracts';
import type { ConnectionStatus } from '@/lib/bootstrap';
import { applyEvent, initialRunState } from '@/lib/runState';
import type { RunState } from '@/lib/runState';
import { useDashboardStore } from '@/stores/dashboard';

/** Query data to place in the cache before rendering: `[queryKey, data]`. */
export type Seed = ReadonlyArray<readonly [QueryKey, unknown]>;

/** A QueryClient that never retries and never refetches, pre-filled with `seed`. */
export function testQueryClient(seed: Seed = []): QueryClient {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, gcTime: Infinity },
      mutations: { retry: false },
    },
  });
  for (const [key, data] of seed) client.setQueryData(key, data);
  return client;
}

/** Render `ui` inside a QueryClientProvider (a fresh `testQueryClient(seed)` by default). */
export function renderWithClient(
  ui: React.ReactElement,
  opts: { seed?: Seed; client?: QueryClient } = {},
): RenderResult & { client: QueryClient } {
  const client = opts.client ?? testQueryClient(opts.seed ?? []);
  const result = render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
  return Object.assign(result, { client });
}

/** Fold `events` into a fresh RunState, `stepMs` apart starting at `startMs`. */
export function foldEvents(
  events: readonly WireDashboardEvent[],
  startMs = 1_000,
  stepMs = 1,
): RunState {
  let run = initialRunState();
  events.forEach((event, i) => {
    run = applyEvent(run, event, startMs + i * stepMs);
  });
  return run;
}

/** Replace the dashboard store's run state (connected and signed in by default). */
export function setStore(
  run: RunState,
  extra: { connection?: ConnectionStatus } = {},
): void {
  useDashboardStore.setState({
    run,
    connection: extra.connection ?? 'connected',
    session: 'signed-in',
  });
}

/** One canned answer for `stubFetch`. `path` is matched against the request pathname. */
export interface FetchRoute {
  method?: string;
  path: string;
  status?: number;
  statusText?: string;
  body?: unknown;
}

/**
 * Replace global fetch with a stub that answers from `routes` (first match on
 * method and pathname). An unmatched request rejects, like a network error.
 * Returns the mock so tests can count calls.
 */
export function stubFetch(routes: readonly FetchRoute[]) {
  const mock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = new URL(
      typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
      'http://localhost',
    );
    const method = (init?.method ?? 'GET').toUpperCase();
    const route = routes.find(
      (r) => (r.method ?? 'GET').toUpperCase() === method && r.path === url.pathname,
    );
    if (!route) throw new TypeError(`unexpected request: ${method} ${url.pathname}`);
    const status = route.status ?? 200;
    const text =
      route.body === undefined
        ? null
        : typeof route.body === 'string'
          ? route.body
          : JSON.stringify(route.body);
    return new Response(status === 204 ? null : text, {
      status,
      statusText: route.statusText ?? '',
      headers: { 'content-type': 'application/json' },
    });
  });
  vi.stubGlobal('fetch', mock);
  return mock;
}

/** The text of `el` (or '' when null), for substring assertions. */
export function textOf(el: Element | null): string {
  return el?.textContent ?? '';
}

/** Fails when `text` shows a missing value: `undefined`, `NaN`, or `null`. */
export function hasMissingValue(text: string): boolean {
  return /undefined|NaN|\bnull\b/.test(text);
}
