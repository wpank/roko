+++
id = "gap-a29711"
kind = "gap"
title = "Production Topology Restore and Kill-Point Gate"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/284-topology-restore-killpoint-gate.md#284 — Production Topology Restore and Kill-Point Gate"
discovered_from = "audit:tmp/backlog/archive/284-topology-restore-killpoint-gate.md#284 — Production Topology Restore and Kill-Point Gate"
anchors = ["crates/roko-graph/src/topology.rs::ProductionPlanTopology", "crates/roko-graph/src/finally.rs::GuaranteedFinallyController", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = ["gap-22b0a2", "gap-8ea1bd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --test topology_kill_points'
+++
[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/284-topology-restore-killpoint-gate.md#284 — Production Topology Restore and Kill-Point Gate`

Warning: every file this item cites is gone (`crates/roko-cli/tests/topology_kill_points.rs`, `crates/roko-cli/tests/topology_restore.rs`, `tmp/engine-audit/22-run-lifecycle-state-model.md`, `tmp/engine-audit/SPEC-HARDENING-ADDENDUM.md`) — likely obsolete or moved.

How to verify: Check: Register plan-controller guaranteed-finally/release/terminal state from #256.; Register workflow iteration/generation/phase/commit state from #257.; Implement every fixed plan/workflow kill point and exact-count assertion above. [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: still open - no crates/roko-cli/tests/topology_restore.rs or topology_kill_points.rs and no kill-point harness anywhere; ProductionPlanTopology (roko-graph/src/topology.rs:90) and GuaranteedFinallyController (roko-graph/src/finally.rs:270) exist without the restore/kill-point gate. Severity lowered p0 -> p2: a blocked proof gate, not a broken core loop.
