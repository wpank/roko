# B — Audit, Taint, and Provenance

Parity review for Docs 02 and 03.

Generated: 2026-04-18

---

## Core Read

The audit/provenance docs should stop implying that Roko only has design sketches here. Two important safety surfaces already ship in `roko-orchestrator`:

- `AuditChain` at **565 LOC**
- `TaintTracker` at **409 LOC**

The right rewrite is to document the shipped subset plainly, then label deeper provenance and taint ambitions as planned.

---

## Shipping Now

### B.01 — `AuditChain` is live

`AuditChain` is a real append-only audit primitive. Docs that describe audit chaining as future tense are stale.

The parity pack should preserve two facts:

- the implementation exists now
- its exact shape is narrower than some of the PRD language

### B.02 — provenance and lineage already exist underneath it

The audit story builds on existing content-addressed provenance rather than on a blank slate. That should remain part of the shipped baseline.

### B.03 — `TaintTracker` is live, but intentionally narrower than the frontier spec

`TaintTracker` exists today and supports a simple operational story: mark, propagate, and check taint.

The docs should say that directly. They should not imply that a Denning lattice or research-heavy label algebra is required before Roko can claim taint tracking exists.

### B.04 — attestation work should extend the current surface

The near-term work here is not "design attestation." It is:

- extend the existing `Attestation` surface
- expand the current taint model

That is the audit-approved ship-soon path.

---

## Narrow, Don’t Inflate

### B.05 — audit sinks and richer custody records are follow-on work

This batch should not pretend that every persistence, export, or chain-anchoring path is already complete. Where the docs go beyond the current audit chain, mark those sections as planned wiring or later work.

### B.06 — advanced taint theory belongs in an explicit future-work bucket

These belong in planned work, not present-tense architecture:

- Denning-style label algebra
- FIDES / RTBAS / PFI / PCAS-style extensions
- richer policy-query layers over taint state

---

## Recommended Doc Posture

For Docs 02 and 03:

1. lead with the shipped `AuditChain` and `TaintTracker`
2. describe the current scope honestly
3. name `Attestation` extension and taint expansion as ship-soon work
4. move deep custody and taint-research material into planned sections
