/**
 * queryClient — shared QueryClient factory.
 *
 * Call `createQueryClient()` once per session (e.g. inside a `useState`
 * initialiser) instead of constructing a QueryClient inline, so the retry
 * policy is defined in one place and can be imported by tests.
 */

import { QueryClient } from '@tanstack/react-query';
import { shouldRetryQuery } from '@/lib/apiErrors';

/**
 * Returns a new QueryClient with the portal's standard defaults:
 *  - `staleTime: 30_000`   — data stays fresh for 30 s (SSE keeps it current)
 *  - `retry: shouldRetryQuery` — never retries 4xx; retries others up to twice
 *  - `refetchOnWindowFocus: false` — SSE makes polling unnecessary
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        retry: shouldRetryQuery,
        refetchOnWindowFocus: false,
      },
    },
  });
}
