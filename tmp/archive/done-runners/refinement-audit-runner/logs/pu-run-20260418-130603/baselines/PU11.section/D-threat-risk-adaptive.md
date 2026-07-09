# D — Threat Model and Adaptive Risk (Docs 08, 09)

Parity of the two biggest safety chapters: threat model (968 lines —
attack trees, NIST AI RMF, MITRE ATLAS, STRIDE-AI, OWASP Agentic Top
10) and adaptive risk (1,101 lines — Kelly sizing, Beta-Binomial
OperationalConfidenceTracker, health scoring, 5D safety budgets).

Most of the **framework-level content** (NIST, MITRE, OWASP mappings)
is informational design — no code implements these taxonomies. Most
of the **specific mechanisms** (Kelly sizing, Beta-Binomial confidence,
safety budgets) are Phase 2+. A few surfaces (hard shields via
SafetyLayer deny-patterns, Daimon integration) ship.

Generated: 2026-04-16.

---

## D.01 — Attack tree taxonomy is informational (Doc 08 §"General-Purpose Attack Trees")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 enumerates 4 attack-tree categories (prompt injection, sandbox escape, credential exfiltration, resource exhaustion) with branching failure modes.
**Reality**: No `AttackTree` type in the codebase. The shipping defenses (SafetyLayer 6-guard + ToolDispatcher 7-stage pipeline + ScrubPolicy secret scrubbing + RateLimiter exhaustion limits) address each attack-tree branch operationally — but the tree enumeration itself is design documentation.
**Fix sketch**: Doc 08 §"Attack Trees" stays informational; cross-link each branch to the shipping defense (prompt injection → ScrubPolicy + XML delimiters + SafetyLayer; sandbox escape → PathPolicy + SandboxEnforcer; exfiltration → TaintTracker + ScrubPolicy; exhaustion → RateLimiter + DreamBudget + DreamSchedulePolicy).

---

## D.02 — Chain-domain attack trees (Doc 08 §"Chain-Domain Attack Trees")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Chain-specific attack trees: MEV, reorg, oracle manipulation, reentrancy.
**Reality**: Cross-ref batch 08 — the chain layer is largely Tier-6 deferred, so chain-specific threat models are correspondingly frontier. Informational.

---

## D.03 — 8 residual risks + formal safety analysis (Doc 08 §"8 Residual Risks", §"Formal Safety Analysis")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 enumerates 8 residual risks + formal safety analysis framework.
**Reality**: Informational — no shipping residual-risk tracker type.

---

## D.04 — NIST AI RMF 4-function alignment (Doc 08 §"NIST AI RMF Alignment")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08's 2025-04 enhancement adds NIST AI RMF alignment across 4 functions (Govern, Map, Measure, Manage).
**Reality**: `Grep 'NIST\|AI RMF' crates/ --include=*.rs` returns zero matches. Pure compliance-framework mapping in docs.
**Fix sketch**: Doc 08 §"NIST AI RMF" should carry `Design — compliance framework only` banner. When the forensic-AI compliance work (Doc 15) advances, this may become a real mapping.

---

## D.05 — MITRE ATLAS technique mapping (10 techniques) (Doc 08 §"MITRE ATLAS Technique Mapping")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08's 2025-04 enhancement maps to 10 MITRE ATLAS adversarial ML techniques.
**Reality**: Same as D.04 — informational. `Grep 'MITRE\|ATLAS' crates/ --include=*.rs` returns zero matches.

---

## D.06 — STRIDE-AI 6-category classification (Doc 08 §"STRIDE-AI Classification")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: STRIDE-AI extends Microsoft's STRIDE with 6 AI-specific categories.
**Reality**: Same as D.04 — informational.

---

## D.07 — OWASP Agentic Top 10 mapping (ASI01-ASI10) (Doc 08 §"OWASP Agentic Top 10")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 maps to OWASP's Agentic Top 10 (ASI01-ASI10).
**Reality**: Same as D.04 — informational.

---

## D.08 — Cascading failure analysis + blast radius modeling (Doc 08 §"Cascading Failure Analysis")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 describes blast-radius modeling for cascading failures.
**Reality**: `Grep 'blast_radius\|cascading_failure' crates/ --include=*.rs` returns zero matches. Design-only.

---

## D.09 — Adversarial testing framework (Doc 08 §"Adversarial Testing Framework")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 08 and Doc 00 both reference an adversarial testing framework with 9 attack categories.
**Reality**: Same as A.11 — 50+ tests ship across safety modules but their coverage of the 9 specific categories is unverified.

---

## D.10 — Five-layer adaptive risk hard shields (Doc 09 §"Hard Shields")

**Status**: DONE (via SafetyLayer)
**Severity**: —
**Doc claim**: Layer 1 of adaptive risk is "hard shields" — boolean refuse-at-sink rules that override any soft reasoning.
**Reality**: SafetyLayer (A.01) implements hard shields: BashPolicy deny patterns (rm -rf, sudo, curl pipe, fork bombs), GitPolicy protected-branch blocks, NetworkPolicy private-network denial, PathPolicy escape prevention, ScrubPolicy secret scrubbing. These are the shipping hard shields.

---

## D.11 — Kelly sizing with confidence multiplier (Doc 09 §"Kelly Sizing")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 09 describes Kelly-criterion sizing for action magnitudes (capital, position size) scaled by confidence.
**Reality**: `Grep 'kelly\|Kelly' crates/ --include=*.rs` returns zero matches. Frontier — would couple to chain-domain position sizing which is itself Tier-6.

---

## D.12 — Beta-Binomial OperationalConfidenceTracker (Doc 09 §"Beta-Binomial")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Beta-Binomial conjugate prior tracks operational confidence per category of action.
**Reality**: `Grep 'beta_binomial\|OperationalConfidenceTracker' crates/ --include=*.rs` returns zero matches. Frontier. The closest shipping analogue is `crates/roko-learn` cascade-router bandit + adaptive gate thresholds, which provide EMA-based learning but not Beta-Binomial conjugacy.

---

## D.13 — Health scoring + Daimon integration (Doc 09 §"Health Scoring", §"Daimon Integration")

**Status**: DONE (cross-cut)
**Severity**: —
**Doc claim**: Health scoring + Daimon affect-aware risk modulation.
**Reality**: Batch 07 E.05 — `HealthMonitor` ships but is dark at runtime. Batch 09 B.06 — `DaimonPolicy` is wired into cascade routing. The daimon-aware risk modulation is live via affect-to-cascade feedback.

---

## D.14 — Safety budgets (5 dimensions) with hierarchical delegation (Doc 09 §"Safety Budgets")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 09 defines 5-dimensional safety budgets (irreversibility, blast radius, footprint, uncertainty, cost) with hierarchical delegation + conservation laws + 3 allocation strategies (equal, proportional, risk-weighted).
**Reality**: `Grep 'SafetyBudget\|irreversibility\|blast_radius\|conservation_law' crates/ --include=*.rs` returns zero matches in the safety layer. The shipping `DreamBudget` (batch 10 A.08) tracks tokens + cost + duration — a simpler 3-dimensional analogue. The 5D hierarchical safety budget with conservation laws is frontier.
**Fix sketch**: Doc 09 §"Safety Budgets" should carry `Design — Phase 2+` banner. Cross-link `DreamBudget` as the shipping minimal budget primitive.

---

## D.15 — Automatic allocation strategies (Doc 09 §"Automatic Allocation Strategies")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Three strategies: equal-division, proportional-to-risk, risk-weighted.
**Reality**: Follows from D.14 — no allocation strategies because no 5D budget.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 2 (D.10 hard shields via SafetyLayer, D.13 daimon-aware risk modulation) |
| PARTIAL | 1 (D.09 adversarial test coverage) |
| NOT DONE | 12 (D.01-D.08 attack-tree / compliance framework mappings, D.11 Kelly, D.12 Beta-Binomial, D.14 5D safety budgets, D.15 allocation strategies) |

Section D is the **most frontier section in topic 11**. Docs 08 +
09 are 2,069 lines of compliance-framework mappings (NIST, MITRE,
STRIDE-AI, OWASP) and advanced risk math (Kelly, Beta-Binomial,
hierarchical budgets). Shipping safety infrastructure (A+B+C) handles
the operational layer; D is the research/compliance layer above it.

## Agent Execution Notes

### D.01 — Attack-tree cross-links

Doc 08's attack-tree enumeration is useful if each branch is
explicitly cross-linked to the shipping defense. One pass through
Doc 08 adding `(shipping: ScrubPolicy / PathPolicy / TaintTracker /
RateLimiter)` callouts per branch makes it operational.

### D.04-D.08 — Compliance banners

All informational compliance-framework mappings (NIST AI RMF, MITRE
ATLAS, STRIDE-AI, OWASP Agentic Top 10, blast-radius) should carry
`Design — compliance framework` or similar banners. They are NOT
code gaps; they are doc-only mappings.

### D.11-D.15 — Risk-math frontier

Kelly sizing, Beta-Binomial confidence, 5D budgets, hierarchical
delegation all stay Phase 2+.

Acceptance criteria:

- Doc 08 attack branches cross-link shipping defenses,
- Doc 08 compliance-framework sections explicitly informational,
- Doc 09 risk math Phase 2+ banners applied.
