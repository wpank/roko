# roko-agent-server — Per-Agent Sidecar
> Status-quo audit · verified 2026-07-08 · **HEAD 5852c93c05** · deep second pass (exhaustive re-read of all 15 src files + `agent_serve.rs` + roko-serve aggregator/heartbeats/agents; every route/line ref/drift claim re-checked at file:line, all hold) · sources: 28 files (14 src in crates/roko-agent-server + 1 tests file, 5 roko-cli, 3 roko-serve, 3 docs/v2*, .roko/GAPS.md, tmp/ux/ux-followup)
>
> **Live-vs-mirage tally (re-verified HEAD 5852c93c05):** 13 advertised routes; **7 carry live data** (`/health`, `/capabilities`, `/logs`, `/message`, `/predictions` ×3 in-memory-but-real), **2 mirage-shape (no producer)** (`/stats`, and the `stats` block inside `/capabilities`), **1 conditional/dead-by-default** (`/research` → stub string, CLI attaches no store), **3 dead** (`/tasks`, `/tasks/{id}/accept`, `/tasks/{id}/complete` — queue has no writer anywhere in the workspace). `/stream` is **half-live**: real for `LlmBackend`-backed state, degrades to blob-at-end under the CLI's `ServingAgentDispatcher` (no `dispatch_streaming` override). Default CLI deploy enables only `.messaging().predictions()` → of the 13, only 9 are even mounted.
> **Re-verification note (2026-07-08):** crate is 14 src files + `tests/relay_registration.rs` (2 e2e tests), ~4.0K LOC. Confirmed at file:line — `build_routes()` (`state.rs:855-889`) emits the 13 advertised paths; `roko agent serve` enables only `.messaging().predictions()` (`agent_serve.rs:245-246`), never `.auth()`; `ServingAgentDispatcher` (`agent_serve.rs:564-604`) implements **only `dispatch`** (real `Agent::run` + `extract_clean_text`), no `dispatch_streaming` override → `/stream` degrades to blob-at-end for CLI sidecars; `tasks: Mutex<VecDeque>` and `stats: Mutex<AgentRuntimeStats>` have **no writer** anywhere (`state.rs:654` only reads `.clone()`); heartbeat hardcodes task counters to 0 (`lib.rs:427-429`); `roko_learn` dep declared but **zero uses in src/**; raw `libc::kill` lifecycle (`agent_serve.rs:801,814`); serve aggregator is self-labelled "Mirage-compatible" (`aggregator.rs:1`).

## Summary

`roko-agent-server` (~3.6K LOC, 12 src files + 1 tests file) is a builder-pattern Axum server that gives every long-lived agent its own HTTP surface: 13 routes across 4 always-on + 4 feature-gated groups (`messaging`, `predictions`, `research`, `tasks`), optional SHA-256 bearer auth, an ERC-8004 agent-card registration path, an outbound `agent-relay` WebSocket bridge, and a 30s heartbeat loop to the roko-serve control plane. The CLAUDE.md claim "13 routes, real LLM dispatch (T9), integration tests (T19)" is **accurate but flattering**: `/message` genuinely reaches a real LLM (via `DispatchLike` → `LlmBackend::send_turn` or the provider-registry `Agent`), and the T19 messaging tests are real, but of the 13 routes only 7 carry live data. `/tasks` is an in-memory queue that **nothing in the workspace ever populates** (always `[]`, accept/complete always 404), `/stats` reads an `AgentRuntimeStats` mutex that **no code ever writes**, `/research` is a local knowledge-store lookup that the CLI never enables nor supplies a store for, and the heartbeat payload hardcodes all task counters to 0. The route shapes (predictions markets, task bounties, "confirmations_given") are mirage-compat by design (`state.rs:257 "shaped to resemble the legacy mirage agent stats"`) — a 🕰️ v1 dashboard-parity surface, not the v2 "agent cell" runtime. The v2 seed *is* present, but in roko-cli: `roko agent serve` tries to start the cognitive loop as a Hot Graph next to the HTTP server (`agent_serve.rs:385-428`) — with all 7 cells still `PassthroughCell` stubs (`.roko/GAPS.md:15-16`).

Lifecycle is split across **two untangled trackers**: `roko agent start` spawns a detached `roko agent serve` child and tracks its PID in `.roko/runtime/agents.json` with raw `libc::kill` (`agent_serve.rs:1064-1090, 799-819`), while roko-serve's `POST /api/agents/{id}/start` spawns the *same* command through `roko-runtime`'s ProcessSupervisor (`roko-serve/src/routes/agents.rs:869-892`). Relation to roko-serve is proxy/aggregation (correct layering, little code duplication): serve fans `/api/agents/*`, `/api/predictions/*`, `/api/tasks/*`, `/api/ws` out to sidecars over HTTP/WS (`aggregator.rs`), and `/api/agents/{id}/message` proxies to the sidecar WS-then-REST (`agents.rs:1214-1260`).

## Route census

All 13 advertised paths from `build_routes()` (`state.rs:855-889`); assembly in `lib.rs:78-114`.

| # | Route | Gate | Handler → backing | Verdict |
|---|---|---|---|---|
| 1 | `GET /health` | public | `features/health.rs:17` — status + uptime | ✅ real |
| 2 | `GET /capabilities` | public | `health.rs:26` → `capabilities_manifest()` (`state.rs:625-649`) | ✅ real (features/routes/skills/stats manifest) |
| 3 | `GET /stats` | protected core | `lib.rs:99` → `stats_payload()` (`state.rs:653-672`) | 🕰️ shape real, data dead — `stats: Mutex<AgentRuntimeStats>` (`state.rs:483`) has **no writer anywhere**; only `metrics` request/message counters (`state.rs:205-243`) are live |
| 4 | `GET /logs?tail=` | protected core | `features/logs.rs:41-78` — tails sidecar log file, `LogScrubber`, cap 2000 | ✅ real (log written by `dispatch_prompt`, `state.rs:591`) |
| 5 | `POST /message` | `messaging` | `messaging.rs:42-60` → `dispatch_prompt` (`state.rs:584-594`) → `DispatchLike` | ✅ **real LLM dispatch** (T9). Two impls: `BackendMessageDispatcher` → `LlmBackend::send_turn` (`state.rs:87-104`); CLI `ServingAgentDispatcher` → provider-registry `Agent::run` + `extract_clean_text` (`agent_serve.rs:570-604`). 503 if no dispatcher, 502 on backend error |
| 6 | `GET /stream` (WS) | `messaging` | `messaging.rs:62-256` → `dispatch_streaming`, frames `{chunk\|reasoning\|tool_call\|usage\|done}` | 🟡 real for `LlmBackend`-backed state (`send_turn_streaming` bridge `state.rs:106-146`); the CLI's `ServingAgentDispatcher` **does not override `dispatch_streaming`** → default falls back to blocking `dispatch` (`state.rs:65-72`), so `roko agent serve` sidecars emit no incremental chunks |
| 7 | `GET/POST /predictions` | `predictions` | `predictions.rs:16-36`, `state.rs:700-723` | 🟡 functional CRUD, **in-memory `Mutex<Vec>` only** — lost on restart; mirage-market shape |
| 8 | `GET /predictions/residuals` | `predictions` | `state.rs:742-779` — mse/hit_rate over actual vs predicted | 🟡 real math, same volatile store |
| 9 | `GET /predictions/{id}` | `predictions` | `predictions.rs:38-46` | 🟡 same |
| 10 | `POST /research` | `research` | `research.rs:14-19` → `state.research()` (`state.rs:783-817`) — top-5 `roko_neuro::KnowledgeStore.query()` | 🔌 real store lookup **if** a store is attached; CLI attaches none and never enables the feature → falls to stub string `"no local knowledge findings…"`. Not the Perplexity research agent |
| 11 | `GET /tasks` | `tasks` | `tasks.rs:23-25`, `state.rs:821-824` | ❌ dead queue — `tasks: Mutex<VecDeque<TaskEntry>>` (`state.rs:482`) has **no enqueue route and no writer in the entire workspace** (grep: only agent-server defines `TaskEntry`); always `[]` |
| 12 | `POST /tasks/{id}/accept` | `tasks` | `state.rs:828-835` | ❌ always 404 (queue empty) |
| 13 | `POST /tasks/{id}/complete` | `tasks` | `state.rs:839-852` (artifacts + summary) | ❌ always 404 |

**What `roko agent serve` actually exposes**: only `.messaging().predictions()` are enabled (`agent_serve.rs:241-246`) → 9 live routes; `research`/`tasks` are enabled only by tests/embedders. No `.auth()` is ever set in the CLI path → sidecars run unauthenticated on `127.0.0.1:0`. Non-route extras: heartbeat loop → `POST {serve}/api/heartbeats` every 30s (`lib.rs:150-160, 416-459`; `DEFAULT_HEARTBEAT_INTERVAL_SECS=30`, `roko-core/src/defaults.rs:202`) with `active_tasks/completed/failed` hardcoded 0; `engram_id` in the `/message` response is a fresh UUID (`messaging.rs:57`), not a persisted Signal.

## Route → producer table (live / mirage-shape / dead)

Every route with its **data producer** (the code that actually writes the bytes the handler serves) at file:line. "Mirage-shape" = struct/JSON shape is fully defined and the read path works, but *nothing in the workspace ever writes the backing store* (so it always serves defaults/zeros/empties). "Dead" = same, plus the mutation routes always 404 because the store is empty.

| # | Route (method) | Handler file:line | Backing store | **Producer** (writer) file:line | Verdict |
|---|---|---|---|---|---|
| 1 | `GET /health` | `features/health.rs:17` | `started_at: Instant` (`state.rs:472`) | set in `AgentState::new` `state.rs:511` | ✅ live |
| 2 | `GET /capabilities` | `features/health.rs:26` → `capabilities_manifest()` `state.rs:625` | `capabilities`+`routes` vecs | built at `build()` → `normalize_capabilities` `lib.rs:396`, `build_routes` `state.rs:855` | ✅ live (but embeds mirage `stats` block, row 3) |
| 3 | `GET /stats` | `features/health.rs:31` → `stats_payload()` `state.rs:653` | `stats: Mutex<AgentRuntimeStats>` `state.rs:483` | **NONE** — only reader is `state.rs:654 (.lock().clone())`; grep across workspace finds zero `stats.lock()` writes | 🕰️ mirage-shape (always zeros); only the nested `metrics` counters `state.rs:224` are live |
| 4 | `GET /logs?tail=` | `features/logs.rs:41` | sidecar log file | written by `append_log_line` on every dispatch `state.rs:544,591` | ✅ live |
| 5 | `POST /message` | `features/messaging.rs:42` → `dispatch_prompt` `state.rs:584` | `message_dispatcher: Arc<dyn DispatchLike>` `state.rs:477` | real LLM: `ServingAgentDispatcher::dispatch` `agent_serve.rs:570` (CLI) or `BackendMessageDispatcher::dispatch` `state.rs:87` | ✅ **live LLM** (503 if unset, 502 on backend err) |
| 6 | `GET /stream` (WS) | `features/messaging.rs:62,153` → `dispatch_streaming` | same dispatcher | `BackendMessageDispatcher::dispatch_streaming` `state.rs:106` (real). CLI `ServingAgentDispatcher` has **no override** → default `state.rs:65-72` calls blocking `dispatch` | 🟡 live for backend / blob-at-end for CLI sidecar |
| 7 | `GET/POST /predictions` | `features/predictions.rs:26,30` → `state.rs:700,720` | `predictions: Mutex<Vec>` `state.rs:481` | `create_prediction` `state.rs:700` (from `POST` body) | 🟡 live CRUD, in-memory only (lost on restart) |
| 8 | `GET /predictions/residuals` | `features/predictions.rs:48` → `state.rs:742` | same `Mutex<Vec>` | same producer (needs `actual_value` in the POST) | 🟡 real math, volatile store |
| 9 | `GET /predictions/{id}` | `features/predictions.rs:38` → `state.rs:727` | same | same | 🟡 live |
| 10 | `POST /research` | `features/research.rs:14` → `research()` `state.rs:783` | `knowledge_store: Option<Arc<KnowledgeStore>>` `state.rs:480` | store injected only via `.knowledge_store(...)` builder `lib.rs:271` — **CLI never calls it** (`agent_serve.rs:240-246` sets neither `.research()` nor a store) → falls to stub `state.rs:810` | 🔌 conditional; dead-by-default (stub string) |
| 11 | `GET /tasks` | `features/tasks.rs:23` → `list_tasks()` `state.rs:821` | `tasks: Mutex<VecDeque<TaskEntry>>` `state.rs:482` | **NONE** — no `POST /tasks`, no `push_back`, no enqueue anywhere; only mutators are accept/complete which need a pre-existing entry | ❌ dead (always `[]`) |
| 12 | `POST /tasks/{id}/accept` | `features/tasks.rs:27` → `accept_task()` `state.rs:828` | same VecDeque | mutates in place, but queue is always empty | ❌ dead (always 404) |
| 13 | `POST /tasks/{id}/complete` | `features/tasks.rs:37` → `complete_task()` `state.rs:839` | same VecDeque | same | ❌ dead (always 404) |

**Non-route producer of note — heartbeat:** `lib.rs:416` `heartbeat_loop` POSTs `roko_core::HeartbeatPayload` to `{serve}/api/heartbeats` every `DEFAULT_HEARTBEAT_INTERVAL_SECS` (30s), but hardcodes `active_tasks:0, completed_tasks:0, failed_tasks:0` (`lib.rs:427-429`) — so serve's `receive_heartbeat` (`roko-serve/src/routes/heartbeats.rs:21`) always stores zeros in its ring. The `/message` response `engram_id` is a throwaway UUID (`messaging.rs:57`), never a persisted Signal.

## Lifecycle trace: `roko agent serve` → boot → `/message` → response

```
roko agent serve --agent-id X --bind 127.0.0.1:0 --serve-url http://localhost:6677
  │
  ├─ agent_serve.rs:717  AgentCmd::Serve → AgentServeRuntimeConfig::from_args → .run()
  │
  ├─ run() agent_serve.rs:209
  │    ├─ build_server()  agent_serve.rs:240
  │    │    ├─ AgentServer::builder().agent_id().bind().serve_url()
  │    │    │        .messaging().predictions()          ← ONLY these 2 features (lib.rs:278,286)
  │    │    │        (no .research(), no .tasks(), no .auth())
  │    │    ├─ try_build_dispatcher()  agent_serve.rs:431
  │    │    │    ├─ load_config_unified(workdir)                    → agent.default_model
  │    │    │    ├─ if ANTHROPIC_API_KEY set & model not custom:
  │    │    │    │      override provider claude_cli → anthropic (direct HTTP)  :449-465
  │    │    │    ├─ spawn_agent_scoped(SpawnAgentSpec{model,...})   :485  (provider factory)
  │    │    │    └─ Some(ServingAgentDispatcher{ agent })          :507  ← dispatch-only impl
  │    │    ├─ .with_message_dispatcher(dispatcher)                 :249
  │    │    ├─ .registration(...) only if relay OR chain args given :252,512
  │    │    └─ .on_start(closure)                                   :258  (see boot below)
  │    ├─ try_start_cognitive_loop()  agent_serve.rs:385  → roko_graph::start_hot (1s tick, 7 stub cells)
  │    └─ server.serve().await   lib.rs:124
  │         ├─ TcpListener::bind → local_addr (real port)          lib.rs:126-131
  │         ├─ build_agent_card(local_addr)                        state.rs:676
  │         ├─ registration.register(...) if Some                  lib.rs:138 (relay/chain only)
  │         ├─ on_start(addr, card)  ── BOOT SIDE EFFECTS ──       agent_serve.rs:259
  │         │    ├─ POST {serve}/api/agents/register (3×2s retry)   :280-336  → serve DiscoveredAgent map
  │         │    ├─ upsert_agent_entry(.roko/runtime/agents.json)  :341,767  ← CLI-side registry (PID)
  │         │    └─ relay/chain: only logged "captured for later"  :343-367
  │         ├─ spawn heartbeat_loop → POST /api/heartbeats /30s     lib.rs:155,416
  │         └─ axum::serve(listener, router())                     lib.rs:162
  │
  └─ POST /message {"prompt":"..."}  ── REQUEST PATH ──
       ├─ messaging.rs:42  message()  (no auth layer in CLI path)
       ├─ state.dispatch_prompt(prompt)  state.rs:584
       │    ├─ metrics.record_message()  state.rs:585  (this IS live in /stats.metrics)
       │    ├─ message_dispatcher().ok_or(NotConfigured)? → 503 if none  state.rs:588
       │    └─ dispatcher.dispatch(chat_request(prompt,false))  state.rs:589
       │         └─ ServingAgentDispatcher::dispatch  agent_serve.rs:570
       │              ├─ extract_prompt(request)  :571,607
       │              ├─ Agent::run(Engram::Prompt, Context::now().with_goal)  :575
       │              ├─ extract_clean_text(raw)  :585  (strips Claude-CLI stream JSON)
       │              └─ ChatResponse{content, usage, finish_reason}  :587
       ├─ append_log_line("message prompt=... status=ok")  state.rs:591  → feeds /logs
       └─ 200 {response, reasoning, usage, session, finish_reason,
                engram_id: "engram-<uuid>" (fake), context}  messaging.rs:51-59
             DispatchError::NotConfigured → 503 ; DispatchFailed → 502  messaging.rs:93
```

## Lifecycle-tracker conflict diagram (two untangled trackers)

```
                    ┌───────────────────────────────────────────────┐
                    │            the same `roko agent serve`         │
                    │                    process                     │
                    └───────────────────────────────────────────────┘
                       ▲                                        ▲
          spawns +     │                                        │  spawns +
          tracks PID   │                                        │  supervises
                       │                                        │
   ┌───────────────────┴─────────────┐          ┌───────────────┴──────────────────┐
   │  TRACKER A — CLI (raw PID)       │          │  TRACKER B — serve (Supervisor)   │
   │  roko agent start/stop/list/status│         │  POST /api/agents/{id}/start|stop │
   │  agent_serve.rs:1028,1098,854,1175│         │  roko-serve/routes/agents.rs:800, │
   │                                  │          │                     1089          │
   │  store: .roko/runtime/agents.json│          │  store: ProcessSupervisor +       │
   │        (AgentEntry{name,pid,bind})│         │         DiscoveredAgent map        │
   │  liveness: libc::kill(pid,0)     │          │         (AppState)                 │
   │            agent_serve.rs:801    │          │  liveness: supervisor handle      │
   │  kill: libc::kill SIGTERM→SIGKILL│          │  spawn: supervisor.spawn(          │
   │        agent_serve.rs:812,1123   │          │         "roko agent serve", label)│
   │  stale-cleanup: YES (:1016-1021) │          │  stale-cleanup: n/a (supervised)  │
   └──────────────────────────────────┘          └───────────────────────────────────┘
                       │                                        │
                       └──────────────┬─────────────────────────┘
                                      ▼
             both feed the SAME on_start → POST /api/agents/register (agent_serve.rs:280)
             so serve's aggregator (aggregator.rs:1076 known_agents) merges
             discovered + supervised, but the two KILL paths never coordinate.

  CONFLICT: `roko agent start X` (Tracker A) is invisible to Tracker B's supervisor,
  and `POST /api/agents/X/stop` (Tracker B, kills by numeric PID) can race/disagree
  with `roko agent stop X` (Tracker A). No shared lock, no single source of truth.
```

## roko-serve aggregation map (how the control plane fans out to sidecars)

Serve does not duplicate sidecar logic — it proxies/aggregates over HTTP+WS (`roko-serve/src/routes/aggregator.rs`, self-labelled *"Mirage-compatible aggregation routes"* `aggregator.rs:1`). Discovery source = `known_agents()` `aggregator.rs:1076` (merges the registered `DiscoveredAgent` map + supervised procs); each carries an optional `proxy_token` used as bearer when proxying (`aggregator.rs:1002,1158`; `fetch_agent_json` `:1146`).

| Serve route | Handler file:line | Fans out to sidecar | Result quality |
|---|---|---|---|
| `GET /api/agents` | `aggregator.rs:148 list_agents` | `GET {sidecar}/health` + `/capabilities` (`hydrate_agent_card` `:1112`) | ✅ live |
| `GET /api/agents/{id}/stats` | `aggregator.rs:219 agent_stats` | `GET {sidecar}/stats` | 🕰️ mirrors sidecar mirage zeros |
| `GET /api/agents/{id}/skills` | `aggregator.rs:253` | `/capabilities.skills` | ✅ live |
| `GET /api/predictions/*` | `aggregator.rs:378,412 collect_agent_predictions :1043` | `GET {sidecar}/predictions` across all agents | 🟡 live-but-volatile |
| `GET /api/tasks`, `/api/tasks/stats` | `aggregator.rs:622,677 collect_agent_tasks :1054` | `GET {sidecar}/tasks` across all agents | ❌ permanently `[]` (no producer) |
| `GET /api/ws` (WS mux) | `aggregator.rs:724 handle_ws → forward_agent_stream :890` | opens `ws://{sidecar}/stream` per agent, muxes frames | 🟡 inherits `/stream` degradation |
| `POST /api/agents/{id}/message` | `agents.rs:1213` | tries sidecar WS then REST `/message`, else local `run` | ✅ live |
| `POST /api/agents/{id}/start\|stop` | `agents.rs:800,1089` | ProcessSupervisor spawn/kill (Tracker B) | ✅ live (parallel to CLI Tracker A) |
| `POST /api/heartbeats` | `heartbeats.rs:21 receive_heartbeat` | receives sidecar heartbeat into ring | 🕰️ stores hardcoded-zero task counters |

## Fix checklist — give `/tasks` + `/stats` real writers and fix `/stream`

Focused, verifiable steps for the three highest-value dead/mirage surfaces:

### A. `/stats` → real `AgentRuntimeStats` writer
- [ ] Add `AgentState::record_dispatch_result(&self, usage, cost, ok)` that locks `stats` (`state.rs:483`) and increments `tasks_completed`/`tasks_failed`, `total_tokens`, `total_cost_usd` from the `ChatResponse.usage` returned by `dispatch_prompt` (`state.rs:589`). Today nothing ever holds `stats.lock()` for a write (only reader is `state.rs:654`).
- [ ] Call it at the end of `dispatch_prompt` (`state.rs:584-594`) and in the WS `stream_prompt` completion arm (`messaging.rs:221`).
- [ ] Feed the same counters into `heartbeat_loop` (`lib.rs:424-433`) instead of literal `0`s, reading a snapshot of `state.stats`.
- Verify: `curl :$PORT/message -d '{"prompt":"hi"}'` then `curl :$PORT/stats` shows nonzero `total_tokens`/`tasks_completed`; `GET {serve}/api/heartbeats` shows nonzero `completed_tasks`.

### B. `/tasks` → real queue writer (pick one producer)
- [ ] Add `POST /tasks` enqueue route in `features/tasks.rs:16` + `AgentState::enqueue_task(TaskEntry)` doing `self.tasks.lock().push_back(..)` (`state.rs:482`). No such method exists today.
- [ ] Wire a producer: either (a) serve's job system (`roko-serve/src/routes/jobs.rs`) assigns → `POST {sidecar}/tasks`, or (b) the relay marketplace pushes via `relay_subscriber.rs`, or (c) the orchestrator `PlanRunner`. All three exist; none connect — decide the owner (Open question #2).
- [ ] Alternatively **drop** the `tasks` feature from `build_routes` (`state.rs:883`) + `capabilities` so `/capabilities` stops advertising dead routes.
- Verify: after assignment `curl :$PORT/tasks` is non-empty and `POST /tasks/{id}/accept` returns 200 (not 404); serve `/api/tasks` aggregates real entries (`aggregator.rs:1054`).

### C. `/stream` → make the CLI sidecar actually stream
- [ ] Implement `DispatchLike::dispatch_streaming` on `ServingAgentDispatcher` (`agent_serve.rs:569`) — currently it inherits the default (`state.rs:65-72`) which just calls blocking `dispatch`, so CLI sidecars emit a single blob at `done`. Either (a) build the sidecar dispatcher from an `LlmBackend` and reuse `BackendMessageDispatcher::dispatch_streaming` (`state.rs:106`, already correct), or (b) bridge `Agent::run`'s streaming variant into the `mpsc::UnboundedSender<StreamChunk>`.
- Verify: `websocat ws://127.0.0.1:$PORT/stream` (send a prompt) shows >1 `{"chunk":…}` frame *before* `{"done":true}`; serve's `/api/ws` mux (`aggregator.rs:890`) forwards incremental frames.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| AgentServer builder + feature flags | README.md §Feature flags | `lib.rs:41-114, 190-384` | ✅ wired | capability list drives router merge; overclaim test `lib.rs:556-608` |
| `/message` real dispatch (T9) | tmp/ux/ux-followup/07-spec-code-drift.md:45-52 (spec #05) | `messaging.rs:42`, `state.rs:584`, `agent_serve.rs:431-510` | ✅ wired | T9 commit `dcd06257`; ANTHROPIC_API_KEY → direct-API provider override `agent_serve.rs:449-465` |
| `/stream` WS streaming | docs/v2-depth/07-agent-runtime/chat-types-and-streaming.md (StreamChunk taxonomy) | `messaging.rs:153-256`, `state.rs:106-146` | 🟡 partial | chunk frames match doc; CLI dispatcher lacks streaming override → blob-at-end |
| Predictions store | mirage parity (README §Predictions) | `state.rs:481, 698-779` | 🕰️ legacy-v1-shape | in-memory, market/direction/confidence fields for old dashboard |
| Tasks queue | mirage parity (README §Tasks) | `state.rs:482, 819-852` | ❌ missing (producer) | no enqueue path in workspace; serve `/api/tasks` aggregates permanently-empty lists (`aggregator.rs:1054-1063`) |
| Research endpoint | README §Research ("citations and a synthesis") | `state.rs:783-817` | 🔌 built-not-wired | KnowledgeStore param exists (`lib.rs:269-274`) but CLI never passes one; roko-cli research agents (Perplexity/Gemini) not connected |
| `/stats` runtime stats | mirage parity | `state.rs:257-278, 653-672` | 🕰️ legacy-v1-shape | `AgentRuntimeStats` never mutated; canonical metric schema wrapper is real (`state.rs:224-242`) |
| Bearer auth | README ("bearer auth") | `auth/bearer.rs:12-66`, `lib.rs:82-89` | 🔌 built-not-wired | SHA-256 verify + middleware tested; CLI path never sets it; serve *can* issue/rotate proxy tokens (`roko-serve/src/routes/agents.rs:70,1655-1664`) and sends them when proxying (`aggregator.rs:1158-1160`) — unused end-to-end by default |
| ERC-8004 registration | Cargo.toml description; roko-chain | `registration.rs:26-250` (`updateAgentCardUri` calldata, DataUriPublisher) | 🟡 partial | full calldata builder + tests; **CLI never constructs a wallet** — `registration()` sets only registry+passport, `wallet_key` arg is dead (`agent_serve.rs:512-526, 168`) → on-chain path unreachable from CLI |
| Relay bridge (agent-relay) | apps/agent-relay protocol | `features/relay_client.rs:163-252`, `relay_subscriber.rs` (ISFR adapter) | ✅ wired | e2e test `tests/relay_registration.rs:157-222` hosts card + round-trips message through relay to `dispatch_prompt` |
| Heartbeat → control plane | README §Aggregation | `lib.rs:150-160, 416-459` → `roko-serve/src/routes/heartbeats.rs:17` → `agents.rs:80,350` | 🟡 partial | plumbing live; payload counters hardcoded 0 |
| Serve registration on start | — | `agent_serve.rs:280-341` (3×2s retry) + `.roko/runtime/agents.json` upsert (`:767-780`) | ✅ wired | discovery for `agent list`/`chat`/dashboard |
| Sidecar lifecycle (`agent start/stop/list/status`) | LIFE-01/LIFE-06 comments | `agent_serve.rs:1028-1222` (detached spawn, SIGTERM→SIGKILL, agents.json) | ✅ wired / 🕰️ | PID+`libc::kill`, **not** ProcessSupervisor |
| Serve-side lifecycle | — | `roko-serve/src/routes/agents.rs:806-901` (`supervisor.spawn`, label=agent_id) | ✅ wired | second, parallel tracker |
| `roko up` fleet boot | — | `commands/server.rs:6-135` (serve bg + `[[agents]]` create/start) | ✅ wired | uses agents.json path |
| `roko agent chat` routing | — | `chat_inline.rs:1170-1201, 4358-4380` | ✅ wired | sidecar-first (`agents.json` lookup → `POST {sidecar}/message`), falls back to serve `/api/agents/{id}/message` |
| Cognitive loop Hot Graph in sidecar | docs/v2/05-AGENT.md:3,13 (agent = Space+…+adaptive clock; loop as Hot Graph) | `agent_serve.rs:224, 385-428` (`roko_graph::start_hot`, 1s tick) | 🟡 partial | loads `examples/graphs/cognitive-loop.toml`; all 7 cells `PassthroughCell` stubs (`.roko/GAPS.md:15-16`, Task 103) |
| Agent-as-cell dispatch alignment | docs/v2-depth/07-agent-runtime/agent-cell-and-providers.md §4 | `agent_serve.rs:485-509` (`spawn_agent_scoped` → `create_agent_for_model`) | ✅ wired | sidecar dispatch goes through the unified provider factory, as designed |
| Integration tests (T19) | tmp/ux/ux-followup/00-INDEX.md:158, 04-t9-t19-residuals.md:70-74 (commit `c9029e20`) | `messaging.rs:259-523` (5 tests incl. real WS server), `logs.rs:98-274`, `lib.rs:467-608`, `tests/relay_registration.rs` (2 e2e), `roko-cli/tests/smoke.rs:364-377` | ✅ wired | README's "hit it via reqwest, every branch" slightly overstates: mostly tower `oneshot` + tokio-tungstenite |
| roko-learn dependency | — | `Cargo.toml:20` | 🔌 built-not-wired | declared, zero `roko_learn` usage in src/ |

## V2-aligned

- **Provider-factory dispatch**: the sidecar's dispatcher is built through `create_agent_for_model`/`spawn_agent_scoped`, exactly the "unified factory as Connect protocol" of `agent-cell-and-providers.md` §4 (`agent_serve.rs:485-509`).
- **Canonical chat types**: `/message`/`/stream` speak `ChatRequest`/`ChatResponse`/`FinishReason`/`StreamChunk` per `chat-types-and-streaming.md` (`messaging.rs:16-20, 105-144`).
- **Cognitive loop as Hot Graph** started alongside HTTP (`agent_serve.rs:385-428`) — the correct v2 shape (05-AGENT §1: "cognitive pipeline is itself a Hot Graph"), pending real cells.
- **Metric schema**: stats counters wrap `CanonicalMetricSchema` descriptors (`state.rs:224-255`, `roko-core/src/obs/schema.rs`).
- **`DispatchLike` seam**: a clean trait boundary that both mock tests and real backends implement (`state.rs:59-73`) — good Cell-ification candidate.

## Old paradigm & tech debt (duplication with roko-serve)

- 🕰️ **Mirage-parity surface**: `/stats` field set ("confirmations_given", "operating_frequency"), `/predictions` market shape, `/tasks` bounty shape all exist to feed the legacy mirage dashboard (`state.rs:257`, `aggregator.rs:1 "Mirage-compatible aggregation routes"`). Three of four feature groups carry no real production data.
- 🕰️ **Two HTTP layers, two lifecycle trackers**: sidecar registry lives in *both* `.roko/runtime/agents.json` (CLI, raw PID/kill, `agent_serve.rs:727-819`) *and* serve's `DiscoveredAgent` map + ProcessSupervisor (`agents.rs:56-71, 869-892`; `aggregator.rs:1076-1110` merges both). Stale-entry cleanup exists only on the CLI side (`agent_serve.rs:1016-1021`). Risk: `agent stop` (CLI) and `POST /agents/{id}/stop` (serve, numeric PID) can disagree.
- **Auth theater**: bearer middleware + serve token rotation both exist but the default deployment runs sidecars authless; `proxy_token` is only honored if someone registers an agent *with* one.
- **Dead knobs**: `--wallet-key` parsed, never used (`agent_serve.rs:168, 549-561`); `--relay-url` config only logged "captured for later hook-up" in `on_start` (`agent_serve.rs:343-349`) though it *is* passed to `registration()`; `llm_backend`/`knowledge_store`/`chain_client` builder slots unused by CLI; `roko-learn` dep unused.
- **Duplicated `ChatRequest` builder**: `message_request()` in `messaging.rs:105-124` ≡ `chat_request()` in `state.rs:182-201`.
- **Heartbeat lies**: fixed zeros for task counts (`lib.rs:424-433`) while serve renders "busy" from them.

## Not implemented

- ❌ Task ingestion: no `POST /tasks`, no serve→sidecar assignment; serve's job system (`routes/jobs.rs`) does not feed sidecar queues.
- ❌ Stats mutation: nothing increments `AgentRuntimeStats` (cost/tokens/tasks) from dispatch results.
- ❌ Prediction persistence (restart loses all; no `.roko/` file).
- ❌ Sidecar-side episode/signal write for `/message` turns (fake `engram_id`; orchestrate.rs episodes don't cover sidecar chat).
- ❌ Real cognitive-loop cells (GAPS.md Task 103) and heartbeat-as-gamma-Hot-Graph (`22-heartbeat-as-hot-graph.md` — plain tokio task today).
- ❌ CLI wallet construction for ERC-8004 on-chain registration.
- ❌ Streaming for the default CLI dispatcher (`ServingAgentDispatcher::dispatch_streaming` missing).
- ❌ Sidecar doc in docs/v1/02-agents (18 files, zero sidecar coverage) — crate exists only in README + CLAUDE.md.

## Migration checklist

- [ ] **[P0]** Unify lifecycle: make `roko agent start/stop` route through ProcessSupervisor (or serve API) instead of raw PID + `libc::kill`; single source of truth for the fleet roster — verify: `rg -n 'register_spawned_pid|libc::kill' crates/roko-cli/src/agent_serve.rs` returns nothing; `roko agent start x && curl :6677/api/managed-agents` shows supervised entry
- [ ] **[P0]** Implement `dispatch_streaming` on `ServingAgentDispatcher` (or build sidecar dispatcher from `LlmBackend`) so `/stream` streams — verify: `websocat ws://127.0.0.1:<port>/stream` shows >1 `{"chunk":…}` frame before `done`
- [ ] **[P1]** Wire `AgentRuntimeStats` writes (tasks_completed/failed, cost, tokens) from dispatch results + real heartbeat counters — verify: `curl :<port>/stats` shows nonzero `total_tokens` after a `/message`
- [ ] **[P1]** Either feed the tasks queue (serve assignment → sidecar enqueue route) or drop the `tasks` feature from cards — verify: `curl :<port>/tasks` non-empty after assignment, or route gone from `/capabilities`
- [ ] **[P1]** Enable `research` in `roko agent serve` with `KnowledgeStore::for_layout(...)` attached, or remove the feature — verify: `curl -XPOST :<port>/research -d '{"topic":"gates"}'` returns store-backed findings
- [ ] **[P2]** Default-on bearer auth: sidecar generates/receives token at registration, serve stores as `proxy_token` — verify: unauthenticated `POST /message` → 401; serve proxy still works
- [ ] **[P2]** Build `AlloyChainWallet` from `--wallet-key` and pass to `AgentRegistration.wallet` — verify: `roko agent serve --wallet-key … --identity-registry … --passport-id 7` submits tx (mock RPC)
- [ ] **[P2]** Replace the 7 `PassthroughCell` cognitive-loop cells with real impls (GAPS.md Task 103) — verify: `roko agent serve` logs real cell activity; `.roko/signals.jsonl` grows per tick
- [ ] **[P2]** Persist predictions + write `/message` turns as Episodes/Signals (real `engram_id`) — verify: restart sidecar, `GET /predictions` retains entries; `rg <engram_id> .roko/`
- [ ] **[P3]** Decide fate of mirage-compat shapes (stats/predictions/tasks fields) once the new dashboard lands; align `AgentCard` with v2 cell manifest — verify: aggregator tests updated, demo-app renders
- [ ] **[P3]** Drop unused `roko-learn` dep; dedupe `chat_request` builders; document the sidecar in docs (02-agents has no sidecar page) — verify: `cargo udeps -p roko-agent-server` clean

## Open questions

1. **Is the sidecar the v2 agent cell runtime or a temporary shell?** v2 05-AGENT wants Agent = Space + three Hot Graphs interpreted by the Engine; today the sidecar is HTTP-first with a stub Hot Graph bolted on. Does the HTTP surface become a thin Observe/Connect layer over an Engine-hosted agent, or stay a parallel structure?
2. Who is the intended producer for the sidecar task queue — serve's jobs system, the orchestrator's PlanRunner, or the relay marketplace? All three exist; none connect.
3. Are predictions (`market`/`direction`) still a product requirement (trading heritage) or should the feature be retired for coding agents?
4. Should sidecar `/message` turns feed the learning loop (episodes, efficiency events, CascadeRouter outcomes)? Currently `roko agent chat` traffic is invisible to `.roko/learn/`.
5. The relay path passes `topic_handler: None` at registration ("pub/sub wiring happens in the ISFR keeper (C2)", `registration.rs:163`) — is general-purpose agent pub/sub planned, or ISFR-only?
