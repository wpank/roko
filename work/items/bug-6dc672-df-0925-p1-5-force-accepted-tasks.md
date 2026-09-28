+++
id = "bug-6dc672"
kind = "bug"
title = "Force-accepted tasks render as clean green passes"
status = "in_progress"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::node_outcome", "crates/roko-core/src/dashboard_snapshot.rs:1515", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome"]
links = { depends_on = [], blocks = [], related = ["bug-82d47b", "bug-06e2d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --lib forced_accept_verdict_is_not_reported_as_passed'
+++
Complete maps to outcome 'passed' and the failure test only matches 'fail'/'error', so operators cannot distinguish force-accepted tasks from genuine passes.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass`

How to verify: Force-accept a task and inspect TUI rendering.

Verified 2026-09-28: still open, fix visibly mid-change in the working tree - runner/graph_tui_bridge.rs:238-247 (uncommitted) maps a ForcedAccept verdict to TASK_OUTCOME_ACCEPTED_WITH_FAILURES, but that constant is not defined anywhere yet (unresolved import from roko_core::dashboard_snapshot) and the snapshot still classifies outcomes only by contains(fail/error) (dashboard_snapshot.rs:1515). With bug-82d47b fixed the Graph dispatcher no longer produces ForcedAccept, so this becomes defensive (legacy checkpoints) once both land.

Fix in progress (2026-09-28): Committed in 725f21e05: TASK_OUTCOME_ACCEPTED_WITH_FAILURES is defined (roko-core/src/dashboard_snapshot.rs:1089), classify_task_outcome matches it before the fail/error heuristic (:1107-1116) and the snapshot uses it (:1590); node_outcome maps (Complete, ForcedAccept) to it (runner/graph_tui_bridge.rs:260-264) with test forced_accept_verdict_is_not_reported_as_passed (:433). The TUI rendering of that outcome as a distinct amber/warning state is uncommitted working-tree work (tui/theme.rs:402, tui/state/mod.rs:251/1093, widgets/plan_tree.rs, widgets/header_bar.rs:340-363, tui/app/mod.rs:303). Close with the commit once the uncommitted part lands.
