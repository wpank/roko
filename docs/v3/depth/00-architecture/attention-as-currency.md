# Attention as Universal Cognitive Currency

> **v3 depth file** -- `/docs/v3/depth/00-architecture/attention-as-currency.md`
> Canonical source: v1 `docs/v1/00-architecture/25-attention-as-currency.md`
> Status: **Target-state concept** -- No demurrage, balance, or attention-currency code exists.
> This document describes a deferred research direction. Cascade routing and composer budgets
> are wired independently; the unified attention economy remains unimplemented.

---

## 1. The Problem: Three Disconnected Resource Systems

Roko manages loop-time cognitive resources through three independent mechanisms, plus a
separate memory-side ledger:

1. **CascadeRouter** (`roko-learn`): Selects among T0/T1/T2 inference tiers using LinUCB
   bandits. Each tier has a different cost. The router optimizes for cost-adjusted quality
   but has no awareness of global budget constraints.

2. **Composer budget** (`roko-compose`): The `Budget` struct caps context window size per
   composition. Each `compose()` call operates independently -- there is no cross-tick budget
   accounting.

3. **VCG Attention Auction** (specified in architecture docs): Designed to allocate "attention
   slots" among competing Signals, but never wired to the actual Router or Composer.

4. **Neuro demurrage** (see decay variants, tier matrix): The deferred design says durable
   Signals would carry `balance`, a holding-cost ledger that decays when memory sits idle and
   is reinforced by use, citation, retrieval, or surprise.

These mechanisms make locally rational decisions that are globally incoherent. An agent might
cascade to T2 for every tick while the budget is exhausted by low-priority context, and the
VCG auction sits disconnected from both.

> "Attention is at once our most powerful and most fragile mental tool."
> -- Herbert Simon, "Designing Organizations for an Information-Rich World" (1971)

Simon's foundational insight -- that a wealth of information creates a poverty of attention --
is the theoretical anchor for this design. In an information-rich agent system, the scarce
resource is not data but the cognitive capacity to process it. Attention must be explicitly
allocated, not implicitly assumed (Simon 1971, pp. 40-41).

---

## 2. The Attention Token Model

### 2.1 Core Abstraction

```rust
/// A single unit of cognitive attention. Dimensionless, fungible.
/// 1 attention token ~ cost of 1 T0 probe tick.
///
/// Exchange rates:
///   T0 probe   = 1 AT
///   T1 fast    = 200 AT
///   T2 full    = 3200 AT
///   Context KB = 10 AT per KB
///   Gate eval  = 50 AT per gate
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AttentionToken(f64);

impl AttentionToken {
    pub const ZERO: Self = Self(0.0);
    pub const T0_COST: Self = Self(1.0);
    pub const T1_COST: Self = Self(200.0);
    pub const T2_COST: Self = Self(3200.0);
    pub const CONTEXT_PER_KB: Self = Self(10.0);
    pub const GATE_COST: Self = Self(50.0);

    pub fn new(amount: f64) -> Self {
        Self(amount.max(0.0))
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    /// Spend tokens from a pool; returns None if insufficient.
    pub fn spend(pool: &mut Self, cost: Self) -> Option<Self> {
        if pool.0 >= cost.0 {
            pool.0 -= cost.0;
            Some(cost)
        } else {
            None
        }
    }
}
```

### 2.2 Budget Pools

Attention tokens are allocated in hierarchical pools that mirror Roko's three cognitive speeds:

```rust
/// Hierarchical attention budget for a single agent session.
pub struct AttentionBudget {
    /// Total session budget (replenished per Delta cycle).
    pub session_total: AttentionToken,
    /// Remaining session tokens.
    pub session_remaining: AttentionToken,

    /// Gamma-speed budget (per-tick cap).
    pub gamma_cap: AttentionToken,
    /// Theta-speed budget (per-reflection cap).
    pub theta_cap: AttentionToken,
    /// Delta-speed budget (per-consolidation cap).
    pub delta_cap: AttentionToken,

    /// Rollover fraction: what % of unspent Gamma tokens carry to next tick.
    pub rollover_fraction: f64,  // default 0.1 (10%)

    /// Emergency reserve: fraction of session budget held back for critical ops.
    pub emergency_reserve: f64,  // default 0.15 (15%)
}

impl Default for AttentionBudget {
    fn default() -> Self {
        Self {
            session_total: AttentionToken::new(100_000.0),
            session_remaining: AttentionToken::new(100_000.0),
            gamma_cap: AttentionToken::new(500.0),
            theta_cap: AttentionToken::new(5_000.0),
            delta_cap: AttentionToken::new(30_000.0),
            rollover_fraction: 0.1,
            emergency_reserve: 0.15,
        }
    }
}
```

### 2.3 The Memory Ledger

Attention tokens buy compute inside a tick. In the deferred companion design, demurrage
would tax the right to keep a durable Signal warm after the tick ends.

```
balance(t + dt) = balance(t) - r*dt - B*balance(t)*dt + reinforcement
```

The `reinforcement` term comes from reads, citations, successful gates, and surprise. In the
memory layer, `balance` is not interchangeable with the live attention pool: a session can be
well-budgeted and still be fed by a petrified memory base if stale Signals never pay a holding
cost.

| Ledger | Charged when | Governs | Example |
|---|---|---|---|
| Attention budget | During the loop | Router, Composer, Gate, Policy spend | A T2 inference burns session tokens |
| Demurrage balance | Between loops | Durable memory residency | A stale playbook slowly loses balance |

### 2.4 Token Flow Per Cognitive Loop Tick

```
+-------------------------------------------------+
|              Session Budget Pool                |
|            (100,000 AT default)                 |
+----------+--------------------------------------+
           | allocate per-speed caps
    +------+------+---------------+
    v             v               v
+--------+  +----------+  +-----------+
| Gamma  |  |  Theta   |  |   Delta   |
| 500 AT |  | 5000 AT  |  | 30000 AT  |
+---+----+  +----+-----+  +-----+-----+
    |            |              |
    v            v              v
 +-----------------------------------------+
 |       VCG Attention Auction             |
 |  Signals bid for attention slots        |
 |  Winners consume AT from speed pool     |
 +-----------------------------------------+
    |            |              |
    v            v              v
 +--------+ +--------+  +-----------+
 |Compose | |Cascade |  |   Gate    |
 |Context | | Route  |  |  Verify   |
 | 10/KB  | |T0-T2   |  |  50/gate  |
 +--------+ +--------+  +-----------+
```

---

## 3. The VCG Attention Auction

### 3.1 Mechanism Design

The VCG (Vickrey-Clarke-Groves) auction allocates attention slots to Signals competing for
inclusion in the cognitive loop. Each Signal "bids" based on its Score; the auction selects
winners and charges them the externality they impose on others -- ensuring truthful bidding
is the dominant strategy (Vickrey 1961, Clarke 1971, Groves 1973).

**Mapping to current implementation**: The E44 cross-cut functors already implement a VCG
auction for context bidding. The `AttentionBidder` variants in the runner (Neuro/Task/Research)
provide the bidder implementations. The full attention token economy described here would
extend that existing auction to govern all resource allocation, not just context slots.

```rust
/// An attention auction that allocates K slots among N competing Signals.
///
/// Properties (Duetting et al. 2024):
///   - DSIC: dominant-strategy incentive compatible
///   - Individually rational: no Signal is worse off for participating
///   - Allocatively efficient: maximizes total attention value
pub struct AttentionAuction {
    /// Number of attention slots available this tick.
    pub slots: usize,
    /// Minimum bid (Score.effective) to participate.
    pub reserve_price: f64,
    /// Maximum fraction of budget any single Signal can consume.
    pub max_bid_fraction: f64,  // default 0.3
}

/// A bid in the attention auction.
pub struct AttentionBid {
    /// The Signal competing for attention.
    pub signal_hash: ContentHash,
    /// Bid value derived from Score.effective() * urgency_multiplier.
    pub bid_value: f64,
    /// Estimated attention cost if this Signal wins (context size + processing).
    pub estimated_cost: AttentionToken,
    /// Priority class: Critical > High > Normal > Background.
    pub priority: AttentionPriority,
}

/// Auction result.
pub struct AuctionOutcome {
    /// Winning Signals in priority order.
    pub winners: Vec<AuctionWinner>,
    /// Total attention tokens committed.
    pub total_cost: AttentionToken,
    /// Signals that bid but lost.
    pub rejected: Vec<ContentHash>,
    /// Revenue (VCG payments) -- recycled into emergency reserve.
    pub vcg_revenue: AttentionToken,
}
```

### 3.2 VCG Payment Computation

For K homogeneous slots, the VCG payment for winner i is the (K+1)th highest bid -- the
first excluded bid. This ensures:

1. **DSIC**: Each Signal's optimal strategy is to bid its true Score.
2. **Allocative efficiency**: The winners are the K highest-value Signals.
3. **Individual rationality**: No winner pays more than their bid.

```rust
impl AttentionAuction {
    pub fn run(
        &self,
        bids: &mut [AttentionBid],
        budget: &mut AttentionToken,
    ) -> AuctionOutcome {
        bids.sort_by(|a, b| b.bid_value.partial_cmp(&a.bid_value).unwrap());

        let mut winners = Vec::with_capacity(self.slots);
        let mut total_cost = AttentionToken::ZERO;
        let mut rejected = Vec::new();

        for (i, bid) in bids.iter().enumerate() {
            if winners.len() >= self.slots
                || bid.bid_value < self.reserve_price
                || bid.estimated_cost.value() > budget.value() * self.max_bid_fraction
            {
                rejected.push(bid.signal_hash);
                continue;
            }

            // VCG payment: the (K+1)th highest bid, or reserve price
            let vcg_payment_value = bids
                .get(self.slots)
                .map(|b| b.bid_value)
                .unwrap_or(self.reserve_price);
            let vcg_payment = AttentionToken::new(
                vcg_payment_value * bid.estimated_cost.value()
                    / bid.bid_value.max(f64::EPSILON),
            );

            if AttentionToken::spend(budget, bid.estimated_cost).is_some() {
                winners.push(AuctionWinner {
                    signal_hash: bid.signal_hash,
                    slot: i,
                    bid_value: bid.bid_value,
                    vcg_payment,
                });
                total_cost = AttentionToken::new(
                    total_cost.value() + bid.estimated_cost.value(),
                );
            } else {
                rejected.push(bid.signal_hash);
            }
        }

        let vcg_revenue = AttentionToken::new(
            winners.iter().map(|w| w.vcg_payment.value()).sum(),
        );

        AuctionOutcome { winners, total_cost, rejected, vcg_revenue }
    }
}
```

### 3.3 Priority Classes

```rust
/// Priority classes modulate the auction reserve price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttentionPriority {
    Background = 0,  // Reserve price x 2.0
    Normal = 1,      // Reserve price x 1.0
    High = 2,        // Reserve price x 0.5
    Critical = 3,    // Reserve price x 0.1
}
```

---

## 4. Unifying CascadeRouter with Attention Tokens

### 4.1 The CascadeRouter as Attention Spender

The CascadeRouter currently selects T0/T1/T2 based on quality estimates from LinUCB bandits.
Under the attention economy, the router becomes a budget-aware spender: it must purchase
inference from the attention pool, and the pool constrains its choices.

```rust
/// Attention-aware cascade routing.
pub struct AttentionCascadeRouter {
    pub bandit: LinUCBRouter,
    pub tier_costs: [AttentionToken; 3],
    pub pressure_threshold: f64,  // default 0.3
    pub pressure_discount: f64,   // default 0.6
}

impl AttentionCascadeRouter {
    pub fn select_tier(
        &self,
        context: &RouterContext,
        budget: &AttentionBudget,
    ) -> Option<(InferenceTier, AttentionToken)> {
        let remaining_fraction = budget.session_remaining.value()
            / budget.session_total.value().max(f64::EPSILON);

        let quality_estimates = self.bandit.estimate_all(context);
        let adjusted: Vec<(InferenceTier, f64, AttentionToken)> = quality_estimates
            .iter()
            .enumerate()
            .map(|(i, &quality)| {
                let tier = InferenceTier::from_index(i);
                let cost = self.tier_costs[i];
                let adj_quality = if remaining_fraction < self.pressure_threshold {
                    let cost_penalty = cost.value() / self.tier_costs[2].value();
                    quality * (1.0 - cost_penalty * (1.0 - self.pressure_discount))
                } else {
                    quality
                };
                (tier, adj_quality, cost)
            })
            .collect();

        adjusted
            .into_iter()
            .filter(|(_, _, cost)| cost.value() <= budget.session_remaining.value())
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(tier, _, cost)| (tier, cost))
    }
}
```

### 4.2 Budget Pressure Curve

Budget pressure follows a sigmoid to avoid sharp transitions:

```
Pressure(r) = 1 / (1 + exp(10 * (r - threshold)))

where r = remaining_fraction, threshold = 0.3

Pressure = 0.0  when budget is plentiful (r >> 0.3)
Pressure = 0.5  when budget hits threshold (r = 0.3)
Pressure = 1.0  when budget is near-exhausted (r << 0.3)
```

---

## 5. Composer Budget Integration

### 5.1 Context Window as Attention Consumer

Under the attention economy, every KB of context assembled by the Composer costs attention
tokens. This creates natural pressure toward concise, relevant prompts.

```rust
pub struct AttentionComposerBudget {
    pub max_context_tokens: usize,
    pub cost_per_kb: AttentionToken,       // default: 10 AT/KB
    pub low_relevance_premium: f64,        // default: 2.0 (Score < 0.3 costs 2x AT)
    pub cache_discount: f64,               // default: 0.5 (cached context costs 0.5x)
}
```

### 5.2 Incentive Alignment

| Behavior | AT Cost | Incentive |
|---|---|---|
| Include high-relevance Signal (Score > 0.7) | 10 AT/KB | Neutral -- base rate |
| Include low-relevance Signal (Score < 0.3) | 20 AT/KB | Discouraged -- premium |
| Reuse cached context from previous tick | 5 AT/KB | Encouraged -- discount |
| Include AntiKnowledge (known-false) | 0 AT | Free -- always include |
| Exceed 80% of context window | 1.5x multiplier | Discouraged -- marginal cost increases |

---

## 6. Cross-Speed Token Economics

### 6.1 The Delta Dividend

During Delta (consolidation) cycles, the system performs knowledge compression and strategy
distillation. Effective consolidation reduces future attention costs by producing higher-quality
heuristics and more relevant knowledge.

```rust
pub struct DeltaDividend {
    pub compression_ratio: f64,
    pub promotions: usize,
    pub anti_knowledge_created: usize,
    pub projected_savings_per_tick: AttentionToken,
}
```

### 6.2 Theta Budget Arbitrage

```
theta_arbitrage = (gamma_AT_saved_by_replan) / (theta_AT_spent)

If theta_arbitrage > 1.0: Theta reflection is net-positive. Increase frequency.
If theta_arbitrage < 0.5: Theta reflection is wasteful. Decrease frequency.
```

### 6.3 Delta Dividend vs Demurrage

Delta consolidation and demurrage act on different sides of the ledger. The Delta dividend
reduces future loop spend by compressing and distilling knowledge; demurrage reduces the
amount of stale durable memory that can claim future attention at all. A cheaper Router does
not fix a bloated memory base, and more aggressive Composer budgeting does not prevent old
Signals from ossifying.

---

## 7. Daimon Modulation of Attention Allocation

The Daimon (affect subsystem) modulates attention allocation based on the agent's affective
state, implementing Kahneman's (1973) resource theory of attention:

```rust
/// Affect-driven attention modulation.
///
/// PAD dimensions (Pleasure-Arousal-Dominance) modulate attention:
///   - High Arousal -> broader attention (lower reserve price, more slots)
///   - High Dominance -> more aggressive tier selection (prefer T2)
///   - Low Pleasure -> risk-averse spending (increase emergency reserve)
pub struct DaimonAttentionModulator {
    pub arousal_slot_bonus: f64,      // default: 0.3
    pub dominance_tier_shift: f64,    // default: 0.2
    pub displeasure_reserve: f64,     // default: 0.1
}
```

---

## 8. Integration with Existing Wiring

### 8.1 Into the Universal Cognitive Loop

| Loop Step | Attention Integration |
|---|---|
| 1. PERCEIVE | Query returns candidates -- they become auction bidders |
| 2. EVALUATE | Score.effective() -> bid value |
| 3. ATTEND | VCG Auction selects winners, charges AT |
| 4. INTEGRATE | Composer draws AT per KB assembled |
| 5. ACT | CascadeRouter draws AT per tier selected |
| 6. VERIFY | Gate draws AT per gate evaluation |
| 7. PERSIST | No AT cost in-loop; durable memory taxed separately |
| 8. ADAPT | Policy observes AT expenditure, adjusts future budgets |
| 9. META-COGNIZE | Daimon modulates next tick's AT allocation |

### 8.2 Current Partial Wiring

| Component | Attention-Adjacent Wiring | Gap |
|---|---|---|
| `AttentionBidder` variants | Neuro/Task/Research bidders exist in runner | Not unified with AT currency |
| CascadeRouter | Budget-aware routing is live | No AT denomination |
| Inference gateway (E26) | Cost accounting, backpressure | USD-denominated, not AT |
| Composer budget | Token-count limits exist | No cross-tick accounting |
| VCG in E44 functors | Conflict VCG is wired | Context-only, not full economy |

---

## 9. Observability

```rust
/// Per-tick attention telemetry.
#[derive(Serialize, Deserialize)]
pub struct AttentionTelemetry {
    pub tick_id: u64,
    pub speed: CognitiveSpeed,
    pub budget_before: f64,
    pub budget_after: f64,
    pub auction_bids: usize,
    pub auction_winners: usize,
    pub vcg_revenue: f64,
    pub cascade_tier: InferenceTier,
    pub cascade_cost: f64,
    pub composer_context_kb: f64,
    pub composer_cost: f64,
    pub gate_cost: f64,
    pub total_spent: f64,
    pub pressure: f64,
    pub pad_modulation: [f64; 3],
    pub demurrage_balance_total: f64,
    pub demurrage_paid_total: f64,
    pub reinforcement_events: usize,
    pub thaw_events: usize,
}
```

---

## 10. Theoretical Foundations

### 10.1 Simon's Attention Scarcity (1971)

Herbert Simon's "Designing Organizations for an Information-Rich World" (1971) established
that "a wealth of information creates a poverty of attention and a need to allocate that
attention efficiently among the overabundance of information sources that might consume it."
This is the foundational insight: in an information-rich agent system, the scarce resource is
not data but the cognitive capacity to process it.

Simon's framing maps directly to Roko's architecture: the Substrate is information-rich (all
persisted Signals), but the cognitive loop can only process a bounded number per tick. The
attention token is Simon's allocation mechanism made computable.

### 10.2 VCG Auction Theory

The Vickrey-Clarke-Groves mechanism (Vickrey 1961, Clarke 1971, Groves 1973) is the unique
mechanism that is simultaneously DSIC, allocatively efficient, and individually rational.
Duetting et al. (2024) extended VCG to multi-agent LLM settings, proving DSIC even when
agents are LLMs with strategic capabilities. arXiv:2504.14824 (2025) further showed
dual-currency VCG with MFMARL reduces collusion risk in distributed settings.

### 10.3 Resource Theory of Attention (Kahneman 1973)

| Kahneman Insight | Roko Mapping |
|---|---|
| Attention pool replenishes slowly | Session budget replenished per Delta cycle |
| Arousal increases pool size | Daimon arousal -> more auction slots |
| Difficult tasks require more attention | T2 costs 16x T1 |
| Automaticity reduces attention cost | T0 probes cost 1 AT |

### 10.4 FrugalGPT Connection (Chen et al. 2023)

Roko's `AttentionCascadeRouter` extends FrugalGPT with three innovations:
1. **Budget pressure** -- routes based on quality x budget state
2. **Affect modulation** -- Daimon shifts routing under stress
3. **VCG pricing** -- mechanism design ensures incentive compatibility

---

## 11. Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_vcg_payment_truthful` | VCG payments make truthful bidding dominant | Unit |
| `test_vcg_individual_rationality` | No winner pays more than their bid | Unit |
| `test_budget_exhaustion_graceful` | When AT = 0, tick completes with T0 only | Integration |
| `test_pressure_shifts_to_cheap_tier` | Below threshold, router prefers T0/T1 | Unit |
| `test_rollover_fraction` | Unspent Gamma AT partially carries to next tick | Unit |
| `test_emergency_reserve_locked` | Emergency reserve not spent by normal operations | Unit |
| `test_delta_dividend_reduces_future_cost` | After consolidation, average tick AT decreases | Integration |
| `test_daimon_high_arousal_more_slots` | High arousal increases auction slots | Unit |
| `test_displeasure_increases_reserve` | Negative pleasure increases emergency reserve | Unit |
| `test_low_relevance_premium_applied` | Score < 0.3 Signals cost 2x AT in composer | Unit |
| `test_cache_discount_applied` | Cached context costs 0.5x AT | Unit |
| `test_telemetry_logged_per_tick` | AttentionTelemetry written every tick | Integration |
| `test_demurrage_reduces_idle_balance` | Idle durable Signals lose balance between loops | Unit |
| `test_reinforcement_refunds_balance` | Citation, retrieval, or surprise increases balance | Unit |

---

## Cross-References

- [Cognitive Energy Model](./cognitive-energy-model.md) -- energy pools that replenish AT budgets
- [Cross-Section Integration Map](./cross-section-integration-map.md) -- subsystem wiring
- v3 Section 0 -- Architecture, Scorer/Gate/Router/Composer/Policy
- v3 Section 5 -- Learning, CascadeRouter and bandit optimization
