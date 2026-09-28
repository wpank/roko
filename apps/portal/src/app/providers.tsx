'use client';

import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useStateHubSSE } from '@/hooks/useStateHubSSE';
import { useDashboardStore } from '@/stores/dashboard';

/**
 * SSEInitializer — rendered inside QueryClientProvider so it can call
 * React Query hooks if needed in the future. Invisible: no DOM output.
 *
 * Mounted unconditionally so the three-step startup (sign in → snapshot →
 * stream) begins immediately, before the session status is known.
 */
function SSEInitializer() {
  useStateHubSSE();
  return null;
}

/**
 * SessionGate — renders its children only once the sign-in step has
 * settled (i.e. `session` is no longer `'pending'`).  This prevents React
 * Query from firing requests that race the session cookie.
 */
function SessionGate({ children }: { children: React.ReactNode }) {
  const session = useDashboardStore((s) => s.session);
  if (session === 'pending') return null;
  return <>{children}</>;
}

/**
 * Providers — wraps the entire client tree.
 *
 * - QueryClientProvider: React Query instance scoped to the session.
 * - SSEInitializer: opens the StateHub SSE stream and fans events into
 *   the Zustand store for the lifetime of the app.
 * - SessionGate: holds children until the session cookie is established.
 */
export function Providers({ children }: { children: React.ReactNode }) {
  // Stable QueryClient instance — one per browser session, never
  // recreated on re-renders.
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            // With live SSE updates, stale-while-revalidate windows
            // can be generous. Keep data fresh for 30 s before background
            // refetches kick in.
            staleTime: 30_000,
            // Retry failed requests twice with exponential back-off.
            retry: 2,
            refetchOnWindowFocus: false,
          },
        },
      }),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <SSEInitializer />
      <SessionGate>{children}</SessionGate>
    </QueryClientProvider>
  );
}
