+++
id = "gap-20ab07"
kind = "gap"
title = "`roko plan status` reads only `succeeded` from Graph checkpoints; interrupted and cancelled runs are not shown"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/plan-status", "roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/commands/plan.rs:1466", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus", "crates/roko-cli/src/graph_checkpoint.rs::canonical_checkpoint_status"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -qE "\"interrupted\"|Interrupted" crates/roko-cli/src/commands/plan.rs'
+++

plan_runner writes a `GraphCheckpointStatus` (`succeeded`, `failed`, `interrupted`, `cancelled`) into each plan's checkpoint. The only CLI reader is the overlay in `roko plan status` (commands/plan.rs:1466-1493), and it checks for `succeeded` only. An interrupted or cancelled run is therefore shown through its tasks.toml statuses, as if nothing had happened. Nothing else outside tests reads the new values (also reported by w3b).

Fix: show the checkpoint status in `roko plan status`, including `interrupted` and `cancelled`.
