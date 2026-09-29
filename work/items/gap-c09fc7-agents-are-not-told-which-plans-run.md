+++
id = "gap-c09fc7"
kind = "gap"
title = "Agents are not told which plans run beside them, so their ad-hoc builds can see other plans' half-finished edits"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/dispatch/prompt_builder", "roko-cli/graph_execution/plan_set"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_execution/plan_set.rs"]
links = { depends_on = [], blocks = [], related = ["gap-0001a1", "gap-c89b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'concurrent_plans' crates/roko-cli/src/dispatch/prompt_builder.rs && cargo test -p roko-cli --lib prompt_names_concurrent_plans"
+++

With `max_parallel_plans` above 1, independent plans run at the same time in the operator's single working tree. Verify gates are serialized by the compile lock, but an agent's own ad-hoc `cargo build` or `cargo test` can compile another plan's half-finished edits and fail for reasons unrelated to its task. The prompt has no section telling the agent that other plans are running.

Fix: tell agents which plans and areas are in flight and to scope their builds to their own crate, or isolate plans in worktrees (gap-0001a1).
