# Property-Based Tests

> Invariant-first testing using proptest: every property that "must always hold" is a named, shrinkable, corpus-backed test.

**Status**: Shipping
**Crate**: `proptest` (primary), `quickcheck` (legacy in `roko-core`)
**Depends on**: [../by-property/README.md](../by-property/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Property-based tests generate hundreds of random inputs per run, verify that invariants hold for all of them, and shrink any failing input to the minimal counterexample. Every property named in [by-property/](../by-property/README.md) has exactly one property test. Shrunk counterexamples are committed to `proptest-regressions/` and re-run on every subsequent CI pass.

---

## Framework: proptest

Roko uses `proptest` as the primary property testing framework. Key advantages over `quickcheck`:
- Structured shrinking: failing inputs are reduced to minimal cases that still trigger the failure.
- Corpus persistence: the `proptest-regressions/` directory stores shrunk failures for regression replay.
- Composable strategies: arbitrary complex types can be generated from smaller strategies.
- Deterministic seeding: `PROPTEST_SEED` env var fixes the seed for debugging.

`quickcheck` remains in `roko-core` for legacy tests that predate the proptest migration.

---

## Invariants Tested (Overview)

For the full catalog, see [../by-property/README.md](../by-property/README.md). Primary property families:

| Property family | Key invariants |
|---|---|
| Content-addressing | `hash(bytes) = hash(bytes)` always; hash is collision-resistant |
| Score arithmetic | Axis independence; normalization range [0,1]; aggregation monotonicity |
| Decay | Exponential/linear/step/none all reach or approach zero; decay is monotone non-increasing |
| Lineage | Parent chain is acyclic; depth is finite; root has no parent |
| Gate verdicts | Monotonicity with threshold; idempotence on identical input |
| Substrate operations | Write idempotence; read-after-write consistency; GC preserves living engrams |
| HDC fingerprints | Bundling commutativity; binding is bijective on disjoint bundles |
| Engram serialization | Round-trip identity; deserialization of any valid serialization succeeds |

---

## Writing a Property Test

```rust
use proptest::prelude::*;
use roko_core::{ContentHash, Engram};

// Strategy: generate arbitrary byte vectors
proptest! {
    /// content_hash_determinism: the same bytes always hash to the same ContentHash.
    #[test]
    fn content_hash_determinism(bytes in any::<Vec<u8>>()) {
        let h1 = ContentHash::from_bytes(&bytes);
        let h2 = ContentHash::from_bytes(&bytes);
        prop_assert_eq!(h1, h2,
            "ContentHash must be deterministic for the same input bytes");
    }
}

proptest! {
    /// score_axis_independence: mutating one axis must not change others.
    #[test]
    fn score_axis_independence(
        novelty in 0.0f32..=1.0,
        relevance in 0.0f32..=1.0,
    ) {
        let mut s = Score::default();
        s.set_novelty(novelty);
        s.set_relevance(relevance);
        prop_assert_eq!(s.novelty(), novelty);
        prop_assert_eq!(s.relevance(), relevance,
            "Setting novelty must not affect relevance");
    }
}
```

---

## Strategies

Common strategies are defined in `roko-test::strategies`:

| Strategy | Type generated |
|---|---|
| `arb_engram()` | `Engram` with valid field ranges |
| `arb_score()` | `Score` with axes in [0,1] |
| `arb_content_hash()` | `ContentHash` (from random bytes) |
| `arb_decay_params()` | Decay type + parameters in valid ranges |
| `arb_lineage_dag()` | DAG of `Engram`s with parent chains (no cycles by construction) |
| `arb_gate_input()` | Structurally valid `GateInput` for any gate |
| `arb_verdict_config()` | Gate config with valid threshold range |

See [../tools-and-harness/03-fixture-library.md](../tools-and-harness/03-fixture-library.md).

---

## Counterexample Handling

When a property test fails, `proptest` shrinks the failing input to the minimal counterexample and writes it to `proptest-regressions/<module_path>/<test_name>.txt`.

**What to do with a shrunk counterexample**:
1. Reproduce with `PROPTEST_SEED=<seed> cargo test <test_name> -- --nocapture`.
2. Identify which invariant was violated and file an issue.
3. Fix the implementation or the property statement (if the property was wrong).
4. Do not delete the regression file — it is now a regression test that prevents re-introduction of the bug.

**Committed regression files** in `proptest-regressions/` are re-run on every CI pass. They are the "long memory" of the property test suite.

---

## Property Test Configuration

In `Cargo.toml`:
```toml
[dev-dependencies]
proptest = "1"
proptest-derive = "0.4"

[profile.test]
opt-level = 1  # needed for proptest shrinking performance
```

In each module's `proptest!` block:
```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,                         // default 256 cases per property
        max_shrink_iters: 10_000,           // shrink up to 10K iterations
        source_file: Some("src/score.rs"),  // for regression file naming
        ..ProptestConfig::default()
    })]
    // …tests…
}
```

For CI, `PROPTEST_CASES=512` is set to run more cases than in developer builds.

---

## Running Property Tests

```bash
# Run all property tests
cargo test proptest

# Run a specific property
cargo test -p roko-core -- content_hash_determinism

# Replay a known regression
PROPTEST_SEED=1234567890 cargo test -p roko-core -- score_axis_independence

# Run with more cases
PROPTEST_CASES=1000 cargo test
```

---

## Property Test vs. Unit Test

| | Unit test | Property test |
|---|---|---|
| Inputs | Fixed, hand-crafted | Generated, hundreds of random cases |
| Failure message | "expected X, got Y" | "minimal failing input: …" |
| Regression artifact | None (implicit) | `proptest-regressions/*.txt` |
| Best for | Specific known cases, edge cases | Invariants over all valid inputs |
| Coverage | High on chosen cases | High over input space distribution |

Use both: property tests find what you didn't think of; unit tests assert what you did.

---

## Invariants

- Every property named in [../by-property/](../by-property/README.md) must have exactly one property test.
- Shrunk counterexamples in `proptest-regressions/` are never manually deleted without a maintainer review.
- Property tests must not depend on global mutable state.

---

## Open Questions

- Should the property corpus be shared across developers via a git-tracked `proptest-regressions/` directory or via CI artifacts?
- Is `proptest-derive` worth adding for complex types like `Engram` and `GateInput`?

## See also

- [../by-property/README.md](../by-property/README.md) — the invariant catalog
- [../tools-and-harness/03-fixture-library.md](../tools-and-harness/03-fixture-library.md) — proptest strategies
- [01-unit-tests.md](01-unit-tests.md) — the complement to property tests
