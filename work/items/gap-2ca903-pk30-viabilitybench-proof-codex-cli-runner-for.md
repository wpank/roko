+++
id = "gap-2ca903"
kind = "gap"
title = "PK30 ViabilityBench proof: Codex CLI runner for the fd_codex arm (+7 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 30
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK30"
anchors = ["benchmarks/viabilitybench/arms/cheap_direct.toml", "benchmarks/viabilitybench/arms/roko_fixed.toml", "benchmarks/viabilitybench/driver/vb.py", "crates/roko-cli/src/commands/bench.rs", "crates/roko-serve/src/routes/bench.rs"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = ["gap-a0043b", "gap-f61823", "gap-08120e", "gap-9e3134", "gap-5ebb4f", "gap-eb1aa3", "gap-daeaa9", "gap-5ddf9b", "gap-c06ff3", "gap-064c40"], blocks = [], related = ["dec-39c781", "gap-644040", "q-ab27d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_codex_runner_prices_turn_completed_usage' benchmarks/viabilitybench/driver/test_run_codex.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_codex.py -k test_codex_runner_prices_turn_completed_usage -q"

[[verify]]
command = "grep -rqw 'fn serve_bench_archives_graded_workspaces' crates/roko-serve/ && cargo test -p roko-serve serve_bench_archives_graded_workspaces"

[[verify]]
command = "grep -rqw 'fn serve_bench_prices_the_served_model' crates/roko-serve/ && cargo test -p roko-serve serve_bench_prices_the_served_model"

[[verify]]
command = "grep -rqw 'fn bench_viability_passes_arguments_through' crates/roko-cli/ && cargo test -p roko-cli bench_viability_passes_arguments_through"

[[verify]]
command = "grep -qw 'def test_log1_manifest_matches_s09_cell_counts' benchmarks/viabilitybench/experiments/test_log1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_log1.py -k test_log1_manifest_matches_s09_cell_counts -q"

[[verify]]
command = "grep -qw 'def test_g2_page_reports_every_check_with_its_value' benchmarks/viabilitybench/analysis/test_gates.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_gates.py -k test_g2_page_reports_every_check_with_its_value -q"

[[verify]]
command = "grep -qw 'def test_replay_is_byte_identical_and_aa_shows_no_effect' benchmarks/viabilitybench/analysis/test_replay_runner.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_runner.py -k test_replay_is_byte_identical_and_aa_shows_no_effect -q"

[[verify]]
command = "grep -qw 'def test_r_h5_replay_covers_at_nominal_rate' benchmarks/viabilitybench/analysis/test_replay_h5.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h5.py -k test_r_h5_replay_covers_at_nominal_rate -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/experiments/prereg.lock.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/lock.py --check benchmarks/viabilitybench/experiments/prereg.lock.json"

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/PREREG.md && grep -q 'prereg_id' tmp/cybernetic-harness/paper/PREREG.md"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK30, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3342 | M | p3 | Codex CLI runner for the fd_codex arm | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3342-codex-cli-runner-for-fd-codex.md` |
| 2 | 3343 | S | p3 | Serve bench archives graded workspaces and prices each task by the served model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3343-serve-bench-archives-workspaces-and-prices-served-model.md` |
| 3 | 3344 | S | p3 | roko bench viability runs the ViabilityBench driver from the CLI | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3344-roko-bench-viability-subcommand.md` |
| 4 | 3347 | S | p1 | LOG1 runbook manifest: blocks, daily interleave, secret assignment and subscription schedule | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3347-log1-runbook-manifest.md` |
| 5 | 3348 | S | p1 | The G2 gate page: completion, infra errors, cap censoring, a 5% re-audit and the frozen-loop census | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3348-g2-gate-page.md` |
| 6 | 3354 | M | p2 | vb replay: deterministic replays on arm-hashed data, with an A/A check | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3354-vb-replay-deterministic-with-aa-check.md` |
| 7 | 3356 | S | p2 | Replay R-H5 on S05's lottery replay | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3356-replay-r-h5-lottery.md` |
| 8 | 3345 | S | p1 | Take the pre-registration lock: commit prereg.lock.json and write PREREG.md (D2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3345-take-the-prereg-lock-and-write-prereg-md.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/gates.py`, `benchmarks/viabilitybench/analysis/replay_h5.py`, `benchmarks/viabilitybench/analysis/replay_runner.py`, `benchmarks/viabilitybench/analysis/test_gates.py`, `benchmarks/viabilitybench/analysis/test_replay_h5.py`, `benchmarks/viabilitybench/analysis/test_replay_runner.py`, `benchmarks/viabilitybench/arms/cheap_direct.toml`, `benchmarks/viabilitybench/arms/fd_claude_lite.toml`, `benchmarks/viabilitybench/arms/fd_codex.toml`, `benchmarks/viabilitybench/arms/roko_fixed.toml`, `benchmarks/viabilitybench/driver/run_codex.py`, `benchmarks/viabilitybench/driver/test_run_codex.py`, `benchmarks/viabilitybench/driver/vb.py`, `benchmarks/viabilitybench/experiments/log1.toml`, `benchmarks/viabilitybench/experiments/prereg.lock.json`, `benchmarks/viabilitybench/experiments/test_log1.py`, `crates/roko-cli/src/commands/bench.rs`, `crates/roko-serve/src/routes/bench.rs`, `tmp/cybernetic-harness/paper/PREREG.md`.

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

- Waits on: PK08 (gap-a0043b), PK10 (gap-f61823), PK12 (gap-08120e), PK13 (gap-9e3134), PK20 (gap-5ebb4f), PK23 (gap-eb1aa3), PK25 (gap-daeaa9), PK27 (gap-5ddf9b), PK28 (gap-c06ff3), PK29 (gap-064c40).
- Existing work items this package covers or touches: dec-39c781, gap-644040, q-ab27d3. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
