# Coverage Tooling

> `cargo-llvm-cov` setup, per-crate thresholds, and PR coverage reporting.

**Status**: Shipping
**Depends on**: [04-ci-integration.md](04-ci-integration.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Coverage is measured with `cargo-llvm-cov` (LLVM source-based coverage). The minimum threshold is 80% line coverage for every shipping crate. Coverage is reported per-crate on every PR. Scaffold and Deferred crates are excluded.

---

## Tool

```bash
# Install
cargo install cargo-llvm-cov

# Run coverage for the workspace
cargo llvm-cov nextest --workspace --lcov --output-path coverage.lcov

# Per-crate report
cargo llvm-cov nextest -p roko-core --html --output-dir coverage/

# Threshold check (fails if any crate < 80%)
cargo llvm-cov nextest --workspace --fail-under-lines 80
```

---

## Per-Crate Thresholds

| Crate | Required line coverage | Notes |
|---|---|---|
| `roko-core` | 90% | Highest bar; kernel correctness is critical |
| `roko-gate` | 85% | Gate semantics must be well-tested |
| `roko-fs` | 85% | Substrate correctness matters |
| `roko-agent` | 80% | Backend adapters have some untestable paths |
| `roko-orchestrator` | 80% | |
| `roko-learn` | 70% | Low count/LOC ratio noted as a gap |
| `roko-std` | 75% | Some tool paths are hard to test hermetically |
| `roko-compose` | 75% | |
| `roko-chain` | 80% | On-chain logic is critical |
| `roko-cli` | 70% | UI state machine partially tested |
| `roko-runtime` | 50% | Tested indirectly via integration tests |
| `roko-serve` | 60% | (gap — see gaps-and-roadmap) |

Crates at **Scaffold** or **Deferred** status are excluded from coverage requirements.

---

## Coverage Report in CI

On every PR, CI posts a coverage comment with:
- Overall workspace coverage.
- Per-crate coverage for any crate that changed.
- Diff coverage: lines added in this PR that are not covered.

A PR that drops coverage below the threshold for any shipping crate requires maintainer override.

---

## What Coverage Measures (and What It Doesn't)

Coverage measures whether a line of code was executed during tests. It does NOT measure:
- Whether the assertion on that line was meaningful.
- Whether all branches of a conditional were covered (use branch coverage for that).
- Whether property tests exercised interesting inputs (see [../tiers/03-property-tests.md](../tiers/03-property-tests.md)).

Coverage is a floor, not a goal. 80% line coverage with strong property tests is better than 100% line coverage with trivial unit tests.

---

## See also

- [04-ci-integration.md](04-ci-integration.md)
- [../quality-gates/02-pr-checks.md](../quality-gates/02-pr-checks.md)
