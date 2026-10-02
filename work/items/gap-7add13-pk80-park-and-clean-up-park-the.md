+++
id = "gap-7add13"
kind = "gap"
title = "PK80 Park and clean up: Park the cognitive clock: `CorticalState`, the scheduler types and the theta/delta… (+6 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
rank = 80
size = "L"
subsystem = ["roko-cli/runtime_feedback"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK80"
anchors = [".github/workflows/ci.yml", "CLAUDE.md", "crates/roko-cli/Cargo.toml", "crates/roko-cli/src/doctor.rs", "crates/roko-cli/src/runtime_feedback/episodes.rs", "crates/roko-cli/src/runtime_feedback/plan_completion.rs", "crates/roko-compose/Cargo.toml", "crates/roko-compose/src/auction.rs", "crates/roko-compose/src/lib.rs", "crates/roko-core/src/config/learning.rs", "crates/roko-gate/Cargo.toml", "crates/roko-gate/src/adaptive_threshold.rs", "crates/roko-gate/src/lib.rs", "crates/roko-learn/Cargo.toml", "crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/lib.rs", "crates/roko-runtime/src/heartbeat.rs", "crates/roko-runtime/src/lib.rs", "docs/v3/00-INDEX.md"]
lane = "rust-hot"
parent = "spec-0b3a32"
links = { depends_on = ["gap-997366", "gap-2339e2", "gap-425d9e"], blocks = [], related = ["q-6b7cca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'heartbeat::CorticalState' crates/roko-cli/src/graph_execution/plan_runner.rs && ! grep -q 'theta_consumer::ThetaConsumer::default()' crates/roko-cli/src/graph_execution/plan_runner.rs"

[[verify]]
command = "grep -q '^cross-cut-functors' crates/roko-compose/Cargo.toml && grep -B1 'pub mod daimon_functor' crates/roko-compose/src/lib.rs | grep -q 'feature = \"cross-cut-functors\"'"

[[verify]]
command = "grep -rqw 'fn default_thresholds_serialize_no_spc_state' crates/roko-gate/ && cargo test -p roko-gate default_thresholds_serialize_no_spc_state"

[[verify]]
command = "grep -q '^active-inference' crates/roko-learn/Cargo.toml && grep -B1 'pub mod active_inference' crates/roko-learn/src/lib.rs | grep -q 'feature = \"active-inference\"'"

[[verify]]
command = "grep -rqw 'fn default_episode_has_no_hdc_fingerprint' crates/roko-cli/ && cargo test -p roko-cli default_episode_has_no_hdc_fingerprint"

[[verify]]
command = "ls benchmarks/park/*.json >/dev/null 2>&1 && grep -q 'Parked (off the default build)' CLAUDE.md"

[[verify]]
command = "grep -rqw 'fn disk_report_skips_creation_marker_dir' crates/roko-cli/src/ && cargo test -p roko-cli disk_report_skips_creation_marker_dir"
+++

## Problem

This package delivers 7 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK80, slice 92xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9222 | M | p3 | Park the cognitive clock: `CorticalState`, the scheduler types and the theta/delta consumers | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9222-park-cognitive-clock-cortical-state-consumers.md` |
| 2 | 9223 | S | p3 | Park roko-compose's unused cross-cut functors and `CrossCutArbitrator` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9223-park-unused-cross-cut-functors-in-roko-compose.md` |
| 3 | 9224 | S | p3 | Park the gate SPC ensemble (CUSUM, EWMA, BOCPD, PELT, Hotelling) behind an `spc` feature | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9224-park-gate-spc-ensemble-behind-spc-feature.md` |
| 4 | 9225 | S | p3 | Park the uncalled active-inference (EFE) tier selector behind a roko-learn feature | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9225-park-active-inference-efe-router.md` |
| 5 | 9226 | S | p3 | Stop writing episode HDC fingerprints by default: 60% of `episodes.jsonl`, read only by the TUI | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9226-stop-writing-episode-hdc-fingerprints-by-default.md` |
| 6 | 9227 | M | p3 | Measure the default build after parking and record what is parked in CLAUDE.md and the docs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9227-measure-and-record-the-parked-default-build.md` |
| 7 | 9237 | S | p3 | roko doctor disk lists .roko/worktrees/.roko-creation, the worktree creation journal, as an orphaned worktree | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9237-doctor-disk-lists-creation-journal-as-orphan.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9200-held-parked-and-cleanup.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/ci.yml`, `CLAUDE.md`, `benchmarks/park/`, `crates/roko-cli/Cargo.toml`, `crates/roko-cli/src/doctor.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/runtime_feedback/episodes.rs`, `crates/roko-cli/src/runtime_feedback/plan_completion.rs`, `crates/roko-compose/Cargo.toml`, `crates/roko-compose/src/auction.rs`, `crates/roko-compose/src/lib.rs`, `crates/roko-core/src/config/learning.rs`, `crates/roko-gate/Cargo.toml`, `crates/roko-gate/src/adaptive_threshold.rs`, `crates/roko-gate/src/lib.rs`, `crates/roko-learn/Cargo.toml`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/lib.rs`, `crates/roko-runtime/src/heartbeat.rs`, `crates/roko-runtime/src/lib.rs`, `docs/v3/00-INDEX.md`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK14 (gap-997366), PK78 (gap-2339e2), PK79 (gap-425d9e).
- Existing work items this package covers or touches: q-6b7cca. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
