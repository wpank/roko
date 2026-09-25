# Four Validation Tiers

> **v3 depth -- 09-memory** | Source: v1/06-neuro/02

Knowledge reliability is tracked through four validation tiers -- Transient,
Working, Consolidated, Persistent -- each with a multiplicative effect on the
base half-life of the knowledge type. Every entry has both a **type** (what
kind of knowledge) and a **tier** (how validated it is).

The four tiers form a progression from unvalidated to core knowledge. Each
tier carries a **multiplier** that scales the type's base half-life: a
Transient entry decays 10x faster than its base rate, while a Persistent
entry decays 5x slower. This two-dimensional decay model (type x tier)
ensures that unreliable knowledge disappears quickly while proven knowledge
persists.

Tier transitions are driven by **outcome feedback**: successful use of a
knowledge entry promotes its tier (increasing the multiplier), while
unsuccessful use demotes it. This creates a natural selection pressure where
useful knowledge rises to Persistent and unreliable knowledge decays to
nothing. The system implements a computational analogue of Complementary
Learning Systems theory (McClelland et al. 1995).

---

## The Four Tiers

### Transient (Multiplier: 0.1x)

**Definition.** Just extracted, unvalidated. The entry has been distilled
from a single episode or small cluster of episodes but has not yet been used
or cross-validated.

**Effective half-life examples:**

| Type | Base Half-Life | x Transient (0.1x) | Effective |
|------|---------------|--------------------|-----------|
| Insight | 30 days | x 0.1 | **3 days** |
| Heuristic | 90 days | x 0.1 | **9 days** |
| Warning | 1 hour | x 0.1 | **6 minutes** |
| CausalLink | 60 days | x 0.1 | **6 days** |
| StrategyFragment | 14 days | x 0.1 | **1.4 days** |

**Entry criteria:** All newly distilled knowledge entries start at Transient.
Entries restored from backup also start at Transient (they must re-prove
themselves in the new context).

**Promotion criteria:** 2+ confirmations with confidence >= 0.5.

**Demotion criteria:** None (Transient is the lowest tier). Entries that are
never used simply decay and are eventually garbage-collected when confidence
falls below `DEFAULT_GC_MIN_CONFIDENCE` (0.05).

**Rationale:** The 0.1x multiplier creates a strong filter. A Transient
Insight has only 3 days of effective half-life -- if it is not used and
confirmed within that window, it will decay rapidly. This mirrors the
hippocampus's rapid forgetting of unconsolidated episodic memories
(McClelland et al. 1995).

### Working (Multiplier: 0.5x)

**Definition.** Used and confirmed. The entry has been retrieved during a
task, the agent acted on it, and the outcome was positive (gate check
passed). It has demonstrated utility in at least one context.

**Effective half-life examples:**

| Type | Base Half-Life | x Working (0.5x) | Effective |
|------|---------------|-------------------|-----------|
| Insight | 30 days | x 0.5 | **15 days** |
| Heuristic | 90 days | x 0.5 | **45 days** |
| Warning | 1 hour | x 0.5 | **30 minutes** |
| CausalLink | 60 days | x 0.5 | **30 days** |
| StrategyFragment | 14 days | x 0.5 | **7 days** |

**Entry criteria:** Promoted from Transient after 2+ successful
confirmations with confidence >= 0.5.

**Promotion criteria:** 3+ distinct contexts with confidence >= 0.6.

**Demotion criteria:** Confidence drops below 0.15. A single negative outcome
is enough for demotion -- the asymmetry between promotion (requires multiple
successes) and demotion (single negative signal) reflects the precautionary
principle.

**Rationale:** Working tier is the standard operating tier for most
knowledge. With a 0.5x multiplier, entries have moderate durability -- enough
to be useful across multiple episodes but not so persistent that stale
entries linger.

### Consolidated (Multiplier: 1.0x)

**Definition.** Validated through repeated use and cross-validation. The
entry has been used 3+ times with positive outcomes across distinct contexts
and has been cross-validated.

**Effective half-life examples:**

| Type | Base Half-Life | x Consolidated (1.0x) | Effective |
|------|---------------|----------------------|-----------|
| Insight | 30 days | x 1.0 | **30 days** |
| Heuristic | 90 days | x 1.0 | **90 days** |
| Warning | 1 hour | x 1.0 | **1 hour** |
| CausalLink | 60 days | x 1.0 | **60 days** |
| StrategyFragment | 14 days | x 1.0 | **14 days** |

**Entry criteria:** Promoted from Working after 3+ distinct contexts with
confidence >= 0.6.

**Promotion criteria:** 30+ days old with confidence >= 0.8.

**Demotion criteria:** Confidence drops below 0.3. Can also be demoted by
newer evidence, AntiKnowledge entries that specifically refute this entry, or
conflicting entries confirmed by multiple agents.

**Rationale:** Consolidated is the "normal" tier where the base half-life
applies without modification (multiplier 1.0x). A Consolidated Insight
(30-day half-life) will naturally need revalidation monthly.

### Persistent (Multiplier: 5.0x)

**Definition.** Core knowledge with high reputation. Used extensively,
confirmed by multiple sources, and represents foundational understanding that
the agent relies on regularly.

**Effective half-life examples:**

| Type | Base Half-Life | x Persistent (5.0x) | Effective |
|------|---------------|--------------------|-----------|
| Insight | 30 days | x 5.0 | **150 days** |
| Heuristic | 90 days | x 5.0 | **450 days** |
| Warning | 1 hour | x 5.0 | **5 hours** |
| CausalLink | 60 days | x 5.0 | **300 days** |
| StrategyFragment | 14 days | x 5.0 | **70 days** |

**Entry criteria:** Promoted from Consolidated after 30+ days old with
confidence >= 0.8.

**Promotion criteria:** None -- Persistent is the highest tier.

**Demotion criteria:** Explicitly deprecated. Persistent entries are not
automatically demoted by a single negative outcome (unlike lower tiers).
They require explicit deprecation -- either by a user command, by
overwhelming AntiKnowledge evidence, or by a manual review process.

**Rationale:** The 5.0x multiplier creates extremely durable knowledge. A
Persistent Heuristic (450-day half-life) persists for over a year. This
durability is appropriate for core knowledge, but incorrect entries at this
tier are dangerous -- they resist correction.

---

## Tier Transition Mechanics

### Promotion flow

```
Transient --(2+ confirms, conf >= 0.5)--> Working
  --(3+ distinct contexts, conf >= 0.6)--> Consolidated
  --(30+ days, conf >= 0.8)--> Persistent
```

Each promotion is tracked through `KnowledgeConfirmationRecord`:

```rust
pub struct KnowledgeConfirmationRecord {
    pub entry_id: String,
    pub confirmed: bool,
    pub episode_id: String,
    pub timestamp: DateTime<Utc>,
}
```

### Demotion flow

```
Persistent --(explicit deprecation)--> Consolidated
  --(confidence < 0.3)--> Working
  --(confidence < 0.15)--> Transient
```

Demotion rules differ by tier:
- **Working -> Transient**: Confidence drops below 0.15
- **Consolidated -> Working**: Confidence drops below 0.3, or contradicted by
  newer evidence or AntiKnowledge
- **Persistent -> Consolidated**: Explicit deprecation only

### Confidence boost on confirmation

```rust
pub const CONFIRMATION_BOOST: f64 = 1.5;
```

Each confirmation multiplies confidence by 1.5, clamped to [0.0, 1.0].
An entry at 0.4 confirmed once becomes 0.6; confirmed again, 0.9.

### Garbage collection

Entries below `DEFAULT_GC_MIN_CONFIDENCE` (0.05) are removed.
AntiKnowledge entries are exempt from GC below their 0.3 floor.

---

## Two-Dimensional Decay: Type x Tier

The effective decay rate is determined by two independent dimensions:

```
effective_half_life = tier_multiplier * type_base_half_life
```

This produces a 5x4 matrix:

| | Transient (0.1x) | Working (0.5x) | Consolidated (1.0x) | Persistent (5.0x) |
|---|---|---|---|---|
| **Insight** (30d) | 3 days | 15 days | 30 days | 150 days |
| **Heuristic** (90d) | 9 days | 45 days | 90 days | 450 days |
| **Warning** (1h) | 6 min | 30 min | 1 hour | 5 hours |
| **CausalLink** (60d) | 6 days | 30 days | 60 days | 300 days |
| **StrategyFragment** (14d) | 1.4 days | 7 days | 14 days | 70 days |

Key observations:
- A Transient Warning (6 minutes) decays extremely fast -- if it is not
  confirmed almost immediately, it vanishes
- A Persistent Heuristic (450 days) is essentially permanent for operational
  purposes
- A Working Insight (15 days) is the default state for most knowledge

---

## CLS Theory Mapping

The four-tier system implements Complementary Learning Systems theory
(McClelland, McNaughton, and O'Reilly 1995):

| CLS Concept | Neuro Implementation |
|-------------|---------------------|
| **Hippocampal fast learning** | Transient tier -- created quickly, decays rapidly |
| **Neocortical slow learning** | Persistent tier -- stable, generalized patterns |
| **Consolidation during sleep** | Dreams subsystem replays and promotes tiers |
| **Interference protection** | Tier separation -- Transient cannot corrupt Persistent |
| **Gradual transfer** | Working -> Consolidated -> Persistent as evidence accumulates |

The CLS model predicts that memories stored quickly in the hippocampus are
initially fragile and must be gradually consolidated into the neocortex
through repeated replay. Neuro mirrors this: fast-extracted Transient entries
are fragile (0.1x multiplier) and must be promoted through use and replay to
reach the stable Persistent tier (5.0x multiplier).

---

## Interaction with Confidence

Tiers and confidence are related but distinct:

- **Confidence** (0.0--1.0) is a continuous score reflecting accumulated
  evidence for/against the entry
- **Tier** (Transient/Working/Consolidated/Persistent) is a discrete level
  reflecting validation state

Confidence changes on every use:
- Positive outcome: confidence x `CONFIRMATION_BOOST` (1.5x)
- Negative outcome: confidence x 0.5 (halved)

An entry can have high confidence at a low tier (e.g., a Transient entry
with 0.8 confidence that was just extracted from a high-quality source) or
low confidence at a high tier (e.g., a Persistent entry with 0.4 confidence
that has not been used recently and has decayed). Tier determines the decay
rate; confidence determines the retrieval priority.

---

## Academic Foundations

- McClelland, J. L. et al. (1995). "Why there are complementary learning
  systems in the hippocampus and neocortex." *Psychological Review*, 102(3).
- Ebbinghaus, H. (1885). *Uber das Gedachtnis* (On Memory).
- Walker, M. P. & van der Helm, E. (2009). "Overnight therapy?"
  *Psychological Bulletin*, 135(5).
- Nader, K. et al. (2000). "Fear memories require protein synthesis in the
  amygdala for reconsolidation after retrieval." *Nature*, 406.

---

## Cross-References

- `six-knowledge-types.md` -- base half-lives per type
- `type-half-lives.md` -- half-life rationale in detail
- `ebbinghaus-decay-with-tier.md` -- full decay formula
- `4-tier-distillation-pipeline.md` -- how distillation drives tier progression
