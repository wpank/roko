+++
id = "gap-386329"
kind = "gap"
title = "Plan-set entries carry no group or directory, so the F1 plan tree cannot group plans"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-core/dashboard_snapshot", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::PlanSetEntry", "crates/roko-cli/src/tui/views/dashboard_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -A40 "pub struct PlanSetEntry" crates/roko-core/src/dashboard_snapshot.rs | grep -qE "pub (group|dir)"'
+++

`PlanSetEntry` (the `plan_set_loaded` payload and the snapshot's `plan_set`) has `plan_id`, `title`, `tasks_total`, `wave`, `depends_on` and `conflicts_with`, but not the plan's group (programme directory) or path. The F2 Plans view now groups by wave (w3a), but the F1 dashboard plan tree lists a nested programme such as `plans/portal-programme/*` flat, and other clients cannot group by directory either (also reported by w3a).

Fix: add the group or relative directory to `PlanSetEntry` and group the F1 tree by it.
