# DeFi Batch Execution Order

> **Source**: Aggregated from agent-executable work batches in docs 01–10
> **Total batches**: 40 | **Total work items**: ~180
> **Legend**: S (<1 day) | M (1-3 days) | L (3-7 days) | XL (1-2 weeks)

---

## Batch Index

| ID | Title | Effort | Crate | Depends on | Doc |
|----|-------|--------|-------|------------|-----|
| 0.1 | Mirage-rs integration toolkit for DeFi agents | M | roko-chain | none | 00 |
| 1.1 | Implement `get_logs` on `AlloyChainClient` | S | roko-chain | none | 01 |
| 1.2 | WebSocket subscription + event bus integration | L | roko-chain | 1.1 | 01 |
| 1.3 | Triage pipeline enrichment | M | roko-chain | 1.2 | 01 |
| 1.4 | Protocol state cache | M | roko-chain | 1.1 | 01 |
| 1.5 | Wallet registry for multi-wallet management | M | roko-chain | 1.1 | 01 |
| 1.6 | Heartbeat chain lag suppression | S | roko-chain | 1.2 | 01 |
| 2.1 | VenueAdapter trait + mock implementation | M | roko-chain | 1.2 | 02 |
| 2.2 | DeFi tool handlers (chain primitives + protocol adapters) | L | roko-chain, roko-std | 2.1 | 02 |
| 2.3 | Wire chain handlers into HandlerRegistry | S | roko-std | 2.2 | 02 |
| 2.4 | Analysis and data-query tool definitions | M | roko-chain | 2.1 | 02 |
| 2.5 | Wallet tool handlers | S | roko-chain | 1.5, 2.2 | 02 |
| 3.1 | Classical indicator expansion | S | roko-learn | none | 03 |
| 3.2 | DeFi-native indicators | L | roko-learn | 1.1 | 03 |
| 3.3 | Microstructure indicators | L | roko-learn | 1.1 | 03 |
| 3.4 | On-chain signals and sentiment | M | roko-learn | 1.1 | 03 |
| 3.5 | Volatility and regime detection | M | roko-learn | 3.1 | 03 |
| 3.6 | HDC composite indicators and market state encoding | L | roko-learn | 3.1, 3.2, 3.3, 3.4, 3.5 | 03 |
| 4.1 | DeFi risk limits and position tracking | L | roko-agent | 2.1 | 04 |
| 4.2 | MEV protection pipeline | L | roko-chain | 1.2 | 04 |
| 4.3 | Custody controls and transaction lifecycle | L | roko-chain | 4.1 | 04 |
| 4.4 | DeFi circuit breakers | M | roko-conductor | 3.1 | 04 |
| 5.1 | Archetype registry and manifest loader | L | roko-agent | none | 05 |
| 5.2 | First five DeFi archetypes | L | roko-agent | 5.1, 2.1 | 05 |
| 5.3 | Delegation DAG and tool profile resolver | L | roko-agent | 5.1, 5.2 | 05 |
| 6.1 | Wire heartbeat clock to DeFi consumers | M | roko-runtime | 1.2 | 06 |
| 6.2 | 9-step decision pipeline | XL | roko-runtime | 6.1, 5.1 | 06 |
| 6.3 | Regime detection and adaptive threshold | L | roko-runtime | 3.1 | 06 |
| 6.4 | DeFi conductor watchers | L | roko-conductor | 6.1, 3.1 | 06 |
| 7.1 | TradingReflect — FIFO P&L attribution | L | roko-learn | 2.1 | 07 |
| 7.2 | Indicator accuracy tracking | M | roko-learn | 3.1, 7.1 | 07 |
| 7.3 | Regime detection and strategy learning | L | roko-learn | 3.1, 7.1 | 07 |
| 7.4 | Trading playbooks | M | roko-learn | 7.1 | 07 |
| 7.5 | Risk-adjusted reward signal | M | roko-learn | 7.1 | 07 |
| 8.1 | PAD mapping from P&L and loss aversion | M | roko-daimon | 7.1 | 08 |
| 8.2 | Affect-to-position-sizing and tilt detection | M | roko-daimon, roko-agent | 8.1, 4.1 | 08 |
| 8.3 | Somatic-TA HDC binding and strategy space | L | roko-daimon, roko-primitives | 8.1, 3.6 | 08 |
| 9.1 | Chain triggers, counterfactual trade replay, threat rehearsal | L | roko-dreams | 7.1, 1.2 | 09 |
| 9.2 | Strategy discovery, dream journal, regime transition dreams | L | roko-dreams | 9.1, 3.1 | 09 |
| 10.1 | Market state HDC encoding and Ebbinghaus | M | roko-neuro | 3.1 | 10 |
| 10.2 | Knowledge-informed model routing and regime classification | M | roko-neuro, roko-learn | 10.1, 7.1 | 10 |

---

## Topological Execution Order

Batches are grouped into execution tiers. All batches within a tier can run **in parallel**. A tier cannot start until all batches in prior tiers have completed.

### Tier 0 — No Dependencies (run immediately, in parallel)

| ID | Title | Effort | Crate |
|----|-------|--------|-------|
| **0.1** | Mirage-rs integration toolkit | M | roko-chain |
| **1.1** | Implement `get_logs` on `AlloyChainClient` | S | roko-chain |
| **3.1** | Classical indicator expansion | S | roko-learn |
| **5.1** | Archetype registry and manifest loader | L | roko-agent |

> 4 batches, all independent. Start here.
>
> **Batch 0.1** provides the mirage-rs integration toolkit: `MirageTestHarness` wrapping `spawn_mirage_test_instance()`, `MirageSimulator` implementing the `TxSimulator` trait from `heartbeat_ext.rs`, helper functions (`fork_at_block()`, `simulate_swap()`, `simulate_lp_add()`, `get_pool_state_at_block()`), and integration test patterns showing how any batch can use mirage for testing. Added to `roko-chain` as a dev feature.

### Tier 1 — Depends only on Tier 0

| ID | Title | Effort | Depends on |
|----|-------|--------|------------|
| **1.2** | WebSocket subscription + event bus | L | 1.1 |
| **1.4** | Protocol state cache | M | 1.1 |
| **1.5** | Wallet registry | M | 1.1 |
| **3.2** | DeFi-native indicators | L | 1.1 |
| **3.3** | Microstructure indicators | L | 1.1 |
| **3.4** | On-chain signals and sentiment | M | 1.1 |
| **3.5** | Volatility and regime detection | M | 3.1 |
| **6.3** | Regime detection and adaptive threshold | L | 3.1 |
| **10.1** | Market state HDC encoding + Ebbinghaus | M | 3.1 |

> 9 batches. All can run in parallel once their single Tier 0 dep completes.

### Tier 2 — Depends on Tier 1

| ID | Title | Effort | Depends on |
|----|-------|--------|------------|
| **1.3** | Triage pipeline enrichment | M | 1.2 |
| **1.6** | Heartbeat chain lag suppression | S | 1.2 |
| **2.1** | VenueAdapter trait + mock | M | 1.2 |
| **3.6** | HDC composite indicators | L | 3.1-3.5 |
| **4.2** | MEV protection pipeline | L | 1.2 |
| **4.4** | DeFi circuit breakers | M | 3.1 |
| **6.1** | Wire heartbeat to DeFi consumers | M | 1.2 |

> 7 batches. 2.1 is critical — it unlocks tools, safety, and learning.

### Tier 3 — Depends on Tier 2

| ID | Title | Effort | Depends on |
|----|-------|--------|------------|
| **2.2** | DeFi tool handlers | L | 2.1 |
| **2.4** | Analysis tool definitions | M | 2.1 |
| **4.1** | DeFi risk limits + position tracking | L | 2.1 |
| **5.2** | First five DeFi archetypes | L | 5.1, 2.1 |
| **6.4** | DeFi conductor watchers | L | 6.1, 3.1 |
| **7.1** | TradingReflect — FIFO P&L | L | 2.1 |

> 6 batches. 7.1 is critical — it unlocks all learning, daimon, and dreams.

### Tier 4 — Depends on Tier 3

| ID | Title | Effort | Depends on |
|----|-------|--------|------------|
| **2.3** | Wire handlers into HandlerRegistry | S | 2.2 |
| **2.5** | Wallet tool handlers | S | 1.5, 2.2 |
| **4.3** | Custody controls + tx lifecycle | L | 4.1 |
| **5.3** | Delegation DAG + tool profiles | L | 5.1, 5.2 |
| **6.2** | 9-step decision pipeline | XL | 6.1, 5.1 |
| **7.2** | Indicator accuracy tracking | M | 3.1, 7.1 |
| **7.3** | Regime detection + strategy learning | L | 3.1, 7.1 |
| **7.4** | Trading playbooks | M | 7.1 |
| **7.5** | Risk-adjusted reward | M | 7.1 |
| **8.1** | PAD mapping from P&L | M | 7.1 |
| **9.1** | Chain triggers + counterfactual replay | L | 7.1, 1.2 |
| **10.2** | Knowledge routing + regime classification | M | 10.1, 7.1 |

> 12 batches. Largest tier — many items unlocked by 7.1 completing.

### Tier 5 — Final Integration

| ID | Title | Effort | Depends on |
|----|-------|--------|------------|
| **8.2** | Affect-to-position-sizing + tilt | M | 8.1, 4.1 |
| **8.3** | Somatic-TA HDC binding + strategy space | L | 8.1, 3.6 |
| **9.2** | Strategy discovery + dream journal | L | 9.1, 3.1 |

> 3 batches. Final integration layer.

---

## Critical Path

The longest dependency chain determines minimum calendar time:

```
1.1 (S)  →  1.2 (L)  →  2.1 (M)  →  7.1 (L)  →  8.1 (M)  →  8.2 (M)
                                       │
                                       ├→  9.1 (L)  →  9.2 (L)
                                       ├→  7.2 (M)
                                       ├→  7.3 (L)
                                       └→  10.2 (M)
```

**Critical path length**: 1.1 → 1.2 → 2.1 → 7.1 → 9.1 → 9.2 = **S + L + M + L + L + L ≈ 20-30 days**

The alternative critical path through the heartbeat:

```
1.1 (S)  →  1.2 (L)  →  6.1 (M)  →  6.2 (XL)
```

**Heartbeat path length**: S + L + M + XL ≈ **15-25 days** (parallel with learning path)

> **Batch 0.1** (Mirage-rs integration toolkit) runs in parallel with all Tier 0 batches. It is not on the critical path but unblocks integration testing across every subsequent tier. Start it alongside 1.1/3.1/5.1.

---

## Effort Distribution

| Effort | Count | Description |
|--------|-------|-------------|
| S | 6 | < 1 day each |
| M | 17 | 1-3 days each (includes 0.1) |
| L | 17 | 3-7 days each |
| XL | 1 | 1-2 weeks |
| **Total** | **41** | |

### By crate

| Crate | Batches | IDs |
|-------|---------|-----|
| roko-chain | 14 | 0.1, 1.1-1.6, 2.1-2.2, 2.4-2.5, 4.2-4.3 |
| roko-learn | 11 | 3.1-3.6, 7.1-7.5 |
| roko-agent | 5 | 4.1, 5.1-5.3, 8.2* |
| roko-runtime | 4 | 6.1-6.3, 6.2 |
| roko-conductor | 2 | 4.4, 6.4 |
| roko-daimon | 3 | 8.1-8.3 |
| roko-dreams | 2 | 9.1-9.2 |
| roko-neuro | 2 | 10.1-10.2 |
| roko-std | 2 | 2.3, 2.2* |
| roko-primitives | 1 | 8.3* |

\* Some batches touch multiple crates.

---

## Parallel Execution Groups

For maximum throughput with N agents, here are valid parallel assignments:

### 2 agents

| Agent A | Agent B |
|---------|---------|
| 1.1 → 1.2 → 2.1 → 2.2 → 2.3 → 7.1 → 7.4 → 7.5 → 8.1 → 8.2 | 3.1 → 3.5 → 5.1 → 5.2 → 5.3 → 6.1 → 6.2 → 6.3 → 6.4 |

### 4 agents

| Agent | Batches |
|-------|---------|
| Chain | 1.1 → 1.2 → 1.3, 1.4, 1.5 → 1.6, 2.1 → 2.2 → 2.3, 2.4, 2.5 |
| Safety+Archetype | 5.1 → 4.1 → 4.3, 5.2 → 5.3, 4.2 |
| Indicators+Learning | 3.1 → 3.2, 3.3, 3.4, 3.5 → 3.6, 7.1 → 7.2, 7.3, 7.4, 7.5 |
| Integration | 6.1 → 6.2, 6.3, 6.4, 4.4 → 8.1 → 8.2, 8.3 → 9.1 → 9.2, 10.1 → 10.2 |

---

## Quick Start: First 5 Batches

If you want to start immediately, these 5 batches have no dependencies and produce immediate value:

1. **1.1** (S) — `get_logs` on alloy client. Unblocks all chain operations.
2. **3.1** (S) — Add 6 classical indicators. Extends existing `ChainOracle`.
3. **5.1** (L) — Archetype registry. Infrastructure for agent specialization.
4. Then: **1.2** (L) — WebSocket subscription. Core chain connectivity.
5. Then: **2.1** (M) — VenueAdapter trait. Unlocks tool handlers, safety, learning.

After these 5, the dependency graph fans out and parallelism becomes available.
