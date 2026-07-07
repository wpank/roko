# Tools and Harness

> The test infrastructure: `roko-test` harness, mock LLMs, fixture library, CI integration, coverage, and snapshot testing.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers |
|---|---|---|
| 01 | [test-harness.md](01-test-harness.md) | `roko-test` harness: TestContext, IntegrationContext, seeded clock, seeded RNG |
| 02 | [mock-llms.md](02-mock-llms.md) | How LLM calls are intercepted and replayed from tape files |
| 03 | [fixture-library.md](03-fixture-library.md) | Shared test fixtures: proptest strategies, Engram factories, gate input builders |
| 04 | [ci-integration.md](04-ci-integration.md) | How tests run in CI: pipeline, caching, parallelism, flakiness policy |
| 05 | [coverage-tooling.md](05-coverage-tooling.md) | `cargo-llvm-cov` setup, thresholds, reporting |
| 06 | [snapshot-testing.md](06-snapshot-testing.md) | Golden file infrastructure, `assert_golden!` macro, update workflow |

---

## See also

- [../tiers/](../tiers/README.md) — tiers that use this harness
- [../quality-gates/04-ci-integration.md](../quality-gates/04-ci-integration.md) — CI gates
