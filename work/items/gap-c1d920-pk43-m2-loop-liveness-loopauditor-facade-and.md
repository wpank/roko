+++
id = "gap-c1d920"
kind = "gap"
title = "PK43 M2 loop-liveness: LoopAuditor facade and the [learning.audit] config section (+5 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 43
size = "L"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK43"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_execution/mod.rs", "crates/roko-cli/src/lib.rs", "crates/roko-core/src/config/learning.rs", "crates/roko-learn/src/routing_log.rs", "crates/roko-learn/src/telemetry/records.rs"]
lane = "rust-hot"
parent = "spec-c6e21b"
links = { depends_on = ["gap-cc5051", "gap-f61823", "gap-b5caf3", "gap-ac2611", "gap-943046", "gap-894977", "gap-1f4bec", "gap-2b3c1b"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'audit' crates/roko-core/src/config/learning.rs && grep -rqw 'fn auditor_serves_schedule_and_honours_enforce' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn auditor_serves_schedule_and_honours_enforce && grep -rqw 'fn learning_audit_config_defaults_match_s03' crates/roko-core/src/config/ && cargo test -p roko-core learning_audit_config_defaults_match_s03"

[[verify]]
command = "grep -rqw 'fn census_measures_exposure_from_decision_rows' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn census_measures_exposure_from_decision_rows"

[[verify]]
command = "grep -rqw 'fn route_decision_logs_arm_before_plan_and_executed_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib route_decision_logs_arm_before_plan_and_executed_model"

[[verify]]
command = "grep -rqw 'fn graph_decisions_log_arm_before_plan_and_receipt' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_decisions_log_arm_before_plan_and_receipt"

[[verify]]
command = "grep -rqw 'fn audit_tick_appends_health_rows_and_publishes_events' crates/roko-cli/src/graph_execution/ && cargo test -p roko-cli --lib audit_tick_appends_health_rows_and_publishes_events"

[[verify]]
command = "grep -rqw 'fn canary_writers_reach_real_readers' crates/roko-cli/src/ && cargo test -p roko-cli --lib canary_writers_reach_real_readers"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK43, slice 51xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5122 | M | p2 | LoopAuditor facade and the [learning.audit] config section | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5122-loop-auditor-facade-and-learning-audit-config.md` |
| 2 | 5123 | M | p2 | Census measured mode over per-run decision and exposure rows | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5123-census-measured-mode-over-decision-rows.md` |
| 3 | 5124 | M | p2 | Route decision rows carry the A-DEC fields and an executed-model receipt | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5124-route-decision-rows-a-dec-fields-and-receipt.md` |
| 4 | 5125 | M | p2 | Knowledge and playbook decision rows carry A-DEC fields and rendered-section receipts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5125-content-decision-rows-a-dec-and-section-receipts.md` |
| 5 | 5126 | M | p2 | Run the loop auditor at plan-run end: ledger rows and StateHub events | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5126-audit-tick-at-plan-run-end.md` |
| 6 | 5127 | M | p2 | Production canary writers for L-know, L-play and L-route | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5127-production-canary-writers-know-play-route.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/graph_execution/loop_audit.rs`, `crates/roko-cli/src/graph_execution/mod.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-cli/src/loop_canary.rs`, `crates/roko-core/src/config/learning.rs`, `crates/roko-learn/src/loop_audit/census.rs`, `crates/roko-learn/src/loop_audit/mod.rs`, `crates/roko-learn/src/routing_log.rs`, `crates/roko-learn/src/telemetry/records.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK09 (gap-cc5051), PK10 (gap-f61823), PK32 (gap-b5caf3), PK34 (gap-ac2611), PK35 (gap-943046), PK38 (gap-894977), PK40 (gap-1f4bec), PK42 (gap-2b3c1b).
- Suggested model: opus.
