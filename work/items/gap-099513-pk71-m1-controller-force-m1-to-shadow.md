+++
id = "gap-099513"
kind = "gap"
title = "PK71 M1 controller: Force M1 to shadow when M2 demotes L-M1, and allow B4 'on' moves only for live loops (+9 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 71
size = "L"
subsystem = ["roko-cli/runtime_feedback"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK71"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/graph_execution/agent_slots.rs", "crates/roko-cli/src/graph_task_dispatch/budget.rs", "crates/roko-cli/src/runtime_feedback/knowledge.rs", "crates/roko-core/src/config/learning.rs", "crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-learn/src/lib.rs", "crates/roko-learn/src/model_call_feedback.rs", "crates/roko-neuro/src/knowledge_store/mod.rs", "crates/roko-neuro/src/lib.rs", "crates/roko-serve/src/routes/learning/mod.rs"]
lane = "rust-hot"
parent = "spec-635697"
links = { depends_on = ["gap-198c9c", "gap-cc5051", "gap-b5caf3", "gap-894977", "gap-2b3c1b", "gap-c1d920", "gap-85d176", "gap-8b67de", "gap-f7bab8", "gap-eb39c1", "gap-4cbd80"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn demoted_l_m1_forces_shadow' crates/roko-cli/ && cargo test -p roko-cli demoted_l_m1_forces_shadow"

[[verify]]
command = "grep -rqw 'fn param_change_takes_effect_on_next_attempt' crates/roko-cli/ && cargo test -p roko-cli --test homeostat_on_mode param_change_takes_effect_on_next_attempt"

[[verify]]
command = "grep -rqw 'fn homeostasis_dashboard_events_serialize' crates/roko-core/ && cargo test -p roko-core homeostasis_dashboard_events_serialize"

[[verify]]
command = "grep -rqw 'fn homeostasis_route_reports_mode_state_theta_lkg' crates/roko-serve/ && cargo test -p roko-serve homeostasis_route_reports_mode_state_theta_lkg"

[[verify]]
command = "grep -rqw 'fn learn_homeostasis_status_reads_controller_state' crates/roko-cli/ && cargo test -p roko-cli learn_homeostasis_status_reads_controller_state"

[[verify]]
command = "grep -rqw 'fn slots_resize_without_dropping_leases' crates/roko-cli/ && cargo test -p roko-cli slots_resize_without_dropping_leases"

[[verify]]
command = "grep -rqw 'fn in_run_budget_cut_applies_from_position' crates/roko-cli/ && cargo test -p roko-cli in_run_budget_cut_applies_from_position"

[[verify]]
command = "grep -rqw 'fn router_commit_rolls_back_on_held_out_regression' crates/roko-learn/ && cargo test -p roko-learn router_commit_rolls_back_on_held_out_regression"

[[verify]]
command = "grep -rqw 'fn knowledge_batch_rolled_back_when_anchor_query_regresses' crates/roko-neuro/ && cargo test -p roko-neuro knowledge_batch_rolled_back_when_anchor_query_regresses"

[[verify]]
command = "grep -rqw 'fn run_end_commits_router_and_knowledge_through_guard' crates/roko-cli/ && cargo test -p roko-cli run_end_commits_router_and_knowledge_through_guard"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK71, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8128 | S | p2 | Force M1 to shadow when M2 demotes L-M1, and allow B4 'on' moves only for live loops | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8128-force-shadow-when-m2-demotes-l-m1.md` |
| 2 | 8129 | S | p2 | On-mode acceptance on the fixture plan: a change reaches the next attempt, holdout rows keep θ₀ | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8129-on-mode-acceptance-on-the-fixture-plan.md` |
| 3 | 8130 | S | p2 | DashboardEvent variants ev.update and m1.episode, published through StateHub | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8130-dashboard-events-ev-update-and-m1-episode.md` |
| 4 | 8131 | M | p2 | Serve routes for the homeostat: state, essential variables, episodes, admin mode and ack | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8131-serve-routes-for-the-homeostat.md` |
| 5 | 8133 | S | p2 | roko learn homeostasis status and replay (read-only) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8133-roko-learn-homeostasis-status-and-replay.md` |
| 6 | 8134 | M | p2 | B5 actuator: agent slots that resize live without dropping held leases (phase 2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8134-b5-agent-slots-resize-live-phase-2.md` |
| 7 | 8135 | S | p2 | In-run budget_cut through an S5 handle read by admit_task_budget (phase 2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8135-in-run-budget-cut-through-s5-handle-phase-2.md` |
| 8 | 8136 | M | p2 | Guarded commit for the cascade router: held-out and anchor checks before its snapshot is replaced | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8136-guarded-commit-for-the-cascade-router.md` |
| 9 | 8137 | M | p2 | Guarded commit for the knowledge store: a run's new entries commit as one batch or roll back | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8137-guarded-commit-for-the-knowledge-store.md` |
| 10 | 8138 | S | p2 | Run-end hook: commit router and knowledge changes through the guard, in observe or enforce mode | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8138-run-end-guarded-commit-hook-observe-or-enforce.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/learn_homeostasis.rs`, `crates/roko-cli/src/graph_execution/agent_slots.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/budget.rs`, `crates/roko-cli/src/runtime_feedback/homeostasis.rs`, `crates/roko-cli/src/runtime_feedback/knowledge.rs`, `crates/roko-cli/tests/homeostat_on_mode.rs`, `crates/roko-core/src/config/learning.rs`, `crates/roko-core/src/dashboard_snapshot.rs`, `crates/roko-learn/src/lib.rs`, `crates/roko-learn/src/model_call_feedback.rs`, `crates/roko-learn/src/router_commit.rs`, `crates/roko-neuro/src/knowledge_store/commit.rs`, `crates/roko-neuro/src/knowledge_store/mod.rs`, `crates/roko-neuro/src/lib.rs`, `crates/roko-serve/src/routes/learning/homeostasis.rs`, `crates/roko-serve/src/routes/learning/mod.rs`.

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

- Waits on: PK04 (gap-198c9c), PK09 (gap-cc5051), PK32 (gap-b5caf3), PK38 (gap-894977), PK42 (gap-2b3c1b), PK43 (gap-c1d920), PK44 (gap-85d176), PK61 (gap-8b67de), PK62 (gap-f7bab8), PK63 (gap-eb39c1), PK65 (gap-4cbd80).
- Suggested model: opus.
