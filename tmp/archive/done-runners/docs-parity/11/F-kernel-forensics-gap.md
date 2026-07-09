# F — Cognitive Kernel, Forensics, Coverage Status (Docs 14, 15, 16)

Parity review for the cognitive-kernel, forensic-AI, and integration-status chapters.

Current audit position:
- The cognitive-kernel chapter is mostly deferred design material and should be handled as status calibration, not a broad redesign agenda for this batch.
- The forensic-AI chapter should be treated as positioning built on real auditability primitives, not as shipped compliance packaging.
- Doc 16 should explicitly replace the stale `critical integration gap` headline with a coverage-status framing: most routed provider-backed paths are covered, while subprocess and specialty paths still need work.

Generated: 2026-04-18.

---

## F.01 — Cognitive kernel remains deferred and should be described narrowly (Doc 14)

**Status**: DEFERRED
**Severity**: LOW
**Doc claim**: Doc 14 specifies namespaces with ACLs, typed cognitive signals, cognitive scheduling, and Engram syscalls.
**Audit reality**: These are not shipped as a coherent kernel subsystem. Some adjacent primitives exist elsewhere in the platform, but the chapter should be rewritten as deferred status-calibration material, not as a near-term architecture program.

This applies to:
- namespace / ACL kernel objects
- typed interrupt channels as a dedicated kernel surface
- kernel-style scheduling
- universal Engram syscall enforcement

---

## F.02 — Forensic foundation ships; compliance packaging does not (Doc 15)

**Status**: POSITIONING
**Severity**: LOW
**Doc claim**: Doc 15 presents the system as pre-compliant for multiple regulatory regimes.
**Audit reality**: The technical foundations for replay, auditability, and lineage exist and are worth documenting. What does not ship is regulator-facing packaging: report generators, evidence bundles, export formats, or pre-certified templates.

The rewrite should separate:
- shipped foundation: replay, audit chain, content-addressed lineage
- deferred / informational packaging: EU AI Act mapping, SEC/CFTC packaging, HIPAA / SOX / GDPR templates, pre-certified agent kits

That makes the chapter accurate without discarding the strategic value of the design.

---

## F.03 — Doc 16 should replace the stale headline with coverage status

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: The old framing emphasized a critical integration gap between `SafetyLayer` and execution.
**Audit reality**: That framing is stale and should be replaced directly. The practical issue is remaining coverage gaps after substantial integration progress.

The revised headline and body should say:
- routed provider-backed paths largely reach `ToolDispatcher` and the existing safety stack
- remaining gaps are concentrated in subprocess-backed paths and a small set of specialty or native endpoints
- this is now a coverage and consistency problem, not evidence that the core safety architecture failed to land

---

## F.04 — Coverage matrix is the right replacement for the stale Doc 16 headline (Doc 16)

**Status**: DOCS ONLY
**Severity**: MEDIUM
**Doc claim**: The current chapter still carries residual language from an earlier, more severe state.
**Audit reality**: The most useful rewrite is a compact coverage matrix or status table showing which execution paths do and do not pass through `ToolDispatcher`.

The table should distinguish:
- covered routed HTTP / provider-backed paths
- uncovered or partially covered subprocess paths
- specialty endpoints needing bespoke adapters

This makes the remaining work concrete and testable.

---

## F.05 — Remaining Doc 16 work is bounded to subprocess and specialty gaps (Doc 16)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: The old document reads like a broad integration failure.
**Audit reality**: The remaining work is narrower:
- subprocess execution paths that bypass dispatcher enforcement
- specialty provider modes that do not fit the main routed path
- consistency / verification work to prove coverage and prevent regressions

That is still important, but it should not be described as if the whole provider stack is unsafe by default.

---

## F.06 — Existing safety crates should be foregrounded, not treated as hypothetical (Docs 15, 16)

**Status**: DONE
**Severity**: —
**Doc claim**: The surrounding sections describe `SafetyLayer`, dispatch enforcement, and auditability.
**Audit reality**: These crates and control surfaces already ship. The docs should foreground them as current baseline capability and describe deferred work as extensions around that baseline.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 |
| PARTIAL | 3 |
| DOCS ONLY | 1 |
| POSITIONING | 1 |
| DEFERRED | 1 |

Section F should be presented as:
- Doc 14: deferred cognitive-kernel design
- Doc 15: real forensic / audit foundation, deferred compliance packaging
- Doc 16: coverage-status report that explicitly replaces the stale `critical integration gap` headline and focuses on subprocess and specialty gaps

## Single-Agent 90-Minute Follow-up

Reasonable work for one person in one session:
1. Rename the Doc 16 framing in headings and summary text from `critical integration gap` to `coverage status` or equivalent.
2. Add a provider-path coverage table showing covered versus uncovered execution paths.
3. Add one short note in Doc 15 separating shipped auditability primitives from deferred regulator-facing packaging.
4. Add `Deferred` banners to the Doc 14 cognitive-kernel subsections.

Not realistic for this batch:
- building a full cognitive kernel
- shipping regulator-facing compliance exports
- closing every subprocess and specialty adapter gap
