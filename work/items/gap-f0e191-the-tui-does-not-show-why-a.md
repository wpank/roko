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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/tui/views/plans_view.rs::render_wave_tree", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot", "crates/roko-core/src/dashboard_snapshot.rs::PlanSetEntry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "conflicts_with" crates/roko-cli/src/tui'
+++

`plan_set_loaded` carries each plan's `wave`, `depends_on` and `conflicts_with`, and the portal now renders them. The TUI never reads `conflicts_with`: a plan held back by the plan-set scheduler shows as queued, and the reason appears only in the event log.

Fix: show the waiting reason (waiting on X, conflicts with Y) on the plan row or in plan detail.
