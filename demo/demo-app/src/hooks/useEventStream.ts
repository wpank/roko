import { useEffect, useRef, useCallback, useState } from 'react';
import { subscribeServerEvents, subscribeSseConnected } from '../app/bootstrap';

// ── Legacy interface (kept for useEventStreamSubscription compat) ─────────────

/** @deprecated Use useServerEventSubscription instead. */
export interface EventStreamManager {
  connected: boolean;
  onConnectedChange: ((connected: boolean) => void) | null;
  subscribe(types: string[], handler: (event: unknown) => void): () => void;
  destroy(): void;
}

// ── Canonical single-manager subscription hooks ──────────────────────────────

/**
 * Subscribe to one or more SSE event types from the canonical dashboard SSE
 * stream managed by `bootstrapTransport()`.
 *
 * `handler` is stabilised internally — no need to memoize at call site.
 * Subscription is torn down on unmount.
 */
export function useServerEventSubscription(
  eventTypes: string[],
  handler: (event: Record<string, unknown>) => void,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  const typesKey = eventTypes.join(',');

  const stable = useCallback((ev: Record<string, unknown>) => {
    handlerRef.current(ev);
  }, []);

  useEffect(() => {
    const types = typesKey.split(',').filter(Boolean);
    if (types.length === 0) return;
    return subscribeServerEvents(types, stable);
  }, [typesKey, stable]);
}

/**
 * Returns the current SSE connection state, updating whenever the
 * canonical bootstrap SSE connects or disconnects.
 */
export function useServerConnected(): boolean {
  const [connected, setConnected] = useState(false);
  useEffect(() => subscribeSseConnected(setConnected), []);
  return connected;
}

/**
 * Hook that subscribes to specific SSE event types via an EventStreamManager.
 * @deprecated Use useServerEventSubscription for the canonical SSE stream.
 */
export function useEventStreamSubscription(
  manager: EventStreamManager | null,
  eventTypes: string[],
  handler: (event: unknown) => void,
) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  const typesKey = eventTypes.join(',');

  const stableHandler = useCallback((event: unknown) => {
    handlerRef.current(event);
  }, []);

  useEffect(() => {
    if (!manager) return;
    const types = typesKey.split(',').filter(Boolean);
    if (types.length === 0) return;
    return manager.subscribe(types, stableHandler);
  }, [manager, typesKey, stableHandler]);
}
