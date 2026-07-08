# Score — Rationale

> Why 7 axes; what was considered and dropped; why these specific weights.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## Why 7 Axes, Not 1

A single-number score (0.0–1.0) loses information. An agent output can be high-confidence
but from a low-reputation source. A knowledge entry can be highly novel but not yet proven
useful. Different consumers legitimately need different quality signals:
- A factual accuracy gate cares most about confidence.
- The Substrate GC cares most about utility.
- A trust-sensitive policy cares most about reputation.

7 axes is a compromise: rich enough to be useful, small enough to be comprehensible.
The 4 stable axes are sufficient for most use cases. The 3 extended axes handle
specialized needs without cluttering the core structure.

---

## Why These 4 Stable Axes

**confidence**: The most fundamental quality signal. A system that cannot distinguish
"I'm sure" from "I'm guessing" cannot work reliably. Confidence is the foundation.

**novelty**: Agents that only retrieve information they already know stagnate. Novelty
drives exploration and learning. It is valued as a second-class signal (lower weight than
confidence and utility) to avoid novelty-seeking at the expense of correctness.

**utility**: The best evidence that an Engram is worth keeping is that it has contributed
to successful outcomes. Utility is the memory system's feedback signal — the equivalent
of synaptic potentiation in biological memory.

**reputation**: Source attribution is necessary for safety and trust hierarchies.
Without reputation, a LocalAgent output and a ChainWitness attestation look equal.

---

## Axes Considered and Dropped

| Candidate axis | Why dropped |
|----------------|-------------|
| `freshness` | Covered by the Decay model; redundant with `created_at_ms` |
| `specificity` | Merged into `precision` (extended axis) |
| `relevance` | Renamed `salience` (extended); context-dependent, so not stored |
| `completeness` | Hard to measure automatically; subjective |
| `verifiability` | Subsumed by `confidence` (if it can't be verified, confidence should be low) |
| `complexity` | Not a quality signal; a property of the content |
| `cost` | A routing concern, not a quality concern |

---

## Why These Weights

The weights `(0.35, 0.20, 0.30, 0.15)` for `(confidence, novelty, utility, reputation)`
were chosen based on the following reasoning:

1. **Correctness dominates.** An agent that produces confidently wrong outputs is worse
   than one that produces uncertainly correct ones. `W_CONFIDENCE = 0.35` is the highest weight.

2. **Evidence beats priors.** Utility (outcome evidence) outweighs reputation (source prior).
   `W_UTILITY = 0.30 > W_REPUTATION = 0.15`.

3. **Novelty is secondary.** New information is good, but not if it is wrong or unproven.
   `W_NOVELTY = 0.20` is the second-lowest weight.

4. **Weights sum to 1.0.** This ensures `effective() ∈ [0.0, 1.0]` without normalization.

---

## Open Questions

- Are the weight constants empirically validated? (No — they are informed priors.)
- Should weights be per-substrate-configuration (different substrates for different use cases)?
- Should the extended axes have non-zero default weights for specialized deployments?

---

## See Also

- [`04-constants.md`](04-constants.md) — the weight values
- [`03-arithmetic.md`](03-arithmetic.md) — the formula
