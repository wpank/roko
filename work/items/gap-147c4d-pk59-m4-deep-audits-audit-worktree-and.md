+++
id = "gap-147c4d"
kind = "gap"
title = "PK59 M4 deep audits: Audit worktree and the A2 clean re-run, off the critical path (S05 task 8, part 1) (+8 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 59
size = "L"
subsystem = ["roko-cli/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK59"
anchors = ["crates/roko-cli/src/graph_task_dispatch/judge_step.rs", "crates/roko-cli/src/lib.rs", "crates/roko-cli/src/runner/gate_adapter.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-gate/src/judge_calibration.rs", "crates/roko-gate/src/llm_judge_gate.rs", "crates/roko-learn/src/gate_gaming.rs"]
lane = "rust-cold"
parent = "spec-c3abc8"
links = { depends_on = ["gap-dff960", "gap-f0a7ee"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_rerun_restores_visible_tests_and_runs_three_times' crates/roko-cli/src/ && cargo test -p roko-cli --lib audit_rerun_restores_visible_tests_and_runs_three_times"

[[verify]]
command = "grep -rqw 'fn a_planted_test_detection_yields_an_audit_result_with_an_a1_finding' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_planted_test_detection_yields_an_audit_result_with_an_a1_finding"

[[verify]]
command = "grep -rqw 'fn b1_suite_from_another_family_is_validated_before_use' crates/roko-cli/src/ && cargo test -p roko-cli --lib b1_suite_from_another_family_is_validated_before_use"

[[verify]]
command = "grep -rqw 'fn a_surviving_extreme_mutant_marks_a_weak_oracle' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_surviving_extreme_mutant_marks_a_weak_oracle"

[[verify]]
command = "grep -qw 'attempt_key' crates/roko-gate/src/judge_calibration.rs && grep -rqw 'fn judge_calibration_rows_carry_the_attempt_key' crates/roko-cli/src/ && cargo test -p roko-cli --lib judge_calibration_rows_carry_the_attempt_key"

[[verify]]
command = "grep -rqw 'fn b3_review_uses_another_family_and_only_corroborates' crates/roko-cli/src/ && cargo test -p roko-cli --lib b3_review_uses_another_family_and_only_corroborates"

[[verify]]
command = "grep -rqw 'fn audited_labels_raise_a_gaming_alert_on_a_planted_stream' crates/roko-cli/src/ && cargo test -p roko-cli --lib audited_labels_raise_a_gaming_alert_on_a_planted_stream"

[[verify]]
command = "! grep -rqw 'fn spawn_gate' crates/roko-cli/src/ && ! grep -rqw 'fn spawn_plan_verify' crates/roko-cli/src/ && ! grep -rqw 'fn default_gate_adapter' crates/roko-cli/src/ && cargo check -p roko-cli --all-targets"

[[verify]]
command = "grep -qw 'fn a_planted_canary_retires_its_suite_and_a_replacement_audits_the_next_unit' crates/roko-cli/tests/audit_canary_drill.rs && cargo test -p roko-cli --test audit_canary_drill a_planted_canary_retires_its_suite_and_a_replacement_audits_the_next_unit"
+++

## Problem

This package delivers 9 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK59, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7122 | M | p2 | Audit worktree and the A2 clean re-run, off the critical path (S05 task 8, part 1) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7122-audit-worktree-and-a2-clean-rerun.md` |
| 2 | 7123 | M | p2 | Audit worker: a queue at concurrency 1, per-audit caps, results in the ledger, and a drain at run close (S05 task 8, part 2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7123-audit-worker-queue-caps-results-drain.md` |
| 3 | 7124 | M | p2 | B1 in the audit worker: author a hidden suite from the spec with another model family, validate it, and run it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7124-b1-author-validate-run-hidden-suite.md` |
| 4 | 7125 | M | p3 | B2: extreme mutation of the changed Rust functions in the audit worktree | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7125-b2-extreme-mutation-of-changed-rust-fns.md` |
| 5 | 7126 | S | p3 | Key the LLM judge's calibration rows by attempt and judge model, so audit labels can score the judge | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7126-judge-calibration-rows-keyed-by-attempt.md` |
| 6 | 7127 | S | p3 | B3: a cross-family review through the LLM-judge gate, as corroboration only | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7127-b3-cross-family-review-corroboration-only.md` |
| 7 | 7128 | S | p3 | Feed the gaming detector audit labels, weighted by inverse propensity, and give its alerts a reader (S05 task 9) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7128-gaming-detector-fed-audit-labels.md` |
| 8 | 7129 | S | p3 | Delete spawn_gate, spawn_plan_verify and default_gate_adapter once audits run checks through ProductionGateService (S05 task 15) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7129-delete-orphaned-gate-dispatch-helpers.md` |
| 9 | 7130 | S | p2 | Canary drill: an integration test for S05 §7.5 (S05 task 17) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7130-canary-drill-integration-test.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/audit/b1.rs`, `crates/roko-cli/src/audit/b2.rs`, `crates/roko-cli/src/audit/b3.rs`, `crates/roko-cli/src/audit/mod.rs`, `crates/roko-cli/src/audit/rerun.rs`, `crates/roko-cli/src/audit/worker.rs`, `crates/roko-cli/src/audit/worktree.rs`, `crates/roko-cli/src/graph_task_dispatch/audit_select.rs`, `crates/roko-cli/src/graph_task_dispatch/judge_step.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-cli/src/runner/gate_adapter.rs`, `crates/roko-cli/src/runner/gate_dispatch.rs`, `crates/roko-cli/tests/audit_canary_drill.rs`, `crates/roko-gate/src/judge_calibration.rs`, `crates/roko-gate/src/llm_judge_gate.rs`, `crates/roko-learn/src/gate_gaming.rs`.

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

- Waits on: PK53 (gap-dff960), PK58 (gap-f0a7ee).
- Suggested model: opus.

## Progress

- 7122: implemented on work/gap-147c4d at a9cd4dd48; cargo verification deferred to the batch gate.
- 7123: implemented on work/gap-147c4d at 5b820fdfc; cargo verification deferred to the batch gate. Canary coverage here is the diff's added lines (the task's Notes); the prompt scan landed with 7130. `[audit] enabled` still defaults to false: decision 7103's switch-on (with `--no-audit`) has no task yet.
- 7124: implemented on work/gap-147c4d at 3b5cabb42; cargo verification deferred to the batch gate.
- 7125: implemented on work/gap-147c4d at 9aa07d162; cargo verification deferred to the batch gate. Adds the roko-cli → roko-lang-rust dependency (Cargo.lock edited by hand).
- 7126: implemented on work/gap-147c4d at a1f56a058; cargo verification deferred to the batch gate.
- 7127: implemented on work/gap-147c4d at 81008a8d0; cargo verification deferred to the batch gate.
- 7128: implemented on work/gap-147c4d at 09d4787d4; cargo verification deferred to the batch gate. Audited units are always green, so the detector also counts every settled attempt's gate verdict (weight 1) for the pass rate.
- 7129: implemented on work/gap-147c4d at a4445c185; cargo verification deferred to the batch gate. `run_gate_once` stays, documented: its tests pin behaviour ProductionGateService lacks.
- 7130: implemented on work/gap-147c4d at edd962f07; cargo verification deferred to the batch gate. Not test-only after all: it needed the dispatch-time prompt scan and the burn (exposed, then retired) on a canary hit.
