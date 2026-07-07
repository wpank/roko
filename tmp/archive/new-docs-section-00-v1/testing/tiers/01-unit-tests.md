# Unit Tests

> In-process, deterministic tests of a single crate's public API surface.

**Status**: Shipping
**Crate**: all shipping crates
**Depends on**: [../00-test-philosophy.md](../00-test-philosophy.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Unit tests are the base of the pyramid: fast (< 30s total), in-process, seeded-random, and limited to one crate's API surface. They use standard `#[test]` with no external frameworks except `proptest` for property suites. Every public function must have at least one unit test before merge.

---

## Scope

A unit test may:
- Construct any type exported from the crate under test.
- Call any public function or method.
- Use `tempfile` for filesystem fixtures.
- Use the `roko-test` harness for controlled time, seeded RNG, and intercepted I/O.

A unit test must not:
- Make real network calls.
- Depend on test execution order.
- Use `thread::sleep` for synchronisation.
- Access real LLM endpoints (use fixture tapes — see [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md)).

---

## Naming Conventions

Unit test functions follow: `<what>_<condition>_<expected_result>`.

Examples:
- `score_with_max_axes_saturates_to_one`
- `engram_duplicate_write_is_idempotent`
- `gate_below_threshold_returns_fail_verdict`
- `decay_exponential_reaches_zero_at_infinity`

Test modules are declared with `#[cfg(test)]` in the same file as the implementation (not in a separate `tests/` module), unless the test file exceeds 300 lines, in which case it moves to `tests/<module>_tests.rs`.

---

## Frameworks

| Framework | Usage |
|---|---|
| `#[test]` (stdlib) | Default for all unit tests |
| `proptest` | Property-based sub-suites within a unit test module |
| `assert_matches!` | Pattern-based assertions on `Result`/`Option` |
| `pretty_assertions` | Structured diff output on complex assertion failures |
| `rstest` | Parameterised test tables for gate threshold grids |

---

## Per-Crate Unit Test Counts

See [../by-subsystem/](../by-subsystem/README.md) for per-crate breakdowns. At the 2026-04-17 audit:

| Crate | Unit tests | Key focus |
|---|---|---|
| `roko-core` | 376 | Engram type, Score axes, ContentHash, decay math |
| `roko-agent` | 346 | LLM backends, CascadeRouter, safety pipeline |
| `roko-gate` | 200 | Gate verdicts, rung transitions, threshold logic |
| `roko-orchestrator` | 158 | Plan DAG scheduling, crash recovery |
| `roko-learn` | 101 | Bandit algorithms, episode logging, playbooks |
| `roko-std` | 96 | Built-in tools, MCP dispatch |
| `roko-chain` | 52 | On-chain type arithmetic, token math |
| `roko-cli` | 38 | PRD lifecycle, command dispatch |
| `roko-fs` | 37 | JSONL substrate, GC correctness |
| `roko-compose` | 23+ | SystemPromptBuilder layers, token budget |

---

## The `TestContext` Fixture

Every unit test that needs filesystem access, clocks, or RNG creates a `TestContext`:

```rust
#[test]
fn substrate_write_then_read_round_trips() {
    let ctx = TestContext::new();          // temp dir, seeded clock, seeded rng
    let substrate = ctx.file_substrate(); // JSONL substrate in temp dir
    let engram = ctx.engram_fixture();    // deterministic test engram

    substrate.write(&engram).unwrap();
    let retrieved = substrate.read(engram.id()).unwrap();

    assert_eq!(retrieved, engram);
}
```

See [../tools-and-harness/01-test-harness.md](../tools-and-harness/01-test-harness.md) for `TestContext` API.

---

## Running Unit Tests

```bash
# All unit tests across the workspace
cargo test

# Single crate
cargo test -p roko-core

# Specific test
cargo test -p roko-core -- score_with_max_axes_saturates_to_one

# With output (usually suppressed)
cargo test -p roko-core -- --nocapture
```

---

## Coverage Requirements

Line coverage ≥ 80% for every shipping crate. Coverage is measured with `cargo-llvm-cov` in CI. A PR that drops coverage below 80% for any shipping crate requires an explicit override comment from a maintainer.

See [../tools-and-harness/05-coverage-tooling.md](../tools-and-harness/05-coverage-tooling.md).

---

## Invariants

- Unit tests must be deterministic: same input → same output on every machine.
- Unit tests must not depend on wall-clock time; use the `TestContext` clock.
- Test names must be unique within a crate (enforced by `cargo test`).

---

## Open Questions

- Should `roko-runtime` unit tests be separated from integration tests, given it manages OS threads?
- Is `rstest` worth the dependency for parameterised gate threshold tables?

## See also

- [02-integration-tests.md](02-integration-tests.md) — multi-crate tests
- [03-property-tests.md](03-property-tests.md) — invariant-focused tests
- [../tools-and-harness/01-test-harness.md](../tools-and-harness/01-test-harness.md)
