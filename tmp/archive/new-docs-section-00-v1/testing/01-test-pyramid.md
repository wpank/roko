# Test Pyramid

> The five-tier structure of Roko's test suite: unit → integration → property → regression → end-to-end.

**Status**: Shipping
**Crate**: cross-crate
**Depends on**: [00-test-philosophy.md](00-test-philosophy.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's test suite has five primary tiers plus two supporting tiers (fuzz and performance). The pyramid is wide at the unit tier (fast, cheap, numerous) and narrow at the E2E tier (slow, expensive, few). Property tests occupy a separate axis — they are not in the pyramid proper but are the highest-value tests per line of code.

---

## The Pyramid

```
          ┌───────────────────────────┐
          │     End-to-End (E2E)      │  ~5 min/run · few · CI/main only
          │   full self-hosting loop  │
          └───────────────────────────┘
        ┌─────────────────────────────────┐
        │      Regression / Golden        │  ~30s · per-gate/per-subsystem
        │  verdict replay, golden diffs   │
        └─────────────────────────────────┘
      ┌───────────────────────────────────────┐
      │         Integration Tests             │  ~60s · cross-crate contracts
      │  multi-crate assembly, mocked LLMs    │
      └───────────────────────────────────────┘
    ┌─────────────────────────────────────────────┐
    │              Unit Tests                     │  ~10s · per-crate API surface
    │  deterministic, no I/O, in-process          │
    └─────────────────────────────────────────────┘

   ──────── orthogonal axis ─────────
   ┌──────────────────────────────────┐
   │       Property-Based Tests       │  proptest/quickcheck · invariant-first
   │ content-addressing, acyclicity,  │
   │ monotonicity, idempotence, …     │
   └──────────────────────────────────┘
```

---

## Tier 1 — Unit Tests

**Scope**: A single crate, a single module, or a single function.

**Constraints**: In-process only. No filesystem access outside of `tempfile` wrappers. No network. All randomness seeded.

**Speed target**: Full unit suite in < 30 seconds on a developer laptop.

**Count**: The majority of the 3,761 total tests.

**Typical pattern**:
```rust
#[test]
fn score_default_has_zero_axes() {
    let s = Score::default();
    assert_eq!(s.novelty(), 0.0);
    assert_eq!(s.relevance(), 0.0);
    // …
}
```

See [tiers/01-unit-tests.md](tiers/01-unit-tests.md) for scope, naming conventions, and crate-level organisation.

---

## Tier 2 — Integration Tests

**Scope**: Two or more crates assembled together, exercising cross-crate contracts.

**Constraints**: Allowed to touch the filesystem in a per-test temp directory (`TestContext`). LLM calls intercepted and replayed from fixture tapes. No real network.

**Speed target**: < 5 minutes for the full integration suite.

**Location**: `tests/` directories at the crate root (Cargo convention), or in dedicated integration harness crates.

**Typical pattern**: orchestrator dispatching a plan step that touches `roko-agent`, `roko-gate`, and `roko-fs` simultaneously.

See [tiers/02-integration-tests.md](tiers/02-integration-tests.md).

---

## Tier 3 — Property-Based Tests

**Scope**: Cross-cut. Any invariant that must hold for all valid inputs.

**Framework**: `proptest` (primary), `quickcheck` (legacy).

**Strategy**: each property in [by-property/](by-property/) has a corresponding `proptest!` block that generates hundreds of random inputs per run, shrinks counterexamples to minimal failing cases, and persists the shrunk corpus.

**Counterexample handling**: failing inputs are written to `proptest-regressions/` directories and replayed on every subsequent run. See [tiers/03-property-tests.md](tiers/03-property-tests.md).

---

## Tier 4 — Regression Tests

**Scope**: Per gate, per golden file, per known-bad input.

**Purpose**: prevent regressions in verdict logic, output format, and subsystem behaviour that are difficult to express as pure assertions.

**Golden files**: stored in `tests/golden/`, diffed on every run. A PR that changes a golden file must include a justification comment.

**Gate verdicts**: each of the 11 gates has a regression fixture set that captures expected `Verdict` values for canonical inputs. The Gate 7-rung pipeline has separate regression tests for each rung transition.

See [tiers/04-regression-tests.md](tiers/04-regression-tests.md).

---

## Tier 5 — End-to-End Tests

**Scope**: The full `roko prd → plan run → gate → persist → resume` loop.

**Environment**: hermetic (no live LLM, no internet, tempfs substrate).

**Speed**: ~5 minutes. Runs in CI on merges to `main` only.

**Coverage**: tests the self-hosting loop with a seed PRD, verifies that the resulting engrams are content-addressed correctly, that the gate pipeline runs to completion, and that the substrate reflects the expected state.

See [tiers/05-end-to-end-tests.md](tiers/05-end-to-end-tests.md).

---

## Supporting Tier — Fuzz Tests

Fuzz tests run against parser and serialization boundaries using `cargo-fuzz` + libFuzzer. Primary targets: Engram deserialization, gate input parsing, and plan DAG parsing.

See [tiers/06-fuzz-tests.md](tiers/06-fuzz-tests.md).

---

## Supporting Tier — Performance Tests

Benchmark harness using `criterion.rs`. Tracks hot-path latencies (scoring, gate pipeline, HDC similarity search) with CI flakiness control via statistical thresholds.

See [tiers/07-performance-tests.md](tiers/07-performance-tests.md).

---

## Tier Execution Order (CI)

```
pre-commit: unit tests → property tests (fast subset)
PR:         unit → property (full) → integration → regression
main merge: all of the above + E2E
pre-release: all + performance benchmarks
```

---

## See also

- [tiers/](tiers/README.md) — full detail for each tier
- [quality-gates/](quality-gates/README.md) — which tier runs at which gate
- [tools-and-harness/01-test-harness.md](tools-and-harness/01-test-harness.md) — the roko-test harness

## Open Questions

- Should fuzz corpora be committed to the repo or managed as CI artifacts?
- At what test count should the unit tier be split into fast-unit vs. slow-unit for pre-commit speed?
