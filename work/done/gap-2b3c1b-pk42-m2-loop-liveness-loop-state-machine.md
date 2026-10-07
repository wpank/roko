+++
id = "gap-2b3c1b"
kind = "gap"
title = "PK42 M2 loop-liveness: Loop state machine with the six false-demotion guards (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 42
size = "L"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "c1eb6c2c5"
source = "tmp/backlog/2026-10-02-complete-and-wire PK42"
anchors = ["crates/roko-cli/src/commands/experiment.rs", "crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-learn/Cargo.toml", "crates/roko-learn/src/model_experiment.rs", "crates/roko-learn/src/prompt_experiment.rs"]
lane = "rust-cold"
parent = "spec-c6e21b"
links = { depends_on = ["gap-cc5051", "gap-ac2611", "gap-1f4bec"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dormant_loop_never_null_or_harm' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn dormant_loop_never_null_or_harm && grep -rqw 'fn no_transition_within_dwell' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn no_transition_within_dwell && grep -rqw 'fn placebo_transition_freezes_auditor' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn placebo_transition_freezes_auditor"

[[verify]]
command = "grep -rqw 'fn loop_audit_rows_round_trip_spec_examples' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn loop_audit_rows_round_trip_spec_examples"

[[verify]]
command = "grep -q 'LoopHealth' crates/roko-core/src/dashboard_snapshot.rs && grep -rqw 'fn loop_events_serialize_with_kind' crates/roko-core/src/ && cargo test -p roko-core loop_events_serialize_with_kind"

[[verify]]
command = "test -f crates/roko-learn/examples/loop_audit_sim.rs && grep -rqw 'fn e2_family_wise_false_demotion_below_alpha' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn e2_family_wise_false_demotion_below_alpha"

[[verify]]
command = "grep -rqw 'fn prompt_experiment_aa_false_winner_rate_below_alpha' crates/roko-learn/src/prompt_experiment.rs && cargo test -p roko-learn prompt_experiment_aa_false_winner_rate_below_alpha && grep -rqw 'fn prompt_assignment_logs_propensity' crates/roko-learn/src/prompt_experiment.rs && cargo test -p roko-learn prompt_assignment_logs_propensity"

[[verify]]
command = "grep -rqw 'fn model_experiment_aa_false_winner_rate_below_alpha' crates/roko-learn/src/model_experiment.rs && cargo test -p roko-learn model_experiment_aa_false_winner_rate_below_alpha"

[[verify]]
command = "grep -rqw 'fn fault_flags_expire_and_are_invisible_to_estimators' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn --features fault-injection fault_flags_expire_and_are_invisible_to_estimators"

[[verify]]
command = "grep -rqw 'fn canary_localizes_injected_cut' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn --features fault-injection canary_localizes_injected_cut"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T07:36:25Z"
commit = "c1eb6c2c5"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T04:13:04Z"
forced = false
evidence = "Gate 7b (work/backlog-batch-7b, merged into main as c1eb6c2c5): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 12,124 tests over roko-agent, -cli, -compose, -core, -fs, -gate, -graph, -learn and -serve (one OpenAPI coverage failure fixed in 484e172fe), roko-cli bin 430 passed, the golden-path canaries pass incl. golden_path_acceptance's fixture plan (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn, roko-graph and roko-agent integration tests pass, each parked feature builds (fault-injection lib 1,321), PK79's tree and chain checks pass; every [[verify]] passes. PK42 8/8: the loop state machine and ledger, LoopHealth events, the false-demotion simulator (5117 at the gate: C4 0/10000, 95% CI 0-0.0004; legacy A/A 0.3352), anytime-valid prompt and model experiments, fault flags, the canary driver. Gate fix c9d4320d1 (one [features] table with PK80's)."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK42, slice 51xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5114 | M | p2 | Loop state machine with the six false-demotion guards | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5114-loop-state-machine-with-false-demotion-guards.md` |
| 2 | 5115 | S | p2 | Loop-audit ledger rows (A-LOOP) and their JSONL writer | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5115-loop-audit-ledger-rows-and-writer.md` |
| 3 | 5116 | S | p2 | DashboardEvent variants for loop health and loop transitions | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5116-dashboard-event-variants-for-loop-health.md` |
| 4 | 5117 | S | p2 | E2 Monte Carlo simulator: family-wise false demotions under β = 0 (C4) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5117-e2-monte-carlo-false-demotion-simulator.md` |
| 5 | 5118 | M | p2 | Prompt experiments declare winners by re-testing χ² after every outcome under UCB1, now on the Graph path | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5118-prompt-experiments-peeking-chi-square-ucb1-on-graph.md` |
| 6 | 5119 | S | p3 | Model experiments use UCB1, a DefaultHasher partition and the same peeking χ² rule | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5119-model-experiments-ucb1-defaulthasher-peeking-rule.md` |
| 7 | 5120 | M | p2 | Fault-flag registry and ground-truth writer behind a fault-injection feature | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5120-fault-flag-registry-and-ground-truth-writer.md` |
| 8 | 5121 | M | p2 | Canary driver (probes P1–P3, P6, P7) with DryRunPlanner and CanaryWriter traits | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5121-canary-driver-with-dry-run-planner-traits.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/experiment.rs`, `crates/roko-core/src/dashboard_snapshot.rs`, `crates/roko-learn/Cargo.toml`, `crates/roko-learn/examples/loop_audit_sim.rs`, `crates/roko-learn/src/loop_audit/canary.rs`, `crates/roko-learn/src/loop_audit/faults.rs`, `crates/roko-learn/src/loop_audit/ledger.rs`, `crates/roko-learn/src/loop_audit/sim.rs`, `crates/roko-learn/src/loop_audit/state.rs`, `crates/roko-learn/src/model_experiment.rs`, `crates/roko-learn/src/prompt_experiment.rs`, `crates/roko-learn/tests/legacy_rule_live.rs`.

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

- Waits on: PK09 (gap-cc5051), PK34 (gap-ac2611), PK40 (gap-1f4bec).
- Suggested model: opus.

## Progress

Worker w3 (no cargo; Rust checks deferred to the batch gate), base `f74890b4b`, branch `work/gap-2b3c1b`:

- 5114 `14de42f0c`: `loop_audit::state` holds the state machine with the six false-demotion guards (dwell, N thresholds, the ε rule first, the placebo and SRM tripwires, exempt and enforce).
- 5115 `a9afabeb9`: `loop_audit::ledger` holds the `roko.loop_audit/1` and `roko.fault/1` rows and their locked JSONL writer. S01 §5.10's three examples round-trip.
- 5116 `17359688d`: `DashboardEvent::{LoopHealth, LoopTransition}`. The match arms in `roko-serve/src/lib.rs` (a hot file) are additive only.
- 5117 `b4912a855`: E2 (`sim::e2_run`) and `examples/loop_audit_sim.rs`. The 10⁴-rep output is for the gate to produce; a Python replica predicts 0/10000 (95% CI 0–0.0004) and a legacy A/A false-winner rate of 0.3352.
- 5118 `a043c553f`: prompt experiments draw uniformly through `telemetry::assign` with a logged propensity and conclude on per-pair `DifferenceCs`. `tests/legacy_rule_live.rs` is deleted, and L-prompt-exp's `misspecified` finding is retired.
- 5119 `8488f2d1d`: model experiments use the same draws (`model_experiment` layer) and the same rule. `DefaultHasher` and UCB1 are gone, `roko experiment model show` prints draws, and L-model-exp's finding is retired.
- 5120 `d5f30aa80`: `loop_audit::faults` adds flags behind the `fault-injection` feature, with ground-truth rows. `crates/roko-learn/Cargo.toml` gains a `[features]` table.
- 5121 `88da18324`: `loop_audit::canary` adds the probe driver with frozen `CanaryWriter` and `DryRunPlanner` traits.
- 2026-10-03 (coordinator, gate 7b): task 5117's example, run at the gate (`cargo run -q -p roko-learn --release --example loop_audit_sim -- --reps 10000`, 265 s):
  `C4 family-wise false harm-demotion: 0.0000 (0/10000, K = 8; 95% CI 0.0000–0.0004)`
  `legacy check_conclusion A/A false-winner: 0.3352 (3352/10000)`
  Both match the worker's Python replica.
