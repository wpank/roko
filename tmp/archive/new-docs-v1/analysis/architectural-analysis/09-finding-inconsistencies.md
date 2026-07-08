---
title: "Finding: Documentation and Code-Doc Inconsistencies"
section: analysis
subsection: architectural-analysis
id: aa-09
source: 23-architectural-analysis-improvements.md (§9)
tags: [inconsistencies, documentation, code-doc-mismatch, roko-fs, score-axes, signal-engram]
---

# Finding: Documentation and Code-Doc Inconsistencies

> **Severity classification**: Medium (functional mismatch) and Trivial (documentation-only). None are blocking for current operation but all create confusion during onboarding and architectural reasoning.

## Context

Source file 23 (§9) identifies 5 documentation inconsistencies and 4 code-documentation mismatches found during the analysis. These were identified by cross-reading all 24 architecture docs, all 22 section INDEX files, STATUS/QUICKSTART/COMPARISON, and actual Cargo.toml dependency graphs.

---

## Part 1: Documentation Inconsistencies (5 items)

### DI-1: Crate Count Mismatch

| Location | Claim |
|---|---|
| `STATUS.md` | "18+ Rust crates" |
| `15-crate-map.md` | 28 crates listed |

**Resolution**: Both are correct but incomplete. The crate map includes 18 primary crates plus MCP/demo/utility crates. STATUS.md should be clarified to "18 primary crates (28 total including MCP, demo, and utility crates)."

**Impact**: Low. Creates confusion when developers count crates in Cargo.toml.

---

### DI-2: roko-fs Layer Assignment

| Location | Claim |
|---|---|
| `12-five-layer-taxonomy.md` (§5, line 116) | Lists `roko-fs` as **L3 Harness** |
| Actual role | `roko-fs` implements `FileSubstrate`, a `Substrate` trait impl |

**Root cause**: `Substrate` is assigned to L0 Runtime. Therefore `roko-fs` (which implements `FileSubstrate`) should be L0 Runtime, not L3 Harness.

**Resolution**: Move `roko-fs` to L0 Runtime in the five-layer taxonomy documentation. Its sole purpose is persistent storage of Engrams — the canonical L0 responsibility.

**Impact**: Medium. The layer taxonomy is the canonical reference for architectural decisions. Having `roko-fs` in the wrong layer misleads any developer reasoning about layer boundaries.

**See**: Finding [AA-03: Layer Taxonomy](./03-finding-layer-taxonomy.md), Improvement [AA-10](./10-prioritized-improvements.md) #3 (Trivial effort, High priority).

---

### DI-3: Substrate Implementation Count

| Location | Claim |
|---|---|
| `06-synapse-traits.md` | "4 Substrate implementations" |
| Actual shipped code | 2 implementations (MemorySubstrate, FileSubstrate) |

**Context**: The doc counts 4 implementations: MemorySubstrate, FileSubstrate, HdcSubstrate, ChainSubstrate. However HdcSubstrate and ChainSubstrate are specified but not shipped.

**Resolution**: Clarify as "2 shipped (Memory, File); 2 planned (HDC, Chain)."

**Impact**: Low. Could cause a developer to search for code that doesn't exist.

---

### DI-4: TUI Status Ambiguity

| Location | Claim |
|---|---|
| `STATUS.md` | TUI status: "Scaffold" |
| `QUICKSTART.md` | Shows `roko dashboard` as a working command |

**Resolution**: Both are correct but confusing. The `roko dashboard` command exists and outputs text; "Scaffold" means no interactive ratatui UI yet (no real-time panels, no keyboard navigation). Should be clarified as: "CLI dashboard command: Working (text output). Interactive ratatui TUI: Scaffold."

**Impact**: Low. Confusion for new contributors evaluating the TUI.

---

### DI-5: Score Axes — 7-Axis Appraisal vs 4-Axis Code

| Location | Claim |
|---|---|
| `02-engram-data-type.md` | References "7-axis appraisal" |
| `roko-core` code (`Score` struct) | 4 axes: `confidence`, `novelty`, `utility`, `reputation` |

**Root cause**: The architecture originally specified 7 axes (4 stable + 3 extended). The 3 extended axes were not implemented.

**Resolution**: Documentation should explicitly distinguish:
- **Current (4 axes)**: confidence, novelty, utility, reputation — stable and implemented
- **Planned (3 additional axes)**: specificity, urgency, empathy — Phase 2+ extension

**Impact**: Medium. Documentation overpromises on a core type. Any developer reading the 7-axis spec and looking at the `Score` struct will be confused. See finding [AA-10](./10-prioritized-improvements.md) #4 (Small effort, High priority).

---

## Part 2: Code-Documentation Mismatches (4 items)

### CM-1: Data Type Name — Engram vs Signal

| Documentation | Code |
|---|---|
| "Engram" (canonical architecture term) | `Signal` (Rust struct name) |

**Impact**: **None** — the divergence is explicitly documented in `01-naming-and-glossary.md`. This is an intentional naming divergence, not an inconsistency.

**Note**: The rename from `Signal` to `Engram` is planned (see Readiness Audit, Gap G8). Until it executes, both names refer to the same struct. When reading code, `Signal` = Engram.

**See**: Readiness Audit finding [RA-01: Architecture](../readiness-audit/subsystem-architecture.md): "Signal→Engram rename (Tier 0D) documented but unexecuted."

---

### CM-2: Score Axes — 7 vs 4 (Code Mismatch Dimension)

| Documentation | Code | Impact |
|---|---|---|
| 7 axes (4 stable + 3 extended) | 4 axes in `Score` struct | **Medium** — documentation overpromises |

This is the code-dimension of DI-5 above. The `Score` struct in `roko-core` has exactly 4 fields; the 3 extended axes (`specificity`, `urgency`, `empathy`) do not exist in the code.

---

### CM-3: Attestation Field

| Documentation | Code | Impact |
|---|---|---|
| `attestation` field specified in Engram/Signal docs | Not present in `Signal` struct | **Low** — Phase 2+ feature |

The attestation field enables cryptographic signing of Engrams for multi-agent trust scenarios. It is correctly documented as a future feature but the absence may mislead developers looking for it.

---

### CM-4: Conductor Layer Assignment

| Documentation | Code | Impact |
|---|---|---|
| `roko-conductor` documented as L3 or L4 | Has compile-time dependency on `roko-learn` (L2/Cross-cut) | **Medium** — layer violation in code, undocumented |

This is the code-dimension of the dependency violation documented in finding [AA-03: Layer Taxonomy](./03-finding-layer-taxonomy.md). The documentation says L3/L4 but the actual dependency graph places it closer to L2 due to the `roko-conductor` → `roko-learn` dependency.

**Resolution**: Fix the violation (extract `HealthMetrics` trait to L0) AND update documentation to accurately reflect the resolved layer assignment.

---

## Consolidated Impact Table

| ID | Category | Severity | Effort to Fix | Priority |
|---|---|---|---|---|
| DI-1 | Doc inconsistency | Low | Trivial | Low |
| **DI-2** | **Doc inconsistency** | **Medium** | **Trivial** | **High** |
| DI-3 | Doc inconsistency | Low | Trivial | Low |
| DI-4 | Doc inconsistency | Low | Trivial | Low |
| **DI-5** | **Doc inconsistency** | **Medium** | **Small** | **High** |
| CM-1 | Code-doc mismatch | None | — (intentional) | — |
| **CM-2** | **Code-doc mismatch** | **Medium** | **Small** | **High** |
| CM-3 | Code-doc mismatch | Low | — (Phase 2+) | Low |
| **CM-4** | **Code-doc mismatch** | **Medium** | **Medium** | **High** |

---

## Cross-References

- [AA-03: Layer Taxonomy](./03-finding-layer-taxonomy.md) — DI-2 and CM-4 are manifestations of the same layer coherence problem
- [AA-02: Trait Sufficiency](./02-finding-trait-sufficiency.md) — DI-3 relates to the Substrate implementation count
- [AA-10: Prioritized Improvements](./10-prioritized-improvements.md) — Improvements #2, #3, #4 address DI-2, DI-3, DI-5
- [RA-00: Architecture Subsystem](../readiness-audit/subsystem-architecture.md) — Readiness audit covers the same terminology divergence
