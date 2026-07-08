# Fixture Library

> Shared test fixtures: proptest strategies, Engram factories, gate input builders, and plan factories.

**Status**: Shipping
**Crate**: `roko-test`
**Depends on**: [01-test-harness.md](01-test-harness.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko-test` provides a centralized fixture library so tests across all crates use the same realistic, well-defined test data. Fixtures are either deterministic factory functions or proptest strategies.

---

## Proptest Strategies

All strategies are in `roko_test::strategies`. Import them in test modules:

```rust
use roko_test::strategies::*;
```

| Strategy | Return type | Description |
|---|---|---|
| `arb_engram()` | `Engram` | Valid Engram with all fields in legal ranges |
| `arb_engram_with_parent()` | `Engram` | Engram with a valid (non-cyclic) parent hash |
| `arb_score()` | `Score` | Score with all 7 axes in valid ranges |
| `arb_content_hash()` | `ContentHash` | ContentHash from random bytes |
| `arb_decay_params()` | `Decay` | One of the 4 decay variants, randomly |
| `arb_lineage_dag(max_size: usize)` | `Vec<Engram>` | DAG of Engrams (acyclic by construction) |
| `arb_gate_input()` | `GateInput` | Structurally valid GateInput for any gate |
| `arb_verdict_config()` | `VerdictConfig` | Gate config with valid threshold |
| `arb_plan_dag(max_tasks: usize)` | `PlanDag` | Valid plan DAG (acyclic by construction) |
| `arb_task_context()` | `TaskContext` | Task context with random type and metadata |
| `arb_hdc_vector()` | `HdcVector` | Random 10,240-bit hypervector |
| `arb_address()` | `Address` | Random 20-byte Ethereum-style address |
| `arb_agent_output()` | `AgentOutput` | Agent output with random content |
| `arb_event_log(n: usize)` | `EventLog` | Valid n-event event log |
| `arb_tier()` | `KnowledgeTier` | One of the 4 knowledge tiers |
| `arb_axis_with_positive_weight()` | `ScoreAxis` | Any of the 7 Score axes (for aggregation tests) |
| `arb_verdict()` | `Verdict` | Pass or Fail verdict with random metrics |

---

## Factory Functions

For non-proptest tests that need specific fixtures:

```rust
use roko_test::fixtures::*;

// Engram factories
let e = Engram::fixture_minimal();           // minimal valid Engram
let e = Engram::fixture_with_body(Body::Text("hello"));
let e = Engram::fixture_with_score(Score::max()); // all axes at max
let e = Engram::fixture_expired();           // decay score at 0

// Gate input factories
let i = GateInput::fixture_clean_rust();     // valid Rust project
let i = GateInput::fixture_syntax_error();  // Rust with syntax error
let i = GateInput::fixture_test_failure();  // project with failing test

// Plan factories
let p = PlanDag::fixture_linear(n: 5);       // 5 tasks in sequence
let p = PlanDag::fixture_parallel(n: 5);     // 5 independent tasks
let p = PlanDag::fixture_diamond();          // A → {B, C} → D
```

---

## Fixture Files on Disk

For tests that need real files (Rust projects, JSON, etc.):

```
tests/fixtures/
  rust_projects/
    clean_project/      clean Cargo project (passes compile+test)
    syntax_error/       Cargo project with syntax error
    test_failure/       Cargo project with failing #[test]
  plans/
    simple_plan.json    3-task plan DAG
    complex_plan.json   10-task plan with branching
  prds/
    seed_prd.md         minimal PRD for E2E tests
  e2e_fixtures/         (see 05-end-to-end-tests.md)
```

---

## Adding New Fixtures

1. For a new proptest strategy: add to `roko-test/src/strategies.rs`.
2. For a new factory function: add to `roko-test/src/fixtures.rs`.
3. For a new fixture file: add to `roko-test/tests/fixtures/` and reference in `FIXTURES.md`.

---

## See also

- [01-test-harness.md](01-test-harness.md) — TestContext that uses these fixtures
- [../tiers/03-property-tests.md](../tiers/03-property-tests.md) — proptest usage
