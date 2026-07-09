# D — Threat Model and Adaptive Risk (Docs 08, 09)

Parity review for the threat-model and adaptive-risk chapters.

Current audit position:
- Existing safety crates already ship the operational controls that matter most here: `SafetyLayer`, `ToolDispatcher`, `ScrubPolicy`, `PathPolicy`, `RateLimiter`, capability gating, and audit surfaces.
- Section D should read mainly as status calibration plus ship-soon threat-model documentation planning.
- Compliance mappings and advanced risk math remain deferred or informational. They should not be described as missing code that blocks current safety posture.

Generated: 2026-04-18.

---

## D.01 — Threat model taxonomy is the main ship-soon item in Section D (Doc 08)

**Status**: DOCS ONLY
**Severity**: LOW
**Doc claim**: Doc 08 lays out attack trees covering prompt injection, sandbox escape, credential exfiltration, and resource exhaustion.
**Audit reality**: The taxonomy itself is not implemented as code and does not need to be. The practical defenses already ship; the main remaining task is to publish the threat-model writeup with direct links to those controls.
**Keep / change**:
- Keep the attack-tree and residual-risk narrative as documentation.
- Add explicit cross-links from each attack family to shipping controls:
  - prompt injection -> `ScrubPolicy`, prompt delimiting, `SafetyLayer`
  - sandbox / filesystem escape -> `PathPolicy`, sandbox enforcement
  - secret exfiltration -> scrubbing, taint-aware handling, audit trail
  - resource exhaustion -> `RateLimiter`, runtime budgets

---

## D.02 — Chain-domain attack trees stay deferred with chain work (Doc 08)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Chain-specific trees cover MEV, reorgs, oracle manipulation, and reentrancy.
**Audit reality**: This material depends on chain-domain safety work that is itself deferred. Keep it as background only, not as a near-term implementation or planning item for this batch posture.

---

## D.03 — Residual-risk and formal-analysis sections are status-calibration material (Doc 08)

**Status**: INFORMATIONAL
**Severity**: LOW
**Doc claim**: Doc 08 enumerates residual risks and outlines formal safety analysis.
**Audit reality**: There is no dedicated residual-risk tracker or formal-analysis subsystem in the repo. That is acceptable for now; these sections should calibrate what is documented versus what is shipped, not open a broader redesign track.

---

## D.04 — Compliance mappings stay informational for now (Doc 08)

**Status**: INFORMATIONAL
**Severity**: LOW
**Doc claim**: The chapter maps controls to NIST AI RMF, MITRE ATLAS, STRIDE-AI, and OWASP Agentic Top 10.
**Audit reality**: These mappings are useful for explanation and future compliance work, but no crate implements these taxonomies directly. They should stay as reference material with a clear banner such as `Documentation / compliance mapping only`.
**Scope note**:
- NIST AI RMF mapping: informational
- MITRE ATLAS mapping: informational
- STRIDE-AI classification: informational
- OWASP Agentic Top 10 mapping: informational
- Blast-radius and cascading-failure modeling: informational

---

## D.05 — Adversarial testing exists, category-level mapping does not (Doc 08, Doc 09)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: The docs describe an adversarial testing framework with named attack categories.
**Audit reality**: Safety tests already exist across the shipping safety stack, but the docs overstate the precision of category-by-category traceability. Reframe this as:
- shipped: substantial safety test coverage
- not yet packaged: a documented matrix tying each test to each named attack category

This is a documentation and evidence-packaging task, not a blocker in the control plane.

---

## D.06 — Hard shields are already shipped (Doc 09)

**Status**: DONE
**Severity**: —
**Doc claim**: Layer 1 of adaptive risk is a set of hard shields that override softer reasoning.
**Audit reality**: This is the part of adaptive risk that is real today. `SafetyLayer` and related policies already implement the concrete deny / gate behavior.

---

## D.07 — Kelly sizing and Beta-Binomial confidence are deferred risk math (Doc 09)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 09 describes Kelly-style sizing and Beta-Binomial confidence tracking.
**Audit reality**: None of this is implemented in the current safety crates. It should be treated as future research or optional optimization work, not as missing baseline safety functionality.

---

## D.08 — Health scoring and Daimon-related modulation need calibrated wording (Doc 09)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Health scoring and Daimon input influence adaptive risk.
**Audit reality**: Some related infrastructure exists, but the broader adaptive-risk chapter still reads more mature than the code. Keep references to shipped hooks, but use this section to calibrate status rather than to imply a larger near-term redesign.

---

## D.09 — Safety budgets and allocation strategies are deferred (Doc 09)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 09 defines 5D safety budgets, hierarchical delegation, conservation laws, and automatic allocation strategies.
**Audit reality**: The full 5D framework is not implemented. Current runtime budgets are narrower and more practical. The doc should say this explicitly and avoid implying an unshipped sophisticated budget engine.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 |
| PARTIAL | 2 |
| DOCS ONLY / INFORMATIONAL | 4 |
| DEFERRED | 2 |

Section D should be presented as:
- shipping operational defenses already exist
- Doc 08 threat modeling is the main ship-soon documentation task
- compliance mappings are reference material
- advanced adaptive-risk math remains deferred

## Single-Agent 90-Minute Follow-up

Reasonable work for one person in one session:
1. Add `Documentation only` or `Deferred` banners to the relevant Doc 08 and Doc 09 sections.
2. Insert a compact threat-to-control table mapping each major attack family to the already-shipped defenses.
3. Add one short note explaining that category-level adversarial test traceability is still being packaged.

Not realistic for this batch:
- implementing RMF / ATLAS / OWASP compliance frameworks in code
- adding Kelly or Beta-Binomial machinery
- building hierarchical 5D safety-budget allocation
