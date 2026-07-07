---
title: "Readiness Audit: Code Intelligence (§15)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-15
source: 31-implementation-readiness-audit.md (§15)
score: 24/30
tags: [code-intelligence, roko-index, symbol-graph, HDC, PageRank, built-unwired]
---

# Readiness Audit: Code Intelligence (§15)

**Score**: 24/30 | **Crate**: roko-index (Built/Unwired, 5 files, ~700 LOC, 32 tests) + 3 lang providers (~2,339 LOC, 92 tests)

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 5 | LanguageProvider trait clean and extensible |
| pseudocode | 5 | PageRank and HDC fingerprint algorithms specified |
| config_params | 3 | Core config present; advanced config absent |
| error_handling | 2 | **Functions return structs directly, no Result** |
| integration_wiring | 4 | Code exists; no consumer |
| test_criteria | 5 | 32 tests (roko-index) + 92 tests (lang providers) |

## The Paradox

**Only section with significant built and tested code** — yet it has no consumer anywhere in the system.

- `LanguageProvider` trait: clean and extensible
- HDC fingerprints: sub-microsecond similarity search
- PageRank: correct and converges
- 3 language providers (Rust, TypeScript, Go): fully tested

**Critical gap**: `roko-index` is a standalone library. Not called from `orchestrate.rs` or any `ContextProvider`. No `CodeIndex` trait. No search API. No error handling.

## Highest-Leverage Near-Term Opportunity

Wiring roko-index + lang providers into the context assembly pipeline would give agents code-aware context for free.

**Gap G2** (Tier 0): Wire roko-index into ContextProvider  
**Gap G3** (Tier 0): Register lang providers in `detect_polyglot`  
**Gap G20** (Tier 2): Create CodeIndex trait + search API

## Cross-References

- [../integration-map/code-intel-x-composition.md](../integration-map/code-intel-x-composition.md) — M8
- [../integration-map/code-intel-x-verification.md](../integration-map/code-intel-x-verification.md) — M16
