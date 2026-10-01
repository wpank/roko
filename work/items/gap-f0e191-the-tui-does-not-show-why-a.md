+++
id = "gap-f0e191"
kind = "gap"
title = "The TUI does not show why a plan is waiting (conflicting plans)"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/tui/views/plans_view.rs::render_wave_tree", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot", "crates/roko-core/src/dashboard_snapshot.rs::PlanSetEntry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'conflicts_with' crates/roko-cli/src/tui/state/mod.rs && grep -rqw 'fn plan_rows_say_why_a_plan_waits' crates/roko-cli/src/tui/views/plans_view.rs && cargo test -p roko-cli --lib plan_rows_say_why_a_plan_waits"
+++

`plan_set_loaded` carries each plan's `wave`, `depends_on` and `conflicts_with`, and the portal now renders them. The TUI never reads `conflicts_with`: a plan held back by the plan-set scheduler shows as queued, and the reason appears only in the event log.

Fix: show the waiting reason (waiting on X, conflicts with Y) on the plan row or in plan detail.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1 (on top of gap-386329, which keeps the announced
  `plan_set` in `TuiState`); cargo verification deferred to the batch check.
  `TuiState::plan_wait_reason` mirrors `PlanSetScheduler::admit`: a pending plan is "waiting on X, Y" while
  prerequisites have not succeeded, else "conflicts with Y" while a plan in its `conflicts_with` is running, or
  comes earlier in the set and has not started. The F2 Plans view shows it on the selected plan's detail row
  (`pending · conflicts with 01-api`) and as a `waiting:` line under the status in the plan detail pane.
  Not shown: the conflict's cause ("both write README.md"), which only the `graph.plan_waiting` event carries.
- The old verify grepped all of `tui/` for `conflicts_with`, which a test fixture alone would satisfy. It now
  needs the state code and the named test.
