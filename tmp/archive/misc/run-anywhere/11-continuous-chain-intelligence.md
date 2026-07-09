# Continuous Chain Intelligence: The Cybernetic Feedback Loop

> **Audience**: Protocol researchers, infrastructure devs, quantitative analysts
> **Scope**: Architecture of Roko's perpetual surveillance layer, active inference loops, and 5-stage triage pipeline.

---

Roko agents do not query the blockchain "on demand." If an agent only sees the chain when it decides to look (via a tool call), it operates blindly in the intervening spaces. 

Instead, Roko employs a **Continuous Chain Intelligence** architecture—a perpetual surveillance and state management layer that ingests every block and maintains a highly precise, in-memory model of the protocols it cares about.

## The Active Inference Loop

Roko's attention model creates a closed cybernetic feedback loop (Friston, 2010): *the agent’s behavior determines what it watches, and what it watches shapes its behavior.*

No human configuration of addresses is required. The loop manages itself:

1. **Cortical State Expectation**: The agent's current positions, strategy, and semantic memory generate expectations of market events.
2. **ChainScope Encoding**: This interest list compresses into a highly optimized `BinaryFuse8` filter.
3. **Pre-Screening**: The `roko-witness` listener screens headers using POPCNT instructions (~10ns). >90% of blocks skip full parsing.
4. **Ingestion & Triage**: When the filter hits, the blocks are pulled and routed to the Triage Pipeline.

## The 4-Stage Triage Pipeline

Events undergo a rigorous, 4-stage pipeline to separate noise from actionable intelligence:

- **Stage 1 (Rule-based Fast Filter)**: Known MEV transactions and low-value noise are discarded.
- **Stage 2 (Statistical Anomaly Detection)**: MIDAS-R and DDSketch evaluate structural velocity and transaction frequency anomalies.
- **Stage 3 (Contextual Enrichment)**: Protocol state changes and ABI resolutions are mapped.
- **Stage 4 (Generative Scoring)**: HDC Fingerprinting and Bayesian Surprise models assign a precise anomaly score.

### Routing by Curiosity Score

The output of Stage 4 is a "Curiosity Score" driving the fan-out behavior:
- **Score > 0.8**: Creates a high-priority `RokoEvent::TriageAlert`, pushing it to the LLM context queue instantly.
- **Score 0.5 - 0.8**: Logs a `RokoEvent::ChainEvent`, updating protocol state without an immediate LLM interrupt.
- **Score 0.2 - 0.5**: Updates internal protocol state (silent state sync).
- **Score < 0.2**: Discarded to the audit log.

## Unified State Storage (Local-First)

The architecture prevents the LLM from executing raw, lagging RPC calls. An embedded multi-layer state ensures a 60fps local TUI render loop natively out of Rust:

- **DashMap (Hot Layer)**: Atomic swaps corresponding to extreme-velocity execution parameters.
- **Redb (Warm Layer)**: Indexed state updates on disk maintaining immediate history of protocol state variants.
- **HNSW / uSearch**: Highly tuned Approximate Nearest Neighbor search handling the hyperdimensional codes of contract fingerprints.

The Chain Intelligence layer operates in asynchronous parallel with the cognitive `Heartbeat`. It guarantees that when the agent is ready to parse its `OBSERVE` step, it is reading a perfectly aligned, noise-isolated cross-section of the entire relevant blockchain memory.

---

## The BinaryFuse8 Pre-Screening Layer (Exact Specs)

### Filter Parameters
- **Filter type**: BinaryFuse8 (xorf crate v0.11)
- **Bits per entry**: 8.7 (vs 9.6 for Bloom, 9.1 for xor, 8.5 for cuckoo)
- **False positive rate**: <1%
- **Hash function**: xxh3_64
- **Reconstruction**: Immutable-rebuild via `arc-swap` pattern (rebuilt each Gamma tick from ChainScope interest list)
- **Check cost**: O(1) via POPCNT, ~10ns per block header

### How It Works

The `logsBloom` field in every Ethereum block header is a 2048-bit Bloom filter over all log topics and emitting addresses in that block. The BinaryFuse8 pre-screen checks whether ANY of the agent's interest topics appear in the block's logsBloom.

```
Block header arrives (via eth_subscribe("newHeads"))
  → Extract logsBloom (2048 bits)
  → Check against ChainScope's BinaryFuse8 filter
  → MISS (>90% of blocks): Skip entirely, update gas_gwei only
  → HIT: Fetch full block + receipts → enter Triage Pipeline
```

### Connection Architecture
- **Subscription connection**: Dedicated WebSocket, isolated (never starved by burst activity)
- **Query pool**: 4 HTTP connections (configurable) for block/receipt fetching
- **Fallback**: HTTP when WebSocket pool saturates
- **Reconnection backoff**: 3s → 6s → 12s → 30s max

### Gap Detection
- **Tracking**: Roaring Bitmap (Lemire et al., 2016) of seen block numbers
- **Retention**: 90 days of block numbers
- **Small gap (≤1,000 blocks)**: Backfill via `eth_getLogs` with filter
- **Large gap (>1,000 blocks)**: Emit `ChainGapDetected` event, resume from head, accept permanent awareness hole
- **Metric**: `witness.filter_hits / (hits + misses)` — alert if >20% (filter too permissive)

**Research**: Binary Fuse filters (Lemire et al., 2022) — 2× faster construction than xor filters. Roaring Bitmaps (Lemire et al., 2016) — compressed bitmap for set membership.

---

## The 4-Stage Triage Pipeline (Expanded)

### Stage 1: Rule-Based Fast Filter
- Discard known MEV bot transactions (sandwich attacks, backruns)
- Filter out dust transfers below configurable threshold
- Whitelist/blacklist by contract address
- **Cost**: ~100μs per transaction (pure string matching)

### Stage 2: Statistical Anomaly Detection
- **MIDAS-R**: Streaming anomaly detection on edge streams (detects unusual transaction patterns in real-time)
- **DDSketch**: Quantile estimation for transaction value distributions (detects volume anomalies)
- **Count-Min Sketch**: Frequency estimation for address activity (detects unusual actors)
- **Cost**: ~1ms per transaction batch

### Stage 3: Contextual Enrichment
- Resolve contract ABIs from local cache or on-chain `getCode`
- Map function selectors to human-readable names
- Track protocol state changes (position updates, rate changes, liquidations)
- Link transactions to known protocol activities (Uniswap swaps, Aave deposits, Morpho supplies)
- **Cost**: ~5ms per enriched transaction

### Stage 4: Generative Scoring
- **HDC Fingerprinting**: Encode transaction pattern as 10,240-bit BSC vector
- **Similarity search**: Compare against known-interesting patterns in Superposition Memory
- **Bayesian Surprise**: `surprise = KL_divergence(posterior || prior)` — how much does this observation change the agent's beliefs?
- **Composite curiosity score**: weighted combination of HDC similarity, Bayesian surprise, and position relevance
- **Cost**: ~1ms per scored transaction

### Curiosity Score Routing

| Score | Action | Latency Impact |
|---|---|---|
| **> 0.8** | High-priority `RokoEvent::TriageAlert` → LLM context queue immediately | Agent processes next Theta tick |
| **0.5 - 0.8** | `RokoEvent::ChainEvent` → protocol state update, no LLM interrupt | Available at next OBSERVE step |
| **0.2 - 0.5** | Silent state sync (protocol cache updated) | Background update |
| **< 0.2** | Discarded to audit log | No impact |

---

## The 16 Deterministic Probes (T0, Zero LLM Cost)

At every Gamma tick (~5-15s), these probes run with zero LLM involvement:

| # | Probe | What It Checks | Trigger |
|---|---|---|---|
| 1 | Price delta | Significant price movement since last tick | |
| 2 | TVL delta | Liquidity changes in monitored pools | |
| 3 | Position health | Health factor of lending positions | < 1.3 |
| 4 | Gas spike | Gas price exceeds threshold | > 3× baseline |
| 5 | Credit balance | Budget remaining vs burn rate | < 20% remaining |
| 6 | RSI | Relative Strength Index extreme | > 70 or < 30 |
| 7 | MACD | Moving Average Convergence Divergence crossover | Signal cross |
| 8 | Circuit breaker | Protocol-level circuit breaker triggered | Any protocol |
| 9 | Kill switch | Owner-initiated emergency halt | Owner signal |
| 10 | Replicant report | Sibling agent alerts | Clade signal |
| 11 | Clade alert | Clade-wide signals | Network signal |
| 12 | Homeostatic drift | Portfolio drifting from target allocation | > 5% drift |
| 13 | World model drift | Environment diverging from agent's model | High residual |
| 14 | Causal consistency | Causal graph predictions matching reality | Violated |
| 15 | VPIN | Volume-synchronized probability of informed trading | > 0.7 |
| 16 | IL threshold | Impermanent loss exceeding tolerance | > configured max |

These probes compute **prediction error** — a single scalar (0.0-1.0) that determines whether the agent needs to think (T1/T2) or can stay on autopilot (T0).

---

## Unified State Storage (Expanded)

### Four-Tier Storage Architecture

| Tier | Engine | Data | Access Pattern | Size |
|---|---|---|---|---|
| **Hot** | DashMap (concurrent HashMap) | CorticalState, latest prices, position health | Atomic reads, 60fps TUI render | ~10 MB |
| **Warm** | redb (ACID, copy-on-write B-tree) | Protocol state history, triage traces | Indexed queries, crash recovery | ~500 MB |
| **Vector** | usearch / LanceDB | HDC fingerprints, float embeddings | Similarity search | ~250 MB |
| **Cold** | Parquet + Zstd (87% compression) | Historical episodes, full block data | Batch analysis, DataFusion SQL | ~5 GB |

**Total 6-month budget**: ~8 GB for ~100 monitored protocols.

### State Isolation from LLM

The LLM never makes raw RPC calls. When the Heartbeat enters the OBSERVE step, it reads from the pre-built local state — a perfectly aligned, noise-isolated snapshot of all relevant blockchain data, already triaged and scored.

This means:
- **Zero RPC latency** during cognition (all data pre-fetched)
- **No stale data** (state updated every Gamma tick)
- **No token waste** on raw blockchain exploration (already indexed)
- **60fps TUI render** from DashMap hot state (no blocking I/O)

---

## The CorticalState Connection

The Chain Intelligence layer writes to the CorticalState (32 atomic signals, lock-free):

| Signal | Writer | Update Frequency |
|---|---|---|
| `environment.gas_gwei` | Witness | Every block header |
| `environment.regime` | Domain probes | Per regime change |
| `environment.regime_confidence` | Domain probes | Per Gamma tick |
| `chain.blocks_behind` | Witness | Per Gamma tick |

Other subsystems (Daimon, Mortality, Inference) read these signals without locks, using `Ordering::Acquire` for cross-group consistency.

**Research**: Active Inference (Friston, 2010) — the agent maintains a generative model of its environment and acts to reduce free energy (prediction error). The ChainScope dynamically adjusts what the Witness watches based on where prediction error is highest — closing the perception-action loop.

---

## Viable Systems Model Mapping (Beer, 1972)

The chain intelligence layer implements Stafford Beer's Viable System Model (VSM):

| VSM System | Chain Layer | Function |
|---|---|---|
| **System 1** (Operations) | Agent heartbeat loop | Actual DeFi operations (trades, LP, lending) |
| **System 2** (Coordination) | Clade sync via Korai | Resolve conflicts between sibling agents |
| **System 3** (Control) | Conductor watchers | Monitor operational performance, intervene on anomaly |
| **System 4** (Intelligence) | Chain Intelligence layer | Scan environment, detect opportunities/threats |
| **System 5** (Policy) | PolicyCage + operator | Set boundaries, define strategy, approve major decisions |

**Key insight**: The Chain Intelligence layer is System 4 — it maintains the agent's model of the external world. Without it, the agent operates reactively (wait for problems). With it, the agent operates **proactively** (detect problems before they affect positions).

---

## Quorum Sensing (Population-Level Coordination)

**Research**: In biology, quorum sensing allows bacteria to coordinate behavior based on population density. When enough individuals detect the same signal, collective action triggers.

Applied to the agent network:

### Pheromone Accumulation

When multiple agents independently detect the same threat (e.g., "Aave V3 on Base has anomalous withdrawal patterns"):

1. Agent A deposits a THREAT pheromone with HDC fingerprint
2. Agent B independently detects the same pattern, deposits a similar pheromone
3. The Korai relay detects >0.6 Hamming similarity between deposits
4. The deposits **bundle** (majority-vote) into a stronger signal
5. When accumulated signal strength exceeds quorum threshold → population-level alert

### Quorum Thresholds

| Pheromone Type | Quorum (% of active agents confirming) | Action |
|---|---|---|
| THREAT | 3 agents or 5% | Broadcast alert, increase position monitoring |
| OPPORTUNITY | 5 agents or 10% | Share discovery, individual agents decide to act |
| WISDOM | 10 agents or 20% | Promote to population-level heuristic |

### No Central Coordination

Agents don't vote. They don't message each other. They deposit HDC-encoded observations in a shared field and read the field on every tick. **O(1) coordination cost per agent** — scales to any population size.

The accumulation happens automatically via BSC majority-vote bundling. Similar observations reinforce each other; dissimilar observations remain separate signals. No protocol needed — the math IS the coordination mechanism.

---

## Constructal Law (Bejan, 2000)

**Research**: Constructal Law states that systems evolve toward configurations that maximize flow access. Applied to the knowledge network:

- **Early network**: Many small, isolated knowledge clusters with poor cross-domain flow
- **Mature network**: Fewer, larger channels connecting major knowledge domains, with smaller tributaries feeding into them
- **Optimal**: Tree-like branching structure maximizing knowledge throughput per unit cost

The chain's HDC search automatically produces this structure: highly-confirmed entries become "main channels" (high weight, many links). Niche entries become "tributaries" (lower weight, fewer links, but connecting specific domains to the main flow).

**Measurement**: Track the ratio of "channel width" (confirmation count) to "tributary count" (number of entries linking to a hub). When this ratio follows a power law (Zipf distribution), the network has self-organized into optimal flow structure.

---

## The Adaptive Clock Mechanism

The Chain Intelligence layer operates on its own adaptive clock, separate from the Heartbeat:

### Gamma Acceleration on Violation

```
gamma_interval = max(5s, 15s / (1.0 + violations × 0.3))
```

Under normal conditions: gamma ticks every 15 seconds. When a probe detects a violation (health factor dropping, unusual volume spike), gamma accelerates to 5 seconds — the agent perceives the environment 3× faster.

### Theta Acceleration on Volatility

When the VPIN (Volume-synchronized Probability of Informed Trading) probe exceeds 0.7:
- Theta interval decreases from 120s to 30s
- The agent's cognitive cycle runs 4× faster
- More T1/T2 calls are made (cost increases, but survival may require it)

### Budget Throttling

Both clocks have a daily budget ceiling. If Gamma has consumed its budget, it slows to the minimum rate. If Theta's inference budget is exhausted, it falls back to T0 only.

```
daily_budget = daily_revenue_estimate × 0.6  // spend 60% of expected earnings
remaining = daily_budget - spent_today
if remaining < 0 → T0 only (zero inference spending)
```

This ensures the agent cannot accidentally spend more than it earns.

---

## The Protocol State Cache (Local-First)

### What Gets Cached

For each monitored protocol, the chain intelligence layer maintains a live cache of:

| Protocol Type | Cached State | Update Trigger |
|---|---|---|
| **Uniswap V3/V4** | Pool slot0 (sqrtPriceX96, tick, liquidity), positions, fee growth | Any swap/mint/burn event |
| **Aave V3** | Reserve data (supply/borrow rates, utilization), user positions, health factors | Any deposit/borrow/repay/liquidation |
| **Morpho Blue** | Market params, supply/borrow shares, oracle prices, vault allocations | Any supply/withdraw/borrow event |
| **Lido/RocketPool** | Exchange rates, buffer balances, validator counts | Rate oracle updates |
| **EigenLayer** | Restaking amounts, withdrawal queues, slashing events | Any stake/unstake event |

### Cache Invalidation

The cache is **event-driven**, not polling:
1. Witness layer detects relevant log event via BinaryFuse8 filter
2. Triage pipeline classifies the event by protocol and action
3. Protocol State layer updates ONLY the affected fields
4. CorticalState signals updated if thresholds crossed (e.g., health factor < 1.3)

**No stale data**: The cache reflects every block that passes the filter. The only gap is the BinaryFuse8 false negative rate (<1% of blocks).

### Multi-Chain Support

Each chain gets its own Witness → Triage → Protocol State pipeline:

```
Base L2:        Witness → Triage → Protocol State → CorticalState
Arbitrum:       Witness → Triage → Protocol State → CorticalState
Ethereum L1:    Witness → Triage → Protocol State → CorticalState
Optimism:       Witness → Triage → Protocol State → CorticalState
```

Cross-chain state is unified in the CorticalState via chain-specific signal namespaces:
- `base.gas_gwei`, `arbitrum.gas_gwei`, `ethereum.gas_gwei`
- `base.aave_health`, `arbitrum.aave_health`

The agent's cognitive cycle reads the unified CorticalState — it doesn't need to know which chain provided which signal.

---

## The Attention Forager (Three-Tier Monitoring)

### How the Agent Decides What to Watch

The agent cannot monitor everything. The Attention Forager maintains three tiers:

| Tier | Protocols | Monitoring Depth | Prediction Frequency |
|---|---|---|---|
| **ACTIVE** | 3-8 | Full state cache, every block | Full prediction cascade per block |
| **WATCHED** | 10-20 | Header-level, every 10 blocks | Lightweight checks per 10 blocks |
| **SCANNED** | 50-100+ | Log-level only, every 100 blocks | One quick check per 100 blocks |

### Promotion/Demotion Triggers

**SCANNED → WATCHED**: Prediction violation. Agent predicted "low activity on Curve" but detected unusual volume → promote to closer monitoring.

**WATCHED → ACTIVE**: Sustained violations (3+ in a row). Something is consistently different from expectations → needs full attention.

**ACTIVE → WATCHED**: Predictions accurate for 50+ blocks. Nothing surprising → reduce monitoring to free budget for exploration.

**WATCHED → SCANNED**: Predictions boring for 200+ blocks. Stable, predictable, low-value → minimal monitoring.

### Budget Constraint

Total monitoring budget shrinks with age (resource conservation):
- Young agent: K=200 protocols monitored (broad exploration)
- Mature agent: K=50 (focused on known-valuable protocols)
- Aging agent: K=20 (concentrate on survival-critical positions)

**Research**: Optimal Foraging Theory (Stephens & Krebs, 1986) — animals allocate attention to patches by marginal value. The agent follows the same principle: monitor where the expected information gain per unit cost is highest.

---

## Protocol-Specific Intelligence Modules

The Chain Intelligence layer is only as useful as its understanding of the protocols it monitors. Raw event streams are noise without protocol-level semantics. The Protocol-Specific Intelligence Modules transform raw chain data into structured, queryable protocol state.

### ProtocolCache: Precomputed State for High-Frequency Queries

Each monitored protocol maintains a live, precomputed state cache optimized for the queries the agent actually makes:

| Protocol | Cached State | Query Examples |
|---|---|---|
| **Uniswap V3/V4** | Pool tick, sqrtPriceX96, liquidity at current tick, fee tiers, tick spacing, accumulated fees per LP range | "What's my current LP range worth?", "Is my position in range?", "What's the 24h fee APR?" |
| **Aave V3** | Per-reserve utilization rate, supply/borrow APY, liquidation threshold, LTV, health factors for tracked positions | "Am I at liquidation risk?", "Which reserve has the best borrow rate?", "Is utilization approaching the kink?" |
| **Morpho Blue** | Per-vault allocation across markets, net APY after fees, risk parameters (LLTV, oracle), supply/borrow shares | "What's my effective yield?", "How is the vault allocator distributing across markets?", "What's the liquidation buffer?" |
| **Lido/RocketPool** | stETH/ETH exchange rate, withdrawal queue depth, buffer balance, validator count, expected yield | "Is the stETH peg stable?", "How long is the withdrawal queue?", "What's the current APR?" |
| **EigenLayer** | Restaked amounts by strategy, withdrawal delay, slashing history, operator performance | "Is my operator at risk?", "How much is in the withdrawal queue?", "Any recent slashing events?" |

The cache is not a generic key-value store. Each protocol has a **typed state struct** with computed fields. When the agent queries "Am I at liquidation risk?", it reads a precomputed `health_factor: f64` — not raw contract storage slots.

### Event-Driven Cache Invalidation

The cache updates are event-driven, not polling-based:

1. **Witness layer** detects a relevant log event via the BinaryFuse8 filter
2. **Triage pipeline** classifies the event by protocol and action type (swap, deposit, liquidation, etc.)
3. **Protocol module** updates ONLY the affected fields in the typed state struct
4. **CorticalState signals** update if thresholds are crossed (health factor < 1.3, utilization > 90%, etc.)

Specific event triggers per protocol:

| Protocol | Invalidation Events | What Gets Recomputed |
|---|---|---|
| **Uniswap** | `Swap`, `Mint`, `Burn`, `PoolModified` | sqrtPrice, tick, liquidity, fee accumulator |
| **Aave** | `ReserveDataUpdated`, `LiquidationCall`, `Borrow`, `Repay` | Utilization, rates, health factors |
| **Morpho** | `SupplyCollateral`, `Withdraw`, `Borrow`, `Liquidate`, `ReallocateSupply` | Market shares, vault allocation, APY |
| **Lido** | `TokenRebased`, `WithdrawalRequested`, `WithdrawalClaimed` | Exchange rate, buffer, queue depth |
| **EigenLayer** | `StakerDelegated`, `WithdrawalQueued`, `OperatorSlashed` | Restaked amounts, queue, slashing state |

**No stale data risk**: The cache reflects every block that passes the BinaryFuse8 filter. Polling-based caches risk stale data between poll intervals. Event-driven invalidation means the cache is always consistent with the latest confirmed block.

### Protocol Grammar: Typed Schemas for DeFi Primitives

Each DeFi primitive has a typed schema that the agent reasons about at the intent level, not the calldata level:

```
Swap { pool, token_in, token_out, amount_in, min_amount_out, deadline }
AddLiquidity { pool, tick_lower, tick_upper, amount0, amount1 }
Lend { market, asset, amount, is_collateral }
Borrow { market, asset, amount, collateral_asset }
Stake { protocol, asset, amount, operator }
Unstake { protocol, asset, amount, withdrawal_delay }
```

The agent never constructs raw calldata. It expresses intents using these typed schemas. The tool layer handles ABI encoding, gas estimation, and nonce management. This separation means:

- The LLM reasons about "should I add liquidity to this range?" — a semantic question
- The tool layer handles "how do I encode a Uniswap V4 `modifyLiquidity` call?" — a mechanical question
- The gate layer verifies "did the resulting state match the intent?" — a verification question

### Cross-Protocol Reasoning

The most valuable intelligence emerges when protocol states are correlated:

**Rate arbitrage detection**: "Aave USDC borrow rate is 4.2%. Morpho Blue USDC supply rate is 6.8%. The spread covers gas + protocol risk."

**Liquidation cascade prediction**: "Aave utilization at 92% and rising. If it crosses 95%, borrow rates spike to 50%+. Positions with health factor < 1.5 on variable-rate borrows are at risk. Three of our monitored positions qualify."

**Yield regime detection**: "Lido staking yield dropped below Aave supply rate. ETH stakers are likely to unstake and supply to Aave instead. Expect Lido withdrawal queue growth and Aave ETH supply increase."

**Correlated risk identification**: "Our Morpho vault and Aave position both use the same Chainlink oracle. If that oracle fails, both positions are at simultaneous risk. The concentrated oracle dependency is a hidden correlation."

These cross-protocol inferences are precomputed during the Chain Intelligence tick — the agent receives them as structured alerts, not as raw data it needs to analyze during its cognitive cycle.

---

## The Virtual State Machine (VSM): Predictive Simulation

### Why Simulate Before Committing

DeFi transactions are irreversible. A swap executed at bad slippage, a liquidity position opened in the wrong tick range, a borrow that pushes health factor below the liquidation threshold — all produce permanent capital loss. The Virtual State Machine provides a sandbox where the agent can test strategies before committing real capital.

### Fork-Based Simulation via Anvil/TEVM

The VSM forks the current chain state locally and runs proposed transactions in simulation:

```
Chain state at block N (live)
  → Fork to local Anvil/TEVM instance
  → Execute proposed transaction(s) in fork
  → Read resulting state diff
  → Compare against expected outcome
  → Decision: commit to mainnet or abort
```

The fork is ephemeral — created per simulation request and destroyed afterward. No persistent local state beyond the simulation result.

### Simulation Types

| Type | Description | Use Case | Cost |
|---|---|---|---|
| **Single-tx** | Will this swap execute at acceptable slippage? | Pre-trade validation | ~5ms |
| **Multi-step** | Will this lending loop (deposit → borrow → swap → deposit) be profitable after gas? | Strategy validation | ~20ms |
| **Counterfactual** | What WOULD have happened if I'd acted 5 blocks ago? | Regret analysis, strategy improvement | ~50ms |
| **Stress test** | What happens to my positions if ETH drops 20% in one block? | Risk assessment | ~100ms |

### Simulation Budget by Decision Tier

Not all decisions warrant simulation. The simulation budget is allocated by decision tier:

| Decision Tier | Simulation Budget | Rationale |
|---|---|---|
| **T0** (deterministic probes) | 0 simulations | No LLM involvement, no capital commitment |
| **T1** (fast inference) | 1 simulation (cached paths only) | Quick validation of well-understood strategies |
| **T2** (deep reasoning) | Up to 3 simulations | Full strategy exploration with fallback options |
| **T3** (human-escalated) | Unlimited (operator decides) | High-stakes decisions get thorough simulation |

### Predicted State Diffs

Each simulation produces a **predicted state diff** — a structured representation of how the world would change:

```
StateDiff {
    balance_changes: [(token, delta)],
    position_changes: [(protocol, position_id, field, old_value, new_value)],
    health_factor_impact: f64,
    gas_cost: u64,
    net_pnl_estimate: f64,
    risk_delta: f64,
}
```

The state diff feeds directly into the decision pipeline. The agent doesn't evaluate "should I do this trade?" in the abstract — it evaluates a concrete, simulated outcome with precise numbers.

### Simulation Cache

Common transaction paths are cached to avoid redundant fork operations:

- "Swap X USDC for ETH on Uniswap V4 pool 0x..." → cached if pool state hasn't changed since last simulation
- Cache key: `(pool_address, block_number, action_type, amount_bucket)`
- Amount bucketing: round to nearest 10% to increase cache hit rate without sacrificing accuracy
- Cache invalidation: on any relevant pool event (same as ProtocolCache invalidation)

---

## Quorum Sensing: Collective Threat Detection

### From Individual Perception to Collective Awareness

A single agent's threat detection is limited by its monitoring scope and inference budget. But when multiple agents independently observe the same anomaly, the collective signal is far stronger than any individual observation. This is quorum sensing — population-level intelligence from decentralized individual observations.

### How Quorum Sensing Works

1. **Agent A** detects anomalous gas patterns on Base L2 (gas spikes correlated with large Aave liquidations)
2. **Agent B** independently detects unusual withdrawal volume from Aave on Base
3. **Agent C** detects MEV bot activity concentrated around Aave positions on Base
4. Each agent deposits a **pheromone** in the shared Korai field with an HDC fingerprint encoding the observation
5. The Korai relay detects >0.6 Hamming similarity between the three deposits
6. The deposits **bundle** via BSC majority-vote into a single, stronger THREAT signal
7. The bundled signal exceeds the quorum threshold (3 agents) → **population-level THREAT alert**

### Quorum Thresholds

| Pheromone Type | Threshold | Half-Life | Action on Quorum |
|---|---|---|---|
| **THREAT** | 3 agents OR 5% of active population within 2-minute window | 2 hours | Broadcast alert, increase position monitoring frequency, tighten risk parameters |
| **OPPORTUNITY** | 5 agents OR 10% of active population within 5-minute window | 6 hours | Share discovery, individual agents evaluate independently |
| **WISDOM** | 10 agents OR 20% of active population within 1-hour window | 30 days | Promote to population-level heuristic in neuro |

### Why Short Half-Lives for THREAT

Market dangers are fast-moving. A gas spike that threatened liquidations 3 hours ago is no longer relevant — conditions have changed. THREAT pheromones decay with a 2-hour half-life to prevent stale danger signals from causing unnecessary conservatism.

OPPORTUNITY pheromones have a 6-hour half-life — opportunities persist longer than threats but still expire as market conditions evolve.

WISDOM pheromones have a 30-day half-life — validated insights about market structure persist much longer than tactical signals.

### HDC-Encoded Similarity (No String Matching)

Pheromones are not labeled with text strings. They are HDC-encoded binary vectors. Two observations about "Aave liquidation risk on Base" will have similar HDC fingerprints even if the agents use completely different internal representations, because the underlying chain events (the actual log topics, addresses, and values) produce similar encodings.

Similarity is computed via Hamming distance on the BSC vectors. Observations with >0.6 similarity bundle automatically via majority-vote. No taxonomy. No ontology. No string matching. The math IS the coordination.

### Zero Coordination Overhead

Agents don't message each other. They don't vote. They don't negotiate. Each agent independently:
1. Observes chain state through its own monitoring pipeline
2. Deposits pheromones encoding its observations
3. Reads the pheromone field on every tick
4. Acts on bundled signals that exceed quorum thresholds

**O(1) coordination cost per agent** — adding the 1,000th agent to the network costs no more coordination overhead than the 10th. The pheromone field scales to arbitrary population sizes.

**Research**: Miller & Bassler (2001) — quorum sensing in bacteria. Individual cells produce and detect signaling molecules (autoinducers). When local concentration exceeds a threshold, gene expression changes coordinate population-level behavior. No central coordinator. No communication protocol. Just chemistry.

---

## The Adaptive Clock: Variable Tick Frequency

### Not All Ticks Are Equal

Running inference on every block is wasteful during calm markets and insufficient during volatile ones. The Adaptive Clock adjusts tick frequency based on market conditions, concentrating computational resources where they matter most.

### Clock Behavior by Market Regime

| Market Condition | Prediction Error | Tick Frequency | Monitoring Depth | Inference Tier |
|---|---|---|---|---|
| **Calm** (low vol, low error) | < 0.1 | Every 6th block (~72s) | Header-only for WATCHED | T0 only (zero LLM cost) |
| **Normal** (moderate vol) | 0.1 - 0.3 | Every 3rd block (~36s) | Standard pipeline | T0 + occasional T1 |
| **Elevated** (rising vol, rising error) | 0.3 - 0.6 | Every block (~12s) | Full pipeline | T0 + T1 + selective T2 |
| **Volatile** (high vol, high error) | > 0.6 | Every block + mempool monitoring | Full pipeline + mempool scan | T0 + T1 + T2 on every tick |

### The Prediction Error Driver

The clock frequency is driven by the **prediction error** signal from the CorticalState. At each tick, the 16 deterministic probes generate predictions about the next tick's state. The difference between predicted and actual state is the prediction error.

```
prediction_error = mean(|predicted[i] - actual[i]| for i in probe_signals)

tick_interval = base_interval / (1.0 + prediction_error × acceleration_factor)
```

Low prediction error means the agent's model of the world is accurate — it doesn't need to update frequently. High prediction error means the world is surprising — the agent needs to perceive more frequently to update its model.

This is a direct implementation of active inference (Friston, 2010): the agent allocates perceptual resources proportional to prediction error, minimizing free energy.

### Mempool Monitoring (Volatile Regime Only)

In the volatile regime (prediction error > 0.6), the Adaptive Clock activates mempool monitoring:

- Subscribe to pending transactions via `eth_subscribe("newPendingTransactions")`
- Filter for transactions targeting monitored protocols
- Pre-simulate large transactions that could affect positions (via VSM)
- Generate early-warning signals BEFORE transactions confirm

Mempool monitoring is expensive (high bandwidth, high false positive rate), so it only activates when market conditions justify the cost.

### Cost Implications

The Adaptive Clock produces dramatic cost savings:

| Market Regime | % of Time (typical) | % of Inference Cost | Cost per Day (est.) |
|---|---|---|---|
| **Calm** | ~60% | ~5% | ~$0.05 |
| **Normal** | ~25% | ~15% | ~$0.15 |
| **Elevated** | ~10% | ~30% | ~$0.30 |
| **Volatile** | ~5% | ~50% | ~$0.50 |
| **Total** | 100% | 100% | ~$1.00 |

Compare with fixed-frequency ticking at every block: ~$4.00/day. The Adaptive Clock reduces inference cost by ~75% while providing BETTER coverage during volatile periods (when it matters most).

### Budget Ceiling Integration

The Adaptive Clock respects the daily budget ceiling:

```
remaining_budget = daily_budget - spent_today
if remaining_budget < daily_budget × 0.1 → force Calm mode (T0 only)
if remaining_budget < 0 → suspend all inference, T0 probes only
```

The agent cannot accidentally overspend by entering a prolonged volatile regime. Budget exhaustion forces a graceful degradation to deterministic-only monitoring.

**Research**: Active Inference (Friston, 2010) — organisms allocate perceptual and computational resources proportional to prediction error. The Adaptive Clock is a direct implementation: low surprise → low tick rate, high surprise → high tick rate. Bayesian Surprise (Itti & Baldi, 2009) — quantifying how much an observation changes the agent's beliefs, used as the prediction error signal driving clock frequency.
