# Test Tiers

> One file per test tier. Each tier has a distinct scope, speed target, and quality contract.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Speed | Count |
|---|---|---|---|---|
| 01 | [unit-tests.md](01-unit-tests.md) | Scope, conventions, frameworks, naming | ~30s total | ~2,800 |
| 02 | [integration-tests.md](02-integration-tests.md) | Cross-crate contracts, TestContext | ~5 min | ~400 |
| 03 | [property-tests.md](03-property-tests.md) | proptest invariants, shrinking, corpus | ~90s | ~200 |
| 04 | [regression-tests.md](04-regression-tests.md) | Golden files, verdict replay, diffing | ~30s | ~100 |
| 05 | [end-to-end-tests.md](05-end-to-end-tests.md) | Full self-hosting loop | ~5 min | ~20 |
| 06 | [fuzz-tests.md](06-fuzz-tests.md) | cargo-fuzz targets, corpus management | continuous | N/A |
| 07 | [performance-tests.md](07-performance-tests.md) | criterion benchmarks, flakiness control | ~10 min | ~50 |

Note: counts are approximate and evolve with the codebase.

---

## Suggested reading order

For a contributor adding a new feature: 01 → 02 → 03.
For a contributor adding a new gate: 02 → 04.
For a performance investigation: 07.
For a security audit: 06.

## See also

- [../01-test-pyramid.md](../01-test-pyramid.md) — structural overview
- [../quality-gates/README.md](../quality-gates/README.md) — which tiers run at which quality gate
