+++
id = "gap-85d176"
kind = "gap"
title = "PK44 M2 loop-liveness: DryRunPlanner over Dispatcher::plan (probes P4 and P5) (+6 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 44
size = "L"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "73a96794e"
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
command = "test -f crates/roko-cli/tests/loop_audit_census_run.rs && grep -qw 'fn fixture_run_logs_arms_before_plan_and_measures_exposure' crates/roko-cli/tests/loop_audit_census_run.rs && cargo test -p roko-cli --test loop_audit_census_run fixture_run_logs_arms_before_plan_and_measures_exposure"

[[verify]]
command = "grep -rqw 'fn learn_loops_lists_one_state_per_registered_loop' crates/roko-serve/src/ && cargo test -p roko-serve learn_loops_lists_one_state_per_registered_loop"

[[verify]]
command = "grep -rqw 'fn loop_fault_route_rejects_non_admin_and_long_ttl' crates/roko-serve/src/ && cargo test -p roko-serve --features fault-injection loop_fault_route_rejects_non_admin_and_long_ttl"

[[verify]]
command = "grep -rqw 'fn learn_loops_canary_and_fault_subcommands' crates/roko-cli/src/commands/ && cargo test -p roko-cli --lib learn_loops_canary_and_fault_subcommands"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T19:23:34Z"
commit = "73a96794e"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T17:08:26Z"
forced = false
evidence = "Gate 9b (work/backlog-batch-9b, merged into main as 73a96794e): cargo check --workspace --tests, roko-cli and roko-serve with fault-injection, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 5,854 tests over roko-cli, -learn and -serve, roko-cli bin 436 passed, the golden-path canaries and the loop-audit census run pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, 159 fault-injection lib tests pass; every [[verify]] passes. PK44 6/7: the dry-run planner, fault read sites behind fault-injection, the census run over 200 chains (as four 50-task plans, 48bc6ac41), the learn-loops routes (documented in OpenAPI at the gate, 1fa67a39b), admin canary and fault routes, roko learn loops canary|fault; 5130 is gap-a13544."
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

## Progress

Worker w3 (no cargo; Rust checks deferred to the batch gate), base `552112110`, branch `work/gap-85d176`:

- 5128: implemented at `485635253`. `dispatch::dry_run_planner::DispatchPlanner` over `Dispatcher::plan` (P4: the prompt holds `CANARY-<nonce>`, the routed model and source). P5 is not offered (`execute_capped` stays `None`), so canaries run dry.
- 5129: implemented at `1742abfa8`. Read sites at the knowledge and playbook readers, the route and the row builders, through `faults::active` (a const `None` without roko-cli's new `fault-injection` feature). 5133 later gates dry-run kinds to `faults::dry_run`.
- 5130: blocked. E1's C2 needs fault signatures the measured census lacks: no stale read status on rows (STALE), no degeneracy measure (DEGENERATE), and a CUT reader leaves 5125's rows ineligible, so it shows as `no_opportunity`. An integration test also needs public pure row builders (`content_audit` is private). Each needs a design choice first.
- 5131: implemented at `09bf91991`. `tests/loop_audit_census_run.rs`: 200 chains through the binary; knowledge arms near 80/20 with SRM e-value < 20, ordering on every row, receipts, and ε > 0 for L-know and L-play.
- 5132: implemented at `c9161708b`. `GET /api/learn/loops`, `/{id}`, `/{id}/decisions`, and the showcase aliases (`routes/learning/loops.rs`); `LoopAuditor::reason`.
- 5133: implemented at `f30e320ee`. Admin canary route (injected `LoopCanaryRunner`, set by `roko serve` and `roko up`), and feature-gated fault and break routes (403, 422, 409). Decision 5101 §9.10 is applied: the Dockerfile builds `fault-injection`, and dry-run kinds reach only `faults::dry_run` reads.
- 5134: implemented at `f3606abd3`. `roko learn loops canary <id>` and `fault <id> <kind>` (exit 2 without the feature or `ROKO_FAULTS=1`). The fault form shows the canary's first failing probe; time to detection waits on 5130.
- gap-135821 (5127's L-route canary): implemented at `318b1a0c3`. A canary-only `CascadeRouter` preference read inside `canary_scope`, `loop_canary::RouteCanary`, and L-route in `DryCanaryRunner`.
- 2026-10-03 (coordinator, gate 9b): 5130 (E1, the fault replay) and its verify left this item for gap-a13544: the census lacks the stale, degenerate and cut signatures E1 needs. The census-run test (5131) runs as four 50-task plans under the 64-task cap (48bc6ac41).
