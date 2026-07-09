# SCORE — Stage 2 of the Cognitive Loop

> Appraise each candidate Engram on seven independent axes and produce a ranked list.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Score type](../10-types/score.md), [Engram](../01-engram/README.md),
[Scorer operator](../05-operators/scorer.md)
**Used by**: [ROUTE](03-stage-route.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

SCORE takes the raw candidate set from QUERY and attaches a `Score` to each Engram.
A `Score` is a seven-axis appraisal covering relevance, recency, trust, utility,
novelty, emotional valence, and cost. The stage returns a ranked `Vec<ScoredEngram>`.
ROUTE will then select from the top of this list.

---

## The Idea

Not all knowledge is equally useful in context. A highly relevant but stale Engram is
less useful than a moderately relevant but fresh one. An Engram from a trusted source
is more useful than one of unknown provenance. Emotional salience may amplify or dampen
relevance depending on the agent's current affective state.

The SCORE stage resolves these tensions by computing a single `Score` struct for each
candidate. The struct holds seven axis values, not a single collapsed float, because
the relative weighting of axes depends on context — a safety-critical task weights
Trust higher than Novelty; a creative task does the reverse.

The Scorer operator encapsulates the weighting policy. The default scorer applies a
configurable linear combination. Domain-specific scorers may apply nonlinear functions
or learned weights.

---

## The Seven Axes

| Axis | Symbol | Range | What it measures |
|---|---|---|---|
| Relevance | `r` | 0.0–1.0 | Semantic distance to the current stimulus (HDC cosine) |
| Recency | `t` | 0.0–1.0 | Temporal freshness (exponential decay from `created_at`) |
| Trust | `τ` | 0.0–1.0 | Provenance attestation quality |
| Utility | `u` | 0.0–1.0 | Historical success rate when this Engram was used |
| Novelty | `ν` | 0.0–1.0 | Information gain relative to what is already in context |
| Valence | `v` | −1.0–1.0 | Emotional charge (from Daimon; 0.0 if Daimon absent) |
| Cost | `c` | 0.0–∞ | Estimated token / compute cost to include this Engram |

The composite score used for ranking is:

```
composite = w_r·r + w_t·t + w_τ·τ + w_u·u + w_ν·ν + w_v·|v| − w_c·c
```

Weights are configurable per agent role. Default weights:
`w_r=0.40, w_t=0.20, w_τ=0.15, w_u=0.10, w_ν=0.10, w_v=0.05, w_c=0.10`.

---

## Specification

```rust
// source: crates/roko-core/src/score.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub relevance: f32,   // 0.0–1.0
    pub recency:   f32,   // 0.0–1.0
    pub trust:     f32,   // 0.0–1.0
    pub utility:   f32,   // 0.0–1.0
    pub novelty:   f32,   // 0.0–1.0
    pub valence:   f32,   // −1.0–1.0
    pub cost:      f32,   // 0.0–∞
}

pub struct ScoredEngram {
    pub engram:    Engram,
    pub score:     Score,
    pub composite: f32,
}

pub trait Scorer: Send + Sync {
    fn score(
        &self,
        candidates: Vec<Engram>,
        stimulus:   &Pulse,
        context:    &ScorerContext,
    ) -> Vec<ScoredEngram>;
}
```

`ScorerContext` carries the current affective state (from Daimon), the active weight
vector, and the current tick timestamp (for recency calculations).

---

## Semantics

1. Receive `Vec<Engram>` from QUERY (may be empty → return empty vec).
2. For each `Engram`, compute all seven axis values.
3. Compute `composite` using the active weight vector.
4. Sort descending by `composite`.
5. Return `Vec<ScoredEngram>`.

The sort is stable. Ties are broken by Engram `id` for determinism.

Axis computation details:

- **Relevance**: `1.0 − hdcDistance(engram.fingerprint, stimulus.fingerprint)`
  clamped to 0.0–1.0.
- **Recency**: `exp(−λ · (now − engram.created_at))` where `λ` is the decay rate
  for the Engram's tier. See [Decay Variants](../10-types/decay.md).
- **Trust**: derived from the Engram's `Provenance` record. See
  [Provenance](../10-types/provenance.md).
- **Utility**: exponential moving average of `reward` signals from prior ticks that
  used this Engram. Defaults to 0.5 for unseen Engrams.
- **Novelty**: `1.0 − maxSimilarity(engram, already_composed)`. For the first
  candidate, novelty = 1.0.
- **Valence**: pulled directly from the Engram's `affect_charge` field (set by Daimon
  at persist time). 0.0 for agents without Daimon.
- **Cost**: estimated by `tokenCount(engram.body) × costPerToken`.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `ScorerError::DivisionByZero` | All weights are zero | Panic in debug; abort tick in release |
| `ScorerError::NaN` | HDC distance produced NaN | Treat as 0.0 relevance; log error |
| Empty output | Input was empty | Return empty `Vec<ScoredEngram>`; tick continues |
| Scoring timeout | Scorer took > stage budget | Return partially scored candidates (sorted by best available) |

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Wall time (64 candidates) | < 2 ms | < 5 ms |
| Wall time (256 candidates) | < 8 ms | < 15 ms |
| Per-candidate cost | O(1) | — |
| Memory allocations | O(N) | — |

Scoring is embarrassingly parallel. The default implementation processes candidates
sequentially; a future optimization can parallelize with `rayon` at no API-surface cost
(the trait is `Send + Sync`).

---

## Examples

### 1. Relevance-dominant ranking

A research agent queries for "active inference Free Energy" and retrieves 12 Engrams.
The three most relevant by HDC similarity float to the top of the ranked list. A fourth
Engram has slightly lower relevance but very high trust (it was attested by a peer
agent with a long positive history), landing it in position 2.

### 2. Novelty deduplication

COMPOSE has already selected two Engrams about Free Energy. A third candidate covers
the same ground. Its Novelty axis is near zero. Even if Relevance is high, its
composite score drops, preventing redundant context.

### 3. Valence boost (Daimon active)

An agent is in a high-urgency behavioral state. Daimon has marked Engrams tagged with
`[urgency]` with a positive valence charge. The Scorer's valence weight amplifies those
candidates, surfacing time-sensitive knowledge ahead of equally relevant but neutral
material.

---

## See also

- [Score type](../10-types/score.md) — the full type definition with invariants
- [Scorer operator](../05-operators/scorer.md) — how to customize scoring
- [QUERY](01-stage-query.md) — produces the candidates scored here
- [ROUTE](03-stage-route.md) — consumes the ranked list produced here
- [Daimon cross-cut](../09-cross-cuts/02-daimon.md) — the source of valence signals
