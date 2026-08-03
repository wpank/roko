/**
 * Transport wiring — connects SSE, WS, and health polling into DataHub.
 *
 * Call `bootstrapTransport()` ONCE before React render.
 * Returns a cleanup function for teardown.
 *
 * Implementation task: T1.11
 *
 * Single SSE manager: the SseAdapter opened here is the ONLY dashboard SSE
 * connection in the app. Components subscribe via `useServerEventSubscription`
 * (see hooks/useEventStream.ts) instead of constructing their own EventSource.
 */

import { api } from '../transport/api';
import { SseAdapter } from '../transport/sse';
import { WsAdapter } from '../transport/ws';
import { parseServerEvent } from '../transport/types';
import { useDataHub } from './DataHub';
import { SERVE_URL, WS_BASE } from '../lib/serve-url';

// ── Module-level event bus ───────────────────────────────────────────────────
// Components subscribe here instead of creating a second EventSource.

type RawEventHandler = (event: Record<string, unknown>) => void;

interface EventBus {
  handlers: Map<string, Set<RawEventHandler>>;
  sseConnected: boolean;
  sseConnectedListeners: Set<(c: boolean) => void>;
}

const _bus: EventBus = {
  handlers: new Map(),
  sseConnected: false,
  sseConnectedListeners: new Set(),
};

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

function _setConnected(connected: boolean): void {
  if (_bus.sseConnected === connected) return;
  _bus.sseConnected = connected;
  for (const fn of _bus.sseConnectedListeners) fn(connected);
}

/**
 * Subscribe to one or more SSE event type names from the canonical SSE stream.
 * Pass `['*']` to receive all events.
 * Returns an unsubscribe function.
 */
export function subscribeServerEvents(
  types: string[],
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
 * Subscribe to SSE connection state changes.
 * Returns an unsubscribe function.
 */
export function subscribeSseConnected(fn: (connected: boolean) => void): () => void {
  _bus.sseConnectedListeners.add(fn);
  // Immediately notify with current state
  fn(_bus.sseConnected);
  return () => _bus.sseConnectedListeners.delete(fn);
}

/** Read the current SSE connection state (non-reactive). */
export function isSseConnected(): boolean {
  return _bus.sseConnected;
}

/**
 * Initialize transport layer and wire events into DataHub.
 * Call ONCE before React render. Returns cleanup function.
 */
export function bootstrapTransport(): () => void {
  const hub = useDataHub.getState;
  const set = useDataHub.setState;

  // 1. Probe server health
  api.probe().then((snap) => {
    set({
      serverStatus: snap.reachable ? 'connected' : 'disconnected',
    });
  });

  // 2. Health poll every 30s
  const healthInterval = setInterval(() => {
    api.probe(true).then((snap) => {
      set({
        serverStatus: snap.reachable ? 'connected' : 'disconnected',
      });
    });
  }, 30_000);

  // 3. Connect SSE -> route events to DataHub AND module-level bus
  const sse = new SseAdapter({
    url: `${SERVE_URL}/api/events`,
    onEvent: (raw) => {
      // Publish to component subscribers first
      _publishRawEvent(raw);
      // Then route to DataHub store
      const event = parseServerEvent(raw);
      if (event) hub().handleServerEvent(event);
    },
    onStatusChange: (status) => {
      set({ sseStatus: status });
      _setConnected(status === 'connected');
    },
  });
  sse.connect();

  // 4. Connect WS (workflow frames -- not routed to DataHub directly)
  const ws = new WsAdapter({
    url: `${WS_BASE}/api/workflow/ws`,
    onFrame: () => {
      // WS frames are WorkflowFrames consumed by workflow-api.ts.
      // DataHub does not process them directly.
    },
    onStatusChange: (status) => set({ wsStatus: status }),
  });
  ws.connect();

  // 5. Initial REST fetches
  hub().fetchConfig();
  hub().fetchServerWorkdir();

  // 6. Cleanup function
  return () => {
    clearInterval(healthInterval);
    sse.destroy();
    ws.destroy();
    _setConnected(false);
  };
}
