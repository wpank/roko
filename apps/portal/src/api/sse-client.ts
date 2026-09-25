/**
 * Roko Portal — Standalone SSE Client
 *
 * Connects to the roko-serve `/api/events` endpoint, parses `DashboardEvent`
 * payloads from SSE `data:` frames, and drives automatic reconnect with
 * exponential backoff and cursor-based replay.
 *
 * Protocol notes (from `roko-serve/src/routes/sse.rs`):
 *
 *  - Regular events:  `id: <seq>\ndata: <DashboardEvent JSON>\n\n`
 *  - Gap frames:      `event: gap\nid: <seq>\ndata: <GapPayload JSON>\n\n`
 *  - Keep-alive:      `: keepalive` comments every 8 s (ignored by this client)
 *
 * The server accepts the last-seen cursor via the `Last-Event-ID` HTTP header
 * (highest precedence) or the `?lastEventId=<seq>` query parameter.  Because
 * `EventSource` does not support custom headers we pass the cursor as a URL
 * parameter on reconnect.
 *
 * The browser's built-in `EventSource` reconnect is disabled by closing and
 * re-opening the connection manually so we have full control over backoff
 * timing, cursor forwarding, and keepalive enforcement.
 */

import type { ConnectionStatus, DashboardEvent, DashboardSnapshot } from '@/api/types';

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/**
 * Callback invoked for every incoming `DashboardEvent`.
 *
 * For `event: gap` frames the parsed `GapPayload.snapshot` is wrapped in a
 * synthetic `SnapshotEvent` so callers can use a single dispatch path.
 */
export type EventCallback = (event: DashboardEvent) => void;

/** Callback invoked whenever the connection status changes. */
export type StatusCallback = (status: ConnectionStatus) => void;

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

/** Initial reconnect delay in milliseconds. */
const INITIAL_RECONNECT_DELAY_MS = 1_000;

/** Maximum reconnect delay cap in milliseconds (16 s). */
const MAX_RECONNECT_DELAY_MS = 16_000;

/** If no event (including keepalive comments) arrives within this window,
 *  force a reconnect to surface stale connections early. */
const KEEPALIVE_TIMEOUT_MS = 60_000;

/** SSE endpoint path on roko-serve. */
const SSE_PATH = '/api/events';

// ---------------------------------------------------------------------------
// GapPayload shape (mirrors Rust `GapPayload` in `routes/sse.rs`)
// ---------------------------------------------------------------------------

interface GapPayload {
  missed_events: number;
  last_materialized_seq: number;
  snapshot: DashboardSnapshot;
}

// ---------------------------------------------------------------------------
// SseClient
// ---------------------------------------------------------------------------

/**
 * Standalone SSE client with cursor-based replay, exponential-backoff
 * reconnect, and keepalive watchdog.
 *
 * @example
 * ```ts
 * const client = new SseClient('http://localhost:6677');
 *
 * client.connect(
 *   (event) => store.getState().applyEvent(event),
 *   (status) => store.getState().setConnectionStatus(status),
 * );
 *
 * // Cleanup on unmount:
 * client.disconnect();
 * ```
 */
export class SseClient {
  private eventSource: EventSource | null = null;
  private lastEventId: string | null = null;

  /** Current reconnect attempt count; reset to 0 after a successful connect. */
  private reconnectAttempt = 0;

  /** Pending reconnect timer handle. */
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  /** Keepalive watchdog timer handle. */
  private keepaliveTimer: ReturnType<typeof setTimeout> | null = null;

  private currentStatus: ConnectionStatus = 'disconnected';

  /** Stored callbacks — set once by the first `connect` call. */
  private onEvent: EventCallback | null = null;
  private onStatus: StatusCallback | null = null;

  /** Whether the client has been permanently disconnected by the caller. */
  private destroyed = false;

  constructor(
    private readonly baseUrl: string,
    private readonly apiKey?: string,
  ) {}

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Begin (or restart) the SSE connection.
   *
   * Calling `connect` while already connected is a no-op unless the
   * connection is in an error/disconnected state, in which case it triggers
   * an immediate reconnect.
   *
   * @param onEvent  Fired for every `DashboardEvent` (including synthetic
   *                 snapshot events generated from gap frames).
   * @param onStatus Fired whenever the connection status changes.
   */
  connect(onEvent: EventCallback, onStatus: StatusCallback): void {
    if (this.destroyed) return;

    // Store callbacks so the internal reconnect loop can reuse them.
    this.onEvent = onEvent;
    this.onStatus = onStatus;

    // If we're already connected do nothing; if there's a pending reconnect
    // let it run.
    if (this.currentStatus === 'connected' || this.currentStatus === 'connecting') {
      return;
    }

    this.openConnection();
  }

  /** Permanently close the connection and cancel any pending reconnects. */
  disconnect(): void {
    this.destroyed = true;
    this.cancelReconnect();
    this.cancelKeepalive();
    this.closeEventSource();
    this.setStatus('disconnected');
  }

  /** Return the current connection status without registering a listener. */
  getStatus(): ConnectionStatus {
    return this.currentStatus;
  }

  /**
   * Force an immediate reconnect regardless of the current state.
   * Useful to expose as a manual "retry" action in the UI.
   */
  forceReconnect(): void {
    if (this.destroyed) return;
    this.reconnectAttempt = 0;
    this.cancelReconnect();
    this.cancelKeepalive();
    this.closeEventSource();
    this.setStatus('connecting');
    this.openConnection();
  }

  // -------------------------------------------------------------------------
  // Internal helpers
  // -------------------------------------------------------------------------

  private buildUrl(): string {
    const base = this.baseUrl.replace(/\/+$/, '');
    // When baseUrl is empty (dev proxy mode), build a relative URL so
    // EventSource connects through the Next.js proxy on the current origin.
    const origin = base || (typeof window !== 'undefined' ? window.location.origin : 'http://localhost:3000');
    const url = new URL(`${origin}${SSE_PATH}`);

    // EventSource cannot set custom headers; pass the cursor as a query
    // parameter.  The server honours `?lastEventId=` at the same precedence
    // as the `Last-Event-ID` header when the header is absent.
    if (this.lastEventId !== null) {
      url.searchParams.set('lastEventId', this.lastEventId);
    }

    return url.toString();
  }

  private openConnection(): void {
    if (this.destroyed) return;

    this.setStatus('connecting');
    this.closeEventSource();

    let es: EventSource;
    try {
      es = new EventSource(this.buildUrl());
    } catch (err) {
      // URL construction or EventSource instantiation failed (e.g. in a test
      // environment without a browser).
      console.error('[SseClient] Failed to create EventSource:', err);
      this.setStatus('error');
      this.scheduleReconnect();
      return;
    }

    this.eventSource = es;

    es.onopen = () => {
      this.reconnectAttempt = 0;
      this.setStatus('connected');
      this.resetKeepalive();
    };

    // Default message handler: incremental DashboardEvent.
    es.onmessage = (ev: MessageEvent<string>) => {
      this.resetKeepalive();
      this.handleDataFrame(ev.data, ev.lastEventId ?? null);
    };

    // `event: gap` frame — the server replaces missing replay with a full
    // DashboardSnapshot so the client can recover without re-fetching.
    es.addEventListener('gap', (ev: Event) => {
      const msgEv = ev as MessageEvent<string>;
      this.resetKeepalive();
      this.handleGapFrame(msgEv.data, msgEv.lastEventId ?? null);
    });

    es.onerror = () => {
      // EventSource error fires on both initial connection failure and mid-
      // stream disconnects.  The readyState disambiguates.
      const wasClosed =
        es.readyState === EventSource.CLOSED ||
        es.readyState === EventSource.CONNECTING;

      this.closeEventSource();
      this.cancelKeepalive();

      if (wasClosed) {
        this.setStatus('disconnected');
      } else {
        this.setStatus('error');
      }

      if (!this.destroyed) {
        this.scheduleReconnect();
      }
    };
  }

  private handleDataFrame(data: string, eventId: string | null): void {
    if (eventId) {
      this.lastEventId = eventId;
    }

    let parsed: unknown;
    try {
      parsed = JSON.parse(data);
    } catch {
      console.warn('[SseClient] Failed to parse event data:', data);
      return;
    }

    // The server serialises DashboardEvent with `#[serde(tag = "type")]` so
    // every variant carries a `type` discriminant.
    if (
      typeof parsed !== 'object' ||
      parsed === null ||
      typeof (parsed as Record<string, unknown>)['type'] !== 'string'
    ) {
      console.warn('[SseClient] Received event without `type` field:', parsed);
      return;
    }

    this.onEvent?.(parsed as DashboardEvent);
  }

  private handleGapFrame(data: string, eventId: string | null): void {
    if (eventId) {
      this.lastEventId = eventId;
    }

    let payload: GapPayload;
    try {
      payload = JSON.parse(data) as GapPayload;
    } catch {
      console.warn('[SseClient] Failed to parse gap payload:', data);
      return;
    }

    if (!payload?.snapshot) {
      console.warn('[SseClient] Gap payload missing snapshot:', payload);
      return;
    }

    // Wrap the snapshot in a synthetic `SnapshotEvent` so the store can use
    // `applyEvent` for both incremental updates and full replacements.
    const syntheticEvent: DashboardEvent = {
      type: 'snapshot',
      snapshot: payload.snapshot,
      cursor: this.lastEventId ?? String(payload.last_materialized_seq),
      timestamp: new Date().toISOString(),
    };

    this.onEvent?.(syntheticEvent);
  }

  private closeEventSource(): void {
    if (this.eventSource) {
      this.eventSource.onopen = null;
      this.eventSource.onmessage = null;
      this.eventSource.onerror = null;
      this.eventSource.close();
      this.eventSource = null;
    }
  }

  // -------------------------------------------------------------------------
  // Reconnect logic (exponential backoff: 1s → 2s → 4s → 8s → 16s cap)
  // -------------------------------------------------------------------------

  private scheduleReconnect(): void {
    if (this.destroyed) return;
    this.cancelReconnect();

    const delay = Math.min(
      INITIAL_RECONNECT_DELAY_MS * Math.pow(2, this.reconnectAttempt),
      MAX_RECONNECT_DELAY_MS,
    );
    this.reconnectAttempt++;

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      if (!this.destroyed) {
        this.openConnection();
      }
    }, delay);
  }

  private cancelReconnect(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  // -------------------------------------------------------------------------
  // Keepalive watchdog
  // -------------------------------------------------------------------------

  private resetKeepalive(): void {
    this.cancelKeepalive();
    this.keepaliveTimer = setTimeout(() => {
      this.keepaliveTimer = null;
      if (!this.destroyed && this.currentStatus === 'connected') {
        // No traffic for KEEPALIVE_TIMEOUT_MS — force reconnect.
        console.warn('[SseClient] Keepalive timeout; forcing reconnect');
        this.closeEventSource();
        this.setStatus('disconnected');
        this.openConnection();
      }
    }, KEEPALIVE_TIMEOUT_MS);
  }

  private cancelKeepalive(): void {
    if (this.keepaliveTimer !== null) {
      clearTimeout(this.keepaliveTimer);
      this.keepaliveTimer = null;
    }
  }

  // -------------------------------------------------------------------------
  // Status helper
  // -------------------------------------------------------------------------

  private setStatus(status: ConnectionStatus): void {
    if (this.currentStatus !== status) {
      this.currentStatus = status;
      this.onStatus?.(status);
    }
  }
}
