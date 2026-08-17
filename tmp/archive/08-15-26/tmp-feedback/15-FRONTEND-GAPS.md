# Frontend & Demo App Gaps

## 1. Current Frontend Architecture

### 1.1 Technology Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Framework | React | 19.1 |
| Bundler | Vite | 6.3 |
| State management | Zustand | 5.0 |
| Routing | react-router | 7.6 |
| Animation | motion | 12.38 |
| 3D | three.js | 0.184 |
| Terminal | @xterm/xterm | 5.5 (with WebGL, Fit, Unicode11, Clipboard, Image, WebLinks addons) |
| Testing | Playwright (e2e) | 1.59 |
| Language | TypeScript | 5.8 (strict mode, noUnusedLocals, noUnusedParameters) |

No unit test runner (vitest/jest) is configured. The only test infrastructure is Playwright e2e tests.

### 1.2 File Structure (key files)

```
demo/demo-app/
  src/
    main.tsx                          # Entry point: React 19 createRoot, BrowserRouter, providers, lazy routes
    app/
      bootstrap.ts                    # Pre-React transport init: SSE + WS + health poll -> DataHub
      DataHub.ts                      # Zustand store: central state (config, agents, episodes, bench, workspace)
      index.ts                        # Re-exports
    transport/
      api.ts                          # RokoApi class: singleton REST client with health probe + TTL cache
      sse.ts                          # SseAdapter class: EventSource with reconnect, Last-Event-ID replay
      ws.ts                           # WsAdapter class: WebSocket with reconnect, ping, send queue
      types.ts                        # ServerEvent union (70+ variants), parseServerEvent(), snakeToCamelObj()
      index.ts                        # Re-exports
    data/
      selectors.ts                    # Thin Zustand selectors: useServerConnected(), useConfigSlice(), etc.
      types.ts                        # Canonical domain types: WorkspaceInfo, AgentInfo, EpisodeInfo, etc.
      index.ts                        # Re-exports
    contexts/
      EventStreamContext.tsx          # React context wrapping createEventStreamManager (legacy SSE path)
    hooks/
      useLiveApi.ts                   # @deprecated: module-level 5s health poll + fetch wrappers
      useRokoConfig.ts                # @deprecated: 15s config poll + RokoConfigProvider context
      useBlockStream.ts               # Mirage-rs WS block subscription with preflight + fallback polling
      useEventStream.ts               # createEventStreamManager: multi-source SSE (events + workflow/events)
      useBench.ts                     # 541 lines: full bench lifecycle (suites, models, runs, SSE, polling)
      useBenchRuns.ts                 # 515 lines: DUPLICATE of useBench.ts with minor API differences
      useBenchSSE.ts                  # Bench-specific SSE: EventSource to /api/bench/events with filtering
      useServerHealth.ts              # SSE-push + one-shot fetch fallback for server health
      useServerStatus.ts              # Thin DataHub selector for serverStatus
      useApi.ts                       # @deprecated: simple fetch wrappers (replaced by transport/api.ts)
      useTerminal.ts                  # xterm.js + WS PTY: execCmd, typeCmd, waitForPrompt (OSC 7777 sideband)
      useWorkspace.ts                 # @deprecated: WorkspaceProvider context (replaced by DataHub)
      useConfig.ts                    # DataHub selector for config
      useEpisodes.ts                  # DataHub selector for episodes
      index.ts                        # Re-exports
    lib/
      serve-url.ts                    # URL resolution (Vite env, local dev, Docker/Railway), TIMEOUTS, RECONNECT_BACKOFF
      config-models.ts                # Config model resolution helpers
      rosedust-theme.ts               # xterm.js theme colors
      bench-types.ts                  # Bench domain types
      perf-markers.ts                 # Performance instrumentation
      selector-utils.ts               # Selector utilities
      prd-pipeline-types.ts           # PRD pipeline state types (still referenced)
    components/
      ErrorBoundary.tsx               # Global error boundary (class component, animated retry UI)
      AppShell.tsx                    # Layout shell with nav, Outlet
      design/
        ConnectionGuard.tsx           # Connected/connecting/error state guard with braille spinner
        ErrorState.tsx                # Error display component
        EmptyState.tsx                # Empty data display
        Skeleton.tsx                  # Loading skeleton
        LoadingTransition.tsx         # Loading state wrapper
        LazyPane.tsx                  # Lazy-loaded pane
      layout/
        PageShell.tsx, Stack.tsx, Tabs.tsx, SplitView.tsx, ScrollArea.tsx, etc.
      Charts/
        BarChart.tsx, CostChart.tsx, HeatmapChart.tsx, etc.
      agent/
        AgentContainer.tsx, AgentFeed.tsx, AgentHeartbeat.tsx, etc.
      SidebarRenderer.tsx             # Demo sidebar with legacy alias routing
    pages/
      Landing.tsx, Terminal.tsx, Builder.tsx, Explorer/, Bench.tsx,
      BenchRunDetail.tsx, BenchCompare.tsx, Settings.tsx, Share.tsx
      dashboard/
        Layout.tsx, CostDashboard.tsx, AgentFleet.tsx, KnowledgeGraph.tsx,
        IntegrityView.tsx, CascadeRouter.tsx, KnowledgeEntries.tsx, DreamsView.tsx
      Demo/
        ScenarioSlot.tsx, index.tsx, BottomTerminalPane.tsx, etc.
  e2e/
    16 test files: landing, navigation, terminal, bench-*, config-*, dashboard*, explorer, builder, settings, demo-*
```

### 1.3 Component Hierarchy

```
<StrictMode>
  <BrowserRouter>
    <ErrorBoundary>                    # Class component, catches render errors
      <WorkspaceProvider>              # @deprecated context (fetches /api/workspaces/default)
        <RokoConfigProvider>           # @deprecated context (15s config poll)
          <EventStreamProvider>        # SSE context: creates EventStreamManager singleton
            <ToastProvider>
              <Suspense fallback={<RouteLoading />}>
                <Routes>
                  <Route element={<AppShell />}>
                    <Route index element={<Landing />} />
                    <Route path="dashboard" element={<DashboardLayout />}>
                      7 nested dashboard routes
                    </Route>
                    <Route path="terminal" .../>
                    <Route path="builder" .../>
                    <Route path="explorer" .../>
                    <Route path="bench" .../>
                    <Route path="bench/run/:id" .../>
                    <Route path="bench/compare" .../>
                    <Route path="settings" .../>
                    <Route path="share/:token" .../>
                  </Route>
                </Routes>
              </Suspense>
            </ToastProvider>
          </EventStreamProvider>
        </RokoConfigProvider>
      </WorkspaceProvider>
    </ErrorBoundary>
  </BrowserRouter>
</StrictMode>
```

### 1.4 Data Flow

Two parallel data systems coexist. Both are active at runtime:

**System A (Legacy): React Contexts + polling**
```
main.tsx
  -> WorkspaceProvider: fetch /api/workspaces/default on mount
  -> RokoConfigProvider:
       -> useLiveApi(): module-level 5s health poll (setInterval)
       -> useRokoConfigState(): 15s config poll (setInterval)
  -> EventStreamProvider:
       -> createEventStreamManager():
            -> EventSource /api/events (unnamed message channel)
            -> EventSource /api/workflow/events (named event listeners)
            -> 3s reconnect on error
            -> Dispatches to typed subscriber sets
```

**System B (Current): bootstrapTransport() + Zustand DataHub**
```
bootstrapTransport() (called ONCE before createRoot)
  -> api.probe() initial health check
  -> setInterval(api.probe, 30_000) health poll
  -> SseAdapter: connects to /api/events, routes parsed events to DataHub.handleServerEvent()
  -> WsAdapter: connects to /api/workflow/ws (frames not routed to DataHub)
  -> DataHub.fetchConfig() initial REST fetch
  -> DataHub.fetchServerWorkdir() initial REST fetch

DataHub (Zustand store) handles:
  - plan_started/plan_completed/phase_transition -> activePlanId, activePhase, planCompleted
  - agent_spawned/agent_stopped -> agents array
  - episode -> episodes ring buffer (max 500)
  - inference_completed -> totalCost, totalTokens, recentInferences ring buffer (max 200)
  - config_reloaded -> triggers fetchConfig()
  - BenchRunCompleted -> triggers fetchBenchRuns()
  - server_shutdown -> serverStatus = 'disconnected'

Selectors (data/selectors.ts) provide thin hooks:
  - useServerConnected(), useTransportStatus()
  - useConfigSlice(), useDefaultModel()
  - useWorkspaceSlice()
  - usePlanSlice()
  - useCostSlice(), useBenchSlice()
```

**SSE connection from server side:**
```
roko-serve/src/routes/sse.rs:
  GET /api/events and GET /api/sse
  - Replays up to 256 events from Last-Event-ID
  - Live stream from broadcast::Sender<DashboardEvent>
  - 8s keepalive interval (SSE comment "keepalive")
  - X-Accel-Buffering: no header for proxy compatibility
```

### 1.5 State Management Architecture

The app is mid-migration from Context + polling to Zustand + push. Both systems run simultaneously:

| Concern | Legacy (active) | Current (active) |
|---------|----------------|-----------------|
| Health | `useLiveApi` 5s poll | `bootstrapTransport` 30s poll |
| Config | `useRokoConfig` 15s poll context | `DataHub.fetchConfig()` on mount + SSE `config_reloaded` |
| Workspace | `WorkspaceProvider` context | `DataHub.fetchServerWorkdir()` on mount |
| SSE events | `EventStreamContext` (2 EventSources) | `SseAdapter` (1 EventSource) |
| WS events | `useEventStream` named listeners | `WsAdapter` (frames not routed) |

This means the client opens **3 EventSource connections** (2 from EventStreamContext + 1 from SseAdapter) and runs **2 health polls** (5s + 30s) simultaneously.

---

## 2. Issues Found

### 2.1 Dual SSE Connection Waste

**File:** `demo/demo-app/src/app/bootstrap.ts:42-49` and `demo/demo-app/src/contexts/EventStreamContext.tsx:26-37`

**What's wrong:** Two independent SSE systems connect on every page load. `bootstrapTransport()` creates an `SseAdapter` to `/api/events`. Then `EventStreamProvider` creates an `EventStreamManager` that opens TWO `EventSource` connections to `/api/events` AND `/api/workflow/events`. Total: 3 concurrent SSE connections to the same server.

**Current code (bootstrap.ts:42-49):**
```typescript
const sse = new SseAdapter({
  url: `${SERVE_URL}/api/events`,
  onEvent: (raw) => {
    const event = parseServerEvent(raw);
    if (event) hub().handleServerEvent(event);
  },
  onStatusChange: (status) => set({ sseStatus: status }),
});
sse.connect();
```

**Current code (EventStreamContext.tsx:26-37):**
```typescript
useEffect(() => {
  const mgr = createEventStreamManager(SERVE_URL); // opens /api/events AND /api/workflow/events
  managerRef.current = mgr;
  mgr.onConnectedChange = (c: boolean) => setConnected(c);
  return () => { mgr.destroy(); managerRef.current = null; };
}, []);
```

**Fix design:** Remove the `EventStreamContext` provider from `main.tsx` and migrate all `useContextEventSubscription()` / `useEventStreamContext()` callers to subscribe via DataHub. The `bootstrapTransport` SseAdapter already routes all events to `DataHub.handleServerEvent()`. Components that need raw event subscription should use a thin Zustand middleware that exposes `subscribe(types, handler)` backed by the DataHub's event dispatch.

**How to test:** After removal, verify with browser DevTools Network tab that only 1 SSE connection to `/api/events` exists. Run `npx playwright test` to confirm e2e tests pass.

### 2.2 Dual Health Polling

**File:** `demo/demo-app/src/hooks/useLiveApi.ts:66-68` and `demo/demo-app/src/app/bootstrap.ts:33-39`

**What's wrong:** Two independent health polling loops run simultaneously. `useLiveApi` polls every 5 seconds at module scope. `bootstrapTransport` polls every 30 seconds. Combined: the server gets a health check every ~4.3 seconds (they are not synchronized).

**Current code (useLiveApi.ts:66-68):**
```typescript
const interval = setInterval(() => {
  void probeServer();
}, 5_000);
```

**Current code (bootstrap.ts:33-39):**
```typescript
const healthInterval = setInterval(() => {
  api.probe(true).then((snap) => {
    set({ serverStatus: snap.reachable ? 'connected' : 'disconnected' });
  });
}, 30_000);
```

**Fix design:** Remove the `useLiveApi` module-level polling entirely. All callers of `useLiveApi().isLive` should switch to `useServerConnected()` from `data/selectors.ts`. All callers of `useLiveApi().get/post/put` should switch to the `api` singleton from `transport/api.ts`. The `@deprecated` annotations are already in place; this is pure deletion.

**How to test:** Search for `useLiveApi` imports. Replace with `useServerConnected()` + `api`. Verify only one `/health` poll visible in Network tab (30s interval). Run e2e tests.

### 2.3 Deprecated Config Polling (15s interval)

**File:** `demo/demo-app/src/hooks/useRokoConfig.ts:126-129`

**What's wrong:** The `RokoConfigProvider` context runs a 15-second `setInterval` polling `/api/config`, independent of the DataHub's SSE-driven config refresh. This is marked `@deprecated` but still mounted in `main.tsx` line 89.

**Current code (useRokoConfig.ts:126-129):**
```typescript
useEffect(() => {
  fetchConfig();
  intervalRef.current = setInterval(fetchConfig, 15_000);
  return () => clearInterval(intervalRef.current);
}, [fetchConfig]);
```

**Fix design:** Remove `<RokoConfigProvider>` from `main.tsx`. Migrate all `useRokoConfig()` callers to `useConfigSlice()` or `useDefaultModel()` / `useDefaultBackend()` from `data/selectors.ts`. The DataHub already handles initial fetch and SSE-triggered refetch via `config_reloaded` events.

**How to test:** `grep -rn 'useRokoConfig' demo/demo-app/src/` to find all callers. Replace each. Remove the provider from main.tsx. Run `tsc -b` and e2e tests.

### 2.4 useBench.ts / useBenchRuns.ts Full Duplication

**File:** `demo/demo-app/src/hooks/useBench.ts` (541 lines) and `demo/demo-app/src/hooks/useBenchRuns.ts` (515 lines)

**What's wrong:** These two files are near-identical implementations of the full bench run lifecycle. Both define the same types (`BenchConfig`, `ActiveRun`, `FeedItem`, `ConnectionState`), same default config, same SSE event processing logic, same `startRun`/`cancelRun`/`exportRun`/`importRun` callbacks, and same ETA computation. The only differences are:

1. `useBench.ts` posts to `/api/bench/run` with `overrides`; `useBenchRuns.ts` posts to `/api/bench/runs` with `config`
2. `useBench.ts` includes `pareto` state and `fetchPareto()`; `useBenchRuns.ts` does not
3. `useBench.ts` wraps API responses with `{runs: []}` / `{suites: []}` normalization; `useBenchRuns.ts` expects flat arrays
4. `useBenchRuns.ts` destructures `events: _sseEvents` while `useBench.ts` keeps them

Both import from `useLiveApi` (deprecated). Only `useBench.ts` is imported by `Bench.tsx`. `useBenchRuns.ts` appears to be an extraction attempt that was never wired.

**Fix design:** Delete `useBenchRuns.ts`. If the extraction was intended to separate concerns, do it properly: extract the SSE event processing into a `useBenchEvents(benchId)` hook that returns the feed, activeRun updates, and visualization state. Keep the REST lifecycle (start/cancel/export/import) in `useBench.ts`. But since `useBenchRuns.ts` has zero imports, deleting it is safe and immediate.

**How to test:** `grep -rn 'useBenchRuns' demo/demo-app/src/` -- if no imports reference it (only the file itself), delete it. Run `tsc -b`.

### 2.5 Legacy Scenario ID References

**File:** `demo/demo-app/src/pages/Demo/ScenarioSlot.tsx:80,994` and `demo/demo-app/src/components/SidebarRenderer.tsx:150,205`

**What's wrong:** ScenarioSlot.tsx and SidebarRenderer.tsx reference legacy scenario IDs `prd-pipeline` and `knowledge-transfer` that were renamed to `pipeline` and `memory` in Task 021's scenario collapse. The code handles both old and new names with `||` fallbacks, but the old names should be removed. The old scenario runners are already archived in `scenario-runners/archive/`.

**ScenarioSlot.tsx:80:**
```typescript
return scenarioId === 'prd-pipeline' ? createPipelineIntroState(example) : EMPTY_PIPELINE_STATE;
```

**SidebarRenderer.tsx:150,205:**
```typescript
if (scenarioId === 'pipeline' || scenarioId === 'prd-pipeline') {
// ...
if (scenarioId === 'memory' || scenarioId === 'knowledge-transfer') {
```

**Fix design:** Remove the `|| scenarioId === 'prd-pipeline'` and `|| scenarioId === 'knowledge-transfer'` fallbacks. Change ScenarioSlot.tsx line 80 to check for `'pipeline'` instead of `'prd-pipeline'`. Change line 994's CSS check from `scenario.id === 'prd-pipeline'` to `scenario.id === 'pipeline'`.

**How to test:** Run the demo app, navigate to each scenario, confirm sidebar and main pane render correctly. Run `npx playwright test e2e/demo-*.spec.ts`.

### 2.6 Missing Error Handling in useApi.ts

**File:** `demo/demo-app/src/hooks/useApi.ts:12-15`

**What's wrong:** The deprecated `useApi` hook throws on non-200 responses with a bare `Error(status statusText)` message. There is no body parsing, no AbortSignal support, and no timeout. The replacement `transport/api.ts` handles all of these (never throws, returns `ApiResult<T>`, supports signal). But `useApi` is still transitively imported by `useLiveApi` which is imported by `useBench`, `useBenchRuns`, `useRokoConfig`.

**Current code:**
```typescript
const get = useCallback(async <T = unknown>(path: string): Promise<T> => {
  const res = await fetch(`${SERVE_URL}${path}`);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json() as Promise<T>;
}, []);
```

**Fix design:** Part of the useLiveApi deprecation. Once useBench and useRokoConfig migrate to the `api` singleton, `useApi.ts` has zero callers and can be deleted.

### 2.7 useBenchSSE Missing Reconnect Backoff

**File:** `demo/demo-app/src/hooks/useBenchSSE.ts:71`

**What's wrong:** On SSE error, the bench SSE hook reconnects after a flat 3 seconds with no backoff. If the server is down for an extended period, this hammers it with reconnection attempts every 3 seconds indefinitely. The transport-layer `SseAdapter` has proper exponential backoff (`baseMs * 2^(retryCount-1)` capped at `maxMs`), but `useBenchSSE` does not use the adapter -- it creates raw `EventSource` instances.

**Current code (useBenchSSE.ts:70-71):**
```typescript
clearTimeout(reconnectTimer);
reconnectTimer = setTimeout(connect, 3_000);
```

**Fix design:** Either (a) switch `useBenchSSE` to use the `SseAdapter` class from `transport/sse.ts`, or (b) add a retry counter and exponential backoff inline. Option (a) is preferred since it reuses the existing adapter with `maxRetries` support. The adapter would be constructed with `url: ${SERVE_URL}/api/bench/events?bench_id=${benchId}` and events dispatched via the `onEvent` callback.

**How to test:** Disconnect the server, observe reconnect timing in DevTools. Should see 1s, 2s, 4s, 8s, 15s backoff (not flat 3s). Should stop after `maxRetries` (5 by default).

### 2.8 Deprecated WorkspaceProvider Still Mounted

**File:** `demo/demo-app/src/main.tsx:88,121` and `demo/demo-app/src/hooks/useWorkspace.ts`

**What's wrong:** `WorkspaceProvider` is marked `@deprecated` but is still in the provider tree in main.tsx (lines 88 and 121). It fetches `/api/workspaces/default` on mount, duplicating the `fetchServerWorkdir()` call in `bootstrapTransport()`. The DataHub workspace slice provides identical functionality.

**Fix design:** Remove `<WorkspaceProvider>` from main.tsx. Migrate `useWorkspace()` callers to `useWorkspaceSlice()` or `useWorkspaceActions()`. Note: `useWorkspace` throws if not inside the provider, so callers MUST be migrated before removal.

**How to test:** `grep -rn 'useWorkspace\b' demo/demo-app/src/ --include='*.ts' --include='*.tsx' | grep -v 'useWorkspaceActions\|useWorkspaceSlice\|useWorkspaceInfo\|node_modules'` to find callers. Replace, remove provider, run `tsc -b`.

### 2.9 EventStreamManager Creates Empty Connect Listeners

**File:** `demo/demo-app/src/hooks/useEventStream.ts:200`

**What's wrong:** Every `subscribe()` call registers an empty `wrappedConnect` function:
```typescript
const wrappedConnect = () => {};
connectListeners.add(wrappedConnect);
```
These empty functions accumulate in the `connectListeners` set and are called on every connect/disconnect cycle, doing nothing. This is a minor memory/performance issue but indicates dead code from an incomplete refactor.

**Fix design:** Remove the `wrappedConnect` creation and `connectListeners.add(wrappedConnect)` call, and the corresponding `connectListeners.delete(wrappedConnect)` in the unsubscribe closure. If connection-state-change notification is needed per-subscriber, add it as an optional callback parameter to `subscribe()`.

### 2.10 Three EventSource Connections

**File:** `demo/demo-app/src/hooks/useEventStream.ts:170-173` and `demo/demo-app/src/app/bootstrap.ts:42-50`

**What's wrong:** `createEventStreamManager()` opens two EventSource connections:
```typescript
function connect() {
  closeSources();
  openSource('/api/events', false);           // EventSource #1
  openSource('/api/workflow/events', true);    // EventSource #2
}
```
And `bootstrapTransport()` opens a third:
```typescript
const sse = new SseAdapter({
  url: `${SERVE_URL}/api/events`,             // EventSource #3
  ...
});
```

The server endpoint `/api/workflow/events` is only needed for named SSE events (event types sent as SSE `event:` field rather than just `data:`). The server's `/api/events` endpoint sends all events as unnamed messages.

**Fix design:** Consolidate into a single SSE connection. The `SseAdapter` from `transport/sse.ts` already registers listeners for all `KNOWN_SSE_EVENT_TYPES` via `addEventListener()`, handling both unnamed `onmessage` and named event types. Remove the EventStreamContext's dual-source approach.

---

## 3. Frontend Architecture Recommendations

### 3.1 Complete the Zustand Migration

The DataHub store is well-designed but the migration is half-done. Three deprecated contexts (`WorkspaceProvider`, `RokoConfigProvider`, `EventStreamProvider`) still wrap the app. Each creates duplicate connections or polls.

**Migration order:**
1. **Remove `RokoConfigProvider`** (most callers already migrated to `useConfigSlice()`)
2. **Remove `WorkspaceProvider`** (callers migrate to `useWorkspaceSlice()`)
3. **Remove `EventStreamProvider`** (requires adding event subscription to DataHub)
4. **Delete deprecated hooks**: `useApi.ts`, `useLiveApi.ts`, `useRokoConfig.ts`, `useWorkspace.ts`

### 3.2 Event Subscription Pattern

The DataHub currently handles events internally (`handleServerEvent`) but does not expose subscription for components that need raw events (e.g., gate verdicts displayed in real-time). Add a lightweight subscription mechanism:

```typescript
// In DataHub.ts
private subscribers: Map<string, Set<(event: ServerEvent) => void>> = new Map();

subscribeEvent(types: string[], handler: (event: ServerEvent) => void): () => void { ... }
```

This replaces `EventStreamContext.subscribe()` and the `createEventStreamManager` machinery.

### 3.3 Error Boundary Strategy

The current error boundary (`ErrorBoundary.tsx`) is a single global catch at the root. It provides no per-page recovery -- any component error blanks the entire app. Recommendations:

- Add per-route error boundaries in `AppShell` or `DashboardLayout`
- Use the existing `design/ErrorState.tsx` component for page-level errors
- The `design/ConnectionGuard.tsx` component handles disconnection state well; wrap data-dependent pages with it
- Consider react-router's `errorElement` for per-route error UI (react-router v7 supports this)

### 3.4 SSE/WS Connection Management

**Target architecture:** One SSE connection + one WS connection, both managed by `bootstrapTransport()`, both feeding the Zustand DataHub.

```
bootstrapTransport() [called once before React render]
  |
  |- SseAdapter -> /api/events
  |    |- onEvent -> DataHub.handleServerEvent()
  |    |- Reconnect: exponential backoff, Last-Event-ID replay
  |
  |- WsAdapter -> /api/workflow/ws
  |    |- onFrame -> DataHub.handleWsFrame() [add this]
  |    |- Reconnect: exponential backoff, subscription replay
  |    |- Auto-ping every 30s
  |
  |- Health poll: api.probe() every 30s
  |
  |- Initial fetches: config, workdir

Result: 1 SSE + 1 WS + 1 health poll (currently: 3 SSE + 1 WS + 2 health polls)
```

### 3.5 Component Patterns

**Data fetching:** Use the DataHub store for all shared data. Page-specific data that does not need cross-page sharing can use local `useEffect` + `api.get()`.

**Loading states:** The `design/Skeleton.tsx`, `design/LoadingTransition.tsx`, and `design/LazyPane.tsx` components exist but are not consistently used. Every page that fetches data on mount should show a skeleton or loading state.

**Lists with virtualization:** `layout/VirtualList.tsx` exists for large lists. Use it for episode lists, agent output streams, and log views.

### 3.6 Testing Strategy

**Current state:** 16 Playwright e2e tests, 0 unit tests, 0 integration tests.

**Recommendations:**
1. **Add Vitest** for unit tests: hooks, selectors, transport/types.ts `parseServerEvent()`, serve-url.ts resolution logic
2. **Test the DataHub store** directly: call `handleServerEvent()` with mock events, assert state changes
3. **Test SseAdapter/WsAdapter**: mock EventSource/WebSocket, verify reconnect behavior
4. **Keep Playwright for e2e**: test full user flows, SSE connection, terminal interactions
5. **Add MSW (Mock Service Worker)** for component tests that need server responses without a running backend

---

## 4. TUI Architecture Analysis

### 4.1 Architecture Overview

The TUI lives in `crates/roko-cli/src/tui/` and is backed by ratatui (crossterm backend).

| Layer | File | Responsibility |
|-------|------|---------------|
| App shell | `app.rs` | Main loop: event poll -> dispatch action -> render frame |
| State | `state.rs` | ~1200-line `TuiState` struct: agents, plans, tasks, metrics, navigation |
| Tabs | `tabs.rs` | `Tab` enum: F1-F10 mapping, next/prev cycling |
| Views | `views/` | Per-tab render functions: dashboard, plans, agents, git, logs, config, inspect, marketplace, atelier, learning |
| Widgets | `widgets/` | Reusable render components: plan_tree, task_progress, token_sparkline, braille, wave_progress, etc. |
| Modals | `modals/` | Overlays: help, plan_detail, task_detail, approval, inject, quit, queue_overview, etc. |
| Input | `input.rs` | `TuiAction` enum dispatch, keybind routing, `FocusZone` |
| Events | `event.rs` | crossterm poll: Key, Mouse, Resize, Tick |
| PostFX | `postfx.rs`, `postfx_pipeline.rs` | Screen-space post-processing effects |
| Atmosphere | `atmosphere.rs` | Ambient animation state |
| Theme | `theme.rs` | Color palette |

### 4.2 TUI State Management

`TuiState` in `state.rs` is a monolithic state struct containing:

- **Navigation:** `active_tab: Tab`, `sub_tab: usize`, `focus_zone: FocusZone`, `input_mode: InputMode`
- **Agents:** `agents: Vec<AgentRow>`, `agent_streams: HashMap<String, AgentStream>`, `agent_route_metrics: HashMap<String, RouteMetrics>`, `process_metrics: HashMap<u32, ProcessMetrics>`
- **Plans:** `plans: Vec<PlanEntry>`, `plan_execution: Option<PlanExecutionSnapshot>`
- **Tasks:** `tasks: Vec<TaskRowStatus>`, task output cursors
- **Modals:** `modal: ModalState`, approval state
- **Metrics:** `sys_metrics: SysMetrics`, `total_cost`, `total_tokens`, gate verdicts
- **Scroll:** per-tab scroll offsets, scroll acceleration
- **Git:** branch, commit, worktree state
- **Knowledge:** knowledge entries, dream journal

This is a single struct approach (not ECS or slice-based). All mutations go through methods on `TuiState` or direct field assignment in `app.rs`.

### 4.3 TUI Data Sources

The TUI has several independent data sources, all feeding into `TuiState`:

| Source | Mechanism | Refresh Cadence |
|--------|-----------|----------------|
| `.roko/` filesystem | `notify::RecommendedWatcher` (debounced 200ms) with poll fallback (1s) | On filesystem change |
| Git metadata | `git_watch.rs` watcher | On git events |
| System metrics (CPU/MEM) | Background thread via `watch::Receiver<SysSnapshot>` | Continuous |
| Agent topology | One-shot HTTP fetch via `std_mpsc::Receiver` | On demand |
| Agent streams | `ws_client.rs` WebSocket per agent | Live streaming |
| Dashboard snapshot | `watch::Receiver<DashboardSnapshot>` from StateHub | Push-based |
| Process supervisor | `ProcessSupervisor` for per-agent process sampling | Per-tick |

### 4.4 TUI Rendering Architecture

The render path is split into two modes:

1. **Mori-style tabs (primary):** `Tab` -> `views::render_tab_content()` -> per-view render function. Each view receives `&DashboardData`, `&TuiState`, `&ViewState`, `&Theme`. Sub-views are selected by number keys (1-9) within a tab.

2. **Legacy scaffold (compatibility):** `PageId` -> `DashboardScaffold` -> `PageRegistry::render()`. Exists for non-interactive text-mode consumers.

The render loop uses an adaptive frame rate:
- Active input: full tick rate
- Idle: reduced tick rate
- Background: minimal renders

### 4.5 TUI Gaps and Issues

**4.5.1 Monolithic state struct:** `TuiState` holds everything in one struct. This means any state change causes a full re-render. For most TUI applications this is acceptable, but with 10+ tabs, each with 2-3 sub-views, the state struct is large enough to cause unnecessary allocations during partial updates.

**4.5.2 Agent stream connection management:** `ws_client.rs` creates one WebSocket per agent. If 10 agents are running, that is 10 concurrent WebSocket connections from the TUI. The event bus approach (subscribe to `agent_output` events on one connection) is available but only used as an alternative path, not the default.

**4.5.3 No TUI tests for view rendering:** The TUI has tests for `tabs.rs` (roundtrip, cycle, index) and `fs_watch.rs` (poll fallback), and `ws_client.rs` (WebSocket frame parsing), but no snapshot or property-based tests for the view rendering functions. Since views are pure functions of `(DashboardData, TuiState, ViewState, Theme) -> Frame`, they are testable with `ratatui::TestBackend`.

**4.5.4 Missing sub-view content for several tabs:** The sub-view system (`SubView` enum) declares 29 sub-views across 10 tabs. Some sub-views may render placeholder or empty content when the corresponding data source is not available (e.g., `MeshStatus`, `EngramDag`, `EpisodeReplay`). This is by design (progressive feature delivery) but should be documented.

**4.5.5 PostFX pipeline overhead:** `postfx.rs` and `postfx_pipeline.rs` implement screen-space effects. These modify the frame buffer after normal rendering, adding a per-frame cost. The `EffectsConfig` controls this, but the default configuration may have effects enabled that are not visible (e.g., `atmosphere.rs` ambient animations on tabs that are not displayed). Effects should be disabled for hidden tabs.

---

## 5. Summary of Required Frontend Fixes

| Priority | Fix | Effort | Section |
|----------|-----|--------|---------|
| **HIGH** | Remove EventStreamProvider (3 SSE -> 1) | 2-3 hours | 2.1, 2.10 |
| **HIGH** | Remove useLiveApi 5s health poll, migrate callers to useServerConnected() | 1 hour | 2.2 |
| **HIGH** | Remove RokoConfigProvider 15s config poll, migrate callers to useConfigSlice() | 1 hour | 2.3 |
| **HIGH** | Remove WorkspaceProvider, migrate callers to useWorkspaceSlice() | 30 min | 2.8 |
| **MEDIUM** | Delete useBenchRuns.ts (dead code duplicate) | 5 min | 2.4 |
| **MEDIUM** | Switch useBenchSSE to SseAdapter for proper reconnect backoff | 30 min | 2.7 |
| **MEDIUM** | Add event subscription mechanism to DataHub | 1 hour | 3.2 |
| **LOW** | Remove prd-pipeline / knowledge-transfer legacy aliases | 10 min | 2.5 |
| **LOW** | Delete useApi.ts after useLiveApi removal | 5 min | 2.6 |
| **LOW** | Remove empty wrappedConnect listeners | 5 min | 2.9 |
| **LOW** | Add Vitest + unit tests for DataHub, transport layer, selectors | 4-6 hours | 3.6 |
| **LOW** | Add per-route error boundaries | 1-2 hours | 3.3 |

### Estimated total: ~12-16 hours for all fixes

The HIGH priority items (removing deprecated providers and their duplicate connections) would cut the SSE connection count from 3 to 1, eliminate 2 redundant polling loops, and remove ~500 lines of deprecated code. These are the most impactful changes.
