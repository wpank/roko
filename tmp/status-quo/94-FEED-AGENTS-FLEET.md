# Feed-agents fleet — the serve-side event firehose (`crates/roko-serve/src/feed_agents/`)

> Status-quo audit · created 2026-07-08 @ HEAD `5852c93c05` on `main` · sources: all 12 `.rs` files of `crates/roko-serve/src/feed_agents/`, `roko-serve/src/lib.rs`, `roko-serve/src/events.rs`, `roko-serve/src/state.rs`, `roko-core/src/config/{chain,schema}.rs`. Companion to `32-EVENTS-BUS-STATEHUB.md` (event bus / StateHub) and `46-SERVE-HTTP-REALTIME.md` (HTTP/SSE/WS surface), which mention "29 feed agents" but do not enumerate the fleet per-agent.
>
> Status vocab: ✅ wired (real data) · 🟡 synthetic/heuristic · 🔌 built-not-wired · ❌ missing

## Why this doc exists

`roko serve` spawns a fleet of **29 background feed agents** that produce the `.roko/events.jsonl` firehose and the demo-app dashboard's live tickers. They are the highest-volume event producers in the system (44 MB `events.jsonl` on the live workspace). Prior docs cite the count but no doc enumerated each agent, what it produces, or which are real vs synthetic. This is that ledger.

## TL;DR

- **29 feed agents**, spawned via `feed_agents::spawn_all(state)` at `roko-serve/src/lib.rs:415`, gated on `feed_agents_enabled()` (default `false`, `roko-core/src/config/chain.rs:105-114`).
- **All implement the `FeedAgent` trait** (`feed_agents/mod.rs:54-68`): `agent_id`, `display_name`, `capabilities`, `feeds`, `run`. Each `run()` is a tokio task looping on its own interval (2s–30s).
- **Every agent publishes `ServerEvent::FeedTick`** (`events.rs:654`) via `FeedAgentContext::publish_tick()` (`mod.rs:78-97`) onto `state.event_bus` (a `roko_runtime::event_bus::EventBus`).
- **26 agents read REAL state** (ISFR keeper rates, chain blocks, catalog); **3 are synthetic/heuristic** (TVL, stablecoin peg, MEV — all in `defi.rs`).
- **The count comment ("29 background agents", `mod.rs:1`) is now accurate.** The `dev.sh` banner "15 active" and doc-comments saying "15" are **stale** — 15 was the original set before the +5 onchain / +5 defi / +4 market additions.
- **A relay bridge** (`start_feed_relay_bridge`, `lib.rs:2661-2771`) forwards ticks to an external agent-relay when a relay URL is configured.

## The FeedAgent trait + spawn (file:line)

- Trait: `feed_agents/mod.rs:54-68` — `agent_id() -> &'static str` `:56`, `display_name()` `:58`, `capabilities()` `:60`, `feeds() -> Vec<FeedDescriptor>` `:62`, `run(self: Arc<Self>, ctx: FeedAgentContext)` `:64`.
- `publish_tick`: `mod.rs:78-97` → `state.event_bus.publish(ServerEvent::FeedTick { agent_id, feed_id, topic, payload, timestamp_ms })`.
- `spawn_all`: `mod.rs:108-230` — builds `Vec<Arc<dyn FeedAgent>>` of 29 agents (`:128-162`), spawns each, and populates `state.feed_agent_catalog` (`:165-201`).
- Gate: `mod.rs:110-114` early-returns empty if `!roko_config.feed_agents_enabled()`.
- Invocation: `lib.rs:415` `let _feed_agents = feed_agents::spawn_all(...)`; relay bridge at `lib.rs:418`.

## Fleet census (29 agents, by file)

| # | Agent group (file) | Agents | Interval | Data | Evidence |
|---|---|---|---|---|---|
| 1 | ISFR keeper (`keeper.rs`) | `IsfrKeeperAgent` | 10s | ✅ real — composite rate from `state.isfr.current_rate` | `keeper.rs:16` |
| 2 | Chain watcher (`chain_watcher.rs`) | `ChainWatcherAgent` | 2s | ✅ real — `state.chain.latest_block` | `chain_watcher.rs:16` |
| 3–6 | Source scouts (`source_scouts.rs`, macro) | `Aave/Compound/Ethena/Lido ScoutAgent` | 10s | ✅ real — per-source ISFR rate from `state.isfr.sources` | macro `:85,93,101,109` |
| 7 | Gas oracle (`gas_oracle.rs`) | `GasOracleAgent` | 5s | ✅ real — EMA gas from block base fees | `gas_oracle.rs:16` |
| 8–10 | Derivatives (`derivatives.rs`) | `RateDerivativeAgent` `:18`, `SpreadMonitorAgent` `:104`, `VolatilityWatcherAgent` `:185` | 15/30/20s | ✅ real — RoC, inter-protocol spreads, rolling stddev from rate history | — |
| 11 | Epoch tracker (`epoch_tracker.rs`) | `EpochTrackerAgent` | 5s | ✅ real — epoch counter + keeper status (atomics) | `epoch_tracker.rs:16` |
| 12 | Oracle submitter (`oracle_submitter.rs`) | `OracleSubmitterAgent` | 10s | ✅ real — submission status per epoch | `oracle_submitter.rs:17` |
| 13–15 | Monitors (`monitors.rs`) | `AgentMonitorAgent` `:17`, `ConfidenceScorerAgent` `:80`, `RelayStatsAgent` `:157` | 10/15/10s | ✅ real — catalog, per-source health, relay health | — |
| 16–20 | On-chain analytics (`onchain.rs`) | `BlockSpaceAgent` `:24`, `TxThroughputAgent` `:104`, `FeeBurnAgent` `:184`, `NetworkHealthAgent` `:251`, `ContractActivityAgent` `:336` | 3/5/5/5/10s | ✅ real (ContractActivity heuristic) — gas ratio, TPS, ETH burned, block-interval health, contract activity | — |
| 21–25 | DeFi analytics (`defi.rs`) | `YieldCurveAgent` `:23`, `LiquidationRiskAgent` `:104`, `TvlTrackerAgent` `:182`, `StablecoinPegAgent` `:259`, `MevTrackerAgent` `:337` | 30/20/30/10/10s | ✅ real (yield, liquidation); 🟡 **synthetic** (TVL, peg, MEV) | see below |
| 26–29 | Market analytics (`market.rs`) | `CorrelationAgent` `:24`, `RegimeClassifierAgent` `:117`, `RiskAdjustedAgent` `:213`, `SystemHeartbeatAgent` `:299` | 30/30/30/5s | ✅ real — correlation matrix, regime, Sharpe-like ratio, aggregate health | — |

**Total: 29** (`mod.rs:128-162`). Note: `source_scouts.rs` uses a macro that generates 4 `impl FeedAgent` blocks, so a naive `grep "impl FeedAgent"` under-counts by ~2.

## Synthetic agents (the 3 that fabricate data)

All in `defi.rs`, flagged as such in code comments:

| Agent | Evidence | Formula |
|---|---|---|
| `TvlTrackerAgent` | *"Derive synthetic TVL from source weights"* `:216`; *"In a real deployment this would query on-chain balances"* `:217` | `base_tvl = 100_000_000 * weight; est_tvl = base_tvl * rate_factor` `:226-228` |
| `StablecoinPegAgent` | *"Deterministic pseudo-random per stablecoin using tick count"* `:311` | `usdc_dev = base_dev + stress_dev * ((tick_count % 7)/7.0)` `:312-319` |
| `MevTrackerAgent` | *"MEV estimation heuristic"* `:374` | `est_mev_eth = util * base_fee_eth * tx_count * 0.01` `:382` |

The other 26 read from real keeper/chain state; several are *derived* analytics (spreads, volatility, Sharpe) but derived from real rate history, not random.

## Event path → `.roko/events.jsonl` firehose

1. `FeedAgent.run()` loops, calls `ctx.publish_tick(...)` (`mod.rs:78-97`).
2. `ServerEvent::FeedTick` published to `state.event_bus` → `roko_runtime::event_bus::EventBus::emit()` (`event_bus.rs:40`) → recorded in the replay ring + broadcast.
3. Ticks reach SSE/WS subscribers (dashboard) and the event-ingest writer → `.roko/events.jsonl` (44 MB on the live workspace; the biggest of the three log files — cf. `signals.jsonl` 80 KB, `engrams.jsonl` 10 KB stale).
4. **Relay bridge** (`start_feed_relay_bridge`, `lib.rs:2661-2771`, gated on `feed_agents_enabled()` `:2669`): registers all catalog feeds with the relay (`:2724-2735`), subscribes to `FeedTick` (`:2742`), forwards each to a relay topic (`handle.publish(&topic, "tick", payload)` `:2754`).

`AppState.feed_agent_catalog` (`state.rs:539`, type `RwLock<FeedAgentCatalog>`, def `:389-396`): `agents`, `feeds`, `messages_per_sec`. Populated by `spawn_all` (`mod.rs:165-201`), refreshed by `AgentMonitorAgent` each tick (`monitors.rs:50-52`).

## Config gate

- `FeedAgentsConfig { enabled: bool }` — `roko-core/src/config/chain.rs:105`, default `false` `:112`.
- `feed_agents_enabled()` accessor — `roko-core/src/config/schema.rs:356-358`.
- Configured via `[feed_agents]` in `roko.toml`. **Off by default** → a fresh `roko serve` spawns zero feed agents unless enabled.

## Drift / notes

- **Stale count strings**: `dev.sh` banner says "15 active"; some doc-comments in the codebase say "15" — the real count is **29** (`mod.rs:1` comment + `spawn_all` vector). Fix the stale banners.
- **Synthetic-data honesty**: 3 of 29 agents fabricate values (TVL/peg/MEV). The dashboard renders these as if real. `42-CHAIN-REGISTRIES-ISFR.md` already flags "mostly synthetic except keeper" — that's imprecise: 26 are real-ish (keeper-derived), 3 are truly synthetic.
- **Feed agents are ISFR/chain-domain, not the self-hosting loop.** They power the demo/marketing dashboard, not plan execution. This is orthogonal to the roko agent-building loop.
- **`events.jsonl` growth**: unbounded firehose; the 44 MB file is dominated by FeedTick. No GC/rotation on this log (cf. `55-DATA-DIR.md`, `60-STATE-PERSISTENCE-LEDGER.md`).

## Verification checklist

- [ ] `grep -c "impl FeedAgent" crates/roko-serve/src/feed_agents/*.rs` (+2 macro-hidden in source_scouts) → 29.
- [ ] `grep -n "spawn_all" crates/roko-serve/src/lib.rs` → `:415` invocation.
- [ ] `grep -n "feed_agents_enabled" crates/roko-core/src/config/schema.rs` → `:356`, default false in `chain.rs:112`.
- [ ] `grep -n "synthetic\|heuristic\|pseudo-random" crates/roko-serve/src/feed_agents/defi.rs` → confirms the 3 synthetic agents.
- [ ] `grep -n "start_feed_relay_bridge" crates/roko-serve/src/lib.rs` → `:2661` bridge, `:418` invocation.
- [ ] `ls -la .roko/events.jsonl` → confirm firehose size vs `signals.jsonl`/`engrams.jsonl`.

## Roadmap (ordered)

1. **[P2]** Fix stale "15 active" banners (`dev.sh` + doc-comments) → 29.
2. **[P2]** Label synthetic agents (TVL/peg/MEV) in the dashboard payload (`synthetic: true` flag) so consumers don't treat fabricated values as on-chain truth.
3. **[P2]** Add rotation/GC for `.roko/events.jsonl` (44 MB, unbounded FeedTick firehose).
4. **[P3]** Decide whether the ISFR/chain feed fleet belongs in the roko self-hosting product or is a separable demo/marketing subsystem; if the latter, feature-gate the whole `feed_agents` module more aggressively.
5. **[P3]** Back the 3 synthetic agents with real on-chain queries (TVL from balances, peg from oracle, MEV from mempool) or drop them.

## Cross-references

- `32-EVENTS-BUS-STATEHUB.md` — EventBus, ServerEvent, StateHub, the "29 agents" note.
- `46-SERVE-HTTP-REALTIME.md` — SSE/WS surface that streams FeedTick.
- `42-CHAIN-REGISTRIES-ISFR.md` — ISFR keeper + sources (the real-data source these agents read).
- `55-DATA-DIR.md` / `60-STATE-PERSISTENCE-LEDGER.md` — `.roko/events.jsonl` firehose and log split-brain.
- `93-ROKO-DEMO.md` — the *other* live event surface (:9090 WS, separate crate).
