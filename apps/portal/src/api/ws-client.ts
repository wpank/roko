/**
 * Roko Portal — Standalone WebSocket Client
 *
 * Provides a bidirectional WebSocket connection to roko-serve endpoints,
 * supporting filtered event subscriptions and (future) terminal PTY streams.
 *
 * Protocol notes:
 *
 *  - All inbound messages are parsed as JSON; malformed frames are logged and
 *    dropped without crashing the client.
 *  - Topic filter is applied server-side via `?topics=X,Y` query parameters.
 *  - The `api_key` is passed as a query parameter (`?api_key=Z`) because the
 *    WebSocket handshake does not support custom request headers in browsers.
 *  - The client sends a JSON `{"type":"ping"}` frame every 30 s to keep the
 *    connection alive through proxies and load balancers that time out idle
 *    TCP connections.
 *  - On close or error the client schedules an exponential-backoff reconnect
 *    (1 s → 2 s → 4 s → 8 s → 16 s cap).  After a successful reconnect it
 *    re-sends the topic subscription so the server can re-apply filtering.
 *
 * URL construction:
 *  - `http://` → `ws://`
 *  - `https://` → `wss://`
 */

import { getRokoServeUrl } from '@/lib/env';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type WsStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface WsClientOptions {
  /** WebSocket path on the roko-serve host, e.g. `'/ws'` or `'/ws/terminal/123'`. */
  path: string;
  /** Optional server-side topic filter; e.g. `['plan_started', 'gate_result']`. */
  topics?: string[];
  /** Called for every successfully parsed inbound message. */
  onMessage: (data: unknown) => void;
  /** Called whenever the connection status changes. */
  onStatus?: (status: WsStatus) => void;
  /** Optional API key forwarded as `?api_key=` in the handshake URL. */
  apiKey?: string;
}

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

/** Initial reconnect delay in milliseconds. */
const INITIAL_RECONNECT_DELAY_MS = 1_000;

/** Maximum reconnect delay cap in milliseconds (16 s). */
const MAX_RECONNECT_DELAY_MS = 16_000;

/** Interval between outbound keepalive ping frames in milliseconds (30 s). */
const PING_INTERVAL_MS = 30_000;

/** Outbound ping frame payload sent to keep the connection alive. */
const PING_FRAME = JSON.stringify({ type: 'ping' });

// ---------------------------------------------------------------------------
// WsClient
// ---------------------------------------------------------------------------

/**
 * Standalone WebSocket client with automatic exponential-backoff reconnect,
 * topic resubscription on reconnect, and periodic keepalive ping frames.
 *
 * @example
 * ```ts
 * const client = new WsClient({
 *   path: '/ws',
 *   topics: ['plan_started', 'gate_result'],
 *   onMessage: (data) => console.log(data),
 *   onStatus: (s) => console.log('ws status:', s),
 * });
 *
 * client.connect();
 *
 * // Cleanup:
 * client.disconnect();
 * ```
 */
export class WsClient {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private reconnectAttempt = 0;
  private status: WsStatus = 'disconnected';

  /** Set to `true` by `disconnect()` to prevent further reconnect attempts. */
  private destroyed = false;

  constructor(private readonly options: WsClientOptions) {}

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Open the WebSocket connection.
   *
   * Calling `connect` while already connected or connecting is a no-op.
   * After a `disconnect()` call this method has no effect.
   */
  connect(): void {
    if (this.destroyed) return;
    if (this.status === 'connected' || this.status === 'connecting') return;
    this.openConnection();
  }

  /**
   * Permanently close the connection and cancel any pending reconnect or
   * ping timers.  After calling `disconnect` the client cannot be reused.
   */
  disconnect(): void {
    this.destroyed = true;
    this.cancelReconnect();
    this.cancelPing();
    this.closeSocket(false /* do not reschedule */);
    this.setStatus('disconnected');
  }

  /**
   * Send a message over the WebSocket.
   *
   * The payload is serialised with `JSON.stringify`.  If the socket is not
   * currently in the `OPEN` state the send is silently dropped and a warning
   * is written to the console.
   */
  send(data: unknown): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[WsClient] send() called while socket is not open; dropping message');
      return;
    }
    try {
      this.ws.send(JSON.stringify(data));
    } catch (err) {
      console.error('[WsClient] send() error:', err);
    }
  }

  /** Return the current connection status without registering a listener. */
  getStatus(): WsStatus {
    return this.status;
  }

  // -------------------------------------------------------------------------
  // Internal helpers
  // -------------------------------------------------------------------------

  private buildUrl(): string {
    let base = getRokoServeUrl().replace(/\/+$/, '');

    // When base is empty (dev proxy mode), use the current browser origin.
    if (!base && typeof window !== 'undefined') {
      base = window.location.origin;
    } else if (!base) {
      base = 'http://localhost:3000';
    }

    // Convert the HTTP base URL to a WebSocket URL.
    const wsBase = base
      .replace(/^https:\/\//, 'wss://')
      .replace(/^http:\/\//, 'ws://');

    const url = new URL(`${wsBase}${this.options.path}`);

    if (this.options.topics && this.options.topics.length > 0) {
      url.searchParams.set('topics', this.options.topics.join(','));
    }

    if (this.options.apiKey) {
      url.searchParams.set('api_key', this.options.apiKey);
    }

    return url.toString();
  }

  private openConnection(): void {
    if (this.destroyed) return;

    this.setStatus('connecting');
    this.closeSocket(false /* do not reschedule */);

    let ws: WebSocket;
    try {
      ws = new WebSocket(this.buildUrl());
    } catch (err) {
      // URL construction or WebSocket instantiation failed (e.g. in a test
      // environment without a browser WebSocket implementation).
      console.error('[WsClient] Failed to create WebSocket:', err);
      this.setStatus('error');
      this.scheduleReconnect();
      return;
    }

    this.ws = ws;

    ws.onopen = () => {
      this.reconnectAttempt = 0;
      this.setStatus('connected');
      this.startPing();

      // Re-send the topic subscription so the server can re-apply its filter
      // after any reconnect.
      if (this.options.topics && this.options.topics.length > 0) {
        this.send({ type: 'subscribe', topics: this.options.topics });
      }
    };

    ws.onmessage = (ev: MessageEvent<unknown>) => {
      // Skip our own ping responses / server keepalive heartbeats that are
      // not JSON-parseable or not meaningful application events.
      const raw = typeof ev.data === 'string' ? ev.data : null;
      if (raw === null) {
        // Binary frames are not expected but are not an error condition.
        return;
      }

      let parsed: unknown;
      try {
        parsed = JSON.parse(raw);
      } catch {
        console.warn('[WsClient] Failed to parse inbound frame:', raw);
        return;
      }

      this.options.onMessage(parsed);
    };

    ws.onclose = (ev: CloseEvent) => {
      // Clean close initiated by the server (code 1000/1001) or by us via
      // `closeSocket`.  Either way, tear down the ping loop and reschedule
      // unless permanently destroyed.
      this.cancelPing();
      this.ws = null;

      if (!this.destroyed) {
        const wasClean = ev.wasClean && ev.code === 1000;
        this.setStatus(wasClean ? 'disconnected' : 'error');
        this.scheduleReconnect();
      }
    };

    ws.onerror = () => {
      // `onerror` is always followed by `onclose` in the browser WebSocket
      // API, so we only update the status here and let `onclose` handle the
      // reconnect scheduling.
      console.error('[WsClient] WebSocket error on path:', this.options.path);
      this.setStatus('error');
      // Do not schedule reconnect here — onclose fires next and will do it.
    };
  }

  /**
   * Close the underlying WebSocket without triggering the reconnect logic.
   *
   * @param reschedule - When `false` the `onclose` handler will not fire a
   *   reconnect because `destroyed` gates that path.  Callers that want to
   *   suppress reconnect should set `this.destroyed = true` first or pass
   *   `false` and handle scheduling themselves.
   */
  private closeSocket(_reschedule: boolean): void {
    if (this.ws) {
      // Null out the handlers before calling close() so the onclose callback
      // does not double-schedule a reconnect.
      const ws = this.ws;
      this.ws = null;
      ws.onopen = null;
      ws.onmessage = null;
      ws.onerror = null;
      ws.onclose = null;

      try {
        ws.close(1000, 'client disconnect');
      } catch {
        // Closing an already-closed socket can throw in some environments.
      }
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
  // Keepalive ping
  // -------------------------------------------------------------------------

  private startPing(): void {
    this.cancelPing();
    this.pingTimer = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        try {
          this.ws.send(PING_FRAME);
        } catch (err) {
          console.warn('[WsClient] Failed to send ping frame:', err);
        }
      }
    }, PING_INTERVAL_MS);
  }

  private cancelPing(): void {
    if (this.pingTimer !== null) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  // -------------------------------------------------------------------------
  // Status helper
  // -------------------------------------------------------------------------

  private setStatus(status: WsStatus): void {
    if (this.status !== status) {
      this.status = status;
      this.options.onStatus?.(status);
    }
  }
}
