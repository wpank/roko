# Testing

> Roko's testing infrastructure — philosophy, tier breakdown, per-subsystem coverage, per-property invariants, harness, and quality gates.

**Status**: Shipping
**Crate**: cross-crate
**Depends on**: [reference/11-crate-map.md](../reference/11-crate-map.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko has 3,761 test functions across 36 workspace members, organized into five tiers (unit, integration, property, regression, end-to-end), a custom `roko-test` harness that mocks LLM calls for deterministic replay, and four quality gates (pre-commit → PR → pre-release → post-deploy). The test philosophy treats gate failure as a verdict rather than an error, and property-based tests are first-class — every invariant that "must always hold" has its own named property test.

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [test-philosophy.md](00-test-philosophy.md) | What we test, why, what we don't | Shipping |
| 01 | [test-pyramid.md](01-test-pyramid.md) | Unit → integration → property → regression → E2E | Shipping |

### Sub-folders

| Folder | Contents |
|---|---|
| [tiers/](tiers/README.md) | One file per test tier: unit, integration, property, regression, E2E, fuzz, performance |
| [by-subsystem/](by-subsystem/README.md) | Per-crate test coverage with exact counts |
| [by-property/](by-property/README.md) | Invariant catalog — every property is a first-class entity |
| [tools-and-harness/](tools-and-harness/README.md) | Harness setup, mock LLMs, fixtures, CI, coverage, snapshots |
| [quality-gates/](quality-gates/README.md) | Pre-commit, PR checks, pre-release, post-deploy |
| [gaps-and-roadmap.md](gaps-and-roadmap.md) | Known gaps and future test work |

---

## Test Count Summary

| Crate | Tests | Tier |
|---|---|---|
| `roko-core` | 376 | unit + property |
| `roko-agent` | 346 | unit + integration |
| `roko-gate` | 200 | unit + integration |
| `roko-orchestrator` | 158 | unit + integration |
| `roko-learn` | 101 | unit |
| `roko-std` | 96 | unit |
| `roko-compose` | 23+ | unit |
| `roko-chain` | 52 | unit |
| `roko-fs` | 37 | unit |
| `roko-cli` | 38 | integration |
| other crates | ~2,334 | mixed |
| **Total** | **3,761** | — |

Source: implementation status audit, 2026-04-17.

---

## Suggested reading order

For a new contributor: [test-philosophy.md](00-test-philosophy.md) → [test-pyramid.md](01-test-pyramid.md) → [tiers/01-unit-tests.md](tiers/01-unit-tests.md) → your subsystem in [by-subsystem/](by-subsystem/README.md).

For an invariant author: [by-property/README.md](by-property/README.md) → relevant property file.

For CI/CD setup: [tools-and-harness/04-ci-integration.md](tools-and-harness/04-ci-integration.md) → [quality-gates/](quality-gates/README.md).

---

## See also

- [reference/11-crate-map.md](../reference/11-crate-map.md) — crate ownership
- [reference/12-design-principles.md](../reference/12-design-principles.md) — architectural principles that drive test design
- [operations/error-handling/](../operations/error-handling/) — failure mode taxonomy tested at each tier
