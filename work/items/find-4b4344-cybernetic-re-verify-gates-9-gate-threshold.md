+++
id = "find-4b4344"
kind = "finding"
title = "[cybernetic re-verify: gates] 9 gate/threshold closures wired into deleted Runner-v2"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-gate"]
created = 2026-09-06
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
anchors = ["crates/roko-cli/src/runner/persist.rs::observe_residual", "crates/roko-cli/src/runner/persist.rs::apply_profile", "crates/roko-cli/src/runner/persist.rs::should_skip_rung_for_temperament", "crates/roko-gate/src/ratchet.rs::GateRatchet", "crates/roko-cli/src/graph_task_dispatch.rs:2030"]
links = { depends_on = [], blocks = [], related = ["reg-c7ecf6", "find-34a4b5"], supersedes = [], duplicate_of = "" }
+++
Checked done 2026-09-06 via runner/event_loop.rs or gate_dispatch.rs: P1-08 observe_residual, P1-10 domain profiles, P1-11 GateRatchet, P1-12 skip advisory, P1-35 symbol rung oracles, P1-36 inner-gate thresholds, P1-39 Evolved provenance, P2-22 Prometheus gate metrics, P3-15 retry alignment.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P2 -- Add Missing Observability`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P3 -- Improve Existing Mechanisms`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops`

A source claims this was fixed; confirm against current code before closing.

How to verify: Confirm Graph gate path (graph_execution/ or roko-graph verify cells) calls these; check .roko/learn/gate-thresholds.json changes after a Graph run. / grep these symbols for non-test call sites reachable from Graph plan execution.

Merged 2 mined candidates: m3-031, m3-032.

Verified 2026-09-28: rung_for_gate_name is used on the Graph path (graph_task_dispatch.rs:2030). But GateThresholds::observe_residual (crates/roko-cli/src/runner/persist.rs:363), apply_profile (:397) and should_skip_rung_for_temperament (:437) have no production callers, and GateRatchet (roko-gate/src/ratchet.rs) is only re-exported. build_rung_execution_inputs and update_gate_threshold no longer exist, though CLAUDE.md still cites the former. suggested_max_retries feeds only displays (tui/dashboard.rs:3154, commands/util.rs:1072), not retry policy. The import warning was wrong: crates/roko-cli/src/runner/gate_dispatch.rs exists.
