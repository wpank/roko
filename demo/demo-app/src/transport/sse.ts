export type SseStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'failed';

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

  constructor(config: SseAdapterConfig) {
    this.config = config;
    this.status = 'idle';
    this.lastEventId = null;
    this.retryCount = 0;
    this.retryTimer = null;
    this.es = null;
    this.destroyed = false;
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
      const delay = Math.min(baseMs * 2 ** (this.retryCount - 1), maxMs);
      this.retryTimer = setTimeout(() => this.connect(), delay);
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
    this.disconnect();
  }
}
