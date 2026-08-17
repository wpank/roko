import { useEffect, useRef, useCallback } from 'react';
import {
  subscribeServerEvents,
  type DashboardSubscriptionType,
} from '../app/bootstrap';

// ── Canonical single-manager subscription hooks ──────────────────────────────

/**
 * Subscribe to one or more SSE event types from the canonical dashboard SSE
 * stream managed by `bootstrapTransport()`.
 *
 * `handler` is stabilised internally — no need to memoize at call site.
 * Subscription is torn down on unmount.
 */
export function useServerEventSubscription(
  eventTypes: readonly DashboardSubscriptionType[],
  handler: (event: Record<string, unknown>) => void,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  const typesKey = eventTypes.join(',');

  const stable = useCallback((ev: Record<string, unknown>) => {
    handlerRef.current(ev);
  }, []);

  useEffect(() => {
    const types = typesKey
      .split(',')
      .filter(Boolean) as DashboardSubscriptionType[];
    if (types.length === 0) return;
    return subscribeServerEvents(types, stable);
  }, [typesKey, stable]);
}
