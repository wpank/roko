+++
id = "gap-be610c"
kind = "gap"
title = "Legacy Fallback Compatibility-Window Exit Gate"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-gate"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/277-legacy-fallback-compatibility-exit-gate.md#277 — Legacy Fallback Compatibility-Window Exit Gate"
discovered_from = "audit:tmp/backlog/archive/277-legacy-fallback-compatibility-exit-gate.md#277 — Legacy Fallback Compatibility-Window Exit Gate"
anchors = ["crates/roko-cli/src/main.rs:2013", "crates/roko-cli/src/commands/plan.rs:597"]
links = { depends_on = [], blocks = [], related = ["gap-2081d2", "gap-824c1d", "find-b9d033", "gap-0aea18"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Obsolete: the compatibility window it gated has closed. Runner-v2 was removed, --engine legacy/runner-v2 now exits with an error (crates/roko-cli/src/main.rs:2013, crates/roko-cli/src/commands/plan.rs:597-604), and #276 already retired WorkflowEngine, so nothing is left to gate. Leftover legacy residue is tracked by find-b9d033 / gap-2081d2."
+++
[blocked] Blocked — external release evidence required —

Imported without verification from:
- `tmp/backlog/archive/277-legacy-fallback-compatibility-exit-gate.md#277 — Legacy Fallback Compatibility-Window Exit Gate`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase D: Lifecycle, Topology, and Cutove 10`

Warning: every file this item cites is gone (`tmp/engine-audit/RUN-LEDGER.md`, `tmp/engine-audit/SPEC-HARDENING-ADDENDUM.md`) — likely obsolete or moved.

How to verify: Check: Evidence links, release identifier, review date, approver, and decision are recorded.; Any unresolved safety/correctness divergence keeps this gate blocked.; Only after this checkbox is complete may #261/#276 become ready. [evidence: own status: Blocked — external release evidence required; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: superseded; the legacy engine it gated is gone.
