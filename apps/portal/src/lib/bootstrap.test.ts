import { describe, it, expect, vi } from 'vitest';
import {
  parseLaunchToken,
  cursorToLastEventId,
  startLiveState,
  SIGN_IN_HINT,
  type ConnectionStatus,
  type LiveDeps,
  type PortalStreamEvent,
  type SessionResult,
} from './bootstrap';
import type { WireDashboardEvent, WireDashboardSnapshot, WireStateHubSnapshotResponse } from '@/api/contracts';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Flush all queued microtasks (up to n rounds). */
async function flush(n = 20): Promise<void> {
  for (let i = 0; i < n; i++) {
    await Promise.resolve();
  }
}

const EMPTY_SNAPSHOT: WireDashboardSnapshot = {
  plans: {},
  tasks: {},
  agents: {},
  gates: [],
  errors: [],
  stats: {
    cost_usd_total: 0,
    total_input_tokens: 0,
    total_output_tokens: 0,
  },
};

function makeSnapshotResponse(cursor = '0x1f'): WireStateHubSnapshotResponse {
  return { cursor, state: EMPTY_SNAPSHOT };
}

interface FakeStream {
  close: () => void;
  emit: (event: PortalStreamEvent) => void;
  emitStatus: (s: ConnectionStatus) => void;
}

interface FakeDepsOpts {
  hash?: string;
  postSessionFn?: (token: string) => Promise<number>;
  fetchSnapshotFn?: () => Promise<WireStateHubSnapshotResponse>;
  sleepFn?: (ms: number) => Promise<void>;
}

interface FakeDeps extends LiveDeps {
  calls: {
    postSession: string[];
    dropFragment: number;
    setSession: SessionResult[];
    fetchSnapshot: number;
    replace: WireDashboardSnapshot[];
    apply: WireDashboardEvent[];
    setStatus: ConnectionStatus[];
    openStream: Array<{ lastEventId: string | null }>;
    sleep: number[];
  };
  streams: FakeStream[];
}

function makeDeps(opts: FakeDepsOpts = {}): FakeDeps {
  const calls: FakeDeps['calls'] = {
    postSession: [],
    dropFragment: 0,
    setSession: [],
    fetchSnapshot: 0,
    replace: [],
    apply: [],
    setStatus: [],
    openStream: [],
    sleep: [],
  };
  const streams: FakeStream[] = [];

  const deps: FakeDeps = {
    hash: opts.hash ?? '',
    calls,
    streams,

    postSession: opts.postSessionFn ?? (async (token: string) => {
      calls.postSession.push(token);
      return 204;
    }),

    dropFragment: () => {
      calls.dropFragment++;
    },

    setSession: (result: SessionResult) => {
      calls.setSession.push(result);
    },

    fetchSnapshot: opts.fetchSnapshotFn ?? (async () => {
      calls.fetchSnapshot++;
      return makeSnapshotResponse();
    }),

    openStream: (
      lastEventId: string | null,
      onEvent: (e: PortalStreamEvent) => void,
      onStatus: (s: ConnectionStatus) => void,
    ) => {
      calls.openStream.push({ lastEventId });
      const stream: FakeStream = {
        close: vi.fn(),
        emit: (e) => onEvent(e),
        emitStatus: (s) => onStatus(s),
      };
      streams.push(stream);
      return stream;
    },

    replace: (snapshot: WireDashboardSnapshot) => {
      calls.replace.push(snapshot);
    },

    apply: (event: WireDashboardEvent) => {
      calls.apply.push(event);
    },

    setStatus: (s: ConnectionStatus) => {
      calls.setStatus.push(s);
    },

    sleep: opts.sleepFn ?? (async (ms: number) => {
      calls.sleep.push(ms);
    }),
  };

  return deps;
}

// ---------------------------------------------------------------------------
// parseLaunchToken
// ---------------------------------------------------------------------------

describe('parseLaunchToken', () => {
  it('returns the token from a bare fragment', () => {
    expect(parseLaunchToken('#token=abc123')).toBe('abc123');
  });

  it('returns the token when other params are present', () => {
    expect(parseLaunchToken('#a=1&token=xyz&b=2')).toBe('xyz');
  });

  it('returns null when fragment is empty', () => {
    expect(parseLaunchToken('')).toBeNull();
  });

  it('returns null when token param is absent', () => {
    expect(parseLaunchToken('#foo=bar')).toBeNull();
  });

  it('returns null when token param is empty', () => {
    expect(parseLaunchToken('#token=')).toBeNull();
  });

  it('parses correctly without leading #', () => {
    expect(parseLaunchToken('token=hello')).toBe('hello');
  });
});

// ---------------------------------------------------------------------------
// cursorToLastEventId
// ---------------------------------------------------------------------------

describe('cursorToLastEventId', () => {
  it('converts "0x1f" to "30" (31 - 1)', () => {
    expect(cursorToLastEventId('0x1f')).toBe('30');
  });

  it('converts "0x1" to "0"', () => {
    expect(cursorToLastEventId('0x1')).toBe('0');
  });

  it('returns null for "0x0" (ring empty)', () => {
    expect(cursorToLastEventId('0x0')).toBeNull();
  });

  it('returns null for null', () => {
    expect(cursorToLastEventId(null)).toBeNull();
  });

  it('returns null for undefined', () => {
    expect(cursorToLastEventId(undefined)).toBeNull();
  });

  it('returns null for a non-hex string (no 0x prefix)', () => {
    expect(cursorToLastEventId('garbage')).toBeNull();
  });

  it('returns null for "0x" with no digits', () => {
    expect(cursorToLastEventId('0x')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SIGN_IN_HINT
// ---------------------------------------------------------------------------

describe('SIGN_IN_HINT', () => {
  it('contains "#token=…" to guide the user', () => {
    expect(SIGN_IN_HINT).toContain('#token=');
  });
});

// ---------------------------------------------------------------------------
// startLiveState — session step
// ---------------------------------------------------------------------------

describe('startLiveState: session with no token', () => {
  it('calls setSession("none") immediately without contacting the server', async () => {
    const deps = makeDeps({ hash: '' });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toEqual(['none']);
    expect(deps.calls.postSession).toHaveLength(0);
  });

  it('does NOT call dropFragment when no token was present', async () => {
    const deps = makeDeps({ hash: '' });
    startLiveState(deps);
    await flush();
    expect(deps.calls.dropFragment).toBe(0);
  });
});

describe('startLiveState: session outcomes', () => {
  it('204 → signed-in', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (token) => { deps.calls.postSession.push(token); return 204; },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toContain('signed-in');
  });

  it('404 → unavailable', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (token) => { deps.calls.postSession.push(token); return 404; },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toContain('unavailable');
  });

  it('405 → unavailable', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (token) => { deps.calls.postSession.push(token); return 405; },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toContain('unavailable');
  });

  it('401 → rejected', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (token) => { deps.calls.postSession.push(token); return 401; },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toContain('rejected');
  });

  it('thrown error → rejected', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (_token) => { throw new Error('network error'); },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.setSession).toContain('rejected');
  });

  it('calls dropFragment whenever a token was present, regardless of outcome', async () => {
    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (_token) => { deps.calls.postSession.push('tok1'); return 500; },
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.dropFragment).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// startLiveState — ordering: session before snapshot
// ---------------------------------------------------------------------------

describe('startLiveState: session settles before snapshot is fetched', () => {
  it('does not call fetchSnapshot before setSession is called', async () => {
    let sessionResolve!: (n: number) => void;
    const sessionPromise = new Promise<number>((r) => { sessionResolve = r; });

    const deps = makeDeps({
      hash: '#token=tok1',
      postSessionFn: async (_token) => { deps.calls.postSession.push('tok1'); return sessionPromise; },
    });

    startLiveState(deps);
    await flush(5);

    // postSession still pending — fetchSnapshot must not have been called yet
    expect(deps.calls.fetchSnapshot).toBe(0);

    // Now resolve the session
    sessionResolve(204);
    await flush();
    expect(deps.calls.fetchSnapshot).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// startLiveState — replace before openStream
// ---------------------------------------------------------------------------

describe('startLiveState: replace is called before openStream', () => {
  it('installs the snapshot before opening the stream', async () => {
    const order: string[] = [];
    const baseFetch = makeDeps().fetchSnapshot;

    const deps = makeDeps({
      hash: '',
      fetchSnapshotFn: async () => {
        const r = await baseFetch();
        return r;
      },
    });

    const origReplace = deps.replace.bind(deps);
    deps.replace = (snap) => { order.push('replace'); origReplace(snap); };

    const origOpen = deps.openStream.bind(deps);
    deps.openStream = (lid, onE, onS) => { order.push('openStream'); return origOpen(lid, onE, onS); };

    startLiveState(deps);
    await flush();

    expect(order.indexOf('replace')).toBeLessThan(order.indexOf('openStream'));
  });
});

// ---------------------------------------------------------------------------
// startLiveState — cursor is converted before being passed to openStream
// ---------------------------------------------------------------------------

describe('startLiveState: cursor conversion', () => {
  it('passes decimal lastEventId derived from hex cursor', async () => {
    const deps = makeDeps({
      hash: '',
      fetchSnapshotFn: async () => makeSnapshotResponse('0x1f'),
    });
    startLiveState(deps);
    await flush();
    // 0x1f = 31; lastEventId = 31 - 1 = 30
    expect(deps.calls.openStream[0]?.lastEventId).toBe('30');
  });

  it('passes null lastEventId when cursor is "0x0"', async () => {
    const deps = makeDeps({
      hash: '',
      fetchSnapshotFn: async () => makeSnapshotResponse('0x0'),
    });
    startLiveState(deps);
    await flush();
    expect(deps.calls.openStream[0]?.lastEventId).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// startLiveState — gap event → replace
// ---------------------------------------------------------------------------

describe('startLiveState: gap frame → replace', () => {
  it('calls replace when the stream emits a synthetic snapshot event', async () => {
    const deps = makeDeps({ hash: '' });
    startLiveState(deps);
    await flush();

    const stream = deps.streams[0];
    expect(stream).toBeDefined();

    const snap2: WireDashboardSnapshot = { ...EMPTY_SNAPSHOT, errors: [{ message: 'gap', ts_millis: 1 }] };
    stream.emit({ type: 'snapshot', snapshot: snap2, cursor: '0x5' });
    await flush();

    const replaces = deps.calls.replace;
    // First replace is the initial snapshot; second is the gap snapshot
    expect(replaces.length).toBeGreaterThanOrEqual(2);
    expect(replaces[replaces.length - 1]).toBe(snap2);
  });
});

// ---------------------------------------------------------------------------
// startLiveState — snapshot_rebased → refetch and reopen
// ---------------------------------------------------------------------------

describe('startLiveState: snapshot_rebased → refetch and reopen', () => {
  it('closes the stream, refetches, and reopens when snapshot_rebased arrives', async () => {
    const deps = makeDeps({ hash: '' });
    startLiveState(deps);
    await flush();

    expect(deps.calls.fetchSnapshot).toBe(1);
    expect(deps.streams).toHaveLength(1);

    deps.streams[0].emit({ type: 'snapshot_rebased', revision: 2 });
    await flush();

    // Should have fetched a second time and opened a second stream
    expect(deps.calls.fetchSnapshot).toBeGreaterThanOrEqual(2);
    expect(deps.streams.length).toBeGreaterThanOrEqual(2);
    expect(deps.streams[0].close).toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// startLiveState — fetch failure → error status, then retry
// ---------------------------------------------------------------------------

describe('startLiveState: fetch failure → error then success', () => {
  it('sets status error on fetch failure and retries', async () => {
    let attempt = 0;
    const deps = makeDeps({
      hash: '',
      fetchSnapshotFn: async () => {
        deps.calls.fetchSnapshot++;
        attempt++;
        if (attempt === 1) throw new Error('fetch failed');
        return makeSnapshotResponse();
      },
      sleepFn: async (ms) => { deps.calls.sleep.push(ms); /* instant */ },
    });

    startLiveState(deps);
    await flush();

    expect(deps.calls.setStatus).toContain('error');
    // After retry, the stream should open
    expect(deps.calls.openStream.length).toBeGreaterThanOrEqual(1);
  });

  it('uses 1s initial backoff then doubles', async () => {
    let attempt = 0;
    const deps = makeDeps({
      hash: '',
      fetchSnapshotFn: async () => {
        deps.calls.fetchSnapshot++;
        attempt++;
        if (attempt <= 2) throw new Error('fail');
        return makeSnapshotResponse();
      },
      sleepFn: async (ms) => { deps.calls.sleep.push(ms); },
    });

    startLiveState(deps);
    await flush();

    expect(deps.calls.sleep[0]).toBe(1000);
    expect(deps.calls.sleep[1]).toBe(2000);
  });
});

// ---------------------------------------------------------------------------
// startLiveState — stop() prevents reopening
// ---------------------------------------------------------------------------

describe('startLiveState: stop() prevents reopening', () => {
  it('closes the open stream on stop()', async () => {
    const deps = makeDeps({ hash: '' });
    const handle = startLiveState(deps);
    await flush();

    expect(deps.streams).toHaveLength(1);
    handle.stop();

    expect((deps.streams[0].close as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThanOrEqual(1);
  });

  it('does not reopen after stop() even if snapshot_rebased arrives', async () => {
    const deps = makeDeps({ hash: '' });
    const handle = startLiveState(deps);
    await flush();

    const initialFetches = deps.calls.fetchSnapshot;

    handle.stop();
    deps.streams[0]?.emit({ type: 'snapshot_rebased', revision: 1 });
    await flush();

    // No additional fetches after stop
    expect(deps.calls.fetchSnapshot).toBe(initialFetches);
  });
});

// ---------------------------------------------------------------------------
// startLiveState — incremental events go to apply
// ---------------------------------------------------------------------------

describe('startLiveState: incremental events go to apply', () => {
  it('routes ordinary events to deps.apply', async () => {
    const deps = makeDeps({ hash: '' });
    startLiveState(deps);
    await flush();

    const ev: WireDashboardEvent = { type: 'error', message: 'boom' };
    deps.streams[0].emit(ev);
    await flush();

    expect(deps.calls.apply).toContain(ev);
  });
});
