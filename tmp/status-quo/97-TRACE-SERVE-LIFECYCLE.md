# 97 — TRACE: `roko serve` Lifecycle (Startup → Request → Realtime → Write Path)

> **Verification header**
> - Repo: `/Users/will/dev/nunchi/roko/roko`
> - Git HEAD: `5852c93c05` (`refactor: remove deprecated dispatch_direct module`), branch `main`
> - Date: 2026-07-08
> - Method: direct code trace, `file:line` at every hop. Runtime artifacts inspected on disk (`.roko/events.jsonl`, 44 MB / 157,264 lines).
> - Supersedes/deepens: `46-SERVE-HTTP-REALTIME`, `59-API-ROUTE-LEDGER`, `32-EVENTS-BUS-STATEHUB`, `94-FEED-AGENTS`.

Status tags: **[WIRED]** live on the runtime path · **[PARTIAL]** built, half-connected · **[DEAD]** persisted-but-unused · **[FOOTGUN]** correct-but-dangerous.

---

## 0. TL;DR (the four surprises)

1. **`events.jsonl` is a write-only firehose that no panel reads.** 97% of the 44 MB file is `feed_tick` (152,965 / 157,264 sampled lines). Every `DashboardEvent` published to the serve `StateHub` is appended to `.roko/events.jsonl` (`state_hub.rs:147-151`), but `DashboardSnapshot::apply` treats `FeedTick`/`FeedAgentOnline`/`FeedAgentOffline` as **no-ops** (`dashboard_snapshot.rs:1223-1225`), and snapshot bootstrap reads a *different* file (`state/events.json`, `dashboard_snapshot.rs:1277`). The firehose grows unbounded and feeds nothing. **[DEAD/FOOTGUN]**
2. **Panels are empty on connect because the snapshot and the feed stream are different data planes.** The Feeds dashboard is fed only by the *live* SSE broadcast; the materialized snapshot never absorbs feed events, and SSE replay is capped at 256 (`sse.rs:54`). A late joiner sees an empty feeds panel until fresh ticks arrive.
3. **There are two "signals" of truth that never meet:** the snapshot bootstraps `signal_gates` from `.roko/engrams.jsonl` (`dashboard_snapshot.rs:1276`, via `read_signal_gates`), while live gate results arrive as `DashboardEvent::GateResult`. `signals.jsonl` (the substrate log) is not read by the dashboard bootstrap at all.
4. **`/ws`, `/roko-ws`, and `/api/ws` are three different endpoints with three different payloads.** `/ws` + `/roko-ws` (`ws.rs:24-25`) stream raw `ServerEvent` from the event bus; `/api/ws` (`aggregator.rs:62`) is a *mux* that also proxies discovered agent-server sockets. The relay proxy is mounted **outside `/api`, unauthenticated** (`mod.rs:248`).

---

## 1. Startup sequence — `roko serve`

Entry: `run_server` (`lib.rs:531`) → `ServerBuilder::run` (`lib.rs:509`) → `ServerBuilder::start_background` (`lib.rs:269`). Config is loaded via `load_config_unified` (`lib.rs:537`).

Ordered list of what `start_background` does (`lib.rs:269-499`):

| # | Step | file:line | Notes |
|---|------|-----------|-------|
| 1 | Resolve bind/port; honor `PORT` env (port only, never bind) | `lib.rs:270-291` | T3-25: `PORT` overrides port, keeps `127.0.0.1` bind. |
| 2 | Build `AppState` (creates `StateHub` w/ event log at `.roko/events.jsonl`) | `lib.rs:297-303`, `state.rs:839-846` | `state_hub_for_workdir` → `StateHub::with_event_log(1024, …)`. |
| 3 | `validate_bind_safety` | `lib.rs:311` | Rejects public bind without ack. |
| 4 | (feat `otlp`) init OTLP tracing | `lib.rs:315-322` | Conditional. |
| 5 | `state.restore_snapshot()` | `lib.rs:324` | Reloads persisted server snapshot. |
| 6 | Spawn `dispatch::dispatch_loop` | `lib.rs:333` | Template-agent dispatch worker. |
| 7 | `start_builtin_event_sources` | `lib.rs:334` | Signal-ingest + configured event sources. |
| 8 | `start_config_watcher` | `lib.rs:335` | `notify` watch on `roko.toml`. |
| 9 | `start_prd_publish_orchestrator` | `lib.rs:336` | PRD-publish → auto-plan trigger. |
| 10 | `start_feedback_loop` | `lib.rs:337` | Gate-failure replan feedback. |
| 11 | **`start_state_hub_bridge`** (Bridge A: EventBus→StateHub) | `lib.rs:339`, def `1320` | Shares `BridgeDedup`. |
| 12 | **`start_orchestrator_event_bridge_dedup`** (Bridge B: StateHub→EventBus) | `lib.rs:341`, def `1663` | Same dedup instance → breaks the cycle. |
| 13 | `start_state_snapshot_saver` | `lib.rs:342` | Periodic snapshot persistence. |
| 14 | `start_job_runner` | `lib.rs:343` | Marketplace job executor. |
| 15 | **`start_cold_archival_timer`** | `lib.rs:344`, def `2096` | Hourly; **gated by `cold_storage.enabled`** — returns a no-op task when disabled (`lib.rs:2099-2101`). |
| 16 | `start_workspace_gc` / `start_handle_gc` | `lib.rs:345-346` | GC timers. |
| 17 | **`start_demurrage_timer`** | `lib.rs:347`, def `1986` | 40 s heartbeat tick; demurrage fires every ~250 ticks (~2.9 h). Drives knowledge confidence decay. |
| 18 | ISFR contract auto-deploy (if `chain.auto_deploy_contracts`) | `lib.rs:348-385` | Best-effort. |
| 19 | `start_isfr_keeper` / `start_block_watcher` | `lib.rs:387-388` | Chain feeds. |
| 20 | `load_persisted_deployments` | `lib.rs:391` | Await. |
| 21 | Prime JWKS cache (if Privy configured) | `lib.rs:394-399` | Async. |
| 22 | `relay::start_workspace_registration` | `lib.rs:403` | Registers workspace with relay. |
| 23 | `start_isfr_relay_bridge` | `lib.rs:412` | Relay TopicMessage → local Pulse. |
| 24 | **`feed_agents::spawn_all`** | `lib.rs:415`, def `feed_agents/mod.rs:108` | Spawns **29** agents (comment says 15; list has 29 — `feed_agents/mod.rs:128-162`). Gated by `feed_agents_enabled()` (`mod.rs:111`). Publishes `FeedAgentOnline` + writes catalog. |
| 25 | `start_feed_relay_bridge` | `lib.rs:418`, def `2659` | Registers feeds, forwards `FeedTick` → relay. |
| 26 | `build_server_router` → `routes::build_router` | `lib.rs:420`, `mod.rs:131` | Router assembly (§1a). |
| 27 | `TcpListener::bind` | `lib.rs:427` | |
| 28 | Spawn chain-watcher subprocess (if `chain.rpc_url`) | `lib.rs:442-484` | Output → `.roko/chain-watcher.log`. |
| 29 | `axum::serve … with_graceful_shutdown(shutdown_on_cancel)` | `lib.rs:487-497` | Server task; cancel token drains. |

### 1a. Router assembly — `routes::build_router` (`mod.rs:131-263`)

1. **SseAdapter wiring** (before router build): `set_state_hub_consumer(dashboard_event_bridge(&state))` (`mod.rs:137-138`) then `start_runtime_event_subscription()` (`mod.rs:139`). This makes the `SseAdapter` a `RuntimeEvent` consumer that (a) fans out to its own broadcast for `/api/workflow/events`, and (b) forwards each `RuntimeEvent` into the `StateHub` as `DashboardEvent`s (`adapters.rs:513-527`).
2. Assemble `api` router by `.merge()`-ing ~40 route modules + `.nest("/providers"|"/models"|"/routing")` + `sse::routes()` + `rpc_proxy::routes()` + `/workflow/events` (`mod.rs:148-191`).
3. **Auth layering (only if `api_auth.enabled`)** — layers applied bottom-up so execution order is `require_api_key` → `require_scope` → handler (`mod.rs:193-201`).
4. `scrub_secrets` layer redacts secrets from text/JSON bodies (`mod.rs:205-208`, `middleware.rs:548`).
5. Terminal routes gated by `serve.terminal_enabled` + bind policy (`mod.rs:210-222`).
6. `ws` router (`/ws`,`/roko-ws`) layered with `require_api_key` **only** (no scope check) when auth enabled (`mod.rs:224-231`).
7. Top-level router: unauth `/health`,`/ready`,`/metrics`, public webhooks, public shared-runs, terminal, `.nest("/api", api)`, `.merge(ws)`, **`.merge(relay_proxy::routes())` — outside `/api`, unauthenticated** (`mod.rs:233-248`), SPA fallback (`mod.rs:250`).
8. Global layers: 4 MiB body cap (`mod.rs:255`), 100 req/s governor rate limit (`mod.rs:256-259`), `TraceLayer`, CORS, `with_state` (`mod.rs:260-262`).

**[FOOTGUN]** `relay_proxy` (`/relay`, `/relay/{*path}`, `/relay/*/ws`) sits outside the `/api` nest, so neither `require_api_key` nor `require_scope` runs on it (`mod.rs:248`, `relay_proxy.rs:22-30`). Only the global rate-limit + CORS apply.

---

## 2. Inbound authed request lifecycle

Take `POST /api/plans/run` with an `X-Api-Key`. Layer order (outer→inner, from `mod.rs`):

```
TCP → CORS → RateLimit(100/s, mod.rs:256) → TraceLayer
    → [/api nest] → scrub_secrets(mod.rs:205)
        → require_api_key(mod.rs:195, def middleware.rs:259)
            → require_scope(mod.rs:194, def middleware.rs:403)
                → route handler (plans.rs)
                    → data source (executor.json / StateHub / in-mem)
                        → Json response
                    ← scrub_secrets buffers+redacts body (middleware.rs:548-585)
                ← X-Auth-Method header appended (middleware.rs:348-351)
```

Step detail:
1. **Rate limit** — shared single-bucket governor; 429 `code=rate_limited` on overflow (`mod.rs:106-122`).
2. **`require_api_key`** (`middleware.rs:259`): `api_credential` reads `X-Api-Key` first, else `Authorization: Bearer` (`middleware.rs:140-159`). Credential resolution order for Bearer: named/legacy API key (`authenticate_api_key`, sync) → agent token (`try_agent_token`, async, hashes to `base64(sha256)` and scans `discovered_agents`) → Privy JWT (`try_privy_jwt`, JWKS validate) (`middleware.rs:282-317`). On success injects `AuthContext{method,scope,user_id}` into extensions (`middleware.rs:345`) + `x-user-id` header (`middleware.rs:338-342`).
3. **`require_scope`** (`middleware.rs:403`): GET/HEAD/OPTIONS bypass immediately (`middleware.rs:408-410`). Otherwise `required_scope_for(method,path)` (`middleware.rs:356`) vs `AuthContext.scope`; **missing context defaults to `"read"`** (`middleware.rs:417`). `is_scope_sufficient` — `admin` passes all, `required=="read"` passes all, else exact string equality (`middleware.rs:389-397`).
4. **Handler** reads its data source (see §4). For plans that is `executor.json` / plan files; for snapshot-backed reads (`/api/projections/{name}`) it is `state.state_hub.current_snapshot()` (`state_hub.rs:185`).
5. **`scrub_secrets`** buffers the whole body (cap 16 MiB), skips `text/event-stream` and binary, redacts via `LogScrubber`, returns 500 on collection failure (`middleware.rs:525-585`).

### 2a. Auth decision table

`required_scope_for` (`middleware.rs:356-386`) × `is_scope_sufficient` (`middleware.rs:389-397`):

| Method + Path prefix | Required scope | Passes with… |
|---|---|---|
| `GET`/`HEAD`/`OPTIONS` (any path) | `read` | **any authenticated caller** (incl. `read`) |
| `POST/PUT/DELETE /api/api-keys`, `/api/secrets`, `/api/config` | `admin` | `admin` only |
| `* /api/events/ingest` (mutating) | `agent:write` | `agent:write` or `admin` |
| `* /api/agents…` (mutating) | `agent:write` | `agent:write` or `admin` |
| `* /api/plans…`, `/api/prd…` (mutating) | `plan:write` | `plan:write` or `admin` |
| `* /api/workspaces…` (mutating) | `write` | `write` or `admin` |
| **any other mutating `/api/*`** | **`read`** (fallback, `middleware.rs:385`) | **any authenticated caller** |
| `/relay/*` (outside `/api`) | — | **no auth layer at all** (`mod.rs:248`) |
| `/health`, `/ready`, `/metrics`, public webhooks, public shared-runs | — | unauthenticated by design (`mod.rs:235-243`) |
| `/ws`, `/roko-ws` | (auth only, no scope) | any authenticated caller when `api_auth.enabled` (`mod.rs:224-231`) |

**[FOOTGUN]** The `"read"` fallback (`middleware.rs:385`) means any mutating route whose prefix is not explicitly enumerated (e.g. `/api/jobs`, `/api/research`, `/api/deployments`, `/api/integrations`, `/api/subscriptions`) is writable by a **read-scoped** key. Scope is a coarse prefix matrix, not per-route.

Scope sources: legacy single key → `admin` (`middleware.rs:192`); named `api_keys` entry → its declared `scope` (`middleware.rs:180`); agent token → `agent:write` (`middleware.rs:241`); Privy JWT → `admin` (`middleware.rs:212`).

---

## 3. Realtime path — event emitted → client

There are **two buses** and **three broadcast fan-outs**. Trace an event from each producer.

### 3a. The two buses

| Bus | Type | Home | Replay ring | Consumers |
|---|---|---|---|---|
| `AppState.event_bus` | `EventBus<ServerEvent>` | `event_bus.rs:25` (wraps `roko_runtime::event_bus`) | runtime ring | `/ws`, `/roko-ws` (`ws.rs:107`), `/api/ws` mux (`aggregator.rs:742`), Bridge A (`lib.rs:1321`) |
| `AppState.state_hub` | `StateHub` (`DashboardEvent`) | `roko-runtime/src/state_hub.rs:80` | ring 1024 (`state.rs:842`) + `watch<DashboardSnapshot>` + on-disk `events.jsonl` | `/api/events`,`/api/sse` (`sse.rs:60`), `/api/projections/*/stream` (`projections.rs:58`), TUI snapshot borrow, Bridge B (`lib.rs:1667`) |

They are kept in sync by the **bidirectional bridge** with `BridgeDedup` (`lib.rs:1260-1318`):
- **Bridge A** `start_state_hub_bridge` (`lib.rs:1320`): `event_bus.subscribe()` → `server_event_to_dashboard` (`lib.rs:1346`) → `state_hub.sender().publish` → marks dashboard seq.
- **Bridge B** `start_orchestrator_event_bridge_dedup` (`lib.rs:1663`): `state_hub.subscribe_events()` → `dashboard_event_to_server` (`lib.rs:1694`) → `event_bus.publish` → marks server seq.
- Dedup: each bridge skips seqs the other produced (`is_bridged_server_seq`/`is_bridged_dashboard_seq`, `lib.rs:1329`,`1675`), breaking the `REST→EventBus→A→StateHub→B→EventBus…` cycle. Sets bounded at 4096 (`lib.rs:1272`).

### 3b. The third fan-out — RuntimeEvent via SseAdapter

`SseAdapter` (`adapters.rs:29`) subscribes to `roko_runtime::event_bus::runtime_event_bus::<RuntimeEvent>()` (`adapters.rs:83`). On each `RuntimeEvent`:
- Converts to `SseEvent` and broadcasts to its own channel → `/api/workflow/events` (`mod.rs:302-336`, `adapters.rs:514-518`).
- Forwards the raw `RuntimeEvent` to the `state_hub_consumer` = `DashboardEventBridge` (`adapters.rs:519-527`, bridge def `lib.rs:580-672`), which maps e.g. `WorkflowStarted→[PlanStarted,TaskStarted]`, `GatePassed→[GateResult,EventLogEntry]` and calls `state_hub.publish_batch` (`lib.rs:668-669`).

### 3c. Fan-out diagram

```
 PRODUCERS                         BUSES                                CLIENT ENDPOINTS
──────────────────────────────────────────────────────────────────────────────────────
 feed agents (29) ──FeedTick──┐
 REST handlers  ──ServerEvent─┤
 chain watcher  ──ServerEvent─┤──► event_bus: EventBus<ServerEvent>
                              │        │  replay_from(0)
                              │        ├──────────────────────────► GET /ws, /roko-ws        (ws.rs)
                              │        ├──────────────────────────► GET /api/ws  (mux+agents)  (aggregator.rs:62)
                              │        └──Bridge A (server→dash)──┐
                              │                                    ▼
 orchestrator ─DashboardEvent────────────────────────────► StateHub<DashboardEvent>
 (roko plan run, in-proc)          Bridge B (dash→server) ◄──┘   │  publish():
                                                                  │   • broadcast ─► GET /api/events, /api/sse  (sse.rs)
                                                                  │                 GET /api/projections/{n}/stream
                                                                  │   • watch<Snapshot> ─► TUI 60fps / REST current_snapshot()
                                                                  │   • append ─► .roko/events.jsonl   ⚠ firehose, no reader
 WorkflowEngine ─RuntimeEvent─► runtime_event_bus ─► SseAdapter ─┤
                                                                  ├─ broadcast ─► GET /api/workflow/events  (mod.rs:191)
                                                                  └─ DashboardEventBridge ─► StateHub.publish_batch (lib.rs:669)
 relay service ──(HTTP/WS)──────────────────────────────────────► /relay/*  (unauth reverse proxy, mod.rs:248)
```

### 3d. Endpoint replay/keepalive semantics

| Endpoint | Payload | Replay on connect | Keepalive | file:line |
|---|---|---|---|---|
| `/api/events`, `/api/sse` | `DashboardEvent` (SSE `data:` + `id:`=seq) | `Last-Event-ID` → `replay_from`, **capped 256** | 8 s `keepalive` | `sse.rs:41-88` |
| `/api/workflow/events` | `SseEvent` (RuntimeEvent-derived) | none (live only) | 8 s | `mod.rs:301-336` |
| `/api/projections/{name}/stream` | projection deltas | — | — | `projections.rs:44-58` |
| `/ws`, `/roko-ws` | `ServerEvent` JSON | `replay_from(0)` then live; client may resend `cursor` + `subscribe` filter | ws ping | `ws.rs:86-204` |
| `/api/ws` | mux `{source,event}` frames | `event_bus.replay_from(0)` tagged `roko-serve` + proxied agent sockets | — | `aggregator.rs:728-773` |

WS filters (`ws.rs:216-265`) support `projection:`, `topic:*`, `engram-stream:` prefixes; `Coalesce`/`ResumeRequired` back-pressure modes are parsed but **not implemented** (`ws.rs:126-134`, `202`) **[PARTIAL]**.

---

## 4. Write path feeding dashboards — and why panels go empty

### 4a. Snapshot bootstrap (what a fresh page load actually shows)

On startup / reconnect the snapshot is built by `DashboardSnapshot::load_from_workdir` (`dashboard_snapshot.rs:1252-1303`), reading:

| Source file | Populates | line |
|---|---|---|
| `.roko/state/executor.json` | plans, tasks, agents, gate results | `1274` |
| `.roko/state/task-trackers.json` | task trackers | `1275` |
| **`.roko/engrams.jsonl`** → `read_signal_gates` | `signal_gates` panel | `1276` |
| `.roko/state/events.json` | event-log entries | `1277` |
| `.roko/learn/experiments.json` | experiment winners | `1278` |
| `.roko/learn/c-factor.jsonl` | C-factor trend | `1279` |
| `.roko/learn/cascade-router.json`, `gate-thresholds.json` | routing/threshold panels | `1291-1294` |
| `.roko/episodes.jsonl`, `.roko/learn/efficiency.jsonl` | episodes, efficiency | `1297-1300` |

**The 44 MB `.roko/events.jsonl` firehose is NOT in this list.** Bootstrap reads `state/events.json` (a curated file), not the append-only `events.jsonl`.

### 4b. The signals vs engrams split

- The dashboard's `signal_gates` come from **`engrams.jsonl`** (`dashboard_snapshot.rs:1276`) — the FileSubstrate engram log.
- The substrate **`signals.jsonl`** (query/signal DAG log) is **not read** by the dashboard bootstrap.
- Live gate outcomes arrive as `DashboardEvent::GateResult` via the bridges; they update `stats` but the historical `signal_gates` panel only reflects what was in `engrams.jsonl` at boot.
- Net effect: a panel labelled "signals/gates" is populated from engrams, so if engrams.jsonl is empty/rotated the panel is blank even though signals.jsonl has data → **[FOOTGUN]** the two logs are named alike but only one feeds the UI.

### 4c. Why the Feeds panel is empty on connect

Trace a feed tick end to end:
1. Feed agent → `ctx.publish_tick` → `event_bus.publish(ServerEvent::FeedTick{…})` (`feed_agents/mod.rs:90-97`).
2. Bridge A → `server_event_to_dashboard(FeedTick)` → `Some(DashboardEvent::FeedTick{…})` (`lib.rs:1602-1614`).
3. `state_hub.sender().publish(FeedTick)` does three things (`state_hub.rs:296-305`):
   - broadcasts → **live** SSE clients see it ✅
   - `snap.apply(&FeedTick)` → **no-op** (`dashboard_snapshot.rs:1223-1225`) ❌ nothing enters the snapshot
   - `event_log.append` → line written to `.roko/events.jsonl` ✅ (firehose)
4. On reconnect: snapshot has no feed data (step 3 was a no-op), and SSE replay is capped at 256 (`sse.rs:54`). So the Feeds panel is blank until fresh live ticks arrive.

The `/api/feeds/catalog` endpoint sidesteps this by reading `state.feed_agent_catalog` written at spawn (`feed_agents/mod.rs:164-202`) — that's why the *catalog* shows agents while the *tick* panel is empty.

### 4d. The firehose, quantified

`.roko/events.jsonl` = 44,903,313 bytes / 157,264 lines. Type histogram (sampled):

```
152,965  feed_tick        ← DashboardEvent, no-op apply, no reader
  3,291  chain_block
    232  feed_agent_online
    133  gate.dispatch.started ┐
    130  gate.completed        │ dotted names = CLI runner/runtime events
    119  task.attempt.started  │ (roko plan run) written to the SAME file
    112  task.attempt.completed┘
     22  run.started / resume.marker …
```

Two producers, two schemas, one file: (a) serve `StateHub` event-log writes `DashboardEvent`s (`state.rs:841`), (b) `roko plan run` writes runtime/runner records to the same path (`run.rs:1231-1236`), and `projection_contract.rs:1601` reads it back as "runner/runtime event records." No rotation, no size cap. **[DEAD/FOOTGUN]**

---

## 5. Corrections / deltas vs prior docs

- **`94-FEED-AGENTS` / `feed_agents/mod.rs:1,101` say "15 agents"** — the actual `spawn_all` vector registers **29** (`feed_agents/mod.rs:128-162`). The "15" figure and the `lib.rs:414` comment are stale.
- **`32-EVENTS-BUS-STATEHUB`** should note that `events.jsonl` is *append-only and read by nothing on the dashboard path*; the snapshot bootstrap uses `state/events.json`, not `events.jsonl` (`dashboard_snapshot.rs:1277`). Prior framing implied the JSONL feed hydrates panels — it does not.
- **`59-API-ROUTE-LEDGER`** — confirm `/api/ws` (aggregator mux) is distinct from top-level `/ws`+`/roko-ws`; and that `/relay/*` is mounted outside `/api` and is unauthenticated.
- **Auth is not per-route.** `46-SERVE-HTTP-REALTIME` should flag the `"read"` fallback (`middleware.rs:385`): non-enumerated mutating routes accept read-scoped keys.

---

## 6. Checklist (verified this pass)

- [x] Startup order captured from `start_background` (`lib.rs:269-499`), 29 steps.
- [x] Router assembly + layer order (`mod.rs:131-263`); auth layered only when `api_auth.enabled`.
- [x] `require_api_key` credential order: X-Api-Key → Bearer(apikey→agent→privy) (`middleware.rs:259-353`).
- [x] `require_scope` prefix matrix + `read` fallback (`middleware.rs:356-435`).
- [x] Two buses (event_bus / state_hub) + bidirectional dedup bridge (`lib.rs:1320`,`1663`).
- [x] Third fan-out: SseAdapter RuntimeEvent → workflow SSE + StateHub (`adapters.rs:66-96,513-527`).
- [x] SSE replay cap 256 (`sse.rs:54`); WS replay-from-0 + filters (`ws.rs`).
- [x] `/api/ws` mux proxies agent sockets (`aggregator.rs:728-773`).
- [x] `relay_proxy` mounted outside `/api`, unauth (`mod.rs:248`).
- [x] Snapshot bootstrap sources (`dashboard_snapshot.rs:1252-1303`).
- [x] `FeedTick`/`FeedAgentOnline`/`Offline` apply = no-op (`dashboard_snapshot.rs:1223-1225`).
- [x] `events.jsonl` firehose measured (44 MB, 97% feed_tick) and traced to `state_hub.rs:147-151`.
- [x] Cold-archival + demurrage timers gating (`lib.rs:2096-2101`, `1986-2082`).

## 7. Roadmap (fix candidates)

1. **[FOOTGUN] Cap/rotate `events.jsonl`** or stop persisting `FeedTick` to the StateHub event log (skip no-op DashboardEvents in `StateHub::publish` when apply is a no-op). Today it grows unbounded and nothing reads it.
2. **[FOOTGUN] Populate the snapshot for feeds** (make `DashboardSnapshot::apply` retain a bounded ring of recent `FeedTick`s per feed) so reconnecting clients aren't blank, or document that Feeds are live-only.
3. **[FOOTGUN] Tighten auth**: replace the `"read"` prefix fallback (`middleware.rs:385`) with an explicit allow-list / default-deny for mutating verbs; put `/relay/*` behind at least `require_api_key`.
4. **[PARTIAL] Implement WS `Coalesce`/`ResumeRequired`** back-pressure or reject them (`ws.rs:126-134`).
5. **Doc fix**: correct the "15 feed agents" comments (`feed_agents/mod.rs:1,101`; `lib.rs:414`) to 29.
6. **Unify the signals/engrams naming** so the gate panel source is unambiguous (`dashboard_snapshot.rs:1276`).
```
