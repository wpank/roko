+++
id = "gap-dff960"
kind = "gap"
title = "PK53 M4 deep audits: Both LLM judges read a list number, a scale echo or a 0–10 score as their 0–1 score (+3 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 53
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK53"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs", "crates/roko-cli/src/graph_task_dispatch/inert_settings.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-gate/src/agent_judge.rs", "crates/roko-learn/src/quality_judge.rs"]
lane = "rust-hot"
parent = "spec-c3abc8"
links = { depends_on = ["gap-c06ff3"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn judge_scores_ignore_list_numbers' crates/roko-gate/src/agent_judge.rs && grep -qw 'fn quality_judge_ignores_list_numbers' crates/roko-learn/src/quality_judge.rs && cargo test -p roko-gate --lib judge_scores_ignore_list_numbers && cargo test -p roko-learn --lib quality_judge_ignores_list_numbers"

[[verify]]
command = "! grep -q 'judge_quality' crates/roko-cli/src/graph_task_dispatch/verification.rs && grep -rqw 'fn verify_feeds_the_gaming_detector_nothing' crates/roko-cli/src/ && cargo test -p roko-cli --lib verify_feeds_the_gaming_detector_nothing"

[[verify]]
command = "! grep -q 'EvalGenerator' crates/roko-cli/src/graph_task_dispatch.rs && grep -rqw 'fn write_eval_artifacts_creates_nothing_in_the_workdir' crates/roko-cli/src/ && cargo test -p roko-cli --lib write_eval_artifacts_creates_nothing_in_the_workdir"

[[verify]]
command = "grep -qw 'def test_a1_flags_every_planted_tamper_kind' benchmarks/viabilitybench/audit/tests/test_battery.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_battery.py -q"
+++

## Problem

This package delivers 4 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK53, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7104 | S | p3 | Both LLM judges read a list number, a scale echo or a 0–10 score as their 0–1 score | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7104-llm-judges-misread-list-numbers-as-scores.md` |
| 2 | 7105 | S | p3 | Stop the paid quality judge and the fixed 0.9 that feed the gaming detector on every verify | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7105-stop-quality-judge-feeding-gaming-detector.md` |
| 3 | 7106 | S | p3 | Delete the dormant EvalGenerator pre-dispatch write | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7106-delete-dormant-evalgenerator-write.md` |
| 4 | 7108 | M | p2 | Python: the offline audit battery (A1 tamper diff, A2 clean re-run) and the vs.label writer (S05 task 1) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7108-python-offline-battery-a1-a2-and-labels.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/audit/battery.py`, `benchmarks/viabilitybench/audit/fixtures/`, `benchmarks/viabilitybench/audit/labels.py`, `benchmarks/viabilitybench/audit/tamper.py`, `benchmarks/viabilitybench/audit/tests/test_battery.py`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`, `crates/roko-cli/src/graph_task_dispatch/inert_settings.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-gate/src/agent_judge.rs`, `crates/roko-learn/src/quality_judge.rs`.

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

- Waits on: PK28 (gap-c06ff3).
- Suggested model: opus.

## Progress

- 7104: implemented at 4dd56c632 (on `work/gap-dff960`; cargo verification deferred to the batch gate)
- 7105: implemented at 5a3e2be50 (on `work/gap-dff960`; cargo verification deferred to the batch gate)
- 7106: implemented at c20dc5f5c (on `work/gap-dff960`; cargo verification deferred to the batch gate)
- 7108: implemented at 00b05e510 (its verify passes in the bench venv: `test_battery.py`, 6 passed)
