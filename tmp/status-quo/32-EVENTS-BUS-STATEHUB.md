# Events, Bus, Relay, Feeds, Triggers, StateHub

> Status-quo audit · verified 2026-07-08 @ HEAD `5852c93c05` · supersedes concise draft (claims kept where correct, corrected where not) · re-verified against current code: serve `EventBus` thin-delegate (`event_bus.rs:1-12` header), StateHub canonical in roko-runtime, `roko-core/src/state_hub.rs` orphan (absent from `roko-core/src/lib.rs` module tree — only `dashboard_snapshot` declared at `lib.rs:83`), 28 `impl FeedAgent` blocks / `spawn_all` comment "29 agents" (`feed_agents/mod.rs:101,108`), both bridges + `BridgeDedup` (`lib.rs:338-341,1261`), SSE `last_event_id`/WS `replay_cursor` replay (`routes/sse.rs:52`, `routes/ws.rs:89-140`), `dispatch_loop` at `dispatch.rs:1464` matching `ServerEvent::WebhookReceived` (`:1512`), `RokoEvent::{PlanRevision,PrdPublished}` at `event_bus.rs:116,118,141`, colon relay topics (`isfr_feed.rs:125-151`). ZERO hits for Group/Space/CoordinationMode, TriggerBinding/TriggerProtocol, and telemetry Lens Cells (all confirmed absent). · sources: ~60 files across `apps/agent-relay`, `apps/roko-chain-watcher`, `apps/mirage-rs`, `roko-serve`, `roko-runtime`, `roko-core`, `roko-agent-server`, `roko-orchestrator`, `roko-cli`, `dev.sh`; specs `docs/v2/{09,10,11,13,15}`, `docs/v2-depth/06-trigger-system`, `docs/v2-depth/09-telemetry/01-observability-as-lens-pipeline.md`, `docs/v2-depth/12-connectivity/01–05`, `docs/v1/12-interfaces/22`; `tmp/relay-bus/{01–05}` (topic-grammar DECISION now landed in `04`); siblings `70-RELAY-PROTOCOL-FREEZE.md`, `46-SERVE-HTTP-REALTIME.md`, `31-GRAPH-CELLS-ENGINE.md`, `44-AGENT-SERVER.md`.

Legend: ✅ wired · 🔌 built-not-wired · 🟡 partial · ❌ absent · 🕰️ old paradigm / dead code

## Summary

The **distributed/event layer is real but three-layered and only partially spec-conformant**. (1) The relay (`apps/agent-relay`) is a working standalone WS pub/sub with per-topic rings, global seq, replay-on-subscribe, feed registry, workspace directory, and request/response bridging — it matches depth doc `12-connectivity/01` closely, with the exact residuals listed in `70-RELAY-PROTOCOL-FREEZE.md` still open (`resume_after`, batch subscribe, outbound timestamps, dot-topics). (2) In-process, `roko-runtime::EventBus` is the single ring+broadcast implementation; `roko-serve::EventBus` is a thin delegating wrapper (post-Task-104) and `StateHub` (canonical in roko-runtime, orphan copy in roko-core) is the dashboard source of truth with bidirectional, dedup-guarded, *lossy* bridges between `ServerEvent` and `DashboardEvent`. (3) Feeds are wired far beyond the draft's knowledge: **29 FeedAgents** spawn at serve startup and bridge to the relay; `ISFRFeed` bridges relay topics back onto a local `Bus` as Pulses. Triggers exist as a **subscription/dispatch system** (cron/file-watch/webhook → `dispatch_loop`) — the v2 Trigger protocol (arm/disarm, `.roko/triggers/`, 7 kinds, `roko trigger` CLI) is unimplemented. **Groups are entirely absent.** Pheromones/stigmergy exist in two disconnected fragments: test-only types in roko-orchestrator, and a live on-chain loop in the mirage-rs/roko-chain-watcher demo stack.

## Event flow map (publisher → channel → consumer)

**In-process (roko-serve, port 6677):**

1. Route handlers, feed agents, dispatch, bench/jobs/deployments → `EventBus<ServerEvent>` (`roko-serve/src/event_bus.rs:25` thin wrapper → `roko_runtime::event_bus::EventBus`; publish call sites e.g. `routes/plans.rs:327`, `routes/jobs.rs:835`, `routes/heartbeats.rs:25`) → consumers:
   - **SSE** `routes/sse.rs:51-60` (replay via `replay_from`, capped at 256, then live) and **WS** `routes/ws.rs:96-140` (`replay_from(replay_cursor)` + live + catch-up on reconnect).
   - **`dispatch_loop`** `roko-serve/src/dispatch.rs:1464` — matches `ServerEvent::WebhookReceived` signals against `SubscriptionRegistry` → spawns `dispatch_agent` (the trigger executor).
   - **Bridge A** `start_state_hub_bridge` `lib.rs:1320` → `server_event_to_dashboard` `lib.rs:1346` (lossy: `_ => None` at `lib.rs:1423,1627`) → StateHub.
2. Orchestrator (plan runner) → **StateHub** (`roko-runtime/src/state_hub.rs:80`; `DashboardEvent`, 44 variants in `roko-core/src/dashboard_snapshot.rs:25`) → consumers:
   - watch-channel snapshot → TUI; broadcast → **Bridge B** `start_orchestrator_event_bridge_dedup` `lib.rs:1663` → `dashboard_event_to_server` `lib.rs:1694` (lossy inverse) → `EventBus<ServerEvent>` → SSE/WS. Cycle broken by `BridgeDedup` (`lib.rs:338-341,1329,1675`).
   - `EventLogWriter` → **`.roko/events.jsonl`** (wired at `roko-serve/src/state.rs:841`; `state_hub.rs:75,144-153`).
3. `WorkflowEngine` → `RuntimeEvent` (26 variants, `roko-core/src/runtime_event.rs:64`) via `emit_runtime_event()` (type-keyed global bus, cap 2048, `roko-runtime/src/event_bus.rs:347-381`) → `JsonlLogger` → **`.roko/runtime-events.jsonl`** (`roko-runtime/src/jsonl_logger.rs:33`) → read by `routes/runs.rs:20`; also `sse_adapter.start_runtime_event_subscription()` (`routes/mod.rs:139`) → `/workflow/events` SSE (`routes/mod.rs:191,302`); ingest route `routes/event_ingest.rs`.
4. Event sources: `CronEventSource` / `FileWatchEventSource` (roko-plugin, spawned by `start_builtin_event_sources` `lib.rs:334,2773`) + webhook route (`routes/webhooks.rs:218`) → mpsc → `signal_ingest_loop` `lib.rs:2797` → `signal_store.put` + `ServerEvent::WebhookReceived` → `dispatch_loop` (flow 1).
5. `RokoEvent` global bus (8 variants, `roko-runtime/src/event_bus.rs:116-196,334-341`): `PrdPublished` → `spawn_prd_publish_subscriber` (`routes/prds.rs:223`, spawned `lib.rs:336`) → auto-plan; `PlanRevision` → gate-failure replan. These are the only two wired event→action chains.
6. `roko-learn::events::EventBus` (`roko-learn/src/events.rs:80`, `AgentEvent`) — separate learning-local bus; consumer `roko-learn/src/event_subscriber.rs`.
7. TUI (`roko dashboard`): `SharedStateHub::new_in_process()` + `bootstrap_from_workdir` + `replay_log_into_snapshot(.roko/events.jsonl)` (`roko-cli/src/tui/app.rs:533-551`) + `notify` file watcher (`tui/fs_watch.rs`). File-replay, not WS.

**Cross-process (relay, port 9011):**

8. Any WS client → `Publish` frame → `TopicBus.publish` (`apps/agent-relay/src/bus.rs:92`: global `AtomicU64` seq, per-topic ring cap 128) → fan-out `TopicMessage` to subscribers (`lib.rs:308-333`); replay-on-subscribe (`lib.rs:279-299`).
9. Relay lifecycle → `RelayEvent` (11 variants, `protocol.rs:232`) → `tokio::broadcast(256)` → `/relay/events/ws` (`lib.rs:360-404`, lagged notice) → demo-app `RelayDashboard.tsx`.
10. Relay chain watcher (`--rpc-ws-url`): polls `eth_blockNumber` every 2s → `new_block` on `chain:{chain_id}` (`chain_watcher.rs:40-89`). That is ALL it does (see Relay reality).
11. roko-serve → relay (three clients): workspace registration + heartbeat w/ circuit breaker (`roko-serve/src/relay.rs:263-388` → `POST /relay/workspaces/register`); `start_isfr_relay_bridge` (`lib.rs:412,2570`) subscribing `isfr:rates`, `isfr:epochs`, `chain:{id}` (`roko-core/src/isfr_feed.rs:147-151`) via `ISFRTopicAdapter` → `ISFRFeed.handle_message` → Pulses on local `BroadcastBus` (colon→dot topic mapping `isfr_feed.rs:125-126`); `start_feed_relay_bridge` (`lib.rs:418,2661`) registers all catalog feeds as agent `roko-feed-publisher` and forwards ticks.
12. roko-agent-server sidecar → relay: `relay_client::connect` (`registration.rs:164`; `RelayHandle` subscribe/publish/register_feed in `features/relay_client.rs:38-118`; `RelaySubscriber` wrapper `features/relay_subscriber.rs:89`).
13. Dashboard → roko-serve `/relay/*` reverse proxy → relay (`routes/relay_proxy.rs:23-31`, WS bridged via `bridge_ws`; 503 when `ROKO_AGENT_RELAY_URL` unset).
14. Chain (rich path, in serve not relay): `start_block_watcher` (`lib.rs:388,2443`) using `roko_chain::block_watcher::{BlockWatcher, ContractEventInfo}` → `ServerEvent::ChainBlock/ChainTx/ChainContractEvent` (`events.rs:620-654`) → DashboardEvent 1:1.
15. On-chain stigmergy demo loop: mirage-rs hosts pheromone/insight surfaces (`apps/mirage-rs/src/http_api/pheromone.rs`, `roko_bridge/subscription/pheromone.rs`) ← `apps/roko-chain-watcher` polls `chain_queryPheromones`/`chain_searchInsights` (`rpc_client.rs:174,194`), reacts, posts back (`main.rs:41-121`).

## Current state table

| Spec concept | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Relay wire protocol (hello/ack/subscribe/publish/feeds/req-resp) | v2-depth 12/01 | `apps/agent-relay/src/{protocol,lib,state}.rs` | ✅ | frames at `protocol.rs:90-156` match doc §3 exactly |
| Per-topic ring + global seq + replay-on-subscribe | 12/01 §5 | `bus.rs:26-127` | ✅ | cap 128 default (`bus.rs:19`), seq `bus.rs:93` |
| `resume_after` reconnect replay | 12/01 §5.4 (planned) | — | ❌ | `Subscribe { topic }` only, `protocol.rs:108-110`; residual in `70-RELAY-PROTOCOL-FREEZE.md` |
| Outbound frame timestamps | freeze residual | — | ❌ | `TopicMessage` frame lacks `timestamp_ms` (`protocol.rs:149-155`); envelope has it (`protocol.rs:167`) but it's dropped on send (`lib.rs:283-289`) |
| Batch subscribe (`topics: [...]`) | freeze residual | — | ❌ | single-topic frames only |
| Topic grammar: dot-separated, validation, reserved prefixes, GC | v2-depth 12/02 + **`tmp/relay-bus/04` DECISION (dots, migrate from colons)** | — | 🟡 **decided-not-done** | The decision is made (`04-topic-grammar.md:3` "Use dots"), not open: code still uses **colons** on the wire (`chain:31337` `chain_watcher.rs:40`, `isfr:rates` `isfr_feed.rs:149`); dots only on local Pulse topics via `map_topic` (`isfr_feed.rs:125`); no `validate_topic`, no reserved prefixes, no topic GC anywhere. Migration is a scoped task, no longer a design fork. |
| Telemetry as Lens Pipeline (Collector/Transform/Export Lens; `telemetry.log.*` Pulse topics; metrics=Lens outputs; traces=lineage Signals; dashboards=Lens compositions) | v2 15, v2-depth 09/01 | — | ❌ **absent** | 0 hits for `CollectorLens`/`TransformLens`/`ExportLens`/`telemetry.log` in `crates/`. Observability is ad-hoc, not a Cell pipeline. |
| Prometheus `/metrics` (ad-hoc reality) | (pre-v2) | `roko-serve/src/routes/metrics.rs:31` | ✅ | `metrics_handler` renders `MetricRegistry` counters/histograms + StateHub aggregate stats as Prometheus text; second route `/api/metrics/prometheus` (`routes/status/metrics.rs`) kept for back-compat. Real but not the spec's Lens model. |
| Workspace discovery + heartbeat + expiry | v2 11 §8 | relay `lib.rs:42-105`, `main.rs:57-68`; serve `relay.rs:263-388` | ✅ | 60s staleness expiry `main.rs:63`; circuit breaker `relay.rs:222` |
| Request/response bridge (`POST /relay/messages`) | 12/01 §6 | `lib.rs:114-124`, `state.rs` oneshot | ✅ | timeouts 15s/60s `protocol.rs:5-9` |
| Relay events WS + lagged notice | 12/01 §7 | `lib.rs:360-404` | ✅ | broadcast cap per doc |
| Relay auth | 12/01 §8 (Phase 2) | — | ❌ | all endpoints unauthenticated, as documented |
| Sidecar deployment (`127.0.0.1:9011`, serve proxy) | 12/04 §2 | `dev.sh:185-220`, `routes/relay_proxy.rs`, `docker/start-railway.sh` | ✅ | dev.sh always starts relay; `ROKO_AGENT_RELAY_URL` exported |
| Shared / validator-embedded relay, multi-relay dedup | 12/04 §3-4 | — | ❌ | library+binary split exists (`lib.rs` `app()`), rest future |
| Room-scoped envelope `{seq,ts,room,type,payload}` | v2 11 §9 | — | 🟡 | relay uses `topic` not `room`; serve WS uses `Envelope<ServerEvent>` with seq (`event_bus.rs:16`); no unified room naming |
| Backpressure strategies (coalesce/drop-oldest/lossless/sample) | v2 11 §14 | — | ❌ | only broadcast-lagged notices (`lib.rs:383`, `lib.rs:1337`) |
| Connect protocol (5 methods) / ConnectorKind / ConnectorManifest | v2 11 §1-3 | roko-core protocol traits | 🔌 | impls test-only per sibling `31-GRAPH-CELLS-ENGINE.md`; no runtime Connector lifecycle |
| Exoskeleton: MCP auto-registration as Connector | v2 11 §4.1 | — | 🟡 | MCP config passthrough wired (roko-agent `--mcp-config`), but no `McpConnector` |
| Exoskeleton: A2A cards + HDC fingerprint | v2 11 §4.2 | relay cards | 🟡 | inline card + `card_uri` `/relay/cards/{id}` (`lib.rs:107-112`); no `/.well-known/agent-card.json`, no HDC in card |
| Exoskeleton: ERC-8004 / x402 | v2 11 §4.3-4.4 | — | ❌ | demo contracts only (mirage/ISFR); no payments anywhere |
| Finality oracle / reorg handling | v2 11 §16 | — | ❌ | no `FinalityLevel`/`ChainReorg` in runtime code |
| StateHub (projection hub) | v1 12-interfaces/22 | `roko-runtime/src/state_hub.rs:80-306` | ✅ | canonical; serve re-exports `lib.rs:55-64`; snapshot+broadcast+event-log |
| StateHub named `Projection` trait contract | v1 22 §2-3 | `roko-runtime/src/projection.rs`, serve `projection_contract.rs`, `routes/projections.rs` | 🟡 | replay/projection helpers exist; the typed `Projection { hydrate/reduce/apply }` catalog is target-state, not implemented |
| roko-core `state_hub.rs` duplicate | draft claim | `roko-core/src/state_hub.rs` | 🕰️ | **orphan — not in `roko-core/src/lib.rs` module tree** (only `dashboard_snapshot` declared, `lib.rs:83`); dead file, delete |
| serve EventBus wrapper | draft claim | `roko-serve/src/event_bus.rs:25` | ✅ | thin delegate post-Task-104 (header comment lines 8-12); no second ring |
| Event vocabularies + lossy bridges | draft claim | see below | 🟡 | `ServerEvent` ~74 variants (`events.rs:87`), `ExecutionEvent` 8 (`events.rs:13`), `DashboardEvent` 44, `RuntimeEvent` 26, `RokoEvent` 8, `RelayEvent` 11, `AgentEvent` (roko-learn); bridges drop unmapped variants both directions (`lib.rs:1423,1627` and inverse) |
| Durable logs | draft claim | `.roko/events.jsonl` (DashboardEvent), `.roko/runtime-events.jsonl` (RuntimeEvent) | ✅ split | writers `state.rs:841`/`jsonl_logger.rs:33`; readers: TUI `app.rs:537`, gates status reads **both** `engrams.jsonl` and `events.jsonl` (`routes/status/gates.rs:86-92`) — draft's "different files" concern confirmed |
| Bus trait + backends | v2 01 | `roko-core/src/bus_backends.rs` | ✅ | `BroadcastBus` (no replay, `:27`), `MemoryBus` (ring, `:106`), `BusErased` (`:223`); `roko-runtime::PulseBus` (`pulse_bus.rs:31-81`) also impls `Bus` |
| Feeds: kernel types (`FeedKind/FeedAccess/FeedRegistry`) | v2 09 §1 | `roko-core/src/feed.rs:24-164` | 🔌 | types + registry defined, **never constructed at runtime** (registry test-only) |
| Feed = Cell+Connect+Trigger+Store; `FeedPublisherExt`; Recipes; paid feeds; on-chain adverts | v2 09 §1,4,7,12 | — | ❌ | no extension system, no RecipeCell, no payment path (`FeedAccess::Paid` unused) |
| Feed agents (live producers) | (not in spec; v2-refactoring work) | `roko-serve/src/feed_agents/` | ✅ | **29 agents** spawned via `spawn_all` (`feed_agents/mod.rs:108`, called `lib.rs:415`); publish `ServerEvent::FeedTick` |
| Relay feed registry + discovery | v2 09 §3 | relay `lib.rs:52-54,499-529` | 🟡 | register/unregister frames + `GET /relay/feeds[/{agent_id}]` work; **no** pagination/filters/`/sample` from spec §3.2-3.3 |
| ISFR feed bridge | tmp/relay-bus | `roko-core/src/isfr_feed.rs:46`; wired `lib.rs:2570-2650` | ✅ | relay TopicMessages → Pulses on local bus |
| `FileWatchFeed` / `ProviderHealthFeed` | 21-TMP-MAY-BATCH | — | ❌ | confirmed absent (0 grep hits) |
| Trigger protocol (arm/disarm), `TriggerBinding`, `.roko/triggers/`, 7 kinds, `roko trigger` CLI, chaining | v2 13 §1-5,11 | — | ❌ | no `TriggerProtocol/TriggerBinding/TriggerFired`; no `roko trigger` subcommand in `roko-cli/src/main.rs` |
| Subscriptions (actual trigger system) | (pre-v2) | `roko-core/src/config/subscriptions.rs:114` (`SubscriptionTrigger::{Cron,FileWatch,Webhook}`), `roko-serve/src/dispatch.rs:804,1464` | ✅ | config: `roko.toml [[subscription]]` + `.roko/subscriptions/*.toml` (writer `roko-cli/src/subscriptions.rs:239`, API `routes/subscriptions.rs`) → `dispatch_loop` matches glob+filters, enforces concurrency/cooldown/dedup → agent dispatch |
| Conductor watchers (10 rules) | v2 13 §9 | `roko-conductor/src/watchers/` | ✅ | acknowledged in spec as existing; thresholds via `[conductor]` |
| SignalPattern / ChainEvent triggers | v2 13 §3.5,3.7 | — | ❌ | raw material exists (`ServerEvent::ChainContractEvent`), no trigger binding |
| Groups (types, routes, coordination modes, invitations, `[[groups]]`) | v2 10 | — | ❌ | zero group-domain code; no `/api/groups` routes; no `Space`/`SpaceId` kernel type |
| `GroupContextBidder` | v2 10 §5.3 | — | ❌ | `AttentionBidder` has 8 variants (Neuro, Daimon, IterationMemory, CodeIntelligence, PlaybookRules, Research, TaskContext, Oracles) — no Group |
| Pheromones | v2 10 §3.1 / v1 13-coordination | `roko-core` `Kind::Pheromone` (kind.rs:94); `roko-orchestrator/src/coordination.rs:190-330` (`Pheromone`, 7 `PheromoneKind`s, `PheromoneScope::{Local,Mesh,Subnet,Global}`) | 🔌 | all constructors test-only in crates/; **live only in demo chain stack**: mirage-rs pheromone HTTP/WS/persist + `apps/roko-chain-watcher` poll/react loop |
| Stigmergy | v1 13-coordination | see pheromones | 🟡 | on-chain demo loop wired; no group/store integration; 0 hits for "stigmerg" in crates/ |
| Mesh sync | v1 13-coordination | `roko-orchestrator/src/mesh_relay.rs:57` (`MeshRelay`: dedup, version vectors, store-and-forward) | 🔌 | test-only; **`roko knowledge sync <peer>` exists** (`roko-cli/src/commands/knowledge.rs:51,559`) but is file-based: delta → `.roko/mesh/outbox/delta-{peer}.jsonl`, vectors at `.roko/neuro/version-vectors.json`; no network transport |

## Relay reality (apps/agent-relay vs depth docs)

- **Protocol conformance**: `AgentInboundFrame`/`RelayOutboundFrame`/`TopicEnvelope`/`RelayEvent` in `protocol.rs` are byte-for-byte what depth doc 01 documents (doc was written *from* the implementation, May 8). HTTP surface matches doc §8 exactly (`lib.rs:35-59`): health, agents, cards, messages, workspaces CRUD+heartbeat, feeds, topics introspection.
- **Drift 1 — topics**: depth doc 02 canonicalizes **dot** grammar with validation/reserved-prefixes/GC; every live topic is **colon**-separated (`isfr:rates`, `chain:31337`), the relay never validates, and nothing GCs topics. `ISFRFeed::map_topic` translates colon→dot only at the Pulse boundary (`isfr_feed.rs:125`). `70-RELAY-PROTOCOL-FREEZE.md` already tracks this; docs and code disagree today.
- **Drift 2 — chain watcher location**: depth doc 03 ("relay's chain watcher": 20-block backfill, tx/receipt fetch, AgentRegistry/MultiAgentMarket/ISFROracle decode, `ContractEventInfo`) describes **`roko-chain::block_watcher` wired into roko-serve** (`lib.rs:2443-2478` imports `BlockWatcher, ContractEventInfo`), not `apps/agent-relay/src/chain_watcher.rs`, which only polls `eth_blockNumber` and emits `new_block` (`chain_watcher.rs:56-95`). Contract-event projection reaches dashboards via `ServerEvent::ChainContractEvent`, bypassing the relay entirely.
- **Deployment / who runs it**: `dev.sh up` always builds and starts `agent-relay` on `127.0.0.1:9011` (`dev.sh:185-218`), passing `--rpc-ws-url ws://127.0.0.1:8545 --chain-id 31337` when mirage-rs is up, and exports `ROKO_AGENT_RELAY_URL`; roko-serve reverse-proxies `/relay/*` + both WS upgrades (`routes/relay_proxy.rs:23-31`); Railway container script `docker/start-railway.sh` also starts it. Env: `ROKO_AGENT_RELAY_BIND/_RPC_WS_URL/_CHAIN_ID` (`main.rs:23-35`). Dependency direction is clean: relay depends on no roko-* crate; `roko-agent-server` is the client library; `roko-serve` connects as three relay clients (registration, ISFR bridge, feed publisher).
- **Freeze status**: protocol NOT frozen. Residuals per `70-RELAY-PROTOCOL-FREEZE.md`, all confirmed still open in code: no `resume_after`, no batch subscribe, `TopicMessage` omits timestamp, colon topics, response-shape parity with `demo-app/src/lib/relay-api.ts` unverified.

## Feed & trigger census

**Feeds — what actually produces data today:**

| Producer | Where | Wired? |
|---|---|---|
| 29 `FeedAgent` impls (IsfrKeeper, 4 source scouts, ChainWatcherAgent, GasOracle, 3 derivatives, EpochTracker, OracleSubmitter, 3 monitors, 5 onchain, 5 defi, 4 market) | `roko-serve/src/feed_agents/*` (trait `mod.rs:54`, spawn `mod.rs:108` ← `lib.rs:415`) | ✅ each is a tokio task publishing `ServerEvent::FeedTick`; catalog in `AppState.feed_agent_catalog` (dev.sh banner "15 active" is stale) |
| `ISFRFeed` (relay→bus bridge) | `roko-core/src/isfr_feed.rs:46` ← `lib.rs:2570` | ✅ |
| `feed_relay_bridge` (serve→relay feed registration + tick forwarding) | `lib.rs:2661-2735` | ✅ gated on `feed_agents_enabled()` + relay URL |
| Relay `FeedDescriptor` registry | `protocol.rs:73-85`, `state.rs` | ✅ simple (list only) |
| `roko-core` `FeedRegistry`/`FeedInfo`/`FeedRuntimeStatus` | `feed.rs:24-164` | 🔌 never instantiated at runtime (M037 migration note in file) |
| `FileWatchFeed`, `ProviderHealthFeed` | — | ❌ never existed |
| CLI: `roko feed list/status` reads runtime feed endpoints | roko-cli `Feed` subcommand (`main.rs:314+`) | ✅ |

**Triggers — what actually fires agents today** (one pipeline, three sources):

```
roko.toml [scheduler]/[watcher] + [[subscription]] + .roko/subscriptions/*.toml
  → start_builtin_event_sources (lib.rs:334,2773): CronEventSource, FileWatchEventSource (roko-plugin)
  → signal_ingest_loop (lib.rs:2797): persist Engram + publish ServerEvent::WebhookReceived
  → dispatch_loop (dispatch.rs:1464): SubscriptionRegistry.find_matching (glob trigger + repo/branch/path/label filters)
      + concurrency_limit + cooldown_secs + dedup window
  → dispatch_agent → TemplateAgentDispatcher (LLM run, episode logged)
```
Plus webhook HTTP ingress (`routes/webhooks.rs`) into the same loop, and two hardcoded bus-event chains: `RokoEvent::PrdPublished` → auto-plan (`routes/prds.rs:223`) and `RokoEvent::PlanRevision` → gate-failure replan. CLI: `roko config subscriptions list/add/remove/enable/disable` (`roko-cli/src/subscriptions.rs`), `roko event-sources list` (`roko-cli/src/event_sources.rs:38`). Nothing writes or reads `.roko/triggers/`.

## V2-aligned

- Relay wire protocol, ring/seq/replay, feed registration frames, workspace directory, request/response bridge, events WS — all per depth docs (which are the implementation's own record).
- `Envelope { seq, ts_millis, payload }` + `replay_from(cursor)` on the in-process bus mirrors the v2 reconnect-with-cursor idea (SSE `Last-Event-Id`-style catch-up in `routes/ws.rs:140`, cap tests in `event_bus.rs:108-185`).
- StateHub-as-projection matches v1 doc 22's near-term description exactly (that doc's 2026-05-05 status note is accurate: canonical in roko-runtime, Task 104 consolidation done).
- Subscription dispatch (concurrency policies, cooldown, dedup) is a de-facto subset of v2 13 §4-6 semantics.
- Conductor watchers already satisfy v2 13 §9.
- Colon→dot mapping in `ISFRFeed` shows the intended v2 Pulse-topic grammar at the local-bus layer.

## Telemetry / observability layer (reality vs v2 15 + depth 09/01)

The spec (`docs/v2/15-TELEMETRY.md`, `docs/v2-depth/09-telemetry/01-observability-as-lens-pipeline.md`) reframes ALL observability as the **Observe protocol**: logs are Bus Pulses on `telemetry.log.*`; metrics are numeric Lens outputs; traces are lineage-annotated Signals; dashboards are named Lens (Cell) compositions; a `Collect → Transform → Export` Lens Pipeline is a Graph. **None of this exists** (0 hits for the Lens types or the topic prefix in `crates/`).

What is actually wired today (all pre-v2, hand-rolled):

| Surface | Where | Notes |
|---|---|---|
| `MetricRegistry` (labelled counters/histograms) | populated by provider dispatch + gate pipeline | source of the numeric metrics |
| `GET /metrics` (Prometheus text) | `roko-serve/src/routes/metrics.rs:31` | `MetricRegistry.render_prometheus()` + StateHub/runtime aggregate stats |
| `GET /api/metrics/prometheus` (back-compat) | `roko-serve/src/routes/status/metrics.rs` | second, older endpoint |
| Durable event logs | `.roko/events.jsonl` (DashboardEvent), `.roko/runtime-events.jsonl` (RuntimeEvent) | the de-facto "trace"/replay substrate |
| StateHub snapshot + `/workflow/events` SSE | `state_hub.rs`, `routes/mod.rs:191` | the de-facto live dashboard feed |

So the runtime already emits rich event streams and Prometheus metrics, but through bespoke handlers and typed-enum logs — not the read-only Lens-Cell Pipeline the spec describes. Converging them is a large, currently-undocumented gap: it would make logs first-class Pulses, unify the seven event vocabularies under the Bus, and turn dashboards into Lens Graphs. Near-term, the ad-hoc `/metrics` path is fine; the P2/P3 checklist item scopes the first Lens slice.

## Old paradigm & tech debt

- 🕰️ `roko-core/src/state_hub.rs` — orphan duplicate, not compiled (absent from `roko-core/src/lib.rs`); `dashboard_snapshot.rs:5,755` still doc-links `super::state_hub::StateHub` (broken link). Delete.
- 🟡 Lossy bidirectional bridges: `server_event_to_dashboard` / `dashboard_event_to_server` each end in `_ => None`; ~74-variant `ServerEvent` vs 44-variant `DashboardEvent` guarantees drop-on-the-floor classes (Inference*, ToolCall*, Bench*, Swe*, Matrix*, Vision* never reach the TUI; some Dashboard-only variants never reach SSE). `BridgeDedup` is a cycle-breaking patch over an architecture that wants a single vocabulary.
- 🟡 Seven event vocabularies (ServerEvent, ExecutionEvent, DashboardEvent, RuntimeEvent, RokoEvent, RelayEvent, AgentEvent) + Pulse. The v2 answer (everything is a Pulse on a topic) exists as `PulseBus`/`BroadcastBus`/`MemoryBus` but only the ISFR path uses it.
- 🟡 Durable-log split: `events.jsonl` (DashboardEvent) vs `runtime-events.jsonl` (RuntimeEvent) vs `signals.jsonl`/`engrams.jsonl`; gates status merges two of them ad hoc (`routes/status/gates.rs:86-92`).
- 🕰️ `roko-core/src/feed.rs` `FeedRegistry` — superseded in practice by serve's `feed_agent_catalog` + relay registry; three feed-metadata sources, none authoritative.
- 🕰️ Colon topics on the relay wire vs dot grammar in canonical docs.
- 🟡 Relay chain watcher (trivial) vs depth doc 03 (rich) — doc describes a different component's behavior.
- 🟡 dev.sh banner says "Feed agents → 15 active" (`dev.sh:282`); code spawns 29.

## Not implemented (groups, stigmergy, v2 trigger system…)

- **Groups**: nothing — no `Group/GroupMember/GroupInvitation/GroupConfig/CoordinationMode` types, no `/api/groups` routes, no `[[groups]]` TOML, no group relay rooms, no invitations, no `Space`/`SpaceId` kernel type. Pure spec (v2 10).
- **Stigmergic coordination in the product path**: pheromone types (`roko-orchestrator/src/coordination.rs:190-330`) and `MeshRelay` (`mesh_relay.rs:57`) are constructed only in `#[cfg(test)]`; `Kind::Pheromone` Signals are never produced by the runtime. The only live pheromone loop is the mirage-rs + roko-chain-watcher demo apps.
- **v2 Trigger system**: no `TriggerProtocol`, no `.roko/triggers/`, no Bus/SignalPattern/ChainEvent/Manual trigger kinds, no `roko trigger` CLI, no declarative trigger chaining.
- **Feeds spec surface**: pagination/filter/sample discovery API, paid feeds (x402/MPP), `FeedPublisherExt`, Recipes, ERC-8004 feed adverts.
- **Connectivity spec surface**: Connect-protocol runtime lifecycle, ConnectorKind discovery, finality oracle, reorg pulses, backpressure strategies, multi-relay dedup, room envelope, relay auth, `resume_after`.
- **Telemetry-as-Lens-Pipeline** (v2 15 / v2-depth 09/01): the entire "observability is the Observe protocol" model is unimplemented — no `CollectorLens`/`TransformLens`/`ExportLens` Cells, no `telemetry.log.*` Bus topics, no metrics-as-Lens-outputs, no lineage-annotated trace Signals, no dashboards-as-Lens-compositions. Today's telemetry is a hand-rolled `MetricRegistry` + `/metrics` Prometheus handler (`routes/metrics.rs:31`) plus the StateHub/durable-log surfaces above. The spec wants logs published as Pulses and every telemetry surface expressed as a read-only Lens Graph over Bus+Store; none of that plumbing exists. This is the largest undocumented gap in the event/observability layer.
- **Network mesh sync**: `roko knowledge sync` is outbox-file-only; MeshRelay has no transport.

## Migration checklist

- [ ] **[P0]** Implement `resume_after` on relay `Subscribe` (+ client support in `relay_client.rs`) and update depth doc 01 §5.4 — verify: `cargo test -p agent-relay resume` and `rg resume_after apps/agent-relay/src`
- [ ] **[P0]** Serialize `timestamp_ms` on outbound `TopicMessage` frames (`protocol.rs`, `lib.rs:283,320`) — verify: `cargo test -p agent-relay` frame-schema test asserting `ts`
- [ ] **[P1]** Execute the landed dots decision (`tmp/relay-bus/04`): migrate `chain:{id}`/`isfr:*` from colons to dots on the wire, remove `ISFRFeed::map_topic` translation shim, add `validate_topic` + reserved-prefix guard on relay publish — verify: `rg '"(chain|isfr):' crates/ apps/` returns zero (no shims left)
- [ ] **[P1]** Delete orphan `crates/roko-core/src/state_hub.rs` and fix `dashboard_snapshot.rs` doc links — verify: `rg 'state_hub' crates/roko-core/src` then `cargo build -p roko-core`
- [ ] **[P1]** Add batch subscribe (`topics: [...]`) per freeze checklist — verify: relay integration test
- [ ] **[P1]** Bridge coverage tests: enumerate `ServerEvent`↔`DashboardEvent` variants and assert every intentionally-dropped variant is listed (kill silent `_ => None` drift) — verify: `cargo test -p roko-serve bridge`
- [ ] **[P1]** Document event layering (transport bus / durable log / projection / API stream) in `docs/v2-depth/12-connectivity/INDEX.md` or a new depth doc; update doc 03 to say contract-event projection lives in roko-serve+roko-chain — verify: doc review
- [ ] **[P2]** Normalize SSE/WS cursor semantics (shared replay-cap constant, `Last-Event-Id` on SSE, gap→snapshot rule) — verify: `cargo test -p roko-serve sse_replay ws`
- [ ] **[P2]** Unify feed metadata: pick one authority among roko-core `FeedRegistry` (delete?), serve `feed_agent_catalog`, relay registry — verify: `rg 'FeedRegistry' crates/` shows zero or wired usage
- [ ] **[P2]** Feed discovery API v2 (pagination/kind/access filters, `/sample`) on relay or descope spec §3 — verify: `curl :9011/relay/feeds?kind=derived`
- [ ] **[P2]** Map `SubscriptionTrigger` onto v2 `TriggerBinding` shape (add Bus kind, `.roko/triggers/` persistence, `roko trigger list/fire`) so the existing dispatch_loop becomes the Trigger Engine — verify: `cargo run -p roko-cli -- trigger list`
- [ ] **[P2]** Graph Engine lifecycle Pulses/events on its default plan path (kept from draft; see `31-GRAPH-CELLS-ENGINE.md`) — verify: run a plan, `rg PhaseTransition .roko/events.jsonl`
- [ ] **[P3]** Groups MVP: `Group` types in roko-core, `/api/groups` CRUD in roko-serve, `group:{id}` relay topics, `[[groups]]` reconcile on serve start — verify: `curl -X POST :6677/api/groups`
- [ ] **[P3]** Wire pheromones into the product path: produce `Kind::Pheromone` Signals from orchestrator, hook `MeshRelay` to relay topics; converge with mirage demo loop — verify: `roko status` shows pheromone signals
- [ ] **[P3]** Network transport for `roko knowledge sync` (peer HTTP or relay topic instead of outbox files) — verify: two-workdir sync integration test
- [ ] **[P3]** Relay auth (publish gated on agent identity) per 12/01 §8 — verify: unauthenticated publish rejected
- [ ] **[P2→P3]** Telemetry Lens Pipeline (v2 15 / v2-depth 09/01): stand up the ad-hoc `/metrics` + `MetricRegistry` as the pragmatic near-term surface (already done), then decide whether to converge on the Lens-Cell model — first slice: publish structured logs as Pulses on `telemetry.log.*` and add a `LogExportLens` Collector→Export pair — verify: `rg 'telemetry.log' crates/` and `rg 'CollectorLens|ExportLens' crates/` non-empty; a metric reachable via a Lens Graph rather than a bespoke handler

## Open questions

1. **One vocabulary or seven?** Is the v2 end-state "everything is a Pulse" (making `PulseBus` the kernel and ServerEvent/DashboardEvent projections of it), or do typed enums stay with generated bridges? The BridgeDedup cycle-breaker suggests the current design is at its complexity ceiling.
2. **Dots vs colons** — RESOLVED as design: `tmp/relay-bus/04-topic-grammar.md` decides **dots** (NATS/RabbitMQ convention, URL-safety, wildcard-readiness, ends the `map_topic` shim). Only the migration remains open: does the relay start *validating* topics (it is deliberately opaque today), and do we do a hard cutover or a dual-accept window on `chain:`/`isfr:`?
3. **Feed truth**: are the 29 FeedAgents product surface or ISFR-demo scaffolding? The answer decides whether roko-core `FeedRegistry` gets wired or deleted, and whether feed discovery belongs to relay or serve.
4. **Trigger convergence**: should v2 TriggerBindings be a new engine in roko-runtime (per spec crate mapping) or a declarative façade over the proven `dispatch_loop`+`SubscriptionRegistry`?
5. **Where does chain-event projection live** — relay (per depth doc 03 title) or serve/roko-chain (per code)? If shared relays are the goal, decoding must move relay-side.
6. **RokoEvent vs ServerEvent**: `RokoEvent` (global runtime bus) carries only 8 infra variants; should PRD/plan-revision chains migrate onto the same bus dashboards consume?
7. **Groups prerequisite**: is a `Space` kernel primitive (Bus partition + Store partition) required first, or can groups ship as relay topics + scoped store queries without new kernel types?
