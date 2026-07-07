# roko-neuro — Test Coverage

> Tests for the tiered knowledge system: 6 knowledge types × 4 validation tiers, HDC encoding, and sub-millisecond similarity search.

**Status**: Built (crate is built but not wired to the runtime)
**Crate**: `roko-neuro`
**Section**: 06 — Neuro
**Last reviewed**: 2026-04-19

---

## Test Count

Exact count not in the 2026-04-17 audit (the status report listed `roko-neuro` as "Built" with no test count). The `roko-core` HDC tests (part of the 376 `roko-core` tests) cover the hypervector primitives that `roko-neuro` builds on.

---

## Key Test Focus Areas

### Knowledge Types

Roko-neuro stores six knowledge types:
1. `Insight` — a validated factual claim.
2. `Heuristic` — a rule of thumb with known reliability.
3. `Warning` — a constraint violation pattern.
4. `CausalLink` — a `A → B` causation claim.
5. `StrategyFragment` — a partial solution template.
6. `AntiKnowledge` — a known-bad approach to avoid.

Tests verify:
- Each knowledge type is stored and retrieved correctly.
- The type is preserved through serialization.
- Retrieval by type returns only that type.

### Validation Tiers (Transient → Working → Consolidated → Persistent)

Knowledge items progress through 4 tiers:
- `Transient`: just observed; not yet validated.
- `Working`: validated by at least one corroborating observation.
- `Consolidated`: validated by multiple observations; high confidence.
- `Persistent`: permanently stored; immune to decay.

Tests verify:
- An item promoted from Transient → Working gains the correct metadata.
- An item can only be promoted, never demoted.
- A Persistent item is not subject to GC regardless of decay score.

Key property: [../by-property/neuro-knowledge-tier-monotonicity.md](../by-property/neuro-knowledge-tier-monotonicity.md).

### HDC Similarity Search

- A query vector returns the top-K most similar stored vectors.
- Similarity scores are in [0.0, 1.0].
- A self-query returns similarity ≈ 1.0.
- An unrelated query returns similarity ≈ 0.5 (random baseline for 10,240-bit vectors).
- Search over 1,000 stored vectors completes in < 5ms (benchmark; see [../tiers/07-performance-tests.md](../tiers/07-performance-tests.md)).

---

## Property Tests

| Property | Test name |
|---|---|
| Knowledge tier monotonicity | `neuro_tier_only_increases` |
| HDC self-similarity | `hdc_self_similarity_near_one` |
| HDC random-similarity | `hdc_random_similarity_near_half` |
| Knowledge type preservation | `knowledge_type_preserved_through_tier` |

---

## Known Gaps

- `roko-neuro` is Built but not yet wired to the runtime, so integration tests are absent.
- No property tests for knowledge degradation under repeated contradictory observations.
- No tests for the knowledge consolidation process that promotes Transient → Working.

## See also

- [../by-property/neuro-knowledge-tier-monotonicity.md](../by-property/neuro-knowledge-tier-monotonicity.md)
- [../by-property/hdc-bundling-commutativity.md](../by-property/hdc-bundling-commutativity.md)
- [../gaps-and-roadmap.md](../gaps-and-roadmap.md) — neuro integration testing noted as a gap
