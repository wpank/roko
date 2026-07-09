# roko-serve — HTTP Control Plane & Realtime

> Status-quo audit · re-verified 2026-07-08 @ HEAD 5852c93c05 (main) · supersedes concise draft · sources: 34 files (109 in crates/roko-serve/src total; 45 declare routes), roko-core/src/config/serve.rs, roko-fs/src/layout.rs, roko-runtime/src/state_hub.rs, 7 design docs, live `.roko/` data dir, sibling audits 35/41/44/59)

## Summary

`roko-serve` is a real, large control plane: **~61K LOC across 109 `.rs` files**, **288 `.route(` registrations** confirmed at HEAD 5852c93c05 (Grep count 2026-07-08). Of these ~**260 are production** and **28 live inside `#[cfg(test)]` modules** — including **7 mock-sidecar routes in `aggregator.rs:1587-1593`** (`/health`, `/capabilities`, `/stats`, `/predictions`, `/predictions/residuals`, `/tasks`, `/stream`), so aggregator's headline "25 registrations" is **18 prod + 7 test**. Terminal contributes 9 registrations across two variant blocks (`terminal.rs:1120-1151`) but mounts only one variant (**5 enabled** or **4 disabled** at runtime). Production surface is **~280 method-operations** over ~255 distinct mounted paths. CLAUDE.md's "~85 routes" is not the mounted surface — it matches the **87 operations documented in `openapi.rs:53-141`** (`/openapi.json`). The testing audit's "~288" is the raw registration count including test routers.

**Router assembly (`routes/mod.rs:131-263`, re-read 2026-07-08):** `build_router` merges 38 route-module `routes()` fns + 3 nested routers (`/providers`, `/models`, `/routing`) into one `/api` sub-router (`:148-191`), applies auth as two stacked `from_fn` layers (`require_scope` then `require_api_key`, `:193-201`) only when `api_auth.enabled`, then a secret-scrubber layer (`:205-208`). Terminal (`:210-222`), WS (`:224-231`), relay proxy (`:248`), webhooks-public + shared-runs-public (`:240-243`), and the 3 top-level probes `/health` `/ready` `/metrics` (`:235-239`) mount **outside** `/api` (no scope layer). SPA fallback at `:250`. Global layers: 4 MiB body cap, rate limiter, TraceLayer, CORS (`:252-262`).

The server is not a mock: `/api/run`, plan/job execution, gateway inference (CascadeRouter + real providers), PRD auto-plan, SSE/WS with replay, 29 feed agents, ISFR keeper, block watcher, terminal PTY, and ~22 background tasks are wired. The three biggest problems are (1) a **data-source split**: dashboards read `.roko/engrams.jsonl` (10 lines, 0 gate verdicts) while the CLI writes canonical GateVerdict signals to `.roko/signals.jsonl` (467 entries); (2) **HTTP plan execution is not the DAG executor** — it flattens the plan into a prompt and calls `run_once`; (3) **auth is real but the scope matrix is coarse** and several v2-AUTH concepts (role grants, auth-as-cells) don't exist. Two realtime event vocabularies (ServerEvent on WS, DashboardEvent on SSE) are bridged lossily.

## Route census (grouped, counted, status per group)

Method: static extraction of `.route("path", methods)` with balanced-paren parsing, split at `#[cfg(test)]`. Raw totals: 288 registrations / 308 ops; production 260 registrations / ~280 ops; test-only 28. Assembly: `routes/mod.rs:131-263` (`build_router`) nests most groups under `/api`, plus top-level public routes.

Legend: ✅ wired-real · 🟡 partial/degraded · 🔌 built-not-wired · ❌ stub · 🕰️ legacy/deprecated-paradigm

| Group (mount) | File | Routes | Status | Notes / evidence |
|---|---|---:|---|---|
| status/metrics/gates/statehub (`/api`) | `routes/status/mod.rs` | 27 | 🟡 | Real handlers (`/status`, `/dashboard`, 12 metrics, `/gates/*`, `/episodes`, `/signals`, `/statehub/snapshot`, `/statehub/events`, `/truth_map`, `/parity`, `/retention`) but gate/signals readers hit the wrong file — see data-source bugs |
| learning (`/api/learn*`) | `routes/learning/mod.rs` | 25 | ✅ | 11 real datasets each aliased `/learn/*` + `/learning/*`, + `/c-factor/trend`, `/executor/state`; reads `.roko/learn/*` via `projection_contract.rs:1590-1634` |
| aggregator (`/api`) | `routes/aggregator.rs` | 18 | 🟡 | Fan-out proxy over discovered sidecars. `/agents`, `/agents/{id}/stats|skills|heartbeat` real; `/agents/{id}/trace` **hardcoded empty** (`aggregator.rs:308-323`); `/tasks`, `/predictions/*` empty-by-inheritance (sidecar queues never fill — see 44-AGENT-SERVER); `/agents/topology` synthesizes edges; `/knowledge/*` real (neuro); `GET /api/ws` exists (`aggregator.rs:62`) — corrects 59-ledger's "Mirage-only /api/ws" claim |
| bench (`/api/bench`) | `routes/bench.rs` | 19 | ✅ | Run/suites/models/pareto/export/SSE events; strategies Minimal…FullCascade dispatch real LLMs, `Demo` simulated (`bench.rs:147-162`); no `/bench/matrix` (frontend calls it — 59-ledger) |
| plans (`/api/plans`) | `routes/plans.rs` | 15 | 🟡 | CRUD/status/gates/costs/reviews/diff/chat/estimate real; **`POST /plans/{id}/execute` flattens plan → prompt → `runtime.run_once`** (`plans.rs:206,223`, `build_plan_execution_prompt` `plans.rs:1432`) — not the roko-orchestrator DAG+gates path used by `roko plan run` |
| agents (`/api/agents`) | `routes/agents.rs` | 13 | ✅ | create/start/stop/restart via ProcessSupervisor (`agents.rs:806-901`); `/message` proxies to sidecar WS `/stream` with fallback POST + `proxy_token` (`agents.rs:1214-1290,1389-1461`); token issue/rotate (`agents.rs:1655+`); second lifecycle tracker parallel to CLI's `agents.json` (44-AGENT-SERVER) |
| jobs (`/api/jobs`) | `routes/jobs.rs` | 10 | ✅ | Marketplace CRUD + lifecycle; `job_runner.rs:47,145,192,282` auto-executes via `runtime.run_once` with stale-lock reclaim |
| prds (`/api/prds`) | `routes/prds.rs` | 9 | ✅ | Idea/draft/promote/plan; `/prd/consolidate` + `/prds/consolidate` alias; publish triggers auto-plan (below) |
| terminal (top-level) | `terminal.rs:1118-1152` | 5 (4 disabled variant) | ✅ | PTY sessions + `/ws/terminal/{id}`; disabled by default (403), loopback allowed w/o auth, public bind requires key (`routes/mod.rs:143-146,210-222`, tests `:406-486`) |
| workflows (`/api`) | `routes/workflows.rs` | 7 | ✅ | Snapshots from `.roko/events.jsonl` (`workflows.rs:1391`), per-workflow SSE streams, `/workflow/ws` |
| chain (`/api/chain`) | `routes/chain.rs` | 7 | ✅ | Reads in-memory ring buffers fed by block watcher; empty without RPC |
| research (`/api/research`) | `routes/research.rs` | 6 | ✅ | topic/enhance-prd/plan/tasks/analyze; analyze reads engrams (`research.rs:509`) |
| deployments (`/api/deployments`) | `routes/deployments.rs` | 5 (8 ops) | ✅ | File-persisted (`load_persisted_deployments`), task proxy + callback |
| gateway (`/api`) | `routes/gateway.rs` | 5 | 🟡 | `POST /inference/complete` real: cached CascadeRouter (`gateway.rs:269-277,874,948-960`) + provider dispatch + cost/health; **paths & scope drift from docs/v2/08-GATEWAY.md** (design: `/api/gateway/inference`, `/batch/flush`, `/gateway/ws`; no 9-cell pipeline) |
| feeds (`/api/feeds`) | `routes/feeds.rs` | 5 | ✅ | Registry (seeded `lib.rs:1025-1138`) + runtime catalog of 29 feed agents |
| subscriptions (`/api/subscriptions`) | `routes/subscriptions.rs` | 5 (7 ops) | ✅ | Config-backed CRUD/enable/disable |
| rpc proxy (`/api/rpc`) | `routes/rpc_proxy.rs` | 5 | ✅ | Mirage/anvil reverse proxy |
| providers/models/routing (`/api/providers`, `/api/models`, `/api/routing`) | `routes/providers.rs` | 5 | ✅ | Nested routers (`routes/mod.rs:186-188`) |
| isfr (`/api/isfr`) | `routes/isfr.rs` | 4 | ✅ | status/current/history/sources from keeper state; **no `/isfr/stream`** (frontend calls it) |
| secrets (`/api/secrets`) | `routes/secrets.rs` | 4 | ✅ | admin-scoped set/delete/test |
| shared runs | `routes/shared_runs.rs` | 4 | ✅ | `POST /api/runs/{id}/share` authed (`:861-863`); `GET /api/runs/{id}`, `GET /api/shared/{token}`, `GET /runs/{id}` **intentionally public** (`:865-876`) |
| relay proxy (top-level `/relay`) | `routes/relay_proxy.rs` | 4 | 🟡 | WS bridges + `ANY /relay/{*path}` catch-all, mounted **outside auth** (`routes/mod.rs:248`); 503 when `ROKO_AGENT_RELAY_URL` unset |
| team (`/api/team`) | `routes/team.rs` | 4 (5 ops) | 🟡 | me/members/invite/role-update persisted; but no middleware role enforcement (see auth) |
| swe-bench (`/api/bench/swe`) | `routes/swe_bench.rs` | 4 | ✅ | Spawns real runs (`swe_bench.rs:75+`), datasets list |
| config (`/api/config`) | `routes/config.rs` | 3 (4 ops) | ✅ | get/put/toml/reload, admin-scoped |
| connectors (`/api/connectors`) | `routes/connectors.rs` | 3 | 🟡 | In-memory registry seeded with 2 defaults so lists aren't empty (`lib.rs:1035-1075`) |
| projections (`/api/projections`) | `routes/projections.rs` | 3 | 🟡 | catalog/get/stream via `RuntimeProjectionSet` (~15 projections + aliases, `projection_contract.rs:433-446,620-709`); v1 names `cohort_health`/`active_tasks`/`alerts` return placeholder shapes (`:625-655`) |
| templates (`/api/templates`) | `routes/templates.rs` | 3 (5 ops) | ✅ | CRUD + deploy (agent templates) |
| vision-loop (`/api/vision-loop`) | `routes/vision_loop.rs` | 3 | ✅ | Spawns `roko vision-loop` subprocess (`vision_loop.rs:3,143`) |
| webhooks | `routes/webhooks.rs` | 3 | ✅ | `/webhooks/github`, `/webhooks/slack` public + signature-verified (top-level, `routes/mod.rs:240`); `/webhooks/generic` authed under `/api` (`:185`) |
| workspaces (`/api/workspaces`) | `routes/workspaces.rs` | 3 (5 ops) | ✅ | Ephemeral workspace registry, persisted; GC'd hourly |
| api-keys (`/api/api-keys`) | `routes/auth.rs` | 2 (3 ops) | ✅ | list/create/delete named keys; **no `/api/auth/me`** despite API-REFERENCE.md documenting it |
| dream (`/api/dream`) | `routes/dream.rs` | 2 | 🟡 | `POST /dream/run` runs DreamRunner but with **`command: "cat"` as review agent** (`dream.rs:65`); `GET /dream/journal` reads `timestamp`/`phases[]` fields the writer never emits → `last_cycle` always `""`, phases synthesized (`dream.rs:143,153-157`) |
| event ingest (`/api/events/ingest`) | `routes/event_ingest.rs` | 2 | ✅ | Single+batch RuntimeEvent ingest; loopback/token/allowlist gating when auth off (`event_ingest.rs:52-82`) |
| heartbeats (`/api`) | `routes/heartbeats.rs` | 2 | 🟡 | Sidecar heartbeat sink + `/network/stats`; sidecar payload counters hardcoded 0 (44-AGENT-SERVER) |
| integrations (`/api/integrations`) | `routes/integrations.rs` | 2 | ✅ | Read-only integration status |
| neuro (`/api`) | `routes/neuro.rs` | 2 | ✅ | `POST /neuro/query` + `GET /knowledge` hit real `roko_neuro::KnowledgeStore` (`neuro.rs:44-98`) |
| run (`/api/run`) | `routes/run.rs` | 2 | ✅ | `POST /run` → background `runtime.run_once` (universal loop) with rich StateHub events (`run.rs:137-140`); `/run/{id}/status` |
| sse (`/api/events`, `/api/sse`) | `routes/sse.rs` | 2 | ✅ | See realtime section |
| ws (top-level `/ws`, `/roko-ws`) | `routes/ws.rs` | 2 | ✅ | See realtime section |
| dashboard runs (`/api/dashboard/runs`) | `routes/runs.rs` | 1 | ✅ | Reads `.roko/runtime-events.jsonl` (`runs.rs:20`) |
| diagnosis (`/api/diagnosis/recent`) | `routes/diagnosis.rs` | 1 | ✅ | Conductor diagnosis log |
| openapi (`/api/openapi.json`) | `openapi.rs` | 1 | 🟡 | utoipa spec covering only 87 ops |
| top-level probes | `routes/mod.rs:235-239` | 3 | ✅ | `/health`, `/ready` (shutdown-aware), `/metrics` (Prometheus) — public |
| workflow SSE (`/api/workflow/events`) | `routes/mod.rs:191,302-336` | 1 | ✅ | RuntimeEvent-typed SSE via SseAdapter |
| SPA fallback | `embedded.rs` via `routes/mod.rs:250`, JSON-404 for `/api|/ws` typos (`lib.rs:850-871`) | — | ✅ | Embedded React app |

Global middleware: 4 MiB body cap, 100 req/s global governor limiter, TraceLayer, CORS (loopback-predicate by default, explicit origins, or permissive only with `unsafe_public_cors` — `middleware.rs:469-500`), secret-scrubbing response layer (`middleware.rs:548-585`).

## Background tasks & subscribers census

Spawned in `ServerBuilder::start_background` (`lib.rs:269-499`) unless noted:

| # | Task | Where | Gate | Status |
|---|---|---|---|---|
| 1 | Agent dispatch loop (`TemplateAgentDispatcher`) | `lib.rs:327-333`, `dispatch.rs` | always | ✅ |
| 2 | Builtin event sources (cron + file-watch) → `signal_ingest_loop` → signal_store + bus | `lib.rs:334,2773-2824` | `[scheduler]`/`[watcher]` config | ✅ |
| 3 | Config hot-reload watcher | `config_watcher.rs`, `lib.rs:335` | always | ✅ |
| 4 | **PRD publish subscriber** (bus) + audit-log follower polling `.roko/episodes.jsonl` | `routes/prds.rs:256-262,196-254`; gate `serve.auto_orchestrate && prd.auto_plan` (`prds.rs:165`) | config | ✅ dual-path |
| 5 | Feedback loop | `feedback.rs`, `lib.rs:337` | always | ✅ |
| 6+7 | EventBus↔StateHub bidirectional bridges with `BridgeDedup` cycle-breaker | `lib.rs:338-341,1260-1344,1663-1690` | always | ✅ (lossy mapping, `_ => None` at `lib.rs:1627,1810`) |
| 8 | State snapshot saver (30 s) | `lib.rs:1858-1871` | always | ✅ |
| 9 | Job runner (auto-execute marketplace jobs) | `job_runner.rs:47` | always | ✅ |
| 10 | Cold archival timer (engrams → `.roko/cold/` + retention compaction) | `lib.rs:2096-2187` | `cold_storage.enabled` (**default off** → "built but not triggered" claim in CLAUDE.md item 14 is half-resolved: wired here, config-gated) | 🟡 |
| 11 | Workspace GC (default 300 s, 1 h max age) | `lib.rs:1880-1955` | always | ✅ |
| 12 | Handle GC (60 s; active_runs/plans/operations) | `lib.rs:1961-1974` | always | ✅ |
| 13 | Knowledge demurrage (40 s tick, fires ≈2.9 h) | `lib.rs:1986-2082` | always | ✅ |
| 14 | ISFR contract auto-deploy | `lib.rs:349-385` | `chain.auto_deploy_contracts` | 🟡 |
| 15 | ISFR keeper (4-source mock fallback; on-chain submit per epoch) | `lib.rs:2203-2436` | `isfr.enabled` | ✅/🟡 no-op silently when disabled or all sources offline |
| 16 | Block watcher (2 s poll, TCP pre-probe) | `lib.rs:2443-2555` | chain client configured | ✅ |
| 17 | JWKS prime | `lib.rs:394-399` | privy_app_id (auto-set) | ✅ |
| 18 | Relay workspace registration | `relay.rs`, `lib.rs:403-408` | `relay.url` | ✅ |
| 19 | ISFR relay bridge (TopicMessages → local bus pulses) | `lib.rs:2570-2656` | `relay.url` | ✅ |
| 20 | **Feed agents: 29** (15 original + 5 onchain + 5 defi + 4 market) | `feed_agents/mod.rs:108-162` | `feed_agents_enabled()` | ✅ (doc comment still says "15") |
| 21 | Feed→relay bridge (register feeds, forward FeedTicks) | `lib.rs:2661-2771` | relay + feed agents | ✅ |
| 22 | `roko-chain-watcher` subprocess (logs to `.roko/chain-watcher.log`) | `lib.rs:442-484` | `chain.rpc_url` | ✅ |
| — | **Dream heartbeat loop: NOT here.** `dreams.rs:39 start_dream_loop` is only spawned by `roko daemon` (`roko-cli/src/daemon.rs:368`); plain `roko serve` never dreams | — | — | 🟡 |

`run_server_with_state` (`lib.rs:782-829`) spawns only a subset (no ISFR/feeds/relay/GC/demurrage) — two startup paths with different task sets.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| REST control plane | v1 12-interfaces/05 (13 route groups) · v2-depth 16-surfaces/05 ("~85 routes") | `routes/` 45 modules, ~260 prod registrations | ✅ (3× larger than docs) | census above; `routes/mod.rs:148-191` |
| OpenAPI | — | `/api/openapi.json`, 87 ops | 🟡 covers ~⅓ of surface | `openapi.rs:53-141` |
| Auth: API keys (4 scopes) | v2 17-AUTH §API keys | `middleware.rs:100-196` SHA-256 named keys + legacy single key | ✅ | `middleware.rs:259-353` |
| Auth: Privy JWT | v2 17-AUTH §Privy | JWKS cache w/ hardcoded app id `cmhw01vut003tjx0d5lmqc8zs` | ✅/🟡 hardcoded tenant | `jwks.rs:14-17`; auto-set `lib.rs:899-901` |
| Auth: agent tokens | v2 17-AUTH §Agent tokens | issue/rotate + bearer match vs `token_hash` | ✅ | `middleware.rs:220-247`; `agents.rs:1655+` |
| Auth: scope enforcement | v2 17-AUTH role grants (Owner/Admin/Member/Viewer) | prefix matrix only: admin→api-keys/secrets/config; agent:write→agents,events/ingest; plan:write→plans/prd; write→workspaces; **all other POSTs need only "read"** | 🟡 | `middleware.rs:356-397` |
| AuthorizeCell / auth-as-Verify-Cells | v2 17-AUTH §pipeline | not present — plain axum middleware fns | ❌ (🕰️ design unrealized) | `middleware.rs` |
| Auth default | draft claimed "enabled by default" | struct `Default` `enabled: true` (`serve.rs:94-105`) but field `#[serde(default)]` → **false when `[serve.auth]` present without `enabled`**; `roko init` writes `false`; auto-enable on public bind w/ stored Privy cred | 🟡 mixed semantics | `roko-core/src/config/serve.rs:79-105`; `lib.rs:902-916`; `validate_bind_safety` `lib.rs:755-774` |
| SSE | v1 06 (co-equal transport, cursors) | `/api/events` `/api/sse`: DashboardEvent stream, `Last-Event-ID` replay capped 256, 8 s keepalive, proxy-buster headers | ✅ | `sse.rs:41-88` |
| WS | v1 06 (subscribe/query/publish frames, 3 BP modes, per-subscription auth) | `/ws` `/roko-ws`: ServerEvent stream, replay-from-0 + cursor catchup, string filters; **Coalesce/ResumeRequired rejected**; no query/publish frames; auth per-connection only | 🟡 | `ws.rs:52-150`, unimplemented modes `ws.rs:126-134` |
| Event vocabulary | one protocol (v1 06) | three: ServerEvent (WS), DashboardEvent (SSE), SseEvent kinds (`/api/workflow/events`); lossy bridges | 🟡 | `lib.rs:1346-1629,1694-1811`; `routes/mod.rs:302-336` |
| StateHub projections | v1 22 (22 canonical projections, filters, freshness) | `RuntimeProjectionSet`: ~15 projections + aliases, catalog with invalidation policies, `/statehub/snapshot|events`; ring 1024 | 🟡 | `projection_contract.rs:133,433-446,620-709`; `state_hub.rs:100-102` |
| Inference gateway | v2 08-GATEWAY (9 Cells, `/api/gateway/*`) | monolithic handlers at `/api/inference/complete`, `/api/inference/batch/*`, `/api/gateway/stats|models`; CascadeRouter selection + provider fallback + cost; no cells, no `batch/flush`, no gateway WS | 🟡 (🕰️ cell design) | `gateway.rs:266-380,874-960` |
| Plan execution over HTTP | parity contract (do ↔ POST /plans/:id/run, v2-depth 16/05:43-53) | prompt-flattening → `run_once`; no DAG, no per-task gates, no snapshots | 🟡 semantic gap | `plans.rs:194-256,1432-1444` |
| PRD auto-plan trigger | CLAUDE.md item 10 | bus subscriber + episodes-audit follower, config-gated | ✅ | `prds.rs:159-262,587` |
| Terminal | API-REFERENCE `/api/terminal/*` | 4 REST + `/ws/terminal/{id}`; disabled→403 default; public-bind auth guard | ✅ | `terminal.rs:1118-1152`; `routes/mod.rs:143-146` |
| Relay/RPC proxies | — | top-level `/relay/*` (no auth) + `/api/rpc/*` | ✅ proxy / 🟡 auth | `relay_proxy.rs:23-31` |
| Tests | — | 364 inline unit tests (middleware 55, dispatch 20, gateway 19); **no `tests/` integration dir**; no SSE/WS replay/contract tests | 🟡 | `grep -rc '#\[test\]\|#\[tokio::test\]' crates/roko-serve/src` |

## V2-aligned

- **Four auth paths exist** (API key, named keys w/ scopes+expiry, agent tokens, Privy JWT) matching v2 17-AUTH's path list, incl. hash-at-rest and token expiry (`middleware.rs:115-247`); team routes (`/api/team/*`) exist for the workspace-sharing model.
- **Gateway centralizes model selection** via persisted CascadeRouter warm-loaded at startup (`lib.rs:928-959`) — agents-don't-hold-keys direction of 08-GATEWAY.
- **Projection layer exists and is queried** (`/api/projections/*`, `/api/statehub/*`), with catalog + invalidation policies — a real (if partial) StateHub read-boundary per v1 22.
- **SSE replay cursors** (`Last-Event-ID`) and WS cursor catchup implement the reconnect core of v1 06.
- **Secure-by-default posture**: public bind refused without auth or explicit ack (`lib.rs:755-774`); PORT env no longer implies 0.0.0.0 (`lib.rs:202-229`); CORS loopback-predicate; body/rate limits; response secret scrubbing.
- **Event ingest** lets out-of-process `roko` commands feed the dashboard (`event_ingest.rs`), closing the CLI→serve telemetry loop.

## Old paradigm & tech debt (incl. engrams/signals reader bugs)

1. **engrams vs signals split (P0 data bug — re-verified live 2026-07-08)**: serve reads `.roko/engrams.jsonl` in `/api/gates/*` (`routes/status/gates.rs:84,92`), `/api/signals` (`routes/status/episodes.rs:35`), metrics (`routes/status/metrics.rs:119`), research analyze (`research.rs:509`, `:385` prompt, `:780` test), and projections (`projection_contract.rs:1602`; the type comment at `:526` even names them "Legacy signal/engram records"). On disk **right now**: `.roko/engrams.jsonl` = **10 lines, `grep -c GateVerdict` = 0**; `.roko/signals.jsonl` = **467 lines, 467 GateVerdicts** (written by CLI runner via `layout.signals_path()` — `roko-cli/src/runner/event_loop.rs`). roko-fs renamed signals→engrams (`roko-fs/src/layout.rs:202-217` marks `signals.jsonl` "legacy") but the writer never migrated. Every serve reader points at the near-empty file; every gate verdict lives in the file no reader opens. Mitigation-by-accident: gate views also merge `gate.completed` rows from `.roko/events.jsonl` (44 MB on disk; `gates.rs:86-89`), so waterfalls aren't fully empty — but canonical verdict signals (lineage, attestation) are invisible to every dashboard and every projection. **This is the single highest-leverage fix in the crate.**
2. **Dream HTTP surface is decorative**: `cat` as review agent (`dream.rs:65`) and journal reader/writer schema mismatch (`dream.rs:143,153-157` expects `timestamp`/`phases[]`; `DreamJournalEntry` writes `cycle_start`/durations). Plus no dream loop under `roko serve` at all (daemon-only).
3. **HTTP plan execute ≠ orchestrator**: `plans.rs:1432` prompt-flattening hides Runner-v2/DAG semantics; `/api/plans/{id}/gates` then reads gate data produced by a pipeline this route never runs.
4. **Two lifecycle trackers**: serve's `DiscoveredAgent` + ProcessSupervisor vs CLI's `.roko/runtime/agents.json` PID/kill (44-AGENT-SERVER); `aggregator.rs:1076-1110` merges both; stop paths can disagree.
5. **Mirage-parity aggregation**: `/api/tasks`, `/api/predictions/*` aggregate permanently-empty sidecar lists; `/agents/{id}/trace` returns `[]` hardcoded (`aggregator.rs:308-323`); topology edges synthetic. Responses don't mark themselves synthetic.
6. **Three event vocabularies + lossy bridges**: unmapped variants silently dropped (`lib.rs:1627,1810`); WS clients never see Diagnosis/ExperimentWinners/CascadeRouterUpdated etc.
7. **Alias sprawl**: `/learn/*` + `/learning/*` (11 dup pairs), `/prd/consolidate` + `/prds/consolidate`, `/bench/run` + `/bench/runs`, `/ws` + `/roko-ws` + `/api/ws` — no canonical/compat labeling.
8. **Design-code drift (🕰️)**: 08-GATEWAY 9-cell pipeline, 17-AUTH Verify-Cell pipeline + role grants, v1 06 query/publish frames and per-subscription auth, v1 22's 22 projections — all unbuilt; code went bespoke-REST instead. Meanwhile API-REFERENCE.md documents ~116 endpoints (incl. nonexistent `/api/auth/me`) vs ~255 mounted → **>130 undocumented**, and `/openapi.json` covers 87.
9. **In-memory-only registries**: connectors/feeds (seeded at boot, `lib.rs:1025-1138`), chain rings, heartbeats, bench handles — lost on restart; only some state is snapshot-persisted (30 s saver).
10. **Config-gated workers fail silent**: ISFR keeper, block watcher, cold archival, feed agents become no-ops without surfacing in `/api/health`/`/api/status`.
11. **Startup-path asymmetry**: `run_server_with_state` spawns ~8 of the ~22 background tasks (`lib.rs:782-829`).

## Not implemented (auth!)

- **Role-based authorization** (Owner/Admin/Member/Viewer grants per 17-AUTH:794-818): absent. `require_scope` is a prefix matrix; any read-scoped key can `POST /api/run`, `/api/jobs`, `/api/dream/run`, `/api/inference/complete`, `/api/deployments`, `/api/bench/run`, `/api/vision-loop` (`middleware.rs:356-397` returns `"read"` for all unlisted paths — read scope always passes `middleware.rs:389-397`).
- **Unauthenticated surfaces**: `/relay/{*path}` proxies **writes** to the relay with no auth (`relay_proxy.rs:29-30` mounted at `routes/mod.rs:248`); `GET /api/runs/{id}` exposes full run transcripts by raw run-id without auth (`shared_runs.rs:871-876` — deliberate, but ID-guessable vs opaque share tokens); `/metrics` public.
- **Per-subscription auth on WS channels** (17-AUTH/06-websocket): connection-level only.
- **`/api/auth/me`**, `POST /api/gateway/batch/flush`, `GET /api/gateway/ws`, `/api/bench/matrix`, `/api/isfr/stream`, `/ws/agents`, `/healthz`/`/readyz`, `/api/state/export|import`, `/api/cost/report`: documented/designed/frontend-called, not mounted.
- **WS back-pressure modes** Coalesce & ResumeRequired (`ws.rs:126-134`).
- **Multi-tenancy / TenantCtx** (v1 05:315): nothing tenant-scoped.
- **OTLP tracing**: config parsed, layer installation deferred — logs intent only (`lib.rs:2890-2909`).
- **Integration tests**: no `crates/roko-serve/tests/`; nothing exercises auth-scope per family, SSE/WS replay, or route contracts end-to-end.

## Migration checklist

- [ ] **[P0]** Unify signal substrate: make CLI runner write (or dual-write/migrate) `signals.jsonl` → `engrams.jsonl`, or point serve readers (`routes/status/gates.rs:84`, `status/episodes.rs:35`, `status/metrics.rs:119`, `projection_contract.rs:1602`, `research.rs:509`) at `layout.signals_path()` — verify: `curl -s localhost:6677/api/gates/summary | jq '.total'` ≥ 467-derived count; `curl -s localhost:6677/api/signals | jq 'length'` > 10
- [ ] **[P0]** Route HTTP plan execution through the real orchestrator (share `roko plan run` entrypoint via CliRuntime instead of `build_plan_execution_prompt`) — verify: `curl -X POST localhost:6677/api/plans/<id>/execute` then `.roko/state/executor.json` updates and `/api/plans/<id>/gates` shows per-task rungs
- [ ] **[P1]** Expand `required_scope_for` to cover all mutating families (jobs, run, dream, inference, deployments, bench, vision-loop, team, templates, connectors, feeds, subscriptions) and add per-family scope tests — verify: read-scoped key gets 403 on `POST /api/run`
- [ ] **[P1]** Fix dream HTTP: real review agent from `[dreams.agent]` config (not `cat`, `dream.rs:65`) and align journal writer/reader schema — verify: `curl -s localhost:6677/api/dream/journal | jq '.last_cycle'` non-empty after a cycle
- [ ] **[P1]** Auth story for `/relay/{*path}`: require agent token for non-GET or move under `/api` — verify: unauthenticated `POST /relay/topics/x` → 401
- [ ] **[P1]** Regenerate API docs from `build_router` (route manifest) and fix API-REFERENCE.md (~116 documented vs ~255 mounted; remove `/api/auth/me`); extend `openapi.rs` beyond 87 ops — verify: CI diff between extracted route list and docs is empty
- [ ] **[P2]** Pick one event vocabulary (or generate the bridge exhaustively so `_ => None` arms disappear, `lib.rs:1627,1810`) — verify: WS client receives CascadeRouterUpdated/Diagnosis events emitted on StateHub
- [ ] **[P2]** Implement or remove advertised WS modes (Coalesce/ResumeRequired) and add reconnect/replay/filter tests — verify: `cargo test -p roko-serve ws_` covers cursor + filter + lag
- [ ] **[P2]** Surface config-gated workers in `/api/status` (isfr_keeper, block_watcher, cold_archival, feed_agents, dream_loop: running|disabled|no-op-reason) — verify: `curl -s localhost:6677/api/status | jq '.workers'`
- [ ] **[P2]** Frontend parity fixes: add or alias `/api/bench/matrix`, `/api/isfr/stream` (SSE), `/api/share/{token}`, `/ws/agents` or update demo-app callers (66-FRONTEND-API-PARITY) — verify: demo-app network tab shows no 404s
- [ ] **[P2]** Reconcile agent lifecycle trackers (serve supervisor vs CLI agents.json) into one registry — verify: `roko agent stop` and `POST /api/agents/{id}/stop` agree on state
- [ ] **[P3]** Mark aliases canonical/compat (`/learn` vs `/learning`, `/prd` vs `/prds`, `/bench/run` vs `/runs`, `/roko-ws`); label synthetic aggregator payloads (`"synthetic": true`) — verify: catalog endpoint lists alias status
- [ ] **[P3]** Wire `start_dream_loop` into `roko serve` (or document daemon-only); unify `run_server_with_state` vs `start_background` task sets — verify: `roko serve` logs dream-loop startup when `auto_dream=true`
- [ ] **[P3]** Decide fate of ServeAuthConfig mixed defaults (serde field=false vs struct=true, `serve.rs:79-105`); make `roko init` output + docs consistent — verify: `roko config show | grep auth.enabled` matches docs
- [ ] **[P3]** Externalize hardcoded `NUNCHI_PRIVY_APP_ID` (`jwks.rs:14`) and anvil default wallet key (`lib.rs:356-358`) to config — verify: grep returns config-sourced values only

## Open questions

1. Which file is *supposed* to be canonical post-rename — `engrams.jsonl` (roko-fs says main) or `signals.jsonl` (CLI writes, CLAUDE.md calls it "Signal log")? A one-shot migration tool exists nowhere I could find.
2. Is public `GET /api/runs/{id}` (raw-ID transcript access, `shared_runs.rs:873`) an accepted trade-off, or should transcripts be token-only (`/api/shared/{token}`)?
3. Should `roko serve` == `roko daemon` for background loops (dreams, retention), or is the split intentional? Two startup paths (`start_background` vs `run_server_with_state`) currently encode a third, undocumented profile.
4. The aggregator's mirage-parity families (`/tasks`, `/predictions`, `/stats`) — keep feeding the legacy dashboard or retire with the v2 cell-manifest AgentCard (44-AGENT-SERVER P3)?
5. Is the 87-op OpenAPI meant to define the *supported/stable* subset (making the other ~170 routes internal), or is it just stale? An explicit stability tier would resolve both the CLAUDE.md "~85" confusion and the doc-drift complaint.
