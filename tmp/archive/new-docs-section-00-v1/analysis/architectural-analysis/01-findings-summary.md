# Findings Summary

> All 11 architectural findings in one table, with severity, status, and links.

**Status**: Analysis
**Crate**: —
**Last reviewed**: 2026-04-13

---

## Finding Inventory

| # | Finding | Severity | Type | File |
|---|---|---|---|---|
| F1 | Six Synapse traits are sufficient — no 7th trait needed | Informational | Validation | [02-finding-trait-sufficiency.md](02-finding-trait-sufficiency.md) |
| F2 | Trait merge candidates all fail — merges would break semantic clarity | Informational | Validation | [02-finding-trait-sufficiency.md](02-finding-trait-sufficiency.md) |
| F3 | **One dependency violation**: `roko-conductor` → `roko-learn` (L3→L2) | **Medium** | Bug | [03-finding-layer-taxonomy.md](03-finding-layer-taxonomy.md) |
| F4 | Six crates unclassified in the five-layer taxonomy | Low | Gap | [03-finding-layer-taxonomy.md](03-finding-layer-taxonomy.md) |
| F5 | `roko-fs` mislabeled as L3 Harness (should be L0 Runtime) | Low | Doc bug | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |
| F6 | Three cognitive speeds map cleanly to all four domains | Informational | Validation | [04-finding-cognitive-speeds.md](04-finding-cognitive-speeds.md) |
| F7 | Delta speed is a genuine innovation — no classical architecture equivalent | Informational | Finding | [04-finding-cognitive-speeds.md](04-finding-cognitive-speeds.md) |
| F8 | Engram/Signal is universal — edge cases handled by existing mechanisms | Informational | Validation | [05-finding-engram-universality.md](05-finding-engram-universality.md) |
| F9 | Engram is strictly richer than the Agent Data Protocol (ADP) | Informational | Finding | [05-finding-engram-universality.md](05-finding-engram-universality.md) |
| F10 | Cross-cut isolation has two gaps: Daimon not injected via trait object; Dreams imports directly | **Medium** | Bug | [06-finding-crosscut-isolation.md](06-finding-crosscut-isolation.md) |
| F11 | Cross-cut arbitration protocol not yet implemented | **Medium** | Gap | [06-finding-crosscut-isolation.md](06-finding-crosscut-isolation.md) |

---

## Documentation Inconsistencies

| # | Location | Issue | Severity | File |
|---|---|---|---|---|
| D1 | STATUS.md vs. 15-crate-map.md | STATUS says "18+ crates"; crate map shows 28 | Low | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |
| D2 | 12-five-layer-taxonomy.md | `roko-fs` listed as L3 Harness, should be L0 Runtime | Low | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |
| D3 | 06-synapse-traits.md | Says "4 Substrate implementations" — 2 are shipped, 2 spec'd | Low | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |
| D4 | TUI status | STATUS.md says "Scaffold"; QUICKSTART.md shows `roko dashboard` as working | Low | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |
| D5 | 02-engram-data-type.md | References "7-axis appraisal"; code has 4 axes | **Medium** | [09-finding-inconsistencies.md](09-finding-inconsistencies.md) |

---

## Code-Documentation Mismatches

| # | Aspect | Documentation | Code | Impact |
|---|---|---|---|---|
| M1 | Data type name | "Engram" | `Signal` | None (documented in glossary) |
| M2 | Score axes | 7 (4 stable + 3 extended) | 4 (confidence, novelty, utility, reputation) | Medium — docs overpromise |
| M3 | Attestation field | Specified in Engram docs | Not in Signal struct | Low — Phase 2+ feature |
| M4 | Conductor layer | Documented as L3 or L4 | Depends on roko-learn (L2) | Medium — layer violation |

---

## Prioritized Action List

| Priority | Action | Finding | Effort |
|---|---|---|---|
| **High** | Fix `roko-conductor` → `roko-learn` dependency violation | F3 | Small |
| **High** | Classify 6 unclassified crates in taxonomy | F4 | Small |
| **High** | Fix `roko-fs` layer assignment in docs (L3→L0) | F5/D2 | Trivial |
| **High** | Align Score documentation (7-axis → 4-axis current, 7-axis planned) | D5/M2 | Small |
| **Medium** | Implement gradient Gate feedback | [08-novel-proposals.md](08-novel-proposals.md) | Medium |
| **Medium** | Define `AffectModel` trait in `roko-core` for Daimon injection | F10 | Small |
| **Medium** | Formalize Pipeline as composable unit | [08-novel-proposals.md](08-novel-proposals.md) | Medium |
| **Medium** | Implement cross-cut arbitration protocol | F11 | Medium |
| Low | CompetitiveRouter (LIDA-inspired) | [08-novel-proposals.md](08-novel-proposals.md) | Large |
| Low | VSA/HDC operations on Signal struct | [08-novel-proposals.md](08-novel-proposals.md) | Large |
| Low | Formal category theory verification of pipeline laws | [07-finding-category-theory.md](07-finding-category-theory.md) | Large |

---

## Open Questions

- Is the cross-cut arbitration protocol (VCG tiebreaker) intended for Phase 1 or Phase 2?
- Should the gradient Gate feedback be added to the standard `loop_tick` or as an opt-in enhancement?
