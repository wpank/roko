import { expect, test } from '@playwright/test';
import {
  CSRF_HEADER,
  RokoApi,
  SESSION_PATH,
  goToLogin,
  loginHref,
  routeToLoginIfLoggedOut,
  type LoginRouting,
  type SessionProbe,
} from '../../src/transport/api';
import { SseAdapter, backoffDelayMs } from '../../src/transport/sse';

/**
 * The client half of session auth (S10 §6 T14, S11 §4.6, 9331), run in Node with stubbed
 * `fetch`, `EventSource`, timers and `document`, in the style of `transport-contracts.spec.ts`.
 */

test.describe.configure({ mode: 'serial' });

/** Let every pending promise settle; `setImmediate` is not faked, unlike the timers. */
function settle(): Promise<void> {
  return new Promise((resolve) => setImmediate(resolve));
}

type FetchCall = { url: string; init: RequestInit };

/** Replace `fetch` for the duration of `run`, answering every call with `answer`. */
async function withFetch(
  answer: (url: string) => Response,
  run: (calls: FetchCall[]) => Promise<void>,
): Promise<void> {
  const original = globalThis.fetch;
  const calls: FetchCall[] = [];
  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    calls.push({ url, init: init ?? {} });
    return answer(url);
  }) as typeof fetch;
  try {
    await run(calls);
  } finally {
    globalThis.fetch = original;
  }
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

const LOGGED_OUT: SessionProbe = {
  authenticated: false,
  login: 'passphrase',
  showcase_mode: true,
  scopes: [],
  expires_at: null,
};

function fakeRouting(pathname: string): LoginRouting & { visited: string[] } {
  const visited: string[] = [];
  return { location: { pathname, search: '?bundle=b-1' }, assign: (href) => visited.push(href), visited };
}

test('a non-GET carries the CSRF header, and every request the same-origin cookie', async () => {
  await withFetch(() => json({ ok: true }), async (calls) => {
    const client = new RokoApi('http://serve.test');
    await client.post('/api/showcase/runs', { arm: 'roko_fixed' });
    await client.delete('/api/auth/session');
    await client.get('/api/showcase/manifest');

    const [post, del, get] = calls.map((call) => call.init);
    const header = (init: RequestInit) => (init.headers as Record<string, string>)[CSRF_HEADER];
    expect(header(post)).toBe('1');
    expect(header(del)).toBe('1');
    expect(header(get)).toBeUndefined();
    for (const init of [post, del, get]) expect(init.credentials).toBe('same-origin');
  });
});

test('a 401 calls onUnauthorized, except from the session endpoint itself', async () => {
  await withFetch(() => json({ error: 'unauthorized' }, 401), async () => {
    let unauthorized = 0;
    const client = new RokoApi('http://serve.test', { onUnauthorized: () => { unauthorized += 1; } });
    const result = await client.get('/api/showcase/manifest');
    expect(result.ok).toBe(false);
    expect(unauthorized).toBe(1);
    await client.get(SESSION_PATH);
    expect(unauthorized).toBe(1);
  });
});

test('a logged-out showcase visitor goes to the login page, with the way back', async () => {
  expect(loginHref('/demo/p1/head-to-head', '/demo/')).toBe(
    '/demo/login?next=%2Fdemo%2Fp1%2Fhead-to-head',
  );
  expect(loginHref('/p2/audits', '/')).toBe('/login?next=%2Fp2%2Faudits');

  await withFetch(() => json(LOGGED_OUT), async (calls) => {
    const routing = fakeRouting('/demo/p1/head-to-head');
    expect(await routeToLoginIfLoggedOut('http://serve.test', routing)).toBe(true);
    expect(calls[0].url).toBe(`http://serve.test${SESSION_PATH}`);
    expect(routing.visited).toEqual([
      `${loginHref('/demo/p1/head-to-head?bundle=b-1')}`,
    ]);
  });

  // A server with token logins, or a live session, is not sent to the passphrase page.
  for (const probe of [{ ...LOGGED_OUT, login: 'token' }, { ...LOGGED_OUT, authenticated: true }]) {
    await withFetch(() => json(probe), async () => {
      const routing = fakeRouting('/demo/');
      expect(await routeToLoginIfLoggedOut('http://serve.test', routing)).toBe(false);
      expect(routing.visited).toEqual([]);
    });
  }

  // The login page never sends a visitor to itself.
  const onLogin = fakeRouting('/demo/login');
  goToLogin(onLogin);
  expect(onLogin.visited).toEqual([]);
});

class FakeEventSource {
  static instances: FakeEventSource[] = [];

  readonly url: string;
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string | URL) {
    this.url = String(url);
    FakeEventSource.instances.push(this);
  }

  addEventListener() {}

  close() {
    this.closed = true;
  }
}

/** Run with fake `EventSource` and timers: delays are recorded, and `fire` runs a pending one. */
async function withFakeStream(
  run: (timers: { delays: number[]; fire: () => void }) => Promise<void>,
): Promise<void> {
  const originalEventSource = globalThis.EventSource;
  const originalSetTimeout = globalThis.setTimeout;
  const originalClearTimeout = globalThis.clearTimeout;
  const pending: Array<{ id: number; callback: () => void }> = [];
  const delays: number[] = [];
  let next = 1;
  globalThis.EventSource = FakeEventSource as unknown as typeof EventSource;
  globalThis.setTimeout = ((callback: () => void, delay?: number) => {
    delays.push(delay ?? 0);
    const id = next++;
    pending.push({ id, callback });
    return id;
  }) as unknown as typeof setTimeout;
  globalThis.clearTimeout = ((id: number) => {
    const at = pending.findIndex((timer) => timer.id === id);
    if (at >= 0) pending.splice(at, 1);
  }) as unknown as typeof clearTimeout;
  const fire = () => pending.shift()?.callback();
  FakeEventSource.instances = [];
  try {
    await run({ delays, fire });
  } finally {
    globalThis.EventSource = originalEventSource;
    globalThis.setTimeout = originalSetTimeout;
    globalThis.clearTimeout = originalClearTimeout;
  }
}

test('reconnects back off 1, 2, 4, 8 s, then every 15 s', async () => {
  expect([1, 2, 3, 4, 5, 6, 7].map((attempt) => backoffDelayMs(attempt))).toEqual([
    1000, 2000, 4000, 8000, 15_000, 15_000, 15_000,
  ]);

  await withFakeStream(async ({ delays, fire }) => {
    const adapter = new SseAdapter({
      url: 'http://serve.test/api/showcase/stream',
      onEvent: () => {},
      onStatusChange: () => {},
      maxRetries: Number.POSITIVE_INFINITY,
      probeSession: async () => ({ ...LOGGED_OUT, authenticated: true }),
    });
    try {
      adapter.connect();
      for (let attempt = 0; attempt < 6; attempt += 1) {
        FakeEventSource.instances.at(-1)?.onerror?.();
        await settle();
        fire();
      }
      expect(delays).toEqual([1000, 2000, 4000, 8000, 15_000, 15_000]);
      expect(FakeEventSource.instances).toHaveLength(7);
    } finally {
      adapter.destroy();
    }
  });
});

test('a stream error with no session stops retrying and goes to the login page', async () => {
  await withFakeStream(async ({ delays }) => {
    let sentToLogin = 0;
    const statuses: string[] = [];
    const adapter = new SseAdapter({
      url: 'http://serve.test/api/showcase/stream',
      onEvent: () => {},
      onStatusChange: (status) => statuses.push(status),
      probeSession: async () => LOGGED_OUT,
      onUnauthenticated: () => { sentToLogin += 1; },
    });
    try {
      adapter.connect();
      FakeEventSource.instances[0].onerror?.();
      await settle();
      expect(sentToLogin).toBe(1);
      expect(adapter.status).toBe('failed');
      expect(statuses).toContain('failed');
      expect(delays).toEqual([1000]);
      expect(FakeEventSource.instances).toHaveLength(1);
    } finally {
      adapter.destroy();
    }
  });
});

test('a tab hidden for two minutes closes its stream and reopens it when shown', async () => {
  const originalDocument = (globalThis as { document?: unknown }).document;
  const listeners: Array<() => void> = [];
  const fakeDocument = {
    visibilityState: 'visible' as DocumentVisibilityState,
    addEventListener: (_type: string, listener: () => void) => listeners.push(listener),
    removeEventListener: () => {},
  };
  (globalThis as { document?: unknown }).document = fakeDocument;
  try {
    await withFakeStream(async ({ delays, fire }) => {
      const adapter = new SseAdapter({
        url: 'http://serve.test/api/showcase/stream',
        onEvent: () => {},
        onStatusChange: () => {},
      });
      try {
        adapter.connect();
        FakeEventSource.instances[0].onopen?.();
        fakeDocument.visibilityState = 'hidden';
        listeners.forEach((listener) => listener());
        expect(delays).toEqual([120_000]);
        fire();
        expect(FakeEventSource.instances[0].closed).toBe(true);
        expect(adapter.status).toBe('idle');

        fakeDocument.visibilityState = 'visible';
        listeners.forEach((listener) => listener());
        expect(FakeEventSource.instances).toHaveLength(2);
        expect(adapter.status).toBe('connecting');
      } finally {
        adapter.destroy();
      }
    });
  } finally {
    (globalThis as { document?: unknown }).document = originalDocument;
  }
});
