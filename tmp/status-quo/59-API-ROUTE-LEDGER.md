# API Route Ledger

> Status-quo ledger · re-verified 2026-07-08 @ HEAD 5852c93c05 (main) · scope: `crates/roko-serve/` (109 `.rs` files, 45 declare routes) · companion to deep audit **46-SERVE-HTTP-REALTIME.md** (read that for realtime/StateHub/auth internals; this ledger is the route-by-route maturity table + drift catalog).

This ledger separates route facts that previous docs blended together. It answers three questions per namespace: **(a) is the route mounted? (b) what data source backs the handler? (c) is that data real, parity-shaped/synthetic, or a stub?**

## Counts (re-counted 2026-07-08)

| Count | Value | Meaning |
|---|---:|---|
| `roko-serve` raw `.route(...)` declarations | **288** | Grep count under `crates/roko-serve/src` at HEAD 5852c93c05; includes aliases, `#[cfg(test)]` routers, top-level routes, nested routers. |
| — of which inside `#[cfg(test)]` | **~28** | Not mounted at runtime. Includes 7 mock-sidecar routes in `aggregator.rs:1587-1593`, terminal test scaffolding, scope-test apps in `middleware.rs`. |
| Production `.route(...)` registrations | **~260** | The mounted surface (terminal mounts one of two variants → net ~255 distinct paths). |
| Method-operations (prod) | **~280** | A route with `get(...).post(...)` counts as 2 ops. |
| Documented in `/openapi.json` | **87** | `openapi.rs:53-141` — covers ~⅓ of the surface. This is the number CLAUDE.md calls "~85 routes". |
| Whole-workspace raw `.route(...)` | **374** | Adds `roko-agent-server`, `agent-relay`, `mirage-rs`, CLI worker/auth test routers, tests. |
| Route-declaring modules under `routes/` | 45 files | Plus `terminal.rs`, `openapi.rs` at crate root. |

**Do not use raw route count as a maturity metric.** Use the maturity ledger below, route-contract tests, and frontend caller coverage.

## Namespace census with maturity tag

Method: `grep -c '.route('` per file (2026-07-08) cross-referenced with handler bodies and on-disk data. Prod counts exclude `#[cfg(test)]` blocks.

Maturity legend: **REAL** = handler reads/writes a live producer · **MIRAGE** = parity-shaped route whose producer never fills (empty-by-inheritance or hardcoded `[]`) · **DEGRADED** = real handler pointed at the wrong/near-empty data file · **STUB** = placeholder response · **PROXY** = fan-out to an upstream that may be 503 · **SEMANTIC-GAP** = mounted + returns data but runs a different pipeline than the name implies.

| Namespace (mount) | File | Prod routes | Maturity | Data source → drift |
|---|---|---:|---|---|
| status / dashboard / metrics / statehub (`/api`) | `routes/status/mod.rs` (27) + submodules | 27 | **DEGRADED** | `/status` `/dashboard` real; `/gates/*` `/signals` `/episodes` + `metrics.rs:119` all read `.roko/engrams.jsonl` (**10 lines, 0 GateVerdicts**) while 467 verdicts sit in `signals.jsonl` — see storage split. `/statehub/snapshot|events` real (RuntimeProjectionSet). |
| learning (`/api/learn*` + `/api/learning*`) | `routes/learning/mod.rs` | 25 | **REAL** | 11 datasets each aliased twice; reads `.roko/learn/*` via `projection_contract.rs:1590-1634`. Alias sprawl only. |
| aggregator (`/api`) | `routes/aggregator.rs` (25 raw) | 18 | **MIRAGE-mixed** | `/agents`, `/agents/{id}/stats|skills|heartbeat` real; **`/agents/{id}/trace` hardcoded `"items":[], "total":0` (`aggregator.rs:315-322`)**; `/tasks`, `/predictions/*` empty-by-inheritance (sidecar queues never fill — see 44); `/agents/topology` synthesizes edges (`:325+`); `/knowledge/*` real (neuro). 7 mock routes at `:1587-1593` are test-only. `GET /api/ws` **is** mounted here (corrects earlier "Mirage-only" claim). |
| bench (`/api/bench`) | `routes/bench.rs` | 19 | **REAL** | run/suites/models/pareto/export/SSE; strategies Minimal…FullCascade dispatch real LLMs, `Demo` simulated (`bench.rs:147-162`). **No `/bench/matrix`** (frontend calls it). |
| plans (`/api/plans`) | `routes/plans.rs` | 15 | **SEMANTIC-GAP** | CRUD/status/gates/costs/reviews/diff/chat/estimate real; **`POST /plans/{id}/execute` flattens plan → prompt → `runtime.run_once` (`plans.rs:206,223`, `build_plan_execution_prompt:1432`)** — NOT the roko-orchestrator DAG+per-task-gates path `roko plan run` uses. `/plans/{id}/gates` then reads verdicts this route never produced. |
| agents (`/api/agents`) | `routes/agents.rs` | 13 | **REAL** | create/start/stop/restart via ProcessSupervisor (`:806-901`); `/message` proxies to sidecar WS `/stream` w/ POST fallback + `proxy_token`; token issue/rotate (`:1655+`). Second lifecycle tracker parallel to CLI `agents.json` (see 44). |
| jobs (`/api/jobs`) | `routes/jobs.rs` | 10 | **REAL/local** | Marketplace CRUD + lifecycle; `job_runner.rs` auto-executes via `runtime.run_once` with stale-lock reclaim. JSON-on-disk, no chain settlement. |
| workflows + workflow (`/api`) | `routes/workflows.rs` (7) + `/workflow/events` (`mod.rs:191`) | 8 | **REAL** | Snapshots from `.roko/events.jsonl`; per-workflow SSE; `/workflow/ws`. |
| prds (`/api/prds`) | `routes/prds.rs` | 9 | **REAL** | idea/draft/promote/plan; publish triggers auto-plan (`prds.rs:159-262`, config-gated `serve.auto_orchestrate && prd.auto_plan`). |
| chain (`/api/chain`) | `routes/chain.rs` | 7 | **REAL/live-optional** | Reads in-memory ring buffers fed by block watcher; **empty without RPC** — no synthetic marker. |
| research (`/api/research`) | `routes/research.rs` | 6 | **REAL/DEGRADED** | topic/enhance-*/analyze real; `analyze` reads `engrams.jsonl` (`:509`) so it inherits the storage-split gap. |
| deployments (`/api/deployments`) | `routes/deployments.rs` | 5 (8 ops) | **REAL** | File-persisted; task proxy + callback. |
| gateway (`/api`) | `routes/gateway.rs` | 5 | **REAL/design-drift** | `POST /inference/complete` real (cached CascadeRouter + provider dispatch + cost/health). Paths & scope drift from `docs/v2/08-GATEWAY.md` (no 9-cell pipeline, no `/gateway/inference`, no `/batch/flush`, no `/gateway/ws`). |
| feeds (`/api/feeds`) | `routes/feeds.rs` | 5 | **REAL** | Registry (seeded `lib.rs:1025-1138`) + 29 runtime feed agents. |
| subscriptions (`/api/subscriptions`) | `routes/subscriptions.rs` | 5 (7 ops) | **REAL** | Config-backed CRUD/enable/disable. |
| rpc proxy (`/api/rpc`) | `routes/rpc_proxy.rs` | 5 | **PROXY** | Mirage/anvil reverse proxy. |
| providers/models/routing (`/api/{providers,models,routing}`) | `routes/providers.rs` | 5 | **REAL** | Nested routers (`mod.rs:186-188`). |
| isfr (`/api/isfr`) | `routes/isfr.rs` | 4 | **REAL/live-optional** | status/current/history/sources from keeper (4-source mock fallback). **No `/isfr/stream`** (frontend calls it). |
| secrets (`/api/secrets`) | `routes/secrets.rs` | 4 | **REAL** | admin-scoped set/delete/test. |
| shared runs | `routes/shared_runs.rs` | 4 | **REAL** | `POST /api/runs/{id}/share` authed; `GET /api/runs/{id}`, `GET /api/shared/{token}`, `GET /runs/{id}` **intentionally public** (`:865-876`). |
| relay proxy (top-level `/relay`) | `routes/relay_proxy.rs` | 4 | **PROXY/unauthed** | WS bridges + `ANY /relay/{*path}` catch-all mounted **outside auth** (`mod.rs:248`); 503 when `ROKO_AGENT_RELAY_URL` unset. Proxies **writes** with no auth. |
| team (`/api/team`) | `routes/team.rs` | 4 (5 ops) | **REAL/no-enforcement** | me/members/invite/role-update persisted; **no middleware role enforcement** (roles stored, never checked — see auth). |
| swe-bench (`/api/bench/swe`) | `routes/swe_bench.rs` | 4 | **REAL** | Spawns real runs; datasets list. |
| config (`/api/config`) | `routes/config.rs` | 3 (4 ops) | **REAL** | get/put/toml/reload, admin-scoped. |
| connectors (`/api/connectors`) | `routes/connectors.rs` | 3 | **DEGRADED** | In-memory registry seeded w/ 2 defaults so lists aren't empty (`lib.rs:1035-1075`); lost on restart. |
| projections (`/api/projections`) | `routes/projections.rs` | 3 | **REAL/partial** | catalog/get/stream via RuntimeProjectionSet (~15 projections); v1 names `cohort_health`/`active_tasks`/`alerts` return **placeholder shapes** (`:625-655`). |
| templates (`/api/templates`) | `routes/templates.rs` | 3 (5 ops) | **REAL** | Agent-template CRUD + deploy. |
| vision-loop (`/api/vision-loop`) | `routes/vision_loop.rs` | 3 | **REAL** | Spawns `roko vision-loop` subprocess. |
| webhooks | `routes/webhooks.rs` | 3 | **REAL** | `/webhooks/github`, `/webhooks/slack` public + signature-verified; `/webhooks/generic` authed. |
| workspaces (`/api/workspaces`) | `routes/workspaces.rs` | 3 (5 ops) | **REAL** | Ephemeral registry, persisted; GC'd hourly. |
| api-keys (`/api/api-keys`) | `routes/auth.rs` | 2 (3 ops) | **REAL** | list/create/delete named keys. **No `/api/auth/me`** despite API-REFERENCE.md documenting it. |
| dream (`/api/dream`) | `routes/dream.rs` | 2 | **DEGRADED** | `POST /dream/run` runs DreamRunner but with **`command: "cat"` as review agent (`dream.rs:65`)**; `GET /dream/journal` reader expects `timestamp`/`phases[]` the writer never emits → `last_cycle` always `""`. No dream loop under `roko serve` (daemon-only). |
| event ingest (`/api/events/ingest`) | `routes/event_ingest.rs` | 2 | **REAL** | Single+batch RuntimeEvent ingest; loopback/token/allowlist gating. |
| heartbeats (`/api`) | `routes/heartbeats.rs` | 2 | **DEGRADED** | Sidecar heartbeat sink + `/network/stats`; sidecar payload counters hardcoded 0 (see 44). |
| integrations (`/api/integrations`) | `routes/integrations.rs` | 2 | **REAL** | Read-only status. |
| neuro (`/api`) | `routes/neuro.rs` | 2 | **REAL** | `POST /neuro/query` + `GET /knowledge` hit real `roko_neuro::KnowledgeStore`. |
| run (`/api/run`) | `routes/run.rs` | 2 | **REAL** | `POST /run` → background `runtime.run_once` (universal loop) w/ StateHub events; `/run/{id}/status`. |
| sse (`/api/events`, `/api/sse`) | `routes/sse.rs` | 2 | **REAL** | Both aliases → `sse_handler` (DashboardEvent stream, `Last-Event-ID` replay ≤256, 8s keepalive). |
| ws (top-level `/ws`, `/roko-ws`) | `routes/ws.rs` | 2 | **REAL/partial** | Both aliases → `ws_upgrade` (ServerEvent stream, cursor catchup); Coalesce/ResumeRequired BP modes rejected (`ws.rs:126-134`); no query/publish frames. |
| dashboard runs (`/api/dashboard/runs`) | `routes/runs.rs` | 1 | **REAL** | Reads `.roko/runtime-events.jsonl`. |
| diagnosis (`/api/diagnosis/recent`) | `routes/diagnosis.rs` | 1 | **REAL** | Conductor diagnosis log. |
| openapi (`/api/openapi.json`) | `openapi.rs` | 1 | **REAL/partial** | utoipa spec, 87 ops only. |
| top-level probes | `routes/mod.rs:235-239` | 3 | **REAL/public** | `/health`, `/ready` (shutdown-aware), `/metrics` (Prometheus) — no auth, no `/api`. |
| workflow SSE (`/api/workflow/events`) | `routes/mod.rs:191` | 1 | **REAL** | RuntimeEvent-typed SSE via SseAdapter. |
| terminal (top-level) | `terminal.rs:1120-1151` | 5 (or 4 disabled) | **REAL/gated** | PTY sessions + `/ws/terminal/{id}`; disabled by default → 403; loopback allowed w/o auth, public bind requires key. 9 raw registrations = enabled(5)+disabled(4) variants. |

## Storage-split cross-check (the navigation-layer's biggest data drift)

Three JSONL substrates, and dashboards read the wrong one:

| File | On disk (2026-07-08) | Written by | Read by serve |
|---|---:|---|---|
| `.roko/signals.jsonl` | **467 lines / 467 GateVerdicts** | CLI runner (`layout.signals_path()`) | **Nobody in serve.** |
| `.roko/engrams.jsonl` | **10 lines / 0 GateVerdicts** | (post-rename target, writer never migrated) | `/api/gates/*`, `/api/signals`, `/api/episodes`, metrics, research-analyze, projections |
| `.roko/events.jsonl` | 44 MB | runtime event bus | gate views merge `gate.completed` rows here as accidental mitigation |

Net effect: **gate/signal panels render, but from `events.jsonl` fallback + a 10-line engrams file — canonical verdicts (lineage, attestation, rung detail) are invisible to every dashboard, projection, and the research-analyze pass.** `roko-fs/src/layout.rs:202-217` marks `signals.jsonl` "legacy"; `projection_contract.rs:526` calls the records "Legacy signal/engram records" — the rename was decided but never carried through the writer. Fix = either dual-write/migrate CLI → `engrams.jsonl`, or repoint all 6 serve readers at `layout.signals_path()`.

## Realtime & projection reality (see 46 for depth)

- **SSE**: `/api/events` and `/api/sse` are aliases of one `sse_handler` (`sse.rs:23-24`) streaming `DashboardEvent` with `Last-Event-ID` replay (cap 256), 8s keepalive, proxy-buster headers. Real.
- **WS**: `/ws` and `/roko-ws` alias one `ws_upgrade` (`ws.rs:24-25`) streaming `ServerEvent`; **third alias `GET /api/ws` lives in aggregator** (not ws.rs). Cursor catchup works; back-pressure modes Coalesce/ResumeRequired are advertised but rejected; no query/publish frames (v1-06 unbuilt).
- **StateHub**: `/api/statehub/snapshot|events` + `/api/projections/*` expose RuntimeProjectionSet (~15 projections + aliases, ring 1024). Bridged by `DashboardEventBridge` (`lib.rs:569-610`) mapping RuntimeEvents→DashboardEvents. **Three event vocabularies** (ServerEvent/WS, DashboardEvent/SSE, SseEvent kinds) with lossy `_ => None` bridge arms (`lib.rs`) — WS clients never see Diagnosis/CascadeRouterUpdated/ExperimentWinners.

## Auth model (verified `routes/middleware.rs` 2026-07-08)

- **Four auth paths**: single legacy API key, named SHA-256 keys (scopes+expiry), agent bearer tokens (issue/rotate, `token_hash` match), Privy JWT (JWKS cache, hardcoded app id `cmhw01vut003tjx0d5lmqc8zs` @ `jwks.rs:14`).
- **Scope enforcement is a coarse prefix matrix** (`required_scope_for` `middleware.rs:356-386`): `/api/{api-keys,secrets,config}`→`admin`; `/api/events/ingest`→`agent:write`; `/api/agents`→`agent:write`; `/api/{plans,prd}`→`plan:write`; `/api/workspaces`→`write`; **everything else falls through to `"read"` (`:385`)**. And `is_scope_sufficient` (`:389-397`) passes any request whose required scope is `"read"`. **Consequence: a read-scoped key can `POST /api/run`, `/api/jobs`, `/api/dream/run`, `/api/inference/complete`, `/api/deployments`, `/api/bench/run`, `/api/vision-loop`, `/api/templates`, `/api/connectors`, `/api/feeds`, `/api/subscriptions`, `/api/team`.**
- **No role-based authorization** (Owner/Admin/Member/Viewer per v2 17-AUTH is unbuilt); `/api/team` stores roles but no middleware checks them.
- **Unauthenticated write surface**: `/relay/{*path}` (mounted at `mod.rs:248`, outside `/api`) proxies writes with no auth. `GET /api/runs/{id}` exposes full transcripts by guessable run-id (deliberate but ID-guessable). `/metrics` public.
- **Auth default is mixed**: struct `Default { enabled: true }` but `#[serde(default)]` → false when `[serve.auth]` present without `enabled`; `roko init` writes `false`; auto-enables on public bind with stored Privy cred.

## Known Frontend Mismatches

| Frontend call | Server reality | Action |
|---|---|---|
| `GET /api/share/{token}` (`demo/demo-app/src/pages/Share.tsx`) | serve exposes `GET /api/shared/{token}` + `GET /api/runs/{id}` | Rename FE call or add compat alias. |
| `POST /api/bench/matrix` (`hooks/useMatrixBench.ts`) | bench has run/runs/suites/models/pareto/export/events; **no matrix** | Implement or remove UI path. |
| `EventSource /api/isfr/stream` (`lib/isfr-api.ts`) | isfr has status/current/history/sources; **no stream** | Add ISFR SSE or point at `/api/events`. |
| `WS /ws/agents` (`pages/isfr/IsfrTabDrawer.tsx`) | serve has `/ws`, `/roko-ws`, `/api/ws` (aggregator), `/api/workflow/ws`, relay WS | Move to existing WS or add route. |
| Mirage `/api/ws` | Mirage app route, distinct from serve's `/api/ws` (aggregator) | Keep Mirage docs separate; note serve now also has `/api/ws`. |
| `/relay/*` helper types | Drift from actual `agent-relay` response shapes | Freeze relay shapes before treating serve proxy as canonical. |

## API documentation drift

- `/openapi.json` (87 ops) omits bench, chain, feeds, relay/RPC proxy, workspaces, workflows, secrets, connectors, team, api-keys, terminal, event-ingest, swe-bench, vision-loop — **>150 mounted routes undocumented**.
- API-REFERENCE.md documents ~116 endpoints incl. **nonexistent `/api/auth/me`**; ~255 mounted → drift both directions.
- Metrics vocabulary is split: top-level `/metrics` (Prometheus), `/api/metrics/prometheus`, JSON `/api/metrics` — docs conflate them.
- Aggregator returns empty/synthetic read models (`/agents/{id}/trace` hardcoded `[]`, topology edges synthesized, `/tasks` `/predictions` empty-by-inheritance) with **no `"synthetic": true` marker**.

## Ordered roadmap (P0 → P3)

1. **[P0] Unify signal substrate.** Repoint the 6 serve readers (`status/gates.rs:84`, `status/episodes.rs:35`, `status/metrics.rs:119`, `research.rs:509`, `projection_contract.rs:1602`) at `layout.signals_path()`, OR dual-write CLI → `engrams.jsonl`. Verify: `curl /api/signals | jq length` > 10; `/api/gates/summary` reflects 467 verdicts.
2. **[P0] Route HTTP plan-execute through the real orchestrator.** Replace `build_plan_execution_prompt` prompt-flattening (`plans.rs:1432`) with the `roko plan run` DAG+gates entrypoint via CliRuntime. Verify: `POST /api/plans/{id}/execute` updates `.roko/state/executor.json` and `/api/plans/{id}/gates` shows per-task rungs.
3. **[P1] Expand scope matrix.** Add every mutating family (jobs, run, dream, inference, deployments, bench, vision-loop, team, templates, connectors, feeds, subscriptions) to `required_scope_for`; add per-family 403 tests. Verify: read-scoped key → 403 on `POST /api/run`.
4. **[P1] Auth for `/relay/{*path}`.** Require agent token for non-GET or move under `/api`. Verify: unauthed `POST /relay/topics/x` → 401.
5. **[P1] Fix dream HTTP.** Real review agent from `[dreams.agent]` config (not `cat`, `dream.rs:65`); align journal writer/reader schema. Verify: `/api/dream/journal | jq .last_cycle` non-empty after a cycle.
6. **[P1] Regenerate API docs from `build_router`.** Route-manifest generator + CI diff vs API-REFERENCE.md; remove `/api/auth/me`; extend `openapi.rs` past 87 ops.
7. **[P2] Label synthetic/empty aggregator payloads** (`"synthetic": true` / `"unimplemented": true`) on `/agents/{id}/trace`, topology, `/tasks`, `/predictions/*`.
8. **[P2] Mark route aliases** canonical/compat/deprecated (`/learn` vs `/learning`, `/prd` vs `/prds`, `/bench/run` vs `/runs`, `/ws` vs `/roko-ws` vs `/api/ws`, `/events` vs `/sse`).
9. **[P2] Frontend parity**: add/alias `/api/bench/matrix`, `/api/isfr/stream` (SSE), `/api/share/{token}`, `/ws/agents` — or fix demo-app callers. Verify: demo network tab shows no 404s.
10. **[P2] Pick one event vocabulary** or make the WS/SSE bridge exhaustive so `_ => None` arms disappear. Verify: WS client receives CascadeRouterUpdated/Diagnosis.
11. **[P2] Surface config-gated workers** (isfr_keeper, block_watcher, cold_archival, feed_agents, dream_loop) in `/api/status`.
12. **[P3] Reconcile agent lifecycle trackers** (serve supervisor vs CLI `agents.json`).
13. **[P3] Externalize hardcoded tenant/wallet** (`jwks.rs:14` Privy app id, anvil default key) to config.
14. **[P3] Split Mirage API docs** from serve docs; keep OpenAPI in sync with the generated route manifest.

## Route contract checklist

- [ ] Generate a route manifest from `build_router` (`routes/mod.rs:131-263`) route assembly.
- [ ] Generate a frontend caller manifest from `demo/demo-app/src`.
- [ ] Fail CI when a frontend call has no matching server route.
- [ ] Tag every route `canonical` / `compat` / `deprecated` in the manifest.
- [ ] Add auth-scope 403 tests per route family (currently only agents/plans/secrets/workspaces covered in `middleware.rs` tests).
- [ ] Add SSE/WS tests for reconnect, replay, filter, authorization (none exist).
- [ ] Mark empty/synthetic/unimplemented responses explicitly in JSON + docs.
- [ ] Keep `/openapi.json` in sync with the route manifest (currently 87 of ~280 ops).
