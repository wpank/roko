+++
id = "gap-34b2ed"
kind = "gap"
title = "Per-task spend and turn-cap retry state are kept in memory only and reset on resume"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch", "roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphTaskSpendLedger", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::TurnCapRetry", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Unlike the plan ledger it is not checkpointed' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib task_spend_and_turn_cap_retry_survive_resume"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:35Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:00Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
