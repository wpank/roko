# Knowledge Demurrage

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/knowledge-demurrage.md`
> Canonical source: v1 `docs/v1/17-lifecycle/11-knowledge-demurrage.md`
> Status: **Current** (balance-based demurrage, tier progression, and cold
> storage archival are live in `roko-neuro`)

---

## 1. The Freigeld Principle

Knowledge demurrage applies Silvio Gesell's Freigeld principle (Gesell 1916)
to both knowledge and tokens. Just as Ebbinghaus decay reduces confidence on
unused Signals (see `ebbinghaus-for-knowledge.md`), periodic demurrage cycles
actively reduce confidence on Signals that have not been re-validated. Both
mechanisms incentivize circulation over hoarding -- use knowledge or lose it,
use tokens or lose them.

Two levels of demurrage:

1. **Knowledge-level demurrage**: Periodic confidence reduction on Signals
   not re-validated.
2. **Token-level demurrage**: 1% annual demurrage on held KORAI tokens
   (Korai chain, mainnet only).

---

## 2. Knowledge-Level Demurrage

### Configuration

```rust
pub struct DemurrageConfig {
    /// Cognitive loop iterations between validation checks.
    /// Default: 250 iterations (~2.9 hours at 1 iteration/40s)
    pub validation_interval: u64,

    /// Confidence loss per missed validation interval.
    /// Default: 0.03 (3% per interval)
    pub decay_per_interval: f64,

    /// Minimum confidence before automatic archiving.
    /// Default: 0.1
    pub archive_threshold: f64,

    /// Domain-specific decay multipliers.
    pub domain_multipliers: HashMap<String, f64>,
}
```

```toml
[neuro.demurrage]
validation_interval = 250
decay_per_interval = 0.03
archive_threshold = 0.1

[neuro.demurrage.domains]
gas_patterns = 2.0       # 6% per interval
price_direction = 1.5    # 4.5% per interval
volatility_regime = 1.0  # 3% per interval
yield_trends = 0.8       # 2.4% per interval
protocol_behavior = 0.5  # 1.5% per interval
```

### The Demurrage Cycle

Every `validation_interval` iterations, the knowledge curator runs a
demurrage cycle:

1. For each active Signal, compute iterations since last validation.
2. Determine the number of missed validation intervals.
3. Apply domain-specific decay: `total_decay = decay_per_interval *
   domain_multiplier * missed_intervals`.
4. Reduce confidence: `new_confidence = (confidence - total_decay).max(0.0)`.
5. If confidence drops below `archive_threshold`, move to Archived status.

Already-archived Signals are not further decayed.

---

## 3. Five Beneficial Dynamics

Knowledge demurrage produces five dynamics that were originally attributed to
mortality pressure in the legacy system but are actually produced by
knowledge-level decay:

1. **A lean, current knowledge store.** Stale Signals fade, keeping active
   context relevant. Decisions are not polluted by outdated patterns that
   confidently encode expired conditions.

2. **Natural knowledge turnover.** Old Signals make room for new ones without
   explicit deletion. The agent does not need to decide what to forget.

3. **Incentive to explore.** Only fresh evidence maintains confidence. An
   agent that retreats to passive monitoring pays a knowledge tax that grows
   with every validation interval. Exploration is not optional -- it is the
   cost of maintaining knowledge.

4. **Forced knowledge circulation.** Signals approaching the archive
   threshold are prime candidates for mesh sharing. The agent's incentive is
   to share marginal knowledge before it depreciates entirely -- better to
   contribute to the group than let it evaporate. This implements Gesell's
   Freigeld principle for information.

5. **Domain-appropriate decay.** Gas price patterns decay in hours. Protocol
   behavior knowledge decays over months. The system tracks per-domain
   freshness, following Arbesman's domain-specific half-life research (2012).

---

## 4. Token-Level Demurrage (KORAI)

For chain-domain agents on the Korai chain, the KORAI token (mainnet) has a
1% annual demurrage rate. DAEJI (testnet) has no demurrage.

### Mechanism

```
balance_effective(t) = balance_raw * (1 - 0.01)^(years_since_last_update)
```

| Time held | Effective balance (from 1000 KORAI) |
|-----------|-------------------------------------|
| 0 days | 1000.00 |
| 30 days | 999.17 |
| 90 days | 997.52 |
| 1 year | 990.00 |
| 5 years | 950.99 |
| 10 years | 904.38 |
| 50 years | 605.03 |

### Why Demurrage

Gesell (1916) argued that money should mirror the decay of physical goods -- a
bushel of wheat rots, a machine rusts, but gold endures forever. Demurrage
equalizes by making money also "rot" slightly over time.

Applied to KORAI:
- **Incentivizes circulation**: Agents and operators are motivated to use
  KORAI rather than hoard it.
- **Mirrors knowledge decay**: Just as Signals lose confidence without
  reinforcement, tokens lose value without use.
- **Prevents wealth concentration**: Long-term holders face gradual dilution.
- **Funds ecosystem**: Demurrage proceeds fund infrastructure and subsidies.

### Why 1% For Now

KORAI's 1% rate is deliberately conservative -- below the 4-7% range
suggested by historical data (Chiemgauer at 6%, Worgl stamp scrip at 12%).
Rationale:
1. Higher rates discourage early adopters.
2. 1% is easy to reason about and communicate.
3. On-chain governance can increase the rate as the ecosystem matures.
4. Signal-level Ebbinghaus decay already provides strong circulation
   incentives; token demurrage is secondary pressure.

### Calibration Framework

```rust
pub struct DemurrageCalibration {
    pub target_velocity_multiplier: f64, // Default: 3.0
    pub hoarding_sensitivity: f64,       // Default: 0.4
    pub minimum_effective_rate: f64,     // Default: 0.02
    pub maximum_practical_rate: f64,     // Default: 0.15
    pub grace_period_days: u32,          // Default: 90
    pub floor_fraction: f64,             // Default: 0.01
}

// Recommended rate: ln(target_velocity) * hoarding_sensitivity
// For defaults: ln(3.0) * 0.4 = 0.044 = 4.4%/yr
```

Historical calibration data: Worgl stamp scrip (1932-33, 12%, ~14x velocity),
Chiemgauer (2003-present, 6%, 3-5x velocity), WIR Bank (1934-present, 0%
since 1952, 2-3x velocity).

---

## 5. Parallel Structure

The two demurrage systems are intentionally parallel:

| Property | Knowledge Demurrage | Token Demurrage |
|----------|-------------------|-----------------|
| Rate | 3% per validation interval | 1% per year |
| Counterforce | Retrieval + validation | Usage (staking, trading) |
| Threshold | Archive at confidence < 0.1 | No minimum (approaches 0) |
| Domain sensitivity | Yes (per-domain multipliers) | No (uniform rate) |
| Reversibility | Yes (re-validate to restore) | No (permanent) |

The design principle: **inactive resources decay, active resources persist.**

---

## 6. Philosophical Grounding

- **Gesell's Freigeld** (1916): Money that depreciates at a fixed rate forces
  circulation. Applied to knowledge: force Signals into circulation before
  they depreciate.

- **Ostrom's Commons Governance** (1990): Shared resources can be sustainably
  managed with appropriate institutional rules. Knowledge demurrage is an
  institutional rule for the knowledge commons.

- **Richards & Frankland's Forgetting as Optimization** (2017): The brain
  forgets to generalize -- removing specific details to extract patterns.
  Knowledge demurrage implements the same principle.

- **Nietzsche's Active Forgetting** (1874, 1887): The capacity to forget is
  essential for health and action. "It is impossible to live at all without
  forgetting." Knowledge demurrage is active forgetting, implemented
  computationally.

---

## 7. Telemetry Events

| Event | Payload | Trigger |
|-------|---------|---------|
| `neuro.demurrage_applied` | Entries processed/archived, total confidence lost | Each cycle |
| `neuro.signal_archived` | Signal hash, final confidence, domain, age | Individual archive |
| `neuro.knowledge_erosion` | Active/archived counts, average confidence | Significant shift |

---

## 8. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `DemurrageConfig` | `crates/roko-agent/src/lifecycle.rs` | Demurrage configuration |
| Demurrage cycle | `crates/roko-neuro/src/` | Apply demurrage to all entries |
| Cold archival | `crates/roko-serve/src/` | Age-based archival policy |
| Chain demurrage | `crates/roko-chain/src/` | KORAI balance computation |

---

## Cross-References

- [ebbinghaus-for-knowledge.md](ebbinghaus-for-knowledge.md) -- Ebbinghaus decay mechanics
- [funding-and-budgets.md](funding-and-budgets.md) -- KORAI token economics
- [academic-foundations.md](academic-foundations.md) -- Gesell, Ostrom, Richards citations
