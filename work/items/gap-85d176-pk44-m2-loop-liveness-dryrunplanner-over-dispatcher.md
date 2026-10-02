+++
id = "gap-85d176"
kind = "gap"
title = "PK44 M2 loop-liveness: DryRunPlanner over Dispatcher::plan (probes P4 and P5) (+6 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 44
size = "L"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK44"
anchors = ["crates/roko-cli/Cargo.toml", "crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/commands/server.rs", "crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-serve/Cargo.toml", "crates/roko-serve/src/routes/learning/mod.rs", "crates/roko-serve/src/state.rs"]
lane = "rust-hot"
parent = "spec-c6e21b"
links = { depends_on = ["gap-b5caf3", "gap-894977", "gap-1f4bec", "gap-c2b1a3", "gap-2b3c1b", "gap-c1d920"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dry_run_canary_reaches_p4_without_learned_writes' crates/roko-cli/src/dispatch/ && cargo test -p roko-cli --lib dry_run_canary_reaches_p4_without_learned_writes"

[[verify]]
command = "grep -rqw 'fn injected_cut_empties_knowledge_section' crates/roko-cli/src/ && cargo test -p roko-cli --lib --features fault-injection injected_cut_empties_knowledge_section"

[[verify]]
command = "test -f crates/roko-cli/tests/loop_audit_faults.rs && grep -qw 'fn structural_faults_detected_and_localized' crates/roko-cli/tests/loop_audit_faults.rs && cargo test -p roko-cli --features fault-injection --test loop_audit_faults structural_faults_detected_and_localized"

[[verify]]
command = "test -f crates/roko-cli/tests/loop_audit_census_run.rs && grep -qw 'fn fixture_run_logs_arms_before_plan_and_measures_exposure' crates/roko-cli/tests/loop_audit_census_run.rs && cargo test -p roko-cli --test loop_audit_census_run fixture_run_logs_arms_before_plan_and_measures_exposure"

[[verify]]
command = "grep -rqw 'fn learn_loops_lists_one_state_per_registered_loop' crates/roko-serve/src/ && cargo test -p roko-serve learn_loops_lists_one_state_per_registered_loop"

[[verify]]
command = "grep -rqw 'fn loop_fault_route_rejects_non_admin_and_long_ttl' crates/roko-serve/src/ && cargo test -p roko-serve --features fault-injection loop_fault_route_rejects_non_admin_and_long_ttl"

[[verify]]
command = "grep -rqw 'fn learn_loops_canary_and_fault_subcommands' crates/roko-cli/src/commands/ && cargo test -p roko-cli --lib learn_loops_canary_and_fault_subcommands"
+++

## Problem

This package delivers 7 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK44, slice 51xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5128 | S | p2 | DryRunPlanner over Dispatcher::plan (probes P4 and P5) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5128-dry-run-planner-over-dispatcher-plan.md` |
| 2 | 5129 | M | p2 | Fault read sites at the knowledge, playbook and route readers (fault-injection builds only) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5129-fault-read-sites-at-knowledge-and-route-readers.md` |
| 3 | 5130 | M | p2 | E1 structural fault replay over 500 dry-run contexts (C2, C3, A4) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5130-e1-structural-fault-replay-500-dry-run-contexts.md` |
| 4 | 5131 | M | p2 | A3 acceptance: a fixture run logs arms before plan() and the census measures ε | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5131-a3-fixture-run-logs-arms-and-measures-exposure.md` |
| 5 | 5132 | M | p2 | Serve the loop audit: GET /api/learn/loops routes and loop SSE | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5132-serve-learn-loops-routes-and-loop-sse.md` |
| 6 | 5133 | M | p3 | Admin canary and feature-gated fault routes on roko serve | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5133-serve-admin-canary-and-fault-routes.md` |
| 7 | 5134 | S | p3 | `roko learn loops canary <id>` and `roko learn loops fault <id> <kind>` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5134-roko-learn-loops-canary-and-fault-subcommands.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/Cargo.toml`, `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/server.rs`, `crates/roko-cli/src/dispatch/dry_run_planner.rs`, `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/tests/loop_audit_census_run.rs`, `crates/roko-cli/tests/loop_audit_faults.rs`, `crates/roko-serve/Cargo.toml`, `crates/roko-serve/src/routes/learning/loops.rs`, `crates/roko-serve/src/routes/learning/mod.rs`, `crates/roko-serve/src/state.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK32 (gap-b5caf3), PK38 (gap-894977), PK40 (gap-1f4bec), PK41 (gap-c2b1a3), PK42 (gap-2b3c1b), PK43 (gap-c1d920).
- Suggested model: opus.
