# E — Chain-Domain Safety (Docs 10, 11, 12, 13)

Parity review for the chain-specific safety chapters.

Current audit position:
- Chain safety beyond the existing transaction gates is Tier-6 / Phase-2+ work and should be described that way.
- The material on MEV, temporal logic, Witness DAG specialization, and formal verification is valuable background, but it is not current shipped functionality.
- The docs should read as design reference or future work, with narrow callouts to the few precursor surfaces that do exist.

Generated: 2026-04-18.

---

## E.01 — MEV taxonomy and detection are Tier-6 / Phase-2+ deferred (Doc 10)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 10 specifies MEV categories, detection algorithms, and protections.
**Audit reality**: There is no MEV detector or MEV-aware decision path in the repo. The practical near-term posture is limited to existing chain gating and simulation surfaces. Keep Doc 10 as Tier-6 / Phase-2+ future design, not as a missing near-term deliverable.

---

## E.02 — LTL / CTL monitoring is Tier-6 / Phase-2+ design material (Doc 11)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 11 describes runtime temporal monitoring, Büchi automata, CTL plan verification, and a pattern library.
**Audit reality**: None of that machinery ships. The chapter should be reframed as Tier-6 / Phase-2+ design material and pattern cataloging, not as an active subsystem waiting on minor wiring.

This applies to:
- runtime `TemporalMonitor` integration
- 40 DeFi temporal patterns
- extended code-agent and multi-agent temporal patterns
- boiling-frog / slow-escalation detectors

---

## E.03 — Witness DAG is partial only at the precursor-foundation level (Doc 12)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 presents a specialized Witness DAG with explicit vertex types, storage, proofs, and provenance queries.
**Audit reality**: The repo has precursor primitives for content-addressed lineage and witnessing, but not the specialized Tier-6 / Phase-2+ package described in the doc. The rewrite should make the boundary explicit:
- precursor foundation exists: content-addressing, lineage, attestation-style surfaces
- deferred: explicit Witness DAG type system, SQLite packaging, Datalog provenance queries, ZK proof layer, specialty safety queries

The gap here is packaging and specialization, not absence of every underlying idea.

---

## E.04 — Formal verification pipeline is Tier-6 / Phase-2+ deferred (Doc 13)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 13 describes a five-stage verification toolchain and a broader verification-guided design story.
**Audit reality**: The named external verification pipeline does not ship in this repo. Treat this as Tier-6 / Phase-2+ chain-assurance work and keep the current language explicitly aspirational.

This includes:
- multi-tool contract verification pipelines
- large property catalogs for host agents and task lifecycle
- bespoke runtime enforcers such as `VeriGuard`

---

## E.05 — Tool contracts may exist in limited form, but not the full chain-safety stack (Doc 13)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Tools carry behavioral contracts and the dispatcher enforces them.
**Audit reality**: Some contract-related scaffolding may exist elsewhere in the repo, but that should not be used to overstate Section E. If referenced at all, it should be described as a small adjacent primitive, not evidence that the MEV / temporal logic / formal-verification story is implemented.

---

## Section Summary

| Status | Count |
|--------|-------|
| PARTIAL | 2 |
| DEFERRED | 3 |

Section E should be framed as Tier-6 / Phase-2+ work:
- MEV work is deferred
- LTL / CTL / temporal-pattern safety is deferred research design
- Witness DAG is partial only at the foundation level
- formal verification packaging is deferred

## Single-Agent 90-Minute Follow-up

Reasonable work for one person in one session:
1. Add `Tier-6 / Phase-2+ Deferred` banners across Docs 10-13.
2. Add one short note in Doc 12 distinguishing shipped lineage primitives from the deferred specialized Witness DAG package.
3. Remove or soften wording that implies MEV, LTL monitoring, or formal verification integration is close to landing.

Not realistic for this batch:
- implementing MEV heuristics
- shipping LTL or CTL monitors
- building Witness DAG storage / proof systems
- integrating external formal-verification toolchains
