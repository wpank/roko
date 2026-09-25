# Ebbinghaus Decay with Tier Shaping

> **v3 depth -- 09-memory** | Source: v1/06-neuro/07. FULL decay formulas,
> worked examples.

Neuro keeps knowledge fresh through demurrage: every durable entry carries a
balance, earns its keep through use, and cools when it stops being retrieved,
cited, or reinforced. Ebbinghaus still matters, but as a rate-shaping
component rather than the whole story.

---

## The Forgetting Curve Formula

### Base decay equation

The retention model combines Ebbinghaus time-decay with economic demurrage:

```
balance(t + dt) = balance(t) - demurrage_tax(dt) + reinforcement(kind, novelty)
freshness(t) = balance(t) * ebbinghaus_weight(age, type_half_life, tier_multiplier)
```

Where:
- `balance` is the durable-memory freshness reserve
- `demurrage_tax` is the holding cost paid per unit time
- `reinforcement` comes from retrieval, citation, gate survival, surprise,
  or agent quoting
- `ebbinghaus_weight` is the rate-shaping curve

### Ebbinghaus weight formula

```
ebbinghaus_weight(age_hours, half_life_hours) = exp(-age_hours * ln(2) / half_life_hours)
```

Equivalently:

```
ebbinghaus_weight = 2^(-age_hours / half_life_hours)
```

At `age = half_life`, weight = 0.5 exactly.

### Effective half-life composition

```
effective_half_life = tier_multiplier * type_base_half_life
```

**Type base half-lives:**

| Type | Base Half-Life |
|------|---------------|
| Warning | 1 hour |
| StrategyFragment | 14 days |
| Insight | 30 days |
| CausalLink | 60 days |
| Heuristic | 90 days |
| AntiKnowledge | 30 days (floor 0.3) |

**Tier multipliers:**

| Tier | Multiplier |
|------|-----------|
| Transient | 0.1x |
| Working | 0.5x |
| Consolidated | 1.0x |
| Persistent | 5.0x |

---

## Implementation

```rust
impl KnowledgeEntry {
    pub fn freshness(&self, now: DateTime<Utc>) -> f64 {
        let age_hours = now.signed_duration_since(self.created_at)
            .num_seconds() as f64 / 3600.0;
        if age_hours <= 0.0 { return self.balance; }
        let half_life_hours = self.effective_half_life_days() * 24.0;
        let ebbinghaus = if half_life_hours > 0.0 {
            (-(age_hours * 2.0_f64.ln()) / half_life_hours).exp()
        } else {
            0.0
        };
        self.balance * ebbinghaus
    }

    pub fn effective_half_life_days(&self) -> f64 {
        let base = if self.half_life_days.is_finite() && self.half_life_days > 0.0 {
            self.half_life_days
        } else {
            self.kind.default_half_life_days()
        };
        base * self.tier.multiplier() as f64
    }
}
```

---

## Worked Examples

### Example 1: A Working Insight (effective half-life = 15 days = 360 hours)

At balance = 1.0, no reinforcement:

| Age | Ebbinghaus weight | Freshness | Derivation |
|-----|------------------|-----------|-----------|
| 0 days | 1.000 | 1.000 | Fresh at ingest |
| 7 days | 0.722 | 0.722 | exp(-168 * ln(2) / 360) = exp(-0.3252) |
| 15 days | 0.500 | 0.500 | Exactly one half-life |
| 30 days | 0.250 | 0.250 | Two half-lives |
| 45 days | 0.125 | 0.125 | Three half-lives -- near floor |
| 65 days | 0.051 | 0.051 | Approaching GC threshold (0.05) |

### Example 2: A Persistent Fact (effective half-life = 1,825 days)

| Age | Ebbinghaus weight | Interpretation |
|-----|------------------|---------------|
| 30 days | 0.989 | Nearly unchanged after a month |
| 365 days | 0.863 | Still strong after a year |
| 1825 days | 0.500 | One half-life after 5 years |

### Example 3: A Transient Warning (effective half-life = 6 min = 0.1 hours)

| Age | Ebbinghaus weight | Interpretation |
|-----|------------------|---------------|
| 1 min | 0.891 | Already decaying |
| 6 min | 0.500 | One half-life |
| 20 min | 0.100 | Nearly gone |
| 1 hour | 0.001 | Effectively zero |

### Example 4: Working Insight with reinforcement

| Age | Event | Balance | Ebbinghaus | Freshness |
|-----|-------|---------|-----------|-----------|
| 0 days | Ingest | 1.000 | 1.000 | 1.000 |
| 7 days | Retrieved (+0.05) | 0.965 | 0.722 | 0.697 |
| 14 days | Gate pass (+0.15) | 1.080 | 0.522 | 0.564 |
| 21 days | Cited (+0.10) | 1.110 | 0.377 | 0.418 |
| 30 days | No event | 1.020 | 0.250 | 0.255 |

Reinforcement keeps the entry warmer than pure decay would allow.

---

## Demurrage Model

### Balance decay

```rust
pub const DEMURRAGE_RATE_PER_HOUR: f64 = 0.005;
pub const BALANCE_GC_FLOOR: f64 = 0.05;

impl KnowledgeEntry {
    pub fn apply_demurrage(&mut self, elapsed_hours: f64) {
        if elapsed_hours <= 0.0 { return; }
        let deduction = DEMURRAGE_RATE_PER_HOUR * elapsed_hours;
        self.balance = (self.balance - deduction).max(0.0);
    }
}
```

At 0.005/hour, balance 1.0 with no reinforcement reaches GC floor (0.05)
after ~190 hours (~8 days).

### Reinforcement signals

Five balance-earning events:

```rust
pub enum ReinforcementSignal {
    Retrieved,     // 0.05 base bump
    Cited,         // 0.10 base bump
    Gated,         // 0.15 base bump
    Surprised,     // 0.08 base bump
    AgentQuoted,   // 0.12 base bump
}
```

Each bump is novelty-weighted: `bump = base_value * (1.0 + novelty)` where
`novelty = 1.0 - max_hdc_similarity` against top-K neighbors. Common entries
get small bumps; rare-but-useful entries get larger bumps. Balance capped
at 5.0.

### Spacing effect

Reinforcement across distinct episodes matters more than repeated retrieval
within a single task. This preserves the Ebbinghaus spacing effect: balance
rises when a rule survives across time, context, and gate outcomes.

---

## Garbage Collection Schedule

GC timeline by Type x Tier (approximate time to reach 0.05 threshold at
4.3 half-lives):

| Type \ Tier | Transient | Working | Consolidated | Persistent |
|---|---|---|---|---|
| **Warning** (1h) | 26 min | 2.2 hours | 4.3 hours | 21.5 hours |
| **StrategyFragment** (14d) | 6 days | 30 days | 60 days | 301 days |
| **Insight** (30d) | 13 days | 65 days | 129 days | 646 days |
| **CausalLink** (60d) | 26 days | 129 days | 258 days | 1,293 days |
| **Heuristic** (90d) | 39 days | 194 days | 387 days | 1,939 days |

AntiKnowledge is protected by its 0.3 confidence floor and is never GC'd.

---

## Cold-Tier Freeze/Thaw

Entries whose balance falls below `BALANCE_GC_FLOOR` (0.05) are frozen
rather than deleted:

```rust
pub const THAW_STARTER_BALANCE: f64 = 0.3;
```

- **Freeze**: Entry keeps content address and lineage; body moves off hot
  path. `frozen = true`, `frozen_at = now`.
- **Thaw**: Restores starter balance (0.3) so the entry can compete again.
  If it keeps failing after thaw, it cools back down.
- **Seven-day depleted-balance freezing**: Entries at zero balance for 7 days
  are automatically frozen. The `balance_depleted_at` timestamp tracks when
  depletion began.

---

## Demurrage vs. Neural Network Embedding Drift

Neuro's retention policy is explicit and auditable. Balance, tier, and age
explain why an entry is warm, cold, or thawed. That is better than opaque
embedding drift, where similarity changes because the model state changed
under the hood. Given an entry's type, tier, balance, and reinforcement
history, Neuro can explain why that knowledge was available at the time of
a decision.

---

## Academic Foundations

- Ebbinghaus, H. (1885). *Uber das Gedachtnis.* Leipzig.
- Murre, J. M. J. & Dros, J. (2015). "Replication and Analysis of
  Ebbinghaus' Forgetting Curve." *PLOS ONE*, 10(7).
- Pimsleur, P. (1967). "A Memory Schedule." *Modern Language Journal*, 51(2).
- McClelland, J. L. et al. (1995). "Complementary learning systems."
  *Psychological Review*, 102(3).

---

## Cross-References

- `type-half-lives.md` -- base half-life rationale per type
- `four-validation-tiers.md` -- tier multiplier system
- `antiknowledge-challenge.md` -- AntiKnowledge floor enforcement
