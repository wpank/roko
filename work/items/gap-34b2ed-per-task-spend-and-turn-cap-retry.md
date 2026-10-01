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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphTaskSpendLedger", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::TurnCapRetry", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Unlike the plan ledger it is not checkpointed' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib task_spend_and_turn_cap_retry_survive_resume"
+++

The per-task spend cap (`budget.max_task_usd` x tier multiplier, plus `max_task_retry_usd`, checked in `admit_task_budget`) counts spend in `GraphTaskSpendLedger`. The raised-cap retry after a turn-cap hit is tracked in `turn_cap_retries` (`TurnCapRetry`). Both are in-memory maps on the dispatcher and neither is checkpointed, so a resumed run starts every task's spend at zero and can repeat a turn-cap retry it already used.

Fix: persist both with the Graph checkpoint, or rebuild them from attempt records on resume.

## Notes

- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  Both are now kept in the plan's `retry-feedback.json`, beside its Graph checkpoint and scoped to the checkpoint
  run, the way gap-460230 keeps ladder standings. The file gains `spend` (micro-USD per task) and `turn_caps`
  (`TurnCapRetry` per task), both defaulted, so schema 1 still reads.
  - Every spend record site goes through `GraphTaskDispatcher::record_task_spend`, which records it and keeps the
    task's running total.
  - `keep_turn_cap_retry` and `take_turn_cap_retry` keep the file in step with the retry map.
  - `attach_retry_feedback` restores both on resume. Restored spend goes into a separate `earlier` map that counts
    toward the task's ceiling but not toward the process spend that `budget.max_daily_usd` measures, which
    `costs.jsonl` already holds.
  - A passing task's entries are cleared with its feedback.
  - Test: `task_spend_and_turn_cap_retry_survive_resume`.
  - Still in memory only: `timeout_retries` (the escalated-timeout retry), which this item does not name.
