+++
id = "bug-6dc672"
kind = "bug"
title = "DF-0925 P1-5: Force-accepted tasks render as clean green passes"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::node_outcome", "crates/roko-core/src/dashboard_snapshot.rs:1515"]
links = { depends_on = [], blocks = [], related = ["bug-82d47b", "bug-06e2d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --lib forced_accept_verdict_is_not_reported_as_passed'
+++
Complete maps to outcome 'passed' and the failure test only matches 'fail'/'error', so operators cannot distinguish force-accepted tasks from genuine passes.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass`

How to verify: Force-accept a task and inspect TUI rendering.

Verified 2026-09-28: still open, fix visibly mid-change in the working tree - runner/graph_tui_bridge.rs:238-247 (uncommitted) maps a ForcedAccept verdict to TASK_OUTCOME_ACCEPTED_WITH_FAILURES, but that constant is not defined anywhere yet (unresolved import from roko_core::dashboard_snapshot) and the snapshot still classifies outcomes only by contains(fail/error) (dashboard_snapshot.rs:1515). With bug-82d47b fixed the Graph dispatcher no longer produces ForcedAccept, so this becomes defensive (legacy checkpoints) once both land.
