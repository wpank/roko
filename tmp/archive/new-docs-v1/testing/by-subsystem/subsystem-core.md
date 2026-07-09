# roko-core — Test Coverage

> 376 tests for the kernel: Engram type, Score axes, ContentHash, decay models, provenance, HDC fingerprints.

**Status**: Shipping
**Crate**: `roko-core`
**Section**: 00 — Architecture
**Depends on**: [../tiers/01-unit-tests.md](../tiers/01-unit-tests.md), [../tiers/03-property-tests.md](../tiers/03-property-tests.md)
**Last reviewed**: 2026-04-19

---

## Test Count: 376

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `engram` | ~80 | Construction, field access, metadata, serialization |
| `score` | ~70 | 7-axis arithmetic, normalization, aggregation |
| `content_hash` | ~40 | BLAKE3 determinism, equality, ordering |
| `decay` | ~50 | Exponential, linear, step, none variants; monotonicity |
| `provenance` | ~40 | Attestation chain, hash-chain integrity |
| `hdc` | ~50 | Hypervector bundling, binding, similarity |
| `kind` | ~20 | Kind enum discriminants, round-trip |
| `body` | ~26 | Body variant construction, serialization |

---

## Key Test Focus Areas

### Engram

- Construction with all field combinations.
- Content hash computed at construction time equals recomputed hash.
- Metadata fields (created_at, author, tags) round-trip through serialization.
- Parent chain (lineage) acyclicity is enforced.

### Score

- All 7 axes (novelty, relevance, confidence, valence, arousal, coherence, utility) are independent.
- Axis values are bounded to [0.0, 1.0]; out-of-range input is clamped or rejected.
- Weighted aggregation: `Score::aggregate` respects axis weights.
- Default score has all axes at 0.0.
- Score update does not mutate axes other than the updated one.

### ContentHash

- `ContentHash::from_bytes(b) == ContentHash::from_bytes(b)` for all `b` (determinism).
- `ContentHash::from_bytes(b) != ContentHash::from_bytes(c)` for `b ≠ c` (collision resistance, sampled).
- Hex encoding round-trips.
- Ordering is consistent with byte-lexicographic order of the raw hash.

### Decay

- `DecayExponential`: value at t=0 equals initial_value; value approaches 0 as t → ∞.
- `DecayLinear`: value reaches exactly 0 at `lifetime`.
- `DecayStep`: value drops to 0 at `step_time`, is unchanged before.
- `DecayNone`: value is always `initial_value` regardless of t.
- All variants: value at t=1 ≤ value at t=0 (monotone non-increasing).

### Provenance

- Attestation chain: each link's hash covers the previous link's content.
- Chain integrity: tampering with any link in the chain is detectable.
- Root attestation has no previous hash.

### HDC Fingerprints

- Bundling is commutative: `bundle(a, b) == bundle(b, a)`.
- Binding: `bind(a, b) ≠ a` and `bind(a, b) ≠ b` (binding changes the vector).
- Similarity: `cosine(a, a) ≈ 1.0`; `cosine(a, random) ≈ 0.5`.
- 10,240-bit vectors: length is always exactly 10,240 bits.

---

## Property Tests in roko-core

`roko-core` has the highest concentration of property tests in the codebase:

| Property | Test name |
|---|---|
| Content hash determinism | `content_hash_determinism` |
| Content hash equality ↔ byte equality | `content_hash_eq_iff_bytes_eq` |
| Score axis independence | `score_axis_independence` |
| Score normalization range | `score_axes_in_range` |
| Decay monotone non-increasing | `decay_monotone_nonincreasing` |
| Engram serialization round-trip | `engram_serde_roundtrip` |
| Lineage acyclicity | `lineage_is_acyclic` |
| HDC bundling commutativity | `hdc_bundle_commutative` |

See [../by-property/](../by-property/README.md) for full property definitions.

---

## Coverage Notes

- `roko-core` has the highest coverage of any crate: > 90% line coverage at last audit.
- The `hdc` module's 10,240-bit vector operations have full coverage of all vector operations.
- The `body` module's variant coverage is complete; all variant constructors are tested.

---

## Known Gaps

- No fuzz tests yet for `Engram` deserialization from malformed JSON (planned in [06-fuzz-tests.md](../tiers/06-fuzz-tests.md)).
- `Score::aggregate` with negative weights is not explicitly tested (would be an error path).

## See also

- [../by-property/content-addressing-determinism.md](../by-property/content-addressing-determinism.md)
- [../by-property/score-axis-independence.md](../by-property/score-axis-independence.md)
- [../by-property/lineage-acyclicity.md](../by-property/lineage-acyclicity.md)
- [../by-property/decay-monotonicity.md](../by-property/decay-monotonicity.md)
