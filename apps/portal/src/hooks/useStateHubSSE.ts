'use client';

/**
 * Roko Portal — `useStateHubSSE` React Hook
 *
 * Drives the three-step portal startup (sign in → snapshot → stream) by
 * calling `startLiveState` with real browser/API dependencies wired up to
 * the `useDashboardStore` Zustand store.
 *
 * Lifecycle:
 *  1. On mount, `startLiveState` is called with concrete deps injected here.
 *     The returned handle's `stop()` is called on unmount.
 *  2. The `postSession` dep POSTs to `/api/auth/session` and returns the HTTP
 *     status code so `startLiveState` can classify the outcome.
 *  3. The `dropFragment` dep strips the `#token=…` fragment from the address
 *     bar after the sign-in exchange completes.
 *  4. The `fetchSnapshot` dep calls `GET /api/statehub/snapshot` via the
 *     shared `api` client.
 *  5. The `openStream` dep builds a `SseClient` seeded with `lastEventId` and
 *     calls `connect(onEvent, onStatus)`.  The returned handle is kept in a
 *     ref so `forceReconnect` can be exposed as a UI "retry" action.
 *
 * Usage:
 * ```tsx
 * // Mount once near the root of your dashboard layout.
 * function DashboardLayout({ children }: { children: React.ReactNode }) {
 *   const { status, reconnect } = useStateHubSSE();
 *   return (
 *     <>
 *       <ConnectionBadge status={status} onRetry={reconnect} />
 *       {children}
 *     </>
 *   );
 * }
 * ```
 */

import { useEffect, useRef, useCallback } from 'react';

import { startLiveState } from '@/lib/bootstrap';
import type { ConnectionStatus } from '@/lib/bootstrap';
import { SseClient } from '@/api/sse-client';
import { api } from '@/api/client';
import type { WireStateHubSnapshotResponse } from '@/api/contracts';
import { useDashboardStore } from '@/stores/dashboard';

// ---------------------------------------------------------------------------
// Return type
// ---------------------------------------------------------------------------

export interface UseStateHubSSEResult {
  /** Current SSE connection status. */
  status: ConnectionStatus;
  /** Force an immediate reconnect (e.g. for a manual "retry" button). */
  reconnect: () => void;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useStateHubSSE(): UseStateHubSSEResult {
  // Grab stable action references from the Zustand store.
  const setSession = useDashboardStore((s) => s.setSession);
  const replaceFromSnapshot = useDashboardStore((s) => s.replaceFromSnapshot);
  const applyEvent = useDashboardStore((s) => s.applyEvent);
  const setConnection = useDashboardStore((s) => s.setConnection);
  const connection = useDashboardStore((s) => s.connection);

  // Keep a stable ref to the SseClient so we can expose `forceReconnect`
  // without pulling it into dependency arrays.
  const sseClientRef = useRef<SseClient | null>(null);

  // Start the live-state machine on mount; stop it on unmount.
  useEffect(() => {
    const handle = startLiveState({
      hash: typeof window !== 'undefined' ? window.location.hash : '',

      postSession: async (token: string): Promise<number> => {
        const res = await fetch('/api/auth/session', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ token }),
        });
        return res.status;
      },

      dropFragment: () => {
        if (typeof window !== 'undefined') {
          window.history.replaceState(
            null,
            '',
            window.location.pathname + window.location.search,
          );
        }
      },

      setSession,

      fetchSnapshot: () =>
        api.get<WireStateHubSnapshotResponse>('/api/statehub/snapshot'),

      openStream: (lastEventId, onEvent, onStatus) => {
        const client = new SseClient('', { lastEventId });
        sseClientRef.current = client;
        client.connect(onEvent, onStatus);
        return {
          close() {
            client.disconnect();
            if (sseClientRef.current === client) {
              sseClientRef.current = null;
            }
          },
        };
      },

      replace: replaceFromSnapshot,
      apply: applyEvent,
      setStatus: setConnection,

      sleep: (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)),
    });

    return () => {
      handle.stop();
    };
    // setSession, replaceFromSnapshot, applyEvent, setConnection are stable
    // Zustand action references — they never change identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Expose a stable reconnect callback for the UI.
  const reconnect = useCallback(() => {
    sseClientRef.current?.forceReconnect();
  }, []);

  return { status: connection, reconnect };
}
