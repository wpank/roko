+++
id = "bug-23ab24"
kind = "bug"
title = "`show costs` aggregates historical broken runs (7.8% pass rate, $120 total)"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/show"]
created = 2026-09-18
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
anchors = ["crates/roko-cli/src/commands/show.rs::render_costs", "crates/roko-cli/src/tui/dashboard.rs::load_efficiency_summary", "crates/roko-cli/src/tui/dashboard.rs::efficiency_summary_from_events"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn show_costs_excludes_events_outside_window' crates/roko-cli/src/commands/show.rs && cargo test -p roko-cli --bin roko show_costs_excludes_events_outside_window"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:10Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:11Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
Cost and pass-rate reports include early runs from broken provider configs with no windowing or decay, making current performance look far worse.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-12: `show costs` reports $120 total but most is from old broken runs`

How to verify: Run roko show costs; check for time window/decay options.

Verified 2026-09-28 (static check against 3d0ee4d02): `roko show` accepts only --dashboard/--live/--follow/--serve-url/--workdir plus a subject (crates/roko-cli/src/main.rs:601-621) and has no --since, window or decay option. render_costs (crates/roko-cli/src/commands/show.rs:203-226) prints total cost and pass rate from DashboardData::load_best_effort, and load_efficiency_summary (crates/roko-cli/src/tui/dashboard.rs:2897-2925) sums cost_usd and gate_passed==Some(true) over every event in .roko/learn/efficiency.jsonl, so old broken runs are still included. show.rs was last changed in 244f564e1; no uncommitted change adds windowing.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `roko show costs` and the overview's cost lines count only the `--since` window (default 7d; `--since all`
  restores every turn, added for bug-6c11d1). The summary names its window and adds an "all time" line. The pass
  rate is now "gate pass rate: P% (p of v gate verdict(s))": it divided by every turn, but most turns carry no
  verdict (a task's earlier turns, provider failures, ungated runs). A Python count of the main checkout's
  efficiency log on 2026-10-01 (rows with an agent id) gives 37 of 42 verdicts passed in the last 7 days, where
  the old formula gave 206 of 874 turns (24%) all-time.
- The verify command ran `--lib`, but `commands/show.rs` belongs to the `roko` binary, so the filter matched
  nothing and passed vacuously. It now runs `--bin roko`.
- Not changed: the TUI dashboard's all-time `EfficiencySummary` (`tui/dashboard.rs::load_efficiency_summary`,
  `efficiency_summary_from_events`), which `roko show` no longer reads.
