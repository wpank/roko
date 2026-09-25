'use client';

/**
 * Roko Portal — `useWebSocket` React Hook
 *
 * Wraps `WsClient` in a React lifecycle, providing a stable interface for
 * bidirectional WebSocket connections with automatic reconnect, topic
 * subscriptions, and proper cleanup on unmount.
 *
 * Lifecycle:
 *  1. On mount (and whenever `enabled` transitions from `false` → `true`), a
 *     new `WsClient` instance is created and `connect()` is called.
 *  2. The `onMessage` callback is stored in a ref so callers can use an
 *     inline arrow function without causing unnecessary reconnects.
 *  3. Status changes from the `WsClient` are mirrored into React state so
 *     components re-render appropriately.
 *  4. On unmount (or when `enabled` transitions `true` → `false`) the client
 *     is permanently disconnected and the ref is cleared.
 *
 * Options stability:
 *  `path`, `topics`, and `apiKey` are captured at mount time.  Changes to
 *  these values after mount do **not** cause a reconnect — the intent is that
 *  the connection target is fixed for the lifetime of the hook instance.  Use
 *  `key` on the parent component to force a fresh mount if the target changes.
 *
 * Usage:
 * ```tsx
 * function PlanStream({ planId }: { planId: string }) {
 *   const { status, send, reconnect } = useWebSocket({
 *     path: '/ws',
 *     topics: ['plan_started', 'task_completed'],
 *     onMessage: (data) => dispatch(data),
 *   });
 *
 *   return (
 *     <div>
 *       <Badge status={status} onRetry={reconnect} />
 *       <button onClick={() => send({ type: 'subscribe_plan', planId })}>
 *         Subscribe
 *       </button>
 *     </div>
 *   );
 * }
 * ```
 */

import { useEffect, useRef, useState, useCallback } from 'react';

import { WsClient } from '@/api/ws-client';
import type { WsStatus } from '@/api/ws-client';

// Re-export WsStatus so consumers can import from this module without pulling
// in the client implementation directly.
export type { WsStatus };

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface UseWebSocketOptions {
  /**
   * WebSocket path on the roko-serve host.
   * e.g. `'/ws'` or `'/ws/terminal/123'`.
   */
  path: string;
  /**
   * Optional server-side topic filter.
   * e.g. `['plan_started', 'gate_result']`.
   * Passed as `?topics=X,Y` in the handshake URL.
   */
  topics?: string[];
  /**
   * When `false` the client will not connect.  Defaults to `true`.
   * Toggle to `false` to temporarily suspend the connection without
   * unmounting the component.
   */
  enabled?: boolean;
  /**
   * Called for every successfully parsed inbound message.
   *
   * The function identity is captured in a ref so callers may pass an
   * inline arrow function without triggering reconnects on every render.
   */
  onMessage: (data: unknown) => void;
  /**
   * Optional API key forwarded as `?api_key=` in the handshake URL.
   * Captured at mount time; changes after mount are ignored.
   */
  apiKey?: string;
}

// ---------------------------------------------------------------------------
// Return type
// ---------------------------------------------------------------------------

export interface UseWebSocketResult {
  /** Current WebSocket connection status. */
  status: WsStatus;
  /**
   * Send a message over the WebSocket.  The payload is serialised with
   * `JSON.stringify`.  If the socket is not currently open the message is
   * dropped with a console warning.
   */
  send: (data: unknown) => void;
  /**
   * Tear down the current client and open a fresh connection immediately.
   * Useful to expose as a manual "retry" button in error states.
   */
  reconnect: () => void;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useWebSocket({
  path,
  topics,
  enabled = true,
  onMessage,
  apiKey,
}: UseWebSocketOptions): UseWebSocketResult {
  // Mirror the client's status into React state so the component re-renders
  // when the connection status changes.
  const [status, setStatus] = useState<WsStatus>('disconnected');

  // Keep a stable ref to the client so `send` and `reconnect` can access it
  // without being included in dependency arrays.
  const clientRef = useRef<WsClient | null>(null);

  // Store the latest `onMessage` in a ref so we never need to reconstruct the
  // client because the caller passed a new inline arrow function.
  const onMessageRef = useRef<(data: unknown) => void>(onMessage);
  useEffect(() => {
    onMessageRef.current = onMessage;
  });

  // Capture options that affect the connection URL at the point the effect
  // runs.  Changes after mount do not cause a reconnect; use `key` on the
  // parent component to force fresh mount if the target changes.
  const pathRef = useRef(path);
  const topicsRef = useRef(topics);
  const apiKeyRef = useRef(apiKey);

  // Create, connect, and tear down the WsClient whenever `enabled` changes.
  useEffect(() => {
    if (!enabled) {
      // If the caller disables the hook while connected, disconnect cleanly.
      if (clientRef.current) {
        clientRef.current.disconnect();
        clientRef.current = null;
      }
      setStatus('disconnected');
      return;
    }

    const client = new WsClient({
      path: pathRef.current,
      topics: topicsRef.current,
      apiKey: apiKeyRef.current,
      onMessage: (data) => {
        // Delegate to the latest onMessage ref so the caller can change
        // the handler between renders without triggering a reconnect.
        onMessageRef.current(data);
      },
      onStatus: (s) => {
        setStatus(s);
      },
    });

    clientRef.current = client;
    client.connect();

    return () => {
      client.disconnect();
      clientRef.current = null;
    };
    // `enabled` is the only dependency that should trigger reconnect.
    // path/topics/apiKey are captured in refs and stable across renders.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled]);

  // Stable send callback — delegates to the ref so the caller always gets a
  // function with a consistent identity across renders.
  const send = useCallback((data: unknown) => {
    if (!clientRef.current) {
      console.warn('[useWebSocket] send() called before client is mounted');
      return;
    }
    clientRef.current.send(data);
  }, []);

  // Stable reconnect callback — tears down the existing client and opens a
  // fresh connection immediately (resets the backoff counter).
  const reconnect = useCallback(() => {
    if (!enabled) return;

    // Disconnect the existing client, if any.
    if (clientRef.current) {
      clientRef.current.disconnect();
      clientRef.current = null;
    }

    setStatus('connecting');

    const client = new WsClient({
      path: pathRef.current,
      topics: topicsRef.current,
      apiKey: apiKeyRef.current,
      onMessage: (data) => {
        onMessageRef.current(data);
      },
      onStatus: (s) => {
        setStatus(s);
      },
    });

    clientRef.current = client;
    client.connect();
    // `enabled` is read at call time, not as a dep, since this is an
    // imperative action callback.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { status, send, reconnect };
}
