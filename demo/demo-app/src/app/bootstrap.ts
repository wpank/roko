/**
 * Transport wiring — connects app-wide SSE and health polling into DataHub.
 *
 * Calls are idempotent. The returned cleanup function tears down all global
 * transports and allows a later call (including HMR) to start them again.
 *
 * Implementation task: T1.11
 *
 * This module owns one generic dashboard SSE connection and one dedicated
 * bench SSE connection. Components subscribe to their DataHub projections
 * instead of constructing EventSource instances per mount.
 */

import { api } from '../transport/api';
import { SseAdapter } from '../transport/sse';
import {
  parseDashboardEvent,
  parseDashboardGap,
  type DashboardEvent,
} from '../transport/types';
import { parseBenchSSEEvent } from '../lib/bench-types';
import { useDataHub } from './DataHub';
import { SERVE_URL } from '../lib/serve-url';

// ── Module-level event bus ───────────────────────────────────────────────────
// Components subscribe here instead of creating a second EventSource.

type RawEventHandler = (event: Record<string, unknown>) => void;
export type DashboardSubscriptionType = DashboardEvent['type'] | 'gap' | '*';

interface EventBus {
  handlers: Map<string, Set<RawEventHandler>>;
}

const _bus: EventBus = {
  handlers: new Map(),
};

let activeTransportCleanup: (() => void) | null = null;

/**
 * Publish a raw SSE event to all subscribed handlers.
 * Called by the SseAdapter's onEvent callback.
 */
function _publishRawEvent(raw: Record<string, unknown>): void {
  const type = typeof raw.type === 'string' ? raw.type : null;
  if (type) {
    const byType = _bus.handlers.get(type);
    if (byType) {
      for (const h of byType) h(raw);
    }
  }
  const wildcard = _bus.handlers.get('*');
  if (wildcard) {
    for (const h of wildcard) h(raw);
  }
}

/**
 * Subscribe to one or more SSE event type names from the canonical SSE stream.
 * Pass `['*']` to receive all events.
 * Returns an unsubscribe function.
 */
export function subscribeServerEvents(
  types: readonly DashboardSubscriptionType[],
  handler: RawEventHandler,
): () => void {
  for (const t of types) {
    let set = _bus.handlers.get(t);
    if (!set) {
      set = new Set();
      _bus.handlers.set(t, set);
    }
    set.add(handler);
  }
  return () => {
    for (const t of types) {
      const set = _bus.handlers.get(t);
      if (set) {
        set.delete(handler);
        if (set.size === 0) _bus.handlers.delete(t);
      }
    }
  };
}

/**
 * Initialize transport layer and wire events into DataHub.
 * Call ONCE before React render. Returns cleanup function.
 */
export function bootstrapTransport(): () => void {
  if (activeTransportCleanup) return activeTransportCleanup;

  const hub = useDataHub.getState;
  const set = useDataHub.setState;
  let active = true;

  // 1. Probe server health
  api.probe().then((snap) => {
    if (!active) return;
    set({
      serverStatus: snap.reachable ? 'connected' : 'disconnected',
    });
  });

  // 2. Health poll every 30s
  const healthInterval = setInterval(() => {
    api.probe(true).then((snap) => {
      if (!active) return;
      set({
        serverStatus: snap.reachable ? 'connected' : 'disconnected',
      });
    });
  }, 30_000);

  // 3. Connect SSE -> route events to DataHub AND module-level bus
  const sse = new SseAdapter({
    url: `${SERVE_URL}/api/events`,
    onEvent: (raw) => {
      if (raw.type === 'gap') {
        const gap = parseDashboardGap(raw);
        if (!gap) return;
        hub().hydrateDashboardSnapshot(gap.snapshot, gap);
        _publishRawEvent({ ...gap, type: 'gap' });
        return;
      }

      const event = parseDashboardEvent(raw);
      if (!event) return;
      // Only validated frames reach either the store or raw subscribers.
      hub().handleServerEvent(event);
      _publishRawEvent(event as unknown as Record<string, unknown>);
    },
    onStatusChange: (status) => set({ sseStatus: status }),
  });
  sse.connect();

  // 4. Connect the one app-wide bench event stream. Bench events come from a
  // separate roko-serve bus and are intentionally not part of DashboardEvent.
  let benchSse: EventSource | null = null;
  let benchRetryTimer: ReturnType<typeof setTimeout> | null = null;
  let benchHasConnected = false;

  const connectBenchSse = () => {
    if (!active) return;
    set({ benchSseStatus: benchHasConnected ? 'reconnecting' : 'connecting' });
    benchSse?.close();

    const connection = new EventSource(`${SERVE_URL}/api/bench/events`);
    benchSse = connection;
    connection.onopen = () => {
      if (!active || benchSse !== connection) return;
      benchHasConnected = true;
      set({ benchSseStatus: 'connected' });
    };
    connection.onmessage = (message) => {
      if (!active || benchSse !== connection) return;
      try {
        const event = parseBenchSSEEvent(JSON.parse(message.data));
        if (event) hub().handleBenchEvent(event);
      } catch {
        // Ignore malformed frames without disturbing the live stream.
      }
    };
    connection.onerror = () => {
      if (!active || benchSse !== connection) {
        connection.close();
        return;
      }
      connection.close();
      benchSse = null;
      set({ benchSseStatus: 'reconnecting' });
      if (benchRetryTimer) clearTimeout(benchRetryTimer);
      benchRetryTimer = setTimeout(connectBenchSse, 3_000);
    };
  };
  connectBenchSse();

  // 5. Workflow SSE/WS is opened by workflow-api only when a workspace-root
  // session exists; a global connection cannot issue the required subscribe.

  // 6. Initial REST fetches
  hub().fetchConfig();
  hub().fetchServerWorkdir();

  // 7. Cleanup function
  const cleanup = () => {
    if (!active) return;
    active = false;
    clearInterval(healthInterval);
    if (benchRetryTimer) clearTimeout(benchRetryTimer);
    benchSse?.close();
    sse.destroy();
    set({ sseStatus: 'idle', benchSseStatus: 'idle', wsStatus: 'idle' });
    if (activeTransportCleanup === cleanup) activeTransportCleanup = null;
  };
  activeTransportCleanup = cleanup;
  return cleanup;
}
