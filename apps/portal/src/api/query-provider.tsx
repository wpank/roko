'use client';

/**
 * Roko Portal — React Query Provider
 *
 * Wraps the application with a `QueryClientProvider` configured with
 * sensible defaults for polling a local roko-serve instance:
 *
 *  - `staleTime`: 30 s — data is considered fresh for 30 seconds after fetch,
 *    matching the server's periodic telemetry sampling cadence.
 *  - `gcTime`: 5 min — unused cache entries are held for 5 minutes before GC.
 *  - `retry`: 2 — transient network blips get two automatic retries.
 *  - `refetchOnWindowFocus`: true — re-validate when the tab regains focus so
 *    the portal stays in sync after the user switches away and returns.
 *  - `refetchOnReconnect`: true — re-validate when the browser comes back
 *    online.
 *
 * Individual hooks may override these defaults by passing their own
 * `staleTime` / `gcTime` / `retry` values.
 */

import React, { useState } from 'react';
import {
  QueryClient,
  QueryClientProvider,
  type DefaultOptions,
} from '@tanstack/react-query';

const DEFAULT_OPTIONS: DefaultOptions = {
  queries: {
    staleTime: 30_000,        // 30 s
    gcTime: 5 * 60_000,       // 5 min
    retry: 2,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
  },
  mutations: {
    retry: 0,
  },
};

/**
 * Create a fresh `QueryClient`.  Called inside a component so that
 * Next.js App Router server-component boundaries each get an isolated
 * client rather than sharing a module-level singleton.
 */
function makeQueryClient(): QueryClient {
  return new QueryClient({ defaultOptions: DEFAULT_OPTIONS });
}

// Stable singleton for non-streaming (non-Suspense) use in browser.
let browserQueryClient: QueryClient | undefined;

function getQueryClient(): QueryClient {
  if (typeof window === 'undefined') {
    // Server-side: always create a new client so we never share state between
    // requests.
    return makeQueryClient();
  }
  if (!browserQueryClient) {
    browserQueryClient = makeQueryClient();
  }
  return browserQueryClient;
}

// ---------------------------------------------------------------------------
// Provider component
// ---------------------------------------------------------------------------

interface QueryProviderProps {
  children: React.ReactNode;
}

/**
 * Drop this at the root of your React tree (e.g. inside a Next.js `layout.tsx`
 * or `providers.tsx`) to enable React Query throughout the portal.
 *
 * @example
 * ```tsx
 * // app/providers.tsx
 * 'use client';
 * import { QueryProvider } from '@/api/query-provider';
 * export function Providers({ children }: { children: React.ReactNode }) {
 *   return <QueryProvider>{children}</QueryProvider>;
 * }
 * ```
 */
export function QueryProvider({ children }: QueryProviderProps) {
  // useState ensures the client is only created once per component instance,
  // even under React 18 concurrent-mode re-renders.
  const [queryClient] = useState(() => getQueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}
