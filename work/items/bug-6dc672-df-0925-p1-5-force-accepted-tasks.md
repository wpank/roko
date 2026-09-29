+++
id = "bug-6dc672"
kind = "bug"
title = "Force-accepted tasks render as clean green passes"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::node_outcome", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome", "crates/roko-core/src/dashboard_snapshot.rs::TASK_OUTCOME_ACCEPTED_WITH_FAILURES", "crates/roko-cli/src/tui/widgets/plan_tree.rs::task_icon"]
links = { depends_on = [], blocks = [], related = ["bug-82d47b", "bug-06e2d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --lib forced_accept_verdict_is_not_reported_as_passed'

[closed]
at = 2026-09-29
commit = "3d0637232"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "node_outcome (runner/graph_tui_bridge.rs:275-279) maps (Complete, ForcedAccept) to TASK_OUTCOME_ACCEPTED_WITH_FAILURES (roko-core dashboard_snapshot.rs:1089), and classify_task_outcome (:1107-1110) matches it before the fail/error heuristic (725f21e05). The TUI rendering of that outcome as a distinct amber warning (tui/state/snapshot.rs:1349, widgets/plan_tree.rs:774/802, widgets/header_bar.rs:340-363, views/plans_view.rs:1083-1173) was committed in 3d0637232, and the working tree has no edits under tui/."
+++
Complete maps to outcome 'passed' and the failure test only matches 'fail'/'error', so operators cannot distinguish force-accepted tasks from genuine passes.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-5. A force-accepted task renders as a clean green pass`

How to verify: Force-accept a task and inspect TUI rendering.

Verified 2026-09-28: still open, fix visibly mid-change in the working tree - runner/graph_tui_bridge.rs:238-247 (uncommitted) maps a ForcedAccept verdict to TASK_OUTCOME_ACCEPTED_WITH_FAILURES, but that constant is not defined anywhere yet (unresolved import from roko_core::dashboard_snapshot) and the snapshot still classifies outcomes only by contains(fail/error) (dashboard_snapshot.rs:1515). With bug-82d47b fixed the Graph dispatcher no longer produces ForcedAccept, so this becomes defensive (legacy checkpoints) once both land.

Fix in progress (2026-09-28): Committed in 725f21e05: TASK_OUTCOME_ACCEPTED_WITH_FAILURES is defined (roko-core/src/dashboard_snapshot.rs:1089), classify_task_outcome matches it before the fail/error heuristic (:1107-1116) and the snapshot uses it (:1590); node_outcome maps (Complete, ForcedAccept) to it (runner/graph_tui_bridge.rs:260-264) with test forced_accept_verdict_is_not_reported_as_passed (:433). The TUI rendering of that outcome as a distinct amber/warning state is uncommitted working-tree work (tui/theme.rs:402, tui/state/mod.rs:251/1093, widgets/plan_tree.rs, widgets/header_bar.rs:340-363, tui/app/mod.rs:303). Close with the commit once the uncommitted part lands.

Re-verified 2026-09-29: the engine side landed in 725f21e05 and the TUI amber/warning rendering (previously uncommitted) landed in 3d0637232. Tests: forced_accept_verdict_is_not_reported_as_passed (graph_tui_bridge.rs:448), accepted_with_failures_is_its_own_task_state (tui/state/tests.rs:2471), accepted_with_failures_is_counted_apart_from_passed_and_failed (roko-core dashboard_snapshot.rs:4428).
