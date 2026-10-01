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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/backlog/archive/284-topology-restore-killpoint-gate.md#284 — Production Topology Restore and Kill-Point Gate"
discovered_from = "audit:tmp/backlog/archive/284-topology-restore-killpoint-gate.md#284 — Production Topology Restore and Kill-Point Gate"
anchors = ["crates/roko-graph/src/topology.rs::ProductionPlanTopology", "crates/roko-graph/src/finally.rs::GuaranteedFinallyController", "crates/roko-cli/src/graph_checkpoint.rs::resume_checkpoint"]
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

Re-verified 2026-09-29: still open. 3d0637232 made Graph checkpoints upgrade-safe (fingerprint matching, rebind, authored-plan checks) but added no production-topology restore and no kill-point harness. graph_checkpoint.rs:1425-1426 still refuses --rich-topology for the checkpoint preview.

## Notes

- 2026-10-01 (wk-tamper): blocked; no code change. Still open at ebdc0f5d5, and it needs scoping before anyone builds
  the gate (several days of work):
  - `crates/roko-graph/src/finally.rs` (`GuaranteedFinallyController`, :270) is not compiled: `roko-graph/src/lib.rs`
    declares no `mod finally`, and `git log -S "pub mod finally"` finds none. The controller finally, release and
    terminal state that the first checklist step registers exists in no build. Wiring the controller into
    `run_one_plan`, or deleting the file, is the unfinished part of #256.
  - The workflow half targets #257's `integrative@1` workflow (compose, implement, gate, autofix, review, commit),
    which ran on the WorkflowEngine that #276 retired. `integrative` is now a task tier (`plan_generate.rs:216`). It
    should be re-scoped to the one-task plan `roko run` writes, or dropped.
  - `ProductionPlanTopology` (`topology.rs:90`) runs only under `--rich-topology`, with passthrough enricher stubs
    (`plan_runner.rs:2512-2525`, `with_allow_test_stubs` at :2687; gap-8ea1bd is parked), and the resume preview
    refuses it (`graph_checkpoint.rs:1833`).
  - No kill-point hook exists: `ROKO_TEST_KILL_POINT` appears nowhere. An env-triggered `process::exit` in the
    shipped binary needs a decision (a test-only cargo feature, or a cfg).
  Next step: a scope decision (Will or the coordinator). Then wire or delete `finally.rs`, add the hook behind a
  test feature, and write `crates/roko-cli/tests/topology_kill_points.rs` with counting fakes on the default
  topology.
