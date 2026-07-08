# Test Philosophy

> What Roko tests, why it tests that way, and what it deliberately leaves to higher-level verification.

**Status**: Shipping
**Crate**: cross-crate
**Depends on**: [reference/12-design-principles.md](../reference/12-design-principles.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko tests behaviour and invariants, not implementation details. Every property that "must always hold" is a named test. Every gate in the 11-gate verification pipeline has its own tests. Every LLM call in tests is mocked or replayed. The central principle inherited from the gate design applies equally to tests: **failure is a verdict, not an error** — a test failure communicates a constraint violation, not an unexpected crash.

---

## What We Test

### 1. Architectural invariants (property-based)

Any property stated in the architecture as "must always hold" is tested using `proptest` or `quickcheck`. These are the highest-value tests in the suite because they verify design decisions, not implementations. They are collected in [by-property/](by-property/README.md).

Examples:
- Content-addressing determinism: the same bytes always produce the same `ContentHash`.
- Score axis independence: mutating one axis must not change other axis values.
- Lineage acyclicity: an Engram's `parent` chain must be a DAG, never a cycle.
- Gate verdict monotonicity: a gate that passes at threshold `t` must also pass at any threshold `t' < t`.
- Substrate idempotence: writing the same Engram twice must leave the substrate in the same state as writing it once.

### 2. Subsystem contracts (unit tests)

Each crate has unit tests that exercise its public API surface. Unit tests are deterministic, in-process, and run without network or filesystem access (the test harness intercepts all I/O).

### 3. Cross-crate interactions (integration tests)

Integration tests assemble two or more crates and verify that their contracts compose correctly. LLM calls are intercepted and replayed from fixture recordings. Integration tests are allowed to touch the filesystem under a per-test temp directory.

### 4. Gate-level verification (regression tests)

For each of the 11 gates, regression tests capture the verdict returned on known inputs and alert if the verdict changes. Golden files hold expected verdicts; the diff is a first-class artifact reviewed on PRs.

### 5. Self-hosting loop (end-to-end tests)

The full `roko prd → plan run → gate → persist → resume` loop is tested end-to-end using a seed PRD and a hermetic environment (no live LLM calls, no internet). These tests are slow (~5 min) and run only in CI on `main`.

---

## What We Don't Test (and Why)

| Not tested | Reason |
|---|---|
| LLM output quality | Non-deterministic; tested via benchmarks in [operations/performance/](../operations/performance/), not assertions. |
| Network I/O paths live | All network calls are mocked. Live integration is done in staging, not in the test suite. |
| Chain settlement | `roko-chain` tests the on-chain logic in isolation; settlement on a live network is a deploy concern. |
| Decay arithmetic over real time | Decay rates are tested with synthetic clocks; real-time decay is tested manually in staging. |
| UI rendering fidelity | The ratatui TUI is tested for state machine transitions, not pixel output. |

---

## Test Design Principles

1. **One property, one test name.** If a property appears twice in the suite under different names, one of them is wrong. Use [by-property/](by-property/README.md) as the canonical registry.

2. **Tests assert, they do not print.** Debug logging is for production observability. A test that passes but prints a warning is a disguised failure.

3. **Determinism is non-negotiable.** Any test that flakes because of timing, random seeds, or file ordering is a P0 bug. The test harness controls all sources of non-determinism.

4. **Test the contract, not the implementation.** Changing a private struct field must not break tests. If it does, the test was testing implementation, not behaviour.

5. **Gate failure is a verdict.** When a test fails, the failure message identifies *which invariant* was violated, not merely *what crashed*. Error messages are first-class citizens.

6. **Coverage is a floor, not a goal.** Line coverage ≥ 80% for every shipping crate is a minimum sanity check; it does not substitute for property tests on the invariants that matter.

---

## The Relationship Between Tests and Gates

The 11-gate verification pipeline (see [by-subsystem/subsystem-gate.md](by-subsystem/subsystem-gate.md)) is itself a test consumer and test producer:

- **Consumer**: Gate 3 (the Test gate) runs `cargo test` as a gate verdict step in the agentic loop.
- **Producer**: Gate 5 (GeneratedTest gate) produces new test cases for agent-generated code, contributing to the test suite growth.
- **Subject**: The gate implementation itself has 200 tests that verify gate semantics.

---

## Philosophy Summary

Roko's testing doctrine can be stated in four sentences:

1. Invariants are first-class documentation — they live in [by-property/](by-property/README.md) alongside the code they constrain.
2. Every subsystem that ships must have tests before the first merge; test debt is technical debt.
3. Non-determinism in tests is a defect in the test infrastructure, not an acceptable tradeoff.
4. The test suite is part of the product: agents use it for self-assessment, and failures drive the learning loop.

---

## Open Questions

- Should the E2E self-hosting tests be promoted from CI-only to pre-commit with caching?
- How should the property test counterexample database (shrunk failing inputs) be stored and shared across developers?

## See also

- [01-test-pyramid.md](01-test-pyramid.md) — structural overview of test tiers
- [by-property/README.md](by-property/README.md) — invariant catalog
- [tools-and-harness/02-mock-llms.md](tools-and-harness/02-mock-llms.md) — deterministic LLM replay
