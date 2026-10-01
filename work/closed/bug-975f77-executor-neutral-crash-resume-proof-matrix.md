+++
id = "bug-975f77"
kind = "bug"
title = "Executor-Neutral Crash/Resume Proof Matrix"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/138-crash-resume-proof-matrix.md#138 — Executor-Neutral Crash/Resume Proof Matrix"
discovered_from = "audit:tmp/backlog/archive/138-crash-resume-proof-matrix.md#138 — Executor-Neutral Crash/Resume Proof Matrix"
anchors = ["crates/roko-graph/tests/crash_resume_equivalence.rs", "crates/roko-cli/tests/runner_crash_recovery.rs"]
links = { depends_on = [], blocks = [], related = ["gap-cecf52", "gap-01b2ff"], supersedes = [], duplicate_of = "gap-6ca8fb" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-6ca8fb (the same #138 process-boundary crash/resume matrix). The blocking premise is gone: Graph became the default engine (#260) and Runner-v2's event loop was deleted (6b5da8616); the in-process P0-GE-1 harness exists at crates/roko-graph/tests/crash_resume_equivalence.rs."
+++
[blocked] Blocked — GraphEngine cannot become the default until restart is proven not to duplicate side effects

Imported without verification from:
- `tmp/backlog/archive/138-crash-resume-proof-matrix.md#138 — Executor-Neutral Crash/Resume Proof Matrix`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase D: Lifecycle, Topology, and Cutove 6B`

Warning: every file this item cites is gone (`crates/roko-cli/tests/graph_crash_resume.rs`, `tmp/engine-audit/16-convergence-plan.md`, `tmp/engine-audit/SPEC-HARDENING-ADDENDUM.md`, `tmp/engine-audit/evidence/crash-resume-matrix.json`) — likely obsolete or moved.

How to verify: Check: Every scenario runs in a deterministic integration harness and asserts exact activity/receipt counts.; Completed tasks and committed external effects execute exactly once across restart.; In-flight ambiguous activities resolve through an… NOTE: CONSOLIDATED P0-GE-1 (prove crash/resume equivalence for Graph engine) is marked done 2026-09-20 and may close this. [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / P0 -- Critical…

Verified 2026-09-28: closed as duplicate; see [closed].evidence.
