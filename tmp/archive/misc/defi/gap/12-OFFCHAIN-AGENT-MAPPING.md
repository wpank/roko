# Offchainservices-Agent → Roko Mapping

> **Source agent**: `/Users/will/dev/nunchi/offchainservices-agent/` (~20K LOC Python)
> **Target**: Roko architecture (18 crates, ~177K LOC Rust)
> **Purpose**: Map production trading bot patterns into roko's **universal** framework — not as trading-specific traits, but as domain-agnostic primitives that work for DeFi, on-chain ops, off-chain workflows, and anything else

---

## Design Principle: Compose Through Existing Traits, Don't Fork Them

Roko's core is already domain-agnostic:

| Layer | Generic? | Notes |
|-------|----------|-------|
| **Engram** (noun) | Yes | Content-addressed blob with `Kind`, `Body`, `Score`, `Decay`, `Attestation` — no domain coupling |
| **Substrate** (store) | Yes | `put(Engram)`, `query(Query)`, `query_similar(HdcVector)` — treats data as opaque |
| **Scorer** (rate) | Yes | 7 axes: confidence, novelty, utility, reputation, precision, salience, coherence — universal |
| **Gate** (verify) | Yes* | Trait is generic (`verify(Engram) → Verdict`); implementations skew Rust/code — extensible |
| **Router** (decide) | Yes | Bandit-based selection with reward feedback — domain-agnostic |
| **Composer** (assemble) | Yes | Budget-constrained engram assembly — works for prompts, tx bundles, anything |
| **Policy** (react) | Yes | Stream processing: observe engrams/pulses → emit interventions |

**The wrong approach**: Create `TradingStrategy`, `PreTradeGuard`, `VenueAdapter`, `DecisionEngine`, `PositionGuard`, `TradingReflect` — parallel domain-locked traits that duplicate the existing system.

**The right approach**: Express each offchain pattern as a *composition* of existing traits + domain-specific config/content, extending only where the trait system genuinely lacks a primitive.

---

## What Actually Needs Building (Domain-Agnostic Primitives)

### P0: Three missing universal patterns

These three gaps are genuinely missing from the trait system. They aren't trading-specific — they're patterns that any continuous agent workflow needs:

#### 1. Tick-Driven Agent Loop (currently missing)

**Offchain analog**: APEX's `evaluate(state, signals) → actions` on a clock

**Universal pattern**: Agents that run continuously on a schedule, not just request-response. Needed by:
- DeFi: MM quoting on 50ms ticks, funding arb on 30s ticks
- On-chain ops: Oracle updates, keeper jobs, governance vote monitoring
- Off-chain: CI/CD polling, data pipeline scheduling, API health monitoring, log rotation

**What to build**: A `TickPolicy` — a `Policy` implementation that fires `decide_with_pulses()` on heartbeat intervals. The heartbeat clock already exists (`HeartbeatPolicy` in `roko-chain/src/heartbeat_ext.rs`) but ticks nothing. Wire it:

```
HeartbeatPolicy (gamma=50ms, theta=2s, delta=30s)
  → fires Pulse at each interval
  → TickPolicy.decide_with_pulses(engrams, pulses, ctx) → actions
  → actions dispatched through existing agent/tool dispatch
```

**Not trading-specific**: the tick rates and the *content* of actions vary by domain. The clock + policy dispatch is universal. APEX's priority logic (risk first → exits → entries) is just one `Policy` implementation — governance monitoring would have different priorities (quorum check first → vote deadline → delegation refresh).

#### 2. Pre-Action Gate Pipeline (currently missing)

**Offchain analog**: QuotingEngine's 7 guards, Guard trailing stop

**Universal pattern**: Gates that run *before* an action, not just after. Needed by:
- DeFi: Risk limits before trade submission, slippage check before swap
- On-chain ops: Gas estimation before tx, nonce check before broadcast, simulation before deployment
- Off-chain: Rate limit check before API call, budget check before LLM call, permission check before file write
- Code: Dry-run before destructive operations, lint before commit

**What to build**: The Gate trait already supports this — `verify(Engram, Context) → Verdict`. The gap is that the *rung pipeline* only runs post-execution in `orchestrate.rs`. Add a second pipeline insertion point:

```
Action intent (Engram with kind=ActionIntent)
  → PreActionPipeline: [Gate1, Gate2, ...].verify(intent)
  → if all pass: execute action
  → if any fail: emit Verdict engram, skip action, optionally replan
  → PostActionPipeline: existing rung gates
```

The 7 quoting guards become 7 `Gate` implementations. Trailing stop becomes a stateful `Gate` whose `verify()` checks position state. Oracle staleness is a `Gate` that checks data freshness. **None of these need new traits** — they're just new Gate implementations configured into a pre-action pipeline.

**Not trading-specific**: the pre-action pattern applies universally. The content of each gate is domain-specific, but the pipeline mechanism is shared. A code agent's pre-action gates might be `[DryRunGate, DiffSizeGate, BranchProtectionGate]`. A trading agent's might be `[RiskLimitGate, StalenessGate, SlippageGate]`.

#### 3. Multi-Slot Agent Composition (currently missing)

**Offchain analog**: APEX managing N concurrent positions with independent guard state per slot

**Universal pattern**: One logical agent managing N concurrent sub-tasks with per-task state. Needed by:
- DeFi: N open positions, each with its own trailing stop state
- On-chain ops: N keeper jobs across M chains, each with independent health
- Off-chain: N parallel CI builds, each with timeout/retry state; N API integrations, each with rate-limit state
- Code: N file edits in a refactoring plan, each with its own gate state

**What to build**: A `CompositePolicy` that manages N `Policy` instances, each with its own state, under a shared resource budget. The existing `ProcessSupervisor` manages agent lifecycles but treats each as independent. The missing piece is a coordination layer:

```
CompositePolicy {
  slots: HashMap<SlotId, (Policy, SlotState)>,
  budget: SharedBudget,  // total exposure, total API calls, total tokens
}

impl Policy for CompositePolicy {
  fn decide_with_pulses(&self, engrams, pulses, ctx) -> PolicyOutputs {
    // For each active slot: slot_policy.decide()
    // Aggregate: enforce shared budget across all slots
    // Emit: per-slot actions + global coordination signals
  }
}
```

**Not trading-specific**: "N concurrent sub-tasks with shared resource constraints" is the universal pattern. In trading the shared budget is notional exposure; in CI it's CPU/parallelism; in API integrations it's rate limits.

---

## Mapping: Offchain Components → Roko Compositions

Each offchain component maps to a *composition* of existing roko traits, not a new parallel system:

### 1. Strategies → DomainProfile + Agent Config + Playbooks

| Offchain | Roko Composition |
|----------|-----------------|
| `BaseStrategy.on_tick()` | `TickPolicy.decide_with_pulses()` on heartbeat interval |
| 14 strategy implementations | 14 `AgentDefinition` configs with domain-specific prompts + tool allowlists |
| Strategy parameters (spread, grid intervals) | `AgentDefinition.metadata` (arbitrary key-value) + `tags` on engrams |
| Ensemble voting | `CompositePolicy` with N sub-policies + `VotingGate` aggregation (already exists in roko-gate) |
| `ClaudeAgentStrategy` | Native roko agent dispatch — already works |

**Key insight**: A "strategy" is just an agent with (a) a tick schedule, (b) a restricted tool set, (c) domain-specific system prompt content, and (d) pre-action gates. All four already have trait-level support.

**Config example** (any domain, not just trading):
```toml
[[agents]]
name = "avellaneda-mm"
domain = "defi.market-making"         # DomainProfile resolves defaults
prompt = "Avellaneda-Stoikov market maker. Target spread: {spread_bps}bps..."
tick_interval_ms = 50                  # gamma heartbeat
tools = ["venue.place_order", "venue.cancel_order", "venue.get_snapshot"]
pre_action_gates = ["risk_limit", "staleness", "inventory_skew"]

[[agents]]
name = "governance-voter"
domain = "governance"
prompt = "Monitor proposals, evaluate against DAO constitution, vote when quorum approaching..."
tick_interval_ms = 30000               # delta heartbeat (30s)
tools = ["chain.read_contract", "chain.submit_tx", "web.fetch"]
pre_action_gates = ["gas_estimate", "quorum_check", "delegation_verify"]

[[agents]]
name = "ci-watcher"
domain = "ops"
prompt = "Monitor GitHub PRs, trigger builds, report failures..."
tick_interval_ms = 10000
tools = ["shell.exec", "web.fetch", "file.read"]
pre_action_gates = ["rate_limit", "branch_protection"]
```

### 2. QuotingEngine → Pre-Action Gate Pipeline

| Offchain Guard | Roko Gate Implementation | Reusable Beyond Trading? |
|---------------|------------------------|--------------------------|
| Oracle staleness (>45s halt) | `FreshnessGate { max_age: Duration }` | Yes — stale API data, stale cache, stale config |
| Vol classification (4-tier multiplier) | `RegimeGate { thresholds: Vec<f32> }` | Yes — any tiered response to environmental conditions |
| Drawdown scoring | `BudgetGate { max_loss_fraction: f32 }` | Yes — token budget, API cost budget, time budget |
| Inventory skew | Domain-specific `Gate` impl | Mostly trading-specific, but generalizes to "resource imbalance" |
| Toxicity feedback | `FeedbackGate` that reads from learning store | Yes — any post-hoc quality signal that modifies future gates |
| Event schedule | `ScheduleGate { blackout_windows: Vec<TimeRange> }` | Yes — maintenance windows, deploy freezes, market close |
| Reduce-only zone | `ThrottleGate { mode: ReduceOnly }` | Yes — read-only mode, graceful degradation, circuit breaker |

**7 of 7 guards generalize beyond trading.** The pattern is: before any action, run a pipeline of `Gate::verify()` checks. The *content* (what each gate checks) is domain-specific; the *mechanism* (pipeline, verdict, short-circuit) is universal.

### 3. APEX Orchestrator → TickPolicy + Priority Router

| Offchain | Roko Composition |
|----------|-----------------|
| `evaluate(state, signals, opps, guards)` | `TickPolicy.decide_with_pulses(engrams, pulses, ctx)` |
| Priority: risk → exits → entries | Priority `Router` that sorts action candidates by urgency tier |
| Multi-slot management | `CompositePolicy` with per-slot sub-policies |
| Pure engine (zero I/O) | `Policy` trait is already pure: `decide()` takes data in, returns actions out |

**Not a new trait.** APEX is a `Policy` implementation that:
1. Receives pulses (tick events) from heartbeat
2. Queries substrate for current state (engrams tagged with slot IDs)
3. Runs pre-action gates on each candidate action
4. Returns prioritized actions via `PolicyOutputs`

### 4. Pulse Signal Engine → Scorer + Oracle + Event Subscription

| Offchain | Roko Composition |
|----------|-----------------|
| 5-tier signal classification | `Scorer` with tiers mapped to score axes (confidence for tier, salience for urgency) |
| OI/volume/funding data | Domain-specific `Engram` kinds ingested via event subscription |
| Live data feed | Event subscription (already built in roko-conductor: 10 watchers) |
| Breakout detection | `PatternDiscovery` (already built) + domain-specific pattern definitions |

**What changes**: ChainOracle's `PricePoint` needs additional fields (volume, OI, funding) — but these are just new `Engram` kinds with richer `Body::Json` payloads. The scoring and tier logic uses existing `Scorer` trait with domain-aware axis weights.

**Generalized**: the same signal engine pattern works for:
- DeFi: price/volume/funding signals
- Governance: proposal count/voting power/quorum proximity signals
- Ops: error rate/latency/queue depth signals
- Code: test failure rate/coverage delta/lint warning count signals

### 5. Radar Opportunity Screening → Scorer + Bandit

| Offchain | Roko Composition |
|----------|-----------------|
| Multi-market scanning | `Scorer::score()` applied to engrams from multiple sources |
| Technical overlay (MACD/RSI) | Additional `Scorer` implementations — each indicator is a scoring dimension |
| Opportunity ranking | `Router::select()` with `UcbBandit` over scored candidates |
| Liquidation cascade detection | Pattern discovery on chain event stream (sequence of engrams) |

**Already 90% covered by existing traits.** The only gap is that nobody has written DeFi-specific `Scorer` implementations yet. The framework is there.

### 6. Guard Trailing Stop → Stateful Gate

| Offchain | Roko Composition |
|----------|-----------------|
| Phase 1 (breathe) → Phase 2 (lock) | `Gate` impl with internal state machine (states stored as engrams in substrate) |
| Tier-based profit locking | Gate config: `thresholds: Vec<(f32, f32)>` — ROE trigger → lock percentage |
| Per-slot guard state | `CompositePolicy` manages N gate instances, each with own state |
| GuardAction: HOLD/CLOSE/TIER_CHANGED | Maps to `Verdict { passed, reason, detail }` + action engrams |

**Generalizes to**: any progressive state machine where conditions tighten over time. CI example: build starts with lenient timeout → after N minutes, starts failing slow tests → after M minutes, hard-kills everything.

### 7. REFLECT Loop → Episode Logger + Domain-Specific Scorer

| Offchain | Roko Composition |
|----------|-----------------|
| FIFO P&L attribution | Domain-specific episode post-processor — reads episode engrams, computes round-trip P&L |
| Per-strategy stats | Episodes tagged with `agent_name` — group + aggregate via `Scorer` |
| Nightly review cycle | Dreams subsystem (already built) triggered on schedule |
| ReflectMetrics | New `Engram` kind with `Body::Json` containing domain-specific metrics |
| Convergence analysis | `ForensicReplay` (already built) + domain-specific analysis logic |

**Key reframe**: Don't create `TradingReflect`. Instead, make the episode logger *extensible* — it already records agent turns. Add a post-processing hook that computes domain-specific metrics from episodes:

```
Episodes (generic: agent turn records)
  → DomainPostProcessor.process(episodes) → Engram<DomainMetrics>
  → DomainMetrics stored in substrate
  → Queried by learning loops, cascade router, dreams
```

For trading: `TradingPostProcessor` computes P&L, Sharpe, drawdown.
For governance: `GovernancePostProcessor` computes vote alignment, quorum participation rate.
For CI: `CIPostProcessor` computes build success rate, mean time to fix, coverage delta.

### 8. VenueAdapter → Connector Trait (Generalized)

**This is the one place where a new trait is justified**, but it should be *much* broader than `VenueAdapter`:

| Offchain | Roko Generalization |
|----------|-------------------|
| `VenueAdapter.connect()` | `Connector.connect()` — establish session with any external system |
| `VenueAdapter.get_snapshot()` | `Connector.query(params) → Engram` — read state from any external system |
| `VenueAdapter.place_order()` | `Connector.execute(action) → Engram` — write to any external system |
| 4 implementations (HyperLiquid, Nunchi, mock, factory) | N implementations: DEX, CEX, RPC, REST API, GitHub, Slack, database, etc. |

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    /// Unique identifier for this connector type (e.g., "hyperliquid", "github", "postgres")
    fn kind(&self) -> &str;

    /// Establish session / verify reachability
    async fn connect(&self, config: &ConnectorConfig) -> Result<()>;

    /// Read state — returns domain-specific data as Engram
    async fn query(&self, request: &Engram, ctx: &Context) -> Result<Vec<Engram>>;

    /// Write / execute action — returns result as Engram
    async fn execute(&self, action: &Engram, ctx: &Context) -> Result<Engram>;

    /// Health check
    async fn health(&self) -> Result<ConnectorHealth>;
}
```

This subsumes `VenueAdapter` (trading), but also covers:
- GitHub connector: `query` = list PRs, `execute` = merge PR
- Slack connector: `query` = read messages, `execute` = post message
- Database connector: `query` = SELECT, `execute` = INSERT/UPDATE
- Chain RPC connector: `query` = eth_call, `execute` = send_tx

Tool handlers delegate to `Connector` implementations. Adding a new external system = implement one trait, get all tools automatically via the tool registry.

### 9. MCP Server → Domain-Specific Tool Packs

Don't create `roko-mcp-defi`. Instead, make tool registration dynamic:

| Offchain | Roko Composition |
|----------|-----------------|
| 16 FastMCP tools | Tool definitions in config, handlers via `Connector` + gate pipeline |
| Strategy-specific tools | Agent's `tools` allowlist in config — each agent sees only its relevant tools |
| Analysis tools | `Scorer` results exposed as MCP tool responses |

**Pattern**: Each `Connector` registers its tools into the tool registry at startup. A Hyperliquid connector registers `venue.place_order`, `venue.cancel_order`, etc. A GitHub connector registers `github.create_pr`, `github.merge`, etc. The MCP server exposes whatever tools are registered — no domain-specific MCP crate needed.

### 10. On-Chain Jobs → TickPolicy Agents

| Offchain | Roko Composition |
|----------|-----------------|
| 11 keeper strategies | 11 `AgentDefinition` configs with `tick_interval_ms` + restricted tool sets |
| Stateless poll → compute → post | `TickPolicy` that queries `Connector`, runs through pre-action gates, executes via `Connector` |
| Keeper configuration | Standard agent config: interval, tools, pre-action gates, connector reference |

**Not keeper-specific infrastructure.** A "keeper" is just a tick-driven agent with a chain connector. Same mechanism works for:
- Cron-like off-chain jobs (data sync, cache warm, health check)
- Continuous monitoring (log tailing, metric alerting)
- Scheduled governance operations

### 11. Risk Management → Hierarchical Gate Composition

| Offchain | Roko Composition |
|----------|-----------------|
| Per-wallet `WalletRiskGate` | `Gate` impl scoped to connector instance (one gate per wallet/account) |
| House-level `HouseRiskGate` | `CompositeGate` that aggregates verdicts across all scoped gates |
| Per-slot `GuardBridge` | `CompositePolicy` slot-level gate state (see P0 #3 above) |
| Cascade protection | `CircuitBreaker` (already in roko-conductor) with configurable triggers |

**Generalizes to**: any hierarchical constraint system:
- Per-agent → per-team → global budget limits (token spending, API calls)
- Per-branch → per-repo → org-wide CI resource limits
- Per-account → per-chain → total cross-chain exposure limits

---

## Revised Design Pattern Mapping

| Offchain Pattern | Generalized Roko Pattern | Status | Domains |
|-----------------|-------------------------|--------|---------|
| **Pure Engine** | `Policy` trait: `decide(engrams, ctx) → actions` | Exists | All |
| **Config + State** | `roko.toml` (immutable) + `.roko/` (mutable) + Engrams in Substrate | Exists | All |
| **Venue Abstraction** | `Connector` trait: `connect/query/execute/health` | **New** | All external systems |
| **Signal Composition** | `Composer` trait: assemble N scored engrams under budget | Exists | All |
| **Multi-Slot** | `CompositePolicy`: N sub-policies with shared budget | **New** | Parallel sub-tasks in any domain |
| **Tick-Driven Loop** | `TickPolicy` + heartbeat clock | **Wire** | Continuous agents in any domain |
| **Pre-Action Pipeline** | Gate pipeline with pre-action insertion point | **Wire** | All domains need pre-checks |
| **Nightly Review** | Dreams subsystem + domain `PostProcessor` | Exists (needs trigger) | All domains with feedback loops |
| **Keeper Pattern** | TickPolicy + Connector = tick-driven external interaction | Composed from above | On-chain, API, infra monitoring |
| **Tier-based Stops** | Stateful `Gate` with progressive state machine | **New impl**, existing trait | Any progressive constraint tightening |

---

## What's New vs What's Wiring

| Category | Work | Domain-Specific? |
|----------|------|-----------------|
| **New trait** | `Connector` (external system interface) | No — universal |
| **New composition** | `CompositePolicy` (N slots + shared budget) | No — universal |
| **Wire existing** | `TickPolicy` firing on heartbeat intervals | No — universal |
| **Wire existing** | Pre-action gate pipeline insertion point | No — universal |
| **Wire existing** | Dreams scheduled trigger for nightly review | No — universal |
| **New Gate impls** | FreshnessGate, BudgetGate, RegimeGate, ThrottleGate, ScheduleGate, FeedbackGate | No — reusable across domains |
| **New Connector impls** | Hyperliquid, Nunchi, Chain RPC, GitHub, Slack, etc. | Yes — one per external system |
| **New Scorer impls** | Market signals, governance signals, CI signals | Yes — one per domain |
| **New PostProcessor impls** | P&L attribution, vote analysis, build metrics | Yes — one per domain |
| **New Agent configs** | Strategy definitions, keeper definitions | Yes — one per use case |

**Summary**: 4 pieces of universal infrastructure (Connector trait, CompositePolicy, TickPolicy wiring, pre-action gates). Everything else is domain-specific *content* plugged into universal *mechanisms*.

---

## Revised Priority Mapping

| Priority | What | Type | Rationale |
|----------|------|------|-----------|
| **P0** | `Connector` trait + mock impl | New trait | Everything that talks to external systems needs this |
| **P0** | Pre-action gate pipeline | Wire | Safety before action — universal requirement |
| **P0** | TickPolicy + heartbeat wiring | Wire | Continuous agents can't work without this |
| **P1** | `CompositePolicy` (multi-slot) | New composition | Parallel sub-task management |
| **P1** | 6 universal Gate impls | New impls | Freshness, Budget, Regime, Throttle, Schedule, Feedback |
| **P1** | First domain Connectors | New impls | Chain RPC, Hyperliquid (or whatever your first target is) |
| **P2** | Domain-specific Scorers | New impls | Market signals, governance signals, etc. |
| **P2** | Domain-specific PostProcessors | New impls | P&L attribution, vote analysis, etc. |
| **P2** | Dreams scheduled trigger | Wire | Nightly review cycles |
| **P3** | Additional Connectors | New impls | GitHub, Slack, database, etc. |
| **P3** | Additional Gate impls | New impls | Domain-specific verification (simulation, audit, etc.) |

---

## DeFi-Specific Instantiation (Example)

To show how the generalized framework instantiates for the original trading bot use case:

```toml
# roko.toml — DeFi market-making agent

[connectors.hyperliquid]
kind = "hyperliquid"
endpoint = "https://api.hyperliquid.xyz"
credentials_secret = "HL_API_KEY"

[heartbeat]
gamma_ms = 50       # fast tick for MM quoting
theta_ms = 2000     # medium tick for signal processing
delta_ms = 30000    # slow tick for position review

[[agents]]
name = "avellaneda-mm"
domain = "defi.market-making"
connector = "hyperliquid"
tick = "gamma"
prompt = "..."
tools = ["connector.hyperliquid.place_order", "connector.hyperliquid.cancel_order", "connector.hyperliquid.get_snapshot"]
pre_action_gates = ["freshness:45s", "budget:max_notional=100000", "regime:vol_tiers=4", "throttle:drawdown_limit=0.05"]
post_action_gates = ["mev_detection"]

[[agents]]
name = "funding-arb"
domain = "defi.arbitrage"
connector = "hyperliquid"
tick = "delta"
prompt = "..."
tools = ["connector.hyperliquid.*"]
pre_action_gates = ["freshness:30s", "budget:max_notional=50000"]
```

The same config structure works for a governance agent, a CI agent, or any other domain — swap the connector, tools, gates, and tick rate.
