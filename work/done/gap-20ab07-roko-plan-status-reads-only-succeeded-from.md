+++
id = "gap-20ab07"
kind = "gap"
title = "`roko plan status` reads only `succeeded` from Graph checkpoints; interrupted and cancelled runs are not shown"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan-status", "roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus", "crates/roko-cli/src/graph_checkpoint.rs::canonical_checkpoint_status", "crates/roko-cli/src/graph_execution/plan_set.rs::checkpoint_label"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_reports_interrupted_and_cancelled_checkpoints' crates/roko-cli/ && cargo test -p roko-cli --bin roko plan_status_reports_interrupted_and_cancelled_checkpoints"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:34Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

plan_runner writes a `GraphCheckpointStatus` (`succeeded`, `failed`, `interrupted`, `cancelled`) into each plan's checkpoint. The only CLI reader is the overlay in `roko plan status` (commands/plan.rs:1466-1493), and it checks for `succeeded` only. An interrupted or cancelled run is therefore shown through its tasks.toml statuses, as if nothing had happened. Nothing else outside tests reads the new values (also reported by w3b).

Fix: show the checkpoint status in `roko plan status`, including `interrupted` and `cancelled`.

Rechecked 2026-09-29 at d9e79e9d8. roko plan status also maps failed and running; only interrupted and cancelled are dropped. The body's 'nothing else reads the new values' is out of date: graph_execution/plan_set.rs::outside_plan_status (added 725f21e05) reads canonical_checkpoint_status for plans outside a plan set, and checkpoint_label there already maps Cancelled and Interrupted. The fix can reuse both instead of the hand-parsed JSON in cmd_plan_dir_status.

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `cmd_plan_dir_status` reads the checkpoint with `graph_checkpoint::canonical_checkpoint_status` instead of hand-parsed
  JSON, and `plan_status_label` maps it: `succeeded` reads `complete`, every other status (now including `interrupted`
  and `cancelled`) shows under its own name, and a plan without a checkpoint keeps the task-count labels.
  `GraphCheckpointStatus::as_str` gives the names, so `plan_set.rs::checkpoint_label` is not needed here.
