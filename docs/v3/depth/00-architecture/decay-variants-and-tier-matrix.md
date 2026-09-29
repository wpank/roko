# Decay Variants and Knowledge Tier Matrix

> **v3 depth file** -- `/docs/v3/depth/00-architecture/decay-variants-and-tier-matrix.md`
> Canonical sources: v1 `docs/v1/00-architecture/04-decay-variants.md` and
> `docs/v1/00-architecture/18-decay-tier-matrix.md`
> Implementation: `crates/roko-core/src/decay.rs`, `crates/roko-neuro/`
> Status: **Shipping** -- the `Decay` enum (`None`, `HalfLife`, `Ttl`, `Ebbinghaus`) and
> `KnowledgeTier` enum (`Transient`, `Working`, `Consolidated`, `Persistent`) are implemented.
> Demurrage economics (balance/reinforcement model) is a target-state extension.

---

## 1. Decay: How Signals Fade

Every Signal decays. Pheromones fade over hours; episodes become less relevant over weeks;
playbook rules age out of playbooks. The `Decay` type unifies all of these: it is a function
that takes an age (in milliseconds) and returns a weight multiplier in [0, 1].

The weight formula that drives Store queries combines Score and Decay:

```
weight(t) = score.effective() x decay.apply(age_ms)
```

A Signal with high score but aggressive decay will eventually fall below the weight threshold
and be excluded from queries or pruned from the Store. This is how the system implements
forgetting -- not by deleting information, but by letting its weight decay below the
threshold of relevance.

---

## 2. The Decay Enum

From `crates/roko-core/src/decay.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Decay {
    /// No decay -- weight is permanent.
    None,

    /// Exponential half-life: weight = 0.5^(age / half_life_ms).
    HalfLife { half_life_ms: u64 },

    /// Hard cutoff: full weight until ttl_ms, then zero.
    Ttl { ttl_ms: u64 },

    /// Ebbinghaus forgetting curve: weight = exp(-age / (strength * scale_ms)).
    Ebbinghaus { strength: f32, scale_ms: u64 },
}
```

---

## 3. Decay Variant Formulas

### 3.1 None -- Permanent Weight

```
weight(age) = 1.0   for all age
```

Use for identity, schema, fixed policy records. Never expires unless explicitly replaced.

### 3.2 HalfLife -- Exponential Decay

```
weight(age) = 0.5^(age_ms / half_life_ms)
```

The classic exponential decay. Weight halves at each half-life interval. The function is
smooth, continuous, and approaches zero asymptotically but never reaches it.

**Properties**:
- `weight(0) = 1.0`
- `weight(half_life) = 0.5`
- `weight(2 * half_life) = 0.25`
- `weight(n * half_life) = 0.5^n`

**Shipped constants** (matching agent pheromone half-lives):

| Constant | Half-life | Use |
|---|---|---|
| `Decay::THREAT` | 2 hours (7,200,000 ms) | Threat pheromone |
| `Decay::OPPORTUNITY` | 4 hours (14,400,000 ms) | Opportunity pheromone |
| `Decay::WISDOM` | 24 hours (86,400,000 ms) | Wisdom pheromone |
| `Decay::GATE_VERDICT` | 24 hours (86,400,000 ms) | Verdict signals |

**Edge case**: `half_life_ms = 0` returns 0.0 immediately.

### 3.3 Ttl -- Step Function

```
weight(age) = 1.0   if age < ttl_ms
            = 0.0   if age >= ttl_ms
```

A hard cutoff: full weight until the TTL, then zero. Appropriate when a record is either
valid or invalid with no gradual transition.

**Note**: The `ttl_ms` field is a relative duration, not an absolute timestamp. Absolute
deadlines are handled at a higher layer: the emitter computes `ttl_ms = deadline - now` at
construction time.

### 3.4 Ebbinghaus -- Forgetting Curve

```
weight(age) = exp(-age_ms / (strength * scale_ms))
```

Named after Hermann Ebbinghaus (1885), who first quantified human memory retention decay.
His experiments demonstrated that memory retention follows an exponential-family curve where
initial forgetting is rapid and then slows progressively.

Ebbinghaus, H. (1885). *Uber das Gedachtnis: Untersuchungen zur experimentellen Psychologie*.
Leipzig: Duncker & Humblot. English translation: *Memory: A Contribution to Experimental
Psychology* (1913, Teachers College, Columbia University).

**Parameters**:
- `strength` in [0, inf): retention multiplier. Higher = signal persists longer.
- `scale_ms`: base time unit in milliseconds.

**Properties**:
- `weight(0) = 1.0`
- `weight(strength * scale_ms) = 1/e ~ 0.368`
- The effective time constant is `tau = strength * scale_ms`

**Edge cases**: Zero `scale_ms`, non-finite `strength`, or `strength <= 0` all return 0.0.

---

## 4. Decay Application and Safety

The shipped `apply()` method handles edge cases defensively:

```rust
pub fn apply(&self, age_ms: i64) -> f32 {
    if age_ms <= 0 { return 1.0; }  // negative age (clock skew) is full weight
    // ... variant-specific computation ...
    finite_weight(result)  // clamp to [0.0, 1.0], replace NaN/Inf with 0.0
}
```

The `is_alive()` method provides a convenience threshold check:

```rust
pub fn is_alive(&self, age_ms: i64, threshold: f32) -> bool {
    threshold.is_finite() && self.apply(age_ms) > threshold
}
```

---

## 5. Knowledge Tier Matrix

The tier policy governs how durable knowledge entries (in `roko-neuro`) behave over time.
The four tiers express progressively stronger confidence in the knowledge's durability.

### 5.1 The 4x4 Matrix

| Tier | Demurrage charge | Reinforcement stickiness | Cold-floor behavior | Thaw rule |
|---|---|---|---|---|
| **Transient** | Highest charge. Balance drops quickly unless actively used. | Reacts fast to `Retrieved` and `Surprised`, but gain is easy to lose. | Freezes early when balance falls below floor or contradictions stack up. | Easy to thaw if cited again or reused successfully. |
| **Working** | Baseline charge. Useful entries stay warm, but neglect costs balance. | `Retrieved`, `Cited`, and `Gated` events keep it sticky across a few episodes. | Freezes when balance weakens or repeated failures show the entry is no longer current. | Thaws on successful reuse and usually returns to the same tier. |
| **Consolidated** | Lower charge. Long-lived knowledge ages slowly unless contradicted. | Cross-plan citations and successful gates matter more than repetition. | Freezes only after sustained contradiction or a long slide in balance. | Thaw requires fresh confirmation, not just a single read. |
| **Persistent** | Lowest charge. Entry should remain available unless the system has strong reason to move it. | Reinforcement mostly protects already-earned balance rather than adding new balance. | Usually pinned rather than deleted. Only extraordinary contradiction or policy says otherwise. | Thaw is explicit and policy-gated. |

### 5.2 Tier Definition

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize)]
pub enum KnowledgeTier {
    Transient,
    Working,
    Consolidated,
    Persistent,
}

impl KnowledgeTier {
    pub fn charge_multiplier(&self) -> f32 {
        match self {
            Self::Transient    => 2.0,
            Self::Working      => 1.0,
            Self::Consolidated => 0.5,
            Self::Persistent   => 0.1,
        }
    }

    pub fn reinforcement_multiplier(&self) -> f32 {
        match self {
            Self::Transient    => 1.5,
            Self::Working      => 1.0,
            Self::Consolidated => 0.75,
            Self::Persistent   => 0.5,
        }
    }

    pub fn cold_floor(&self) -> f64 {
        match self {
            Self::Transient    => 0.25,
            Self::Working      => 0.15,
            Self::Consolidated => 0.05,
            Self::Persistent   => 0.0,
        }
    }
}
```

### 5.3 Suggested Balance Bands

| Tier | Balance band |
|---|---|
| **Transient** | < 0.35 |
| **Working** | 0.35 - 0.8 |
| **Consolidated** | 0.8 - 1.2 |
| **Persistent** | > 1.2 |

---

## 6. Demurrage: Target-State Extension

The demurrage model extends decay with an attention-economy mechanism where Signals
explicitly carry balance that is taxed over time and restored through use.

### 6.1 Demurrage State

```rust
pub struct Engram {
    // ... existing fields ...
    pub balance: f64,          // starts at 1.0
    pub demurrage_paid: f64,   // monotonic total holding cost
    pub last_touched_at: Timestamp,
}
```

### 6.2 Demurrage Rate Law

```
balance(t + dt) = balance(t) - flat_tax * dt - exp_tax * balance(t) * dt
```

This gives the system a floor-aware, compounding holding cost.

### 6.3 Effective Weight Under Demurrage

```
weight = score.effective() x demurrage.effective_weight(engram)
```

This changes the shape of memory selection in three ways:
1. Age alone no longer decides relevance.
2. Reinforced knowledge can outrank older but unused knowledge.
3. Retrieval learns from use, not just passage of time.

### 6.4 Reinforcement Kinds

```rust
pub enum ReinforceKind {
    Cited,        // participates in lineage
    Retrieved,    // solved a query
    Gated,        // survived verification
    Surprised,    // informationally novel
    AgentQuoted,  // another agent turned it into output
}
```

### 6.5 Novelty-Weighted Reinforcement

Reinforcement is coupled to HDC similarity for anti-hoarding:

```
reinforcement = bonus(kind) x novelty(engram)
novelty = 1 - max(similarity(top-K HDC neighbors))
```

Citing a common Signal gives a small bump. Citing a rare Signal gives a larger bump.

---

## 7. Promotion Rules

Promotion is based on usage plus balance, not age alone:

```
Transient --> Working:
  - At least 2 successful uses in distinct episodes
  - Balance remains above the Transient floor (0.35)
  - No unresolved contradiction in the recent window

Working --> Consolidated:
  - At least 5 successful uses across 2+ plans
  - Balance remains in the Working/Consolidated band (>= 0.8)
  - Contradictions do not dominate recent evidence

Consolidated --> Persistent:
  - At least 10 successful uses across 3+ sessions
  - Balance remains above the Persistent threshold (>= 1.2)
  - No unresolved contradictions in the recent window
```

---

## 8. Demotion Rules

Demotion is the reverse path: balance falls, contradiction rises, or the entry stops paying
for its place in memory.

```
Persistent --> Consolidated:
  - Balance drops below 1.2
  - OR a recent contradiction removes confidence

Consolidated --> Working:
  - Balance drops below 0.8
  - OR repeated contradictions in a short window

Working --> Transient:
  - Balance drops below 0.35
  - OR several failed uses show the entry is no longer current

Transient --> Cold storage:
  - Balance falls to the floor
  - OR entry is repeatedly contradicted
```

---

## 9. Demurrage by Knowledge Type

| Knowledge type | Demurrage shape | Reinforcement bias | Cold-floor note |
|---|---|---|---|
| **Insight** | Moderate charge | Strong on reuse and citation | Freeze only after long non-use or contradiction |
| **Heuristic** | Moderate charge, more stickiness after validation | `Gated` and `AgentQuoted` matter more | Should thaw into Working before promotion |
| **Warning** | Higher charge unless proving current relevance | `Surprised` events keep it alive | Stale warnings should not dominate routing |
| **CausalLink** | Lower charge (causal knowledge survives longer) | `Cited` and `Gated` are sticky | Contradiction demotes faster than age |
| **StrategyFragment** | Low charge, broad utility | Cross-plan reuse reinforces strongly | Good candidate for cold storage and later thaw |
| **AntiKnowledge** | Special case: ordinary demurrage should not make falsified knowledge vanish | Reinforce when it prevents re-exploration of dead ends | Stays retrievable even when frozen |

---

## 10. Cold Tier: Freeze and Thaw

```rust
pub trait ColdStore: Send + Sync {
    async fn archive(&self, signal: Signal) -> Result<ContentHash>;
    async fn thaw(&self, id: &ContentHash) -> Result<Option<Signal>>;
    async fn contains(&self, id: &ContentHash) -> Result<bool>;
    async fn archived_count(&self) -> Result<usize>;
    async fn storage_bytes(&self) -> Result<u64>;
    async fn purge_before(&self, epoch_ms: i64) -> Result<usize>;
}
```

The flow:
1. `charge()` reduces balance over time.
2. Reinforcement raises balance while the Signal remains useful.
3. When balance reaches `cold_floor()`, the entry is frozen via `ColdStore::archive()`.
4. Retrieval can thaw it on demand and reset balance to a starter value (default 0.3).
5. Thawing emits a Bus Pulse so observers can update caches and policy state.

This is not hard deletion. It is a tier shift from hot to cold with lineage intact.

---

## 11. Configuration Parameters

### 11.1 Tier Charge and Reinforcement

| Parameter | Default | Range | Meaning |
|---|---|---|---|
| `transient_charge_multiplier` | 2.0 | 1.0 - 4.0 | Highest demurrage charge |
| `working_charge_multiplier` | 1.0 | 0.5 - 2.0 | Baseline charge |
| `consolidated_charge_multiplier` | 0.5 | 0.1 - 1.0 | Lower charge for cross-validated knowledge |
| `persistent_charge_multiplier` | 0.1 | 0.01 - 0.5 | Near-pinned charge |
| `transient_reinforcement_multiplier` | 1.5 | 1.0 - 3.0 | Fast response to new evidence |
| `working_reinforcement_multiplier` | 1.0 | 0.5 - 2.0 | Balanced reinforcement |
| `consolidated_reinforcement_multiplier` | 0.75 | 0.25 - 1.5 | Broad but restrained stickiness |
| `persistent_reinforcement_multiplier` | 0.5 | 0.1 - 1.0 | Mostly preserves earned balance |

### 11.2 Cold Floor and Thaw

| Parameter | Default | Range | Meaning |
|---|---|---|---|
| `transient_floor_balance` | 0.25 | 0.05 - 0.5 | Freeze threshold for Transient |
| `working_floor_balance` | 0.15 | 0.05 - 0.3 | Freeze threshold for Working |
| `consolidated_floor_balance` | 0.05 | 0.0 - 0.2 | Freeze threshold for Consolidated |
| `persistent_floor_balance` | 0.0 | 0.0 - 0.1 | Persistent is usually pinned |
| `thaw_start_balance` | 0.3 | 0.1 - 0.5 | Conservative balance after thaw |

### 11.3 Demurrage Rates

```toml
[demurrage]
flat_tax_per_day   = 0.01
exp_decay_per_day  = 0.005
min_balance        = 0.0
cited_bonus        = 0.05
retrieved_bonus    = 0.02
gated_bonus        = 0.03
surprised_bonus    = 0.15
agent_quoted_bonus = 0.08
```

---

## 12. Error Handling

| Condition | Response |
|---|---|
| Balance falls below zero | Clamp to cold floor and freeze if needed |
| Promotion and demotion both trigger | Demotion wins if contradiction or floor pressure is present |
| Clock skew (negative elapsed time) | Do not charge demurrage for the negative interval |
| Thaw request hits witness-locked entry | Rehydrate only if policy allows; otherwise keep frozen |
| Reinforcement and charge collide in same tick | Apply charge first, then reinforcement |

---

## 13. Worked Examples

### Example 1: Playbook Freshness

```
t=0:   Playbook created. Tier = Transient. Balance = 1.0.
t=1d:  Agent reuses it twice in successful tasks. Reinforcement offsets charge.
       Balance stays above 0.35. Promote to Working.
t=1w:  The same playbook keeps being cited across two plans.
       Balance stays in Working/Consolidated band. Promote to Consolidated.
t=1m:  The playbook is still useful across multiple sessions.
       Balance stays above 1.2. Promote to Persistent.
```

### Example 2: Contradiction and Thaw

```
t=0:   Heuristic is Consolidated. Balance = 0.9.
t=2d:  Two tasks fail after following the heuristic.
       Recent contradiction pushes it below Consolidated band. Demote to Working.
t=5d:  More failures drop balance to floor. Freeze into cold storage.
t=20d: A new task reuses the same pattern and the frozen entry is thawed.
       Bus publishes thaw Pulse, entry returns with conservative balance.
```

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Ebbinghaus 1885, *Uber das Gedachtnis* | Forgetting curve and retention decay. Foundation for the Ebbinghaus variant. |
| Gesell 1916 | Demurrage as a carrying cost on idle money. Economic inspiration for attention-economy memory. |
| Averell & Heathcote 2011 | Exponential traces with power-law aggregates. |
| Murre & Dros 2015 | Sleep-linked consolidation discontinuity. |
| FSRS / spaced-repetition work | Reinforcement updates strength from retrieval history. |

---

## Cross-References

- `score-7-axis-appraisal.md` -- How Score and Decay combine into weight
- `substrate-trait.md` -- How pruning uses decay
- `naming-and-glossary.md` -- Canonical vocabulary
