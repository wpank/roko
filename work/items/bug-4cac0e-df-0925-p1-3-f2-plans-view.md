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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one"
anchors = ["crates/roko-cli/src/tui/dashboard.rs::load_plan_summaries", "crates/roko-cli/src/tui/views/plans_view.rs::render_wave_tree", "crates/roko-cli/src/tui/views/plans_view.rs::plan_entries_by_id", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn f2_lists_the_plan_set_not_earlier_runs' crates/roko-cli/src/tui/views/plans_view.rs && cargo test -p roko-cli --lib f2_lists_the_plan_set_not_earlier_runs"
+++
load_plan_summaries scans plans/*/tasks.toml only, loads once and is never refreshed in connected mode; plans_view zips live state to summaries by index, painting the wrong plan as active.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-3. The F2 Plans view lists 21 unrelated plans and mislabels the active one`

How to verify: Run a nested plan set and check F2 labels.

Verified 2026-09-28: Partly fixed in the working tree (uncommitted): load_plan_summaries now uses the recursive crate::plan::plan_dirs_by_id and plans_view joins live PlanEntry rows by id (plan_entries_by_id; test live_status_joins_disk_plans_by_id_not_index), so the wrong-plan-active mislabel is gone. Still true: render_wave_tree lists every disk plan summary rather than the announced plan_set, so unrelated workspace plans are still shown. Severity lowered p1 -> p2 for the remainder.

Re-verified 2026-09-29: the by-id join (plans_view.rs::plan_entries_by_id) and recursive discovery (dashboard.rs::load_plan_summaries via plan::plan_dirs_by_id) are committed in 725f21e05, so the wrong-plan-active mislabel is fixed. In connected mode the F2 list is rebuilt from the hub snapshot's plans on each update (tui/state/snapshot.rs:1003-1045). What remains: in file-polled mode (no hub), load_plan_summaries lists every workspace plan that has a tasks.toml whenever the runner projection carries no plan_states, and Graph runs do not write plan_states. Unrelated plans are therefore still listed there.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  Re-checked at BASE: the interactive TUI has no hub-less file-polled mode any more. `App::new` (standalone
  `roko dashboard`) bootstraps an in-process StateHub and replays `.roko/events.jsonl`, and `roko plan run` uses
  the run's hub; F1 and F2 list the hub snapshot's plans. The unrelated plans came from there: `PlanSetLoaded`
  keeps every earlier plan in the snapshot, so a replayed workspace log (48 distinct plans in the main
  checkout's) or a long-lived serve hub listed earlier runs beside the current set.
  `TuiState::update_from_dashboard_snapshot` now lists, while a plan set is announced, its members plus any
  plan still running, and drops the hidden plans' tasks so they do not return as orphan rows. Without a plan set
  nothing changes. The by-id join (mislabel) was already fixed in 725f21e05.
- Not changed (reported for a new item): `DashboardData::load_plan_summaries` still lists every workspace plan
  with a `tasks.toml` when no Runner projection names plans (Graph runs write none). Besides `roko show plans`,
  that list reaches the standalone dashboard after an explicit full refresh (`tui/app/event_loop.rs::
  refresh_snapshot`, run when `replay_disk_snapshots` is set): `update_from_snapshot` replaces `plans` and
  `plan_summaries` with it, and `drain_snapshot_channel` re-applies the hub's list only when the snapshot next
  changes.
