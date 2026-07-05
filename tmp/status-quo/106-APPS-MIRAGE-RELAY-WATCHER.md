# 106 — Apps deep-dive: mirage-rs, agent-relay, roko-chain-watcher

> **Verification header**
> - Repo: `/Users/will/dev/nunchi/roko/roko`
> - Git HEAD: `5852c93c05` (branch `main`)
> - Date: 2026-07-08
> - Method: read `apps/*/src` entry points + routers + state; cross-referenced consumers in `crates/roko-serve`, `crates/roko-agent-server`, `crates/roko-cli`, `crates/roko-demo`; confirmed build wiring in root `Cargo.toml`.
> - Scope: the three `apps/` workspace members. This is the second-pass, exhaustive companion to the thin app coverage elsewhere in the pack (docs 59 route ledger, 70 relay-protocol freeze).

**Status tags:** `[WIRED]` reachable from a shipped binary at runtime · `[STANDALONE]` its own binary, only reachable if you launch it · `[PROXIED]` fronted by roko-serve when configured · `[LEGACY]` reference surface roko-serve now re-implements · `[LIB-CONSUMED]` its library is compiled into a shipped crate even though the binary isn't shipped.

---

## 0. TL;DR

| App | Files | LOC | Binary shipped by default? | What it is | Verdict |
|---|---|---|---|---|---|
| `apps/mirage-rs` | 52 | ~33.3K | **No** | In-process Ethereum **fork simulator** (revm) + JSON-RPC + a `chain` knowledge/pheromone/agent layer + REST dashboard API. The **reference backend** whose REST shape roko-serve's aggregator re-implements. | **KEEP** (own it as the sim substrate); **retire the mirroring**, don't retire mirage. |
| `apps/agent-relay` | 7 (incl. lib) | ~1.7K | **No** (but lib is compiled into `roko-agent-server`) | Narrow in-memory **websocket relay/topic-bus** for agent presence, cards, message forwarding, workspaces, feeds. The standalone service behind `tmp/relay-bus/01-relay-service-spec`. | **KEEP standalone**; it's the real relay. The embedded roko-serve relay is only a **reverse proxy** to it. |
| `apps/roko-chain-watcher` | 7 | ~2.9K | **No** (but roko-serve spawns its **binary** if present) | Long-running agent that polls a mirage chain, analyses blocks, decides reactions, posts insights/pheromones/challenges back via JSON-RPC. | **KEEP**, but **fix the runaway** (23 MB error-spam log) or gate it off by default. |

**Root build fact:** `default-members` in `Cargo.toml:84-89` is only `roko-cli`, `roko-mcp-code`, `roko-mcp-github`. **None of these three apps build on a plain `cargo build`.** They are workspace members (`Cargo.toml:29-37`) but opt-in binaries. This is the single most important fact for the pack: much of the "mirage-shape / no real producer" confusion in doc 59 stems from roko-serve advertising routes whose *reference implementation lives in an app that the default build never compiles*.

---

## 1. What Mirage actually IS

Mirage is **not** a legacy dashboard backend and **not** a fork of roko-serve. It is a **standalone in-process Ethereum fork simulator** — conceptually "anvil/hardhat-node written in Rust on top of revm" — that grew a bolt-on **agent-coordination knowledge layer**. Three concentric surfaces, gated by cargo features (`apps/mirage-rs/Cargo.toml:80-118`):

1. **EVM core (always on):** `fork.rs` (3,120 LOC — `MirageFork`, `HybridDB`, lazy upstream reads, copy-on-write), `provider.rs` (1,063 — `UpstreamRpc`), `rpc.rs` (**6,117 LOC**, the file — ~205 `eth_*`/`anvil_*` JSON-RPC method arms), `scenario.rs` (1,353 — COW scenario branching), `replay.rs` (1,859 — targeted follower / historical replay), `cow.rs`, `resources.rs` (profiles/modes), `rate_limit.rs`. Binds `127.0.0.1:8545` by default (`main.rs:43`). This is a genuine, large, working EVM simulator.
2. **`chain` feature (default-on):** `apps/mirage-rs/src/chain/` — an HDC-indexed **knowledge store** (`insight.rs`, `knowledge.rs` — `InsightEntry` state machine: Active/Confirmed/Challenged/Decaying), **stigmergy pheromones** (`pheromone.rs` — Threat/Opportunity/Wisdom w/ time decay), **agent registry + tasks + predictions** (`agent.rs`, `task.rs`, `prediction.rs`), HNSW switchover (`hnsw.rs`), and `chain_rpc.rs` (**2,154 LOC**, ~53 `chain_*` JSON-RPC methods) plus a custom revm HDC precompile at `0xA0C` (`precompiles/hdc.rs`, 842 LOC). Pulls in `roko-primitives` for HDC vectors.
3. **`dashboard-api` feature (implied by `chain`):** `apps/mirage-rs/src/http_api/` — **the REST surface roko-serve mirrors.** `http_api/mod.rs:275-367` mounts `/api/*`: health, pheromones (+summary/query/heatmap/projection), knowledge (entries/confirm/challenge/decay/edges/search/kinds), agents (topology/registry/trace/heartbeat/stats/skills), tasks (full lifecycle), predictions (sessions/claims/calibration), stats, deployment registry, and (roko feature) a `/api/ws` live stream. `ApiState` (`http_api/mod.rs:242-256`) is the shared `Arc<RwLock<ChainContext>>`.
4. **`roko` feature (opt-in, implies chain):** `apps/mirage-rs/src/roko_bridge/` — implements **roko-core traits over mirage internals**: `SimulationGate` (a `Gate` that runs a planned tx through a fork and returns a `Verdict`), `HdcSubstrate` + `ChainSubstrate` (`Substrate` impls backed by the HDC/knowledge layer), and a subscription/backpressure sink stack. Docstring `roko_bridge/mod.rs:1-30`: "drop-in simulation + knowledge substrate for roko golems." **This is the intended integration path and it is currently unused at runtime** — no shipped binary constructs these bridges (they exist only as library types + examples `roko_chain_watcher`, `persona_chain_native`).

**Entry point:** `apps/mirage-rs/src/main.rs` (869 LOC), `[[bin]]` gated on `required-features=["binary"]`. Rich CLI: host/port, upstream rpc/ws URLs, rps budget, chain-id, cache, profile/mode, auto-mine (`--block-interval-ms`), `--enable-hdc/-knowledge/-stigmergy`, HNSW threshold, snapshot interval, `--state-dir` (default `.roko/state/`), `--no-persist`. Telemetry via `roko_runtime::event_bus::EventBus` (`main.rs:24,32`) — this is the roko-native replacement for the old `golem-core::EventFabric`.

### 1a. The Mirage ↔ roko-serve mirroring map

roko-serve relates to mirage **two different ways simultaneously**, which is the crux of the doc-59 confusion:

**(A) Reverse proxy — `PROXIED`, real mirage required.** `crates/roko-serve/src/routes/rpc_proxy.rs` ("Mirage JSON-RPC reverse proxy routes", :1) forwards to a *real* mirage at `ROKO_MIRAGE_URL` (`state.rs:533-534,1020`). Returns 503 `mirage_not_configured` when unset (`rpc_proxy.rs:38-42`).

| roko-serve route | Forwards to mirage | Source |
|---|---|---|
| `POST /api/rpc` | mirage JSON-RPC (`:8545`) | `rpc_proxy.rs:26,47` |
| `GET /api/rpc` (WS) | `eth_subscribe` upgrade | `rpc_proxy.rs:28` |
| `GET /api/rpc/events` (WS) | mirage live-events WS | `rpc_proxy.rs:30` |
| `GET /api/rpc/health` | mirage health | `rpc_proxy.rs:32` |
| `ANY /api/rpc/api/{*path}` | mirage `/api/*` REST catch-all | `rpc_proxy.rs:34` |

**(B) Compatibility re-implementation — `LEGACY` shape, mirage NOT required.** `crates/roko-serve/src/routes/aggregator.rs` ("**Mirage-compatible** aggregation routes backed by discovered agent servers", :1). This router (`aggregator.rs:40-63`) **re-exposes mirage's `/api/*` REST shape** but sources data from **discovered roko-agent-server sidecars** instead of a chain:

| aggregator route (mirrors mirage `http_api`) | Real producer | Mirage original |
|---|---|---|
| `GET /agents`, `/agents/topology`, `/agents/{id}/{stats,skills,heartbeat,trace}` | agent-server fan-out (`AgentCard`) | `http_api/{agent,skills,topology}.rs` |
| `GET /predictions/sessions`, `/claims`, `/calibration/{agent}` | agent-server aggregation | `http_api/prediction.rs` |
| `GET /knowledge/{entries,edges,search,kinds}` | agent-server aggregation | `http_api/knowledge.rs` |
| `GET /tasks`, `/tasks/stats`, `/tasks/{id}` | agent-server aggregation | `http_api/task.rs` |
| `GET /ws` | WS mux over agents | `http_api/ws.rs` |

The test `compatibility_agent_routes_match_mirage_shapes` (`aggregator.rs:1293-1328`) explicitly asserts the JSON envelopes match mirage's shapes (`skills`, `heartbeat.alive/busy`, `trace.items/total/limit`). **Conclusion for doc 59:** any aggregator route that reports empty/zero with "no real producer" is a mirage-shaped envelope whose data only appears when (a) roko-agent-server sidecars are registered, or (b) you point `ROKO_MIRAGE_URL` at a live mirage. The shape is frozen against mirage; the producer is optional. Mirage is the **schema-of-record**, roko-serve is a **shape-compatible aggregator/proxy**.

---

## 2. agent-relay

**What it is:** a narrow, **in-memory** websocket relay (Cargo desc: "Narrow in-memory relay for agent websocket presence, cards, and message forwarding"). This is the concrete **standalone relay service** referenced by `tmp/relay-bus/01-relay-service-spec` (doc 70). Binds `127.0.0.1:9011` (`main.rs:23`).

**Entry point:** `apps/agent-relay/src/main.rs` (90 LOC) — parses CLI (`--bind`, `--rpc-ws-url`, `--chain-id`), builds `RelayState`, spawns a 30s stale-workspace expiry loop (60s TTL, `main.rs:59-68`), optionally spawns the chain watcher (`main.rs:71-81`), serves `app(state)`.

**Modules:**
- `lib.rs` (529) — the axum router `app()` (`lib.rs:34-61`): `/relay/health`, `/relay/agents` (+`/ws`), `/relay/cards/{id}`, `POST /relay/messages`, `/relay/events/ws`, workspaces (list/register/{id}/heartbeat/delete), feeds (`/relay/feeds`, `/relay/feeds/{agent_id}`), and **feed metadata (A5)**: `/relay/topics`, `/relay/topics/{topic}/messages`, `/relay/topics/{topic}/subscribers`.
- `state.rs` (487) — `RelayState`: registered agents, workspaces, cards, message-await/begin state machine (`AwaitMessageError`, `BeginMessageError`).
- `protocol.rs` (275) — the **wire types** (`AgentHello`, `AgentInboundFrame`, `RelayOutboundFrame`, `TopicEnvelope`, `FeedDescriptor`, `ConnectedAgent/Workspace`, `RelayMessageRequest`). **This is the shared contract** — imported by `roko-agent-server` (`features/relay_client.rs:5,85`) and by tests in `roko-cli` and `roko-agent-server`. Hence **`[LIB-CONSUMED]`**: even though the `agent-relay` *binary* isn't shipped, `agent_relay` the *library* is compiled into roko-agent-server.
- `bus.rs` (208) — `TopicBus`/`TopicBusConfig` pub/sub ring.
- `chain_watcher.rs` (132) — optional: when `--rpc-ws-url` set, polls `eth_blockNumber` every 2s and publishes `new_block` `TopicEnvelope` on topic `chain:{chain_id}` (`chain_watcher.rs:1-45`). (Note: a **third** distinct "chain watcher" — see §4.)

**Relationship to embedded relay in roko-serve:** roko-serve does **not** re-implement the relay; it **reverse-proxies** to it. `crates/roko-serve/src/routes/relay_proxy.rs` ("Agent-relay reverse proxy routes", :1) forwards `/relay/agents/ws`, `/relay/events/ws`, and `ANY /relay/{*path}` to `ROKO_AGENT_RELAY_URL` (`relay_proxy.rs:26-31`; `state.agent_relay_url` `state.rs:535-536,1023`); 503 `agent_relay_not_configured` when unset. So the relay is authoritative and standalone; roko-serve is a pass-through on the single public port.

---

## 3. roko-chain-watcher

**What it is:** a long-running **roko agent** (Cargo desc: "subscribes to a mirage chain and posts insights"). It observes a mirage chain, analyses blocks + pheromones/insights, decides reactions, and posts them back over JSON-RPC. Depends on `roko-core` + `roko-chain` (`Cargo.toml`).

**Entry point:** `apps/roko-chain-watcher/src/main.rs` (124 LOC). Probes mirage via `eth_blockNumber` + `chain_version` (`main.rs:60-69`), then spawns two independent loops:
- **Block observer** (`block_observer.rs`, 888 LOC) — real chain analysis over an eth RPC URL (defaults to the mirage URL), with backfill + optional full-tx fetch, `--dry-run`. `main.rs:72-98`.
- **Reaction loop** (`watcher.rs`, 291 + `reactions.rs`, 682) — `reactions::decide(...)` (`reactions.rs:183`) maps observed state to a `ReactionKind` (`reactions.rs:24`) such as `PostInsight`, `ConfirmInsight`, `ChallengeInsight`, deposit pheromone — then posts via `rpc_client.rs` (333, `MirageRpcClient`). `known_addresses.rs` (500) seeds address labels.

**What it watches/emits:** watches mirage blocks + the chain knowledge/pheromone layer; emits `chain_postInsight` / `chain_confirmInsight` / `chain_challengeInsight` / pheromone deposits back into mirage. This is the producer that populates the knowledge/pheromone data mirage's dashboard API (and thus roko-serve's aggregator shape) reports.

**Runtime wiring — `[WIRED]` but fragile.** roko-serve **spawns the chain-watcher binary as a subprocess** when `chain.rpc_url` is configured (`crates/roko-serve/src/lib.rs:442-484`): it resolves `roko-chain-watcher` next to the current exe, runs `--rpc-url <rpc>` with `ROKO_LOG=warn`, and **redirects stdout/stderr to `.roko/chain-watcher.log`**. This explains the 23 MB log.

**⚠ Runaway finding:** `.roko/chain-watcher.log` is **23 MB** and is a tight loop of `WARN reaction failed kind=ChallengeInsight error=RPC error from chain_challengeInsight: code=-32101 message=duplicate challenge...` (last entry 2026-05-06). The watcher re-issues duplicate challenges the chain rejects, with no backoff/dedup, spamming the log. Two bugs: (1) reaction dedup missing in `reactions.rs`/`watcher.rs`; (2) roko-serve appends forever with no rotation (`lib.rs:452-456`).

**⚠ Naming collision — three "chain watchers":**
1. `apps/roko-chain-watcher/` — the real analysis+reaction agent (this section).
2. `apps/agent-relay/src/chain_watcher.rs` — relay's block→topic poller (§2).
3. `crates/roko-serve/src/feed_agents/chain_watcher.rs` — `ChainWatcherAgent` (`:14`), an in-process **feed agent** publishing `feed:chain:blocks` every 2s from `state.chain` ring buffers. Registered in `feed_agents/mod.rs:135`. Distinct from #1 and unrelated to mirage.
`crates/roko-serve/src/routes/chain.rs:44,204` also exposes `GET /chain/watcher` status — reporting on #1's presence.

---

## 4. Status matrix

| Dimension | mirage-rs | agent-relay | roko-chain-watcher |
|---|---|---|---|
| Workspace member | yes (`Cargo.toml:33`) | yes (`:34`) | yes (`:37`) |
| In `default-members` | **no** | **no** | **no** |
| Binary shipped by `cargo build` | no | no | no |
| Library consumed by a shipped crate | roko feature only (unused) | **yes — `roko-agent-server`** | no |
| Reachable at runtime today | via `ROKO_MIRAGE_URL` proxy | via `ROKO_AGENT_RELAY_URL` proxy | **spawned by `roko serve`** if `chain.rpc_url` set |
| Corresponding roko-serve surface | `rpc_proxy` (proxy) + `aggregator` (shape mirror) | `relay_proxy` (proxy) | `feed_agents/chain_watcher` (unrelated) + `/chain/watcher` status |
| Corresponding crate | `roko-chain` (client), `roko-primitives` (HDC), `roko-core` (traits via `roko` feature) | `roko-agent-server` (protocol consumer) | `roko-core` + `roko-chain` |
| Overall tag | `[STANDALONE][PROXIED][LEGACY-shape source]` | `[STANDALONE][PROXIED][LIB-CONSUMED]` | `[STANDALONE][WIRED-spawn]` |
| Health | large, tests present (Plan 03 anchors, `lib.rs:2-4`); roko bridge unused | healthy, narrow, protocol is the frozen contract | **runaway log bug** |

---

## 5. Keep / merge / retire recommendation

**mirage-rs → KEEP (own it as the simulation substrate); retire the *mirroring*, not the app.**
It is the largest, most substantial app and the schema-of-record for the whole chain/knowledge/pheromone dashboard surface. Actions:
- Wire the **`roko` bridge** into a real runtime path (SimulationGate as a gate rung; ChainSubstrate as a knowledge substrate). Today it's built-not-connected — classic roko pattern.
- Decide the aggregator's fate: either (a) keep `aggregator.rs` as the shape-compatible façade and document it as "mirage-shape, agent-sourced" (kills the doc-59 "no producer" mystery), or (b) if agents never produce this data, **retire the aggregator routes** and rely solely on `rpc_proxy` → real mirage. Do **not** retire mirage itself.
- Consider adding mirage to a non-default build profile / feature so CI compiles it (it currently escapes `cargo build`).

**agent-relay → KEEP standalone.**
It's the real relay; `protocol.rs` is a frozen shared contract already compiled into roko-agent-server. Do not merge into roko-serve — the current proxy split is correct. Only cleanup: fold the relay's `chain_watcher.rs` block-poller into the naming taxonomy (§3) so the three "chain watchers" are disambiguated in docs/config.

**roko-chain-watcher → KEEP, but fix the runaway before anything else.**
It is the genuine producer of insights/pheromones. Blocking bugs:
1. Add reaction **dedup/backoff** so `chain_challengeInsight` duplicate errors (-32101) stop looping (`reactions.rs`/`watcher.rs`).
2. Add **log rotation or size cap** to the roko-serve spawn (`lib.rs:452-456`), or gate the auto-spawn behind an explicit config flag (it currently auto-starts whenever `chain.rpc_url` is set).
3. Truncate the existing 23 MB `.roko/chain-watcher.log`.

---

## 6. Checklist (verifiable claims)

- [x] Root `default-members` excludes all three apps — `Cargo.toml:84-89`.
- [x] aggregator self-labels "Mirage-compatible" — `aggregator.rs:1`; test asserts mirage shapes — `aggregator.rs:1293`.
- [x] rpc_proxy forwards to real mirage at `ROKO_MIRAGE_URL` — `rpc_proxy.rs:1-42`, `state.rs:1020`.
- [x] relay_proxy forwards to `ROKO_AGENT_RELAY_URL` — `relay_proxy.rs:1-31`, `state.rs:1023`.
- [x] `agent_relay` library imported by `roko-agent-server` — `features/relay_client.rs:5,85`.
- [x] roko-serve spawns `roko-chain-watcher` binary → `.roko/chain-watcher.log` — `lib.rs:442-484`.
- [x] `.roko/chain-watcher.log` = 23 MB of duplicate-challenge WARN spam (last 2026-05-06).
- [x] Three distinct "chain watchers" exist (app / relay module / feed agent) — §3.
- [x] mirage `roko_bridge` (Gate+Substrate) built but unused at runtime — `roko_bridge/mod.rs:32-44`.
- [x] mirage REST `/api/*` shape defined in `http_api/mod.rs:275-367`.

## 7. Roadmap (docs + code)

1. Update doc 59 (API route ledger): annotate every aggregator route with "mirage-shape; producer = agent sidecars OR proxied mirage; empty by default." Cross-link here.
2. Update doc 70 (relay freeze): note `agent-relay` binary is standalone/unshipped but `protocol.rs` is the compiled-in contract.
3. Add a GAP entry for: (a) mirage `roko` bridge unwired, (b) chain-watcher runaway + no log rotation, (c) three-way chain-watcher naming collision, (d) apps excluded from default/CI build.
4. Code: fix chain-watcher dedup + log cap; decide aggregator keep-vs-retire; feature-gate mirage into CI.
