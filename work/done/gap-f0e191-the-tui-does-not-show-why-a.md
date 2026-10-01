+++
id = "gap-f0e191"
kind = "gap"
title = "The TUI does not show why a plan is waiting (conflicting plans)"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/tui/views/plans_view.rs::render_wave_tree", "crates/roko-cli/src/tui/state/snapshot.rs::update_from_dashboard_snapshot", "crates/roko-core/src/dashboard_snapshot.rs::PlanSetEntry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'conflicts_with' crates/roko-cli/src/tui/state/mod.rs && grep -rqw 'fn plan_rows_say_why_a_plan_waits' crates/roko-cli/src/tui/views/plans_view.rs && cargo test -p roko-cli --lib plan_rows_say_why_a_plan_waits"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:36Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:11Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
