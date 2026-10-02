+++
id = "gap-8b67de"
kind = "gap"
title = "PK61 M1 controller: HarnessParams: the M1 knob surface with notch ladders, θ₀, an atomic handle and a… (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 61
size = "L"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK61"
anchors = ["crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-learn/Cargo.toml", "crates/roko-learn/src/lib.rs"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladders_roundtrip_and_digest_stable' crates/roko-core/ && cargo test -p roko-core ladders_roundtrip_and_digest_stable"

[[verify]]
command = "grep -rqw 'fn homeostasis_config_defaults_and_validation' crates/roko-core/ && cargo test -p roko-core homeostasis_config_defaults_and_validation"

[[verify]]
command = "grep -rqw 'fn ev_fold_dedupes_and_counts_only_passed' crates/roko-learn/ && cargo test -p roko-learn ev_fold_dedupes_and_counts_only_passed"

[[verify]]
command = "grep -rqw 'fn e2_unmeasurable_on_unknown_cost' crates/roko-learn/ && grep -rqw 'fn drive_and_bands_match_hand_computed_fixture' crates/roko-learn/ && cargo test -p roko-learn e2_unmeasurable_on_unknown_cost && cargo test -p roko-learn drive_and_bands_match_hand_computed_fixture"

[[verify]]
command = "grep -q 'roko-gate' crates/roko-learn/Cargo.toml && grep -rqw 'fn cusum_arl0_at_least_100_in_bounds' crates/roko-learn/ && grep -rqw 'fn step_detected_within_10' crates/roko-learn/ && cargo test -p roko-learn cusum_arl0_at_least_100_in_bounds && cargo test -p roko-learn step_detected_within_10"

[[verify]]
command = "grep -rqw 'fn validator_never_admits_widening_removal_or_ceiling_raise' crates/roko-learn/ && cargo test -p roko-learn validator_never_admits_widening_removal_or_ceiling_raise"

[[verify]]
command = "grep -rqw 'fn guarded_commit_rolls_back_on_failed_check' crates/roko-learn/ && grep -rqw 'fn lkg_stack_survives_restart' crates/roko-learn/ && cargo test -p roko-learn guarded_commit_rolls_back_on_failed_check && cargo test -p roko-learn lkg_stack_survives_restart"

[[verify]]
command = "grep -rqw 'fn first_move_lies_in_requisite_variety_row' crates/roko-learn/ && cargo test -p roko-learn first_move_lies_in_requisite_variety_row"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK61, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8104 | M | p2 | HarnessParams: the M1 knob surface with notch ladders, θ₀, an atomic handle and a stable digest | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8104-harness-params-knob-surface-handle-and-digest.md` |
| 2 | 8105 | S | p2 | `[homeostasis]` config section: mode off, shadow or on, and the controller's constants | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8105-homeostasis-config-section-and-mode.md` |
| 3 | 8106 | M | p2 | Homeostasis module skeleton and the task-resolution fold over verdict records and historical logs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8106-homeostasis-skeleton-and-task-resolution-fold.md` |
| 4 | 8107 | M | p2 | Essential-variable estimators, Schmitt bands and the drive D | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8107-ev-estimators-schmitt-bands-and-drive.md` |
| 5 | 8108 | S | p2 | Change detectors over roko-gate's SPC charts with 2-of-2 confirmation and an ARL₀ simulation | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8108-change-detectors-over-spc-with-confirmation.md` |
| 6 | 8109 | M | p2 | SafetyBox validator and the read-only S5 viability policy loader | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8109-safetybox-validator-and-s5-policy-loader.md` |
| 7 | 8110 | M | p2 | Guarded-commit primitive: versioned last-known-good snapshots, checks and rollback | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8110-guarded-commit-primitive-lkg-checks-rollback.md` |
| 8 | 8111 | S | p2 | Move catalog and requisite-variety matrix for the regulable disturbances | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8111-move-catalog-and-requisite-variety-matrix.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-core/src/config/harness_params.rs`, `crates/roko-core/src/config/homeostasis.rs`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-learn/Cargo.toml`, `crates/roko-learn/src/guarded_commit.rs`, `crates/roko-learn/src/homeostasis/catalog.rs`, `crates/roko-learn/src/homeostasis/controller.rs`, `crates/roko-learn/src/homeostasis/coupling.rs`, `crates/roko-learn/src/homeostasis/detect.rs`, `crates/roko-learn/src/homeostasis/ev.rs`, `crates/roko-learn/src/homeostasis/holdout.rs`, `crates/roko-learn/src/homeostasis/ledger.rs`, `crates/roko-learn/src/homeostasis/lkg.rs`, `crates/roko-learn/src/homeostasis/mod.rs`, `crates/roko-learn/src/homeostasis/policy.rs`, `crates/roko-learn/src/homeostasis/priors.rs`, `crates/roko-learn/src/homeostasis/replay.rs`, `crates/roko-learn/src/homeostasis/resolution.rs`, `crates/roko-learn/src/homeostasis/saso.rs`, `crates/roko-learn/src/homeostasis/streams.rs`, `crates/roko-learn/src/lib.rs`.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

Implemented on `work/gap-8b67de` without cargo; cargo verification is deferred to the batch gate.

- 8104: implemented at 3190db0a5
- 8105: implemented at db0cf684b
- 8106: implemented at 7520b006e
- 8107: implemented at fb441f530
- 8108: implemented at 61d4e9d69 (see the first note)
- 8109: implemented at 044a5efab
- 8110: implemented at a0208283d
- 8111: implemented at e5f6d76e8

Notes for the gate and for Will:

- 8108 adds `roko-gate` to roko-learn's dependencies, as decision 8102 (point 8, S06 §9.13) chose. The edge is acyclic, but
  roko-learn is layer 2 and roko-gate layer 3, so CI's Layer Check (`roko layer-check`) reports
  `L2 roko-learn -> L3 roko-gate`. It needs Will: relabel roko-gate as layer 2 (its own dependencies are layers 1 and 2,
  and nothing at layer 2 depends on it), or move `spc.rs` into roko-core and re-export it from roko-gate. `Cargo.lock`
  already lists the new edge.
- 8108's detector thresholds come from simulation, as S06 §4.4 asks: E1 H = 1.1 and E2/E4 H = 1.2 (with two alarms within
  four observations to confirm) give a joint ARL₀ near 135 and a median confirmed delay of 8 resolutions on each C1 step.
  S06's starting values (H = 2 and 4) are single-alarm thresholds that confirm in more than 20 resolutions.
- 8105 wires `[homeostasis]` validation into `validate_invariants` as invariant 9 (a small block in `validation.rs`), so the
  loader rejects `mode = "on"` with `holdout = 0`. A zero holdout is allowed in shadow and off modes (S06 §4.8).
- 8111: model swap and convention flip show the same breach signature (E1 low) but have different requisite-variety rows,
  so their first directed move cannot lie in both rows; the test checks that the directed moves meet every regulable row,
  and that the first lies in the row wherever the signature is unique.
