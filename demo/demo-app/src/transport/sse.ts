import { SERVE_URL } from '../lib/serve-url';
import { goToLogin, loggedOut, probeSession, type SessionProbe } from './api';

export type SseStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'failed';

/** How long a hidden tab keeps its streams open, by default (S11 §4.6). */
const HIDDEN_CLOSE_MS = 120_000;

/**
 * The delay before reconnect attempt `attempt` (1-based): `baseMs` doubling up to `maxMs`, so
 * 1, 2, 4 and 8 s, then every 15 s with the defaults.
 */
export function backoffDelayMs(attempt: number, baseMs = 1000, maxMs = 15_000): number {
  return Math.min(baseMs * 2 ** (attempt - 1), maxMs);
}

/** The session probe of a browser: none outside one, so tests and Node never call out. */
function defaultProbe(): Promise<SessionProbe | null> {
  return typeof window === 'undefined' ? Promise.resolve(null) : probeSession(SERVE_URL);
}

const KNOWN_SSE_EVENT_TYPES = [
  // `/api/events` sends DashboardEvent payloads as unnamed SSE messages. Its
  // only named frame is the replay-gap notification.
  'gap',
] as const;

export interface SseAdapterConfig {
  /** Full URL to SSE endpoint. */
  url: string;
  /** Called on every parsed SSE event. Receives the JSON-parsed object. */
  onEvent: (event: Record<string, unknown>) => void;
  /** Called whenever connection status changes. */
  onStatusChange: (status: SseStatus) => void;
  /** Max reconnect attempts before entering 'failed'. Default: 5. */
  maxRetries?: number;
  /** Max backoff delay in ms. Default: 15_000. */
  maxBackoffMs?: number;
  /** Base backoff delay in ms. Default: 1_000. */
  baseBackoffMs?: number;
  /**
   * Asked after a stream error, while the reconnect waits (9331). When the server takes
   * passphrase logins and this browser holds no session, `onUnauthenticated` runs and the
   * adapter stops instead of retrying a stream that can only fail. Default: the session endpoint.
   */
  probeSession?: () => Promise<SessionProbe | null>;
  /** What a logged-out visitor sees instead. Default: the login page. */
  onUnauthenticated?: () => void;
  /**
   * Close the stream after the tab has been hidden this long, and reopen it when the tab shows
   * again, so a forgotten tab does not keep the server awake. Default: 2 minutes.
   */
  hiddenCloseMs?: number;
}

export class SseAdapter {
  status: SseStatus;
  /** Last-Event-ID from server -- sent on reconnect for replay. */
  lastEventId: string | null;

  private config: SseAdapterConfig;
  private retryCount: number;
  private retryTimer: ReturnType<typeof setTimeout> | null;
  private es: EventSource | null;
  private destroyed: boolean;
  private hiddenTimer: ReturnType<typeof setTimeout> | null;
  private closedWhileHidden: boolean;
  private readonly onVisibilityChange = (): void => this.handleVisibility();

  constructor(config: SseAdapterConfig) {
    this.config = config;
    this.status = 'idle';
    this.lastEventId = null;
    this.retryCount = 0;
    this.retryTimer = null;
    this.es = null;
    this.destroyed = false;
    this.hiddenTimer = null;
    this.closedWhileHidden = false;
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', this.onVisibilityChange);
    }
  }

  /** A hidden tab closes its stream after `hiddenCloseMs`; showing it again reopens the stream. */
  private handleVisibility(): void {
    if (this.destroyed) return;
    if (document.visibilityState === 'hidden') {
      if (this.hiddenTimer === null && this.status !== 'idle') {
        this.hiddenTimer = setTimeout(() => {
          this.hiddenTimer = null;
          if (document.visibilityState === 'hidden') {
            this.disconnect();
            this.closedWhileHidden = true;
          }
        }, this.config.hiddenCloseMs ?? HIDDEN_CLOSE_MS);
      }
      return;
    }
    if (this.hiddenTimer !== null) {
      clearTimeout(this.hiddenTimer);
      this.hiddenTimer = null;
    }
    if (this.closedWhileHidden) {
      this.closedWhileHidden = false;
      this.connect();
    }
  }

  /** Stop retrying, and send a logged-out visitor to the login page, when the probe says so. */
  private async checkSession(): Promise<void> {
    const probe = await (this.config.probeSession ?? defaultProbe)();
    if (this.destroyed || !loggedOut(probe)) return;
    if (this.retryTimer !== null) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
    this.es?.close();
    this.es = null;
    this.setStatus('failed');
    (this.config.onUnauthenticated ?? goToLogin)();
  }

  private setStatus(s: SseStatus): void {
    if (s !== this.status) {
      this.status = s;
      this.config.onStatusChange(s);
    }
  }

  private handleMessage(event: MessageEvent, fallbackType?: string): void {
    if (/^\d{1,20}$/.test(event.lastEventId)) {
      this.lastEventId = event.lastEventId;
    }
    try {
      const parsed = JSON.parse(event.data) as Record<string, unknown>;
      const nested = parsed.data !== null && typeof parsed.data === 'object' && !Array.isArray(parsed.data)
        ? parsed.data as Record<string, unknown>
        : {};
      const type = typeof parsed.type === 'string'
        ? parsed.type
        : typeof parsed.kind === 'string'
          ? parsed.kind
          : fallbackType;
      this.config.onEvent({ ...nested, ...parsed, ...(type ? { type } : {}) });
    } catch {
      // skip unparseable events
    }
  }

  /** Open the EventSource connection. Idempotent -- does nothing if already connected. */
  connect(): void {
    if (this.destroyed || this.status === 'connected' || this.status === 'connecting') {
      return;
    }
    this.setStatus(this.retryCount === 0 ? 'connecting' : 'reconnecting');

    if (this.es) {
      this.es.close();
      this.es = null;
    }

    let url = this.config.url;
    if (this.lastEventId) {
      const separator = url.includes('?') ? '&' : '?';
      url = url + separator + 'lastEventId=' + encodeURIComponent(this.lastEventId);
    }

    const es = new EventSource(url);
    this.es = es;

    es.onopen = () => {
      if (this.destroyed || es !== this.es) return;
      this.retryCount = 0;
      this.setStatus('connected');
    };

    es.onmessage = (e: MessageEvent) => {
      if (this.destroyed || es !== this.es) return;
      this.handleMessage(e);
    };
    for (const type of KNOWN_SSE_EVENT_TYPES) {
      es.addEventListener(type, (e) => {
        if (this.destroyed || es !== this.es) return;
        this.handleMessage(e as MessageEvent, type);
      });
    }

    es.onerror = () => {
      if (this.destroyed) {
        es.close();
        return;
      }
      es.close();
      this.es = null;
      this.retryCount += 1;

      const maxRetries = this.config.maxRetries ?? 5;
      if (this.retryCount > maxRetries) {
        this.setStatus('failed');
        return;
      }

      this.setStatus('reconnecting');
      const baseMs = this.config.baseBackoffMs ?? 1000;
      const maxMs = this.config.maxBackoffMs ?? 15_000;
      const delay = backoffDelayMs(this.retryCount, baseMs, maxMs);
      this.retryTimer = setTimeout(() => this.connect(), delay);
      void this.checkSession();
    };
  }

  /** Close the connection and cancel any pending reconnect. Resets retry counter. */
  disconnect(): void {
    if (this.retryTimer !== null) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
    if (this.es) {
      this.es.close();
      this.es = null;
    }
    this.retryCount = 0;
    this.setStatus('idle');
  }

  /** Close + set status to 'idle'. After destroy(), connect() is a no-op. */
  destroy(): void {
    this.destroyed = true;
    if (this.hiddenTimer !== null) {
      clearTimeout(this.hiddenTimer);
      this.hiddenTimer = null;
    }
    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', this.onVisibilityChange);
    }
    this.disconnect();
  }
}
