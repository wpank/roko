+++
id = "bug-16a6b6"
kind = "bug"
title = "DF-0925 P1-2: TUI never learns the full plan set (progress denominator and clock wrong)"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-2. The TUI has never been told more than one plan exists"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-2. The TUI has never been told more than one plan exists"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::plan_set_entries", "crates/roko-core/src/dashboard_snapshot.rs::DashboardSnapshot::plan_set_complete", "crates/roko-cli/src/tui/state/snapshot.rs:483"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'DashboardEvent::PlanSetLoaded' crates/roko-cli/src/graph_execution/plan_runner.rs"

[[verify]]
command = "cargo test -p roko-core plan_set_completes_only_when_every_member_is_terminal"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "(working tree, uncommitted; not in HEAD 91b4745f8) crates/roko-cli/src/graph_execution/plan_runner.rs publishes DashboardEvent::PlanSetLoaded (plan_set_entries) before any plan starts; roko-core dashboard_snapshot.rs handles PlanSetLoaded/plan_set_complete(); tui/state/snapshot.rs:483 keeps one run clock across plan boundaries via plan_set_running."
+++
No PlanSetLoaded event; snap.plans grows only on PlanStarted, so F1 shows 1/10 not x/87, the plan tree has one row and the run clock resets between plans.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-2. The TUI has never been told more than one plan exists`

How to verify: Run a multi-plan set; compare F1 totals with task count.

Verified 2026-09-28: closed as done; see [closed].evidence.
