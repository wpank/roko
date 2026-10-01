+++
id = "bug-0fe64a"
kind = "bug"
title = "plan list drops interrupted and cancelled Graph runs: overlay_graph_checkpoint_status ignores those statuses"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-20ab07"
anchors = ["crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-20ab07"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib overlay_graph_checkpoint_status"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:06Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:16Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-20ab07 made `roko plan status` show interrupted and cancelled runs. `overlay_graph_checkpoint_status` in `plan.rs`, which `plan list` and the plan summaries use, still maps only the other checkpoint statuses, so those runs show their stale `tasks.toml` status.

## Plan

Map interrupted and cancelled checkpoints the way `plan status` now does, and add a test named `overlay_graph_checkpoint_status_*`.

## Done when

- `cargo test -p roko-cli --lib overlay_graph_checkpoint_status` passes, and covers both statuses.

## Notes

- Reported on 2026-10-01 by the worker on gap-20ab07, during the evening close-out round.
- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `overlay_graph_checkpoint_status` maps `interrupted` and `cancelled` checkpoints to that status (with the age suffix the
  other statuses get), for `plan list --json`, serve's plan listing and the TUI. Neither marks the plan complete. Test:
  `overlay_graph_checkpoint_status_shows_interrupted_and_cancelled_runs`.
