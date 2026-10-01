+++
id = "gap-22b0a2"
kind = "gap"
title = "Pre-Topology Lifecycle Checkpoint Extension Gate"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-graph"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/282-lifecycle-checkpoint-completeness.md#282 — Pre-Topology Lifecycle Checkpoint Extension Gate"
discovered_from = "audit:tmp/backlog/archive/282-lifecycle-checkpoint-completeness.md#282 — Pre-Topology Lifecycle Checkpoint Extension Gate"
anchors = ["crates/roko-graph/src/snapshot.rs::CheckpointExtension", "crates/roko-graph/src/topology.rs::ProductionPlanTopology"]
links = { depends_on = [], blocks = [], related = ["gap-6ca8fb", "bug-975f77"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Obsolete gate: #282 was a pre-topology gate blocking #256/#259/#272/#284, but that work shipped without it. ProductionPlanTopology is at crates/roko-graph/src/topology.rs:90, and the #251 layered extension ledger is at crates/roko-graph/src/snapshot.rs:192 (CheckpointExtension; EXT_* namespaces :532-552; round-trip tests :734/:751; crates/roko-cli/src/graph_checkpoint.rs:1440). The packet's own registry file (graph_execution/checkpoint_extensions.rs with provider-attempt/workspace.attempt/gate.verdict/completion-sinks/process namespaces) and per-receipt kill-point assertions were never built. The remaining kill-point proof is tracked by gap-6ca8fb and bug-975f77."
+++
[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/282-lifecycle-checkpoint-completeness.md#282 — Pre-Topology Lifecycle Checkpoint Extension Gate`

How to verify: Check: Create the exact registry, namespace table, fixture, and per-state kill assertions above.; Verify extension version/fingerprint compatibility and unknown optional preservation.; Add kill points before/after each receipt transition and assert… [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: superseded as an obsolete gate (topology and extension ledger shipped without it); residual proof work lives in gap-6ca8fb / bug-975f77.
