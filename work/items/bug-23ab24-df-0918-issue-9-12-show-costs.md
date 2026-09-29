+++
id = "bug-23ab24"
kind = "bug"
title = "`show costs` aggregates historical broken runs (7.8% pass rate, $120 total)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/show"]
created = 2026-09-18
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
anchors = ["crates/roko-cli/src/commands/show.rs::render_costs", "crates/roko-cli/src/tui/dashboard.rs::load_efficiency_summary", "crates/roko-cli/src/tui/dashboard.rs::efficiency_summary_from_events"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn show_costs_excludes_events_outside_window' crates/roko-cli/ && cargo test -p roko-cli --lib show_costs_excludes_events_outside_window"
+++
Cost and pass-rate reports include early runs from broken provider configs with no windowing or decay, making current performance look far worse.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-12: `show costs` reports $120 total but most is from old broken runs`

How to verify: Run roko show costs; check for time window/decay options.

Verified 2026-09-28 (static check against 3d0ee4d02): `roko show` accepts only --dashboard/--live/--follow/--serve-url/--workdir plus a subject (crates/roko-cli/src/main.rs:601-621) and has no --since, window or decay option. render_costs (crates/roko-cli/src/commands/show.rs:203-226) prints total cost and pass rate from DashboardData::load_best_effort, and load_efficiency_summary (crates/roko-cli/src/tui/dashboard.rs:2897-2925) sums cost_usd and gate_passed==Some(true) over every event in .roko/learn/efficiency.jsonl, so old broken runs are still included. show.rs was last changed in 244f564e1; no uncommitted change adds windowing.
