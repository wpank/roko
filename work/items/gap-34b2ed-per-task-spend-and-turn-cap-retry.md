+++
id = "gap-34b2ed"
kind = "gap"
title = "Per-task spend and turn-cap retry state are kept in memory only and reset on resume"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch", "roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskSpendLedger", "crates/roko-cli/src/graph_task_dispatch.rs::TurnCapRetry", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Unlike the plan ledger it is not checkpointed' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib task_spend_and_turn_cap_retry_survive_resume"
+++

The per-task spend cap (`budget.max_task_usd` x tier multiplier, plus `max_task_retry_usd`, checked in `admit_task_budget`) counts spend in `GraphTaskSpendLedger`. The raised-cap retry after a turn-cap hit is tracked in `turn_cap_retries` (`TurnCapRetry`). Both are in-memory maps on the dispatcher and neither is checkpointed, so a resumed run starts every task's spend at zero and can repeat a turn-cap retry it already used.

Fix: persist both with the Graph checkpoint, or rebuild them from attempt records on resume.
