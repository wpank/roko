/**
 * bootstrap.ts — three-step portal startup: sign in, fetch snapshot, stream.
 *
 * Step 1: Exchange the launch token from the URL fragment for a session cookie.
 * Step 2: Fetch GET /api/statehub/snapshot to get the materialized state and
 *         its cursor (the next sequence number in hex).
 * Step 3: Open the SSE stream from lastEventId = cursor - 1 (decimal).
 *
 * The cursor from the snapshot is the *next* sequence number in hex (e.g. "0x1f"
 * means events 0..30 are already included). `/api/events?lastEventId=N` replays
 * from N+1, so we pass N = cursor - 1. "0x0" means the ring is empty; open the
 * stream with no lastEventId.
 *
 * A `snapshot_rebased` stream event means the server's ring was reset; refetch
 * the snapshot and reopen the stream from the new cursor.
 */

import type { WireDashboardEvent, WireDashboardSnapshot, WireStateHubSnapshotResponse } from '@/api/contracts';

// ---------------------------------------------------------------------------
// Exported types defined here (T09 will make the SSE client import these)
// ---------------------------------------------------------------------------

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

/**
 * Events that flow through the live stream callback.
 *
 * The SSE client emits a synthetic `{type:'snapshot'}` when it receives a
 * `event: gap` frame from the server, so callers get a single dispatch path.
 */
export type PortalStreamEvent =
  | WireDashboardEvent
  | { type: 'snapshot'; snapshot: WireDashboardSnapshot; cursor: string };

// ---------------------------------------------------------------------------
// Session result
// ---------------------------------------------------------------------------

export type SessionResult = 'none' | 'signed-in' | 'rejected' | 'unavailable';

/**
 * Shown in the UI when the user needs to sign in by opening the portal link
 * printed by `roko serve`.
 */
export const SIGN_IN_HINT =
  'Not signed in to this roko serve. Open the portal link it printed (the one ending in #token=…).';

// ---------------------------------------------------------------------------
// Fragment / cursor helpers
// ---------------------------------------------------------------------------

/**
 * Extract the `token` parameter from a URL fragment string.
 *
 * @param hash - The `window.location.hash` value, e.g. `"#token=abc"` or
 *               `"#a=1&token=abc"`.  The leading `#` is optional.
 * @returns The token string, or `null` when absent or empty.
 */
export function parseLaunchToken(hash: string): string | null {
  const fragment = hash.startsWith('#') ? hash.slice(1) : hash;
  if (!fragment) return null;
  const params = new URLSearchParams(fragment);
  const token = params.get('token');
  return token !== null && token.length > 0 ? token : null;
}

/**
 * Convert a hex next-sequence cursor to the decimal `lastEventId` query
 * parameter value used by `/api/events`.
 *
 * The snapshot's `cursor` field holds the *next* sequence number in hex.
 * The stream endpoint replays from `lastEventId + 1`, so we pass
 * `cursor - 1` as a decimal string.
 *
 * @param cursor - Hex cursor from the snapshot response, e.g. `"0x1f"`.
 * @returns Decimal string, e.g. `"30"`, or `null` when the cursor is zero,
 *          absent, or unparsable (open the stream from the beginning).
 */
export function cursorToLastEventId(cursor: string | null | undefined): string | null {
  if (!cursor) return null;
  if (!cursor.startsWith('0x')) return null;
  const hexPart = cursor.slice(2);
  if (!hexPart) return null;
  const n = parseInt(hexPart, 16);
  if (Number.isNaN(n) || n <= 0) return null;
  return String(n - 1);
}

// ---------------------------------------------------------------------------
// Live-state dependencies (injected for testability)
// ---------------------------------------------------------------------------

export interface LiveDeps {
  /** Current `window.location.hash` (with or without the leading `#`). */
  hash: string;

  /** POST /api/auth/session — returns the HTTP status code. */
  postSession(token: string): Promise<number>;

  /** Remove the `#token=…` fragment from the address bar. */
  dropFragment(): void;

  /** Called once the sign-in step resolves. */
  setSession(result: SessionResult): void;

  /** GET /api/statehub/snapshot */
  fetchSnapshot(): Promise<WireStateHubSnapshotResponse>;

  /**
   * Open the SSE stream.
   *
   * @param lastEventId - decimal cursor for `?lastEventId=`, or `null` to
   *                      start from the beginning.
   * @param onEvent     - invoked for every `PortalStreamEvent`.
   * @param onStatus    - invoked when the connection status changes.
   * @returns Handle that allows closing the stream.
   */
  openStream(
    lastEventId: string | null,
    onEvent: (e: PortalStreamEvent) => void,
    onStatus: (s: ConnectionStatus) => void,
  ): { close(): void };

  /** Install a full snapshot (replacing all state). */
  replace(snapshot: WireDashboardSnapshot): void;

  /** Fold one incremental dashboard event into the state. */
  apply(event: WireDashboardEvent): void;

  /** Update the UI connection-status indicator. */
  setStatus(s: ConnectionStatus): void;

  /** Await `ms` milliseconds (injectable so tests can resolve instantly). */
  sleep(ms: number): Promise<void>;
}

// ---------------------------------------------------------------------------
// startLiveState
// ---------------------------------------------------------------------------

const INITIAL_RETRY_MS = 1_000;
const MAX_RETRY_MS = 16_000;

/**
 * Drive the three-step portal startup and the live-streaming loop.
 *
 * Returns a handle whose `stop()` method closes the stream and prevents any
 * further retries or reconnections.
 */
export function startLiveState(deps: LiveDeps): { stop(): void } {
  let stopped = false;
  let stream: { close(): void } | null = null;

  // ── Step 1: session ────────────────────────────────────────────────────────

  async function doSession(): Promise<void> {
    const token = parseLaunchToken(deps.hash);

    if (!token) {
      deps.setSession('none');
      return;
    }

    let result: SessionResult;
    try {
      const status = await deps.postSession(token);
      if (status === 204) {
        result = 'signed-in';
      } else if (status === 404 || status === 405) {
        result = 'unavailable';
      } else {
        result = 'rejected';
      }
    } catch {
      result = 'rejected';
    }

    // Drop the fragment from the address bar regardless of outcome.
    deps.dropFragment();
    deps.setSession(result);
  }

  // ── Steps 2+3: fetch snapshot then open stream (with retry backoff) ────────

  async function fetchAndOpen(retryDelayMs: number): Promise<void> {
    if (stopped) return;

    let resp: WireStateHubSnapshotResponse;
    try {
      resp = await deps.fetchSnapshot();
    } catch {
      if (stopped) return;
      deps.setStatus('error');
      await deps.sleep(retryDelayMs);
      if (!stopped) {
        await fetchAndOpen(Math.min(retryDelayMs * 2, MAX_RETRY_MS));
      }
      return;
    }

    if (stopped) return;

    // Install the snapshot BEFORE opening the stream.
    deps.replace(resp.state);

    if (stopped) return;

    const lastEventId = cursorToLastEventId(resp.cursor);

    stream = deps.openStream(
      lastEventId,
      (event: PortalStreamEvent) => {
        if (stopped) return;

        if (event.type === 'snapshot') {
          // Synthetic gap-recovery event from the SSE client.
          deps.replace(
            (event as { type: 'snapshot'; snapshot: WireDashboardSnapshot; cursor: string })
              .snapshot,
          );
        } else if (event.type === 'snapshot_rebased') {
          // Server reset its ring — close and start over.
          if (stream) {
            stream.close();
            stream = null;
          }
          void fetchAndOpen(INITIAL_RETRY_MS);
        } else {
          deps.apply(event as WireDashboardEvent);
        }
      },
      (status: ConnectionStatus) => {
        if (!stopped) {
          deps.setStatus(status);
        }
      },
    );
  }

  // ── Entry point ────────────────────────────────────────────────────────────

  async function run(): Promise<void> {
    await doSession();
    if (stopped) return;
    await fetchAndOpen(INITIAL_RETRY_MS);
  }

  void run();

  return {
    stop(): void {
      stopped = true;
      if (stream) {
        stream.close();
        stream = null;
      }
    },
  };
}
