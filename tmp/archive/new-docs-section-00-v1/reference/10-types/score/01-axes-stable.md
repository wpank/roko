# Score — Stable Axes

> The 4 stable axes: confidence, novelty, utility, reputation. Present on every Engram; always in [0.0, 1.0].

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The 4 stable axes are mandatory — every Engram has them. They are the axes that enter
the effective score formula and are the primary inputs to gate thresholds. Each axis
has a clear semantic meaning and a set of Scorers that compute it.

---

## Axis 1: `confidence` — Certainty of the Source

**What it measures:** How certain is the component that produced this Engram that the
content is correct?

**Semantic range:**
- 1.0 = The source is 100% certain (e.g., a deterministic algorithm with a verified result)
- 0.5 = 50% confidence (e.g., an LLM output without any verification)
- 0.0 = The source is certain this is wrong (e.g., a failed assertion)

**How it is typically set:**

| Source | Typical confidence |
|--------|-------------------|
| Deterministic computation | 1.0 |
| LLM output (unverified) | 0.5–0.7 |
| LLM output (chain-of-thought, self-checked) | 0.7–0.85 |
| LLM output (peer-verified) | 0.85–0.95 |
| HDC active inference (from prediction error) | `1.0 - normalized_hamming_distance` |
| Gate verdict | The gate's `confidence` field |

**Weight in effective score:** `w_confidence = 0.35` (highest weight — correctness is paramount)

---

## Axis 2: `novelty` — Information Newness

**What it measures:** How new is this information relative to what is already in the
Substrate? High novelty = the Substrate has not seen this before.

**Semantic range:**
- 1.0 = Completely new; no similar Engrams exist in the Substrate
- 0.5 = Moderate novelty; some related Engrams exist
- 0.0 = Identical to existing content; pure duplicate

**How it is computed:**

Novelty is typically computed by a Scorer that queries the HDC fingerprint similarity:

```rust
<!-- source: crates/roko-core/src/scorer/novelty.rs -->

fn score_novelty(engram: &Engram, substrate: &impl Substrate) -> f64 {
    if let Some(fp) = &engram.fingerprint {
        let similar = substrate.find_similar(fp, 0.0, 1);
        if let Some((best, similarity)) = similar.into_iter().next() {
            1.0 - similarity as f64
        } else {
            1.0  // no similar Engrams: maximally novel
        }
    } else {
        0.5  // no fingerprint: assume moderate novelty
    }
}
```

**Weight in effective score:** `w_novelty = 0.20`

---

## Axis 3: `utility` — Proven Usefulness

**What it measures:** Has this Engram been useful in past agent runs? High utility
means: when this Engram was retrieved, the downstream agent succeeded.

**Semantic range:**
- 1.0 = Always led to a successful gate verdict when retrieved
- 0.5 = Mixed track record; roughly as often helpful as not
- 0.0 = Never led to success; likely misleading or outdated

**How it is computed:**

Utility is an outcome-driven score. At emission time, utility = 0.5 (no data). After
a gate verdict resolves on downstream Engrams that had this in their lineage or context,
the utility is updated:

```rust
// When a gate passes and Engram X was in context:
substrate.update_score(&x.id, Score { utility: x.score.utility + UTILITY_DELTA, ..x.score })?;

// When a gate fails:
substrate.update_score(&x.id, Score { utility: (x.score.utility - UTILITY_DELTA).max(0.0), ..x.score })?;
```

**Weight in effective score:** `w_utility = 0.30` (second-highest — proven value matters)

---

## Axis 4: `reputation` — Source Trustworthiness

**What it measures:** How trustworthy is the author of this Engram?

**Semantic range:**
- 1.0 = Chain-witnessed: the Engram's provenance has been attested on a distributed ledger
- 0.75 = Peer-verified: another agent in the mesh has reviewed and attested
- 0.5 = Self-verified: the author checked its own output
- 0.25 = Local agent: unverified, local-process output
- 0.0 = Tainted: source is known-bad

**How it is computed:**

Reputation maps directly from `TrustLevel`:

```rust
<!-- source: crates/roko-core/src/scorer/reputation.rs -->

fn trust_to_reputation(trust: TrustLevel, tainted: bool) -> f64 {
    if tainted { return 0.0; }
    match trust {
        TrustLevel::LocalAgent   => 0.25,
        TrustLevel::SelfVerified => 0.50,
        TrustLevel::PeerVerified => 0.75,
        TrustLevel::ChainWitness => 1.00,
    }
}
```

**Weight in effective score:** `w_reputation = 0.15`

---

## Interactions Between Stable Axes

The axes are independent inputs to the effective score formula, but they have semantic
relationships:

- **confidence × reputation**: A high-confidence claim from a low-reputation source
  is less trustworthy than the same claim from a high-reputation source. The effective
  score reflects this naturally: if reputation is 0.25 and confidence is 0.9, the
  effective score is lower than if reputation were 0.75.

- **novelty × utility**: A highly novel Engram has low utility by definition (it has
  not been used yet). As the Engram is retrieved and proves useful, utility rises while
  novelty typically stays the same.

---

## Invariants

1. All stable axes are in [0.0, 1.0]
2. All stable axes are always present (not `Option<f64>`)
3. `Score::default()` sets all stable axes to 0.5

---

## See Also

- [`02-axes-extended.md`](02-axes-extended.md) — optional extended axes
- [`03-arithmetic.md`](03-arithmetic.md) — how axes combine
- [`04-constants.md`](04-constants.md) — weight constants
