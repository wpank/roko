+++
id = "gap-20ab07"
kind = "gap"
title = "`roko plan status` reads only `succeeded` from Graph checkpoints; interrupted and cancelled runs are not shown"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan-status", "roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus", "crates/roko-cli/src/graph_checkpoint.rs::canonical_checkpoint_status", "crates/roko-cli/src/graph_execution/plan_set.rs::checkpoint_label"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_reports_interrupted_and_cancelled_checkpoints' crates/roko-cli/ && cargo test -p roko-cli --bin roko plan_status_reports_interrupted_and_cancelled_checkpoints"
+++

plan_runner writes a `GraphCheckpointStatus` (`succeeded`, `failed`, `interrupted`, `cancelled`) into each plan's checkpoint. The only CLI reader is the overlay in `roko plan status` (commands/plan.rs:1466-1493), and it checks for `succeeded` only. An interrupted or cancelled run is therefore shown through its tasks.toml statuses, as if nothing had happened. Nothing else outside tests reads the new values (also reported by w3b).

Fix: show the checkpoint status in `roko plan status`, including `interrupted` and `cancelled`.

Rechecked 2026-09-29 at d9e79e9d8. roko plan status also maps failed and running; only interrupted and cancelled are dropped. The body's 'nothing else reads the new values' is out of date: graph_execution/plan_set.rs::outside_plan_status (added 725f21e05) reads canonical_checkpoint_status for plans outside a plan set, and checkpoint_label there already maps Cancelled and Interrupted. The fix can reuse both instead of the hand-parsed JSON in cmd_plan_dir_status.
