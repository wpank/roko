'use client';

/**
 * Roko Portal — `useStateHubSSE` React Hook
 *
 * Wires the `SseClient` to the `useDashboardStore` Zustand store.
 *
 * Lifecycle:
 *  1. On mount, a `SseClient` instance is created (memoised) and `connect`
 *     is called.  The client URL is resolved once from `getRokoServeUrl()`.
 *  2. Every `DashboardEvent` is dispatched to `store.applyEvent`.
 *  3. `SnapshotEvent` payloads (synthetic events generated from SSE gap
 *     frames) also call `store.replaceSnapshot` for an atomic full replacement
 *     before dispatching the event so subscribers can react to the reset.
 *  4. Status changes from the SSE client update `store.connectionStatus`.
 *  5. On unmount the client is permanently disconnected.
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
 *
 * The hook is safe to call from multiple components simultaneously: each call
 * creates its own isolated `SseClient` instance.  For a single shared
 * connection, mount the hook once in a layout component and read state via
 * `useDashboardStore` elsewhere.
 */

import { useEffect, useRef, useCallback } from 'react';

import { SseClient } from '@/api/sse-client';
import type { ConnectionStatus, DashboardEvent } from '@/api/types';
import { getRokoServeUrl } from '@/lib/env';
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
  const applyEvent = useDashboardStore((s) => s.applyEvent);
  const replaceSnapshot = useDashboardStore((s) => s.replaceSnapshot);
  const setConnectionStatus = useDashboardStore((s) => s.setConnectionStatus);
  const connectionStatus = useDashboardStore((s) => s.connectionStatus);

  // Keep a stable ref to the client so we can call `disconnect` in cleanup
  // without pulling it into dependency arrays.
  const clientRef = useRef<SseClient | null>(null);

  // Build the event callback.  useCallback keeps the identity stable across
  // renders; `applyEvent` and `replaceSnapshot` are themselves stable Zustand
  // action references so this never reconstructs unnecessarily.
  const handleEvent = useCallback(
    (event: DashboardEvent) => {
      // When the server sends a gap frame we synthesise a `snapshot` event.
      // Call replaceSnapshot first so the store's derived state is atomically
      // replaced before applyEvent fires any additional listeners.
      if (event.type === 'snapshot') {
        replaceSnapshot(event.snapshot);
      }
      // Always call applyEvent so the store's switch statement can run any
      // snapshot-variant-specific logic (e.g. clearing error state) and so
      // external subscribers to applyEvent see the event regardless of type.
      applyEvent(event);

      if (process.env.NODE_ENV !== 'production') {
        // eslint-disable-next-line no-console
        console.debug('[StateHubSSE]', event.type, event);
      }
    },
    [applyEvent, replaceSnapshot],
  );

  const handleStatus = useCallback(
    (status: ConnectionStatus) => {
      setConnectionStatus(status);
    },
    [setConnectionStatus],
  );

  // Create the client once on mount.  We resolve the URL inside the effect
  // so that localStorage is always available (client-only code path).
  useEffect(() => {
    const url = getRokoServeUrl();
    const client = new SseClient(url);
    clientRef.current = client;

    client.connect(handleEvent, handleStatus);

    return () => {
      client.disconnect();
      clientRef.current = null;
    };
    // handleEvent / handleStatus have stable identities from useCallback.
    // We intentionally omit them from the dep array to avoid reconnecting on
    // every render while still closing over the latest store actions.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Expose a stable reconnect callback for the UI.
  const reconnect = useCallback(() => {
    clientRef.current?.forceReconnect();
  }, []);

  return { status: connectionStatus, reconnect };
}
