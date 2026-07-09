# 105 — Frontend: demo/demo-app (deep second pass)

**Verification header**
- Repo HEAD: `5852c93c05` on `main`
- Date: 2026-07-08
- Scope: `/Users/will/dev/nunchi/roko/roko/demo/demo-app/` only. The copies under
  `.claude/worktrees/*/demo/demo-app/` are IGNORED per task scope.
- Method: read of entry/transport/store/context source + cross-checked every
  frontend call URL against `crates/roko-serve/src/routes/*.rs` route tables at
  this HEAD.
- Supersedes the frontend slices of `43-SURFACES-DEMO-UX`, `66-FRONTEND-API-PARITY`,
  `76-DATA-CONTRACTS` (those looked at the app only through API-parity; this goes
  into the actual React code).

Status tags: **[WIRED]** live path, **[DUP]** duplicate impl, **[404]** calls a
route that does not exist server-side, **[DEAD]** defined but never called,
**[DEPRECATED]** superseded but still mounted, **[EMPTY]** renders but backing
data is normally empty.

---

## 1. Build / stack / how it is served

**Stack** (`package.json`): Vite 6.3 + React 19.1 + TypeScript 5.8, `react-router`
7.6, `zustand` 5.0 (state), `three` 0.184 + `motion` 12 (hero/ambient viz),
`@xterm/xterm` 5.5 (terminal), `@playwright/test` (e2e). No CSS framework — hand-
rolled CSS under `src/styles/*.css` (10 sheets imported in `main.tsx`).

**Scripts**: `dev` = `vite`; `build` = `tsc -b && vite build` → `dist/`;
`preview`; `e2e` = `npx playwright test`.

**Vite config** (`vite.config.ts`): dev server proxies `"/api"`, `"/ws"`,
`"/relay"` (all `ws: true`) and `"/health"` → `http://localhost:6677`. Manual
chunks split `three` and `@xterm`. Watch ignores `.roko/`, `roko.toml`, `target/`
so backend writes during a demo don't trigger HMR reloads.

**How it reaches the browser — IT IS SERVED BY roko-serve.** This is the key
correction to earlier docs that implied it is standalone.
`crates/roko-serve/src/routes/mod.rs:249-250` installs a catch-all
`.fallback(crate::embedded::serve_embedded)`. `crates/roko-serve/src/embedded.rs`
is **disk-first, embedded-fallback**:
- `#[derive(rust_embed::Embed)] #[folder = "../../demo/demo-app/dist/"]` bakes
  `dist/` into the `roko serve` binary at compile time (`embedded.rs:17-19`).
- At runtime `serve_embedded` (`embedded.rs:111`) tries, in order: `ROKO_SPA_DIR`
  env dir → compile-time-relative `demo/demo-app/dist` (dev) → embedded bytes →
  `404`. SPA client routes fall back to `index.html`. Hashed `/assets/` get
  `max-age=31536000 immutable`; HTML is `no-cache`.

So there are **two serving modes**: (a) `npm run dev` on :5173 proxying to
:6677 (developer loop), and (b) production/Docker/Railway where `roko serve`
serves the built SPA and the API from a single origin :6677. `src/lib/serve-url.ts`
detects both: on a local dev port (≠6677) or same-origin it returns `''` (relative
paths → proxy/same-origin); env overrides `VITE_ROKO_SERVE_URL` / `VITE_ROKO_WS_BASE`.

> Caveat: the embed points at `dist/`. If `dist/` is stale or absent at build
> time, `rust-embed` bakes an old or empty SPA. The demo is only as fresh as the
> last `npm run build` before `cargo build -p roko-serve`.

---

## 2. Architecture

### Entry (`src/main.tsx`)
1. `void bootstrapTransport()` runs **before** `createRoot` (`main.tsx:85`) — the
   transport layer is initialized outside React.
2. Render tree wraps `<BrowserRouter>` → `<ErrorBoundary>` →
   `<WorkspaceProvider>` → `<RokoConfigProvider>` → `<EventStreamProvider>` →
   `<ToastProvider>` → `<Suspense>` → `<Routes>`.
   **Three of those four providers are the deprecated pre-DataHub layer** (see §6).
3. All route pages are `lazy()` code-split.

### Transport layer (the new spine) — `src/app/` + `src/transport/`
- `app/bootstrap.ts` `bootstrapTransport()`: (1) `api.probe()` health; (2) 30 s
  health poll; (3) `SseAdapter` → `${SERVE_URL}/api/events` → `parseServerEvent`
  → `useDataHub.handleServerEvent`; (4) `WsAdapter` → `${WS_BASE}/api/workflow/ws`
  (frames not routed to DataHub — consumed by `lib/workflow-api.ts`); (5) initial
  `fetchConfig()` + `fetchServerWorkdir()`. Returns a cleanup fn.
- `transport/api.ts` `RokoApi` / singleton `api`: never-throwing
  `ApiResult<T>` (`{ok,data} | {ok:false,error}`); `get/post/put/delete`; `probe()`
  hits `/health` with 30 s TTL + 2 s timeout, deduped.
- `transport/sse.ts` `SseAdapter`: `EventSource` with exponential backoff (5
  retries, 1 s→15 s), `lastEventId` replay, listens to `message` + 30 named event
  types (`KNOWN_SSE_EVENT_TYPES`).
- `transport/ws.ts` `WsAdapter`: reconnecting `WebSocket`, send-queue, 30 s ping,
  subscription re-registration; frame type `state|delta|ack|error|pong`.
- `transport/types.ts`: the `ServerEvent` discriminated union (**~85 variants**,
  documented as camelCase conversions of Rust snake_case) + `parseServerEvent()`
  which recursively snake→camel converts keys (`snakeToCamelObj`) but preserves
  the `type`/`kind` tag verbatim (Bench events are PascalCase).

### State store — `src/app/DataHub.ts` (Zustand)
Single `useDataHub` store (T1.9/T1.10). Its header comment declares it *replaces*
`EventStreamContext`, `useRokoConfig`, `useLiveApi`, `useServerHealth`,
`useApiWithFallback`, `useWorkspace`. Slices: connection status (server/sse/ws),
config, workspace(+cache Map), plan execution, agents, episodes/metrics
(+`recentInferences` ring buffer 200), bench, ISFR (rate/history/sources/field-
history/event-log), chain (blocks/txs/events/gas), feed catalog, relay dashboard.
`handleServerEvent` is a big switch over camelCase event types; REST fetch actions
call `api.*` and map snake→camel by hand. Ring-buffer caps are constants
(`MAX_EPISODES=500`, `MAX_INFERENCES=200`, etc.).

Selector layer: `src/data/selectors.ts` + `src/data/index.ts` + `lib/selector-utils.ts`
(memoized derived reads over DataHub).

### Contexts / hooks (the older layer, still present)
- `contexts/EventStreamContext.tsx` `EventStreamProvider`: builds
  `createEventStreamManager(SERVE_URL)` and exposes `subscribe(types,handler)` +
  `useContextEventSubscription`.
- `hooks/useEventStream.ts` `createEventStreamManager`: a **second** singleton
  SSE manager that opens **two** EventSources — `/api/events` and
  `/api/workflow/events` — with its own 26-entry `KNOWN_SSE_EVENT_TYPES` list.
- ~30 feature hooks: `useRokoConfig`, `useWorkspace`, `useLiveApi`,
  `useServerStatus/useServerHealth`, `useBench`/`useBenchRuns`/`useBenchSSE`,
  `useMatrixBench`, `useLearningStats`, `useChain`, `useBlockStream`,
  `useTerminal`, `useInferenceTrace`, `useAgentHandoffs`, `useOperationEvents`, …

### Component tree by feature area
`main.tsx` routes (all under `<AppShell>`):

```
/                       Landing                     (hero/three.js marketing)
/dashboard              Layout (nested outlet)
  ├ index               CostDashboard    [WIRED] learn/* + metrics  → real
  ├ fleet               AgentFleet       [WIRED] managed-agents/topology
  ├ knowledge           KnowledgeGraph   [EMPTY] neuro store usually empty
  ├ integrity           IntegrityView    [WIRED] episodes + gates/history → real
  ├ entries             KnowledgeEntries [EMPTY]
  ├ routing             CascadeRouter    [WIRED] learn/cascade-router
  ├ dreams              DreamsView       [EMPTY] dream/journal (no runtime trigger)
  ├ feeds               FeedsDashboard   [EMPTY] needs feed agents connected
  └ relay               RelayDashboard   [EMPTY] needs relay agents/workspaces
/isfr                   IsfrDashboard    [EMPTY] needs ISFR keeper running
/demo                   (element: null — handled inside AppShell/scenario slots)
/terminal               Terminal         [WIRED] xterm over /api/ws
/builder                Builder
/explorer               Explorer         [WIRED] episodes + statehub/events
/settings               Settings         [WIRED] config
/bench, /bench/run/:id, /bench/compare   Bench* [WIRED] bench/*
/share/:token, /share   SharePage        [404]  see §3
```
Supporting: `components/` ~200 files (charts, ascii, ambient/three, Spectre agent
viz, Cell/Terminal grids, GateBar/GateWaterfall, etc.). `src` is ~340 `.ts(x)`.

**DUP pages**: `pages/dashboard/IsfrDashboard.tsx` + `pages/isfr/IsfrDashboard.tsx`
and `pages/dashboard/FeedsDashboard.tsx` + `pages/feeds/FeedsDashboard.tsx` both
exist; `main.tsx` imports the `pages/isfr/` and `pages/feeds/` variants — the
`pages/dashboard/` copies of these two are effectively dead. (`pages/dashboard/`
also holds a second `RelayDashboard`/`CostDashboard` used by the nested layout.)

---

## 3. Full frontend-call → serve-route table

Verified against route tables at HEAD `5852c93c05`. "Base" `/api`, `/relay`,
`/health` all reach roko-serve (proxy in dev, same-origin in prod).

### REST — exists ✅
| Frontend call (file:line) | Serve route | Status |
|---|---|---|
| `/api/config` GET/PUT — DataHub.ts:783/795, useRokoConfig.ts | config.rs:36/68 | ✅ |
| `/api/workspaces/default` — DataHub.ts:829 | workspaces.rs:69 | ✅ |
| `/api/workspaces` POST, `/api/workspaces/{id}` DELETE — DataHub.ts:838/857 | workspaces.rs | ✅ |
| `/api/managed-agents` — DataHub.ts:821, AgentFleet:407, IsfrDashboard:1178 | agents.rs:58 | ✅ |
| `/api/agents/topology` — AgentFleet:410 | aggregator.rs:43 | ✅ |
| `/api/episodes` — IntegrityView:200, Explorer:72 | status/episodes.rs:16 | ✅ |
| `/api/gates/history` — IntegrityView:199 | (routes) | ✅ |
| `/api/statehub/events` — Explorer:73 | status/mod.rs:45 | ✅ |
| `/api/metrics/c_factor` — CostDashboard:131 | status | ✅ |
| `/api/c-factor/trend` — CostDashboard:132 | learning/mod.rs:26 | ✅ |
| `/api/learn/efficiency` — CostDashboard:130, BenchLearningInsights:167 | learning/mod.rs:28 | ✅ |
| `/api/learn/cascade-router` — CostDashboard:133, CascadeRouter:149 | learning/mod.rs:38 | ✅ |
| `/api/learn/cascade` — BenchLearningInsights:161 | learning/mod.rs:42 | ✅ |
| `/api/learn/provider-outcomes` — CostDashboard:134 | learning/mod.rs:32 | ✅ |
| `/api/learn/adaptive-thresholds` — CostDashboard:135 | learning/mod.rs:45 | ✅ |
| `/api/learn/router` — useLearningStats:103 | learning/mod.rs:49 | ✅ |
| `/api/learn/gate-thresholds` — useLearningStats:122 | learning/mod.rs:48 | ✅ |
| `/api/learning/gate-thresholds` — BenchLearningInsights:173 | learning/mod.rs:47 | ✅ |
| `/api/learn/experiments` — useLearningStats:136 | learning/mod.rs:43 | ✅ |
| `/api/knowledge/entries` — KnowledgeGraph:39, DreamsView:50, KnowledgeEntries:247 | neuro routes | ✅ (empty) |
| `/api/knowledge/edges` — KnowledgeGraph:40 | neuro routes | ✅ (empty) |
| `/api/dream/journal` — DreamsView:49, DreamPhaseViz:205 | dream.rs | ✅ (empty) |
| `/api/bench/runs`,`/suites`,`/models`,`/run(s)/{id}`,`/cancel`,`/export`,`/compare`,`/pareto`,`/cost-summary`,`/provider-status` — DataHub, useBench(Runs) | bench.rs:35-53 | ✅ |
| `/api/isfr/status|current|history|sources` — DataHub, isfr-api.ts | isfr.rs:23-26 | ✅ |
| `/api/chain/blocks|transactions|events|watcher` — DataHub, chain-api.ts | chain.rs | ✅ |
| `/api/feeds/catalog` — DataHub:1217, feeds-api.ts | feeds.rs:31 | ✅ |
| `/relay/agents|workspaces|feeds|topics|health` — relay-api.ts | relay_proxy.rs `/relay/{*path}` | ✅ (proxied) |
| `/health` — api.ts probe, useLiveApi | health | ✅ |

### SSE / WebSocket — exists ✅
| Frontend stream (file:line) | Serve route | Status |
|---|---|---|
| SSE `/api/events` — SseAdapter (bootstrap), useEventStream:195 | sse.rs `/api/events` | ✅ |
| SSE `/api/workflow/events` — useEventStream:196 | mod.rs:191 | ✅ |
| SSE `/api/workflows/latest/stream` — workflow-api.ts:190 | workflows.rs:37 | ✅ |
| SSE `/api/bench/events` — useBenchSSE:39 | bench.rs:53 | ✅ |
| WS `/api/workflow/ws` — bootstrap ws, workflow-api.ts:236, dashboard/IsfrDashboard:936 | workflows.rs:41 | ✅ |
| WS `/api/ws` (terminal) — useTerminal:406 | ws.rs:24 | ✅ |
| WS `/relay/events/ws` — dashboard/RelayDashboard:36 | relay_proxy.rs:27 | ✅ |
| WS mirage `/api/rpc/events` — useChain:128 (MIRAGE_EVENTS_WS_URL) | rpc_proxy.rs:79 | ✅ (needs mirage) |

### 404 / mismatch — ❌ (the four hard breaks)
| Frontend call (file:line) | What it should be | Status |
|---|---|---|
| `/api/share/${token}` — pages/Share.tsx:103 | server exposes `/api/shared/{token}` (shared_runs.rs:874) | **[404]** singular vs plural |
| `/api/bench/matrix` POST — hooks/useMatrixBench.ts:151 | no matrix route in bench.rs at all | **[404]** matrix bench unimplemented server-side |
| `/api/isfr/stream` — lib/isfr-api.ts:113 `isfrStreamUrl()` | no `/isfr/stream` route (isfr.rs:23-26) | **[404]+[DEAD]** — also has **no caller** in `src`; latent |
| `${WS_BASE}/ws/agents` — pages/isfr/IsfrTabDrawer.tsx:101 (`AgentsPanel`) | server has `/api/ws` + `/roko-ws`, not `/ws/agents` | **[404]** WS upgrade falls into SPA fallback → HTML → fails |

All four resolve to the `serve_embedded` SPA fallback, so they return `index.html`
(HTTP 200 HTML) or a 404 rather than JSON/WS — the panels silently show empty /
"Connecting…". `isfrStreamUrl` is dead (defined, never imported), so it never
actually fires; the other three fire at runtime.

---

## 4. Event-schema handling (camelCase vs snake_case drift)

The server (`roko-core`/`roko-serve`) serializes events in **snake_case** (Rust
serde), with some `#[serde(rename)]` PascalCase tags for Bench/Matrix/SWE. The
frontend has **two independent ingestion paths that disagree**:

**Path A — DataHub (new, correct).** `SseAdapter` → `parseServerEvent`
(`transport/types.ts:203`) recursively converts snake→camel
(`snakeToCamelObj`) before handing a typed `ServerEvent` (camelCase) to
`useDataHub.handleServerEvent`. This matches the camelCase `ServerEvent` union.

**Path B — EventStreamManager (old, raw).** `hooks/useEventStream.ts`
`normalizeEvent` (line 46) does **not** camel-convert — it just spreads
`{...nested, ...parsed, type}` and dispatches the **raw snake_case** object to
`useContextEventSubscription` subscribers. Any component still on Path B receives
snake_case fields (`plan_id`, `input_tokens`) while the rest of the app assumes
camelCase → the documented drift.

### Data-shape drift table
| Concern | Path A (DataHub) | Path B (EventStreamManager) | Consequence |
|---|---|---|---|
| Key casing | camelCase (converted) | snake_case (raw) | components mixing paths read wrong field names |
| Known-event lists | `transport/sse.ts` 30 types (incl `isfr_*`, `chain_*`, `feed_*`) | `useEventStream.ts` 26 types (incl `workflow_started`, `gate_started/passed/failed`, `agent_completed/failed`, `feedback_recorded`, `state_checkpointed`) | **[DUP]** two hand-maintained lists; Path B lists event names not present in the `ServerEvent` union (server emits `gate_result`, `agent_stopped`, `plan_*` — not `gate_started`), so those named listeners never fire |
| Nested arrays-of-objects | `snakeToCamelObj` recurses objects but **skips arrays** (`types.ts:189`) | n/a | array-item keys (e.g. ISFR `readings[]`) stay snake_case; DataHub REST fetchers hand-map them (`fetchIsfrCurrent`) — SSE array payloads would not be converted |
| Bench/Matrix/SWE tags | PascalCase preserved verbatim (correct) | not in Path B list | Bench UI relies on `useBenchSSE` (separate `/api/bench/events` stream), not Path A/B |
| REST snake→camel | manual per-fetcher in DataHub (`chain`, `isfr`, `feeds`) | per-hook ad hoc | drift risk each time a server field is added |

Net: **two SSE managers connect to `/api/events` simultaneously** (SseAdapter +
EventStreamManager), plus EventStreamManager opens a third socket to
`/api/workflow/events`. Duplicate connections + divergent parsing.

---

## 5. Real-data vs empty panels (signals/engrams split)

Backing store availability decides what renders:
- **Real (signals/episodes/learn are populated at rest):** CostDashboard
  (`learn/*`, `metrics/c_factor`), IntegrityView (`episodes`, `gates/history`),
  CascadeRouter (`learn/cascade-router`), Explorer (`episodes`, `statehub/events`),
  Bench pages (`bench/*` — has fixture suites), AgentFleet (`managed-agents`),
  Settings (`config`), Terminal (live `/api/ws`).
- **Empty unless a subsystem is actively running (engrams/neuro/dreams/feeds):**
  KnowledgeGraph + KnowledgeEntries (neuro/durable store empty until distillation
  runs), DreamsView (`dream/journal` — no runtime dream trigger per CLAUDE.md
  gap #14), IsfrDashboard (needs ISFR keeper), FeedsDashboard (needs feed agents
  connected to relay), RelayDashboard (needs relay agents/workspaces), ChainTab
  (needs mirage / chain watcher). These map to the "engrams empty" side of the
  signals-vs-engrams split — the panels are wired but usually blank.

---

## 6. DataHub migration — incomplete

`DataHub.ts`'s own header lists six modules it *replaces*, yet `main.tsx` still
mounts three of them as providers: `WorkspaceProvider` (useWorkspace),
`RokoConfigProvider` (useRokoConfig), `EventStreamProvider` (EventStreamContext).
43 files still import the deprecated hooks (`useRokoConfig`, `useWorkspace`,
`useLiveApi`, `useServerStatus/Health`, `useContextEventSubscription`). Result:
config, health, workspace, and event state exist in **both** DataHub and the old
hooks; two SSE managers run in parallel (§4). The migration created the new spine
but never removed the old one.

---

## 7. Checklist to fix

### 404s
- [ ] **/api/share → /api/shared**: change `pages/Share.tsx:103` to
  `/api/shared/${token}` (or add a `/api/share/{token}` alias in
  `routes/shared_runs.rs`). Pick one; prefer fixing the FE to match server.
- [ ] **/api/bench/matrix**: either implement a `POST /bench/matrix` handler in
  `routes/bench.rs` (matrix run launcher) or gate/remove `useMatrixBench.ts` +
  `MatrixBuilder`/`MatrixRaceTrack`/`MatrixDetailView` UI until the server exists.
- [ ] **/api/isfr/stream**: delete dead `isfrStreamUrl()` (`lib/isfr-api.ts:112-114`)
  — it has no caller — or add an `/isfr/stream` SSE route and wire it. (ISFR data
  already flows via `/api/events` `isfr_*` events, so deletion is preferred.)
- [ ] **/ws/agents**: repoint `IsfrTabDrawer.tsx:101 AgentsPanel` to a real socket
  (`/api/ws` terminal, or an agent-output stream) or remove the panel; today it
  hits the SPA fallback and shows "Connecting to agent stream…" forever.

### Schema drift
- [ ] Collapse the two `KNOWN_SSE_EVENT_TYPES` lists into one shared export
  (`transport/sse.ts` is the accurate one) and delete the stale Path-B names
  (`workflow_started`, `gate_started/passed/failed`, `agent_completed/failed`,
  `feedback_recorded`, `state_checkpointed`) that no server variant emits.
- [ ] Make Path B camel-convert too (reuse `snakeToCamelObj`) OR migrate all
  `useContextEventSubscription` consumers to DataHub selectors, then delete
  `useEventStream.ts` + `EventStreamContext.tsx`.
- [ ] Fix `snakeToCamelObj` to recurse into arrays-of-objects (`types.ts:189`) so
  SSE array payloads convert consistently, or document that array items stay
  snake_case and keep hand-mapping in DataHub.
- [ ] Add a generated shared event-schema (single source of truth from the Rust
  `ServerEvent`/`DashboardEvent` enum) to stop hand-maintaining ~85 variants.

### Finish DataHub migration
- [ ] Remove `EventStreamProvider` from `main.tsx`; migrate its subscribers to
  DataHub. Eliminates the duplicate `/api/events` connection.
- [ ] Remove `RokoConfigProvider` + `useRokoConfig`; route config reads through
  `useDataHub`/`fetchConfig`.
- [ ] Remove `WorkspaceProvider` + `useWorkspace`; use DataHub workspace slice.
- [ ] Delete superseded hooks (`useLiveApi`, `useServerHealth`, `useServerStatus`,
  `useApiWithFallback`) once callers move to `serverStatus`/`sseStatus` selectors.
- [ ] Delete the dead duplicate pages `pages/dashboard/IsfrDashboard.tsx` and
  `pages/dashboard/FeedsDashboard.tsx` (unused; `main.tsx` uses `pages/isfr/` +
  `pages/feeds/`).

### Build hygiene
- [ ] Ensure CI runs `npm run build` in `demo/demo-app` **before**
  `cargo build -p roko-serve`, so the `rust-embed` `dist/` is fresh in the binary.

---

## 8. Roadmap (suggested order)

1. **Fix the 4 route breaks** (share rename, isfr/stream delete, ws/agents
   repoint, matrix gate) — small, removes visibly-broken panels.
2. **Unify SSE**: one event-type list + camel conversion, drop the second
   `/api/events` connection. Immediate correctness + perf win.
3. **Retire the deprecated providers** one at a time (config → workspace →
   events), deleting hooks as callers move to DataHub selectors.
4. **Generate the event schema** from Rust to kill hand-maintained drift.
5. **Delete dead duplicate pages** and confirm the dev/embed build pipeline.

---

## 9. Key file map

| Concern | Path |
|---|---|
| Entry / providers / routes | `demo/demo-app/src/main.tsx` |
| Transport bootstrap | `demo/demo-app/src/app/bootstrap.ts` |
| State store (Zustand) | `demo/demo-app/src/app/DataHub.ts` |
| REST client | `demo/demo-app/src/transport/api.ts` |
| SSE adapter (new) | `demo/demo-app/src/transport/sse.ts` |
| WS adapter (new) | `demo/demo-app/src/transport/ws.ts` |
| Event union + parser | `demo/demo-app/src/transport/types.ts` |
| URL resolution | `demo/demo-app/src/lib/serve-url.ts` |
| SSE manager (old) | `demo/demo-app/src/hooks/useEventStream.ts` |
| Event context (old) | `demo/demo-app/src/contexts/EventStreamContext.tsx` |
| Relay/isfr/chain/feeds libs | `demo/demo-app/src/lib/{relay,isfr,chain,feeds}-api.ts` |
| SPA serving (server) | `crates/roko-serve/src/embedded.rs`, `routes/mod.rs:249` |
| Route tables (server) | `crates/roko-serve/src/routes/*.rs` |
