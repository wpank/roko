# F3: Add ISFR Data Slice to DataHub + Transport Types

## Context

The demo-app (`demo/demo-app/`) is a plain React SPA (Vite + React 19 + Zustand) that
connects directly to roko-serve via REST + SSE + WebSocket. There is no Tauri layer.

This task wires ISFR data into the frontend's existing architecture:
- **Transport types**: new `ServerEvent` variants for ISFR events
- **DataHub**: ISFR state slice + event handling in `handleServerEvent()`
- **Selectors**: thin hooks for components to consume ISFR state
- **REST client**: dedicated `isfr-api.ts` module using the `api` singleton

Pattern reference: CostDashboard — initial REST fetch + SSE-triggered debounced refetch.

## Files to Create

- `demo/demo-app/src/lib/isfr-api.ts` (NEW)

## Files to Modify

- `demo/demo-app/src/transport/types.ts` — add ISFR ServerEvent variants
- `demo/demo-app/src/app/DataHub.ts` — add ISFR state slice + event handling + fetch actions
- `demo/demo-app/src/data/selectors.ts` — add ISFR selector hooks

## Pre-Check

```bash
# Verify existing transport types
grep -n "ServerEvent" demo/demo-app/src/transport/types.ts | head -5
# Check current DataHub interface fields
grep -n "interface DataHub" demo/demo-app/src/app/DataHub.ts
# See how api singleton is used
grep -rn "import.*api.*from" demo/demo-app/src/lib/ --include="*.ts"
# Check for any existing ISFR references
grep -rn "isfr\|ISFR" demo/demo-app/src/ --include="*.ts"
```

## Implementation

### Step 1: Add ISFR ServerEvent Variants to `types.ts`

In `demo/demo-app/src/transport/types.ts`, add these variants to the `ServerEvent`
union (before the `// System` section near the end):

```typescript
  // ISFR (interest-free secured rate)
  | { type: 'isfr_rate_computed'; compositeBps: number; lendingBps: number;
       structuredBps: number; fundingBps: number; stakingBps: number;
       confidenceBps: number; sourceCount: number; timestampMs: number }
  | { type: 'isfr_source_health_changed'; sourceId: string;
       health: 'healthy' | 'degraded' | 'down'; lastRateBps: number | null }
  | { type: 'isfr_keeper_state_changed'; running: boolean }
```

These match the Rust `ServerEvent` variants added in F1 (`events.rs`), with field names
converted from snake_case to camelCase by the existing `parseServerEvent()` function.

### Step 2: Add ISFR State Slice to DataHub

In `demo/demo-app/src/app/DataHub.ts`:

**2a. Add ISFR types (after `InferenceRecord` interface):**

```typescript
// ── ISFR types ──────────────────────────────────────────────────

export interface IsfrRate {
  compositeBps: number;
  lendingBps: number;
  structuredBps: number;
  fundingBps: number;
  stakingBps: number;
  confidenceBps: number;
  sourceCount: number;
  timestampMs: number;
}

export interface IsfrSource {
  id: string;
  health: 'healthy' | 'degraded' | 'down';
  lastRateBps: number | null;
}

export type IsfrKeeperStatus = 'unknown' | 'running' | 'stopped';
```

**2b. Add ISFR fields to the `DataHub` interface:**

```typescript
  // -- ISFR slice -------------------------------------------------
  isfrCurrentRate: IsfrRate | null;
  isfrHistory: IsfrRate[];       // ring buffer, max 256
  isfrSources: IsfrSource[];
  isfrKeeperStatus: IsfrKeeperStatus;
```

**2c. Add ISFR actions to the `DataHub` interface:**

```typescript
  // -- Actions: ISFR REST fetches ---------------------------------
  fetchIsfrStatus: () => Promise<void>;
  fetchIsfrCurrent: () => Promise<void>;
  fetchIsfrHistory: (limit?: number) => Promise<void>;
  fetchIsfrSources: () => Promise<void>;
```

**2d. Add ring-buffer limit constant:**

```typescript
const MAX_ISFR_HISTORY = 256;
```

**2e. Add initial state (inside `create<DataHub>()`):**

```typescript
  isfrCurrentRate: null,
  isfrHistory: [],
  isfrSources: [],
  isfrKeeperStatus: 'unknown',
```

**2f. Add ISFR event handling in `handleServerEvent` switch:**

```typescript
      case 'isfr_rate_computed':
        set((s) => ({
          isfrCurrentRate: {
            compositeBps: event.compositeBps,
            lendingBps: event.lendingBps,
            structuredBps: event.structuredBps,
            fundingBps: event.fundingBps,
            stakingBps: event.stakingBps,
            confidenceBps: event.confidenceBps,
            sourceCount: event.sourceCount,
            timestampMs: event.timestampMs,
          },
          isfrHistory: [
            ...s.isfrHistory.slice(-(MAX_ISFR_HISTORY - 1)),
            {
              compositeBps: event.compositeBps,
              lendingBps: event.lendingBps,
              structuredBps: event.structuredBps,
              fundingBps: event.fundingBps,
              stakingBps: event.stakingBps,
              confidenceBps: event.confidenceBps,
              sourceCount: event.sourceCount,
              timestampMs: event.timestampMs,
            },
          ],
        }));
        break;

      case 'isfr_source_health_changed':
        set((s) => ({
          isfrSources: s.isfrSources.some((src) => src.id === event.sourceId)
            ? s.isfrSources.map((src) =>
                src.id === event.sourceId
                  ? { ...src, health: event.health, lastRateBps: event.lastRateBps }
                  : src,
              )
            : [
                ...s.isfrSources,
                { id: event.sourceId, health: event.health, lastRateBps: event.lastRateBps },
              ],
        }));
        break;

      case 'isfr_keeper_state_changed':
        set({ isfrKeeperStatus: event.running ? 'running' : 'stopped' });
        break;
```

**2g. Add ISFR REST fetch actions (after existing fetch actions):**

```typescript
  // -- ISFR fetch actions -----------------------------------------

  fetchIsfrStatus: async () => {
    const res = await api.get<{ keeper_running: boolean; sources_count: number;
      current_rate_bps: number | null }>('/api/isfr/status');
    if (res.ok) {
      set({
        isfrKeeperStatus: res.data.keeper_running ? 'running' : 'stopped',
      });
    }
  },

  fetchIsfrCurrent: async () => {
    const res = await api.get<{
      composite_bps: number; lending_bps: number; structured_bps: number;
      funding_bps: number; staking_bps: number; confidence_bps: number;
      source_count: number; timestamp_ms: number;
    }>('/api/isfr/current');
    if (res.ok) {
      set({
        isfrCurrentRate: {
          compositeBps: res.data.composite_bps,
          lendingBps: res.data.lending_bps,
          structuredBps: res.data.structured_bps,
          fundingBps: res.data.funding_bps,
          stakingBps: res.data.staking_bps,
          confidenceBps: res.data.confidence_bps,
          sourceCount: res.data.source_count,
          timestampMs: res.data.timestamp_ms,
        },
      });
    }
  },

  fetchIsfrHistory: async (limit = 50) => {
    const res = await api.get<Array<{
      composite_bps: number; lending_bps: number; structured_bps: number;
      funding_bps: number; staking_bps: number; confidence_bps: number;
      source_count: number; timestamp_ms: number;
    }>>(`/api/isfr/history?limit=${limit}`);
    if (res.ok) {
      set({
        isfrHistory: res.data.map((r) => ({
          compositeBps: r.composite_bps,
          lendingBps: r.lending_bps,
          structuredBps: r.structured_bps,
          fundingBps: r.funding_bps,
          stakingBps: r.staking_bps,
          confidenceBps: r.confidence_bps,
          sourceCount: r.source_count,
          timestampMs: r.timestamp_ms,
        })),
      });
    }
  },

  fetchIsfrSources: async () => {
    const res = await api.get<Array<{
      id: string; health: 'healthy' | 'degraded' | 'down';
      last_rate_bps: number | null;
    }>>('/api/isfr/sources');
    if (res.ok) {
      set({
        isfrSources: res.data.map((s) => ({
          id: s.id,
          health: s.health,
          lastRateBps: s.last_rate_bps,
        })),
      });
    }
  },
```

### Step 3: Add ISFR Selectors

In `demo/demo-app/src/data/selectors.ts`, add after the Bench section:

```typescript
// ── ISFR ────────────────────────────────────────────────────────────

/** Current ISFR composite rate (null until first computation). */
export const useIsfrCurrentRate = () => useDataHub((s) => s.isfrCurrentRate);

/** ISFR rate history ring buffer. */
export const useIsfrHistory = () => useDataHub((s) => s.isfrHistory);

/** ISFR source health list. */
export const useIsfrSources = () => useDataHub((s) => s.isfrSources);

/** ISFR keeper running/stopped/unknown. */
export const useIsfrKeeperStatus = () => useDataHub((s) => s.isfrKeeperStatus);

/** Full ISFR slice for dashboard tiles. */
export const useIsfrSlice = () =>
  useDataHub(
    useShallow((s) => ({
      currentRate: s.isfrCurrentRate,
      history: s.isfrHistory,
      sources: s.isfrSources,
      keeperStatus: s.isfrKeeperStatus,
      fetchIsfrStatus: s.fetchIsfrStatus,
      fetchIsfrCurrent: s.fetchIsfrCurrent,
      fetchIsfrHistory: s.fetchIsfrHistory,
      fetchIsfrSources: s.fetchIsfrSources,
    })),
  );

/** Derived: composite rate as percentage (bps / 100). */
export const useIsfrCompositePercent = () =>
  useDataHub((s) =>
    s.isfrCurrentRate ? s.isfrCurrentRate.compositeBps / 100 : null,
  );

/** Derived: number of healthy sources. */
export const useIsfrHealthySourceCount = () =>
  useDataHub(
    (s) => s.isfrSources.filter((src) => src.health === 'healthy').length,
  );
```

### Step 4: Create `isfr-api.ts` REST Client Module

Create `demo/demo-app/src/lib/isfr-api.ts`:

```typescript
/**
 * ISFR API client — typed wrappers around roko-serve ISFR endpoints.
 *
 * Uses the singleton `api` from transport/api.ts (never throws, returns ApiResult).
 * Components should prefer DataHub fetch actions for state integration; use these
 * directly only for one-off queries or non-reactive contexts.
 */

import { api } from '../transport/api';
import type { ApiResult } from '../transport/api';

// ── Response types (snake_case, matching roko-serve JSON) ────────

export interface IsfrStatusResponse {
  enabled: boolean;
  keeper_running: boolean;
  sources_count: number;
  current_rate_bps: number | null;
  current_confidence_bps: number | null;
  current_epoch: number | null;
  poll_interval_secs: number;
  epoch_duration_secs: number;
}

export interface IsfrRateResponse {
  composite_bps: number;
  lending_bps: number;
  structured_bps: number;
  funding_bps: number;
  staking_bps: number;
  confidence_bps: number;
  source_count: number;
  timestamp_ms: number;
}

export interface IsfrSourceResponse {
  id: string;
  health: 'healthy' | 'degraded' | 'down';
  last_rate_bps: number | null;
  last_updated_ms: number | null;
}

// ── API functions ────────────────────────────────────────────────

/** GET /api/isfr/status — keeper status and config overview. */
export function fetchIsfrStatus(
  signal?: AbortSignal,
): Promise<ApiResult<IsfrStatusResponse>> {
  return api.get<IsfrStatusResponse>('/api/isfr/status', signal);
}

/** GET /api/isfr/current — latest computed composite rate. */
export function fetchIsfrCurrent(
  signal?: AbortSignal,
): Promise<ApiResult<IsfrRateResponse>> {
  return api.get<IsfrRateResponse>('/api/isfr/current', signal);
}

/** GET /api/isfr/history?limit=N — historical rate samples. */
export function fetchIsfrHistory(
  limit = 50,
  signal?: AbortSignal,
): Promise<ApiResult<IsfrRateResponse[]>> {
  return api.get<IsfrRateResponse[]>(
    `/api/isfr/history?limit=${limit}`,
    signal,
  );
}

/** GET /api/isfr/sources — all configured sources with health. */
export function fetchIsfrSources(
  signal?: AbortSignal,
): Promise<ApiResult<IsfrSourceResponse[]>> {
  return api.get<IsfrSourceResponse[]>('/api/isfr/sources', signal);
}

// ── SSE stream URL (for dedicated ISFR-only EventSource) ─────────

/**
 * Returns the URL for the ISFR-filtered SSE stream (F2).
 * Use with SseAdapter or a raw EventSource for dedicated ISFR streaming
 * separate from the main /api/events stream.
 */
export function isfrStreamUrl(): string {
  return `${api.baseUrl}/api/isfr/stream`;
}
```

### Step 5: Wire SSE Event Types

In `demo/demo-app/src/transport/sse.ts`, add ISFR event types to the
`KNOWN_SSE_EVENT_TYPES` array so the SseAdapter registers listeners:

```typescript
  'isfr_rate_computed',
  'isfr_source_health_changed',
  'isfr_keeper_state_changed',
```

## Verification

```bash
cd demo/demo-app

# Type-check (no build required — just verify types compile)
npx tsc --noEmit

# Verify the new file resolves
node -e "require('./src/lib/isfr-api.ts')" 2>&1 | grep -v "Cannot use import"

# Run dev server and check no runtime errors
npm run dev &
sleep 2
# Confirm the store hydrates (will 404 until F1 is deployed, but no crash)
curl -s http://localhost:5173 | grep -q "root" && echo "OK"
kill %1
```

## Runtime Behaviour

1. On app mount, components using `useIsfrSlice()` call `fetchIsfrStatus()` and
   `fetchIsfrCurrent()` to hydrate initial state.
2. The main SSE stream (`/api/events`) delivers `isfr_rate_computed` events.
   `handleServerEvent()` updates `isfrCurrentRate` and appends to `isfrHistory`.
3. Source health changes arrive via `isfr_source_health_changed` — upserted into
   `isfrSources`.
4. Keeper start/stop arrives via `isfr_keeper_state_changed` — updates
   `isfrKeeperStatus`.
5. Optionally, a component can open a **dedicated** ISFR SSE stream via
   `isfrStreamUrl()` + `SseAdapter` for isolated high-frequency rate updates
   without the overhead of the full event firehose.

## Dependencies

- **F1** (roko-serve ISFR endpoints must exist — provides `/api/isfr/*` routes and `ServerEvent` variants)
- **F2** (roko-serve ISFR SSE stream — provides `/api/isfr/stream` endpoint)
