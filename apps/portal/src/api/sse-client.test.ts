/**
 * SseClient: the connection lifecycle of GET /api/events (spec-ddc287, PB-005):
 * cursor replay, exponential backoff, gap recovery, the keepalive watchdog,
 * event parsing and the connection status.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SseClient } from './sse-client';
import type { ConnectionStatus, PortalStreamEvent } from '@/lib/bootstrap';
import type { WireDashboardSnapshot } from '@/api/contracts';

interface Frame {
  data: string;
  lastEventId: string;
}

/** Stands in for the browser's EventSource; tests drive it with open/send/gap/drop. */
class FakeEventSource {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSED = 2;

  readyState = FakeEventSource.CONNECTING;
  onopen: (() => void) | null = null;
  onmessage: ((frame: Frame) => void) | null = null;
  onerror: (() => void) | null = null;
  private gapListener: ((frame: Frame) => void) | null = null;

  constructor(readonly url: string) {
    opened.push(this);
  }

  addEventListener(type: string, listener: (frame: Frame) => void): void {
    if (type === 'gap') this.gapListener = listener;
  }

  close(): void {
    this.readyState = FakeEventSource.CLOSED;
  }

  open(): void {
    this.readyState = FakeEventSource.OPEN;
    this.onopen?.();
  }

  send(data: unknown, lastEventId = ''): void {
    this.onmessage?.({ data: typeof data === 'string' ? data : JSON.stringify(data), lastEventId });
  }

  gap(payload: unknown, lastEventId: string): void {
    this.gapListener?.({ data: JSON.stringify(payload), lastEventId });
  }

  /** The connection drops; the browser would retry on its own (CONNECTING). */
  drop(): void {
    this.readyState = FakeEventSource.CONNECTING;
    this.onerror?.();
  }
}

let opened: FakeEventSource[] = [];

const latest = () => opened[opened.length - 1]!;
const cursorOf = (es: FakeEventSource) => new URL(es.url).searchParams.get('lastEventId');

const SNAPSHOT: WireDashboardSnapshot = {
  plans: {},
  tasks: {},
  agents: {},
  gates: [],
  errors: [],
  stats: { cost_usd_total: 0, total_input_tokens: 0, total_output_tokens: 0 },
};

function start(lastEventId: string | null = null) {
  const events: PortalStreamEvent[] = [];
  const statuses: ConnectionStatus[] = [];
  const client = new SseClient('', { lastEventId });
  client.connect((e) => events.push(e), (s) => statuses.push(s));
  return { client, events, statuses };
}

beforeEach(() => {
  opened = [];
  vi.useFakeTimers();
  vi.stubGlobal('EventSource', FakeEventSource);
  vi.spyOn(console, 'warn').mockImplementation(() => {});
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('SseClient', () => {
  it('connects to /api/events on the page origin, from the snapshot cursor when there is one', () => {
    const { statuses } = start('30');
    expect(new URL(latest().url).pathname).toBe('/api/events');
    expect(cursorOf(latest())).toBe('30');
    latest().open();
    expect(statuses).toEqual(['connecting', 'connected']);

    start();
    expect(new URL(latest().url).searchParams.has('lastEventId')).toBe(false);
  });

  it('replays from the last event id after a reconnect', () => {
    start('30');
    latest().open();
    latest().send({ type: 'plan_started', plan_id: 'hello', tasks_total: 1 }, '41');
    latest().drop();
    vi.advanceTimersByTime(1_000);
    expect(opened).toHaveLength(2);
    expect(cursorOf(latest())).toBe('41');
  });

  it('backs off 1 s, 2 s, 4 s, 8 s, then 16 s between failed attempts', () => {
    start();
    for (const delay of [1_000, 2_000, 4_000, 8_000, 16_000, 16_000]) {
      const count = opened.length;
      latest().drop();
      vi.advanceTimersByTime(delay - 1);
      expect(opened).toHaveLength(count);
      vi.advanceTimersByTime(1);
      expect(opened).toHaveLength(count + 1);
    }
  });

  it('starts the backoff over once a connection opens', () => {
    start();
    latest().drop();
    vi.advanceTimersByTime(1_000);
    latest().drop();
    vi.advanceTimersByTime(2_000);
    latest().open();
    latest().drop();
    vi.advanceTimersByTime(1_000);
    expect(opened).toHaveLength(4);
  });

  it('replaces the state with the snapshot a gap frame carries, and resumes after it', () => {
    const { events } = start();
    latest().open();
    latest().gap({ missed_events: 12, last_materialized_seq: 840, snapshot: SNAPSHOT }, '840');
    expect(events).toEqual([{ type: 'snapshot', snapshot: SNAPSHOT, cursor: '840' }]);

    latest().drop();
    vi.advanceTimersByTime(1_000);
    expect(cursorOf(latest())).toBe('840');
  });

  it('reconnects when nothing arrives for 60 s, and each event restarts that window', () => {
    start();
    latest().open();
    vi.advanceTimersByTime(50_000);
    latest().send({ type: 'plan_started', plan_id: 'hello' }, '7');
    vi.advanceTimersByTime(59_999);
    expect(opened).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(opened).toHaveLength(2);
    expect(opened[0]!.readyState).toBe(FakeEventSource.CLOSED);
    expect(cursorOf(latest())).toBe('7');
  });

  it('passes typed events through and drops frames it cannot read', () => {
    const { events } = start();
    latest().open();
    latest().send('not json');
    latest().send({ plan_id: 'hello' });
    latest().send({ type: 'plan_started', plan_id: 'hello', tasks_total: 2 }, '3');
    expect(events).toEqual([{ type: 'plan_started', plan_id: 'hello', tasks_total: 2 }]);
  });

  it('retries at once when asked, instead of waiting out the backoff', () => {
    const { client } = start();
    latest().drop();
    client.forceReconnect();
    expect(opened).toHaveLength(2);
    vi.advanceTimersByTime(1_000);
    expect(opened).toHaveLength(2);
  });

  it('reports each status change and stays down after disconnect()', () => {
    const { client, statuses } = start();
    latest().open();
    latest().drop();
    vi.advanceTimersByTime(1_000);
    latest().open();
    client.disconnect();
    vi.advanceTimersByTime(120_000);

    expect(opened).toHaveLength(2);
    expect(latest().readyState).toBe(FakeEventSource.CLOSED);
    expect(statuses).toEqual(['connecting', 'connected', 'disconnected', 'connecting', 'connected', 'disconnected']);
  });
});
