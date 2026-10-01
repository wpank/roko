+++
id = "bug-7e1b6b"
kind = "bug"
title = "The dashboard counts unverified and skipped tasks as passed"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/graph-tui-bridge", "roko-core/dashboard"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::node_outcome", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome", "crates/roko-core/src/dashboard_snapshot.rs:1606", "apps/portal/src/lib/runState.ts:308"]
links = { depends_on = [], blocks = [], related = ["bug-6dc672"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '(NodeStatus::Complete, _) => \"passed\"' crates/roko-cli/src/runner/graph_tui_bridge.rs && grep -A10 'pub fn classify_task_outcome' crates/roko-core/src/dashboard_snapshot.rs | grep -q 'skipped' && grep -rqw 'fn skipped_and_unverified_tasks_are_not_counted_as_passed' crates/roko-core/src/ && cargo test -p roko-core --lib skipped_and_unverified_tasks_are_not_counted_as_passed"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:19Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91; only a verified pass counts as passed, unverified, skipped and already_satisfied counted apart in the snapshot, TUI and portal (portal 789/789 in wk-gates' clone); C1 passes. Merged 4c0e5646e."
+++
- `node_outcome` maps every completed node to "passed" whatever its verdict, including `Unverified`. Only `ForcedAccept` gets its own outcome (`graph_tui_bridge.rs`).
- `classify_task_outcome` (`dashboard_snapshot.rs:1107`) counts every outcome string without "fail" or "error" as passed, including "skipped" and "condition-skipped".
- Tasks accepted with failures count toward `plan.tasks_done`.

The TUI, SSE and portal therefore overstate verified success, and no dashboard count can be used as a success metric.

Fix: give unverified and skipped tasks their own outcomes end to end, and count only verified passes as passed.

Checked 2026-09-29 at f99e45dba: unchanged. A unit test in crates/roko-cli/src/runner/graph_tui_bridge.rs:483-486 asserts node_outcome(NodeStatus::Complete, Some(TaskGateVerdict::Unverified)) == "passed", so the fix must update that test.
