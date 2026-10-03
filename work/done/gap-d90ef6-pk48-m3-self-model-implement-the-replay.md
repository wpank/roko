+++
id = "gap-d90ef6"
kind = "gap"
title = "PK48 M3 self-model: Implement the replay baselines, including the production model ladder as it runs today (+5 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 48
size = "L"
subsystem = ["roko-learn/self_model"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "823f2cfca"
source = "tmp/backlog/2026-10-02-complete-and-wire PK48"
anchors = ["crates/roko-learn/src", "crates/roko-learn/tests"]
lane = "rust-cold"
parent = "spec-abbc62"
links = { depends_on = ["gap-1f4bec", "gap-62e1b9"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn production_ladder_baseline_follows_ladder_rules' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::baselines"

[[verify]]
command = "grep -rqw 'fn lcb_aci_failure_rate_tracks_target' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::policy"

[[verify]]
command = "grep -rqw 'fn predicted_start_rung_skips_a_rung_below_break_even' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::cascade"

[[verify]]
command = "grep -rqw 'fn replaying_the_oracle_reproduces_matrix_oracle_cpr' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::replay"

[[verify]]
command = "grep -rqw 'fn ips_and_dr_cover_the_known_value' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::ope"

[[verify]]
command = "grep -rqw 'fn circuit_breaker_drops_to_shadow_on_worse_than_base_rate' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::gate"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T06:07:26Z"
commit = "823f2cfca"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T04:13:34Z"
forced = false
evidence = "Gate 7a (work/backlog-batch-7a, merged into main as 823f2cfca): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib over roko-cli, -compose, -core, -gate, -learn and -runtime (8,225 tests; two calibration-gate fixture failures fixed in 1952f3e82), roko-cli bin 429 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, the parked features' lib tests pass (cognitive-clock 286, cross-cut-functors 556, spc 699, active-inference 1,297), the default roko-cli tree still has no roko-chain or Alloy provider graph and --features chain builds, ViabilityBench suite 652 passed after the gate's path fix; every [[verify]] passes. PK48 6/6; the gate's fixture fix spreads each group's passes so the breaker's five-outcome bins stay calibrated (1952f3e82)."
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK48, slice 61xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6117 | M | p2 | Implement the replay baselines, including the production model ladder as it runs today | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6117-implement-the-replay-baselines-including-the-production.md` |
| 2 | 6118 | M | p2 | Implement policy (a): the cheapest arm whose lower bound meets an adaptive-conformal target | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6118-implement-policy-a-the-cheapest-arm-whose-lower-bound-meets.md` |
| 3 | 6119 | M | p2 | Implement policy (b) on the model ladder: predicted start rung, post-failure step and post-pass depth request | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6119-implement-policy-b-on-the-model-ladder-predicted-start-rung.md` |
| 4 | 6120 | M | p2 | Replay routing policies prequentially on the benchmark's run records | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6120-replay-routing-policies-prequentially-on-the-benchmark-s-run.md` |
| 5 | 6121 | S | p3 | Add IPS and doubly robust estimates for logged route decisions | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6121-add-ips-and-doubly-robust-estimates-for-logged-route.md` |
| 6 | 6122 | M | p2 | Gate promotion on calibration: rolling EVs, an eligibility report and an automatic fall back to shadow | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6122-gate-promotion-on-calibration-rolling-evs-an-eligibility.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-learn/src/self_model/baselines.rs`, `crates/roko-learn/src/self_model/cascade.rs`, `crates/roko-learn/src/self_model/gate.rs`, `crates/roko-learn/src/self_model/ope.rs`, `crates/roko-learn/src/self_model/policy.rs`, `crates/roko-learn/src/self_model/replay.rs`, `crates/roko-learn/tests/fixtures/self_model/matrix/**`.

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

- Waits on: PK40 (gap-1f4bec), PK47 (gap-62e1b9).
- Suggested model: opus.

## Progress

- 6117: implemented at b2cdf8cd0 (follow-up fix at the commit after b135dc61c); cargo verification deferred to the batch gate.
- 6118: implemented at 44b9973af; cargo verification deferred to the batch gate.
- 6119: implemented at a47802ffb; cargo verification deferred to the batch gate.
- 6120: implemented at dbba57263; cargo verification deferred to the batch gate.
- 6121: implemented at 433105270 (DR calls S03.T5's AIPW estimator in loop_audit, which has landed); cargo verification deferred to the batch gate.
- 6122: implemented at b135dc61c; cargo verification deferred to the batch gate.
