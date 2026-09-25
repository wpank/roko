# Type Half-Lives: Rationale and Design

> **v3 depth -- 09-memory** | Source: v1/06-neuro/03

Each knowledge type has a base half-life calibrated to the expected staleness
rate of that category of knowledge, drawn from memory research and practical
agent operation.

---

## The Ebbinghaus Decay Model

### Historical foundation

Hermann Ebbinghaus (1885) conducted the first systematic experiments on
memory retention. Using nonsense syllables to control for prior knowledge, he
demonstrated that memory follows an approximately exponential decay curve:

```
retention(t) = e^(-t / S)
```

where `t` is time since encoding and `S` is a stability parameter that
increases with each successful retrieval. Key findings:

1. **Most forgetting happens early** -- retention drops steeply in the first
   hours and days
2. **Spacing effects strengthen memory** -- items retrieved at increasing
   intervals are retained longer
3. **Meaningfulness matters** -- meaningful material decays slower than
   arbitrary material

### Application to agent knowledge

Neuro applies the Ebbinghaus model:

```
weight(entry) = 2^(-age_days / half_life_days)
```

At time `t = half_life_days`, the weight equals 0.5 (half the original
strength). The `half_life_days` is determined by two factors:

- **Type base half-life**: How quickly this *kind* of knowledge becomes stale
- **Tier multiplier**: How validated this *specific* entry is

The product `effective_half_life = tier_multiplier * type_base_half_life`
gives the actual decay rate.

### Why exponential, not linear

Linear decay (weight decreases by a fixed amount per day) would mean that old
knowledge and new knowledge decay at the same absolute rate. Exponential
decay ensures that:
- **Fresh knowledge decays fastest** -- a 1-day-old entry with a 7-day
  half-life has already lost 10% of its weight
- **Old knowledge decays slowly** -- an entry that has survived for 3
  half-lives has only 12.5% weight remaining
- **The GC threshold creates a natural cutoff** -- entries below 0.05
  confidence are removed, which happens after approximately 4.3 half-lives

---

## Base Half-Lives by Type

### Warning: 1 hour

**Rationale.** Danger signals must be current. A warning about a specific
vulnerability, a dangerous API behavior, or a market condition is only
valuable if it reflects the present state. Warnings about software
vulnerabilities may be patched within hours. Warnings about network
conditions change with every block.

**Effective half-lives by tier:**

| Tier | Multiplier | Effective Half-Life |
|------|-----------|-------------------|
| Transient | 0.1x | 6 minutes |
| Working | 0.5x | 30 minutes |
| Consolidated | 1.0x | 1 hour |
| Persistent | 5.0x | 5 hours |

**Design note.** Even Persistent warnings have a relatively short effective
half-life (5 hours). No warning should persist for long without
reconfirmation. If a warning is still valid, it will be reconfirmed by
ongoing experience.

### StrategyFragment: 14 days

**Rationale.** Strategies are context-dependent. A StrategyFragment encodes a
multi-step procedure tailored to current conditions: current tool versions,
API behaviors, market microstructure, team conventions. These conditions
change frequently.

**Effective half-lives by tier:**

| Tier | Multiplier | Effective Half-Life |
|------|-----------|-------------------|
| Transient | 0.1x | 1.4 days |
| Working | 0.5x | 7 days |
| Consolidated | 1.0x | 14 days |
| Persistent | 5.0x | 70 days |

**Design note.** Transient StrategyFragments have only 1.4 days of effective
half-life. An unvalidated strategy essentially evaporates within 6 days.

### Insight: 30 days

**Rationale.** Observations need regular revalidation. An Insight records a
pattern the agent has observed ("Rust's borrow checker errors often mean you
need Arc here"). Patterns can shift as codebases evolve.

```rust
pub const INSIGHT_HALF_LIFE_DAYS: f64 = 30.0;
```

**Design note.** 30 days is a balance point. Shorter (7--14 days) would cause
too much churn. Longer (60--90 days) would allow stale observations to
persist.

### CausalLink: 60 days

**Rationale.** Causal relationships need periodic confirmation but are more
durable than simple observations. A CausalLink captures a structural
relationship that tends to hold across a wider range of conditions.

### Heuristic: 90 days

**Rationale.** Rules of thumb are the most durable category of practical
knowledge. A Heuristic represents a pattern abstracted from multiple
observations and validated across contexts.

```rust
pub const HEURISTIC_HALF_LIFE_DAYS: f64 = 90.0;
```

### AntiKnowledge: 30 days (confidence floor 0.3)

**Rationale.** Known unknowns are always valuable. AntiKnowledge entries
decay at the standard 30-day rate, but their confidence has a **floor of
0.3** -- it can never drop below this. They are exempt from garbage
collection.

**The confidence floor of 0.3:** This value was chosen because it is:
- High enough to ensure the entry remains retrievable (above the retrieval
  noise floor of 0.1--0.2)
- Low enough that newer positive evidence can outweigh it in retrieval
  ranking
- The same as the initial confidence assigned to Dream-generated hypotheses

**On-chain demurrage.** When AntiKnowledge entries are published to the chain,
they use 0.5x the standard demurrage rate. Standard entries decay at 1% per
year; AntiKnowledge decays at 0.5% per year.

---

## Half-Life Ordering and Design Logic

```
Warning (1h) < StrategyFragment (14d) < Insight (30d) < CausalLink (60d) < Heuristic (90d) < AntiKnowledge (floor)
```

This ordering follows **abstraction durability**: the more abstract and
general a piece of knowledge is, the longer it persists. Concrete,
context-dependent knowledge (Warnings, StrategyFragments) decays fastest.
Abstract, validated patterns (Heuristics) persist longest. Knowledge about
what is wrong (AntiKnowledge) never fully decays.

---

## Default Half-Life in Code

```rust
// From crates/roko-neuro/src/lib.rs
pub const INSIGHT_HALF_LIFE_DAYS: f64 = 30.0;
pub const HEURISTIC_HALF_LIFE_DAYS: f64 = 90.0;
pub const WARNING_HALF_LIFE_DAYS: f64 = 1.0 / 24.0;  // 1 hour
pub const CAUSAL_LINK_HALF_LIFE_DAYS: f64 = 60.0;
pub const STRATEGY_FRAGMENT_HALF_LIFE_DAYS: f64 = 14.0;

impl KnowledgeKind {
    pub const fn default_half_life_days(self) -> f64 {
        match self {
            Self::Insight => INSIGHT_HALF_LIFE_DAYS,
            Self::Heuristic => HEURISTIC_HALF_LIFE_DAYS,
            Self::Warning => WARNING_HALF_LIFE_DAYS,
            Self::CausalLink => CAUSAL_LINK_HALF_LIFE_DAYS,
            Self::StrategyFragment => STRATEGY_FRAGMENT_HALF_LIFE_DAYS,
            Self::AntiKnowledge => 30.0,
        }
    }
}
```

---

## Academic Foundations

- Ebbinghaus, H. (1885). *Uber das Gedachtnis.* Leipzig: Duncker & Humblot.
- Murre, J. M. J. & Dros, J. (2015). "Replication and Analysis of
  Ebbinghaus' Forgetting Curve." *PLOS ONE*, 10(7), e0120644.
- Wixted, J. T. & Ebbesen, E. B. (1991). "On the Form of Forgetting."
  *Psychological Science*, 2(6), 409--415.
- McClelland, J. L. et al. (1995). "Complementary learning systems."
  *Psychological Review*, 102(3).

---

## Cross-References

- `six-knowledge-types.md` -- the six types and their definitions
- `four-validation-tiers.md` -- the tier multiplier system
- `ebbinghaus-decay-with-tier.md` -- full decay formula with worked examples
- `antiknowledge-challenge.md` -- AntiKnowledge's special decay behavior
