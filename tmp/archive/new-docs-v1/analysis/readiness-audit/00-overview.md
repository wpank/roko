---
title: "Readiness Audit: Overview"
section: analysis
subsection: readiness-audit
id: ra-00
source: 31-implementation-readiness-audit.md (§Methodology, §Section Scorecard, §Crate Status)
tags: [overview, scorecard, methodology, crate-status]
---

# Readiness Audit: Overview

> **Generated**: 2026-04-13  
> **Coverage**: 21 doc sections (excluding §21-references), 350+ files, 18 crates (~177K LOC, ~3,391 tests)

## Scoring Methodology

Every markdown file in each section was read and scored against 6 criteria (0 = absent, 5 = exemplary):

| Criterion | What It Measures |
|---|---|
| **rust_structs** | Quality/completeness of struct, trait, enum definitions with field-level types |
| **pseudocode** | Algorithm pseudocode, Rust code blocks, step-by-step decision logic |
| **config_params** | Configuration parameters with defaults, ranges, rationale |
| **error_handling** | Error types, recovery paths, failure mode specification |
| **integration_wiring** | How components connect to other crates and the CLI entry point |
| **test_criteria** | Observable test conditions, acceptance thresholds, named test cases |

File status classifications:
- **Specified** — Has concrete Rust types, real constants, wiring details; directly implementable
- **Scaffold** — Has design intent, partial types, or pseudocode but gaps remain
- **Concept only** — Primarily theoretical, no concrete implementation material
- **Built** — Code exists in crates and tests pass
- **Wired** — Code exists AND is called from the CLI/orchestration loop

---

## Section Scorecard

| # | Section | Structs | Pseudo | Config | Errors | Wiring | Tests | Total | Crate Status |
|---|---|---|---|---|---|---|---|---|---|
| 00 | Architecture | 4 | 4 | 4 | 3 | 3 | 3 | **21/30** | roko-core: Stable |
| 01 | Orchestration | 5 | 5 | 5 | 5 | 5 | 5 | **30/30** | roko-orchestrator: Wired |
| 02 | Agents | 4 | 4 | 4 | 3 | 3 | 3 | **21/30** | roko-agent: Stable/Wired |
| 03 | Composition | 4 | 5 | 5 | 3 | 4 | 4 | **25/30** | roko-compose: Wired |
| 04 | Verification | 5 | 4 | 5 | 4 | 5 | 4 | **27/30** | roko-gate: Wired |
| 05 | Learning | 5 | 5 | 5 | 4 | 5 | 5 | **29/30** | roko-learn: Wired |
| 06 | Neuro | 4 | 4 | 5 | 3 | 3 | 3 | **22/30** | (in roko-core/roko-golem) |
| 07 | Conductor | 5 | 5 | 5 | 5 | 5 | 4 | **29/30** | roko-conductor: Wired |
| 08 | Chain | 4 | 3 | 3 | 2 | 3 | 3 | **18/30** | roko-chain: Scaffold |
| 09 | Daimon | 5 | 4 | 4 | 3 | 3 | 4 | **23/30** | (in roko-golem) |
| 10 | Dreams | 4 | 4 | 5 | 2 | 4 | 4 | **23/30** | (in roko-golem) |
| 11 | Safety | 5 | 4 | 5 | 4 | 5 | 4 | **27/30** | (in roko-orchestrator) |
| 12 | Interfaces | 4 | 4 | 5 | 3 | 4 | 3 | **23/30** | roko-cli: Wired (partial) |
| 13 | Coordination | 5 | 5 | 5 | 3 | 5 | 4 | **27/30** | 0% implemented |
| 14 | Identity/Economy | 5 | 5 | 4 | 3 | 4 | 4 | **25/30** | 0% implemented |
| 15 | Code Intelligence | 5 | 5 | 3 | 2 | 4 | 5 | **24/30** | roko-index: Built/Unwired |
| 16 | Heartbeat | 5 | 5 | 5 | 4 | 5 | 5 | **29/30** | 0% implemented |
| 17 | Lifecycle | 5 | 5 | 5 | 5 | 4 | 5 | **29/30** | Partial |
| 18 | Tools | 5 | 5 | 5 | 4 | 4 | 5 | **28/30** | roko-std: Stable |
| 19 | Deployment | 4 | 5 | 5 | 4 | 3 | 4 | **25/30** | Native only |
| 20 | Tech Analysis | 5 | 5 | 4 | 3 | 2 | 5 | **24/30** | 0% implemented |

---

## Criterion Averages

| Criterion | Mean | Min | Sections at Min |
|---|---|---|---|
| rust_structs | 4.6 | 4 | 00, 02, 03, 06, 08, 10, 12, 19 |
| pseudocode | 4.5 | 3 | 08 |
| config_params | 4.6 | 3 | 08, 15 |
| **error_handling** | **3.4** | **2** | **08, 10, 15** |
| integration_wiring | 3.9 | 2 | 20 |
| test_criteria | 4.1 | 3 | 00, 02, 06, 08, 12 |

**Universal weakness**: error_handling (mean 3.4/5). The codebase consistently specifies the happy path with mathematical precision but under-specifies failure modes. Only sections 01, 07, 17 score 5/5 on errors.

---

## Crate Implementation Status

| Crate | Files | LOC | Tests | CLI Wired? | Maturity |
|---|---|---|---|---|---|
| roko-core | 59 | ~6,500 | 610 | Yes (kernel) | **Stable** |
| roko-agent | 97 | ~9,500 | 567 | Yes | **Stable/Wired** |
| roko-orchestrator | 23 | ~3,000 | 315 | Yes | **Wired** |
| roko-gate | 22 | ~2,800 | 216 | Yes | **Wired** |
| roko-compose | 39 | ~4,500 | 264 | Yes | **Wired** |
| roko-conductor | 19 | ~2,200 | ~130 | Yes | **Wired** |
| roko-learn | 36 | ~5,000 | 348 | Yes | **Wired** |
| roko-cli | 101 | ~12,000 | ~300 | Entry point | **Stable/Wired** |
| roko-fs | 12 | ~1,800 | ~60 | Yes | **Stable** |
| roko-std | 33 | ~3,500 | ~120 | Yes | **Stable** |
| bardo-runtime | 6 | ~900 | ~12 | Yes | **Stable** |
| bardo-primitives | 3 | ~500 | 18 | Yes | **Stable** |
| roko-index | 5 | ~700 | 32 | **No** | Built/Unwired |
| roko-lang-rust | 1 | ~820 | 37 | **No** | Built/Unwired |
| roko-lang-typescript | 1 | ~918 | 31 | **No** | Built/Unwired |
| roko-lang-go | 1 | ~601 | 24 | **No** | Built/Unwired |
| roko-golem | 7 | ~600 | 3 | **No** | Scaffold |
| roko-chain | 10 | ~1,200 | ~10 | **No** | Scaffold |

**Key finding**: 12 of 18 crates are Stable/Wired. The remaining 6:
- **Built/Unwired** (roko-index + 3 lang providers): Complete, tested code with no consumer
- **Scaffold** (roko-golem, roko-chain): Phase 2+ placeholder code

---

## Cross-References

- [01-audit-summary.md](./01-audit-summary.md) — Systemic strengths and weaknesses
- [99-next-actions.md](./99-next-actions.md) — All gaps G1-G33
- [../integration-map/00-overview.md](../integration-map/00-overview.md) — The 20 missing integrations
