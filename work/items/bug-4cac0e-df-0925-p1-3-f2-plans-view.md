+++
id = "bug-4cac0e"
kind = "bug"
title = "F2 Plans view scans one level, shows unrelated plans and zips status by index"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one"
anchors = ["crates/roko-cli/src/tui/dashboard.rs::load_plan_summaries", "crates/roko-cli/src/tui/views/plans_view.rs::render_wave_tree", "crates/roko-cli/src/tui/views/plans_view.rs::plan_entries_by_id", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
load_plan_summaries scans plans/*/tasks.toml only, loads once and is never refreshed in connected mode; plans_view zips live state to summaries by index, painting the wrong plan as active.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one`

How to verify: Run a nested plan set and check F2 labels.

Verified 2026-09-28: Partly fixed in the working tree (uncommitted): load_plan_summaries now uses the recursive crate::plan::plan_dirs_by_id and plans_view joins live PlanEntry rows by id (plan_entries_by_id; test live_status_joins_disk_plans_by_id_not_index), so the wrong-plan-active mislabel is gone. Still true: render_wave_tree lists every disk plan summary rather than the announced plan_set, so unrelated workspace plans are still shown. Severity lowered p1 -> p2 for the remainder.

Re-verified 2026-09-29: the by-id join (plans_view.rs::plan_entries_by_id) and recursive discovery (dashboard.rs::load_plan_summaries via plan::plan_dirs_by_id) are committed in 725f21e05, so the wrong-plan-active mislabel is fixed. In connected mode the F2 list is rebuilt from the hub snapshot's plans on each update (tui/state/snapshot.rs:1003-1045). What remains: in file-polled mode (no hub), load_plan_summaries lists every workspace plan that has a tasks.toml whenever the runner projection carries no plan_states, and Graph runs do not write plan_states. Unrelated plans are therefore still listed there.
