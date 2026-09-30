+++
id = "gap-568056"
kind = "gap"
title = "Graph runs write no .roko/state/status.json, so roko status and evidence status sampling see no live run"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["roko-cli/graph-execution", "roko-cli/status"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "bbebe40d6"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-evidence's report on gap-09e478)"
anchors = ["crates/roko-cli/src/runner/status_file.rs::write_status_debounced", "crates/roko-cli/src/status.rs:234", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-hot"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["gap-09e478", "q-1faa0c", "gap-082a14"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_graph_run_writes_status_json' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_graph_run_writes_status_json"
+++

## Problem

`runner/status_file.rs` has writers (`write_status_debounced`, `write_status_immediate`) and a reader (`read_runner_status`). Since the Runner-v2 event loop was deleted, nothing outside the module's own tests calls the writers, so a Graph run never writes `.roko/state/status.json`. The readers remain: `roko status` takes the runner's phase and liveness from it (`status.rs:234-250`), and the evidence collector samples it (`scripts/run_evidence.py`, `--status-file`). During a Graph run, `roko status` shows no live runner, and evidence bundles record status sampling as `skipped`.

## Why it matters

The development record (epic spec-f2463d) needs to see what a run is doing while it runs, not only afterwards.

## Where

- `crates/roko-cli/src/runner/status_file.rs`: the file format, writers and reader.
- `crates/roko-cli/src/status.rs:234`: `roko status` reads it.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: where a Graph run would write it.

## Current state

Checked at BASE: `write_status_debounced`, `write_status_immediate` and `status_file_path` have no references outside `runner/status_file.rs`; `read_runner_status` has two, both in `status.rs`. gap-082a14 (writing task status back to `tasks.toml`) and q-1faa0c (checking old runtime fixes on Graph) mention the missing file but don't cover writing it. gap-09e478 left it out because it needs `plan_runner.rs`.

## Plan

Design choice:

- **Option A (recommended): write `status.json` from Graph runs.** Plan id, run id, phase (dispatch, gate, merge, idle), running and finished task counts, pid and a heartbeat time, derived from the StateHub events `plan_runner.rs` already publishes.
- **Option B: retire `status.json`.** Make `roko status` and the collector read the Graph checkpoint and the PID registry instead, and delete the writers.

Steps for Option A:

1. Subscribe a small writer to the run's StateHub in `plan_runner.rs`.
2. Write immediately at start and finish, debounced in between.
3. Mark the file finished (or remove it) when the run ends.
4. Add `a_graph_run_writes_status_json`.

## Done when

- [ ] During a Graph run, `roko status` reports it as live with its phase (or Option B's replacement does), and the collector's status samples are not empty.
- [ ] The `[[verify]]` command passes. If Option B is chosen, move the test and the verify command with it.

## Notes

- `plan_runner.rs` is a hot file.
- The parked gap-c3f8a3 (status.json lacks PID and staleness semantics) has design notes on the file's fields.
- Implemented on `work/bug-4c4eea` at `bbebe40d6` (the writer landed at `617c809ec`); cargo verification deferred to the
  batch check.
- Option A. `runner/status_file.rs` has `GraphRunStatus` (a fold of the run's StateHub events) and `GraphStatusWriter`,
  which `run_graph_plan_in_run` spawns before the body and finishes after `RunCompleted`. It writes the starting status at
  once, then a changed status within a second, and an unchanged one every 5 s as a heartbeat. The terminal status stays
  in the file.
- Phases: `dispatch`, `gate`, `idle`, then `completed`, `failed` or `cancelled`. There is no `merge` phase, because Graph
  runs publish no merge events. The file gains `plan_id`, `running_tasks`, `finished_tasks` and `total_tasks`.
- Run id: `ROKO_EVIDENCE_RUN_ID` when set (the same id as the `--log-file` lines), else the caller's run id, else a new
  `graph-<uuid>`. The terminal phase comes only from `finish`, because runs started by `roko serve` share one hub and a
  `RunCompleted` there may be another run's.
- After a CLI run exits, `roko status` shows `stale/offline (was: <phase>)`, as it did for Runner-v2. After a run in a
  live `roko serve` it keeps showing the terminal phase as active, since the writer's PID is still alive.
- `scripts/test_run_evidence_graph.py::test_bundles_hold_only_their_own_run` now expects the run's own status samples
  (state `sampled`). It needs a built `roko`; `CollectorUnits` pass.
