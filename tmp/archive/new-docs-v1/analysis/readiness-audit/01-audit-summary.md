---
title: "Readiness Audit: Summary"
section: analysis
subsection: readiness-audit
id: ra-01-summary
source: 31-implementation-readiness-audit.md (§Documentation Quality Observations, §Estimated Implementation Effort)
tags: [summary, strengths, weaknesses, patterns, effort-estimates]
---

# Readiness Audit: Summary

> **Three patterns dominate the audit findings**:
> 1. The documentation is unusually honest and mathematically precise
> 2. Error handling is the universal weak spot
> 3. The "built but not wired" pattern is the central failure mode

---

## Systemic Strengths

### 1. Self-Aware Status Documents

Sections 02, 06, 09, 10, 12, 13, 15, 19 each contain a "current status and gaps" document with honest inventories. These are the most valuable documents in the corpus. Self-aware status docs are rare in engineering projects and signal unusually good epistemic hygiene.

### 2. Academic Grounding

Every section cites primary research. The citations are correctly applied to design decisions (not decorative). Best examples:
- §13 Coordination: 40+ papers (Turing 1952, Kauffman 1993, Woolley 2010, Dorigo 1996)
- §10 Dreams: 30+ papers (McClelland 1995, Mattar & Daw 2018, Walker & van der Helm 2009)
- §05 Learning: UCB1, LinUCB, Thompson sampling with proper equations

### 3. Config Completeness

17 of 21 sections score 4+ on `config_params`. `roko.toml` schema coverage is extensive with validation rules, env var mappings, and TOML blocks.

### 4. Mathematical Precision

Sections 01, 03, 05, 07, 13, 14, 16, 20 have formulas specified at paper-quality precision with worked examples. Specific cases:
- §03 Composition: VCG Attention Auction with truthful bidding proofs and PoA bounds
- §05 Learning: 31.6× collective calibration derivation, mathematically grounded
- §16 Heartbeat: T0 probe system with exact cost budgets; `CorticalState<const N: usize>` const generics
- §07 Conductor: 21 production failures mapped to conductor mechanisms; every threshold constant traces to a production failure issue number

---

## Systemic Weaknesses

### 1. Error Handling Under-Specified (Universal)

Mean 3.4/5 — the worst-scoring criterion. The happy path is always precise; failure modes are often implicit.

Only three sections score 5/5 on errors: §01 Orchestration, §07 Conductor, §17 Lifecycle.

The weakest sections are:
- **§08 Chain**: errors=2/5 (worst in the audit)
- **§10 Dreams**: errors=2/5 
- **§15 Code Intelligence**: errors=2/5 (functions return structs directly, no `Result`)

### 2. The "Built But Not Wired" Pattern

This is the codebase's central failure mode. Complete, tested code that has no consumer:

| Component | Status |
|---|---|
| ToolDispatcher (02/11) | Built, wired to SafetyLayer, never invoked from orchestrate.rs |
| roko-index (15) | Built, 32 tests, no consumer |
| roko-lang-rust/ts/go (15) | Built, 92 tests combined, no consumer |
| ConductorBandit (07) | Built, not wired into `evaluate()` |
| PAD persistence (03/09) | PAD resets every session |

### 3. Test Criteria for Advanced Features

Core components have good test specs. Advanced features often have no test criteria at all:
- VCG auction (§03): 9/9 implementation items "Not yet"
- MVT foraging (§03): stopping rule not applied
- Sheaf geometry (§20): no test criteria
- Causal discovery (§20): no test criteria

### 4. Phase Boundary Ambiguity

Some sections mix immediately implementable features with Phase 2+ aspirations without clear separation. Well-handled: §08, §14 (explicit Tier labels). Less well-handled: §06, §12.

---

## Implementation Effort Summary

| Category | Effort | ROI |
|---|---|---|
| **Tier 0 (G1-G5)** | ~4 person-weeks | Immediate — unblocks self-hosting quality |
| **Tier 1 (G6-G15)** | ~8 person-weeks | High — closes feedback loops, fixes data integrity |
| **Tier 2 (G16-G25)** | ~20 person-weeks | Medium — feature enrichment |
| **Tier 3 (G26-G33)** | ~50+ person-weeks | Long-term — Phase 2+ advanced capabilities |

After Weeks 1-4 (Tier 0 + early Tier 1): roko reaches **full self-hosting with safety**  
After Weeks 4-8 (mid Tier 1): roko reaches **intelligent self-hosting**  
After Weeks 8-12 (late Tier 1 + early Tier 2): roko reaches **optimal self-hosting**

---

## Per-Section Effort Estimates

| Section | Core Wiring | Advanced Features | Total |
|---|---|---|---|
| 00 Architecture | 1 (rename) | 8 (docs 25-29) | 9 wks |
| 01 Orchestration | 0 (done) | 4 (CRDT, saga) | 4 wks |
| 02 Agents | 2 (G1, G4) | 6 (HTTP backends, temperament) | 8 wks |
| 03 Composition | 1 (G6 base) | 8 (VCG, MVT, HDC dedup) | 9 wks |
| 04 Verification | 0 (done) | 6 (eval gen, EvoSkills, forensic) | 6 wks |
| 05 Learning | 2 (G7) | 4 (ADAS, TrackAndStop) | 6 wks |
| 06 Neuro | 2 (G10, G11) | 12 (somatic, cross-domain, library) | 14 wks |
| 07 Conductor | 0 (done) | 6 (cognitive signals, L3/L4) | 6 wks |
| 08 Chain | 0 (deferred) | 24+ (full DeFi stack) | 24+ wks |
| 09 Daimon | 2 (G5, G12) | 6 (somatic landscape, contrarian) | 8 wks |
| 10 Dreams | 1 (G15) | 10 (REM, HDC counterfactual, hypnagogia) | 11 wks |
| 11 Safety | 1 (G13) | 12 (witness DAG, CaMeL, formal) | 13 wks |
| 12 Interfaces | 3 (TUI completion) | 16 (Spectre, Portal, A2UI) | 19 wks |
| 13 Coordination | 1 (G19) | 16 (mesh, morphogenetic, pathology) | 17 wks |
| 14 Identity/Economy | 0 (deferred) | 24+ (blockchain + DeFi) | 24+ wks |
| 15 Code Intelligence | 2 (G2, G3, G20) | 4 (SQLite, MCP server) | 6 wks |
| 16 Heartbeat | 0 | 12 (POMDP, VCG, probes) | 12 wks |
| 17 Lifecycle | 1 | 4 (type-state pipeline) | 5 wks |
| 18 Tools | 1 (G22) | 8 (plugin SDK, WASM) | 9 wks |
| 19 Deployment | 1 (G14) | 6 (daemon, roko-serve) | 7 wks |
| 20 Technical Analysis | 0 | 18+ (oracles, causal, sheaf) | 18+ wks |

---

## Cross-References

- [00-overview.md](./00-overview.md) — Scorecard and crate status table
- [99-next-actions.md](./99-next-actions.md) — Prioritized gap list
- [../integration-map/00-overview.md](../integration-map/00-overview.md) — Missing integrations (separate from the gaps here)
