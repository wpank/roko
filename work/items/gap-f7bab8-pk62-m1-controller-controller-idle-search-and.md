+++
id = "gap-f7bab8"
kind = "gap"
title = "PK62 M1 controller: Controller: IDLE, SEARCH and HOLD with Thompson plus Ashby moves, dwell, rollback and… (+9 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 62
size = "L"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK62"
anchors = ["crates/roko-core/src/lib.rs"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = ["gap-ac2611", "gap-62e1b9", "gap-8b67de"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rollback_on_collateral' crates/roko-learn/ && grep -rqw 'fn hold_after_n_max' crates/roko-learn/ && grep -rqw 'fn no_move_within_dwell' crates/roko-learn/ && cargo test -p roko-learn rollback_on_collateral && cargo test -p roko-learn hold_after_n_max && cargo test -p roko-learn no_move_within_dwell"

[[verify]]
command = "grep -rqw 'fn lkg_restored_after_abnormal_exit' crates/roko-learn/ && cargo test -p roko-learn lkg_restored_after_abnormal_exit"

[[verify]]
command = "grep -rqw 'fn controller_rows_round_trip_s06_examples' crates/roko-learn/ && cargo test -p roko-learn controller_rows_round_trip_s06_examples"

[[verify]]
command = "test -f crates/roko-learn/examples/homeostat_replay.rs && cargo run -q -p roko-learn --example homeostat_replay -- --stream synthetic:model_swap@20 --mode shadow --seed 7 | grep -q '\"param\":\"tier_floor'"

[[verify]]
command = "grep -rqw 'fn saso_matches_hand_computed_fixture' crates/roko-learn/ && cargo test -p roko-learn saso_matches_hand_computed_fixture"

[[verify]]
command = "grep -rqw 'fn replay_arms_share_common_random_numbers' crates/roko-learn/ && cargo test -p roko-learn replay_arms_share_common_random_numbers"

[[verify]]
command = "grep -rqw 'fn holdout_rows_always_theta0' crates/roko-learn/ && cargo test -p roko-learn holdout_rows_always_theta0"

[[verify]]
command = "grep -rqw 'fn disturbance_kinds_match_benchmark_driver' crates/roko-core/ && grep -rqw 'fn controller_never_reads_ground_truth' crates/roko-learn/ && cargo test -p roko-core disturbance_kinds_match_benchmark_driver && cargo test -p roko-learn controller_never_reads_ground_truth"

[[verify]]
command = "grep -rqw 'fn canonical_disturbances_meet_c1_c2_c5_c7' crates/roko-learn/ && cargo test -p roko-learn --test homeostat_disturbances canonical_disturbances_meet_c1_c2_c5_c7"

[[verify]]
command = "grep -rqw 'fn m3_prior_weight_follows_ece' crates/roko-learn/ && cargo test -p roko-learn m3_prior_weight_follows_ece"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK62, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8112 | M | p2 | Controller: IDLE, SEARCH and HOLD with Thompson plus Ashby moves, dwell, rollback and relaxation | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8112-controller-idle-search-hold-with-rollback.md` |
| 2 | 8113 | S | p2 | Commit the controller's θ through the guarded commit and restore the last-known-good at start | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8113-commit-controller-theta-through-guarded-commit.md` |
| 3 | 8114 | S | p2 | A-CTL controller record types that round-trip S06's example payloads | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8114-a-ctl-controller-record-types.md` |
| 4 | 8115 | S | p2 | Shadow replay example homeostat_replay over historical and synthetic streams (first slice) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8115-shadow-replay-example-homeostat-replay.md` |
| 5 | 8116 | S | p2 | SASO step-response metrics and IAE | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8116-saso-step-response-metrics-and-iae.md` |
| 6 | 8117 | M | p2 | Full-information replay evaluator for arms A0–A5, A3-gated and A3-mis with common random numbers | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8117-replay-evaluator-arms-a0-a5-common-random-numbers.md` |
| 7 | 8118 | S | p2 | Fixed 10% harness_policy holdout through telemetry::assign; holdout rows always run θ₀ | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8118-harness-policy-holdout-through-telemetry-assign.md` |
| 8 | 8119 | M | p2 | DisturbanceSpec, between-run overlays and a hidden ground-truth writer sharing disturb.py's kind list | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8119-disturbance-spec-overlays-and-hidden-ground-truth.md` |
| 9 | 8120 | S | p2 | Disturbance acceptance suite at $0: detection, first move, HOLD on unregulable kinds, P1 under budget_cut | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8120-disturbance-acceptance-suite-at-zero-cost.md` |
| 10 | 8121 | S | p2 | Seed controller move priors from the self-model; feedforward pre-arm off by default | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8121-m3-seeded-move-priors-and-feedforward-prearm.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-core/src/disturbance.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-learn/examples/homeostat_replay.rs`, `crates/roko-learn/src/homeostasis/controller.rs`, `crates/roko-learn/src/homeostasis/holdout.rs`, `crates/roko-learn/src/homeostasis/ledger.rs`, `crates/roko-learn/src/homeostasis/lkg.rs`, `crates/roko-learn/src/homeostasis/priors.rs`, `crates/roko-learn/src/homeostasis/replay.rs`, `crates/roko-learn/src/homeostasis/saso.rs`, `crates/roko-learn/src/homeostasis/streams.rs`, `crates/roko-learn/tests/homeostasis_ground_truth.rs`, `crates/roko-learn/tests/homeostat_disturbances.rs`.

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

- Waits on: PK34 (gap-ac2611), PK47 (gap-62e1b9), PK61 (gap-8b67de).
- Suggested model: opus.

## Progress

Implemented on `work/gap-f7bab8`; cargo verification deferred to the batch gate.

- 8112: implemented at 741300094
- 8113: implemented at 57138c685
- 8114: implemented at 9821b0351
- 8115: implemented at 4eeae3122
- 8116: implemented at dcb437e09
- 8117: implemented at a5c340cfd
- 8118: implemented at ea70ee4d6
- 8119: implemented at 986fff88f
- 8120: implemented at 75494e7f3 (synthetic regimes moved to S06 C1's steps and `AUX_SHARE` to 0.15 in the same commit)
- 8121: implemented at 508ef3b22 (adds `Controller::set_detector_tuning` for the pre-arm)
- Lint follow-up: 47aedae8f splits first doc paragraphs over 200 characters (8115, 8117, 8118, 8119).
- model_swap vs convention_flip (S06 C2): with the catalog-sign prior both get B1 floor up first, which is in model_swap's row only. The controller's doc records it; 8120 checks convention_flip's first guided move against the union of the two rows; 8121's test shows calibrated M3 priors picking a convention_flip counter (B2 retries) first. budget_cut's signature (E2 high, budget ends) forms only after E1 confirms, so 8120 checks its row over the episode.
- S06 A1 for `--seed 7`: an emulation of rand_chacha gives the first decision draw u = 0.158 < `random_step_prob` 0.2, so the first change is an Ashby step; the next decision (u = 0.99) is the directed B1 floor raise the verify greps for. Detection by resolution 30 for seed 7 is unverified.
