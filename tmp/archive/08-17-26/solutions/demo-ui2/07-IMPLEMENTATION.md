# 07. Implementation Plan

Phased implementation with concrete task checkboxes. Each task has acceptance criteria that an agent can verify independently. Tasks reference specific files, line numbers, and exact code from the codebase audit (08-AUDIT-FINDINGS.md).

---

## Phase 0: Cleanup & Dead Code Removal

Remove broken/orphaned code before building new architecture. Every task is safe and non-breaking. ~813 lines deleted, 3 bugs fixed, 2 memory leaks patched.

### 0.1 Delete Dead Hooks (6 files, ~348 lines)

- [x] **T0.1: Delete 6 dead hook files** ✓ DONE
  - Delete: `hooks/useSSE.ts` (~80L, zero imports, replaced by useEventStream)
  - Delete: `hooks/useAgents.ts` (~45L, zero imports, /api/managed-agents never wired)
  - Delete: `hooks/useDashboard.ts` (~60L, zero imports, dashboard page removed)
  - Delete: `hooks/useKnowledge.ts` (~55L, zero imports, Explorer fetches inline)
  - Delete: `hooks/useSweBench.ts` (~70L, zero imports, SWE-bench never shipped)
  - Delete: `hooks/useDemoMode.ts` (~38L, zero imports, demo mode handled inline)
  - **Verify**: `grep -rn 'useSSE\|useAgents\|useDashboard\|useKnowledge\|useSweBench\|useDemoMode' src/ --include='*.ts' --include='*.tsx'` returns no results. `npx tsc --noEmit` passes.

### 0.2 Delete Dead Components (6 files, ~465 lines)

- [x] **T0.2: Delete 6 dead component files** ✓ DONE
  - Delete: `components/WorkflowConstellation.tsx` (~120L, Three.js workflow viz, zero imports)
  - Delete: `components/StatCard.tsx` (~45L, replaced by Mosaic cells)
  - Delete: `components/LiveIndicator.tsx` (~35L, status dots inline everywhere)
  - Delete: `components/Skeleton.tsx` (~40L, loading states inline or missing)
  - Delete: `components/CrushedBar.tsx` (~85L, replaced by inline SVG in Bench.tsx)
  - Delete: `components/ModelPicker.tsx` (~140L, model selection inline in Builder.tsx)
  - **Verify**: `grep -rn 'WorkflowConstellation\|StatCard\|LiveIndicator\|Skeleton\|CrushedBar\|ModelPicker' src/ --include='*.ts' --include='*.tsx'` returns no results. `npx tsc --noEmit` passes.

### 0.3 Remove Dead Exports from Live Files

- [x] **T0.3: Clean dead exports in scenarios.ts** ✓ DONE
  - Removed `SCENARIO_MAP` and `resetScenarioState()`. `stripAnsi` kept (used internally).
  - `npx tsc --noEmit` passes.

- [x] **T0.4: Clean dead exports in useChain.ts** ✓ DONE
  - Removed `useChain` and `mirageRpc`. Kept `useChainWs` + `InsightEvent` (used by Demo.tsx).

### 0.4 Fix Memory Leaks

- [x] **T0.5: Fix module-level health poll in useLiveApi.ts** ✓ DONE
  - **File**: `hooks/useLiveApi.ts:43` — `let _healthPoll` at module scope never cleared on unmount
  - Move interval creation into a `useEffect` with cleanup: `return () => clearInterval(id)`
  - Remove module-level `_healthPoll` variable entirely
  - **Verify**: Mount component using `useLiveApi`, unmount, mount again. Only 1 interval running (check via `console.log` in interval callback). No duplicate polling.

- [x] **T0.6: Fix unbounded events array in useBenchSSE.ts** ✓ DONE
  - **File**: `hooks/useBenchSSE.ts:55` — `setEvents(prev => [...prev, parsed])` grows without bound
  - Add ring buffer: `setEvents(prev => [...prev.slice(-499), parsed])`
  - **Verify**: Start bench run with 100+ tasks. Events array stays ≤500 entries.

### 0.5 Fix Critical Bugs

- [x] **T0.7: Fix shell command injection in Builder.tsx** ✓ DONE
  - **File**: `pages/Builder.tsx` in `handleSend` — `handle.execCmd(\`roko run "${prompt}"...\`)`
  - Prompt with quotes/backticks breaks shell command or executes arbitrary code
  - Escape special chars: `const escaped = prompt.replace(/["\\` + '`' + `$]/g, '\\\\$&')`
  - **Verify**: Enter prompt containing `"; rm -rf /; echo "` — command is safely escaped, no injection.

- [x] **T0.8: Fix terminal listener accumulation** ✓ DONE
  - **File**: `hooks/useTerminal.ts` — `onData`/`onResize` listeners registered inside `connectWs()`, not in `attach()`
  - Move listener registration to `attach()` with a ref guard: `if (listenersAttached.current) return`
  - **Verify**: Open terminal, disconnect WS, reconnect. `term.onData` handler count === 1.

- [x] **T0.9: Fix SSE reconnect timer leak** ✓ DONE
  - Audited 5 files: useBenchSSE.ts (clean), useEventStream.ts (clean), EventStreamContext.tsx (clean)
  - Fixed `useChain.ts` line 170: added `clearTimeout` before `setTimeout` in onclose handler
  - Fixed `useTerminal.ts` line 254: added `clearTimeout` before `setTimeout` in onclose handler

- [x] **T0.10: Fix stale useCallback dependency in Builder.tsx** ✓ DONE
  - **File**: `pages/Builder.tsx` — `handleSend` callback captures `selectedModel` but deps may be stale
  - Include `selectedModel` in `useCallback` dependency array
  - **Verify**: Change model selector, send prompt — correct model used in command.

### 0.6 Relocate Misplaced Files

- [x] **T0.11: Move useTerminalSession.ts to lib/** ✓ DONE
  - Moved to `lib/terminal-session.ts`, updated 4 consumer imports (Builder.tsx, Terminal.tsx, Demo.tsx, scenarios.ts)
  - Corrected internal import paths for the new location

---

## Phase 1: Foundation — DataHub + Transport

Build the centralized data layer and unified transport. Replaces 19 hooks + 3 context providers + 3 SSE implementations.

### 1.1 Shared Utility Extraction

Do this first so all subsequent code can import shared utilities.

- [x] **T1.1: Create `lib/color.ts` — shared color utilities** ✓ DONE
  - Extract `hexToRgba` (duplicated in 6+ files: `CostRace.tsx`, `HeroParticleField.tsx`, `KnowledgeFlowPanel.tsx`, `DreamPhaseViz.tsx`, `MatrixRaceTrack.tsx`, `HeroScene.tsx`)
  - Add `getCssVar(name: string): string` for canvas contexts
  - Replace all 6+ inline copies with import
  - **Verify**: `grep -rn 'function hexToRgba' src/` returns only `lib/color.ts`. `npx tsc --noEmit` passes.

- [x] **T1.2: Create `lib/format.ts` — shared formatters** ✓ DONE
  - Extract `shortModel` (duplicated in 4 files: `BenchRunDetail.tsx`, `MatrixDetailView.tsx`, `CostRace.tsx`, `Bench.tsx`)
  - Extract `fmtUptime` (duplicated 3x in `Explorer.tsx`)
  - Extract `relativeTime` (duplicated in `Explorer.tsx`, `BenchRunDetail.tsx`)
  - Replace all inline copies with imports
  - **Verify**: `grep -rn 'function shortModel\|function fmtUptime\|function relativeTime' src/` returns only `lib/format.ts`.

- [x] **T1.3: Create `lib/palette.ts` — shared color maps** ✓ DONE
  - Extract `DOMAIN_COLORS` (duplicated in 3 files: `CostRace.tsx`, `Bench.tsx`, `MatrixDetailView.tsx`)
  - Extract `ROLE_COLORS` (duplicated in 2 files)
  - Extract `MODEL_COLORS` map
  - **Verify**: `grep -rn 'DOMAIN_COLORS\|ROLE_COLORS' src/` — only `lib/palette.ts` defines them.

- [x] **T1.4: Create `hooks/useCanvasSetup.ts` — shared canvas boilerplate** ✓ DONE
  - Hook created with DPR calculation, ResizeObserver, RAF batching, cleanup
  - Refactored 13 components: TokenVelocitySparkline, CostChart, BarChart, ScatterChart, ParetoChart, RadarChart, HeatmapChart, TimelineChart, CFactorSparkline, GateWaterfall, DreamPhaseViz, CostRace, MatrixRaceTrack, KnowledgeFlowPanel
  - Animated components (CostRace, MatrixRaceTrack, KnowledgeFlowPanel) use hybrid pattern: hook for setup/resize, own rAF loop for animation
  - **Remaining**: HeroParticleField, HeroScene, Explorer (5 canvases), BenchRunDetail (2) — deferred to scene rebuilds in Phase 3

### 1.2 Transport Layer

- [ ] **T1.5: Create `transport/api.ts` — typed REST client**
  - **Create**: `src/transport/api.ts`
  - **Read first**: `src/hooks/useApi.ts` (all), `src/hooks/useLiveApi.ts` (all), `src/hooks/useApiWithFallback.ts` (all), `src/lib/serve-url.ts` (all)
  - **Imports**:
    ```ts
    import { SERVE_URL } from '../lib/serve-url';
    ```
  - **Interface**:
    ```ts
    export interface ApiError {
      status: number;
      statusText: string;
      body: string | null;
    }

    /** Result type -- never throws. Callers check `.ok` and branch. */
    export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

    /** Health probe result cached with TTL. */
    export interface HealthSnapshot {
      reachable: boolean;
      checkedAt: number; // Date.now() ms
    }

    export class RokoApi {
      readonly baseUrl: string;
      private healthCache: HealthSnapshot | null;
      private healthInflight: Promise<HealthSnapshot> | null;
      private static readonly HEALTH_TTL_MS = 30_000;

      constructor(baseUrl?: string);
      /** GET with JSON parse. Returns ApiResult -- never throws. */
      get<T = unknown>(path: string, signal?: AbortSignal): Promise<ApiResult<T>>;
      /** POST with JSON body. Returns ApiResult -- never throws. */
      post<T = unknown>(path: string, body?: unknown, signal?: AbortSignal): Promise<ApiResult<T>>;
      /** PUT with JSON body. Returns ApiResult -- never throws. */
      put<T = unknown>(path: string, body?: unknown, signal?: AbortSignal): Promise<ApiResult<T>>;
      /** DELETE. Returns ApiResult -- never throws. */
      delete<T = unknown>(path: string, signal?: AbortSignal): Promise<ApiResult<T>>;
      /** Probe /health with 30s TTL cache + 2s timeout. Deduplicated -- only one in-flight. */
      probe(force?: boolean): Promise<HealthSnapshot>;
    }

    /** Singleton instance. Import this everywhere instead of constructing. */
    export const api: RokoApi;
    ```
  - **Implementation**:
    1. `constructor(baseUrl?)`: set `this.baseUrl = baseUrl ?? SERVE_URL`, init `healthCache = null`, `healthInflight = null`
    2. Private helper `async request<T>(method, path, body?, signal?)`:
       - Build `url = this.baseUrl + path`
       - Build headers: `{ 'Content-Type': 'application/json' }` (only if body present)
       - Wrap in try/catch -- on ANY error (network, abort, JSON parse), return `{ ok: false, error: { status: 0, statusText: err.message, body: null } }`
       - On response: if `!res.ok`, read body text, return `{ ok: false, error: { status: res.status, statusText: res.statusText, body: text } }`
       - On success: `return { ok: true, data: await res.json() as T }`
    3. `get<T>`, `post<T>`, `put<T>`, `delete<T>`: each calls `this.request(METHOD, path, body, signal)`
    4. `probe(force?)`:
       - If `!force && this.healthCache && Date.now() - this.healthCache.checkedAt < HEALTH_TTL_MS`: return cached
       - If `this.healthInflight`: return it (dedup)
       - Set `this.healthInflight = (async () => { ... })()`:
         - `fetch(this.baseUrl + '/health', { signal: AbortSignal.timeout(2000) })`
         - On success: `{ reachable: res.ok, checkedAt: Date.now() }`
         - On error: `{ reachable: false, checkedAt: Date.now() }`
         - Assign to `this.healthCache`, clear `this.healthInflight`
       - Return the promise
    5. Export singleton: `export const api = new RokoApi();`
  - **Replaces**: `hooks/useApi.ts` (raw fetch wrapper), `hooks/useLiveApi.ts` (health probe + fetch), `hooks/useApiWithFallback.ts` (offline fallback fetch). Delete these three after DataHub wiring (T1.11).
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.5 OK"`

- [ ] **T1.6: Create `transport/sse.ts` — SSE adapter with auto-reconnect**
  - **Create**: `src/transport/sse.ts`
  - **Read first**: `src/hooks/useEventStream.ts` (all -- the `createEventStreamManager` pattern), `src/hooks/useBenchSSE.ts` (all -- the `EventSource` + reconnect pattern), `src/contexts/EventStreamContext.tsx` (all -- the provider pattern being replaced), `src/lib/serve-url.ts` (all)
  - **Imports**:
    ```ts
    import { SERVE_URL } from '../lib/serve-url';
    ```
  - **Interface**:
    ```ts
    export type SseStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'failed';

    export interface SseAdapterConfig {
      /** Full URL to SSE endpoint, e.g. `${SERVE_URL}/api/events` */
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

      constructor(config: SseAdapterConfig);
      /** Open the EventSource connection. Idempotent -- does nothing if already connected. */
      connect(): void;
      /** Close the connection and cancel any pending reconnect. Resets retry counter. */
      disconnect(): void;
      /** Close + set status to 'idle'. After destroy(), connect() is a no-op. */
      destroy(): void;
    }
    ```
  - **Implementation**:
    1. Constructor: store `config`, set `status = 'idle'`, `lastEventId = null`, `retryCount = 0`, `retryTimer: ReturnType<typeof setTimeout> | null = null`, `es: EventSource | null = null`, `destroyed = false`
    2. Private `setStatus(s: SseStatus)`: if `s !== this.status`, set `this.status = s`, call `this.config.onStatusChange(s)`
    3. `connect()`:
       - If `destroyed || status === 'connected' || status === 'connecting'`: return
       - Set status to `retryCount === 0 ? 'connecting' : 'reconnecting'`
       - Close any existing `this.es`
       - Build URL string. If `this.lastEventId`, append `?lastEventId=${encodeURIComponent(this.lastEventId)}` (or `&` if URL already has `?`)
       - `this.es = new EventSource(url)`
       - `es.onopen`: if not destroyed and `es === this.es`, `retryCount = 0`, `setStatus('connected')`
       - `es.onmessage`: if not destroyed and `es === this.es`:
         - If `e.lastEventId`: store as `this.lastEventId`
         - Parse `JSON.parse(e.data)` in try/catch -- on parse error, skip silently
         - Call `this.config.onEvent(parsed as Record<string, unknown>)`
       - `es.onerror`: if destroyed, close + return. Else: close es, set `this.es = null`, increment `retryCount`
         - If `retryCount > (config.maxRetries ?? 5)`: `setStatus('failed')`, return
         - `setStatus('reconnecting')`
         - Compute delay: `Math.min((config.baseBackoffMs ?? 1000) * 2 ** (retryCount - 1), config.maxBackoffMs ?? 15_000)`
         - `this.retryTimer = setTimeout(() => this.connect(), delay)`
    4. `disconnect()`: clear `retryTimer` with `clearTimeout`, close `es`, set `es = null`, `retryCount = 0`, `setStatus('idle')`
    5. `destroy()`: set `this.destroyed = true`, call `this.disconnect()`
  - **Replaces**: `hooks/useEventStream.ts` (`createEventStreamManager` + `useEventStreamSubscription`), `hooks/useBenchSSE.ts` (bench-specific SSE), `contexts/EventStreamContext.tsx` (React context wrapper). Delete these three files after DataHub wiring (T1.11).
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.6 OK"`

- [ ] **T1.7: Create `transport/ws.ts` — WebSocket adapter with subscriptions**
  - **Create**: `src/transport/ws.ts`
  - **Read first**: `src/lib/workflow-api.ts` (all -- the WS connection in `openWorkflowSubscriptions`), `src/lib/serve-url.ts` (all -- `WS_BASE` export)
  - **Imports**:
    ```ts
    import { WS_BASE } from '../lib/serve-url';
    ```
  - **Interface**:
    ```ts
    export type WsStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'failed';

    /** Outbound control messages. */
    export interface WsSubscribeMsg {
      type: 'subscribe';
      root: string;
      projections: string[];
    }
    export interface WsUnsubscribeMsg {
      type: 'unsubscribe';
      root: string;
    }
    export interface WsResizeMsg {
      type: 'resize';
      cols: number;
      rows: number;
    }
    export interface WsPingMsg {
      type: 'ping';
    }
    export type WsOutbound = WsSubscribeMsg | WsUnsubscribeMsg | WsResizeMsg | WsPingMsg;

    /** Inbound frames from the server. */
    export interface WsFrame {
      type: 'state' | 'delta' | 'ack' | 'error' | 'pong';
      channel?: string;
      cursor?: number;
      workflow_id?: string | null;
      workdir?: string;
      data?: unknown;
      event?: unknown;
      message?: string;
    }

    export interface WsAdapterConfig {
      /** Full WS URL, e.g. `${WS_BASE}/api/workflow/ws` */
      url: string;
      /** Called on every parsed inbound frame. */
      onFrame: (frame: WsFrame) => void;
      /** Called whenever connection status changes. */
      onStatusChange: (status: WsStatus) => void;
      /** Max reconnect attempts. Default: 5. */
      maxRetries?: number;
      /** Max backoff ms. Default: 15_000. */
      maxBackoffMs?: number;
      /** Ping interval ms. Default: 30_000. Set 0 to disable. */
      pingIntervalMs?: number;
    }

    export class WsAdapter {
      status: WsStatus;
      /** Map of root -> projections[] currently subscribed. */
      readonly subscriptions: Map<string, string[]>;

      constructor(config: WsAdapterConfig);
      /** Open the WebSocket. Idempotent. */
      connect(): void;
      /** Send a typed outbound message. Queues if not yet connected. */
      send(msg: WsOutbound): void;
      /** Subscribe to projections for a root. Sends subscribe message + stores in map. */
      subscribe(root: string, projections: string[]): void;
      /** Unsubscribe from a root. Sends unsubscribe message + removes from map. */
      unsubscribe(root: string): void;
      /** Close connection, cancel reconnects. */
      disconnect(): void;
      /** Close + prevent reconnect permanently. */
      destroy(): void;
    }
    ```
  - **Implementation**:
    1. Constructor: store config, set `status = 'idle'`, `subscriptions = new Map()`, `ws: WebSocket | null = null`, `destroyed = false`, `retryCount = 0`, `retryTimer: ReturnType<typeof setTimeout> | null = null`, `pingTimer: ReturnType<typeof setInterval> | null = null`, `sendQueue: WsOutbound[] = []`
    2. Private `setStatus(s)`: if changed, set + call `config.onStatusChange(s)`
    3. `connect()`:
       - Guard: if destroyed, connected, or connecting, return
       - Set status `retryCount === 0 ? 'connecting' : 'reconnecting'`
       - `this.ws = new WebSocket(this.config.url)`
       - `ws.onopen`:
         - `retryCount = 0`, `setStatus('connected')`
         - Flush `sendQueue`: for each msg, `ws.send(JSON.stringify(msg))`, then clear array
         - Start ping interval (if `config.pingIntervalMs !== 0`): `this.pingTimer = setInterval(() => { if (ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: 'ping' })); }, config.pingIntervalMs ?? 30_000)`
         - Re-subscribe: for each `[root, projs]` in `this.subscriptions`, send `{ type: 'subscribe', root, projections: projs }`
       - `ws.onmessage`: try `JSON.parse(e.data) as WsFrame`, call `config.onFrame(frame)`. On parse error, skip. If `frame.type === 'pong'`, no further action.
       - `ws.onerror`: no-op (WebSocket fires onerror then onclose -- handle reconnect in onclose only)
       - `ws.onclose`:
         - Clear ping timer with `clearInterval`
         - If destroyed: `setStatus('idle')`, return
         - Increment `retryCount`
         - If `retryCount > (config.maxRetries ?? 5)`: `setStatus('failed')`, return
         - `setStatus('reconnecting')`
         - Backoff: `Math.min(1000 * 2 ** (retryCount - 1), config.maxBackoffMs ?? 15_000)`
         - `retryTimer = setTimeout(() => this.connect(), delay)`
    4. `send(msg)`: if `ws?.readyState === WebSocket.OPEN`, `ws.send(JSON.stringify(msg))`. Else push to `sendQueue`.
    5. `subscribe(root, projections)`: `this.subscriptions.set(root, projections)`, call `this.send({ type: 'subscribe', root, projections })`
    6. `unsubscribe(root)`: `this.subscriptions.delete(root)`, call `this.send({ type: 'unsubscribe', root })`
    7. `disconnect()`: `clearTimeout(retryTimer)`, `clearInterval(pingTimer)`, if ws: `ws.close()`, `ws = null`. `retryCount = 0`, `setStatus('idle')`
    8. `destroy()`: `destroyed = true`, call `disconnect()`, `sendQueue.length = 0`
  - **Replaces**: WS connection in `lib/workflow-api.ts` (the `openWorkflowSubscriptions` WS branch), PTY WS in `hooks/useTerminal.ts`. These files are NOT deleted -- they are modified in later tasks to use `WsAdapter` instead of raw `new WebSocket()`.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.7 OK"`

- [ ] **T1.8: Create `transport/types.ts` — ServerEvent type definitions**
  - **Create**: `src/transport/types.ts`
  - **Read first**: `crates/roko-serve/src/events.rs` (ALL -- every `ServerEvent` variant and its fields), `src/lib/bench-types.ts` (all -- existing bench event types to align with)
  - **Imports**: None (pure type file, no runtime imports except `parseServerEvent`)
  - **Interface**:
    ```ts
    // -- Execution sub-events (nested inside ServerEvent::Execution) ----
    export type ExecutionEvent =
      | { type: 'plan_started' }
      | { type: 'task_started'; taskId: string; title: string; phase: string }
      | { type: 'task_phase_changed'; taskId: string; oldPhase: string; newPhase: string }
      | { type: 'gate_result'; taskId: string; gate: string; passed: boolean; message: string }
      | { type: 'task_completed'; taskId: string; outcome: string }
      | { type: 'plan_completed'; outcome: string; stats: Record<string, unknown> }
      | { type: 'replan_triggered'; taskId: string; strategy: string }
      | { type: 'watcher_alert'; watcher: string; message: string };

    // -- Top-level ServerEvent discriminated union ----------------------
    // 50 variants. Field names are camelCase conversions of Rust snake_case.
    // Variants with #[serde(rename = "...")] use the RENAMED value as the type tag.
    export type ServerEvent =
      // Plan execution
      | { type: 'plan_started'; planId: string }
      | { type: 'plan_completed'; planId: string; success: boolean }
      | { type: 'phase_transition'; planId: string; from: string; to: string }
      | { type: 'execution'; planId: string; event: ExecutionEvent }
      | { type: 'episode'; planId: string; taskId: string; passed: boolean }
      | { type: 'efficiency_event'; planId: string; taskId: string;
           metric: string; value: number }
      // Task lifecycle (dashboard-facing)
      | { type: 'task_started'; planId: string; taskId: string;
           description: string }
      | { type: 'task_completed'; planId: string; taskId: string;
           success: boolean }
      | { type: 'task_failed'; planId: string; taskId: string; error: string }
      // Agent lifecycle
      | { type: 'agent_spawned'; agentId: string; role: string; model: string }
      | { type: 'agent_output'; agentId: string; runId?: string;
           content: string; done: boolean;
           metadata?: Record<string, unknown> }
      | { type: 'agent_trace'; agentId: string; runId?: string;
           content: string; toolCalls?: unknown[]; reasoning?: string;
           usage?: Record<string, unknown>; done: boolean }
      | { type: 'agent_started'; agentId: string }
      | { type: 'agent_stopped'; agentId: string; reason: string }
      // Gate results
      | { type: 'gate_result'; planId: string; taskId: string;
           gate: string; rung: number; passed: boolean }
      // Inference tracking
      | { type: 'inference_started'; requestId: string; model: string;
           agentId: string; autoRouted: boolean }
      | { type: 'inference_completed'; requestId: string; model: string;
           agentId: string; inputTokens: number; outputTokens: number;
           costUsd: number; durationMs: number }
      | { type: 'inference_failed'; requestId: string; model: string;
           agentId: string; error: string }
      // One-shot runs
      | { type: 'run_started'; runId: string; prompt: string }
      | { type: 'run_completed'; runId: string; success: boolean }
      // Generic operations
      | { type: 'operation_started'; opId: string; kind: string }
      | { type: 'operation_completed'; opId: string; kind: string;
           success: boolean }
      // Somatic / affect
      | { type: 'somatic_marker_fired'; planId: string; taskId: string;
           valence: number; intensity: number; sourceEpisodes: string[];
           strategyParam: string }
      // Deployment lifecycle
      | { type: 'deployment_created'; id: string; name: string }
      | { type: 'deployment_ready'; id: string; url: string }
      | { type: 'deployment_failed'; id: string; reason: string }
      | { type: 'deployment_torn_down'; id: string }
      // Job marketplace
      | { type: 'job_created'; job: Record<string, unknown> }
      | { type: 'job_posted_to_candidate'; jobId: string; agentId: string;
           reward: string }
      | { type: 'job_updated'; job: Record<string, unknown> }
      | { type: 'job_transitioned'; jobId: string; from: string; to: string;
           assignedTo?: string }
      | { type: 'job_execution_started'; jobId: string; jobType: string;
           agentId: string }
      | { type: 'job_progress'; jobId: string; percent: number;
           message: string }
      | { type: 'job_agent_output'; jobId: string; agentId: string;
           content: string; done: boolean }
      | { type: 'job_submitted'; jobId: string; agentId: string }
      | { type: 'job_evaluated'; jobId: string; accepted: boolean;
           feedback: string }
      | { type: 'job_state_changed'; jobId: string; from: string;
           to: string }
      // Worker
      | { type: 'worker_task_started'; deploymentId: string;
           taskId: string }
      | { type: 'worker_task_completed'; deploymentId: string;
           taskId: string; success: boolean }
      // Chain triage
      | { type: 'chain_triage_result'; jobId: string; eventCount: number;
           anomalyCount: number; summary: string }
      // Heartbeat
      | { type: 'heartbeat_received'; senderId: string;
           activeTasks: number; activeAgents: number }
      | { type: 'heartbeat'; agentId: string; blockNumber?: number }
      // Config / strategy reload
      | { type: 'config_reloaded'; appliedSections: string[];
           restartRequired: string[] }
      | { type: 'strategy_reloaded'; goalsCount: number;
           tacticsCount: number }
      // Vision loop
      | { type: 'vision_loop_iteration'; runId: string; iteration: number;
           score: number; notes: string }
      | { type: 'vision_loop_completed'; runId: string; iterations: number;
           bestScore: number; stopReason: string }
      // Webhook
      | { type: 'webhook_received'; signal: Record<string, unknown> }
      // Bench (PascalCase type tags -- server uses #[serde(rename)])
      | { type: 'BenchRunStarted'; benchId: string; suiteId: string;
           totalTasks: number }
      | { type: 'BenchTaskStarted'; benchId: string; taskId: string;
           taskName: string; taskIndex: number; totalTasks: number }
      | { type: 'BenchTaskCompleted'; benchId: string; taskId: string;
           result: Record<string, unknown> }
      | { type: 'BenchLearningEvent'; benchId: string; taskId: string;
           playbooksCreated: number; antiPatternsCreated: number;
           totalPlaybooks: number; totalAntiPatterns: number }
      | { type: 'BenchProgress'; benchId: string; completed: number;
           total: number; costSoFar: number }
      | { type: 'BenchRunCompleted'; benchId: string;
           summary: Record<string, unknown> }
      | { type: 'BenchGateVerdict'; benchId: string; taskId: string;
           gate: string; passed: boolean; message?: string;
           durationMs: number }
      | { type: 'BenchTokenVelocity'; benchId: string; taskId: string;
           tokensPerSecond: number; tokensIn: number; tokensOut: number;
           durationMs: number }
      | { type: 'BenchAgentOutput'; benchId: string; taskId: string;
           agentId: string; content: string; done: boolean;
           toolCalls?: unknown[]; reasoning?: string }
      // Matrix bench
      | { type: 'MatrixRunStarted'; matrixId: string; suiteId: string;
           laneIds: string[]; totalLanes: number }
      | { type: 'MatrixLaneCompleted'; matrixId: string; laneId: string;
           passRate: number; costUsd: number }
      | { type: 'MatrixRunCompleted'; matrixId: string;
           summary: Record<string, unknown>[] }
      // SWE-bench
      | { type: 'SweRunStarted'; runId: string; dataset: string;
           totalInstances: number }
      | { type: 'SweInstanceCompleted'; runId: string; instanceId: string;
           resolved: boolean; durationMs: number }
      | { type: 'SweRunCompleted'; runId: string; resolved: number;
           total: number; passRate: number }
      // System
      | { type: 'server_shutdown' }
      | { type: 'error'; message: string };

    /**
     * Parse a raw JSON object (from SSE `e.data`) into a typed ServerEvent.
     * Converts snake_case field names to camelCase.
     * Returns null if the `type` field is missing.
     */
    export function parseServerEvent(
      raw: Record<string, unknown>,
    ): ServerEvent | null;
    ```
  - **Implementation**:
    1. Export all types above as written (pure type declarations)
    2. Implement `parseServerEvent(raw)`:
       - If `typeof raw.type !== 'string'`, return `null`
       - Create a new object. Copy `type` field as-is (do NOT convert to camelCase -- Bench events use PascalCase like `BenchRunStarted`)
       - For every other key in `raw`: convert from snake_case to camelCase using `key.replace(/_([a-z])/g, (_, c) => c.toUpperCase())`
       - If a value is a plain object (not array, not null), recursively convert its keys too
       - Return the converted object cast as `unknown as ServerEvent`
    3. **snake_case to camelCase field mapping reference** (Rust field -> TS field):
       - `plan_id` -> `planId`, `task_id` -> `taskId`, `agent_id` -> `agentId`
       - `run_id` -> `runId`, `op_id` -> `opId`, `job_id` -> `jobId`
       - `bench_id` -> `benchId`, `matrix_id` -> `matrixId`, `suite_id` -> `suiteId`
       - `cost_usd` -> `costUsd`, `duration_ms` -> `durationMs`
       - `input_tokens` -> `inputTokens`, `output_tokens` -> `outputTokens`
       - `auto_routed` -> `autoRouted`, `cost_so_far` -> `costSoFar`
       - `prompt_preview` (serde rename) -> `prompt` (keep as `prompt`)
       - `tool_calls` -> `toolCalls`, `tokens_per_second` -> `tokensPerSecond`
       - `pass_rate` -> `passRate`, `lane_ids` -> `laneIds`
       - `total_lanes` -> `totalLanes`, `total_tasks` -> `totalTasks`
       - `task_name` -> `taskName`, `task_index` -> `taskIndex`
       - `applied_sections` -> `appliedSections`, `restart_required` -> `restartRequired`
       - `goals_count` -> `goalsCount`, `tactics_count` -> `tacticsCount`
       - `block_number` -> `blockNumber`, `active_tasks` -> `activeTasks`
       - `sender_id` -> `senderId`, `active_agents` -> `activeAgents`
       - `assigned_to` -> `assignedTo`, `event_count` -> `eventCount`
       - `anomaly_count` -> `anomalyCount`, `deployment_id` -> `deploymentId`
       - `best_score` -> `bestScore`, `stop_reason` -> `stopReason`
       - `instance_id` -> `instanceId`, `total_instances` -> `totalInstances`
       - `source_episodes` -> `sourceEpisodes`, `strategy_param` -> `strategyParam`
       - `playbooks_created` -> `playbooksCreated`, `anti_patterns_created` -> `antiPatternsCreated`
       - `total_playbooks` -> `totalPlaybooks`, `total_anti_patterns` -> `totalAntiPatterns`
       - `tokens_in` -> `tokensIn`, `tokens_out` -> `tokensOut`
  - **Replaces**: Scattered inline type assertions like `JSON.parse(e.data) as Record<string, unknown>` in `useEventStream.ts:56`, `as BenchSSEEvent` in `useBenchSSE.ts:51`, `as WorkflowFrame` in `workflow-api.ts:202`. The bench-specific event types in `lib/bench-types.ts` (lines 108-343) remain for now -- they are consumed by bench components and will be aligned in Phase 3.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.8 OK"`

### 1.3 DataHub (Zustand Store)

- [ ] **T1.9: Create `app/DataHub.ts` — core Zustand store**
  - **Create**: `src/app/DataHub.ts`
  - **Read first**: `src/hooks/useRokoConfig.ts` (all -- the `RokoConfigState` + `applyConfig` pattern being replaced), `src/hooks/useBench.ts` (lines 1-50 -- `BenchConfig`, `ActiveRun`, `ConnectionState` types), `src/lib/bench-types.ts` (all -- `BenchRun`, `BenchSuite`, `BenchModel` types), `src/transport/types.ts` (T1.8 -- `ServerEvent` union), `src/transport/api.ts` (T1.5 -- `api` singleton)
  - **Imports**:
    ```ts
    import { create } from 'zustand';
    import type { ServerEvent } from '../transport/types';
    import { api } from '../transport/api';
    import type { BenchRun, BenchSuite, BenchModel } from '../lib/bench-types';
    ```
  - **Interface**:
    ```ts
    export type ServerStatus = 'connected' | 'checking' | 'disconnected';
    export type StreamStatus =
      | 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'failed';

    export interface WorkspaceInfo {
      id: string;
      path: string;
      ready: boolean;
    }

    export interface AgentInfo {
      agentId: string;
      role: string;
      model: string;
      status: 'running' | 'stopped';
    }

    export interface EpisodeInfo {
      planId: string;
      taskId: string;
      passed: boolean;
      timestamp: number;
    }

    export interface InferenceRecord {
      requestId: string;
      model: string;
      agentId: string;
      inputTokens: number;
      outputTokens: number;
      costUsd: number;
      durationMs: number;
    }

    export interface DataHub {
      // -- Connection status -------------------------------------------
      serverStatus: ServerStatus;
      sseStatus: StreamStatus;
      wsStatus: StreamStatus;

      // -- Config slice ------------------------------------------------
      config: Record<string, unknown> | null;
      defaultModel: string;
      defaultBackend: string;

      // -- Workspace slice ---------------------------------------------
      serverWorkdir: string | null;
      workspace: WorkspaceInfo | null;
      workspaceCache: Map<string, WorkspaceInfo>;

      // -- Plan execution slice ----------------------------------------
      activePlanId: string | null;
      activePhase: string | null;
      planCompleted: boolean;

      // -- Agent slice -------------------------------------------------
      agents: AgentInfo[];

      // -- Episode / metrics slice -------------------------------------
      episodes: EpisodeInfo[];
      totalCost: number;
      totalTokens: number;
      recentInferences: InferenceRecord[]; // ring buffer, max 200

      // -- Bench slice -------------------------------------------------
      benchRuns: BenchRun[];
      benchSuites: BenchSuite[];
      benchModels: BenchModel[];

      // -- Actions: event handling -------------------------------------
      handleServerEvent: (event: ServerEvent) => void;
      setServerStatus: (status: ServerStatus) => void;
      setSseStatus: (status: StreamStatus) => void;
      setWsStatus: (status: StreamStatus) => void;

      // -- Actions: REST fetches ---------------------------------------
      fetchConfig: () => Promise<void>;
      updateConfig: (partial: Record<string, unknown>) => Promise<boolean>;
      fetchBenchRuns: () => Promise<void>;
      fetchBenchSuites: () => Promise<void>;
      fetchBenchModels: () => Promise<void>;
      fetchAgents: () => Promise<void>;
      fetchServerWorkdir: () => Promise<void>;
      ensureWorkspace: (
        prefix: string,
        opts?: { gitInit?: boolean },
      ) => Promise<WorkspaceInfo>;
      destroyWorkspace: (id: string) => Promise<void>;
    }

    export const useDataHub: import('zustand').UseBoundStore<
      import('zustand').StoreApi<DataHub>
    >;
    ```
  - **Implementation**:
    1. `export const useDataHub = create<DataHub>()((set, get) => ({ ... }))`
    2. **Initial state**:
       ```ts
       serverStatus: 'checking',
       sseStatus: 'idle',
       wsStatus: 'idle',
       config: null,
       defaultModel: '',
       defaultBackend: '',
       serverWorkdir: null,
       workspace: null,
       workspaceCache: new Map(),
       activePlanId: null,
       activePhase: null,
       planCompleted: false,
       agents: [],
       episodes: [],
       totalCost: 0,
       totalTokens: 0,
       recentInferences: [],
       benchRuns: [],
       benchSuites: [],
       benchModels: [],
       ```
    3. **`handleServerEvent(event)`** -- switch on `event.type`:
       - `'plan_started'`: `set({ activePlanId: event.planId, activePhase: 'started', planCompleted: false })`
       - `'plan_completed'`: `set({ planCompleted: true, activePhase: 'completed' })`
       - `'phase_transition'`: `set({ activePhase: event.to })`
       - `'agent_spawned'`: `set(s => ({ agents: [...s.agents, { agentId: event.agentId, role: event.role, model: event.model, status: 'running' as const }] }))`
       - `'agent_stopped'`: `set(s => ({ agents: s.agents.map(a => a.agentId === event.agentId ? { ...a, status: 'stopped' as const } : a) }))`
       - `'episode'`: `set(s => ({ episodes: [...s.episodes.slice(-499), { planId: event.planId, taskId: event.taskId, passed: event.passed, timestamp: Date.now() }] }))`
       - `'inference_completed'`: `set(s => ({ totalCost: s.totalCost + event.costUsd, totalTokens: s.totalTokens + event.inputTokens + event.outputTokens, recentInferences: [...s.recentInferences.slice(-199), { requestId: event.requestId, model: event.model, agentId: event.agentId, inputTokens: event.inputTokens, outputTokens: event.outputTokens, costUsd: event.costUsd, durationMs: event.durationMs }] }))`
       - `'gate_result'`: no store update (consumed by components via raw event subscriptions)
       - `'config_reloaded'`: call `get().fetchConfig()`
       - `'BenchRunCompleted'`: call `get().fetchBenchRuns()` to refresh the run list
       - `'server_shutdown'`: `set({ serverStatus: 'disconnected' })`
       - `'error'`: `console.warn('[DataHub] server error:', event.message)`
       - Default: no-op (unknown events silently ignored)
    4. **`setServerStatus(status)`**: `set({ serverStatus: status })`
    5. **`setSseStatus(status)`**: `set({ sseStatus: status })`
    6. **`setWsStatus(status)`**: `set({ wsStatus: status })`
    7. **`fetchConfig()`**: call `api.get<Record<string, unknown>>('/api/config')`. If `res.ok`: extract `defaultModel` from `(res.data.agent as Record<string,string>)?.default_model ?? ''` and `defaultBackend` from `(res.data.agent as Record<string,string>)?.default_backend ?? ''` (same pattern as `useRokoConfig.ts:79-91`). Then `set({ config: res.data, defaultModel, defaultBackend })`.
    8. **`updateConfig(partial)`**: call `api.put<Record<string, unknown>>('/api/config', partial)`. If `res.ok`: `set({ config: res.data })`, return `true`. Else return `false`.
    9. **`fetchBenchRuns()`**: `api.get<BenchRun[]>('/api/bench/runs')`. If ok: `set({ benchRuns: res.data })`.
    10. **`fetchBenchSuites()`**: `api.get<BenchSuite[]>('/api/bench/suites')`. If ok: `set({ benchSuites: res.data })`.
    11. **`fetchBenchModels()`**: `api.get<BenchModel[]>('/api/bench/models')`. If ok: `set({ benchModels: res.data })`.
    12. **`fetchAgents()`**: `api.get<AgentInfo[]>('/api/managed-agents')`. If ok: `set({ agents: res.data })`.
    13. **`fetchServerWorkdir()`**: `api.get<{ path: string }>('/api/workspaces/default')`. If ok: `set({ serverWorkdir: res.data.path })`.
    14. Ring buffer sizes: episodes capped at 500, recentInferences capped at 200 (use `.slice(-N)` before spread).
  - **Replaces**: `contexts/EventStreamContext.tsx` (event dispatch), `hooks/useRokoConfig.ts` (config state + polling), `hooks/useLiveApi.ts` (health tracking), `hooks/useServerHealth.ts` (status polling), `hooks/useApiWithFallback.ts` (offline detection). These files are deleted after T1.11 wiring.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.9 OK"`

- [ ] **T1.10: Add workspace slice to DataHub**
  - **Modify**: `src/app/DataHub.ts` (add to the store created in T1.9)
  - **Read first**: `src/hooks/useWorkspace.ts` (all -- the `ensureWorkspace`/`createWorkspace`/`destroyWorkspace` pattern), `src/transport/api.ts` (T1.5 -- `api` singleton)
  - **Implementation** (add these action bodies inside the `create<DataHub>()` call from T1.9):
    1. **`ensureWorkspace(prefix, opts?)`**:
       ```ts
       ensureWorkspace: async (prefix, opts) => {
         const cached = get().workspaceCache.get(prefix);
         if (cached) return cached;
         const res = await api.post<WorkspaceInfo>('/api/workspaces', {
           prefix,
           git_init: opts?.gitInit ?? true,
         });
         if (!res.ok) {
           throw new Error(
             `Failed to create workspace: ${res.error.status} ${res.error.body}`,
           );
         }
         const ws = res.data;
         set(s => {
           const next = new Map(s.workspaceCache);
           next.set(prefix, ws);
           return { workspace: ws, workspaceCache: next };
         });
         return ws;
       },
       ```
    2. **`destroyWorkspace(id)`**:
       ```ts
       destroyWorkspace: async (id) => {
         await api.delete(`/api/workspaces/${encodeURIComponent(id)}`);
         set(s => {
           const next = new Map(s.workspaceCache);
           for (const [key, ws] of next.entries()) {
             if (ws.id === id) { next.delete(key); break; }
           }
           return {
             workspace: s.workspace?.id === id ? null : s.workspace,
             workspaceCache: next,
           };
         });
       },
       ```
    3. **`fetchServerWorkdir()`** (already specified in T1.9 -- ensure it is implemented):
       ```ts
       fetchServerWorkdir: async () => {
         const res = await api.get<{ path: string }>(
           '/api/workspaces/default',
         );
         if (res.ok) set({ serverWorkdir: res.data.path });
       },
       ```
  - **NOTE**: T1.9 and T1.10 target the same file. T1.10 exists as a separate task for independent verification. If implementing both together, include all workspace actions in the initial store creation.
  - **Replaces**: `hooks/useWorkspace.ts` (the entire `WorkspaceProvider` + `useWorkspace` context). Delete after T1.11 wiring.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.10 OK"`

- [ ] **T1.11: Wire transport into DataHub and remove old providers**
  - **Modify**: `src/main.tsx`, `src/components/AppShell.tsx`
  - **Create**: `src/app/bootstrap.ts` (transport initialization)
  - **Read first**: `src/main.tsx` (all -- current provider wrappers: `EventStreamProvider` on line 52, `WorkspaceProvider` on line 53), `src/components/AppShell.tsx` (all -- current `useApiWithFallback` usage), `src/transport/api.ts` (T1.5), `src/transport/sse.ts` (T1.6), `src/transport/ws.ts` (T1.7), `src/app/DataHub.ts` (T1.9/T1.10)
  - **Imports for `src/app/bootstrap.ts`**:
    ```ts
    import { api } from '../transport/api';
    import { SseAdapter } from '../transport/sse';
    import { WsAdapter } from '../transport/ws';
    import { parseServerEvent } from '../transport/types';
    import { useDataHub } from './DataHub';
    import { SERVE_URL, WS_BASE } from '../lib/serve-url';
    ```
  - **Interface for `src/app/bootstrap.ts`**:
    ```ts
    /**
     * Initialize transport layer and wire events into DataHub.
     * Call ONCE before React render. Returns cleanup function.
     */
    export function bootstrapTransport(): () => void;
    ```
  - **Implementation**:
    1. **Create `src/app/bootstrap.ts`** (full implementation):
       ```ts
       export function bootstrapTransport(): () => void {
         const hub = useDataHub.getState;
         const set = useDataHub.setState;

         // 1. Probe server health
         api.probe().then(snap => {
           set({
             serverStatus: snap.reachable ? 'connected' : 'disconnected',
           });
         });

         // 2. Health poll every 30s
         const healthInterval = setInterval(() => {
           api.probe(true).then(snap => {
             set({
               serverStatus: snap.reachable ? 'connected' : 'disconnected',
             });
           });
         }, 30_000);

         // 3. Connect SSE -> route events to DataHub
         const sse = new SseAdapter({
           url: `${SERVE_URL}/api/events`,
           onEvent: (raw) => {
             const event = parseServerEvent(raw);
             if (event) hub().handleServerEvent(event);
           },
           onStatusChange: (status) => set({ sseStatus: status }),
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
         };
       }
       ```
    2. **Modify `src/main.tsx`** -- exact changes:
       - **Remove** import: `import { EventStreamProvider } from './contexts/EventStreamContext';`
       - **Remove** import: `import { WorkspaceProvider } from './hooks/useWorkspace';`
       - **Add** import: `import { bootstrapTransport } from './app/bootstrap';`
       - **Add** before `createRoot(...)`: `const cleanupTransport = bootstrapTransport();`
       - **Remove** `<EventStreamProvider>` opening tag (line 52)
       - **Remove** `</EventStreamProvider>` closing tag (line 82)
       - **Remove** `<WorkspaceProvider>` opening tag (line 53)
       - **Remove** `</WorkspaceProvider>` closing tag (line 81)
       - Resulting render tree:
         ```tsx
         <StrictMode>
           <BrowserRouter>
             <ErrorBoundary>
               <Suspense fallback={<RouteLoading />}>
                 <Routes>
                   <Route element={<AppShell />}> ... </Route>
                 </Routes>
               </Suspense>
             </ErrorBoundary>
           </BrowserRouter>
         </StrictMode>
         ```
    3. **Modify `src/components/AppShell.tsx`** -- exact changes:
       - **Remove**: `import { useApiWithFallback } from '../hooks/useApiWithFallback';`
       - **Add**: `import { useDataHub } from '../app/DataHub';`
       - **Replace**: `const { dataMode } = useApiWithFallback();`
         **With**: `const serverStatus = useDataHub(s => s.serverStatus);`
       - **Replace**: `{dataMode === 'seed' && (`
         **With**: `{serverStatus === 'disconnected' && (`
       - **Replace** badge text: `SEED DATA` -> `OFFLINE`
  - **Replaces**: `<EventStreamProvider>` wrapper in `main.tsx`, `<WorkspaceProvider>` wrapper in `main.tsx`. After this task, delete these files: `src/contexts/EventStreamContext.tsx`, `src/hooks/useWorkspace.ts`. Do NOT yet delete `hooks/useLiveApi.ts`, `hooks/useApiWithFallback.ts`, `hooks/useApi.ts` -- they may still be imported by unmigrated components. Defer deletion to Phase 3 scene rebuilds.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.11 OK"`

### 1.4 Thin Selector Hooks

- [ ] **T1.12: Create thin selector hooks**
  - **Create**: 7 hook files + 1 barrel export in `src/hooks/`
  - **Read first**: `src/app/DataHub.ts` (T1.9 -- the `DataHub` interface and `useDataHub` export)
  - **Imports**: Each hook imports `useDataHub` from `'../app/DataHub'` and optionally `useEffect` from `'react'` and `useShallow` from `'zustand/react/shallow'`

  - **File 1 -- `src/hooks/useServerStatus.ts`** (full file, 5 lines):
    ```ts
    import { useDataHub } from '../app/DataHub';

    export function useServerStatus() {
      return useDataHub(s => s.serverStatus);
    }
    ```

  - **File 2 -- `src/hooks/useConfig.ts`** (full file, 15 lines):
    ```ts
    import { useEffect } from 'react';
    import { useDataHub } from '../app/DataHub';
    import { useShallow } from 'zustand/react/shallow';

    export function useConfig() {
      const slice = useDataHub(
        useShallow(s => ({
          config: s.config,
          defaultModel: s.defaultModel,
          defaultBackend: s.defaultBackend,
          fetchConfig: s.fetchConfig,
          updateConfig: s.updateConfig,
        })),
      );
      useEffect(() => {
        if (!slice.config) slice.fetchConfig();
      }, [slice.config, slice.fetchConfig]);
      return {
        config: slice.config,
        defaultModel: slice.defaultModel,
        defaultBackend: slice.defaultBackend,
        updateConfig: slice.updateConfig,
        refreshConfig: slice.fetchConfig,
      };
    }
    ```

  - **File 3 -- `src/hooks/useAgents.ts`** (full file, 12 lines):
    ```ts
    import { useEffect } from 'react';
    import { useDataHub } from '../app/DataHub';
    import { useShallow } from 'zustand/react/shallow';

    export function useAgents() {
      const { agents, fetchAgents } = useDataHub(
        useShallow(s => ({
          agents: s.agents,
          fetchAgents: s.fetchAgents,
        })),
      );
      useEffect(() => { fetchAgents(); }, [fetchAgents]);
      return agents;
    }
    ```

  - **File 4 -- `src/hooks/useBenchData.ts`** (full file, 18 lines):
    ```ts
    import { useEffect } from 'react';
    import { useDataHub } from '../app/DataHub';
    import { useShallow } from 'zustand/react/shallow';

    export function useBenchData() {
      const slice = useDataHub(
        useShallow(s => ({
          benchRuns: s.benchRuns,
          benchSuites: s.benchSuites,
          benchModels: s.benchModels,
          fetchBenchRuns: s.fetchBenchRuns,
          fetchBenchSuites: s.fetchBenchSuites,
          fetchBenchModels: s.fetchBenchModels,
        })),
      );
      useEffect(() => {
        slice.fetchBenchRuns();
        slice.fetchBenchSuites();
        slice.fetchBenchModels();
      }, [slice.fetchBenchRuns, slice.fetchBenchSuites, slice.fetchBenchModels]);
      return {
        benchRuns: slice.benchRuns,
        benchSuites: slice.benchSuites,
        benchModels: slice.benchModels,
      };
    }
    ```

  - **File 5 -- `src/hooks/useCostMetrics.ts`** (full file, 9 lines):
    ```ts
    import { useDataHub } from '../app/DataHub';
    import { useShallow } from 'zustand/react/shallow';

    export function useCostMetrics() {
      return useDataHub(
        useShallow(s => ({
          totalCost: s.totalCost,
          totalTokens: s.totalTokens,
          recentInferences: s.recentInferences,
        })),
      );
    }
    ```

  - **File 6 -- `src/hooks/useEpisodes.ts`** (full file, 5 lines):
    ```ts
    import { useDataHub } from '../app/DataHub';

    export function useEpisodes() {
      return useDataHub(s => s.episodes);
    }
    ```

  - **File 7 -- `src/hooks/useWorkspaceActions.ts`** (full file, 11 lines):
    ```ts
    import { useDataHub } from '../app/DataHub';
    import { useShallow } from 'zustand/react/shallow';

    export function useWorkspaceActions() {
      return useDataHub(
        useShallow(s => ({
          serverWorkdir: s.serverWorkdir,
          workspace: s.workspace,
          ensureWorkspace: s.ensureWorkspace,
          destroyWorkspace: s.destroyWorkspace,
        })),
      );
    }
    ```

  - **Barrel export -- `src/hooks/index.ts`** (create new file, 7 lines):
    ```ts
    export { useServerStatus } from './useServerStatus';
    export { useConfig } from './useConfig';
    export { useAgents } from './useAgents';
    export { useBenchData } from './useBenchData';
    export { useCostMetrics } from './useCostMetrics';
    export { useEpisodes } from './useEpisodes';
    export { useWorkspaceActions } from './useWorkspaceActions';
    ```

  - **Replaces**: `hooks/useServerHealth.ts` (health polling -> `useServerStatus`), `hooks/useRokoConfig.ts` (config context -> `useConfig`), `hooks/useWorkspace.ts` (workspace context -> `useWorkspaceActions`). Old hooks are NOT deleted yet -- they remain importable until all consumers are migrated in Phase 3 scene rebuilds.
  - **Verify**: `cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npx tsc --noEmit && echo "T1.12 OK"`
---

## Phase 2: Design System Components

Build the reusable visual primitives. Each component follows ROSEDUST v2 tokens (04-DESIGN-SYSTEM.md).

### 2.1 CSS Foundation

- [x] **T2.1: Consolidate keyframes into `styles/animations.css`** ✓ DONE
  - Created `styles/animations.css` with 4 shared keyframes: `fadeIn`, `pulse-dot`, `benchlive-pulse`, `status-blink`
  - Removed duplicate keyframes from: TopNav.css, Explorer.css, Bench.css, Builder.css, Terminal.css
  - Imported in main.tsx

- [x] **T2.2: Replace hardcoded colors in TSX with CSS vars** ✓ DONE
  - Replaced 121/124 hardcoded hex colors across 23 TSX files with `var(--token)` or `getCssVar('--token')`
  - Added status color tokens to rosedust.css (`--status-active/success/error/warning/blocked`, `--border-strong`)
  - 3 remaining are unique chart palette colors with no token match (marked TODO)
  - Key replacements:
    - `#2dd4bf` → `var(--status-active)` (8+ instances)
    - `#4ade80` → `var(--status-success)` (6+ instances)
    - `#fb7185` → `var(--status-error)` (5+ instances)
    - `#fbbf24` → `var(--status-warning)` (4+ instances)
    - `rgba(255,255,255,0.07)` → `var(--border)` (20+ instances)
    - `#0a0810`/`#080810`/`#060608` → `var(--bg-raised)`/`var(--bg-mid)`/`var(--bg-void)` (~15 instances)
  - For canvas contexts: use `getCssVar()` from `lib/color.ts` (T1.1)
  - **Verify**: `grep -rn '#2dd4bf\|#4ade80\|#fb7185\|#fbbf24' src/ --include='*.tsx'` returns zero results.

- [x] **T2.3: Rewrite Settings.css to use ROSEDUST tokens** ✓ DONE
  - Replaced all alien variables (`--surface-0`, `--accent`, etc.) with ROSEDUST tokens

- [x] **T2.4: Fix TerminalPane.css hardcoded background** ✓ DONE
  - Replaced `#0e0c10` → `var(--bg-deeper)`

- [x] **T2.5: Resolve scrollbar conflict** ✓ DONE
  - Removed duplicate scrollbar styles from global.css; rosedust.css is canonical

- [x] **T2.6: Fix PrdPipelinePanel.css danger color mismatch** ✓ DONE
  - Replaced `#cc6f6f` → `var(--danger)` (3 instances)

### 2.2 Cell System

- [x] **T2.7: Create `cells/Cell.tsx` — base cell container** ✓ DONE
  - Glass background, status ring with glow, hover lift, selected state, LED identity header
  - Cell.css with entrance animation, specular highlight, 5 status variants

- [x] **T2.8: Create `cells/CellGrid.tsx` — responsive grid** ✓ DONE
  - CSS grid auto-fill with staggered fadeUp entrance (--i * 40ms)

- [x] **T2.9: Create entity cell renderers** ✓ DONE
  - TaskCell, AgentCell, PlanCell, EpisodeCell, BenchRunCell — all use Cell + design primitives (Badge, StatusBadge, GateBar)
  - Barrel export in cells/index.ts

### 2.3 Layout Primitives

- [x] **T2.10: Create `layout/DataSurface.tsx` — universal state wrapper** ✓ DONE
  - Created `components/layout/DataSurface.tsx` + CSS — generic `<T>` wrapper handling loading/error/empty/content
  - Also created: `Stack.tsx`, `PageShell.tsx` + CSS, `ScrollArea.tsx` + CSS, barrel `index.ts`

- [x] **T2.11: Create `layout/PhaseRail.tsx` — horizontal step indicator** ✓ DONE
  - Done/current/pending/failed visual states, dot scale-up + line draw animations
  - Rose-glow pulsing for current phase

- [x] **T2.12: Create `layout/MetricStrip.tsx` — live metric bar** ✓ DONE
  - Uses AnimatedNumber from motion system, Fraunces italic 24px values, mono labels

- [x] **T2.13: Create `layout/SplitView.tsx` — resizable two-pane** ✓ DONE
  - Created with draggable divider, pointer capture, min/max % constraints, horizontal + vertical support
  - Also created `Tabs.tsx` + CSS (horizontal tab nav with rose-glow active state, badge counts)

### 2.4 Design Components

- [x] **T2.14: Create `design/StatusBadge.tsx` — universal status indicator** ✓ DONE
  - Created `components/design/StatusBadge.tsx` + CSS — 6 statuses (idle/active/success/error/blocked/warning)
  - Dot + icon (○/◉/✓/✕/⬡/⚠) + label, active pulses via pulse-dot animation

- [x] **T2.15: Create `design/EmptyState.tsx` + `design/ErrorState.tsx`** ✓ DONE
  - EmptyState: centered message + action + hint, fadeIn entrance
  - ErrorState: expandable details, retry button, error icon
  - Also created: Skeleton (text/rect/circle/pane + shimmer), Pulse (LED dot), Badge (5 variants), barrel index.ts

- [x] **T2.16: Create `design/GateBar.tsx` — gate status strip** ✓ DONE
  - Pass: ✓ success + scale-up, Fail: ✕ error + flash, Running: ◉ bone + pulse, Pending: ○ ghost, Skipped: —
  - Mono 10px uppercase, gate-scale-up + gate-error-flash component-local keyframes

### 2.5 Motion System

- [x] **T2.17: Create `motion/tokens.ts` — animation constants** ✓ DONE
  - springs, durations, STAGGER_MS, easings, variants — pure TypeScript, zero deps

- [x] **T2.18: Create `motion/AnimatedNumber.tsx` — spring-animated number** ✓ DONE
  - Pure rAF + cubic ease-out interpolation (no framer-motion), flash effect on value change

- [x] **T2.19: Create `motion/AnimatedList.tsx` — list with enter/exit** ✓ DONE
  - Generic AnimatedList<T> with CSS fadeUp + 40ms stagger, Transition wrapper component
  - Added scaleIn + slideRight keyframes to animations.css

---

## Phase 3: Scene Rebuilds

Rebuild all pages using DataHub + Cell + Motion primitives. Split monolithic files.
Every file path below is relative to `demo/demo-app/src/`.

### 3.1 Orchestrate (Replace Demo.tsx — 834L)

---

- [ ] **T3.1: Create scenario registry (config-only metadata)**

  **Read first:**
  - `src/lib/scenarios.ts` lines 36-77 (ScenarioStep, ScenarioContext, Scenario interfaces)
  - `src/lib/scenarios.ts` lines 1886-1900 (export array — 13 scenario IDs)
  - `src/lib/prd-pipeline-types.ts` lines 1-60 (PipelinePhase, PipelineDemoState types)

  **Create:** `src/lib/scenario-registry.ts`

  ```ts
  // --- src/lib/scenario-registry.ts ---

  export type ScenarioComplexity = 'simple' | 'medium' | 'complex';

  export interface ScenarioMeta {
    id: string;            // matches Scenario.id in scenarios.ts
    label: string;         // human name for card title
    description: string;   // 1-2 sentence description for card body
    complexity: ScenarioComplexity;
    phases: string[];      // phase names for PhaseRail
    estimatedDuration: number; // seconds at 1x speed
    tags: string[];        // for filtering/grouping: 'prd', 'bench', 'model', 'knowledge', 'chain'
  }

  export const SCENARIO_REGISTRY: ScenarioMeta[] = [
    {
      id: 'prd-pipeline',
      label: 'PRD Pipeline',
      description: 'Full idea-to-code workflow: draft PRD, research, generate plan, execute tasks, validate with gates.',
      complexity: 'complex',
      phases: ['Idea', 'Draft', 'Research', 'Plan', 'Execute', 'Gate', 'Done'],
      estimatedDuration: 45,
      tags: ['prd'],
    },
    {
      id: 'prd-research-loop',
      label: 'Research Loop',
      description: 'Research-enhanced PRD generation with Perplexity-powered context enrichment.',
      complexity: 'medium',
      phases: ['Draft', 'Research', 'Enhance', 'Done'],
      estimatedDuration: 30,
      tags: ['prd'],
    },
    {
      id: 'race',
      label: 'Model Race',
      description: 'Side-by-side model comparison on identical prompts with cost and quality tracking.',
      complexity: 'medium',
      phases: ['Configure', 'Race', 'Score', 'Done'],
      estimatedDuration: 25,
      tags: ['model', 'bench'],
    },
    {
      id: 'gate-retry',
      label: 'Gate Retry',
      description: 'Demonstrates gate failure detection and automatic replan-retry loop.',
      complexity: 'medium',
      phases: ['Execute', 'Gate Fail', 'Replan', 'Retry', 'Pass'],
      estimatedDuration: 30,
      tags: ['prd'],
    },
    {
      id: 'providers',
      label: 'Provider Health',
      description: 'Iterates all configured providers and checks health, latency, and model availability.',
      complexity: 'simple',
      phases: ['Scan', 'Test', 'Report'],
      estimatedDuration: 15,
      tags: ['model'],
    },
    {
      id: 'provider-race',
      label: 'Provider Race',
      description: 'Concurrent provider benchmark: same prompt to multiple backends, first-to-finish wins.',
      complexity: 'medium',
      phases: ['Configure', 'Race', 'Compare', 'Done'],
      estimatedDuration: 20,
      tags: ['model', 'bench'],
    },
    {
      id: 'explore',
      label: 'Code Explorer',
      description: 'Code intelligence walkthrough: index build, semantic search, dependency graph.',
      complexity: 'simple',
      phases: ['Index', 'Search', 'Graph', 'Done'],
      estimatedDuration: 20,
      tags: ['knowledge'],
    },
    {
      id: 'knowledge-accumulation',
      label: 'Knowledge Accumulation',
      description: 'Shows neuro store ingestion, distillation, tier progression, and query.',
      complexity: 'medium',
      phases: ['Ingest', 'Distill', 'Promote', 'Query', 'Done'],
      estimatedDuration: 25,
      tags: ['knowledge'],
    },
    {
      id: 'dream-consolidation',
      label: 'Dream Consolidation',
      description: 'Offline dream cycle: hypnagogia, imagination, consolidation, journal entry.',
      complexity: 'complex',
      phases: ['Hypnagogia', 'Imagine', 'Consolidate', 'Journal', 'Done'],
      estimatedDuration: 35,
      tags: ['knowledge'],
    },
    {
      id: 'chat',
      label: 'Agent Chat',
      description: 'Interactive chat session with streaming response and tool calls.',
      complexity: 'simple',
      phases: ['Connect', 'Chat', 'Done'],
      estimatedDuration: 15,
      tags: ['model'],
    },
    {
      id: 'knowledge-transfer',
      label: 'Knowledge Transfer',
      description: 'Mesh knowledge sync between agents with custody verification.',
      complexity: 'complex',
      phases: ['Source', 'Transfer', 'Verify', 'Done'],
      estimatedDuration: 30,
      tags: ['knowledge', 'chain'],
    },
    {
      id: 'chain-intelligence',
      label: 'Chain Intelligence',
      description: 'Chain witness anchoring with HDC fingerprints and custody audit.',
      complexity: 'complex',
      phases: ['Fingerprint', 'Anchor', 'Verify', 'Done'],
      estimatedDuration: 35,
      tags: ['chain'],
    },
    {
      id: 'mirage',
      label: 'Mirage Deploy',
      description: 'Deploy pipeline: build, test, containerize, deploy to Mirage endpoint.',
      complexity: 'medium',
      phases: ['Build', 'Test', 'Container', 'Deploy', 'Done'],
      estimatedDuration: 30,
      tags: ['chain'],
    },
  ];

  /** Look up a scenario by ID. Returns undefined if not found. */
  export function getScenarioMeta(id: string): ScenarioMeta | undefined {
    return SCENARIO_REGISTRY.find((s) => s.id === id);
  }

  /** Get scenarios filtered by tag. */
  export function getScenariosByTag(tag: string): ScenarioMeta[] {
    return SCENARIO_REGISTRY.filter((s) => s.tags.includes(tag));
  }
  ```

  **Implementation steps:**
  1. Create the file at `src/lib/scenario-registry.ts` with the exact content above.
  2. The 13 `id` values must match the 13 scenario objects exported from `src/lib/scenarios.ts` lines 1886-1900: `prdPipeline.id`, `prdResearchLoop.id`, `race.id`, `gateRetry.id`, `providers.id`, `providerRace.id`, `explore.id`, `knowledgeAccumulation.id`, `dreamConsolidation.id`, `chat.id`, `knowledgeTransfer.id`, `chainIntelligence.id`, `mirage.id`. Check each ID string against the `id` field in each scenario object.
  3. No runtime code goes here. This is pure config metadata.

  **Verify:** `npx tsc --noEmit` passes. `grep -c "id:" src/lib/scenario-registry.ts` returns 13.

---

- [ ] **T3.2: Extract terminal orchestration + split scenario runners**

  **Read first:**
  - `src/lib/scenarios.ts` lines 79-99 (rawSleep, stripAnsi, compactTime helpers)
  - `src/lib/terminal-session.ts` full file (321L — setSpeedMultiplier, resolveRoko, getRoko, setupWorkspace, joinWorkspace, enterWorkspace, showCmd, trackMetrics)
  - `src/lib/scenarios.ts` lines 100-200 (first scenario: prdPipeline run function)
  - `src/pages/Demo.tsx` lines 241-297 (buildContext callback)
  - `src/pages/Demo.tsx` lines 366-438 (handlePlay function)

  **Create:** `src/lib/terminal-orchestration.ts`

  ```ts
  // --- src/lib/terminal-orchestration.ts ---
  // Re-exports shared utilities that scenario runners need.
  // This file replaces direct imports from terminal-session.ts in runner files.

  export {
    setSpeedMultiplier,
    resolveRoko,
    getRoko,
    setupWorkspace,
    joinWorkspace,
    enterWorkspace,
    showCmd,
    trackMetrics,
  } from './terminal-session';

  // Re-export helpers that were inline in scenarios.ts
  export { rawSleep, stripAnsi, compactTime } from './scenarios';
  ```

  Wait — `rawSleep`, `stripAnsi`, `compactTime` are currently defined inline in `scenarios.ts` (lines 79-99). They need to be extracted first.

  **Modify:** `src/lib/scenarios.ts`
  1. Move the three helper functions (`rawSleep`, `stripAnsi`, `compactTime` — lines 79-99) into a new file `src/lib/scenario-helpers.ts`.
  2. In `scenarios.ts`, replace the function bodies with `export { rawSleep, stripAnsi, compactTime } from './scenario-helpers';`
  3. In `terminal-orchestration.ts`, change the re-export source to `'./scenario-helpers'`.

  **Create:** `src/lib/scenario-helpers.ts`

  ```ts
  // --- src/lib/scenario-helpers.ts ---

  /** Sleep that bypasses playback speed multiplier. */
  export function rawSleep(ms: number): Promise<void> {
    return new Promise((r) => setTimeout(r, ms));
  }

  /** Strip ANSI escape codes from terminal output. */
  export function stripAnsi(s: string): string {
    return s.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '');
  }

  /** Format milliseconds as compact time string. */
  export function compactTime(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m${Math.round(s % 60)}s`;
  }
  ```

  **Create:** `src/lib/scenario-runners/` directory with the following files. Each file exports a single `Scenario` object (same interface as defined in `scenarios.ts` lines 56-77).

  Split plan for `scenarios.ts` (1900L):
  - `src/lib/scenario-runners/prd-pipeline.ts` — extracts the `prdPipeline` scenario object
  - `src/lib/scenario-runners/prd-research-loop.ts` — extracts `prdResearchLoop`
  - `src/lib/scenario-runners/race.ts` — extracts `race`
  - `src/lib/scenario-runners/gate-retry.ts` — extracts `gateRetry`
  - `src/lib/scenario-runners/providers.ts` — extracts `providers`
  - `src/lib/scenario-runners/provider-race.ts` — extracts `providerRace`
  - `src/lib/scenario-runners/explore.ts` — extracts `explore`
  - `src/lib/scenario-runners/knowledge-accumulation.ts` — extracts `knowledgeAccumulation`
  - `src/lib/scenario-runners/dream-consolidation.ts` — extracts `dreamConsolidation`
  - `src/lib/scenario-runners/chat.ts` — extracts `chat`
  - `src/lib/scenario-runners/knowledge-transfer.ts` — extracts `knowledgeTransfer`
  - `src/lib/scenario-runners/chain-intelligence.ts` — extracts `chainIntelligence`
  - `src/lib/scenario-runners/mirage.ts` — extracts `mirage`
  - `src/lib/scenario-runners/index.ts` — re-exports all 13 + aggregated array

  **Create:** `src/lib/scenario-runners/index.ts`

  ```ts
  // --- src/lib/scenario-runners/index.ts ---
  import type { Scenario } from '../scenarios';

  import { prdPipeline } from './prd-pipeline';
  import { prdResearchLoop } from './prd-research-loop';
  import { race } from './race';
  import { gateRetry } from './gate-retry';
  import { providers } from './providers';
  import { providerRace } from './provider-race';
  import { explore } from './explore';
  import { knowledgeAccumulation } from './knowledge-accumulation';
  import { dreamConsolidation } from './dream-consolidation';
  import { chat } from './chat';
  import { knowledgeTransfer } from './knowledge-transfer';
  import { chainIntelligence } from './chain-intelligence';
  import { mirage } from './mirage';

  export {
    prdPipeline, prdResearchLoop, race, gateRetry, providers, providerRace,
    explore, knowledgeAccumulation, dreamConsolidation, chat, knowledgeTransfer,
    chainIntelligence, mirage,
  };

  export const allScenarios: Scenario[] = [
    prdPipeline, prdResearchLoop, race, gateRetry, providers, providerRace,
    explore, knowledgeAccumulation, dreamConsolidation, chat, knowledgeTransfer,
    chainIntelligence, mirage,
  ];
  ```

  **Per runner file template** (example for `prd-pipeline.ts`):

  ```ts
  // --- src/lib/scenario-runners/prd-pipeline.ts ---
  import type { Scenario, ScenarioContext } from '../scenarios';
  import { rawSleep, stripAnsi, compactTime } from '../scenario-helpers';
  import { showCmd, enterWorkspace, trackMetrics } from '../terminal-session';

  export const prdPipeline: Scenario = {
    id: 'prd-pipeline',
    label: 'PRD Pipeline',
    // ... copy the full run() function body from scenarios.ts for this scenario
  };
  ```

  **Implementation steps:**
  1. Create `src/lib/scenario-helpers.ts` with the 3 helper functions (copied exactly from `scenarios.ts` lines 79-99).
  2. Create `src/lib/scenario-runners/` directory.
  3. For each of the 13 scenarios in `scenarios.ts`:
     a. Find the scenario object (search for `export const scenarioName: Scenario = {`).
     b. Copy the entire object into a new file under `src/lib/scenario-runners/`.
     c. Add imports for any helpers/functions the scenario uses (check `rawSleep`, `stripAnsi`, `compactTime`, `showCmd`, `enterWorkspace`, `trackMetrics`, `resolveRoko`, `getRoko`, `setupWorkspace`, `joinWorkspace`).
  4. Create `src/lib/scenario-runners/index.ts` with the re-export barrel.
  5. In `scenarios.ts`, replace the 13 scenario object definitions with re-exports: `export { allScenarios } from './scenario-runners';` and keep only the type definitions (`ScenarioStep`, `ScenarioContext`, `Scenario` interfaces at lines 36-77).
  6. Create `src/lib/terminal-orchestration.ts` re-exporting from both `terminal-session` and `scenario-helpers`.
  7. Update all existing imports of scenarios (search with `grep -rn "from.*scenarios" src/` — currently `Demo.tsx` line 7 imports the scenarios array).

  **Verify:** `npx tsc --noEmit` passes. Each file in `src/lib/scenario-runners/` is < 250L. `scenarios.ts` is < 100L (types + re-exports only).

---

- [ ] **T3.3: Build Orchestrate scene shell + idle phase (ScenarioSelector)**

  **Read first:**
  - `src/pages/Demo.tsx` lines 56-66 (top-level state: selectedScenario, entries, metrics, etc.)
  - `src/pages/Demo.tsx` lines 325-365 (selectScenario + handlePipelineExampleSelect)
  - `src/pages/Demo.tsx` lines 561-610 (JSX: outer layout structure)
  - `src/components/layout/PhaseRail.tsx` full file (47L — phases, current, failed props)
  - `src/components/cells/Cell.tsx` full file (status, identity, actions, onClick, children)
  - `src/components/design/StatusBadge.tsx` full file (status, label, size props)
  - `src/lib/scenario-registry.ts` (created in T3.1)

  **Create:** `src/scenes/Orchestrate.tsx`

  ```tsx
  // --- src/scenes/Orchestrate.tsx ---
  import { useState, useCallback, useRef, useEffect } from 'react';
  import type { ScenarioMeta } from '../lib/scenario-registry';
  import { SCENARIO_REGISTRY, getScenarioMeta } from '../lib/scenario-registry';
  import type { Scenario, ScenarioContext } from '../lib/scenarios';
  import { allScenarios } from '../lib/scenario-runners';
  import { PlaybackController, TimelineStepper } from '../lib/playback-controller';
  import { PhaseRail } from '../components/layout/PhaseRail';
  import { MetricStrip } from '../components/layout/MetricStrip';
  import { ScenarioSelector } from '../components/orchestrate/ScenarioSelector';
  import { ArtifactPanel } from '../components/orchestrate/ArtifactPanel';
  import { TaskBoard } from '../components/orchestrate/TaskBoard';
  import { CompletionSummary } from '../components/orchestrate/CompletionSummary';
  import { PlaybackBar } from '../components/orchestrate/PlaybackBar';
  import './Orchestrate.css';

  type OrchestratePhase = 'idle' | 'running' | 'complete';

  interface OrchestrateState {
    phase: OrchestratePhase;
    selectedId: string | null;
    currentPhaseIndex: number;
    failedPhaseIndex: number | undefined;
    metrics: { cost: number; tokens: number; elapsed: number; tasks: number; gates: number };
    entries: Array<{ ts: number; text: string; stream: string }>;
  }

  const INITIAL_METRICS = { cost: 0, tokens: 0, elapsed: 0, tasks: 0, gates: 0 };

  export function Orchestrate() {
    const [state, setState] = useState<OrchestrateState>({
      phase: 'idle',
      selectedId: null,
      currentPhaseIndex: 0,
      failedPhaseIndex: undefined,
      metrics: { ...INITIAL_METRICS },
      entries: [],
    });

    const controllerRef = useRef(new PlaybackController());
    const stepperRef = useRef(new TimelineStepper());
    const abortRef = useRef<AbortController | null>(null);

    const meta: ScenarioMeta | undefined = state.selectedId
      ? getScenarioMeta(state.selectedId)
      : undefined;

    // --- handlers wired in T3.4 (handlePlay) and T3.6 (playback) ---

    const handleSelect = useCallback((id: string) => {
      setState((prev) => ({ ...prev, selectedId: id, phase: 'idle' }));
    }, []);

    // ... handlePlay, handlePause, handleStep, handleReset, handleSpeedChange
    // (wired in T3.4 and T3.6 — leave as TODO stubs for now)

    return (
      <div className="orchestrate">
        {/* Phase rail at top */}
        <PhaseRail
          phases={meta?.phases ?? ['Select']}
          current={state.currentPhaseIndex}
          failed={state.failedPhaseIndex}
          className="orchestrate__rail"
        />

        {/* Metric strip below rail */}
        <MetricStrip
          metrics={[
            { label: 'Cost', value: state.metrics.cost, format: (n) => `$${n.toFixed(4)}` },
            { label: 'Tokens', value: state.metrics.tokens, format: (n) => n.toLocaleString() },
            { label: 'Time', value: state.metrics.elapsed, format: (n) => `${n.toFixed(1)}s`, suffix: 's' },
            { label: 'Tasks', value: state.metrics.tasks },
            { label: 'Gates', value: state.metrics.gates },
          ]}
          className="orchestrate__metrics"
        />

        {/* Main content area — switches on phase */}
        <div className="orchestrate__body">
          {state.phase === 'idle' && (
            <ScenarioSelector
              scenarios={SCENARIO_REGISTRY}
              selectedId={state.selectedId}
              onSelect={handleSelect}
            />
          )}
          {state.phase === 'running' && (
            <div className="orchestrate__running">
              {/* ArtifactPanel + TaskBoard + Terminal — wired in T3.4 */}
            </div>
          )}
          {state.phase === 'complete' && (
            <CompletionSummary metrics={state.metrics} entries={state.entries} />
          )}
        </div>

        {/* Playback bar pinned to bottom — wired in T3.6 */}
        <PlaybackBar
          controller={controllerRef.current}
          phase={state.phase}
          selectedId={state.selectedId}
        />
      </div>
    );
  }
  ```

  **Create:** `src/components/orchestrate/ScenarioSelector.tsx`

  ```tsx
  // --- src/components/orchestrate/ScenarioSelector.tsx ---
  import type { ScenarioMeta, ScenarioComplexity } from '../../lib/scenario-registry';
  import { Cell } from '../cells/Cell';
  import { Badge } from '../design/Badge';
  import './ScenarioSelector.css';

  const COMPLEXITY_COLOR: Record<ScenarioComplexity, string> = {
    simple: 'var(--sage)',
    medium: 'var(--dream)',
    complex: 'var(--rose-bright)',
  };

  interface ScenarioSelectorProps {
    scenarios: ScenarioMeta[];
    selectedId: string | null;
    onSelect: (id: string) => void;
  }

  export function ScenarioSelector({ scenarios, selectedId, onSelect }: ScenarioSelectorProps) {
    return (
      <div className="scenario-selector">
        <h2 className="scenario-selector__title">Select Scenario</h2>
        <div className="scenario-selector__grid">
          {scenarios.map((s) => (
            <Cell
              key={s.id}
              status={selectedId === s.id ? 'active' : 'idle'}
              identity={s.id}
              onClick={() => onSelect(s.id)}
              selected={selectedId === s.id}
            >
              <div className="scenario-selector__card-label">{s.label}</div>
              <div className="scenario-selector__card-desc">{s.description}</div>
              <div className="scenario-selector__card-meta">
                <Badge variant="info">{s.complexity}</Badge>
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-xs)' }}>
                  ~{s.estimatedDuration}s
                </span>
              </div>
            </Cell>
          ))}
        </div>
      </div>
    );
  }
  ```

  **Create:** `src/components/orchestrate/ScenarioSelector.css`

  ```css
  /* --- src/components/orchestrate/ScenarioSelector.css --- */
  .scenario-selector__title {
    font-family: var(--display);
    font-size: 30px;
    font-weight: 300;
    color: var(--bone-bright);
    letter-spacing: -0.01em;
    margin-bottom: 24px;
  }
  .scenario-selector__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }
  .scenario-selector__card-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-primary);
  }
  .scenario-selector__card-desc {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    margin-top: 6px;
    line-height: 1.4;
  }
  .scenario-selector__card-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
  }
  ```

  **Create:** `src/scenes/Orchestrate.css`

  ```css
  /* --- src/scenes/Orchestrate.css --- */
  .orchestrate {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-void);
    padding: 16px 24px;
    gap: 16px;
  }
  .orchestrate__rail {
    flex-shrink: 0;
  }
  .orchestrate__metrics {
    flex-shrink: 0;
  }
  .orchestrate__body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .orchestrate__running {
    display: flex;
    gap: 16px;
    height: 100%;
  }
  ```

  **Implementation steps:**
  1. Create `src/scenes/` directory if it does not exist.
  2. Create `src/scenes/Orchestrate.tsx` with the shell above. Leave `handlePlay` / `handlePause` / `handleStep` / `handleReset` / `handleSpeedChange` as TODO stubs (they are wired in T3.4 and T3.6).
  3. Create `src/scenes/Orchestrate.css`.
  4. Create `src/components/orchestrate/` directory.
  5. Create `src/components/orchestrate/ScenarioSelector.tsx` and `ScenarioSelector.css`.
  6. CSS tokens used: `--bg-void`, `--bone-bright`, `--text-primary`, `--text-secondary`, `--text-sm`, `--text-xs`, `--display` (Fraunces), `--sage`, `--dream`, `--rose-bright`. All defined in `src/styles/rosedust.css`.

  **Verify:** `npx tsc --noEmit` passes. The Orchestrate scene renders at its route (add temp route in `main.tsx` if needed). ScenarioSelector shows 13 cards. Click highlights one. PhaseRail displays phases for selected scenario.

---

- [ ] **T3.4: Build Orchestrate scene — running phase (ArtifactPanel + TaskBoard)**

  **Read first:**
  - `src/pages/Demo.tsx` lines 366-438 (handlePlay — scenario run function, context wiring)
  - `src/pages/Demo.tsx` lines 241-297 (buildContext — setMetric, setGate, logCommand, setPipeline, etc.)
  - `src/pages/Demo.tsx` lines 636-743 (sidebar conditionals: PrdPipelinePanel, knowledge, chain, default)
  - `src/components/PrdPipelinePanel.tsx` full file (reference for PRD/plan/task rendering)
  - `src/lib/prd-pipeline-types.ts` lines 1-60 (PipelinePhase, PipelineTask, PipelineDemoState)
  - `src/components/layout/SplitView.tsx` full file (left, right, defaultSplit, minLeft, maxLeft)
  - `src/components/design/GateBar.tsx` full file (gates, size props)
  - `src/components/layout/MetricStrip.tsx` full file (metrics array with AnimatedNumber)

  **Create:** `src/components/orchestrate/ArtifactPanel.tsx`

  ```tsx
  // --- src/components/orchestrate/ArtifactPanel.tsx ---
  import { Pane } from '../Pane';
  import type { PipelineDemoState } from '../../lib/prd-pipeline-types';
  import './ArtifactPanel.css';

  interface ArtifactPanelProps {
    pipeline: PipelineDemoState | null;
  }

  export function ArtifactPanel({ pipeline }: ArtifactPanelProps) {
    if (!pipeline) {
      return (
        <Pane title="Artifacts" className="artifact-panel artifact-panel--empty">
          <p className="artifact-panel__placeholder">
            Start a scenario to see artifacts here.
          </p>
        </Pane>
      );
    }

    return (
      <Pane title="Artifacts" className="artifact-panel">
        {/* PRD section */}
        {pipeline.prd && (
          <section className="artifact-panel__section">
            <h3 className="artifact-panel__heading">PRD</h3>
            <div className="artifact-panel__content">{pipeline.prd.title}</div>
          </section>
        )}

        {/* Tasks section */}
        {pipeline.tasks.length > 0 && (
          <section className="artifact-panel__section">
            <h3 className="artifact-panel__heading">Tasks</h3>
            {pipeline.tasks.map((t) => (
              <div key={t.id} className={`artifact-panel__task artifact-panel__task--${t.status}`}>
                <span className="artifact-panel__task-id">{t.id}</span>
                <span className="artifact-panel__task-title">{t.title}</span>
                <span className="artifact-panel__task-status">{t.status}</span>
              </div>
            ))}
          </section>
        )}

        {/* Event log */}
        {pipeline.events.length > 0 && (
          <section className="artifact-panel__section">
            <h3 className="artifact-panel__heading">Events</h3>
            <div className="artifact-panel__events">
              {pipeline.events.slice(-20).map((e, i) => (
                <div key={i} className="artifact-panel__event">{e.message}</div>
              ))}
            </div>
          </section>
        )}
      </Pane>
    );
  }
  ```

  **Create:** `src/components/orchestrate/TaskBoard.tsx`

  ```tsx
  // --- src/components/orchestrate/TaskBoard.tsx ---
  import { Pane } from '../Pane';
  import { GateBar } from '../design/GateBar';
  import { StatusBadge } from '../design/StatusBadge';
  import type { PipelineTask } from '../../lib/prd-pipeline-types';
  import './TaskBoard.css';

  type GateStatus = 'idle' | 'active' | 'success' | 'error';

  interface TaskBoardProps {
    tasks: PipelineTask[];
    gates: Array<{ name: string; status: GateStatus }>;
  }

  export function TaskBoard({ tasks, gates }: TaskBoardProps) {
    return (
      <Pane title="Task Board" className="task-board">
        {/* Gate summary bar */}
        {gates.length > 0 && (
          <div className="task-board__gates">
            <GateBar gates={gates} size="sm" />
          </div>
        )}

        {/* Task list */}
        <div className="task-board__list">
          {tasks.map((t) => (
            <div key={t.id} className={`task-board__item task-board__item--${t.status}`}>
              <StatusBadge
                status={t.status === 'done' ? 'success' : t.status === 'running' ? 'active' : t.status === 'failed' ? 'error' : 'idle'}
                size="sm"
              />
              <span className="task-board__item-id">{t.id}</span>
              <span className="task-board__item-title">{t.title}</span>
            </div>
          ))}
        </div>
      </Pane>
    );
  }
  ```

  **Create:** CSS files `src/components/orchestrate/ArtifactPanel.css` and `src/components/orchestrate/TaskBoard.css`

  For `ArtifactPanel.css`:
  ```css
  .artifact-panel { overflow-y: auto; }
  .artifact-panel--empty { opacity: 0.5; }
  .artifact-panel__placeholder { color: var(--text-secondary); font-size: var(--text-sm); }
  .artifact-panel__section { margin-bottom: 16px; }
  .artifact-panel__heading { font-size: var(--text-xs); font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px; }
  .artifact-panel__content { color: var(--text-primary); font-size: var(--text-sm); }
  .artifact-panel__task { display: flex; gap: 8px; align-items: center; padding: 4px 0; font-size: var(--text-xs); }
  .artifact-panel__task--done { opacity: 0.6; }
  .artifact-panel__task-id { color: var(--text-secondary); font-family: var(--mono); }
  .artifact-panel__task-title { color: var(--text-primary); flex: 1; }
  .artifact-panel__task-status { color: var(--text-secondary); }
  .artifact-panel__events { max-height: 200px; overflow-y: auto; }
  .artifact-panel__event { font-size: var(--text-xs); color: var(--text-secondary); padding: 2px 0; font-family: var(--mono); }
  ```

  For `TaskBoard.css`:
  ```css
  .task-board { overflow-y: auto; }
  .task-board__gates { margin-bottom: 12px; }
  .task-board__list { display: flex; flex-direction: column; gap: 4px; }
  .task-board__item { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 6px; background: var(--glass-bg); }
  .task-board__item--running { background: var(--glass-bg-hover); }
  .task-board__item--done { opacity: 0.6; }
  .task-board__item--failed { border-left: 2px solid var(--rose-bright); }
  .task-board__item-id { font-family: var(--mono); font-size: var(--text-xs); color: var(--text-secondary); min-width: 32px; }
  .task-board__item-title { font-size: var(--text-sm); color: var(--text-primary); }
  ```

  **Modify:** `src/scenes/Orchestrate.tsx`
  1. Wire `handlePlay` function: look up scenario runner from `allScenarios` by `state.selectedId`, build `ScenarioContext` (see `Demo.tsx` lines 241-297 for exact fields), set `phase: 'running'`, call `scenario.run(context)`, on completion set `phase: 'complete'`.
  2. Fill in the `{state.phase === 'running' && ...}` block with `<SplitView>` containing `<ArtifactPanel>` on left and terminal + `<TaskBoard>` on right.

  **Implementation steps:**
  1. Create `src/components/orchestrate/ArtifactPanel.tsx` and `ArtifactPanel.css`.
  2. Create `src/components/orchestrate/TaskBoard.tsx` and `TaskBoard.css`.
  3. In `Orchestrate.tsx`, implement `handlePlay`:
     a. Create AbortController, store in `abortRef`.
     b. Find the matching `Scenario` from `allScenarios.find(s => s.id === state.selectedId)`.
     c. Build `ScenarioContext` object with `entries`, `playback` (from controllerRef), `timeline` (from stepperRef), `setMetric`, `setGate`, `logCommand`, etc. — mirror `Demo.tsx` lines 241-297 exactly.
     d. Set `phase: 'running'`, `currentPhaseIndex: 0`.
     e. `await scenario.run(context)`.
     f. Set `phase: 'complete'`.
  4. Render `SplitView` with `ArtifactPanel` on left (40%) and `TaskBoard` on right (60%).
  5. CSS tokens: `--glass-bg`, `--glass-bg-hover`, `--rose-bright`, `--text-primary`, `--text-secondary`, `--text-xs`, `--text-sm`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Start a scenario. ArtifactPanel shows PRD/tasks. TaskBoard shows task list with status badges. Gates render.

---

- [ ] **T3.5: Build Orchestrate scene — completion phase (CompletionSummary)**

  **Read first:**
  - `src/pages/Demo.tsx` lines 440-470 (pause/step/reset/speed handlers — to understand post-run state)
  - `src/components/Mosaic.tsx` full file (62L — MosaicCell props: label, value, sub, color, mono, icon; Mosaic: columns, children)
  - `src/components/motion/AnimatedNumber.tsx` (value, format props)

  **Create:** `src/components/orchestrate/CompletionSummary.tsx`

  ```tsx
  // --- src/components/orchestrate/CompletionSummary.tsx ---
  import { Mosaic, MosaicCell } from '../Mosaic';
  import { AnimatedNumber } from '../motion/AnimatedNumber';
  import { Pane } from '../Pane';
  import './CompletionSummary.css';

  interface CompletionMetrics {
    cost: number;
    tokens: number;
    elapsed: number;
    tasks: number;
    gates: number;
  }

  interface CompletionEntry {
    ts: number;
    text: string;
    stream: string;
  }

  interface CompletionSummaryProps {
    metrics: CompletionMetrics;
    entries: CompletionEntry[];
  }

  export function CompletionSummary({ metrics, entries }: CompletionSummaryProps) {
    return (
      <div className="completion-summary">
        <h2 className="completion-summary__title">Run Complete</h2>

        {/* Summary mosaic */}
        <Mosaic columns={3}>
          <MosaicCell
            label="Total Cost"
            value={`$${metrics.cost.toFixed(4)}`}
            color="var(--dream)"
            mono
          />
          <MosaicCell
            label="Tokens"
            value={metrics.tokens.toLocaleString()}
            color="var(--sage)"
            mono
          />
          <MosaicCell
            label="Duration"
            value={`${metrics.elapsed.toFixed(1)}s`}
            color="var(--bone)"
            mono
          />
          <MosaicCell
            label="Tasks"
            value={String(metrics.tasks)}
            color="var(--text-primary)"
          />
          <MosaicCell
            label="Gates Passed"
            value={String(metrics.gates)}
            color="var(--sage)"
          />
          <MosaicCell
            label="Entries"
            value={String(entries.length)}
            color="var(--text-secondary)"
          />
        </Mosaic>

        {/* Task log */}
        {entries.length > 0 && (
          <Pane title="Run Log" className="completion-summary__log" flat>
            <div className="completion-summary__entries">
              {entries.slice(-50).map((e, i) => (
                <div key={i} className="completion-summary__entry">
                  <span className="completion-summary__entry-ts">
                    {new Date(e.ts).toLocaleTimeString()}
                  </span>
                  <span className="completion-summary__entry-text">{e.text}</span>
                </div>
              ))}
            </div>
          </Pane>
        )}
      </div>
    );
  }
  ```

  **Create:** `src/components/orchestrate/CompletionSummary.css`

  ```css
  .completion-summary { padding: 16px 0; }
  .completion-summary__title {
    font-family: var(--display);
    font-size: 30px;
    font-weight: 300;
    color: var(--bone-bright);
    letter-spacing: -0.01em;
    margin-bottom: 24px;
  }
  .completion-summary__log { margin-top: 24px; }
  .completion-summary__entries { max-height: 300px; overflow-y: auto; }
  .completion-summary__entry { display: flex; gap: 12px; padding: 3px 0; font-size: var(--text-xs); }
  .completion-summary__entry-ts { color: var(--text-secondary); font-family: var(--mono); min-width: 80px; }
  .completion-summary__entry-text { color: var(--text-primary); font-family: var(--mono); }
  ```

  **Implementation steps:**
  1. Create `src/components/orchestrate/CompletionSummary.tsx` and `CompletionSummary.css`.
  2. Uses `Mosaic` (3 columns) for the metric summary tiles and `Pane` with `flat` prop for the log.
  3. CSS tokens: `--display`, `--bone-bright`, `--dream`, `--sage`, `--bone`, `--text-primary`, `--text-secondary`, `--text-xs`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Complete a scenario run. Summary appears with 6 mosaic tiles. Animated numbers spring in. Run log shows last 50 entries.

---

- [ ] **T3.6: Wire PlaybackController into Orchestrate scene (PlaybackBar)**

  **Read first:**
  - `src/lib/playback-controller.ts` full file (PlaybackController: mode, setMode, reset, waitForStep, advanceStep, setProgress, onProgress, onWaitingChange; TimelineStepper: reset, addStep, setActive, onChange)
  - `src/pages/Demo.tsx` lines 42-46 (singleton instantiation of PlaybackController + TimelineStepper)
  - `src/pages/Demo.tsx` lines 189-196 (controller wiring: mode subscription, speed handling)
  - `src/pages/Demo.tsx` lines 440-470 (pause/step/reset/speed handlers)
  - `src/pages/Demo.tsx` lines 472-492 (keyboard shortcuts: Space, n, r, 1-4 speed)
  - `src/pages/Demo.tsx` lines 748-792 (playback bar JSX)

  **Create:** `src/components/orchestrate/PlaybackBar.tsx`

  ```tsx
  // --- src/components/orchestrate/PlaybackBar.tsx ---
  import { useState, useEffect, useCallback } from 'react';
  import type { PlaybackController } from '../../lib/playback-controller';
  import './PlaybackBar.css';

  const SPEEDS = [0.5, 1, 2, 4] as const;

  interface PlaybackBarProps {
    controller: PlaybackController;
    phase: 'idle' | 'running' | 'complete';
    selectedId: string | null;
    onPlay?: () => void;
    onReset?: () => void;
  }

  export function PlaybackBar({ controller, phase, selectedId, onPlay, onReset }: PlaybackBarProps) {
    const [mode, setMode] = useState(controller.mode);
    const [speed, setSpeed] = useState(1);
    const [progress, setProgress] = useState(0);

    useEffect(() => {
      const unsub = controller.onProgress((p) => setProgress(p));
      return unsub;
    }, [controller]);

    useEffect(() => {
      setMode(controller.mode);
    }, [controller.mode]);

    const handlePlayPause = useCallback(() => {
      if (phase === 'idle' && selectedId) {
        onPlay?.();
      } else if (phase === 'running') {
        controller.setMode(mode === 'play' ? 'pause' : 'play');
        setMode(mode === 'play' ? 'pause' : 'play');
      }
    }, [phase, selectedId, mode, controller, onPlay]);

    const handleStep = useCallback(() => {
      if (phase === 'running') {
        controller.setMode('step');
        setMode('step');
        controller.advanceStep();
      }
    }, [phase, controller]);

    const handleSpeedChange = useCallback((s: number) => {
      setSpeed(s);
      // Speed multiplier is set via setSpeedMultiplier from terminal-session.ts
      // The scenario runner reads it each time it calls sleep.
    }, []);

    const handleReset = useCallback(() => {
      controller.reset();
      setMode('pause');
      setProgress(0);
      onReset?.();
    }, [controller, onReset]);

    // Keyboard shortcuts
    useEffect(() => {
      function onKey(e: KeyboardEvent) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        switch (e.code) {
          case 'Space': e.preventDefault(); handlePlayPause(); break;
          case 'KeyN': handleStep(); break;
          case 'KeyR': handleReset(); break;
          case 'Digit1': handleSpeedChange(0.5); break;
          case 'Digit2': handleSpeedChange(1); break;
          case 'Digit3': handleSpeedChange(2); break;
          case 'Digit4': handleSpeedChange(4); break;
        }
      }
      window.addEventListener('keydown', onKey);
      return () => window.removeEventListener('keydown', onKey);
    }, [handlePlayPause, handleStep, handleReset, handleSpeedChange]);

    return (
      <div className="playback-bar">
        {/* Progress track */}
        <div className="playback-bar__track">
          <div className="playback-bar__fill" style={{ width: `${progress * 100}%` }} />
        </div>

        <div className="playback-bar__controls">
          {/* Play / Pause button */}
          <button
            className="playback-bar__btn playback-bar__btn--primary"
            onClick={handlePlayPause}
            disabled={phase === 'idle' && !selectedId}
          >
            {phase === 'idle' ? 'START' : mode === 'play' ? 'PAUSE' : 'PLAY'}
          </button>

          {/* Step button */}
          <button
            className="playback-bar__btn"
            onClick={handleStep}
            disabled={phase !== 'running'}
          >
            STEP
          </button>

          {/* Reset button */}
          <button
            className="playback-bar__btn"
            onClick={handleReset}
            disabled={phase === 'idle'}
          >
            RESET
          </button>

          {/* Speed selector */}
          <div className="playback-bar__speed">
            {SPEEDS.map((s) => (
              <button
                key={s}
                className={`playback-bar__speed-btn ${speed === s ? 'playback-bar__speed-btn--active' : ''}`}
                onClick={() => handleSpeedChange(s)}
              >
                {s}x
              </button>
            ))}
          </div>

          {/* Keyboard hint */}
          <span className="playback-bar__hint">
            Space: play/pause &middot; N: step &middot; R: reset &middot; 1-4: speed
          </span>
        </div>
      </div>
    );
  }
  ```

  **Create:** `src/components/orchestrate/PlaybackBar.css`

  ```css
  .playback-bar {
    flex-shrink: 0;
    background: var(--glass-bg);
    border-top: 1px solid var(--glass-border);
    padding: 8px 16px;
  }
  .playback-bar__track {
    height: 3px;
    background: var(--bg-surface);
    border-radius: 2px;
    margin-bottom: 8px;
    overflow: hidden;
  }
  .playback-bar__fill {
    height: 100%;
    background: var(--rose-bright);
    transition: width 0.3s ease;
  }
  .playback-bar__controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .playback-bar__btn {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    color: var(--text-primary);
    padding: 4px 12px;
    border-radius: 4px;
    font-size: var(--text-xs);
    font-family: var(--mono);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .playback-bar__btn:hover { background: var(--glass-bg-hover); }
  .playback-bar__btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .playback-bar__btn--primary { background: var(--rose-bright); color: var(--bg-void); border-color: var(--rose-bright); }
  .playback-bar__btn--primary:hover { opacity: 0.9; }
  .playback-bar__speed { display: flex; gap: 4px; margin-left: 8px; }
  .playback-bar__speed-btn {
    background: none;
    border: 1px solid var(--glass-border);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: var(--text-xs);
    font-family: var(--mono);
    cursor: pointer;
  }
  .playback-bar__speed-btn--active { background: var(--glass-bg); color: var(--text-primary); border-color: var(--text-primary); }
  .playback-bar__hint { margin-left: auto; font-size: 11px; color: var(--text-secondary); opacity: 0.6; }
  ```

  **Modify:** `src/scenes/Orchestrate.tsx`
  1. Wire `handlePlay` to pass `onPlay` prop to `PlaybackBar`.
  2. Wire `handleReset` to pass `onReset` prop to `PlaybackBar`.
  3. Import `setSpeedMultiplier` from `'../lib/terminal-session'` and call it when speed changes.
  4. In `handlePlay`, set `controllerRef.current.setMode('play')` before starting the scenario.

  **Implementation steps:**
  1. Create `src/components/orchestrate/PlaybackBar.tsx` and `PlaybackBar.css`.
  2. Wire `PlaybackBar` into `Orchestrate.tsx` with `onPlay={handlePlay}` and `onReset={handleReset}`.
  3. In `Orchestrate.tsx`, implement speed change: `import { setSpeedMultiplier } from '../lib/terminal-session';` then call `setSpeedMultiplier(newSpeed)` in the speed handler.
  4. CSS tokens: `--glass-bg`, `--glass-border`, `--glass-bg-hover`, `--bg-surface`, `--bg-void`, `--rose-bright`, `--text-primary`, `--text-secondary`, `--text-xs`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Start a run. Space pauses/resumes. Speed buttons change timing. N advances one step. R resets to idle. Progress bar fills.

---

### 3.2 Observe (Replace Explorer.tsx — 862L)

---

- [ ] **T3.7: Build Observe scene shell + Status tab**

  **Read first:**
  - `src/pages/Explorer.tsx` lines 12-42 (HealthData, Episode, StateEvent types)
  - `src/pages/Explorer.tsx` lines 47-72 (constants: API_BASE, LABEL_DEFS, PIE_COLORS, THEME)
  - `src/pages/Explorer.tsx` lines 76-125 (helpers: fmtCost, fmtPct, fmtN, fmtDur, fmtTs, scoreColor, drawRoundedRect)
  - `src/pages/Explorer.tsx` lines 131-173 (drawSparkline function — 42L)
  - `src/pages/Explorer.tsx` lines 179-201 (state: 6 useState, 7 useRef)
  - `src/pages/Explorer.tsx` lines 203-223 (refresh + polling useEffect)
  - `src/pages/Explorer.tsx` lines 225-302 (derived data: stats, sparkData, heatmapData computations)
  - `src/pages/Explorer.tsx` lines 603-710 (header JSX + sparkline strip + heatmap)
  - `src/components/layout/Tabs.tsx` full file (tabs: {id, label, badge}[], active, onChange)
  - `src/components/layout/MetricStrip.tsx` full file (35L)

  **Create:** `src/scenes/Observe.tsx`

  ```tsx
  // --- src/scenes/Observe.tsx ---
  import { useState } from 'react';
  import { Tabs } from '../components/layout/Tabs';
  import { StatusTab } from '../components/observe/StatusTab';
  import { FleetTab } from '../components/observe/FleetTab';
  import { KnowledgeTab } from '../components/observe/KnowledgeTab';
  import { RoutingTab } from '../components/observe/RoutingTab';
  import { DreamsTab } from '../components/observe/DreamsTab';
  import './Observe.css';

  const TABS = [
    { id: 'status', label: 'Status' },
    { id: 'fleet', label: 'Fleet' },
    { id: 'knowledge', label: 'Knowledge' },
    { id: 'routing', label: 'Routing' },
    { id: 'dreams', label: 'Dreams' },
  ];

  export function Observe() {
    const [activeTab, setActiveTab] = useState('status');

    return (
      <div className="observe">
        <Tabs tabs={TABS} active={activeTab} onChange={setActiveTab} />

        <div className="observe__body">
          {activeTab === 'status' && <StatusTab />}
          {activeTab === 'fleet' && <FleetTab />}
          {activeTab === 'knowledge' && <KnowledgeTab />}
          {activeTab === 'routing' && <RoutingTab />}
          {activeTab === 'dreams' && <DreamsTab />}
        </div>
      </div>
    );
  }
  ```

  **Create:** `src/scenes/Observe.css`

  ```css
  .observe { display: flex; flex-direction: column; height: 100%; background: var(--bg-void); padding: 16px 24px; }
  .observe__body { flex: 1; min-height: 0; overflow-y: auto; margin-top: 16px; }
  ```

  **Create:** `src/components/observe/StatusTab.tsx`

  This component extracts lines 603-710 from `Explorer.tsx` (header + sparkline strip + heatmap + episode cards).

  ```tsx
  // --- src/components/observe/StatusTab.tsx ---
  import { useState, useEffect, useRef, useCallback } from 'react';
  import { Mosaic, MosaicCell } from '../Mosaic';
  import { MetricStrip } from '../layout/MetricStrip';
  import { Pane } from '../Pane';
  import './StatusTab.css';

  // Types extracted from Explorer.tsx lines 12-42
  interface HealthData {
    agents: number;
    signals: number;
    episodes: number;
    uptime: number;
    providers: Array<{
      name: string;
      status: string;
      latency_ms: number;
      models: string[];
    }>;
    c_factor?: {
      cost_efficiency: number;
      quality_score: number;
      velocity: number;
      composite: number;
    };
  }

  // Constants from Explorer.tsx lines 47-55
  const API_BASE = '/api';

  // Helpers from Explorer.tsx lines 76-100
  function fmtCost(n: number): string { return n < 0.01 ? `$${n.toFixed(4)}` : `$${n.toFixed(2)}`; }
  function fmtPct(n: number): string { return `${(n * 100).toFixed(1)}%`; }
  function fmtN(n: number): string { return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n); }
  function fmtDur(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m${Math.round(s % 60)}s`;
  }

  export function StatusTab() {
    const [health, setHealth] = useState<HealthData | null>(null);
    const [loading, setLoading] = useState(true);

    // Fetch health data + 10s polling
    useEffect(() => {
      let active = true;
      async function refresh() {
        try {
          const res = await fetch(`${API_BASE}/health`);
          if (res.ok && active) setHealth(await res.json());
        } catch { /* ignore */ }
        if (active) setLoading(false);
      }
      refresh();
      const interval = setInterval(refresh, 10_000);
      return () => { active = false; clearInterval(interval); };
    }, []);

    if (loading) return <div className="status-tab__loading">Loading...</div>;
    if (!health) return <div className="status-tab__error">Failed to load health data</div>;

    return (
      <div className="status-tab">
        {/* Hero mosaic */}
        <Mosaic columns={4}>
          <MosaicCell label="Agents" value={String(health.agents)} color="var(--dream)" />
          <MosaicCell label="Signals" value={fmtN(health.signals)} color="var(--sage)" />
          <MosaicCell label="Episodes" value={fmtN(health.episodes)} color="var(--bone)" />
          <MosaicCell label="Uptime" value={fmtDur(health.uptime * 1000)} color="var(--text-primary)" />
        </Mosaic>

        {/* C-factor strip */}
        {health.c_factor && (
          <MetricStrip
            metrics={[
              { label: 'Cost Eff.', value: health.c_factor.cost_efficiency, format: fmtPct },
              { label: 'Quality', value: health.c_factor.quality_score, format: fmtPct },
              { label: 'Velocity', value: health.c_factor.velocity, format: fmtPct },
              { label: 'Composite', value: health.c_factor.composite, format: fmtPct },
            ]}
            className="status-tab__cfactor"
          />
        )}

        {/* Provider health table */}
        <Pane title="Providers" className="status-tab__providers" flat>
          <ProviderTable providers={health.providers} />
        </Pane>

        {/* Sparkline strip placeholder — uses useCanvasSetup from T1.4 */}
        {/* TODO: wire sparkline canvases using useCanvasSetup */}
      </div>
    );
  }

  // Inline sub-component for provider table
  function ProviderTable({ providers }: { providers: HealthData['providers'] }) {
    return (
      <table className="provider-table">
        <thead>
          <tr>
            <th>Provider</th>
            <th>Status</th>
            <th>Latency</th>
            <th>Models</th>
          </tr>
        </thead>
        <tbody>
          {providers.map((p) => (
            <tr key={p.name}>
              <td className="provider-table__name">{p.name}</td>
              <td>
                <span className={`provider-table__status provider-table__status--${p.status}`}>
                  {p.status}
                </span>
              </td>
              <td className="provider-table__latency">{p.latency_ms}ms</td>
              <td className="provider-table__models">{p.models.join(', ')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    );
  }
  ```

  **Create:** `src/components/observe/StatusTab.css`

  ```css
  .status-tab { display: flex; flex-direction: column; gap: 20px; }
  .status-tab__loading, .status-tab__error { color: var(--text-secondary); padding: 40px; text-align: center; }
  .status-tab__cfactor { margin-top: 8px; }
  .status-tab__providers { margin-top: 8px; }

  .provider-table { width: 100%; border-collapse: collapse; font-size: var(--text-xs); }
  .provider-table th { text-align: left; color: var(--text-secondary); padding: 6px 8px; border-bottom: 1px solid var(--glass-border); text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; }
  .provider-table td { padding: 6px 8px; color: var(--text-primary); border-bottom: 1px solid var(--glass-border); }
  .provider-table__name { font-weight: 600; }
  .provider-table__status { font-family: var(--mono); font-size: 11px; padding: 2px 6px; border-radius: 3px; }
  .provider-table__status--healthy { background: rgba(var(--sage-rgb, 140, 180, 140), 0.15); color: var(--sage); }
  .provider-table__status--degraded { background: rgba(var(--dream-rgb, 180, 160, 200), 0.15); color: var(--dream); }
  .provider-table__status--down { background: rgba(var(--rose-bright-rgb, 200, 100, 100), 0.15); color: var(--rose-bright); }
  .provider-table__latency { font-family: var(--mono); }
  .provider-table__models { color: var(--text-secondary); max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  ```

  **Implementation steps:**
  1. Create `src/scenes/Observe.tsx` and `src/scenes/Observe.css`.
  2. Create `src/components/observe/` directory.
  3. Create `src/components/observe/StatusTab.tsx` and `StatusTab.css`.
  4. Extract types (`HealthData`), constants (`API_BASE`), and helpers (`fmtCost`, `fmtPct`, `fmtN`, `fmtDur`) from `Explorer.tsx` into the StatusTab component. (In a later cleanup pass, shared helpers can be moved to a `lib/formatters.ts`.)
  5. Create stub files for `FleetTab.tsx`, `KnowledgeTab.tsx`, `RoutingTab.tsx`, `DreamsTab.tsx` that each export a simple placeholder component returning `<div>TODO</div>` — they are built in T3.8, T3.9, T3.10.
  6. CSS tokens: `--bg-void`, `--text-primary`, `--text-secondary`, `--text-xs`, `--dream`, `--sage`, `--bone`, `--rose-bright`, `--glass-border`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Navigate to Observe scene. Status tab renders mosaic + C-factor strip + provider table. 10s polling updates data.

---

- [ ] **T3.8: Build Observe scene — Fleet tab (TimelineCanvas + AgentCell)**

  **Read first:**
  - `src/pages/Explorer.tsx` lines 304-527 (drawTimeline canvas callback — 222L. Swim lanes, agent bars, phase labels, tooltip)
  - `src/pages/Explorer.tsx` lines 529-586 (canvas effects: useEffect for resize + draw, mouse tracking)
  - `src/pages/Explorer.tsx` lines 711-805 (episode cards JSX — uses inline card rendering)
  - `src/components/cells/AgentCell.tsx` full file (agent: {name, role, status, taskCount, cost, model}, onClick)
  - `src/components/layout/SplitView.tsx` full file (left, right, defaultSplit)

  **Create:** `src/components/observe/TimelineCanvas.tsx`

  ```tsx
  // --- src/components/observe/TimelineCanvas.tsx ---
  // Extract drawTimeline from Explorer.tsx lines 304-527 into a standalone canvas component.
  import { useRef, useEffect, useCallback } from 'react';
  import './TimelineCanvas.css';

  interface TimelineEntry {
    agent: string;
    start: number;  // timestamp ms
    end: number;     // timestamp ms
    phase: string;
    status: 'success' | 'running' | 'failed';
  }

  interface TimelineCanvasProps {
    entries: TimelineEntry[];
    height?: number;
    onAgentClick?: (agent: string) => void;
  }

  // Theme constants (from Explorer.tsx lines 56-72 THEME object)
  const THEME = {
    bg: '#0a0908',
    lane: '#1a1816',
    laneAlt: '#141210',
    barSuccess: '#8cb48c',     // --sage
    barRunning: '#b4a0c8',     // --dream
    barFailed: '#c86464',      // --rose-bright
    text: '#d4c8b8',           // --bone
    textDim: '#8a8078',        // --text-secondary
    grid: '#2a2420',
  };

  export function TimelineCanvas({ entries, height = 300, onAgentClick }: TimelineCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    const draw = useCallback(() => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container) return;

      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${height}px`;

      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);

      // --- Paste drawTimeline logic from Explorer.tsx lines 310-525 here ---
      // Group entries by agent, compute lanes, draw swim lanes + bars + labels.
      // Use THEME colors above. Replace any external refs with local state.
      // The draw function body is 215 lines — copy it verbatim and replace
      // variable references (entries, canvasWidth, canvasHeight) with local equivalents.

      ctx.fillStyle = THEME.bg;
      ctx.fillRect(0, 0, rect.width, height);

      // ... (full drawTimeline body from Explorer.tsx)
    }, [entries, height]);

    useEffect(() => {
      draw();
      const onResize = () => draw();
      window.addEventListener('resize', onResize);
      return () => window.removeEventListener('resize', onResize);
    }, [draw]);

    return (
      <div ref={containerRef} className="timeline-canvas">
        <canvas ref={canvasRef} />
      </div>
    );
  }
  ```

  **Create:** `src/components/observe/FleetTab.tsx`

  ```tsx
  // --- src/components/observe/FleetTab.tsx ---
  import { useState, useEffect } from 'react';
  import { SplitView } from '../layout/SplitView';
  import { AgentCell } from '../cells/AgentCell';
  import { TimelineCanvas } from './TimelineCanvas';
  import { Pane } from '../Pane';
  import './FleetTab.css';

  interface AgentInfo {
    name: string;
    role?: string;
    status: 'idle' | 'active' | 'stopped';
    taskCount?: number;
    cost?: number;
    model?: string;
  }

  const API_BASE = '/api';

  export function FleetTab() {
    const [agents, setAgents] = useState<AgentInfo[]>([]);
    const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
    const [timelineEntries, setTimelineEntries] = useState<Array<{
      agent: string; start: number; end: number; phase: string;
      status: 'success' | 'running' | 'failed';
    }>>([]);

    useEffect(() => {
      let active = true;
      async function load() {
        try {
          const res = await fetch(`${API_BASE}/agents`);
          if (res.ok && active) setAgents(await res.json());
        } catch { /* ignore */ }
        try {
          const res = await fetch(`${API_BASE}/episodes?limit=200`);
          if (res.ok && active) {
            const eps = await res.json();
            // Transform episodes to timeline entries
            setTimelineEntries(eps.map((e: any) => ({
              agent: e.agent ?? 'unknown',
              start: e.start_ts ?? e.ts,
              end: e.end_ts ?? e.ts + (e.duration_ms ?? 1000),
              phase: e.phase ?? 'run',
              status: e.success ? 'success' : e.running ? 'running' : 'failed',
            })));
          }
        } catch { /* ignore */ }
      }
      load();
      const interval = setInterval(load, 10_000);
      return () => { active = false; clearInterval(interval); };
    }, []);

    return (
      <div className="fleet-tab">
        <SplitView
          left={
            <Pane title="Timeline" flat>
              <TimelineCanvas entries={timelineEntries} height={280} />
            </Pane>
          }
          right={
            <Pane title="Agents" flat>
              <div className="fleet-tab__agents">
                {agents.map((a) => (
                  <AgentCell
                    key={a.name}
                    agent={a}
                    onClick={() => setSelectedAgent(a.name)}
                  />
                ))}
                {agents.length === 0 && (
                  <p className="fleet-tab__empty">No agents active</p>
                )}
              </div>
            </Pane>
          }
          defaultSplit={55}
        />
      </div>
    );
  }
  ```

  **Create:** `src/components/observe/FleetTab.css` and `src/components/observe/TimelineCanvas.css`

  ```css
  /* FleetTab.css */
  .fleet-tab { height: 100%; }
  .fleet-tab__agents { display: flex; flex-direction: column; gap: 8px; overflow-y: auto; max-height: 100%; }
  .fleet-tab__empty { color: var(--text-secondary); font-size: var(--text-sm); padding: 20px; text-align: center; }

  /* TimelineCanvas.css */
  .timeline-canvas { width: 100%; position: relative; }
  .timeline-canvas canvas { display: block; width: 100%; }
  ```

  **Implementation steps:**
  1. Create `src/components/observe/TimelineCanvas.tsx` — copy the draw body from `Explorer.tsx` lines 310-525 verbatim into the `draw` callback. Replace external variable references (`canvasW`, `canvasH`, `entries`, etc.) with the component's local equivalents.
  2. Create `src/components/observe/FleetTab.tsx` — agent list using `AgentCell` + timeline canvas.
  3. Create the CSS files.
  4. Update the stub `FleetTab` in `Observe.tsx` to import the real one.

  **Verify:** `npx tsc --noEmit` passes. Fleet tab shows timeline canvas with swim lanes. Agent cards list on right. Click agent highlights (detail in future task).

---

- [ ] **T3.9: Build Observe scene — Knowledge tab**

  **Read first:**
  - `src/pages/Explorer.tsx` lines 225-302 (derived knowledge data computations, if any)
  - Any existing knowledge components (check `src/components/` for knowledge-related files)

  **Create:** `src/components/observe/KnowledgeTab.tsx`

  ```tsx
  // --- src/components/observe/KnowledgeTab.tsx ---
  import { useState, useEffect, useRef, useCallback } from 'react';
  import { Pane } from '../Pane';
  import { Tabs } from '../layout/Tabs';
  import './KnowledgeTab.css';

  interface KnowledgeEntry {
    id: string;
    topic: string;
    summary: string;
    citations: number;
    tier: 'ephemeral' | 'working' | 'durable';
    created_at: number;
  }

  const API_BASE = '/api';
  const MODE_TABS = [
    { id: 'list', label: 'List' },
    { id: 'graph', label: 'Graph' },
  ];

  export function KnowledgeTab() {
    const [mode, setMode] = useState('list');
    const [entries, setEntries] = useState<KnowledgeEntry[]>([]);
    const [search, setSearch] = useState('');
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
      let active = true;
      async function load() {
        try {
          const res = await fetch(`${API_BASE}/knowledge?limit=500`);
          if (res.ok && active) setEntries(await res.json());
        } catch { /* ignore */ }
      }
      load();
      return () => { active = false; };
    }, []);

    const filtered = entries.filter((e) =>
      !search || e.topic.toLowerCase().includes(search.toLowerCase())
        || e.summary.toLowerCase().includes(search.toLowerCase())
    );

    // Force-directed graph draw
    const drawGraph = useCallback(() => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container || mode !== 'graph') return;

      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = 400 * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = '400px';

      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);

      ctx.fillStyle = '#0a0908';
      ctx.fillRect(0, 0, rect.width, 400);

      // Simple force-directed layout:
      // - Each entry = node, radius = 4 + citations * 2 (clamped 4-20)
      // - Color by tier: ephemeral=#8a8078, working=#b4a0c8, durable=#8cb48c
      // - Position: random initial, then N iterations of repulsion + centering
      // Full implementation: ~80 lines of force simulation
      const nodes = filtered.map((e, i) => ({
        ...e,
        x: rect.width / 2 + (Math.random() - 0.5) * rect.width * 0.6,
        y: 200 + (Math.random() - 0.5) * 300,
        r: Math.min(20, 4 + e.citations * 2),
      }));

      // Draw nodes
      const tierColor: Record<string, string> = {
        ephemeral: '#8a8078',
        working: '#b4a0c8',
        durable: '#8cb48c',
      };
      for (const n of nodes) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, n.r, 0, Math.PI * 2);
        ctx.fillStyle = tierColor[n.tier] ?? '#8a8078';
        ctx.globalAlpha = 0.7;
        ctx.fill();
        ctx.globalAlpha = 1;
      }
    }, [filtered, mode]);

    useEffect(() => { drawGraph(); }, [drawGraph]);

    const selected = selectedId ? entries.find((e) => e.id === selectedId) : null;

    return (
      <div className="knowledge-tab">
        <div className="knowledge-tab__header">
          <Tabs tabs={MODE_TABS} active={mode} onChange={setMode} />
          <input
            className="knowledge-tab__search"
            type="text"
            placeholder="Search knowledge..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        <div className="knowledge-tab__body">
          {mode === 'list' && (
            <div className="knowledge-tab__list">
              {filtered.map((e) => (
                <div
                  key={e.id}
                  className={`knowledge-tab__item ${selectedId === e.id ? 'knowledge-tab__item--selected' : ''}`}
                  onClick={() => setSelectedId(e.id)}
                >
                  <span className="knowledge-tab__item-topic">{e.topic}</span>
                  <span className={`knowledge-tab__item-tier knowledge-tab__item-tier--${e.tier}`}>{e.tier}</span>
                  <span className="knowledge-tab__item-citations">{e.citations} cites</span>
                </div>
              ))}
            </div>
          )}

          {mode === 'graph' && (
            <div ref={containerRef} className="knowledge-tab__graph">
              <canvas ref={canvasRef} />
            </div>
          )}
        </div>

        {/* Detail panel */}
        {selected && (
          <Pane title={selected.topic} className="knowledge-tab__detail" flat>
            <p className="knowledge-tab__detail-summary">{selected.summary}</p>
            <div className="knowledge-tab__detail-meta">
              <span>Tier: {selected.tier}</span>
              <span>Citations: {selected.citations}</span>
              <span>Created: {new Date(selected.created_at).toLocaleDateString()}</span>
            </div>
          </Pane>
        )}
      </div>
    );
  }
  ```

  **Create:** `src/components/observe/KnowledgeTab.css`

  ```css
  .knowledge-tab { display: flex; flex-direction: column; gap: 12px; height: 100%; }
  .knowledge-tab__header { display: flex; align-items: center; gap: 16px; }
  .knowledge-tab__search { background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-primary); padding: 6px 12px; border-radius: 4px; font-size: var(--text-sm); width: 240px; }
  .knowledge-tab__search::placeholder { color: var(--text-secondary); }
  .knowledge-tab__body { flex: 1; min-height: 0; overflow-y: auto; }
  .knowledge-tab__list { display: flex; flex-direction: column; gap: 4px; }
  .knowledge-tab__item { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 6px; cursor: pointer; font-size: var(--text-sm); background: var(--glass-bg); }
  .knowledge-tab__item:hover { background: var(--glass-bg-hover); }
  .knowledge-tab__item--selected { border-left: 2px solid var(--dream); }
  .knowledge-tab__item-topic { flex: 1; color: var(--text-primary); font-weight: 500; }
  .knowledge-tab__item-tier { font-family: var(--mono); font-size: 11px; padding: 2px 6px; border-radius: 3px; }
  .knowledge-tab__item-tier--ephemeral { color: var(--text-secondary); background: rgba(138, 128, 120, 0.15); }
  .knowledge-tab__item-tier--working { color: var(--dream); background: rgba(180, 160, 200, 0.15); }
  .knowledge-tab__item-tier--durable { color: var(--sage); background: rgba(140, 180, 140, 0.15); }
  .knowledge-tab__item-citations { color: var(--text-secondary); font-size: var(--text-xs); }
  .knowledge-tab__graph { width: 100%; }
  .knowledge-tab__graph canvas { display: block; width: 100%; }
  .knowledge-tab__detail { margin-top: 12px; }
  .knowledge-tab__detail-summary { color: var(--text-primary); font-size: var(--text-sm); line-height: 1.5; }
  .knowledge-tab__detail-meta { display: flex; gap: 16px; margin-top: 8px; font-size: var(--text-xs); color: var(--text-secondary); }
  ```

  **Implementation steps:**
  1. Create `src/components/observe/KnowledgeTab.tsx` and `KnowledgeTab.css`.
  2. The graph mode uses a basic force-directed layout drawn on canvas. Nodes are colored by tier (ephemeral=`#8a8078`, working=`#b4a0c8`, durable=`#8cb48c`). Node radius = `min(20, 4 + citations * 2)`.
  3. List mode is a searchable, filterable list. Click selects an entry and shows detail panel below.
  4. Update Observe.tsx stub import to use the real KnowledgeTab.

  **Verify:** `npx tsc --noEmit` passes. Knowledge tab shows list/graph toggle. Search filters entries. Click entry shows detail panel. Graph renders nodes.

---

- [ ] **T3.10: Build Observe scene — Routing tab + Dreams tab**

  **Read first:**
  - `src/pages/Explorer.tsx` lines 808-858 (provider/event drawer — for routing display patterns)
  - `src/components/Mosaic.tsx` full file (for stats display)
  - `src/components/layout/DataSurface.tsx` full file (loading/error/empty states)

  **Create:** `src/components/observe/RoutingTab.tsx`

  ```tsx
  // --- src/components/observe/RoutingTab.tsx ---
  import { useState, useEffect, useRef, useCallback } from 'react';
  import { Mosaic, MosaicCell } from '../Mosaic';
  import { Pane } from '../Pane';
  import './RoutingTab.css';

  interface RoutingStats {
    total_decisions: number;
    models: Array<{ model: string; count: number; avg_cost: number; avg_quality: number }>;
    cascade_hits: number;
    fallback_rate: number;
  }

  const API_BASE = '/api';

  export function RoutingTab() {
    const [stats, setStats] = useState<RoutingStats | null>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
      let active = true;
      async function load() {
        try {
          const res = await fetch(`${API_BASE}/learn/router`);
          if (res.ok && active) setStats(await res.json());
        } catch { /* ignore */ }
      }
      load();
      return () => { active = false; };
    }, []);

    // Distribution chart (horizontal bar chart on canvas)
    const drawChart = useCallback(() => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container || !stats) return;

      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = 200 * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = '200px';

      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);

      ctx.fillStyle = '#0a0908';
      ctx.fillRect(0, 0, rect.width, 200);

      const total = stats.models.reduce((s, m) => s + m.count, 0) || 1;
      const barHeight = 24;
      const gap = 8;
      const labelWidth = 120;
      const barMax = rect.width - labelWidth - 40;

      const colors = ['#8cb48c', '#b4a0c8', '#d4c8b8', '#c86464', '#a09080'];

      stats.models.forEach((m, i) => {
        const y = 10 + i * (barHeight + gap);
        const w = (m.count / total) * barMax;

        // Label
        ctx.fillStyle = '#d4c8b8';
        ctx.font = '12px monospace';
        ctx.textBaseline = 'middle';
        ctx.fillText(m.model, 8, y + barHeight / 2);

        // Bar
        ctx.fillStyle = colors[i % colors.length];
        ctx.globalAlpha = 0.8;
        ctx.fillRect(labelWidth, y, w, barHeight);
        ctx.globalAlpha = 1;

        // Count
        ctx.fillStyle = '#8a8078';
        ctx.fillText(String(m.count), labelWidth + w + 6, y + barHeight / 2);
      });
    }, [stats]);

    useEffect(() => { drawChart(); }, [drawChart]);

    if (!stats) return <div style={{ color: 'var(--text-secondary)', padding: 20 }}>Loading routing data...</div>;

    return (
      <div className="routing-tab">
        <Mosaic columns={3}>
          <MosaicCell label="Decisions" value={String(stats.total_decisions)} color="var(--dream)" mono />
          <MosaicCell label="Cascade Hits" value={String(stats.cascade_hits)} color="var(--sage)" mono />
          <MosaicCell label="Fallback Rate" value={`${(stats.fallback_rate * 100).toFixed(1)}%`} color="var(--rose-bright)" mono />
        </Mosaic>

        <Pane title="Model Distribution" flat>
          <div ref={containerRef} className="routing-tab__chart">
            <canvas ref={canvasRef} />
          </div>
        </Pane>

        <Pane title="Model Details" flat>
          <table className="routing-tab__table">
            <thead>
              <tr><th>Model</th><th>Calls</th><th>Avg Cost</th><th>Avg Quality</th></tr>
            </thead>
            <tbody>
              {stats.models.map((m) => (
                <tr key={m.model}>
                  <td>{m.model}</td>
                  <td>{m.count}</td>
                  <td>${m.avg_cost.toFixed(4)}</td>
                  <td>{(m.avg_quality * 100).toFixed(1)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Pane>
      </div>
    );
  }
  ```

  **Create:** `src/components/observe/DreamsTab.tsx`

  ```tsx
  // --- src/components/observe/DreamsTab.tsx ---
  import { useState, useEffect } from 'react';
  import { Pane } from '../Pane';
  import { PhaseRail } from '../layout/PhaseRail';
  import './DreamsTab.css';

  interface DreamCycle {
    id: string;
    phase: string;
    progress: number; // 0-1
    started_at: number;
    entries: Array<{ ts: number; type: string; summary: string }>;
  }

  const API_BASE = '/api';
  const DREAM_PHASES = ['Hypnagogia', 'Imagine', 'Consolidate', 'Journal', 'Done'];

  export function DreamsTab() {
    const [cycles, setCycles] = useState<DreamCycle[]>([]);

    useEffect(() => {
      let active = true;
      async function load() {
        try {
          const res = await fetch(`${API_BASE}/knowledge/dreams`);
          if (res.ok && active) setCycles(await res.json());
        } catch { /* ignore */ }
      }
      load();
      return () => { active = false; };
    }, []);

    return (
      <div className="dreams-tab">
        {cycles.length === 0 && (
          <p className="dreams-tab__empty">No dream cycles recorded yet.</p>
        )}

        {cycles.map((c) => {
          const phaseIndex = DREAM_PHASES.indexOf(c.phase);

          return (
            <Pane key={c.id} title={`Cycle ${c.id}`} className="dreams-tab__cycle" flat>
              <PhaseRail
                phases={DREAM_PHASES}
                current={phaseIndex >= 0 ? phaseIndex : 0}
              />
              <div className="dreams-tab__entries">
                {c.entries.map((e, i) => (
                  <div key={i} className="dreams-tab__entry">
                    <span className="dreams-tab__entry-type">{e.type}</span>
                    <span className="dreams-tab__entry-summary">{e.summary}</span>
                  </div>
                ))}
              </div>
            </Pane>
          );
        })}
      </div>
    );
  }
  ```

  **Create:** CSS files `src/components/observe/RoutingTab.css` and `src/components/observe/DreamsTab.css`

  ```css
  /* RoutingTab.css */
  .routing-tab { display: flex; flex-direction: column; gap: 16px; }
  .routing-tab__chart { width: 100%; }
  .routing-tab__chart canvas { display: block; width: 100%; }
  .routing-tab__table { width: 100%; border-collapse: collapse; font-size: var(--text-xs); }
  .routing-tab__table th { text-align: left; color: var(--text-secondary); padding: 6px 8px; border-bottom: 1px solid var(--glass-border); text-transform: uppercase; letter-spacing: 0.05em; }
  .routing-tab__table td { padding: 6px 8px; color: var(--text-primary); border-bottom: 1px solid var(--glass-border); font-family: var(--mono); }

  /* DreamsTab.css */
  .dreams-tab { display: flex; flex-direction: column; gap: 16px; }
  .dreams-tab__empty { color: var(--text-secondary); padding: 40px; text-align: center; }
  .dreams-tab__cycle { margin-bottom: 8px; }
  .dreams-tab__entries { display: flex; flex-direction: column; gap: 4px; margin-top: 12px; }
  .dreams-tab__entry { display: flex; gap: 8px; font-size: var(--text-xs); padding: 4px 0; }
  .dreams-tab__entry-type { color: var(--dream); font-family: var(--mono); min-width: 80px; }
  .dreams-tab__entry-summary { color: var(--text-primary); }
  ```

  **Implementation steps:**
  1. Create `src/components/observe/RoutingTab.tsx` and `RoutingTab.css`.
  2. Create `src/components/observe/DreamsTab.tsx` and `DreamsTab.css`.
  3. Update Observe.tsx imports to use real components instead of stubs.
  4. Routing tab: stats mosaic (3 columns) + canvas bar chart + detail table. API endpoint: `GET /api/learn/router`.
  5. Dreams tab: PhaseRail per cycle + entry list. API endpoint: `GET /api/knowledge/dreams`.
  6. CSS tokens: `--dream`, `--sage`, `--rose-bright`, `--text-primary`, `--text-secondary`, `--text-xs`, `--glass-border`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Routing tab shows mosaic + chart + table. Dreams tab shows cycles with PhaseRail. Numbers render.

---

### 3.3 Evaluate (Replace Bench.tsx — 701L + BenchRunDetail.tsx — 654L)

---

- [ ] **T3.11: Extract inline chart components from BenchRunDetail.tsx**

  **Read first:**
  - `src/pages/BenchRunDetail.tsx` lines 15-252 (CostBreakdownChart — 237L inline component with tokenCost, fitLabel, truncateLabel, buildCostSegments helpers)
  - `src/pages/BenchRunDetail.tsx` lines 254-383 (TokenFlowChart — 129L inline component)
  - `src/pages/BenchRunDetail.tsx` lines 385-469 (OutputPreviewPanel — 84L inline component)
  - `src/pages/BenchRunDetail.tsx` lines 471-654 (BenchRunDetail main component — 183L)

  **Create:** `src/components/evaluate/CostBreakdownChart.tsx`

  ```tsx
  // --- src/components/evaluate/CostBreakdownChart.tsx ---
  // Extracted from BenchRunDetail.tsx lines 15-252.
  // Copy the CostBreakdownChart function component and its 4 helper functions
  // (tokenCost, fitLabel, truncateLabel, buildCostSegments) verbatim.
  // Update imports: the component uses useRef, useEffect, useCallback from React.
  // No external component dependencies — this is a self-contained canvas component.

  import { useRef, useEffect, useCallback } from 'react';

  // Copy tokenCost() from BenchRunDetail.tsx lines ~18-30
  // Copy fitLabel() from BenchRunDetail.tsx lines ~32-45
  // Copy truncateLabel() from BenchRunDetail.tsx lines ~47-55
  // Copy buildCostSegments() from BenchRunDetail.tsx lines ~57-90

  // Copy the full interface for props from BenchRunDetail.tsx

  // Copy CostBreakdownChart function component from lines ~92-252
  // Export it as named export.
  export { CostBreakdownChart };  // after pasting the function
  ```

  **Create:** `src/components/evaluate/TokenFlowChart.tsx`

  ```tsx
  // --- src/components/evaluate/TokenFlowChart.tsx ---
  // Extracted from BenchRunDetail.tsx lines 254-383.
  // Copy the TokenFlowChart function component verbatim.
  // Uses: useRef, useEffect, useCallback from React. No external deps.

  import { useRef, useEffect, useCallback } from 'react';

  // Copy full component from lines 254-383
  export { TokenFlowChart };
  ```

  **Create:** `src/components/evaluate/OutputPreview.tsx`

  ```tsx
  // --- src/components/evaluate/OutputPreview.tsx ---
  // Extracted from BenchRunDetail.tsx lines 385-469.
  // Copy the OutputPreviewPanel, rename to OutputPreview for consistency.

  import { useState } from 'react';

  // Copy full component from lines 385-469
  // Rename: OutputPreviewPanel -> OutputPreview
  export { OutputPreview };
  ```

  **Modify:** `src/pages/BenchRunDetail.tsx`
  1. Delete lines 15-469 (all three inline components + their helpers).
  2. Add imports at top:
     ```ts
     import { CostBreakdownChart } from '../components/evaluate/CostBreakdownChart';
     import { TokenFlowChart } from '../components/evaluate/TokenFlowChart';
     import { OutputPreview } from '../components/evaluate/OutputPreview';
     ```
  3. In the JSX, rename `<OutputPreviewPanel>` to `<OutputPreview>` if the component was renamed.

  **Implementation steps:**
  1. Create `src/components/evaluate/` directory.
  2. Create `CostBreakdownChart.tsx`: copy lines 15-252 from `BenchRunDetail.tsx` verbatim. Add `export` keyword to the function. Ensure all 4 helper functions are included above the component.
  3. Create `TokenFlowChart.tsx`: copy lines 254-383 verbatim. Add `export`.
  4. Create `OutputPreview.tsx`: copy lines 385-469 verbatim. Rename `OutputPreviewPanel` to `OutputPreview`. Add `export`.
  5. In `BenchRunDetail.tsx`, delete the inline definitions (lines 15-469) and add the 3 imports.
  6. `BenchRunDetail.tsx` should go from 654L to ~185L (just the main component + imports).

  **Verify:** `npx tsc --noEmit` passes. `BenchRunDetail.tsx` is < 200L. Each extracted file compiles independently. `wc -l src/components/evaluate/*.tsx` shows 3 files.

---

- [ ] **T3.12: Build Evaluate scene shell + split Bench.tsx tabs**

  **Read first:**
  - `src/pages/Bench.tsx` lines 28-45 (Tab type, STRATEGIES, TABS, RUN_COLORS constants)
  - `src/pages/Bench.tsx` lines 62-99 (state declarations + hooks)
  - `src/pages/Bench.tsx` lines 130-148 (hero section JSX)
  - `src/pages/Bench.tsx` lines 160-297 (Configure tab — 137L)
  - `src/pages/Bench.tsx` lines 299-449 (Live tab — 150L)
  - `src/pages/Bench.tsx` lines 451-505 (Results tab — 54L)
  - `src/pages/Bench.tsx` lines 507-560 (History tab — 53L)
  - `src/pages/Bench.tsx` lines 562-618 (Compare tab — 56L)
  - `src/pages/Bench.tsx` lines 620-687 (Analysis tab — 67L)
  - `src/pages/Bench.tsx` lines 689-696 (Learning tab — 7L)
  - `src/components/layout/Tabs.tsx` full file

  **Create:** `src/scenes/Evaluate.tsx`

  ```tsx
  // --- src/scenes/Evaluate.tsx ---
  import { useState } from 'react';
  import { Tabs } from '../components/layout/Tabs';
  import { ConfigureTab } from '../components/evaluate/ConfigureTab';
  import { LiveTab } from '../components/evaluate/LiveTab';
  import { ResultsTab } from '../components/evaluate/ResultsTab';
  import { HistoryTab } from '../components/evaluate/HistoryTab';
  import { CompareTab } from '../components/evaluate/CompareTab';
  import { AnalysisTab } from '../components/evaluate/AnalysisTab';
  import './Evaluate.css';

  const TABS = [
    { id: 'configure', label: 'Configure' },
    { id: 'live', label: 'Live' },
    { id: 'results', label: 'Results' },
    { id: 'history', label: 'History' },
    { id: 'compare', label: 'Compare' },
    { id: 'analysis', label: 'Analysis' },
  ];

  export function Evaluate() {
    const [activeTab, setActiveTab] = useState('configure');

    // useBench hook is passed down to tabs that need it.
    // Import from hooks/useBench.ts (or split hooks from T3.13).

    return (
      <div className="evaluate">
        <Tabs tabs={TABS} active={activeTab} onChange={setActiveTab} />
        <div className="evaluate__body">
          {activeTab === 'configure' && <ConfigureTab />}
          {activeTab === 'live' && <LiveTab />}
          {activeTab === 'results' && <ResultsTab />}
          {activeTab === 'history' && <HistoryTab />}
          {activeTab === 'compare' && <CompareTab />}
          {activeTab === 'analysis' && <AnalysisTab />}
        </div>
      </div>
    );
  }
  ```

  **Create:** `src/scenes/Evaluate.css`

  ```css
  .evaluate { display: flex; flex-direction: column; height: 100%; background: var(--bg-void); padding: 16px 24px; }
  .evaluate__body { flex: 1; min-height: 0; overflow-y: auto; margin-top: 16px; }
  ```

  **Create tab components** — one file per tab, extracted from `Bench.tsx`:
  - `src/components/evaluate/ConfigureTab.tsx` — lines 160-297 from `Bench.tsx`
  - `src/components/evaluate/LiveTab.tsx` — lines 299-449
  - `src/components/evaluate/ResultsTab.tsx` — lines 451-505
  - `src/components/evaluate/HistoryTab.tsx` — lines 507-560
  - `src/components/evaluate/CompareTab.tsx` — lines 562-618
  - `src/components/evaluate/AnalysisTab.tsx` — lines 620-687

  Each tab file follows this pattern:
  ```tsx
  // --- src/components/evaluate/ConfigureTab.tsx ---
  import { /* needed hooks/components */ } from '...';

  interface ConfigureTabProps {
    // Props it needs from the parent (bench state, handlers)
    // Initially, keep these minimal — just the slice of useBench it needs.
  }

  export function ConfigureTab(/* props */) {
    // Paste the JSX from the corresponding Bench.tsx section.
    // Replace direct state references with props.
    return (
      <div className="configure-tab">
        {/* ... extracted JSX ... */}
      </div>
    );
  }
  ```

  **Implementation steps:**
  1. Create `src/scenes/Evaluate.tsx` and `Evaluate.css`.
  2. For each of the 6 tab sections in `Bench.tsx`:
     a. Create the tab component file under `src/components/evaluate/`.
     b. Copy the JSX block for that tab from `Bench.tsx`.
     c. Identify which pieces of `useBench` state the tab uses (check variable references).
     d. Define a props interface with those fields.
     e. Export the component.
  3. The `Learning` tab (lines 689-696, 7L) is trivially small — fold it into `AnalysisTab` or omit for now.
  4. `Bench.tsx` should be reduced to just the `Evaluate` scene import + route, or deleted entirely once routing is updated.
  5. Constants (`STRATEGIES`, `TABS`, `RUN_COLORS`) go into `src/lib/bench-constants.ts`.

  **Verify:** `npx tsc --noEmit` passes. Each tab file is < 200L. `Evaluate.tsx` is < 50L (just shell + tabs). Navigate to Evaluate scene — all 6 tabs render.

---

- [ ] **T3.13: Split useBench.ts hook into focused sub-hooks**

  **Read first:**
  - `src/hooks/useBench.ts` lines 60-76 (17 useState declarations)
  - `src/hooks/useBench.ts` lines 96-289 (SSE event processing — 8 event types)
  - `src/hooks/useBench.ts` lines 294-310 (fetchPareto)
  - `src/hooks/useBench.ts` lines 312-396 (startRun)
  - `src/hooks/useBench.ts` lines 398-455 (cancelRun, exportRun, importRun)
  - `src/hooks/useBench.ts` lines 462-484 (activeRunSummary computed)
  - `src/hooks/useBench.ts` lines 486-533 (return object — 32 fields)

  **Create:** `src/hooks/useBenchRuns.ts`

  ```ts
  // --- src/hooks/useBenchRuns.ts ---
  // Manages: runs[], activeRunId, startRun, cancelRun, exportRun, importRun
  // Extracted from useBench.ts: state for runs (lines 60-65), startRun (312-396),
  // cancelRun/exportRun/importRun (398-455), SSE connection (96-289).

  import { useState, useCallback, useRef, useEffect } from 'react';

  // Types from bench-types.ts
  import type { BenchRun, BenchTask, BenchEvent } from '../lib/bench-types';

  interface UseBenchRunsReturn {
    runs: BenchRun[];
    activeRunId: string | null;
    activeRunSummary: { passRate: number; cost: number; tokens: number; elapsed: number } | null;
    startRun: (config: { suite: string; model: string; strategy: string }) => Promise<void>;
    cancelRun: (id: string) => void;
    exportRun: (id: string) => string;
    importRun: (json: string) => void;
    events: BenchEvent[];
  }

  export function useBenchRuns(): UseBenchRunsReturn {
    // Paste state declarations from useBench.ts lines 60-65 (runs, activeRunId, events)
    // Paste SSE connection logic from lines 96-289
    // Paste startRun from lines 312-396
    // Paste cancelRun/exportRun/importRun from lines 398-455
    // Paste activeRunSummary from lines 462-484
    // Return the subset of fields

    // ... (implementation extracted from useBench.ts)
    return {} as any; // placeholder — fill with extracted code
  }
  ```

  **Create:** `src/hooks/useBenchFilter.ts`

  ```ts
  // --- src/hooks/useBenchFilter.ts ---
  // Manages: sort, filter, search state for the run list and history.
  // Extracted from useBench.ts: filter/sort state (lines 66-72).

  import { useState, useMemo } from 'react';
  import type { BenchRun } from '../lib/bench-types';

  type SortField = 'date' | 'passRate' | 'cost' | 'model';
  type SortDir = 'asc' | 'desc';

  interface UseBenchFilterReturn {
    sortField: SortField;
    sortDir: SortDir;
    search: string;
    setSortField: (f: SortField) => void;
    setSortDir: (d: SortDir) => void;
    setSearch: (s: string) => void;
    filterRuns: (runs: BenchRun[]) => BenchRun[];
  }

  export function useBenchFilter(): UseBenchFilterReturn {
    const [sortField, setSortField] = useState<SortField>('date');
    const [sortDir, setSortDir] = useState<SortDir>('desc');
    const [search, setSearch] = useState('');

    const filterRuns = useMemo(() => (runs: BenchRun[]) => {
      let result = [...runs];
      if (search) {
        const q = search.toLowerCase();
        result = result.filter((r) =>
          r.suite.toLowerCase().includes(q) || r.model.toLowerCase().includes(q)
        );
      }
      result.sort((a, b) => {
        const dir = sortDir === 'asc' ? 1 : -1;
        switch (sortField) {
          case 'date': return dir * (a.startedAt - b.startedAt);
          case 'passRate': return dir * (a.passRate - b.passRate);
          case 'cost': return dir * (a.cost - b.cost);
          case 'model': return dir * a.model.localeCompare(b.model);
          default: return 0;
        }
      });
      return result;
    }, [sortField, sortDir, search]);

    return { sortField, sortDir, search, setSortField, setSortDir, setSearch, filterRuns };
  }
  ```

  **Create:** `src/hooks/useBenchMatrix.ts`

  ```ts
  // --- src/hooks/useBenchMatrix.ts ---
  // Manages: matrix configuration, multi-model eval progress.
  // This wraps the existing useMatrixBench hook or extracts matrix state from useBench.

  import { useState, useCallback } from 'react';

  // Re-export or wrap useMatrixBench from hooks/useMatrixBench.ts
  export { useMatrixBench as useBenchMatrix } from './useMatrixBench';
  ```

  **Modify:** `src/hooks/useBench.ts`
  1. Replace the 17 useState + all logic with composition of the 3 sub-hooks:
  ```ts
  export function useBench() {
    const runs = useBenchRuns();
    const filter = useBenchFilter();
    const matrix = useBenchMatrix();

    return { ...runs, ...filter, ...matrix };
  }
  ```
  2. This preserves the existing API — all 32 return fields still available.

  **Implementation steps:**
  1. Create `src/hooks/useBenchRuns.ts` — extract state for runs, SSE, start/cancel/export/import.
  2. Create `src/hooks/useBenchFilter.ts` — extract sort/filter/search state.
  3. Create `src/hooks/useBenchMatrix.ts` — re-export or wrap `useMatrixBench`.
  4. Rewrite `src/hooks/useBench.ts` to compose the 3 sub-hooks and spread their returns.
  5. Do NOT change any component that currently calls `useBench()` — the return type must be backward-compatible.

  **Verify:** `npx tsc --noEmit` passes. `useBench.ts` is < 30L (just composition). Each sub-hook file is < 200L. All components importing `useBench` still compile without changes.

---

### 3.4 Build (Chat Interface)

---

- [ ] **T3.14: Build Build scene — chat interface**

  **Read first:**
  - `src/hooks/useTerminalSession.ts` full file (WebSocket terminal session management)
  - `src/hooks/useChain.ts` full file (chain/chat hook patterns)
  - `src/lib/terminal-session.ts` lines 196-234 (showCmd — command typing + output detection)
  - `src/components/AgentOutputStream.tsx` full file (streaming agent output display)
  - `src/components/layout/SplitView.tsx` full file

  **Create:** `src/scenes/Build.tsx`

  ```tsx
  // --- src/scenes/Build.tsx ---
  import { useState, useCallback, useRef, useEffect } from 'react';
  import { SplitView } from '../components/layout/SplitView';
  import { Pane } from '../components/Pane';
  import { Badge } from '../components/design/Badge';
  import './Build.css';

  const MODELS = ['claude-sonnet-4-20250514', 'claude-opus-4-20250514', 'gpt-4o', 'o3'] as const;
  const PRESETS = [
    'Explain this codebase',
    'Find and fix bugs',
    'Add comprehensive tests',
    'Refactor for clarity',
    'Generate documentation',
  ];

  interface ChatMessage {
    id: string;
    role: 'user' | 'assistant';
    content: string;
    toolCalls?: Array<{ name: string; input: string; output: string }>;
    timestamp: number;
  }

  export function Build() {
    const [model, setModel] = useState<string>(MODELS[0]);
    const [input, setInput] = useState('');
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [streaming, setStreaming] = useState(false);
    const [showTerminal, setShowTerminal] = useState(false);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    // Auto-scroll on new messages
    useEffect(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    // Keyboard: T toggles terminal
    useEffect(() => {
      function onKey(e: KeyboardEvent) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (e.code === 'KeyT') { e.preventDefault(); setShowTerminal((s) => !s); }
      }
      window.addEventListener('keydown', onKey);
      return () => window.removeEventListener('keydown', onKey);
    }, []);

    const handleSend = useCallback(async () => {
      if (!input.trim() || streaming) return;
      const userMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content: input.trim(),
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, userMsg]);
      setInput('');
      setStreaming(true);

      try {
        const res = await fetch('/api/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ model, message: userMsg.content }),
        });

        if (!res.ok) throw new Error(`Chat failed: ${res.status}`);

        // Streaming response
        const reader = res.body?.getReader();
        const decoder = new TextDecoder();
        let assistantContent = '';
        const assistantId = crypto.randomUUID();

        setMessages((prev) => [...prev, {
          id: assistantId,
          role: 'assistant',
          content: '',
          timestamp: Date.now(),
        }]);

        if (reader) {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            assistantContent += decoder.decode(value, { stream: true });
            setMessages((prev) =>
              prev.map((m) => m.id === assistantId ? { ...m, content: assistantContent } : m)
            );
          }
        }
      } catch (err) {
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `Error: ${err instanceof Error ? err.message : 'Unknown error'}`,
          timestamp: Date.now(),
        }]);
      } finally {
        setStreaming(false);
      }
    }, [input, model, streaming]);

    const chatPanel = (
      <div className="build__chat">
        {/* Model selector chips */}
        <div className="build__model-selector">
          {MODELS.map((m) => (
            <button
              key={m}
              className={`build__model-chip ${model === m ? 'build__model-chip--active' : ''}`}
              onClick={() => setModel(m)}
            >
              {m.split('-').slice(0, 2).join(' ')}
            </button>
          ))}
        </div>

        {/* Message thread */}
        <div className="build__messages">
          {messages.length === 0 && (
            <div className="build__empty">
              <p>Select a model and type a prompt, or choose a preset:</p>
              <div className="build__presets">
                {PRESETS.map((p) => (
                  <button key={p} className="build__preset" onClick={() => setInput(p)}>
                    {p}
                  </button>
                ))}
              </div>
            </div>
          )}
          {messages.map((msg) => (
            <div key={msg.id} className={`build__msg build__msg--${msg.role}`}>
              <Badge variant={msg.role === 'user' ? 'default' : 'info'}>
                {msg.role}
              </Badge>
              <div className="build__msg-content">
                {msg.content || (streaming && msg.role === 'assistant' ? '...' : '')}
              </div>
              {msg.toolCalls?.map((tc, i) => (
                <details key={i} className="build__tool-call">
                  <summary>{tc.name}</summary>
                  <pre className="build__tool-input">{tc.input}</pre>
                  <pre className="build__tool-output">{tc.output}</pre>
                </details>
              ))}
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>

        {/* Input bar */}
        <div className="build__input-bar">
          <input
            className="build__input"
            type="text"
            placeholder="Type a prompt..."
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            disabled={streaming}
          />
          <button
            className="build__send"
            onClick={handleSend}
            disabled={!input.trim() || streaming}
          >
            {streaming ? 'Sending...' : 'Send'}
          </button>
        </div>
      </div>
    );

    if (showTerminal) {
      return (
        <div className="build">
          <SplitView
            left={chatPanel}
            right={
              <Pane title="Terminal" flat>
                <div className="build__terminal-placeholder">
                  {/* Terminal component from useTerminalSession goes here */}
                  <p style={{ color: 'var(--text-secondary)' }}>Terminal panel (T to toggle)</p>
                </div>
              </Pane>
            }
            defaultSplit={60}
          />
        </div>
      );
    }

    return <div className="build">{chatPanel}</div>;
  }
  ```

  **Create:** `src/scenes/Build.css`

  ```css
  .build { height: 100%; background: var(--bg-void); display: flex; flex-direction: column; }
  .build__chat { display: flex; flex-direction: column; height: 100%; padding: 16px 24px; }
  .build__model-selector { display: flex; gap: 8px; margin-bottom: 16px; flex-shrink: 0; }
  .build__model-chip { background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-secondary); padding: 6px 14px; border-radius: 16px; font-size: var(--text-xs); cursor: pointer; }
  .build__model-chip--active { background: var(--rose-bright); color: var(--bg-void); border-color: var(--rose-bright); }
  .build__messages { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 12px; min-height: 0; }
  .build__empty { padding: 40px 0; text-align: center; color: var(--text-secondary); }
  .build__presets { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin-top: 16px; }
  .build__preset { background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-primary); padding: 8px 16px; border-radius: 8px; font-size: var(--text-sm); cursor: pointer; }
  .build__preset:hover { background: var(--glass-bg-hover); }
  .build__msg { padding: 12px; border-radius: 8px; }
  .build__msg--user { background: var(--glass-bg); }
  .build__msg--assistant { background: transparent; }
  .build__msg-content { margin-top: 8px; color: var(--text-primary); font-size: var(--text-sm); line-height: 1.6; white-space: pre-wrap; }
  .build__tool-call { margin-top: 8px; font-size: var(--text-xs); }
  .build__tool-call summary { color: var(--dream); cursor: pointer; font-family: var(--mono); }
  .build__tool-input, .build__tool-output { background: var(--bg-surface); padding: 8px; border-radius: 4px; font-size: 11px; overflow-x: auto; margin-top: 4px; color: var(--text-primary); }
  .build__input-bar { display: flex; gap: 8px; flex-shrink: 0; margin-top: 12px; }
  .build__input { flex: 1; background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-primary); padding: 10px 14px; border-radius: 8px; font-size: var(--text-sm); }
  .build__input::placeholder { color: var(--text-secondary); }
  .build__send { background: var(--rose-bright); color: var(--bg-void); border: none; padding: 10px 20px; border-radius: 8px; font-size: var(--text-sm); font-weight: 600; cursor: pointer; }
  .build__send:disabled { opacity: 0.4; cursor: not-allowed; }
  .build__terminal-placeholder { padding: 20px; }
  ```

  **Implementation steps:**
  1. Create `src/scenes/Build.tsx` and `Build.css`.
  2. Model selector uses chip/pill buttons. Active chip: `--rose-bright` bg, `--bg-void` text.
  3. Chat messages scroll automatically. User messages have `--glass-bg` background.
  4. Streaming: uses `fetch` + `ReadableStream` reader for SSE-style streaming from `/api/chat`.
  5. Tool calls are rendered as `<details>` with expandable input/output.
  6. Terminal panel toggles with T key. Uses `SplitView` when visible.
  7. CSS tokens: `--bg-void`, `--bg-surface`, `--glass-bg`, `--glass-border`, `--glass-bg-hover`, `--rose-bright`, `--dream`, `--text-primary`, `--text-secondary`, `--text-sm`, `--text-xs`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Build scene renders. Model chips toggle. Input accepts text. Enter sends. Presets fill input. T toggles terminal panel.

---

### 3.5 Knowledge

---

- [ ] **T3.15: Build Knowledge scene (standalone page)**

  **Read first:**
  - `src/components/observe/KnowledgeTab.tsx` (created in T3.9 — reuse graph + list patterns)
  - `src/components/observe/DreamsTab.tsx` (created in T3.10 — reuse dream cycle display)
  - `src/components/layout/Tabs.tsx` full file
  - `src/components/layout/SplitView.tsx` full file

  **Create:** `src/scenes/Knowledge.tsx`

  The Knowledge scene is a full-page version of the Observe Knowledge tab (T3.9) with:
  - Larger graph canvas (full height minus header)
  - SplitView: graph/list on left, detail panel on right
  - Dreams sub-section as a tab
  - Richer detail panel (full summary, citations list, related entries)

  ```tsx
  // --- src/scenes/Knowledge.tsx ---
  import { useState, useEffect, useRef, useCallback } from 'react';
  import { Tabs } from '../components/layout/Tabs';
  import { SplitView } from '../components/layout/SplitView';
  import { Pane } from '../components/Pane';
  import { PhaseRail } from '../components/layout/PhaseRail';
  import './Knowledge.css';

  interface KnowledgeEntry {
    id: string;
    topic: string;
    summary: string;
    citations: number;
    tier: 'ephemeral' | 'working' | 'durable';
    created_at: number;
    related?: string[];
  }

  interface DreamCycle {
    id: string;
    phase: string;
    progress: number;
    started_at: number;
    entries: Array<{ ts: number; type: string; summary: string }>;
  }

  const API_BASE = '/api';
  const TABS = [
    { id: 'explore', label: 'Explore' },
    { id: 'dreams', label: 'Dreams' },
  ];
  const DREAM_PHASES = ['Hypnagogia', 'Imagine', 'Consolidate', 'Journal', 'Done'];

  export function Knowledge() {
    const [activeTab, setActiveTab] = useState('explore');
    const [entries, setEntries] = useState<KnowledgeEntry[]>([]);
    const [dreams, setDreams] = useState<DreamCycle[]>([]);
    const [search, setSearch] = useState('');
    const [mode, setMode] = useState<'graph' | 'list'>('graph');
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
      let active = true;
      async function load() {
        try {
          const [kRes, dRes] = await Promise.all([
            fetch(`${API_BASE}/knowledge?limit=1000`),
            fetch(`${API_BASE}/knowledge/dreams`),
          ]);
          if (kRes.ok && active) setEntries(await kRes.json());
          if (dRes.ok && active) setDreams(await dRes.json());
        } catch { /* ignore */ }
      }
      load();
      return () => { active = false; };
    }, []);

    const filtered = entries.filter((e) =>
      !search || e.topic.toLowerCase().includes(search.toLowerCase())
        || e.summary.toLowerCase().includes(search.toLowerCase())
    );

    const selected = selectedId ? entries.find((e) => e.id === selectedId) : null;

    // Force-directed graph (reuse pattern from T3.9 KnowledgeTab, but larger canvas)
    const drawGraph = useCallback(() => {
      const canvas = canvasRef.current;
      const container = containerRef.current;
      if (!canvas || !container || mode !== 'graph') return;

      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;

      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);

      ctx.fillStyle = '#0a0908';
      ctx.fillRect(0, 0, rect.width, rect.height);

      // Same force-directed logic as T3.9, but using full container height
      const tierColor: Record<string, string> = {
        ephemeral: '#8a8078',
        working: '#b4a0c8',
        durable: '#8cb48c',
      };

      const nodes = filtered.map((e) => ({
        ...e,
        x: rect.width / 2 + (Math.random() - 0.5) * rect.width * 0.7,
        y: rect.height / 2 + (Math.random() - 0.5) * rect.height * 0.7,
        r: Math.min(24, 5 + e.citations * 2),
      }));

      for (const n of nodes) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, n.r, 0, Math.PI * 2);
        ctx.fillStyle = tierColor[n.tier] ?? '#8a8078';
        ctx.globalAlpha = 0.7;
        ctx.fill();
        ctx.globalAlpha = 1;

        // Label for larger nodes
        if (n.r > 10) {
          ctx.fillStyle = '#d4c8b8';
          ctx.font = '11px monospace';
          ctx.textAlign = 'center';
          ctx.fillText(n.topic.slice(0, 15), n.x, n.y + n.r + 12);
        }
      }
    }, [filtered, mode]);

    useEffect(() => { drawGraph(); }, [drawGraph]);

    const exploreContent = (
      <SplitView
        left={
          <div className="knowledge__main">
            <div className="knowledge__toolbar">
              <button
                className={`knowledge__mode-btn ${mode === 'graph' ? 'knowledge__mode-btn--active' : ''}`}
                onClick={() => setMode('graph')}
              >Graph</button>
              <button
                className={`knowledge__mode-btn ${mode === 'list' ? 'knowledge__mode-btn--active' : ''}`}
                onClick={() => setMode('list')}
              >List</button>
              <input
                className="knowledge__search"
                type="text"
                placeholder="Search..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>

            {mode === 'graph' && (
              <div ref={containerRef} className="knowledge__graph">
                <canvas ref={canvasRef} />
              </div>
            )}

            {mode === 'list' && (
              <div className="knowledge__list">
                {filtered.map((e) => (
                  <div
                    key={e.id}
                    className={`knowledge__item ${selectedId === e.id ? 'knowledge__item--selected' : ''}`}
                    onClick={() => setSelectedId(e.id)}
                  >
                    <span className="knowledge__item-topic">{e.topic}</span>
                    <span className={`knowledge__item-tier knowledge__item-tier--${e.tier}`}>{e.tier}</span>
                    <span className="knowledge__item-citations">{e.citations}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        }
        right={
          selected ? (
            <Pane title={selected.topic} flat>
              <p className="knowledge__detail-summary">{selected.summary}</p>
              <div className="knowledge__detail-meta">
                <span>Tier: {selected.tier}</span>
                <span>Citations: {selected.citations}</span>
                <span>Created: {new Date(selected.created_at).toLocaleDateString()}</span>
              </div>
              {selected.related && selected.related.length > 0 && (
                <div className="knowledge__detail-related">
                  <h4>Related</h4>
                  {selected.related.map((rid) => {
                    const rel = entries.find((e) => e.id === rid);
                    return rel ? (
                      <div key={rid} className="knowledge__related-item" onClick={() => setSelectedId(rid)}>
                        {rel.topic}
                      </div>
                    ) : null;
                  })}
                </div>
              )}
            </Pane>
          ) : (
            <Pane title="Detail" flat>
              <p style={{ color: 'var(--text-secondary)', padding: 20 }}>Select an entry to view details</p>
            </Pane>
          )
        }
        defaultSplit={65}
      />
    );

    const dreamsContent = (
      <div className="knowledge__dreams">
        {dreams.length === 0 && (
          <p style={{ color: 'var(--text-secondary)', padding: 40, textAlign: 'center' }}>
            No dream cycles recorded yet.
          </p>
        )}
        {dreams.map((c) => (
          <Pane key={c.id} title={`Cycle ${c.id}`} flat>
            <PhaseRail phases={DREAM_PHASES} current={DREAM_PHASES.indexOf(c.phase)} />
            <div className="knowledge__dream-entries">
              {c.entries.map((e, i) => (
                <div key={i} className="knowledge__dream-entry">
                  <span className="knowledge__dream-type">{e.type}</span>
                  <span className="knowledge__dream-summary">{e.summary}</span>
                </div>
              ))}
            </div>
          </Pane>
        ))}
      </div>
    );

    return (
      <div className="knowledge">
        <Tabs tabs={TABS} active={activeTab} onChange={setActiveTab} />
        <div className="knowledge__body">
          {activeTab === 'explore' && exploreContent}
          {activeTab === 'dreams' && dreamsContent}
        </div>
      </div>
    );
  }
  ```

  **Create:** `src/scenes/Knowledge.css`

  ```css
  .knowledge { display: flex; flex-direction: column; height: 100%; background: var(--bg-void); padding: 16px 24px; }
  .knowledge__body { flex: 1; min-height: 0; margin-top: 16px; }
  .knowledge__main { display: flex; flex-direction: column; height: 100%; }
  .knowledge__toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; flex-shrink: 0; }
  .knowledge__mode-btn { background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-secondary); padding: 4px 12px; border-radius: 4px; font-size: var(--text-xs); cursor: pointer; }
  .knowledge__mode-btn--active { color: var(--text-primary); border-color: var(--text-primary); }
  .knowledge__search { background: var(--glass-bg); border: 1px solid var(--glass-border); color: var(--text-primary); padding: 6px 12px; border-radius: 4px; font-size: var(--text-sm); width: 200px; }
  .knowledge__search::placeholder { color: var(--text-secondary); }
  .knowledge__graph { flex: 1; min-height: 0; }
  .knowledge__graph canvas { display: block; width: 100%; height: 100%; }
  .knowledge__list { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; }
  .knowledge__item { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 6px; cursor: pointer; font-size: var(--text-sm); background: var(--glass-bg); }
  .knowledge__item:hover { background: var(--glass-bg-hover); }
  .knowledge__item--selected { border-left: 2px solid var(--dream); }
  .knowledge__item-topic { flex: 1; color: var(--text-primary); font-weight: 500; }
  .knowledge__item-tier { font-family: var(--mono); font-size: 11px; padding: 2px 6px; border-radius: 3px; }
  .knowledge__item-tier--ephemeral { color: var(--text-secondary); }
  .knowledge__item-tier--working { color: var(--dream); }
  .knowledge__item-tier--durable { color: var(--sage); }
  .knowledge__item-citations { color: var(--text-secondary); font-size: var(--text-xs); }
  .knowledge__detail-summary { color: var(--text-primary); font-size: var(--text-sm); line-height: 1.5; }
  .knowledge__detail-meta { display: flex; gap: 16px; margin-top: 12px; font-size: var(--text-xs); color: var(--text-secondary); }
  .knowledge__detail-related { margin-top: 16px; }
  .knowledge__detail-related h4 { font-size: var(--text-xs); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px; }
  .knowledge__related-item { padding: 4px 8px; font-size: var(--text-sm); color: var(--dream); cursor: pointer; }
  .knowledge__related-item:hover { text-decoration: underline; }
  .knowledge__dreams { display: flex; flex-direction: column; gap: 16px; }
  .knowledge__dream-entries { display: flex; flex-direction: column; gap: 4px; margin-top: 12px; }
  .knowledge__dream-entry { display: flex; gap: 8px; font-size: var(--text-xs); }
  .knowledge__dream-type { color: var(--dream); font-family: var(--mono); min-width: 80px; }
  .knowledge__dream-summary { color: var(--text-primary); }
  ```

  **Implementation steps:**
  1. Create `src/scenes/Knowledge.tsx` and `Knowledge.css`.
  2. Reuses the graph + list patterns from `KnowledgeTab` (T3.9), but full-page with SplitView.
  3. Detail panel on the right shows summary, metadata, and clickable related entries.
  4. Dreams sub-tab reuses the `PhaseRail` + entry list pattern from `DreamsTab` (T3.10).
  5. CSS tokens: `--bg-void`, `--glass-bg`, `--glass-border`, `--glass-bg-hover`, `--text-primary`, `--text-secondary`, `--text-sm`, `--text-xs`, `--dream`, `--sage`, `--mono`.

  **Verify:** `npx tsc --noEmit` passes. Knowledge scene renders. Graph/list toggle works. Search filters entries. Click entry shows detail with related links. Dreams tab shows cycles with PhaseRail.

---

## Phase 4: Chrome & Navigation

### 4.1 App Shell

- [ ] **T4.1: Rebuild AppShell with DataHub**

  **Read first:**
  - `src/components/AppShell.tsx` -- current shell, 63 lines
  - `src/main.tsx` -- current provider wrappers, lines 50-84
  - `src/hooks/useApiWithFallback.ts` -- probe logic to replace
  - `src/contexts/EventStreamContext.tsx` -- provider to remove
  - `src/hooks/useWorkspace.ts` -- provider to remove

  **Providers to remove** (currently wrapping `<Routes>` in `src/main.tsx`):
  1. `<EventStreamProvider>` -- line 52 of main.tsx. SSE singleton; replaced by DataHub built-in SSE.
  2. `<WorkspaceProvider>` -- line 53 of main.tsx. Workspace CRUD; replaced by DataHub workspace slice.

  **Atmospheric overlays to keep** (currently in AppShell.tsx lines 31-34):
  - `<Grain />` from `./Grain`
  - `<HeroParticleField />` from `./HeroParticleField` (lazy-loaded in T4.4)
  - `<Curtain />` from `./Curtain`
  - `<ScrollTrack />` from `./ScrollTrack`

  **Step-by-step:**

  1. In `src/main.tsx`, remove the `<EventStreamProvider>` and `<WorkspaceProvider>` wrapper elements (lines 52-53 and their closing tags at lines 81-82). Keep `<BrowserRouter>`, `<ErrorBoundary>`, `<Suspense>`.
  2. In `src/main.tsx`, add DataHub initialization before `createRoot`:
     ```tsx
     import { useDataHub } from '../stores/DataHub';

     // Initialize DataHub connection on app start
     useDataHub.getState().init();
     ```
  3. In `src/components/AppShell.tsx`, replace the `useApiWithFallback` import and usage:
     ```tsx
     // BEFORE:
     import { useApiWithFallback } from '../hooks/useApiWithFallback';
     const { dataMode } = useApiWithFallback();

     // AFTER:
     import { useDataHub } from '../stores/DataHub';
     const dataMode = useDataHub((s) => s.dataMode);
     ```
  4. Keep the `IntersectionObserver` effect (lines 13-27) unchanged.
  5. Keep atmospheric overlays: `<Grain />`, `<HeroParticleField />`, `<Curtain />`, `<ScrollTrack />`.
  6. Keep the SEED DATA badge (lines 36-57) reading `dataMode` from DataHub.
  7. Keep `<TopNav />` and `<Outlet />` unchanged.

  **Resulting `AppShell.tsx` structure:**
  ```tsx
  import { useEffect } from 'react';
  import { Outlet } from 'react-router';
  import Grain from './Grain';
  import HeroParticleField from './HeroParticleField';
  import Curtain from './Curtain';
  import ScrollTrack from './ScrollTrack';
  import TopNav from './TopNav';
  import { useDataHub } from '../stores/DataHub';

  export default function AppShell() {
    const dataMode = useDataHub((s) => s.dataMode);
    useEffect(() => { /* IntersectionObserver -- unchanged */ }, []);
    return (
      <>
        <Grain />
        <HeroParticleField />
        <Curtain />
        <ScrollTrack />
        <TopNav />
        {dataMode === 'seed' && ( /* SEED DATA badge -- unchanged */ )}
        <div className="app-frame" style={{ paddingTop: 48, position: 'relative', zIndex: 1, minHeight: '100vh' }}>
          <Outlet />
        </div>
      </>
    );
  }
  ```

  **Verify:** `npm run dev` -- app loads. React DevTools Components tab: search "Provider" -- only BrowserRouter internal provider. No EventStreamProvider, no WorkspaceProvider. DataHub store visible in zustand devtools.

- [ ] **T4.2: Rebuild TopNav**

  **Read first:**
  - `src/components/TopNav.tsx` -- current nav, 80 lines
  - `src/components/TopNav.css` -- current styles, 244 lines
  - `src/hooks/useServerHealth.ts` -- hook to remove (replaced by DataHub)

  **New nav links array** (replacing current `NAV_LINKS` at TopNav.tsx lines 8-16):
  ```tsx
  const NAV_LINKS = [
    { to: '/orchestrate', label: 'ORCHESTRATE' },
    { to: '/observe',     label: 'OBSERVE' },
    { to: '/evaluate',    label: 'EVALUATE' },
    { to: '/build',       label: 'BUILD' },
    { to: '/knowledge',   label: 'KNOWLEDGE' },
  ];
  ```

  **Step-by-step:**

  1. Replace `NAV_LINKS` constant with the 5-item array above.
  2. Remove imports: `useApi` (line 3), `fmtUptime` (line 4), `useServerHealth` (line 5).
  3. Remove the `get`/`useApi()` call (line 24), the `useServerHealth()` call (line 25), the `uptime` state (line 26), and the entire polling `useEffect` (lines 28-43).
  4. Replace with DataHub selector:
     ```tsx
     import { useDataHub } from '../stores/DataHub';

     export default function TopNav() {
       const serverStatus = useDataHub((s) => s.serverStatus);
       // serverStatus: 'connected' | 'disconnected' | 'reconnecting'
     ```
  5. Update StatusPill logic (currently lines 45, 73-76):
     ```tsx
     const statusClass = serverStatus === 'connected' ? 'live'
                       : serverStatus === 'reconnecting' ? 'reconnecting'
                       : 'demo';
     const statusText = serverStatus === 'connected' ? 'LIVE'
                      : serverStatus === 'reconnecting' ? 'RECONNECTING'
                      : 'OFFLINE';
     const statusChar = serverStatus === 'connected' ? '\u25CF' : '\u25CB';

     // In JSX:
     <span className={`status-pill ${statusClass}`}>
       <span className="status-char">{statusChar}</span>
       {statusText}
     </span>
     ```
  6. The `conn-blink`/`term-dot-blink` consolidation is already done -- `animations.css` has a unified `status-blink` keyframe (line 41). No CSS change needed.
  7. Add `reconnecting` CSS state in `TopNav.css` after the `.status-pill.demo` block (after line 233):
     ```css
     /* Reconnecting state */
     .topnav .status-pill.reconnecting {
       color: var(--warning);
       border-color: rgba(200, 154, 104, .2);
     }
     .topnav .status-pill.reconnecting .status-char {
       animation: status-blink 1.2s step-end infinite;
     }
     ```
     CSS tokens: `--warning` (#d8a878), `--mono`, `--sp-1`, `--sp-3`.

  **Verify:** `npm run dev` -- click each nav link, active shows rose glow + underline. StatusPill reads "LIVE" when roko-serve running, "OFFLINE" when stopped. Stop server mid-session: pill shows "RECONNECTING" with blinking dot.

- [ ] **T4.3: Add StreamOverlay**

  **Files to create:** `src/components/StreamOverlay.tsx`, `src/components/StreamOverlay.css`

  **Read first:**
  - `src/stores/DataHub.ts` -- for `sseConnected`, `wsConnected` selectors
  - `src/styles/rosedust.css` lines 92-101 -- z-index layers

  **Exports:**
  ```tsx
  // No props -- reads all state from DataHub
  export default function StreamOverlay(): JSX.Element;
  ```

  **Imports:**
  ```tsx
  import { useState } from 'react';
  import { useDataHub } from '../stores/DataHub';
  import './StreamOverlay.css';
  ```

  **Step-by-step:**

  1. Create `src/components/StreamOverlay.tsx`:
     ```tsx
     import { useState } from 'react';
     import { useDataHub } from '../stores/DataHub';
     import './StreamOverlay.css';

     export default function StreamOverlay() {
       const [expanded, setExpanded] = useState(false);
       const sseConnected = useDataHub((s) => s.sseConnected);
       const wsConnected = useDataHub((s) => s.wsConnected);

       const sseColor = sseConnected ? 'var(--success)' : 'var(--status-error)';
       const wsColor = wsConnected ? 'var(--success)' : 'var(--status-error)';

       return (
         <div
           className={`stream-overlay${expanded ? ' expanded' : ''}`}
           onMouseEnter={() => setExpanded(true)}
           onMouseLeave={() => setExpanded(false)}
         >
           <span className="stream-dot" style={{ background: sseColor }} />
           <span className="stream-dot" style={{ background: wsColor }} />
           {expanded && (
             <div className="stream-detail">
               <div className="stream-row">
                 <span className="stream-label">SSE</span>
                 <span className="stream-val">{sseConnected ? 'connected' : 'disconnected'}</span>
               </div>
               <div className="stream-row">
                 <span className="stream-label">WS</span>
                 <span className="stream-val">{wsConnected ? 'connected' : 'disconnected'}</span>
               </div>
             </div>
           )}
         </div>
       );
     }
     ```

  2. Create `src/components/StreamOverlay.css`:
     ```css
     .stream-overlay {
       position: fixed;
       bottom: var(--sp-4);
       right: var(--sp-4);
       z-index: var(--z-floating);
       display: flex;
       align-items: center;
       gap: var(--sp-1);
       padding: var(--sp-1) var(--sp-2);
       background: var(--bg-raised);
       border: 1px solid var(--border-soft);
       border-radius: var(--radius-sm);
       cursor: default;
       transition: all var(--duration-snap);
     }
     .stream-overlay.expanded {
       flex-direction: column;
       align-items: flex-start;
       gap: var(--sp-2);
       padding: var(--sp-2) var(--sp-3);
     }
     .stream-dot {
       width: 6px; height: 6px;
       border-radius: 50%;
       flex-shrink: 0;
     }
     .stream-detail {
       font-family: var(--mono);
       font-size: var(--text-xs);
       color: var(--text-dim);
       letter-spacing: .08em;
     }
     .stream-row { display: flex; gap: var(--sp-2); align-items: center; }
     .stream-row .stream-label { color: var(--text-ghost); min-width: 3ch; }
     ```
     CSS tokens: `--sp-1` (4px), `--sp-2` (8px), `--sp-3` (12px), `--sp-4` (16px), `--bg-raised` (#12101a), `--border-soft`, `--radius-sm` (3px), `--z-floating` (100), `--mono`, `--text-xs` (12px), `--text-dim`, `--text-ghost`, `--success` (#8a9c86), `--status-error` (#fb7185), `--duration-snap` (150ms).

  3. Add `<StreamOverlay />` to `AppShell.tsx` after `<TopNav />`:
     ```tsx
     import StreamOverlay from './StreamOverlay';
     // In JSX:
     <TopNav />
     <StreamOverlay />
     ```

  **Verify:** `npm run dev` -- bottom-right shows two dots (green when server running). Hover: expands to "SSE connected / WS connected". Kill `roko serve`: dots turn red, labels show "disconnected".

### 4.2 Routing & Transitions

- [ ] **T4.4: Configure React Router with lazy loading**

  **Read first:**
  - `src/main.tsx` -- current routing, 85 lines, already uses `React.lazy()`
  - `vite.config.ts` -- already has `manualChunks` for three/xterm (lines 12-15)
  - `src/components/HeroParticleField.tsx` -- re-exports AmbientParticles (uses Three.js)

  **Current state:** `main.tsx` already uses `React.lazy()` for all page components (lines 11-28). `vite.config.ts` already has `manualChunks` splitting Three.js and xterm. This task adds: (a) lazy-loading the ambient Three.js layer from AppShell, (b) scene-to-route mapping for the new 5-scene architecture.

  **Exact lazy imports** (replace current lines 11-28 in `src/main.tsx`):
  ```tsx
  // Scene lazy imports
  const Orchestrate    = lazy(() => import('./pages/Orchestrate'));
  const Observe        = lazy(() => import('./pages/Observe'));
  const Evaluate       = lazy(() => import('./pages/Evaluate'));
  const Build          = lazy(() => import('./pages/Build'));
  const Knowledge      = lazy(() => import('./pages/Knowledge'));

  // Sub-routes
  const BenchRunDetail = lazy(() => import('./pages/BenchRunDetail'));
  const BenchCompare   = lazy(() => import('./pages/BenchCompare'));
  const SharePage      = lazy(() => import('./pages/Share'));

  // Dashboard sub-pages (nested under Observe)
  const CostDashboard    = lazy(() => import('./pages/dashboard/CostDashboard'));
  const AgentFleet       = lazy(() => import('./pages/dashboard/AgentFleet'));
  const KnowledgeGraph   = lazy(() => import('./pages/dashboard/KnowledgeGraph'));
  const IntegrityView    = lazy(() => import('./pages/dashboard/IntegrityView'));
  const CascadeRouter    = lazy(() => import('./pages/dashboard/CascadeRouter'));
  const KnowledgeEntries = lazy(() => import('./pages/dashboard/KnowledgeEntries'));
  const DreamsView       = lazy(() => import('./pages/dashboard/DreamsView'));
  ```

  **Route config** (replace current `<Routes>` block):
  ```tsx
  <Routes>
    <Route element={<AppShell />}>
      <Route index element={<Orchestrate />} />
      <Route path="orchestrate" element={<Orchestrate />} />
      <Route path="observe" element={<Observe />}>
        <Route index element={<CostDashboard />} />
        <Route path="fleet" element={<AgentFleet />} />
        <Route path="knowledge" element={<KnowledgeGraph />} />
        <Route path="integrity" element={<IntegrityView />} />
        <Route path="entries" element={<KnowledgeEntries />} />
        <Route path="routing" element={<CascadeRouter />} />
        <Route path="dreams" element={<DreamsView />} />
      </Route>
      <Route path="evaluate" element={<Evaluate />} />
      <Route path="evaluate/run/:id" element={<BenchRunDetail />} />
      <Route path="evaluate/compare" element={<BenchCompare />} />
      <Route path="build" element={<Build />} />
      <Route path="knowledge" element={<Knowledge />} />
      <Route path="share/:token" element={<SharePage />} />
    </Route>
  </Routes>
  ```

  **Lazy-load HeroParticleField in AppShell.tsx** (replace static import):
  ```tsx
  // BEFORE:
  import HeroParticleField from './HeroParticleField';

  // AFTER:
  import { lazy, Suspense } from 'react';
  const HeroParticleField = lazy(() => import('./HeroParticleField'));

  // In JSX, wrap with Suspense:
  <Suspense fallback={null}>
    <HeroParticleField />
  </Suspense>
  ```

  **Vite manualChunks** (already correct in `vite.config.ts`, no changes needed):
  ```ts
  manualChunks(id) {
    if (id.includes('/node_modules/three/')) return 'vendor-three';
    if (id.includes('/node_modules/@xterm/')) return 'vendor-xterm';
  },
  ```

  **Step-by-step:**
  1. Update `src/main.tsx`: replace lazy imports with the scene-based list above.
  2. Update `src/main.tsx`: replace `<Routes>` block with new route config.
  3. In `src/components/AppShell.tsx`: change HeroParticleField to lazy import with `<Suspense fallback={null}>`.
  4. Confirm `vite.config.ts` manualChunks already correct (it is).
  5. Note: scene page files (`Orchestrate.tsx`, `Observe.tsx`, etc.) are created in Phase 3 tasks.

  **Verify:** `npx tsc --noEmit` (type check). `npm run build` -- check `dist/assets/` for `vendor-three-*.js` and `vendor-xterm-*.js` chunks. DevTools Network tab: navigate scenes -- chunks load on demand.

- [ ] **T4.5: Add keyboard shortcuts**

  **File to create:** `src/hooks/useKeyboardShortcuts.ts`

  **Read first:**
  - `src/components/AppShell.tsx` -- where to mount the hook
  - `src/stores/DataHub.ts` -- for `toggleDebug`, playback controls

  **Exports:**
  ```tsx
  /** Mount once in AppShell. Ignores keypresses in input/textarea/contenteditable. */
  export function useKeyboardShortcuts(): void;
  ```

  **Imports:**
  ```tsx
  import { useEffect } from 'react';
  import { useNavigate, useLocation } from 'react-router';
  import { useDataHub } from '../stores/DataHub';
  ```

  **Complete shortcut table:**

  | Key | Code check | Action | Context |
  |---|---|---|---|
  | Cmd+K / Ctrl+K | `(e.metaKey \|\| e.ctrlKey) && e.key === 'k'` | `store.toggleCommandPalette()` | Global |
  | ? | `e.key === '?' && !e.metaKey && !e.ctrlKey` | `store.toggleHelpOverlay()` | Global |
  | Shift+D | `e.key === 'D' && e.shiftKey` | `store.toggleDebugPanel()` | Global |
  | 1 | `e.key === '1'` | `navigate('/orchestrate')` | Global |
  | 2 | `e.key === '2'` | `navigate('/observe')` | Global |
  | 3 | `e.key === '3'` | `navigate('/evaluate')` | Global |
  | 4 | `e.key === '4'` | `navigate('/build')` | Global |
  | 5 | `e.key === '5'` | `navigate('/knowledge')` | Global |
  | Space | `e.key === ' '` | `store.togglePlayPause()` | `/orchestrate` only |
  | n | `e.key === 'n'` | `store.stepNext()` | `/orchestrate` only |
  | r | `e.key === 'r'` | `store.resetDemo()` | `/orchestrate` only |
  | t | `e.key === 't'` | `store.toggleTerminal()` | `/build` only |

  **Complete implementation:**
  ```tsx
  import { useEffect } from 'react';
  import { useNavigate, useLocation } from 'react-router';
  import { useDataHub } from '../stores/DataHub';

  const INPUT_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT']);

  export function useKeyboardShortcuts(): void {
    const navigate = useNavigate();
    const { pathname } = useLocation();

    useEffect(() => {
      function handler(e: KeyboardEvent) {
        const target = e.target as HTMLElement;
        if (INPUT_TAGS.has(target.tagName) || target.isContentEditable) return;

        const store = useDataHub.getState();

        // Cmd/Ctrl+K -- command palette
        if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
          e.preventDefault(); store.toggleCommandPalette(); return;
        }
        // ? -- help overlay
        if (e.key === '?' && !e.metaKey && !e.ctrlKey) {
          e.preventDefault(); store.toggleHelpOverlay(); return;
        }
        // Shift+D -- debug panel
        if (e.key === 'D' && e.shiftKey) {
          e.preventDefault(); store.toggleDebugPanel(); return;
        }
        // Number keys 1-5 -- scene navigation
        if (e.key >= '1' && e.key <= '5' && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
          const routes = ['/orchestrate', '/observe', '/evaluate', '/build', '/knowledge'];
          e.preventDefault(); navigate(routes[parseInt(e.key) - 1]); return;
        }
        // Orchestrate-only
        if (pathname.startsWith('/orchestrate')) {
          if (e.key === ' ') { e.preventDefault(); store.togglePlayPause(); return; }
          if (e.key === 'n') { e.preventDefault(); store.stepNext(); return; }
          if (e.key === 'r') { e.preventDefault(); store.resetDemo(); return; }
        }
        // Build-only
        if (pathname.startsWith('/build')) {
          if (e.key === 't') { e.preventDefault(); store.toggleTerminal(); return; }
        }
      }
      window.addEventListener('keydown', handler);
      return () => window.removeEventListener('keydown', handler);
    }, [navigate, pathname]);
  }
  ```

  **Mount in AppShell.tsx:**
  ```tsx
  import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';

  export default function AppShell() {
    useKeyboardShortcuts();
    // ... rest of component
  ```

  **CommandPalette integration:** This hook calls `store.toggleCommandPalette()` -- a boolean on DataHub. The `<CommandPalette />` component (T7.30) reads `useDataHub(s => s.commandPaletteOpen)`. Shortcut hook does NOT need to know about the palette component.

  **Verify:** `npm run dev` -- press `?` for help overlay. `Cmd+K` for command palette. `1`-`5` navigates scenes. On Orchestrate, Space toggles play/pause. Click into text input, press `?` -- nothing happens (guard works).

---

## Phase 5: Polish & Performance

### 5.1 Accessibility

- [ ] **T5.1: Add ARIA labels to all canvas elements**

  **Read first:** Every file listed below at the `<canvas` line.

  **Complete canvas element list** (34 total, from `grep -rn '<canvas' src/ --include='*.tsx'`):

  | # | File | Line | Add |
  |---|---|---|---|
  | 1 | `src/components/AmbientParticles.tsx` | 80 | `role="img" aria-label="Ambient particle field background animation"` |
  | 2 | `src/components/Charts/BarChart.tsx` | 71 | `role="img" aria-label="Bar chart visualization"` |
  | 3 | `src/components/Charts/CFactorSparkline.tsx` | 216 | `role="img" aria-label="C-factor trend sparkline"` |
  | 4 | `src/components/Charts/CostChart.tsx` | 91 | `role="img" aria-label="Cost over time chart"` |
  | 5 | `src/components/Charts/HeatmapChart.tsx` | 77 | `role="img" aria-label="Heatmap data visualization"` |
  | 6 | `src/components/Charts/ParetoChart.tsx` | 125 | `role="img" aria-label="Pareto distribution chart"` |
  | 7 | `src/components/Charts/RadarChart.tsx` | 122 | `role="img" aria-label="Radar chart comparing metrics"` |
  | 8 | `src/components/Charts/ScatterChart.tsx` | 153 | `role="img" aria-label="Scatter plot visualization"` |
  | 9 | `src/components/Charts/TimelineChart.tsx` | 119 | `role="img" aria-label="Timeline chart of events"` |
  | 10 | `src/components/CostRace.tsx` | 381 | `role="img" aria-label="Cost race comparison between models"` |
  | 11 | `src/components/DreamPhaseViz.tsx` | 140 | `role="img" aria-label="Dream consolidation phase visualization"` |
  | 12 | `src/components/GateWaterfall.tsx` | 189 | `role="img" aria-label="Gate waterfall timing chart"` |
  | 13 | `src/components/KnowledgeFlowPanel.tsx` | 365 | `role="img" aria-label="Knowledge flow network visualization"` |
  | 14 | `src/components/MatrixRaceTrack.tsx` | 213 | `role="img" aria-label="Matrix benchmark race track"` |
  | 15 | `src/components/ThresholdGauge.tsx` | 191 | `role="img" aria-label="Adaptive threshold gauge"` |
  | 16 | `src/components/TokenVelocitySparkline.tsx` | 122 | `role="img" aria-label="Token velocity sparkline"` |
  | 17 | `src/components/ambient/FluidGradient.tsx` | 142 | `role="presentation" aria-hidden="true"` |
  | 18 | `src/components/ambient/GlitchOverlay.tsx` | 131 | `role="presentation" aria-hidden="true"` |
  | 19 | `src/components/ambient/HeartbeatLine.tsx` | 137 | `role="presentation" aria-hidden="true"` |
  | 20 | `src/components/ambient/NoiseBackground.tsx` | 89 | `role="presentation" aria-hidden="true"` |
  | 21 | `src/pages/BenchRunDetail.tsx` | 247 | `role="img" aria-label="Task duration distribution chart"` |
  | 22 | `src/pages/BenchRunDetail.tsx` | 380 | `role="img" aria-label="Gate pass rate chart"` |
  | 23 | `src/pages/Explorer.tsx` | 654 | `role="img" aria-label="Signal flow graph"` |
  | 24 | `src/pages/Explorer.tsx` | 668 | `role="img" aria-label="Episodes sparkline"` |
  | 25 | `src/pages/Explorer.tsx` | 673 | `role="img" aria-label="Cost sparkline"` |
  | 26 | `src/pages/Explorer.tsx` | 678 | `role="img" aria-label="Agent activity sparkline"` |
  | 27 | `src/pages/Explorer.tsx` | 683 | `role="img" aria-label="Gate pass rate sparkline"` |
  | 28 | `src/pages/Explorer.tsx` | 688 | `role="img" aria-label="Duration sparkline"` |
  | 29 | `src/pages/dashboard/AgentFleet.tsx` | 400 | `role="img" aria-label="Agent fleet topology network"` |
  | 30 | `src/pages/dashboard/CascadeRouter.tsx` | 122 | `role="img" aria-label="Cascade router model distribution"` |
  | 31 | `src/pages/dashboard/IntegrityView.tsx` | 172 | `role="img" aria-label="Integrity verification timeline"` |
  | 32 | `src/pages/dashboard/KnowledgeEntries.tsx` | 117 | `role="img" aria-label="Knowledge entry frequency chart"` |
  | 33 | `src/pages/dashboard/KnowledgeEntries.tsx` | 220 | `role="img" aria-label="Knowledge tier distribution chart"` |
  | 34 | `src/pages/dashboard/KnowledgeGraph.tsx` | 264 | `role="img" aria-label="Knowledge graph network visualization"` |

  **Rules:**
  - Decorative/atmospheric canvases (#17-20) get `role="presentation" aria-hidden="true"`.
  - Data canvases (#1-16, #21-34) get `role="img" aria-label="<description>"`.

  **Pattern:**
  ```tsx
  // BEFORE:
  <canvas ref={canvasRef} className="chart-canvas" />
  // AFTER:
  <canvas ref={canvasRef} className="chart-canvas" role="img" aria-label="Cost over time chart" />
  ```

  **Step-by-step:**
  1. Open each of the 34 files at the listed line.
  2. Add `role` and `aria-label` (or `aria-hidden`) attrs as specified.
  3. Save each file.

  **Verify:** `npx tsc --noEmit`. Browser with macOS VoiceOver (Cmd+F5): navigate to Explorer -- announces "Signal flow graph, image". Ambient canvases are silent.

- [ ] **T5.2: Add keyboard navigation to interactive tables**

  **Read first:**
  - `src/components/TaskTable.tsx` -- sortable table, `onClick` on `<th>` headers (lines 93-99), no keyboard nav
  - `src/components/ChainActivityPanel.tsx` -- collapsible blocks, `onClick` on `.ca-block-head` (line 87), no tabIndex
  - `src/pages/dashboard/KnowledgeEntries.tsx` -- entry list, no interactivity on rows
  - `src/pages/Bench.tsx` -- history rows, checkbox input only

  **Components needing keyboard nav:**

  | Component | File | Element | Current |
  |---|---|---|---|
  | TaskTable | `src/components/TaskTable.tsx` | `<th>` headers (lines 93-99) | onClick only |
  | ChainActivityPanel | `src/components/ChainActivityPanel.tsx` | `.ca-block-head` (line 87) | onClick only |
  | KnowledgeEntries | `src/pages/dashboard/KnowledgeEntries.tsx` | Entry rows | Not interactive |
  | Bench history | `src/pages/Bench.tsx` | Run rows | Checkbox only |

  **Step 1: Create `src/lib/a11y.ts`:**
  ```tsx
  /** Keyboard handler: Enter/Space activates, ArrowUp/Down moves focus. */
  export function handleRowKeyDown(
    e: React.KeyboardEvent,
    onClick: () => void,
  ): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClick();
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = (e.currentTarget as HTMLElement).nextElementSibling as HTMLElement | null;
      next?.focus();
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = (e.currentTarget as HTMLElement).previousElementSibling as HTMLElement | null;
      prev?.focus();
    }
  }
  ```

  **Step 2: TaskTable.tsx** -- add to each `<th>` (lines 93-99):
  ```tsx
  // BEFORE:
  <th onClick={() => handleSort('task_name')}>Task{arrow('task_name')}</th>
  // AFTER:
  <th tabIndex={0} role="columnheader"
      onClick={() => handleSort('task_name')}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSort('task_name'); } }}>
    Task{arrow('task_name')}
  </th>
  ```
  Apply to all 7 `<th>` elements. Add `role="table"` to the parent `<table>`.

  **Step 3: ChainActivityPanel.tsx** -- add to `.ca-block-head` (line 87):
  ```tsx
  // BEFORE:
  <div className="ca-block-head" onClick={() => toggle(block.number)}>
  // AFTER:
  <div className="ca-block-head" tabIndex={0} role="button"
       onClick={() => toggle(block.number)}
       onKeyDown={(e) => handleRowKeyDown(e, () => toggle(block.number))}>
  ```
  Import: `import { handleRowKeyDown } from '../lib/a11y';`

  **Step 4: KnowledgeEntries.tsx** -- add to entry row container:
  ```tsx
  <div className="ke-entry-row" tabIndex={0} role="row"
       onKeyDown={(e) => handleRowKeyDown(e, () => { /* expand/select */ })}>
  ```

  **Step 5: Bench.tsx** -- add to history run rows:
  ```tsx
  <tr tabIndex={0} role="row"
      onKeyDown={(e) => handleRowKeyDown(e, () => { /* toggle selection */ })}>
  ```

  **Step 6: Add focus ring CSS** (append to `src/styles/rosedust.css`):
  ```css
  /* Focus rings for keyboard navigation */
  [tabIndex="0"]:focus-visible,
  [role="button"]:focus-visible,
  [role="row"]:focus-visible,
  [role="columnheader"]:focus-visible {
    outline: 1px solid var(--rose-dim);
    outline-offset: -1px;
    box-shadow: 0 0 0 2px rgba(184, 122, 148, 0.2);
  }
  ```

  **Verify:** Tab through table rows -- focus ring visible. Enter on TaskTable header sorts column. ArrowDown/Up moves focus between rows. Lighthouse Accessibility: no "interactive element not focusable" warnings.

- [ ] **T5.3: Reduced motion support**

  **Read first:**
  - `src/styles/animations.css` -- 6 shared keyframes
  - `src/styles/rosedust.css` -- 11 keyframes

  **Files already handling reduced motion (20 CSS + 4 TSX) -- DO NOT modify:**
  CSS: `design/ContentSwitch.css`, `design/StepProgress.css`, `design/ConnectionGuard.css`, `design/MilestoneProgress.css`, `design/CircularProgress.css`, `design/LoadingTransition.css`, `design/LazyPane.css`, `design/VerticalTimeline.css`, `GateVerdictCard.css`, `Terminal/TerminalPane.css`, `agent/AgentFeed.css`, `agent/AgentHandoff.css`, `agent/AgentHeartbeat.css`, `inference/ArtifactGem.css`, `inference/ArtifactTray.css`, `inference/ConfidenceMeter.css`, `inference/CrystallizeTransition.css`, `inference/CyberneticIntensity.css`, `inference/ModelSlot.css`, `pages/Demo.css`.
  TSX: `ambient/NoiseBackground.tsx`, `ambient/GlitchOverlay.tsx`, `ambient/FluidGradient.tsx`, `ambient/HeartbeatLine.tsx`.

  **CSS files with `animation:` but NO reduced-motion query (34 files):**
  `styles/animations.css`, `styles/rosedust.css`, `ChainActivityPanel.css`, `CommandLog.css`, `ConfigWidget.css`, `GateVerdictTicker.css`, `HeroScene.css`, `KnowledgeFlowPanel.css`, `MatrixBuilder.css`, `MatrixDetailView.css`, `PrdPipelinePanel.css`, `RevealWhen.css`, `Timeline.css`, `TopNav.css`, `ascii/AsciiLabel.css`, `cells/Cell.css`, `cells/CellGrid.css`, `design/EmptyState.css`, `design/ErrorState.css`, `design/GateBar.css`, `design/Pulse.css`, `design/Skeleton.css`, `design/StatusBadge.css`, `feeds/BlockFeed.css`, `feeds/EventStream.css`, `feeds/InferenceFeed.css`, `layout/PageShell.css`, `layout/PhaseRail.css`, `layout/ResizablePane.css`, `motion/AnimatedList.css`, `overlay/FloatingChat.css`, `pages/Bench.css`, `pages/Builder.css`, `pages/Explorer.css`.

  **Approach: single global kill switch** (append to `src/styles/rosedust.css`):
  ```css
  /* Reduced motion: global override */
  @media (prefers-reduced-motion: reduce) {
    *,
    *::before,
    *::after {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
    }
  }
  ```
  This sets animation/transition durations to near-zero (effectively instant). Uses `!important` as a user-preference override. Does NOT set `animation: none` (preserves `animation-fill-mode` end states).

  **Step-by-step:**
  1. Open `src/styles/rosedust.css`.
  2. Append the `@media (prefers-reduced-motion: reduce)` block at the very end.
  3. No per-file changes needed.

  **Verify:** Chrome DevTools > Rendering > "Emulate prefers-reduced-motion: reduce". All pulsing/breathing/shimmer animations freeze. Color/opacity transitions apply instantly. Functional flow unchanged.

### 5.2 Performance

- [ ] **T5.4: Code-split heavy dependencies**

  **Read first:**
  - `src/main.tsx` -- lazy imports already exist for pages
  - `src/components/AppShell.tsx` -- static import of HeroParticleField
  - `src/components/HeroParticleField.tsx` -- re-exports AmbientParticles (Three.js)
  - `vite.config.ts` -- already has `manualChunks` for `three` and `@xterm`

  **Current state:** Vite already splits three/xterm into separate chunks. Pages are lazy-loaded. Remaining: HeroParticleField is statically imported in AppShell.tsx, forcing Three.js to load on every page.

  **Change 1: `src/components/AppShell.tsx`** -- lazy-load HeroParticleField:
  ```tsx
  // REMOVE:
  import HeroParticleField from './HeroParticleField';

  // ADD:
  import { lazy, Suspense } from 'react';
  const LazyHeroParticleField = lazy(() => import('./HeroParticleField'));

  // In JSX -- BEFORE:
  <HeroParticleField />
  // AFTER:
  <Suspense fallback={null}>
    <LazyHeroParticleField />
  </Suspense>
  ```
  `fallback={null}` because decorative background -- no skeleton needed.

  **Change 2: Verify `vite.config.ts`** (already correct, no changes):
  ```ts
  manualChunks(id) {
    if (id.includes('/node_modules/three/')) return 'vendor-three';
    if (id.includes('/node_modules/@xterm/')) return 'vendor-xterm';
  },
  ```

  **No changes for xterm** -- only imported from Terminal.tsx and Builder.tsx, both already lazy-loaded.

  **Step-by-step:**
  1. In `src/components/AppShell.tsx`: replace static import with lazy + Suspense.
  2. Confirm vite.config.ts manualChunks correct (it is).
  3. Confirm xterm pages already lazy-loaded (they are).

  **Verify:**
  ```bash
  cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npm run build
  ls -lh dist/assets/vendor-three-*.js dist/assets/vendor-xterm-*.js
  gzip -c dist/assets/index-*.js | wc -c  # should be < 200kB
  ```
  DevTools Network: initial load should NOT include vendor-three. Navigate to Build: vendor-xterm loads on demand.

- [ ] **T5.5: Animation performance audit**

  **Read first:**
  - `src/styles/rosedust.css` -- all keyframe declarations
  - `src/styles/animations.css` -- shared keyframes
  - `src/components/TopNav.css` -- mark-breathe animation

  **Chrome DevTools audit procedure:**
  1. Open `http://localhost:5173/orchestrate`.
  2. DevTools > Performance tab > gear > check "Screenshots" and "Web Vitals".
  3. Record. Run full Orchestrate demo (play, wait for all tasks).
  4. Stop.

  **Targets:**
  - Frame budget: no frame > 16ms. 95th percentile < 8ms.
  - Concurrent animations: DevTools > Animations tab. Target <= 8 during peak.
  - Layout thrashing: Performance tab, purple "Layout" bars. Any > 4ms is a problem.

  **Highest-risk animations (may animate layout properties):**
  - `step-line-draw` in `design/StepProgress.css` -- animates `width` -> CONVERT to `scaleX`
  - `step-line-draw-v` in `design/StepProgress.css` -- animates `height` -> CONVERT to `scaleY`
  - `ms-pop` in `design/MilestoneProgress.css` -- check for width/height
  - `shimmerMove` in `rosedust.css` -- background-position (usually OK)
  - `mark-breathe` in `TopNav.css` -- box-shadow (paint only, OK)

  **Transform conversion pattern:**
  ```css
  /* BEFORE (triggers layout): */
  @keyframes step-line-draw {
    from { width: 0; }
    to { width: 100%; }
  }
  /* AFTER (GPU-accelerated): */
  @keyframes step-line-draw {
    from { transform: scaleX(0); }
    to { transform: scaleX(1); }
  }
  /* Parent needs: transform-origin: left center; */
  ```

  **Step-by-step:**
  1. Run Performance audit as described.
  2. If frames > 16ms, identify cause in flame chart.
  3. Find layout-triggering keyframes:
     ```bash
     grep -A5 '@keyframes' src/**/*.css | grep -E 'width|height|top:|left:|right:|bottom:|margin|padding'
     ```
  4. Convert to transform-based.
  5. box-shadow animations: keep unless paint > 4ms.
  6. Verify concurrent count <= 8 during peak.

  **Verify:** Re-run Performance recording. No frames > 16ms. Animations tab <= 8 concurrent at peak. Lighthouse Performance >= 90.

- [ ] **T5.6: DataHub selector optimization**

  **Read first:**
  - `src/stores/DataHub.ts` -- DataHub store definition (built in Phase 2)

  **Import pattern:**
  ```tsx
  import { useDataHub } from '../stores/DataHub';
  import { shallow } from 'zustand/shallow';
  ```

  **Problem:** Components selecting multiple fields without `shallow` re-render on ANY store change.

  **Multi-field selector pattern:**
  ```tsx
  // BAD -- re-renders on ANY store change:
  const { plans, agents } = useDataHub((s) => ({ plans: s.plans, agents: s.agents }));

  // GOOD -- re-renders only when plans or agents change:
  const { plans, agents } = useDataHub(
    (s) => ({ plans: s.plans, agents: s.agents }),
    shallow,
  );
  ```

  **Derived data pattern:**
  ```tsx
  import { useMemo } from 'react';
  const tasks = useDataHub((s) => s.tasks);
  const sortedTasks = useMemo(
    () => [...tasks].sort((a, b) => a.name.localeCompare(b.name)),
    [tasks],
  );
  ```

  **Finding selectors to optimize:**
  ```bash
  grep -rn 'useDataHub.*=>' src/ --include='*.tsx' | grep '{.*,.*}'
  ```
  Every match destructuring multiple fields needs `shallow`.

  **Step-by-step:**
  1. Run grep to find all multi-field DataHub selectors.
  2. For each, add `shallow` as second arg to `useDataHub()`.
  3. Add `import { shallow } from 'zustand/shallow';` to each file.
  4. Wrap inline sort/filter of DataHub data in `useMemo`.
  5. Do NOT add `shallow` to single-field selectors (reference equality is correct).

  **Verify:** React DevTools Profiler. Record: navigate all 5 scenes. No component re-renders more than once per navigation unless its data changed.

### 5.3 Error Handling

- [ ] **T5.7: Add error boundaries around heavy components**

  **Read first:**
  - `src/components/ErrorBoundary.tsx` -- current global boundary, 57 lines, full-screen error

  **Current problem:** Global ErrorBoundary catches at app root. A WebGL crash kills the entire app.

  **Create `src/components/ComponentErrorBoundary.tsx`** (~35 lines):
  ```tsx
  import { Component } from 'react';
  import type { ErrorInfo, ReactNode } from 'react';
  import './ComponentErrorBoundary.css';

  interface Props { name: string; children: ReactNode; fallback?: ReactNode; }
  interface State { hasError: boolean; error: Error | null; }

  export default class ComponentErrorBoundary extends Component<Props, State> {
    state: State = { hasError: false, error: null };

    static getDerivedStateFromError(error: Error): State {
      return { hasError: true, error };
    }

    componentDidCatch(error: Error, info: ErrorInfo) {
      console.error(`[${this.props.name}] ErrorBoundary caught:`, error, info.componentStack);
    }

    render() {
      if (this.state.hasError) {
        if (this.props.fallback) return this.props.fallback;
        return (
          <div className="component-error-boundary">
            <span className="ceb-icon" aria-hidden="true">{'\u26A0'}</span>
            <span className="ceb-name">{this.props.name}</span>
            <span className="ceb-msg">{this.state.error?.message ?? 'Unknown error'}</span>
            <button className="ceb-retry"
              onClick={() => this.setState({ hasError: false, error: null })}>
              Reload
            </button>
          </div>
        );
      }
      return this.props.children;
    }
  }
  ```

  **Create `src/components/ComponentErrorBoundary.css`:**
  ```css
  .component-error-boundary {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: var(--sp-2); padding: var(--sp-6); min-height: 120px;
    background: var(--bg-raised); border: 1px solid var(--border-soft);
    border-radius: var(--radius-md); font-family: var(--mono); text-align: center;
  }
  .ceb-icon { font-size: 24px; opacity: 0.6; }
  .ceb-name { font-size: var(--text-xs); color: var(--text-ghost); letter-spacing: .12em; text-transform: uppercase; }
  .ceb-msg { font-size: var(--text-sm); color: var(--text-dim); max-width: 300px; word-break: break-word; }
  .ceb-retry {
    margin-top: var(--sp-2); padding: var(--sp-1) var(--sp-4);
    font-family: var(--mono); font-size: var(--text-xs); letter-spacing: .15em; text-transform: uppercase;
    color: var(--rose-glow); background: transparent; border: 1px solid var(--rose-dim);
    border-radius: var(--radius-sm); cursor: pointer; transition: border-color var(--duration-snap);
  }
  .ceb-retry:hover { border-color: var(--rose-glow); }
  ```
  CSS tokens: `--sp-1`, `--sp-2`, `--sp-4`, `--sp-6`, `--bg-raised`, `--border-soft`, `--radius-md`, `--radius-sm`, `--mono`, `--text-xs`, `--text-sm`, `--text-dim`, `--text-ghost`, `--rose-glow`, `--rose-dim`, `--duration-snap`.

  **Components to wrap:**

  | Component | Wrap location | Boundary name |
  |---|---|---|
  | HeroParticleField | `AppShell.tsx` | `"HeroParticleField"` |
  | CostRace | wherever imported | `"CostRace"` |
  | KnowledgeFlowPanel | wherever imported | `"KnowledgeFlowPanel"` |
  | DreamPhaseViz | wherever imported | `"DreamPhaseViz"` |
  | MatrixRaceTrack | wherever imported | `"MatrixRaceTrack"` |
  | TokenVelocitySparkline | wherever imported | `"TokenVelocitySparkline"` |
  | ThresholdGauge | wherever imported | `"ThresholdGauge"` |
  | GateWaterfall | wherever imported | `"GateWaterfall"` |
  | All Charts/* | wherever imported | `"BarChart"`, `"CostChart"`, etc. |
  | Explorer graph | `Explorer.tsx` | `"SignalFlowGraph"` |
  | AgentFleet topology | `AgentFleet.tsx` | `"AgentFleetTopology"` |
  | KnowledgeGraph | `KnowledgeGraph.tsx` | `"KnowledgeGraph"` |

  **Pattern:**
  ```tsx
  import ComponentErrorBoundary from './ComponentErrorBoundary';

  <ComponentErrorBoundary name="CostRace">
    <CostRace data={data} />
  </ComponentErrorBoundary>
  ```

  **Step-by-step:**
  1. Create `src/components/ComponentErrorBoundary.tsx`.
  2. Create `src/components/ComponentErrorBoundary.css`.
  3. Wrap each canvas component (see table) with `<ComponentErrorBoundary>`.
  4. For HeroParticleField in AppShell.tsx:
     ```tsx
     <ComponentErrorBoundary name="HeroParticleField">
       <Suspense fallback={null}><LazyHeroParticleField /></Suspense>
     </ComponentErrorBoundary>
     ```

  **Verify:** Console: `document.querySelector('canvas').getContext('webgl')?.getExtension('WEBGL_lose_context')?.loseContext();` -- boundary catches, shows name + error + "Reload". Other components unaffected. Click "Reload" -- re-mounts.

- [ ] **T5.8: Standardize error handling across all fetch paths**

  **Read first:**
  - `src/stores/DataHub.ts` -- DataHub store (built in Phase 2)
  - `src/hooks/useApi.ts` -- current fetch wrapper (throws on non-OK)
  - `src/hooks/useApiWithFallback.ts` -- current fallback wrapper (swallows errors)

  **Add to DataHub store** (`src/stores/DataHub.ts`):
  ```tsx
  interface DataHubState {
    // ... existing fields ...
    errors: Record<string, string>;
    setError: (endpoint: string, message: string) => void;
    clearError: (endpoint: string) => void;
    clearAllErrors: () => void;
  }

  // In store creator:
  errors: {},
  setError: (endpoint, message) => set((s) => ({
    errors: { ...s.errors, [endpoint]: message },
  })),
  clearError: (endpoint) => set((s) => {
    const { [endpoint]: _, ...rest } = s.errors;
    return { errors: rest };
  }),
  clearAllErrors: () => set({ errors: {} }),
  ```

  **DataHub fetch pattern** (use in every action):
  ```tsx
  async fetchPlans() {
    const endpoint = '/api/plans';
    try {
      const res = await fetch(`${SERVE_URL}${endpoint}`);
      if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
      const data = await res.json();
      set({ plans: data });
      get().clearError(endpoint);
    } catch (err) {
      get().setError(endpoint, err instanceof Error ? err.message : 'Unknown error');
    }
  },
  ```

  **Create `src/components/DataSurface.tsx`:**
  ```tsx
  import { useDataHub } from '../stores/DataHub';
  import type { ReactNode } from 'react';
  import './DataSurface.css';

  interface DataSurfaceProps {
    endpoint: string;
    loading?: boolean;
    children: ReactNode;
    onRetry?: () => void;
  }

  export default function DataSurface({ endpoint, loading, children, onRetry }: DataSurfaceProps) {
    const error = useDataHub((s) => s.errors[endpoint]);
    if (loading) return <div className="ds-loading">Loading...</div>;
    if (error) {
      return (
        <div className="ds-error">
          <span className="ds-error-msg">{error}</span>
          {onRetry && <button className="ds-retry" onClick={onRetry}>Retry</button>}
        </div>
      );
    }
    return <>{children}</>;
  }
  ```

  **Create `src/components/DataSurface.css`:**
  ```css
  .ds-loading, .ds-error {
    display: flex; align-items: center; justify-content: center; gap: var(--sp-3);
    padding: var(--sp-6); min-height: 80px;
    font-family: var(--mono); font-size: var(--text-sm); color: var(--text-dim);
  }
  .ds-error { color: var(--status-error); }
  .ds-error-msg { max-width: 400px; text-align: center; }
  .ds-retry {
    padding: var(--sp-1) var(--sp-3); font-family: var(--mono); font-size: var(--text-xs);
    letter-spacing: .12em; text-transform: uppercase;
    color: var(--rose-glow); background: transparent; border: 1px solid var(--rose-dim);
    border-radius: var(--radius-sm); cursor: pointer;
  }
  ```
  CSS tokens: `--sp-1`, `--sp-3`, `--sp-6`, `--mono`, `--text-xs`, `--text-sm`, `--text-dim`, `--status-error`, `--rose-glow`, `--rose-dim`, `--radius-sm`.

  **Usage in scenes:**
  ```tsx
  <DataSurface endpoint="/api/plans" loading={!plans} onRetry={() => store.fetchPlans()}>
    <PlanList plans={plans} />
  </DataSurface>
  ```

  **Step-by-step:**
  1. Add `errors`, `setError`, `clearError`, `clearAllErrors` to DataHub.
  2. Update every `fetch()` in DataHub actions to use `setError`/`clearError`.
  3. Create `src/components/DataSurface.tsx`.
  4. Create `src/components/DataSurface.css`.
  5. Wrap data-dependent sections in each scene with `<DataSurface>`.

  **Verify:** Stop roko-serve. Navigate to data-loading page. DataSurface shows error + "Retry". Start server. Click "Retry". Data loads.

- [ ] **T5.9: Workspace cleanup/GC**

  **Read first:**
  - `src/hooks/useWorkspace.ts` -- current workspace provider with `destroyWorkspace`
  - `crates/roko-serve/src/routes/run.rs` -- workspace creation endpoint

  **Server-side: background GC task** (add to `crates/roko-serve/src/state.rs` at server startup):
  ```rust
  // Spawn workspace GC on server start
  tokio::spawn(async move {
      let ttl = std::time::Duration::from_secs(3600); // 1 hour
      let interval = std::time::Duration::from_secs(300); // 5 min
      loop {
          tokio::time::sleep(interval).await;
          if let Ok(entries) = std::fs::read_dir("/tmp") {
              let cutoff = std::time::SystemTime::now() - ttl;
              for entry in entries.flatten() {
                  let name = entry.file_name();
                  let name_str = name.to_string_lossy();
                  if name_str.starts_with("roko-ws-") {
                      if let Ok(meta) = entry.metadata() {
                          if let Ok(modified) = meta.modified() {
                              if modified < cutoff {
                                  let _ = std::fs::remove_dir_all(entry.path());
                                  tracing::info!("GC: removed expired workspace {}", name_str);
                              }
                          }
                      }
                  }
              }
          }
      }
  });
  ```

  **Client-side: DataHub action:**
  ```tsx
  // In DataHub store:
  async destroyWorkspace(id: string) {
    await fetch(`${SERVE_URL}/api/workspaces/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    });
    set((s) => {
      const { [id]: _, ...rest } = s.workspaces;
      return { workspaces: rest };
    });
  },
  ```

  **Step-by-step:**
  1. Add workspace GC background task to roko-serve startup.
  2. Add `destroyWorkspace` action to DataHub.
  3. In Build scene, add "Close workspace" button calling `destroyWorkspace`.

  **Verify:** Create workspace via Build page. Wait 5 min (or set TTL to 10s for test). `/tmp/roko-ws-*` dir deleted. DataHub cache cleared.

### 5.4 Inline Style Reduction

- [ ] **T5.10: Convert static inline styles to CSS classes**

  **Read first:** Each file below -- scan for `style={{`.

  **Actual inline style counts** (from `grep -c 'style={{' <file>`):

  | # | File | Count | Has CSS file? |
  |---|---|---|---|
  | 1 | `src/components/DreamPhaseViz.tsx` | 23 | No -- create `DreamPhaseViz.css` |
  | 2 | `src/pages/Bench.tsx` | 20 | Yes -- `src/pages/Bench.css` |
  | 3 | `src/pages/BenchRunDetail.tsx` | 18 | No -- create `BenchRunDetail.css` |
  | 4 | `src/pages/Share.tsx` | 11 | No -- create `Share.css` |
  | 5 | `src/pages/Explorer.tsx` | 11 | Yes -- `src/pages/Explorer.css` |
  | 6 | `src/pages/BenchCompare.tsx` | 8 | No -- create `BenchCompare.css` |
  | 7 | `src/components/ThresholdGauge.tsx` | 7 | No -- create `ThresholdGauge.css` |

  Total: 98 inline styles across 7 files. Target: reduce to ~23 (keep only dynamic/data-driven).

  **Rules -- what to extract vs. keep inline:**
  - EXTRACT to CSS: fixed layout (`display`, `gap`, `padding`, `margin`, `flexDirection`), fixed colors (`background`, `color`, `border`), fixed dimensions (`width`, `height`, `minHeight`), typography (`fontSize`, `fontFamily`, `letterSpacing`).
  - KEEP inline: data-driven values (`width: \`${percent}%\``), conditional (`display: isOpen ? 'flex' : 'none'`).
  - Replace hardcoded colors with tokens: `'#8a9c86'` -> `var(--success)`, `'rgba(255,255,255,0.05)'` -> `var(--glass-bg)`.

  **Step-by-step per file:**

  1. **`src/components/DreamPhaseViz.tsx`** (23 inline):
     Create `src/components/DreamPhaseViz.css`. Add `import './DreamPhaseViz.css';`.
     Extract statics to classes prefixed `dpv-`. Target: ~18 extracted, ~5 kept.

  2. **`src/pages/Bench.tsx`** (20 inline):
     CSS exists: `src/pages/Bench.css`. Extract to classes prefixed `bench-`. Target: ~15 extracted, ~5 kept.

  3. **`src/pages/BenchRunDetail.tsx`** (18 inline):
     Create `src/pages/BenchRunDetail.css`. Add import. Prefix: `brd-`. Target: ~14 extracted, ~4 kept.

  4. **`src/pages/Share.tsx`** (11 inline):
     Create `src/pages/Share.css`. Add import. Prefix: `share-`. Target: ~9 extracted, ~2 kept.

  5. **`src/pages/Explorer.tsx`** (11 inline):
     CSS exists: `src/pages/Explorer.css`. Prefix `expl-`. Target: ~8 extracted, ~3 kept.

  6. **`src/pages/BenchCompare.tsx`** (8 inline):
     Create `src/pages/BenchCompare.css`. Add import. Prefix: `bc-`. Target: ~6 extracted, ~2 kept.

  7. **`src/components/ThresholdGauge.tsx`** (7 inline):
     Create `src/components/ThresholdGauge.css`. Add import. Prefix: `tg-`. Target: ~5 extracted, ~2 kept.

  **Before/after example (from Bench.tsx):**
  ```tsx
  // BEFORE:
  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
    <span style={{ fontFamily: 'var(--mono)', fontSize: 11, color: 'var(--text-dim)', letterSpacing: '.08em' }}>
      STRATEGY
    </span>
  </div>

  // AFTER:
  <div className="bench-strategy-row">
    <span className="bench-strategy-label">STRATEGY</span>
  </div>
  ```
  ```css
  /* In Bench.css: */
  .bench-strategy-row {
    display: flex; align-items: center; gap: var(--sp-2); margin-bottom: var(--sp-3);
  }
  .bench-strategy-label {
    font-family: var(--mono); font-size: var(--text-xs); color: var(--text-dim); letter-spacing: .08em;
  }
  ```

  **Verify per file:**
  ```bash
  grep -c 'style={{' src/components/DreamPhaseViz.tsx  # <= 5
  grep -c 'style={{' src/pages/Bench.tsx               # <= 5
  grep -c 'style={{' src/pages/BenchRunDetail.tsx      # <= 4
  grep -c 'style={{' src/pages/Share.tsx               # <= 2
  grep -c 'style={{' src/pages/Explorer.tsx            # <= 3
  grep -c 'style={{' src/pages/BenchCompare.tsx        # <= 2
  grep -c 'style={{' src/components/ThresholdGauge.tsx # <= 2
  ```
  Total from 98 down to ~23. Visual appearance unchanged -- screenshot compare before/after.

---

## Phase 6: Advanced Features

### 6.1 Blockchain / Mirage-RS

- [ ] **T6.1: Wire chain data panels — live blocks + positions from mirage-rs WS**

  **Read first:**
  - `src/hooks/useChain.ts` (full file) — `useChainWs` hook, WS message types `WsConnectedMsg` and `WsChannelMsg`, `InsightEvent`/`PheromoneEvent` types
  - `src/components/ChainActivityPanel.tsx` (lines 1-40) — `BlockData { number, timestamp, transactions: TxData[] }`, `TxData { hash, type, description }`
  - `src/components/LivePositionsPanel.tsx` (lines 1-18) — `AgentPosition { name, address, color, balances, keyMetric, strategy? }`
  - `src/components/ChainIntelPanel.tsx` (lines 1-34) — `ChainIntelPanelProps`, composite panel that stacks KnowledgeFlowPanel + ChainActivityPanel + LivePositionsPanel + EfficiencyBar
  - `src/pages/Demo.tsx` (lines 92-116) — existing chain state: `ciBlocks`, `ciPositions`, `ciInsights`, `ciMetrics`, `ciLeftAgent`, `ciRightAgent`

  **File to modify:** `src/pages/Demo.tsx`

  **WS endpoint:** `ws://localhost:8545/api/ws?insights=true&pheromones=true&agents=true` (already connected via `useChainWs` at line 93)

  **WS message types (from `useChain.ts`):**
  ```ts
  // Server confirmation
  { type: 'connected', pheromones: boolean, insights: boolean, agents: boolean, predictions: boolean }
  // Channel data
  { channel: 'insight' | 'pheromone' | 'agent', data: InsightEvent | PheromoneEvent }
  // Backpressure
  { channel: string, type: 'lagged', missed: number }
  ```

  **Step 1 — Add block accumulation from WS pheromone events (Demo.tsx, after line ~116):**
  The `useChainWs` hook already parses `insight` and `pheromone` channels. Blocks are synthesized from pheromone events (each pheromone deposit ~= a block confirmation). Add a `useEffect` that derives `BlockData[]` from pheromone events:
  ```ts
  // After line ~116, replace the static `const [ciBlocks] = useState<BlockData[]>([]);`
  // with a derived computation:
  const ciBlocks: BlockData[] = useMemo(() => {
    return chainWs.pheromones.slice(-20).map((p, i) => ({
      number: p.id,
      timestamp: p.depositedAt,
      transactions: [{
        hash: `0x${p.id.toString(16).padStart(8, '0')}${p.kind.slice(0, 4)}`,
        type: (p.kind === 'strategy' ? 'defi' : p.kind === 'causal' ? 'insight' : 'other') as TxData['type'],
        description: `${p.kind} pheromone (intensity: ${p.intensity.toFixed(2)})`,
      }],
    }));
  }, [chainWs.pheromones]);
  ```

  **Step 2 — Add position updates from WS agent channel:**
  The current `ciPositions` state at line 95-116 is static. Add a `useEffect` that updates balances from agent-channel WS messages:
  ```ts
  // After the ciBlocks derivation, add:
  useEffect(() => {
    if (!chainWs.connected) return;
    // Update positions when new pheromone data indicates activity
    const totalPheromones = chainWs.stats.pheromones;
    if (totalPheromones > 0) {
      setCiPositions(prev => prev.map((pos, i) => ({
        ...pos,
        keyMetric: i === 0
          ? { label: 'APR', value: `${(5 + totalPheromones * 0.1).toFixed(1)}%` }
          : { label: 'HF', value: `${(2.0 + totalPheromones * 0.02).toFixed(2)}` },
      })));
    }
  }, [chainWs.connected, chainWs.stats.pheromones]);
  ```

  **Step 3 — Make ciPositions mutable:**
  Change line 95 from `const [ciPositions] = useState<AgentPosition[]>([...])` to `const [ciPositions, setCiPositions] = useState<AgentPosition[]>([...])`.

  **Step 4 — Import TxData type:**
  At line 21, the existing import `import type { BlockData } from '../components/ChainActivityPanel';` — change to:
  ```ts
  import type { BlockData, TxData } from '../components/ChainActivityPanel';
  ```

  **Verification:**
  ```bash
  # Start mirage-rs in one terminal:
  cargo run -p mirage-rs --features chain -- --rpc-url https://eth.llamarpc.com --block-interval-ms 2000
  # Start demo-app in another:
  cd demo/demo-app && npm run dev
  # Navigate to /demo, select "Chain Intelligence" scenario
  # Verify: ChainActivityPanel shows blocks with pheromone-derived transactions
  # Verify: LivePositionsPanel shows updating APR/HF metrics
  # Verify: No console errors when mirage is offline (graceful degradation)
  ```

### 6.2 Self-Learning Visualization

- [ ] **T6.2: Wire learning feedback loop visualization — router confidence, gate thresholds, experiments**

  **Read first:**
  - `src/components/inference/ConfidenceMeter.tsx` (lines 1-30) — `ConfidenceMeterProps { confidence: number, trend: 'improving'|'stable'|'declining', decisions: number, label?, compact?, className? }`
  - `src/components/PrdPipelinePanel.tsx` (lines 280-310) — Routing section with tier cards and gate summary
  - `src/hooks/useEventStream.ts` (lines 1-40) — `EventStreamManager { connected, subscribe(types[], handler), destroy() }`, SSE at `${baseUrl}/api/events`
  - `src/contexts/EventStreamContext.tsx` — `useEventStreamContext()` returns `{ connected, manager }`

  **API endpoints (roko-serve on :6677):**
  - `GET /api/learn/router` — `{ model_stats: Record<string, { selections, successes, avg_confidence }>, total_decisions }`
  - `GET /api/learn/gate-thresholds` — `{ thresholds: Record<string, { ema, count, last_updated }> }`
  - `GET /api/learn/experiments` — `{ experiments: { id, variant_a, variant_b, a_wins, b_wins, status }[] }`

  **SSE event types (from `/api/events`):**
  - `InferenceCompleted` — `{ type: 'InferenceCompleted', model, tier, tokens_in, tokens_out, cost, router_confidence?, duration_ms }`
  - `GateResult` — `{ type: 'GateResult', task_id, gate_name, passed, duration_ms, message? }`

  **File to create:** `src/hooks/useLearningStats.ts`

  **Interface:**
  ```ts
  export interface LearningStats {
    routerConfidence: number;       // 0-1, from latest InferenceCompleted or /api/learn/router
    confidenceTrend: 'improving' | 'stable' | 'declining';
    totalDecisions: number;
    gateThresholds: { name: string; ema: number; count: number }[];
    experiments: { id: string; variantA: string; variantB: string; aWins: number; bWins: number; status: string }[];
    loading: boolean;
  }
  export function useLearningStats(): LearningStats;
  ```

  **Implementation steps:**

  1. Create `src/hooks/useLearningStats.ts`:
     - Import `useEventStreamContext` from `../contexts/EventStreamContext`
     - Import `useApi` from `./useApi`
     - On mount, `GET /api/learn/router` to seed initial `routerConfidence` and `totalDecisions`
     - Subscribe to `InferenceCompleted` via `manager.subscribe(['InferenceCompleted'], handler)`
     - On each `InferenceCompleted` event: update `routerConfidence` from `event.router_confidence` (if present), increment `totalDecisions`, compute `confidenceTrend` by comparing current to previous value (delta > 0.02 = improving, delta < -0.02 = declining, else stable)
     - Poll `/api/learn/gate-thresholds` and `/api/learn/experiments` every 10s
     - Return cleanup in useEffect

  2. Wire into `src/components/PrdPipelinePanel.tsx`:
     - **Import:** `import { ConfidenceMeter } from './inference';`
     - **Props addition:** Add `learningStats?: LearningStats` to the inline props type at line 125
     - **Insert after the gates section (line ~308):** Inside the `hasTasks` conditional, after `pp-gates` div:
       ```tsx
       {learningStats && learningStats.totalDecisions > 0 && (
         <div className="pp-router-confidence" style={{ marginTop: 8 }}>
           <ConfidenceMeter
             confidence={learningStats.routerConfidence}
             trend={learningStats.confidenceTrend}
             decisions={learningStats.totalDecisions}
             label="ROUTER"
             compact
           />
         </div>
       )}
       ```

  3. Wire into `src/pages/Demo.tsx`:
     - **Import:** `import { useLearningStats } from '../hooks/useLearningStats';`
     - **Call hook** after line 66: `const learningStats = useLearningStats();`
     - **Pass to PrdPipelinePanel** at line 640: add prop `learningStats={learningStats}`
     - **Add to default sidebar** (line ~718-726 stats mosaic area): After the `MosaicCell` for MODEL, add a `ConfidenceMeter` row:
       ```tsx
       <RevealWhen visible={learningStats.totalDecisions > 0}>
         <div style={{ padding: '4px 16px' }}>
           <ConfidenceMeter
             confidence={learningStats.routerConfidence}
             trend={learningStats.confidenceTrend}
             decisions={learningStats.totalDecisions}
             label="ROUTER CONFIDENCE"
             compact
           />
         </div>
       </RevealWhen>
       ```

  **CSS tokens used:** None new — `ConfidenceMeter` uses its own `ConfidenceMeter.css` which references `--rose-bright`, `--bone-bright`, `--dream-bright` from rosedust.

  **Verification:**
  ```bash
  # Start roko serve:
  cargo run -p roko-cli -- serve
  # Start demo-app:
  cd demo/demo-app && npm run dev
  # Run a scenario that triggers inference (PRD Pipeline or Self-Developing Agent)
  # Verify: ConfidenceMeter appears in PrdPipelinePanel routing section after first inference
  # Verify: Confidence bar width animates, trend arrow updates
  # Verify: Default sidebar shows confidence meter when decisions > 0
  # Verify: No errors when serve is offline (loading state, graceful fallback)
  ```

### 6.3 Agent-to-Agent

- [ ] **T6.3: External agent connection panel — AgentHandoff in fleet context**

  **Read first:**
  - `src/components/agent/AgentHandoff.tsx` (lines 1-21) — `AgentHandoffProps { from: AgentInfo, to: AgentInfo, status: 'pending'|'active'|'done'|'error', direction?, label?, sublabel?, artifacts?, progress?, compact?, className? }` where `AgentInfo { name, role?, status? }`
  - `src/hooks/useEventStream.ts` — SSE subscription pattern
  - `src/pages/Demo.tsx` (lines 649-683) — knowledge-transfer sidebar where agent coordination is displayed

  **SSE event types (from `/api/events`):**
  - `AgentSpawned` — `{ type: 'AgentSpawned', agent_name, role, task_id }`
  - `AgentCompleted` — `{ type: 'AgentCompleted', agent_name, task_id, success }`
  - `TaskAssigned` — `{ type: 'TaskAssigned', task_id, agent_name, plan_id }`

  **WS endpoint (per-agent sidecar):** `ws://localhost:6678/api/agents/:name/stream` — sends agent lifecycle events

  **File to create:** `src/hooks/useAgentHandoffs.ts`

  **Interface:**
  ```ts
  import type { AgentHandoffProps } from '../components/agent/AgentHandoff';

  interface HandoffEntry {
    id: string;
    from: { name: string; role?: string; status?: 'idle' | 'working' | 'done' };
    to: { name: string; role?: string; status?: 'idle' | 'working' | 'done' };
    status: 'pending' | 'active' | 'done' | 'error';
    label: string;
    taskId?: string;
    timestamp: number;
  }

  export function useAgentHandoffs(): {
    handoffs: HandoffEntry[];
    activeHandoff: HandoffEntry | null;
  };
  ```

  **Implementation steps:**

  1. Create `src/hooks/useAgentHandoffs.ts`:
     - Import `useEventStreamContext` from `../contexts/EventStreamContext`
     - Subscribe to `['AgentSpawned', 'AgentCompleted', 'TaskAssigned']`
     - On `AgentSpawned`: push a new handoff entry with `status: 'pending'`, `from: { name: 'orchestrator', role: 'dispatcher' }`, `to: { name: event.agent_name, role: event.role }`
     - On `TaskAssigned`: find the handoff for this agent and update `status: 'active'`, set `label: event.task_id`
     - On `AgentCompleted`: find the handoff for this agent and update `status: event.success ? 'done' : 'error'`
     - Derive `activeHandoff` as the most recent entry with `status === 'active'`
     - Cap handoffs array at 20 entries (FIFO)

  2. Wire into `src/pages/Demo.tsx`:
     - **Import:** `import { useAgentHandoffs } from '../hooks/useAgentHandoffs';`
     - **Import:** `import AgentHandoff from '../components/agent/AgentHandoff';`
     - **Call hook** after the `useLearningStats` call: `const { handoffs, activeHandoff } = useAgentHandoffs();`
     - **Insert in knowledge-transfer sidebar** (line ~651, inside the `scenario.id === 'knowledge-transfer'` branch), before the `<RevealWhen visible={timelineDisplay.length > 0}>`:
       ```tsx
       <RevealWhen visible={activeHandoff !== null}>
         {activeHandoff && (
           <Pane title="HANDOFF" flat>
             <div style={{ padding: '8px 12px' }}>
               <AgentHandoff
                 from={activeHandoff.from}
                 to={activeHandoff.to}
                 status={activeHandoff.status}
                 direction="forward"
                 label={activeHandoff.label}
                 compact
               />
             </div>
           </Pane>
         )}
       </RevealWhen>
       ```
     - **Insert in PRD Pipeline sidebar** (line ~638-648, inside `scenario.id === 'prd-pipeline'` branch), add after the `<PrdPipelinePanel>`:
       ```tsx
       <RevealWhen visible={handoffs.length > 0}>
         <Pane title="AGENT FLOW" flat>
           <div style={{ padding: '8px 12px', display: 'flex', flexDirection: 'column', gap: 6 }}>
             {handoffs.slice(-3).map(h => (
               <AgentHandoff
                 key={h.id}
                 from={h.from}
                 to={h.to}
                 status={h.status}
                 direction="forward"
                 label={h.label}
                 compact
               />
             ))}
           </div>
         </Pane>
       </RevealWhen>
       ```

  **Data flow:** SSE `/api/events` -> `EventStreamManager` -> `useAgentHandoffs` -> `handoffs[]` state -> `AgentHandoff` component props

  **Verification:**
  ```bash
  # Start roko serve:
  cargo run -p roko-cli -- serve
  # Start demo-app:
  cd demo/demo-app && npm run dev
  # Run PRD Pipeline scenario — agents spawn for plan generation + task execution
  # Verify: "AGENT FLOW" pane appears in PRD Pipeline sidebar showing orchestrator -> agent handoffs
  # Verify: Status transitions from pending -> active -> done as tasks progress
  # Verify: Knowledge Transfer scenario shows active handoff between Alpha and Beta agents
  # Verify: No errors when serve is offline
  ```

---

## Phase 7: Expressive Primitives

New component library for rich layout, loading transitions, ambient WebGL effects, agent-scoped visuals, event feeds, overlays, and advanced layout. These are reusable building blocks for future scene work.

### 7.1 Fix Critical Layout Issues

- [x] **T7.1: Fix navbar disappearing on resize** ✓ DONE
  - Root cause: `body { overflow-y: hidden }` in rosedust.css + TopNav at `position: relative`
  - Fix: Make TopNav `position: sticky; top: 0; z-index: 100`. Change AppShell layout to flex column with content area `flex: 1; overflow-y: auto`. Remove `overflow-y: hidden` from body (keep `overflow-x: hidden` if needed).
  - Files: `rosedust.css`, `TopNav.css`, `AppShell.tsx`
  - **Verify**: Resize panes aggressively. Navbar always visible and clickable.

- [ ] **T7.2: Fix pane loading race conditions — readiness gate + WS message buffer**

  **Read first:**
  - `src/hooks/useTerminal.ts` (full file, 322 lines) — the `useTerminal` hook
  - `src/pages/Demo.tsx` (lines 308-320) — `waitForTerminalReadiness` that polls for connected state
  - `src/pages/Demo.tsx` (lines 622-631) — `TerminalPaneWithHandle` rendering in the grid

  **Root cause analysis (3 separate issues):**

  1. **WS connects before xterm fits (useTerminal.ts line 126 vs line 253-301):** `term.open(el)` at line 126 triggers a synchronous DOM layout, but `fitAddon.fit()` runs in a `requestAnimationFrame` callback at line 138. Meanwhile, `connectWs()` at line 303 fires immediately and can receive data (onmessage at line 275) before the fit has sized the terminal. If a `resize` JSON message is sent on open (line 265-268), the `proposeDimensions()` may return stale values.

  2. **WS data arrives before xterm is fully attached:** The `ws.onmessage` handler at line 275 calls `term.write()` directly. If the terminal is not yet fitted, the data is written to a 0-row terminal, causing blank output.

  3. **Scenario switch causes state array mismatch (Demo.tsx):** When `activeIdx` changes, `sessionIds` regenerate but `handleRefs.current` may still reference old terminals. The `updateTerminalState` callback at line ~534 mutates `terminalStates` by index, but the old state array has stale length from the previous scenario.

  **Fix — File 1: `src/hooks/useTerminal.ts`**

  **Step 1 — Add a readiness flag and message buffer (inside `useEffect`, after line 124):**
  ```ts
  // After: let disposed = false;
  let ready = false;
  const pendingMessages: Uint8Array[] = [];
  ```

  **Step 2 — Gate the WS connection on fit completion (replace lines 138-148):**
  Replace the `requestAnimationFrame` block:
  ```ts
  // OLD (lines 138-148):
  requestAnimationFrame(() => {
    if (!disposed) {
      try {
        fitAddon.fit();
        const rows = term.rows;
        if (rows > 1) {
          term.write('\n'.repeat(rows - 1));
        }
      } catch { /* disposed */ }
    }
  });

  // NEW:
  requestAnimationFrame(() => {
    if (disposed) return;
    try {
      fitAddon.fit();
      const rows = term.rows;
      if (rows > 1) {
        term.write('\n'.repeat(rows - 1));
      }
      // Flush any buffered messages
      ready = true;
      for (const buf of pendingMessages) {
        term.write(buf);
        appendOutput(new TextDecoder().decode(buf));
      }
      pendingMessages.length = 0;
      // Now connect WS (was previously called immediately)
      connectWs();
    } catch { /* disposed */ }
  });
  ```

  **Step 3 — Buffer messages when not ready (replace lines 275-284 in ws.onmessage):**
  ```ts
  // OLD:
  ws.onmessage = (e: MessageEvent) => {
    if (disposed) return;
    if (e.data instanceof ArrayBuffer) {
      const text = new TextDecoder().decode(e.data);
      term.write(new Uint8Array(e.data));
      appendOutput(text);
    } else if (typeof e.data === 'string') {
      term.write(e.data);
      appendOutput(e.data);
    }
  };

  // NEW:
  ws.onmessage = (e: MessageEvent) => {
    if (disposed) return;
    if (e.data instanceof ArrayBuffer) {
      const bytes = new Uint8Array(e.data);
      if (!ready) {
        pendingMessages.push(bytes);
        return;
      }
      term.write(bytes);
      appendOutput(new TextDecoder().decode(e.data));
    } else if (typeof e.data === 'string') {
      if (!ready) {
        pendingMessages.push(new TextEncoder().encode(e.data));
        return;
      }
      term.write(e.data);
      appendOutput(e.data);
    }
  };
  ```

  **Step 4 — Remove the immediate `connectWs()` call at line 303:**
  Delete or comment out `connectWs();` — it is now called from the `requestAnimationFrame` callback after fit completes.

  **Fix — File 2: `src/pages/Demo.tsx`**

  **Step 5 — Reset terminal states on scenario switch (around line 534):**
  In the `updateTerminalState` callback, add a bounds check:
  ```ts
  // OLD (line ~534):
  const updateTerminalState = useCallback((index: number, state: TerminalPaneState) => {
    setTerminalStates((prev) => {
      ...
    });
  }, []);

  // NEW — add bounds check:
  const updateTerminalState = useCallback((index: number, state: TerminalPaneState) => {
    setTerminalStates((prev) => {
      if (index < 0 || index >= scenario.panes) return prev;
      while (prev.length < scenario.panes) prev = [...prev, { status: 'connecting', connected: false }];
      const next = prev.slice();
      next[index] = state;
      return next;
    });
  }, [scenario.panes]);
  ```

  **Step 6 — Clear terminal states on scenario change:**
  In the `selectScenario` handler (find the function that sets `setActiveIdx`), add:
  ```ts
  setTerminalStates([]);
  ```

  **Verification:**
  ```bash
  cd demo/demo-app && npm run dev
  # 1. Open /demo page
  # 2. Rapidly click between scenario tabs (1-5) at least 10 times in quick succession
  # 3. Verify: No blank panes — every terminal shows a shell prompt after connection
  # 4. Verify: No console errors (check DevTools Console, filter for "error" and "uncaught")
  # 5. Verify: Terminal output appears correctly even for the first few lines (no missing initial output)
  # 6. Verify: Resize browser window during scenario switch — no layout corruption
  ```

### 7.1b ASCII / Terminal Aesthetic

- [x] **T7.35: Restyle TopNav with terminal/demoscene aesthetic** ✓ DONE
  - Brand: `⌈ NUNCHI ⌋` mono uppercase with rose LED
  - Links: `▸ DEMO ◂` active state, `[ DEMO ]` hover brackets, mono 11px uppercase
  - Status pill: `● LIVE 2h 28m` / `○ SEED` terminal style
  - Removed FlatIcon dependency from nav

- [x] **T7.36: Create `components/ascii/` primitives** ✓ DONE
  - AsciiLabel (5 frame variants, typewriter + flicker animations)
  - AsciiDivider (6 variants: line, double, dashed, dotted, braille, chevron)
  - AsciiFrame (4 variants: single, double, rounded, heavy)
  - AsciiBraille (4 patterns: noise, wave, density, spinner at ~10fps)
  - AsciiProgress (3 variants: blocks, braille, arrows)
  - AsciiWaveform (8-level block oscilloscope)

### 7.1c Layout Density

- [x] **T7.37: Fix global layout to scrollable container** ✓ DONE
  - Body: removed viewport lock, content scrolls naturally
  - TopNav: stays sticky at top
  - app-frame: flex child, no overflow hidden
  - Pages flow naturally, no viewport-locking

- [x] **T7.38: Apply density principles to all existing pages** ✓ DONE
  - Builder.css: removed viewport lock (height: calc(100vh-48px) → flex), removed overflow hidden
  - Terminal.css: replaced height: 100% with flex, removed overflow hidden
  - Settings.css: converted to flex, replaced rem values with design tokens
  - Explorer.css: tightened stat pill, heatmap, card section padding
  - Bench.css: tightened hero padding, stats margin, title size
  - Dashboard Layout.tsx: removed overflow hidden, tightened padding
  - rosedust.css: tightened pane head/body/foot, mosaic cell padding/icons, fixed html/root flex chain

### 7.2 Resizable Pane System

- [x] **T7.3: Create `layout/ResizablePane.tsx`** ✓ DONE — Glass panel with header, status LED, collapse, pointer-capture drag resize on right/bottom edges
- [ ] **T7.4: Create `layout/PaneGrid.tsx` — grid of resizable panes with proportional resize + localStorage persistence**

  **Read first:**
  - `src/components/layout/ResizablePane.tsx` (full file) — `ResizablePaneProps { id, label?, icon?, minWidth?, minHeight?, resizable?, showHeader?, headerActions?, collapsible?, collapsed?, onCollapse?, status?, children, className? }`, pointer-capture resize on right/bottom edges
  - `src/components/layout/ResizablePane.css` — glass panel styling, handle hit areas, collapse animation
  - `src/styles/rosedust.css` — `--glass`, `--surface-0`, `--pane-pad`, `--gap-sm`, `--gap-md`

  **File to create:** `src/components/layout/PaneGrid.tsx`

  **Interface:**
  ```ts
  import type { ReactNode } from 'react';

  export interface PaneGridItem {
    id: string;
    /** Grid area name (e.g., 'main', 'sidebar', 'footer') */
    area: string;
    /** Content to render inside this grid cell */
    children: ReactNode;
    /** Minimum width in px (default 120) */
    minWidth?: number;
    /** Minimum height in px (default 80) */
    minHeight?: number;
  }

  export interface PaneGridLayout {
    /** CSS grid-template-columns, e.g. '1fr 300px' */
    columns: string;
    /** CSS grid-template-rows, e.g. '1fr 200px' */
    rows: string;
    /** CSS grid-template-areas, e.g. '"main sidebar" "footer footer"' */
    areas: string;
  }

  export interface PaneGridProps {
    items: PaneGridItem[];
    layout: PaneGridLayout;
    /** localStorage key for persisted column/row sizes (default: none = no persistence) */
    persistKey?: string;
    /** Gap between grid cells in px (default 4) */
    gap?: number;
    className?: string;
  }

  export default function PaneGrid(props: PaneGridProps): JSX.Element;
  ```

  **Implementation steps:**

  1. Create `src/components/layout/PaneGrid.tsx`:

  2. **State:** Store `colSizes: number[]` and `rowSizes: number[]` in state. Initialize from `localStorage.getItem(persistKey)` (JSON parse) if `persistKey` provided, else derive from `layout.columns`/`layout.rows` by parsing fr/px values into pixel amounts based on container width.

  3. **Resize logic — column handles:**
     - Render invisible resize handles (4px wide, cursor: col-resize) between each column
     - On `onPointerDown`: call `setPointerCapture(e.pointerId)`, record `startX`, `startColSizes`
     - On `onPointerMove`: compute `deltaX = e.clientX - startX`, distribute proportionally:
       ```ts
       const newLeft = Math.max(items[leftIdx].minWidth ?? 120, startColSizes[leftIdx] + deltaX);
       const newRight = Math.max(items[rightIdx].minWidth ?? 120, startColSizes[rightIdx] - deltaX);
       setColSizes(prev => { const n = [...prev]; n[leftIdx] = newLeft; n[rightIdx] = newRight; return n; });
       ```
     - On `onPointerUp`: release capture, persist to localStorage

  4. **Resize logic — row handles:** Same pattern but vertical (cursor: row-resize), use `minHeight`, adjust `deltaY`.

  5. **Persistence:** On every resize end, write `JSON.stringify({ cols: colSizes, rows: rowSizes })` to `localStorage.setItem(persistKey, ...)`. On mount, read and validate against current item count.

  6. **Render:**
     ```tsx
     <div
       className={`pane-grid ${className ?? ''}`}
       style={{
         display: 'grid',
         gridTemplateColumns: colSizes.map(s => `${s}px`).join(' '),
         gridTemplateRows: rowSizes.map(s => `${s}px`).join(' '),
         gridTemplateAreas: layout.areas,
         gap,
       }}
     >
       {items.map(item => (
         <div key={item.id} style={{ gridArea: item.area, overflow: 'hidden' }}>
           {item.children}
         </div>
       ))}
       {/* Column resize handles */}
       {colSizes.slice(0, -1).map((_, i) => (
         <div
           key={`col-handle-${i}`}
           className="pane-grid__col-handle"
           style={{ gridColumn: `${i + 1} / ${i + 2}`, gridRow: '1 / -1', justifySelf: 'end' }}
           onPointerDown={handleColPointerDown(i)}
         />
       ))}
       {/* Row resize handles */}
       {rowSizes.slice(0, -1).map((_, i) => (
         <div
           key={`row-handle-${i}`}
           className="pane-grid__row-handle"
           style={{ gridRow: `${i + 1} / ${i + 2}`, gridColumn: '1 / -1', alignSelf: 'end' }}
           onPointerDown={handleRowPointerDown(i)}
         />
       ))}
     </div>
     ```

  7. Create `src/components/layout/PaneGrid.css`:
     ```css
     .pane-grid { position: relative; width: 100%; height: 100%; }
     .pane-grid__col-handle {
       width: 4px; cursor: col-resize; z-index: 10;
       background: transparent; transition: background 0.15s;
     }
     .pane-grid__col-handle:hover { background: var(--rose-bright, #d89ab2); opacity: 0.3; }
     .pane-grid__row-handle {
       height: 4px; cursor: row-resize; z-index: 10;
       background: transparent; transition: background 0.15s;
     }
     .pane-grid__row-handle:hover { background: var(--rose-bright, #d89ab2); opacity: 0.3; }
     ```

  8. Export from `src/components/layout/index.ts` (create if missing):
     ```ts
     export { default as PaneGrid } from './PaneGrid';
     export type { PaneGridProps, PaneGridItem, PaneGridLayout } from './PaneGrid';
     ```

  **Verification:**
  ```bash
  cd demo/demo-app && npx tsc --noEmit
  # Create a test page or storybook entry that renders:
  # <PaneGrid
  #   persistKey="test-grid"
  #   layout={{ columns: '1fr 300px', rows: '1fr 200px', areas: '"main sidebar" "footer footer"' }}
  #   items={[
  #     { id: 'main', area: 'main', children: <div>Main</div> },
  #     { id: 'sidebar', area: 'sidebar', children: <div>Sidebar</div> },
  #     { id: 'footer', area: 'footer', children: <div>Footer</div> },
  #   ]}
  # />
  # Verify: Drag column handle — both columns resize proportionally
  # Verify: Drag row handle — both rows resize proportionally
  # Verify: Reload page — layout restored from localStorage
  # Verify: No pane shrinks below minWidth/minHeight
  ```

- [ ] **T7.5: Create `layout/PaneGroup.tsx` — vertical/horizontal pane stacks with shared resize handles**

  **Read first:**
  - `src/components/layout/ResizablePane.tsx` (full file) — individual pane with glass styling
  - `src/components/layout/PaneGrid.tsx` (created in T7.4) — grid with pointer-capture resize pattern

  **File to create:** `src/components/layout/PaneGroup.tsx`

  **Interface:**
  ```ts
  import type { ReactNode } from 'react';

  export interface PaneGroupItem {
    id: string;
    /** Content to render */
    children: ReactNode;
    /** Initial size as fraction (0-1), all items must sum to 1.0 */
    initialSize?: number;
    /** Minimum size in px */
    minSize?: number;
  }

  export interface PaneGroupProps {
    /** Stack direction */
    direction: 'horizontal' | 'vertical';
    /** Pane items to stack */
    items: PaneGroupItem[];
    /** Gap between panes in px (default 2) */
    gap?: number;
    /** localStorage key for persisted sizes */
    persistKey?: string;
    className?: string;
  }

  export default function PaneGroup(props: PaneGroupProps): JSX.Element;
  ```

  **Implementation steps:**

  1. Create `src/components/layout/PaneGroup.tsx`:

  2. **State:** `sizes: number[]` — stores each pane's pixel size. Initialize by measuring container size via `useRef<HTMLDivElement>` + `useEffect` and distributing based on `initialSize` fractions (default: equal). If `persistKey` exists and localStorage has data, use that instead.

  3. **Measure container:**
     ```ts
     const containerRef = useRef<HTMLDivElement>(null);
     const [containerSize, setContainerSize] = useState(0);

     useEffect(() => {
       const el = containerRef.current;
       if (!el) return;
       const ro = new ResizeObserver(([entry]) => {
         const s = direction === 'horizontal' ? entry.contentRect.width : entry.contentRect.height;
         setContainerSize(s);
       });
       ro.observe(el);
       return () => ro.disconnect();
     }, [direction]);
     ```

  4. **Derive initial sizes from container:**
     ```ts
     useEffect(() => {
       if (containerSize <= 0) return;
       // Check localStorage first
       const stored = persistKey ? localStorage.getItem(persistKey) : null;
       if (stored) {
         try { const parsed = JSON.parse(stored); if (Array.isArray(parsed) && parsed.length === items.length) { setSizes(parsed); return; } } catch {}
       }
       const totalGap = (items.length - 1) * (gap ?? 2);
       const available = containerSize - totalGap;
       const fractions = items.map(it => it.initialSize ?? 1 / items.length);
       setSizes(fractions.map(f => Math.max(f * available, items.find(i => i.id)?.minSize ?? 60)));
     }, [containerSize, items.length]);
     ```

  5. **Shared resize handles:** Render a resize handle between each pair of panes. Use the same pointer-capture pattern as ResizablePane:
     ```ts
     const handlePointerDown = (handleIndex: number) => (e: React.PointerEvent) => {
       e.preventDefault();
       (e.target as HTMLElement).setPointerCapture(e.pointerId);
       dragRef.current = {
         index: handleIndex,
         startPos: direction === 'horizontal' ? e.clientX : e.clientY,
         startSizes: [...sizes],
       };
     };
     ```
     On `onPointerMove`: redistribute between `sizes[handleIndex]` and `sizes[handleIndex + 1]`, respecting `minSize`.
     On `onPointerUp`: release capture, persist if `persistKey`.

  6. **Render:**
     ```tsx
     <div
       ref={containerRef}
       className={`pane-group pane-group--${direction} ${className ?? ''}`}
       style={{ display: 'flex', flexDirection: direction === 'horizontal' ? 'row' : 'column', width: '100%', height: '100%' }}
     >
       {items.map((item, i) => (
         <Fragment key={item.id}>
           <div
             className="pane-group__pane"
             style={{
               [direction === 'horizontal' ? 'width' : 'height']: sizes[i] ?? 'auto',
               overflow: 'hidden',
               flexShrink: 0,
             }}
           >
             {item.children}
           </div>
           {i < items.length - 1 && (
             <div
               className={`pane-group__handle pane-group__handle--${direction}`}
               onPointerDown={handlePointerDown(i)}
               onPointerMove={handlePointerMove}
               onPointerUp={handlePointerUp}
             />
           )}
         </Fragment>
       ))}
     </div>
     ```

  7. Create `src/components/layout/PaneGroup.css`:
     ```css
     .pane-group { position: relative; }
     .pane-group__handle--horizontal {
       width: 4px; cursor: col-resize; flex-shrink: 0;
       background: transparent; transition: background 0.15s;
     }
     .pane-group__handle--horizontal:hover { background: rgba(216, 154, 178, 0.3); }
     .pane-group__handle--vertical {
       height: 4px; cursor: row-resize; flex-shrink: 0;
       background: transparent; transition: background 0.15s;
     }
     .pane-group__handle--vertical:hover { background: rgba(216, 154, 178, 0.3); }
     .pane-group__handle:active { background: var(--rose-bright, #d89ab2); opacity: 0.5; }
     ```

  8. Export from `src/components/layout/index.ts`:
     ```ts
     export { default as PaneGroup } from './PaneGroup';
     export type { PaneGroupProps, PaneGroupItem } from './PaneGroup';
     ```

  **Verification:**
  ```bash
  cd demo/demo-app && npx tsc --noEmit
  # Create test usage:
  # <PaneGroup direction="horizontal" persistKey="test-group" items={[
  #   { id: 'left', children: <div>Left</div>, initialSize: 0.3, minSize: 100 },
  #   { id: 'center', children: <div>Center</div>, initialSize: 0.5, minSize: 200 },
  #   { id: 'right', children: <div>Right</div>, initialSize: 0.2, minSize: 100 },
  # ]} />
  # Verify: Dragging handle between left/center resizes both, neither goes below minSize
  # Verify: Vertical direction stacks top-to-bottom with row-resize cursor
  # Verify: Refresh page — sizes restored from localStorage
  # Verify: Resizing browser window recalculates proportionally
  ```

### 7.3 Loading & Transition Primitives

- [x] **T7.6: Create `design/LoadingTransition.tsx`** ✓ DONE — CRT dither reveal (random/scanline/radial patterns), canvas overlay, prefers-reduced-motion support
- [x] **T7.7: Create `design/ContentSwitch.tsx`** ✓ DONE — Fade-through/crossfade content swap with skeleton intermediate, smooth height transitions
- [x] **T7.8: Create `design/LazyPane.tsx`** ✓ DONE — Connection-aware pane: connecting→skeleton, connected→data, error→retry, disconnected→banner+dimmed content
- [x] **T7.9: Create `design/ConnectionGuard.tsx`** ✓ DONE — Connection gate: braille spinner while connecting, error+retry+countdown, renders children when connected

### 7.4 Expressive Progress Components

- [x] **T7.10: Create `design/StepProgress.tsx`** ✓ DONE — Horizontal/vertical, gradient connecting lines, status-driven circles, active pulse, icon slots
- [x] **T7.11: Create `design/VerticalTimeline.tsx`** ✓ DONE — Left rail + nodes, glass cards, staggered fadeIn, maxHeight scroll with gradient fade
- [x] **T7.12: Create `design/CircularProgress.tsx`** ✓ DONE — SVG ring with gradient stroke, draw animation, display italic center metric
- [x] **T7.13: Create `design/MilestoneProgress.tsx`** ✓ DONE — Linear bar with diamond markers, crossing detection, sparkle celebration animation

### 7.5 Three.js / WebGL Ambient Primitives

- [x] **T7.14: Create `ambient/NoiseBackground.tsx`** ✓ DONE — ImageData pixel noise at half resolution, 10fps, configurable density/opacity/color
- [x] **T7.15: Create `ambient/FluidGradient.tsx`** ✓ DONE — 5 radial gradient blobs on Lissajous curves, 1/4 resolution, 15fps
- [x] **T7.16: Create `ambient/HeartbeatLine.tsx`** ✓ DONE — EKG waveform at 3 speeds (fast/medium/slow), glow stroke, 30fps cap
- [x] **T7.17: Create `ambient/GlitchOverlay.tsx`** ✓ DONE — Scanline noise + slice displacement + RGB shift, intensity-driven
- [x] **T7.18: Create `ambient/AmbientContainer.tsx`** ✓ DONE — Wrapper with noise/fluid/none background, z-index layering

### 7.6 Agent-Namespaced Components

- [x] **T7.19: Create `agent/AgentContainer.tsx`** ✓ DONE — Per-agent card with accent border, heartbeat LED, name+role header, status-driven glow
- [x] **T7.20: Create `agent/AgentMetricBar.tsx`** ✓ DONE — Horizontal metric strip with trend arrows, compact mode, display italic values
- [x] **T7.21: Create `agent/AgentFeed.tsx`** ✓ DONE — Scrollable event feed, type-colored badges, auto-scroll with manual override
- [x] **T7.22: Create `agent/AgentHeartbeat.tsx`** ✓ DONE — 3-timescale (fast 0.7s/medium 3s/slow 8s), 4 status colors, optional label
- [x] **T7.23: Create `agent/AgentAvatar.tsx`** ✓ DONE — Name-hashed color circle, 3 sizes, hover tooltip

### 7.7 Event Feed & Block Stream

- [x] **T7.24: Create `feeds/EventStream.tsx`** ✓ DONE — Scrollable event feed, severity badges, auto-scroll with "New events ↓" indicator, gradient fades
- [x] **T7.25: Create `feeds/BlockFeed.tsx`** ✓ DONE — Block stream with connecting line, node dots, hash truncation, slide-in + flash animations
- [x] **T7.26: Create `feeds/InferenceFeed.tsx`** ✓ DONE — LLM inference cards, model/provider/status/tokens/cost, streaming shimmer, running total

### 7.8 Floating Chat & Overlays

- [x] **T7.27: Create `overlay/FloatingChat.tsx`** ✓ DONE — Draggable chat widget, pointer-capture drag, minimize, streaming dots, auto-scroll
- [x] **T7.28: Create `overlay/Modal.tsx`** ✓ DONE — Portal-based, focus trap, escape/backdrop close, scale animation, body scroll lock
- [x] **T7.29: Create `overlay/Drawer.tsx`** ✓ DONE — Slide-in left/right/bottom, portal, backdrop blur, configurable width/height
- [x] **T7.30: Create `overlay/CommandPalette.tsx`** ✓ DONE — Fuzzy search, category grouping, keyboard navigation, shortcut hints

### 7.9 Advanced Layout

- [x] **T7.31: Fix StickyTopLayout** ✓ DONE (covered by T7.37 + T7.38 — TopNav sticky, content scrolls naturally)
- [x] **T7.32: Create `layout/TreeView.tsx`** ✓ DONE — Expandable tree, indent guides, keyboard nav, chevron animation, controlled/uncontrolled
- [x] **T7.33: Create `layout/VirtualList.tsx`** ✓ DONE — Fixed-height virtualized scroll, overscan buffer, imperative scrollToIndex

### 7.10 Generalize Three.js Backgrounds

- [ ] **T7.34: Generalize `HeroParticleField` into reusable `ParticleField` with configurable density, colors, speed, reactivity**

  **Read first:**
  - `src/components/HeroParticleField.tsx` (full file, 5 lines) — thin wrapper: `export default function HeroParticleField() { return <AmbientParticles />; }`
  - `src/components/AmbientParticles.tsx` (full file, ~84 lines) — the actual Canvas2D implementation: 30 particles, hardcoded colors `[220,165,189]` (rose) and `[200,184,144]` (bone), fixed velocity `0.00012`, fixed alpha `0.18 + sin * 0.12`, fixed shadowBlur `8*dpr`
  - `src/components/AppShell.tsx` (line 32) — `<HeroParticleField />` rendered as a full-screen fixed background

  **Current setup to extract (AmbientParticles.tsx):**
  - Particle struct: `{ x, y, vx, vy, sz, phase, hue: [r,g,b] }` — all hardcoded
  - Init loop (line 26-36): `N=30` particles, velocity `(Math.random()-0.5)*0.00012`, size `0.3 + Math.random()*1.4`
  - Render loop (line 50-67): `clearRect`, per-particle `arc()` with `shadowBlur: 8*dpr`
  - No configurable params, no reactivity to external input

  **Files to modify:**
  - `src/components/AmbientParticles.tsx` — refactor to accept props
  - `src/components/HeroParticleField.tsx` — pass hero-specific config to AmbientParticles

  **New interface for `AmbientParticles.tsx`:**
  ```ts
  export interface ParticleFieldConfig {
    /** Number of particles (default 30) */
    count?: number;
    /** Base velocity magnitude (default 0.00012) */
    speed?: number;
    /** Particle colors as [r,g,b] tuples (default: rosedust rose + bone) */
    colors?: [number, number, number][];
    /** Min particle radius in CSS px (default 0.3) */
    minSize?: number;
    /** Max particle radius in CSS px (default 1.7) */
    maxSize?: number;
    /** Base alpha (default 0.18) */
    baseAlpha?: number;
    /** Alpha oscillation amplitude (default 0.12) */
    alphaSwing?: number;
    /** Shadow blur radius multiplied by dpr (default 8) */
    glowRadius?: number;
    /** Animation speed factor (1.0 = default, 2.0 = double speed) */
    animSpeed?: number;
    /** Reactivity: 0-1 value that modulates speed + glow in real-time */
    reactivity?: number;
  }

  export interface AmbientParticlesProps {
    config?: ParticleFieldConfig;
    className?: string;
  }
  ```

  **Implementation steps:**

  1. **Refactor `src/components/AmbientParticles.tsx`:**

     Replace the hardcoded constants with props destructuring:
     ```ts
     // OLD (lines 3-4):
     const TAU = Math.PI * 2;
     const N = 30;

     // NEW:
     const TAU = Math.PI * 2;
     const DEFAULTS: Required<ParticleFieldConfig> = {
       count: 30,
       speed: 0.00012,
       colors: [[220, 165, 189], [200, 184, 144]],
       minSize: 0.3,
       maxSize: 1.7,
       baseAlpha: 0.18,
       alphaSwing: 0.12,
       glowRadius: 8,
       animSpeed: 1.0,
       reactivity: 0,
     };
     ```

     Replace the component signature:
     ```ts
     // OLD: export default function AmbientParticles() {
     // NEW:
     export default function AmbientParticles({ config, className }: AmbientParticlesProps) {
       const cfg = { ...DEFAULTS, ...config };
     ```

     Replace particle init (lines 26-36):
     ```ts
     // Use cfg.count, cfg.speed, cfg.colors, cfg.minSize, cfg.maxSize
     for (let i = 0; i < cfg.count; i++) {
       particles.push({
         x: Math.random(),
         y: Math.random(),
         vx: (Math.random() - 0.5) * cfg.speed,
         vy: (Math.random() - 0.5) * cfg.speed,
         sz: cfg.minSize + Math.random() * (cfg.maxSize - cfg.minSize),
         phase: Math.random() * TAU,
         hue: cfg.colors[i % cfg.colors.length],
       });
     }
     ```

     Replace render loop (lines 50-67):
     ```ts
     // Use cfg.animSpeed, cfg.baseAlpha, cfg.alphaSwing, cfg.glowRadius, cfg.reactivity
     function tick() {
       t += 0.005 * cfg.animSpeed;
       const w = can!.width, h = can!.height;
       const reactMult = 1 + (cfg.reactivity * 2); // reactivity boosts speed + glow
       ctx.clearRect(0, 0, w, h);
       for (const p of particles) {
         p.x += p.vx * reactMult;
         p.y += p.vy * reactMult;
         if (p.x < 0 || p.x > 1) p.vx *= -1;
         if (p.y < 0 || p.y > 1) p.vy *= -1;
         const a = cfg.baseAlpha + Math.sin(t * 1.5 + p.phase) * cfg.alphaSwing;
         ctx.fillStyle = `rgba(${p.hue[0]},${p.hue[1]},${p.hue[2]},${a})`;
         ctx.shadowBlur = cfg.glowRadius * dpr * reactMult;
         ctx.shadowColor = `rgba(${p.hue[0]},${p.hue[1]},${p.hue[2]},.5)`;
         ctx.beginPath();
         ctx.arc(p.x * w, p.y * h, p.sz * dpr, 0, TAU);
         ctx.fill();
       }
       ctx.shadowBlur = 0;
       raf = requestAnimationFrame(tick);
     }
     ```

     Add `className` to the container div (line 79):
     ```ts
     // OLD: <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0 }}>
     // NEW:
     <div className={className} style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0 }}>
     ```

     Store `cfg` in a ref so the animation loop always reads the latest config without re-initializing particles:
     ```ts
     const cfgRef = useRef(cfg);
     useEffect(() => { cfgRef.current = cfg; }, [cfg]);
     // In tick(): read from cfgRef.current instead of cfg
     ```

  2. **Update `src/components/HeroParticleField.tsx`:**
     ```ts
     // OLD:
     import AmbientParticles from './AmbientParticles';
     export default function HeroParticleField() {
       return <AmbientParticles />;
     }

     // NEW — no change needed since AmbientParticles now uses optional config prop.
     // HeroParticleField passes no config = defaults = identical to current behavior.
     ```

  3. **Export types from AmbientParticles:**
     Add to top of file: `export type { ParticleFieldConfig, AmbientParticlesProps };`

  **Usage examples for other scenes:**
  ```tsx
  // Dense, fast, dream-colored particles for a dashboard background:
  <AmbientParticles config={{ count: 60, speed: 0.0003, colors: [[180, 140, 200]], animSpeed: 1.5 }} />

  // Reactive particles that pulse with agent activity:
  <AmbientParticles config={{ reactivity: agentActivityLevel, glowRadius: 12, count: 40 }} />
  ```

  **Verification:**
  ```bash
  cd demo/demo-app && npx tsc --noEmit
  # Open /demo — HeroParticleField should look identical to before (default config)
  # Temporarily test with custom config in AppShell.tsx:
  #   <AmbientParticles config={{ count: 60, speed: 0.0005, colors: [[255, 100, 100]], reactivity: 0.5 }} />
  # Verify: More particles, faster, red-tinted, larger glow
  # Revert to <HeroParticleField /> — confirm original appearance
  ```

### 7.11 Inference & Cybernetic Primitives

- [x] **T7.39: Create `inference/InferenceTag.tsx`** ✓ DONE — Compact annotation pill: tier badge (T0/T1/T2), model name, tokens, cost, latency
- [x] **T7.40: Create `inference/ModelSlot.tsx`** ✓ DONE — Slot-machine character animation when model changes, staggered per-character roll
- [x] **T7.41: Create `inference/CyberneticIntensity.tsx`** ✓ DONE — Progressive wrapper: 0-1 value maps to ghost→emerging→building→confident→crystallized visual states
- [x] **T7.42: Create `inference/ConfidenceMeter.tsx`** ✓ DONE — Cascade router confidence bar with trend arrow, crystallization at >0.8
- [x] **T7.43: Create `inference/TraceAnnotation.tsx`** ✓ DONE — Inline annotation strip: agent badge, tier/model, confidence intensity, cost, tokens
- [x] **T7.44: Create `inference/ArtifactGem.tsx`** ✓ DONE — Collectible artifact: type-shaped gem (hexagon/diamond/circle/square), significance-driven sparkle
- [x] **T7.45: Create `inference/CrystallizeTransition.tsx`** ✓ DONE — Achievement celebration: canvas particle burst + prismatic shimmer + ring pulse
- [x] **T7.46: Create `inference/ArtifactTray.tsx`** ✓ DONE — Persistent collectible counter: artifact shape icons + counts, sparkle on new additions

### 7.12 Design System Updates

- [x] **T7.47: Add inference/cybernetic/artifact specs to design system** ✓ DONE
  - Updated 04-DESIGN-SYSTEM.md: section 11 (Inference & Cybernetic State System) with progressive intensity scale, tier colors, crystallization effect, artifact collectibles, model slot machine, trace annotations
  - Updated 10-EXPRESSIVE-PRIMITIVES.md: section 10 (Inference, Learning & Artifact Primitives) with full component specs for all 8 new components

### 7.13 Multi-Agent Coordination

- [x] **T7.48: Create `agent/AgentHandoff.tsx` — multi-agent handoff visualization** ✓ DONE
  - Crystal flow animation between agent nodes with directional particles (5 per direction)
  - Status states: pending (dashed connector), active (flowing crystals + glow), done (settled green), error (red broken line)
  - Direction modes: forward, reverse, bidirectional (particles flow both ways)
  - Compact variant for inline use (smaller avatars, single-line layout)
  - Props: `from: AgentInfo`, `to: AgentInfo`, `status`, `direction`, `label`, `sublabel`, `artifacts`, `progress`, `compact`
  - Uses `AgentAvatar` for agent identity nodes
  - File: `src/components/agent/AgentHandoff.tsx` + `AgentHandoff.css`

### 7.14 Terminal Improvements

- [x] **T7.49: xterm terminal configuration improvements** ✓ DONE
  - Smaller font: 12px (was 14px) with JetBrainsMono Nerd Font Mono
  - Bar cursor (2px width) with rosedust-themed cursor color `#d89ab2`
  - Line height 1.1 (tighter than default 1.2)
  - Scrollback 5000 lines (was 1000)
  - Smooth scroll duration 80ms
  - Custom glyphs enabled, overview ruler (8px width)
  - Bottom-anchored text approach via newline padding
  - Enhanced rosedust theme: `selectionBackground: rgba(184, 122, 148, 0.28)`, `selectionForeground: #f0e4d0`, `selectionInactiveBackground: rgba(68, 56, 68, 0.4)`, full 16-color ANSI palette
  - Files: `src/hooks/useTerminal.ts`, `src/lib/rosedust-theme.ts`

- [x] **T7.50: xterm CSS polish** ✓ DONE
  - Thin rose-tinted scrollbar: 6px width, `rgba(168, 112, 140, 0.3)` thumb, `rgba(168, 112, 140, 0.5)` hover
  - Focus-within glow: `0 0 0 1px rgba(220, 165, 189, 0.2)` ring on terminal container
  - Selection border-radius and viewport styling for `.xterm-viewport`
  - File: `src/components/Terminal/TerminalPane.css`

- [x] **T7.51: Terminal header density** ✓ DONE
  - Slimmer header bar: 2px 10px padding (was 6px 12px)
  - Label font: 10px mono (was 11px)
  - Status font: 9px mono (was 10px)
  - Status dot: 5px (was 6px)
  - File: `src/pages/Demo.css`

### 7.15 Gate Verification

- [x] **T7.52: Create `GateVerdictTicker` component** ✓ DONE
  - Horizontal strip of gate verdict chips grouped by task ID
  - Each chip: pass/fail icon (checkmark/cross) + gate name + duration in ms
  - Task grouping with truncated task ID labels and dividers
  - Current task highlighting (non-current tasks dimmed)
  - Props: `verdicts: GateVerdictItem[]`, `currentTaskId?: string`
  - `GateVerdictItem`: `{ taskId, gate, passed, message?, durationMs }`
  - File: `src/components/GateVerdictTicker.tsx` + `GateVerdictTicker.css`

### 7.16 Integration Wiring (PENDING)

These tasks wire the new expressive primitives into the actual demo pages and scenarios.

- [ ] **T7.53: Wire `AgentHandoff` into Demo page scenarios**
  - Show handoff visualization in PRD Pipeline scenario when agents coordinate (planner -> writer -> reviewer transitions)
  - Show in Knowledge Transfer scenario during agent-to-agent data passing
  - Integration point: `src/pages/Demo.tsx` scenario sidebar panels
  - Data source: agent lifecycle events from SSE `AgentSpawned`, `TaskAssigned` events
  - **Acceptance**: PRD Pipeline scenario shows at least one handoff between planner and writer agents. Handoff status transitions through pending -> active -> done as task progresses.

- [ ] **T7.54: Wire `GateVerdictTicker` into Demo sidebar**
  - Replace plain `GateBar` with expressive `GateVerdictTicker` in scenario sidebars where gate results are displayed
  - Integration point: `src/pages/Demo.tsx` sidebar panels, `src/components/PrdPipelinePanel.tsx` gate section
  - Data source: gate results from SSE `GateResult` events, mapped to `GateVerdictItem[]`
  - Mapping: `GateResult.gate_name` -> `gate`, `GateResult.passed` -> `passed`, `GateResult.duration_ms` -> `durationMs`, `GateResult.task_id` -> `taskId`
  - **Acceptance**: Gate results appear as ticker chips instead of basic pass/fail dots. Current task's gates are highlighted, previous tasks' gates dimmed.

- [ ] **T7.55: Wire `InferenceTag` into terminal output**
  - Parse inference annotations from agent output lines (look for `[T0|T1|T2]` tier markers or `model=` prefixes in structured log output)
  - Overlay `InferenceTag` pills above or beside relevant terminal output lines
  - Integration point: `src/hooks/useTerminal.ts` output handler, `src/components/AgentOutputStream.tsx`
  - Token pattern: `/\[(T[012])\]\s*(\w+)\s+(\d+)tok/` or structured JSON events
  - **Acceptance**: When agent output includes inference metadata, an `InferenceTag` pill appears inline showing tier, model, tokens, cost.

- [ ] **T7.56: Wire `ArtifactTray` into AppShell**
  - Add persistent artifact counter in TopNav or app chrome
  - Collect artifact counts from SSE events: `Episode` -> episodes count, `InferenceCompleted` -> increment insights when significant, `HdcFingerprint` -> hdc count
  - Integration point: `src/components/AppShell.tsx` (TopNav area), `src/components/TopNav.tsx`
  - State source: DataHub or local accumulator state, reset per session
  - **Acceptance**: Tray visible in nav bar. Counts increment with spring animation when SSE events arrive. Clicking opens detail drawer.

- [ ] **T7.57: Wire `CrystallizeTransition` into gate verdicts**
  - Trigger crystallization effect when all gates in a task pass (batch gate success)
  - Trigger when C-factor exceeds 0.8 threshold on task summary
  - Integration point: `src/components/GateVerdictTicker.tsx` (all-pass detection), `src/pages/Demo.tsx` task completion handler
  - Trigger logic: check `verdicts.filter(v => v.taskId === currentTaskId).every(v => v.passed)` -> fire crystallize
  - **Acceptance**: When all gates pass for a task, a sparkle/shimmer/ring celebration plays over the gate verdict area. Effect is one-shot, debounced to 3s.

- [ ] **T7.58: Wire `ConfidenceMeter` into sidebar panels**
  - Show cascade router confidence in real-time during scenario execution
  - Integration point: `src/components/PrdPipelinePanel.tsx` stats section, `src/pages/Demo.tsx` sidebar
  - Data source: SSE `InferenceCompleted` events carry `router_confidence` field, or poll `/api/learn/router` endpoint
  - Display: confidence bar with trend arrow (compare current vs previous value), decision count from accumulated inference events
  - **Acceptance**: Confidence meter visible in sidebar during active scenarios. Bar width animates on confidence updates. Trend arrow shows direction.

- [ ] **T7.59: Wire `ModelSlot` into scenario stats**
  - Show model tier/name with slot machine animation when cascade router changes model selection
  - Integration point: `src/pages/Demo.tsx` scenario stats strip, `src/components/PrdPipelinePanel.tsx` model display
  - Data source: SSE `InferenceCompleted` events carry `model` and `tier` fields
  - Trigger: detect model change by comparing current event's model to previous event's model; on change, set `animate=true`
  - **Acceptance**: Model name visible in stats area. When model changes mid-scenario, characters roll through slot-machine animation settling left-to-right.

- [ ] **T7.60: Wire `TraceAnnotation` into log/event feeds**
  - Annotate log entries and event feed items with agent namespace, model tier, confidence level, and cost
  - Integration point: `src/components/AgentOutputStream.tsx`, `src/components/feeds/EventStream.tsx`, `src/components/agent/AgentFeed.tsx`
  - Data mapping: each event/log line that has inference metadata gets a `TraceAnnotation` strip appended
  - Props: extract `agentName` from event source, `tier`/`model` from inference metadata, `confidence` from router state, `cost` from event
  - **Acceptance**: Event feed entries that involve LLM inference show a compact annotation strip with agent dot, tier badge, model name, and cost.

---

## Dependency Graph

```
Phase 0 (cleanup) ─────────────────────────────→ independent, do first
  │
  ▼
Phase 1 (DataHub + transport + utilities) ──────→ foundation for everything
  │
  ├──▶ Phase 2 (design components) ─────────────→ depends on Phase 1 for types + utilities
  │     │
  │     ├──▶ Phase 7 (expressive primitives) ──→ depends on Phase 2; parallel with Phase 3-5
  │     │
  │     ▼
  │   Phase 3 (scenes) ─────────────────────────→ depends on Phase 1 + Phase 2
  │     │
  ├──▶ Phase 4 (chrome) ───────────────────────→ can parallel with Phase 3
  │     │
  │     ▼
  └──▶ Phase 5 (polish) ───────────────────────→ after Phase 3 + 4
        │
        ▼
      Phase 6 (advanced) ──────────────────────→ after Phase 5
```

**Parallelizable within phases:**
- Phase 0: All tasks independent (can run all in parallel)
- Phase 1: T1.1-T1.4 (utilities) parallel. T1.5-T1.8 (transport) parallel. T1.9-T1.11 serial.
- Phase 2: T2.1-T2.6 (CSS fixes) parallel. T2.7-T2.19 (components) mostly parallel after CSS.
- Phase 3: T3.7-T3.15 (Observe, Evaluate, Build, Knowledge) parallel after T3.1-T3.6.
- Phase 4: T4.1-T4.5 all parallel with Phase 3.
- Phase 7: T7.1-T7.2 (layout fixes) first. T7.3-T7.52 (all component groups) mostly parallel after fixes. T7.53-T7.60 (integration wiring) depend on the component they wire + Phase 1 DataHub for SSE event plumbing. Entire phase runs parallel with Phase 3-5.

**Critical path:**
```
T0.1-T0.11 → T1.1-T1.4 → T1.5-T1.8 → T1.9-T1.11 → T2.7-T2.9 → T3.1-T3.6 → T4.1-T4.2 → T5.4-T5.6
```

---

## Task Count Summary

| Phase | Tasks | Done | Pending | Focus |
|-------|-------|------|---------|-------|
| 0 — Cleanup | 11 | 11 | 0 | Delete 813L dead code, fix 4 bugs, 2 memory leaks |
| 1 — Foundation | 12 | 4 | 8 | Transport, DataHub, utilities, thin hooks |
| 2 — Design System | 19 | 19 | 0 | CSS fixes, Cell system, layout, design, motion |
| 3 — Scenes | 15 | 0 | 15 | All 5 pages rebuilt from monolithic files |
| 4 — Chrome | 5 | 0 | 5 | AppShell, TopNav, routing, shortcuts |
| 5 — Polish | 10 | 0 | 10 | A11y, performance, error handling, style cleanup |
| 6 — Advanced | 3 | 0 | 3 | Blockchain, learning viz, agent-to-agent |
| 7 — Expressive Primitives | 60 | 48 | 12 | Layout fixes, ASCII/terminal aesthetics, layout density, resizable panes, loading transitions, progress, WebGL ambient, agent components, event feeds, overlays, advanced layout, Three.js, inference/cybernetic, multi-agent coordination, terminal improvements, gate verification, integration wiring |
| **Total** | **135** | **82** | **53** |

---

## Verification Checklist

After all phases:

1. [ ] `npx tsc --noEmit` — zero TypeScript errors
2. [ ] `npx vite build` — builds successfully, main bundle < 200kB gzipped
3. [ ] Start `roko serve`, open :5173 — all scenes load
4. [ ] Run Orchestrate scenario — complete pipeline with animations, no errors
5. [ ] Navigate all 5 scenes — transitions work, data loads, no blank screens
6. [ ] Observe → Fleet — agents visible, topology graph renders
7. [ ] Evaluate → run a bench — SSE-driven progress, results in list
8. [ ] Build → type a prompt — response streams, terminal works
9. [ ] Knowledge → browse entries — graph renders, search works
10. [ ] Browser console — zero errors during full walkthrough
11. [ ] `prefers-reduced-motion` — all motion animations disabled
12. [ ] Kill server — app shows offline state, reconnects when server returns
13. [ ] Accessibility: tab through all interactive elements — focus ring visible everywhere
14. [ ] `grep -rn 'function hexToRgba' src/` — only in `lib/color.ts`
15. [ ] `grep -rn '#2dd4bf\|#4ade80\|#fb7185' src/ --include='*.tsx'` — zero results
16. [ ] No files > 500 lines (split monoliths all resolved)
