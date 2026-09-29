+++
id = "bug-230de6"
kind = "bug"
title = "Graph (default) engine does not produce events.jsonl in mock plan runs"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "visibility"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-05
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "crates/roko-cli/tests/default_engine.rs:11"
discovered_from = "audit:crates/roko-cli/tests/default_engine.rs:11"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/event_log.rs::EventTap", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded", "crates/roko-cli/src/runner/persist.rs::append_run_scoped_event", "crates/roko-cli/src/serve_runtime.rs::collect_runner_gate_results", "crates/roko-runtime/src/state_hub.rs::replay_log_into_snapshot", "crates/roko-cli/tests/default_engine.rs::default_engine_does_real_work"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a", "gap-8a1fb3", "gap-09e478"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --test default_engine -- --include-ignored default_engine_does_real_work'
+++

## Problem

A plain `roko plan run <dir>` on the Graph engine (the only engine) writes no workspace event log. Nothing
appends to `.roko/events.jsonl`, and no per-run event index is created. The run's events exist only in memory on
its StateHub. They are written to disk only when the operator passes `--log-file <path>`, and that flag has no
default.

Things that read `.roko/events.jsonl` therefore see nothing from Graph runs:

- `roko dashboard` started in another terminal. `tui/app/mod.rs` (~lines 1019-1040) replays
  `.roko/events.jsonl` as `DashboardEvent` lines, and the file watcher tails it.
- `serve_runtime.rs::collect_runner_gate_results` (~line 1319). After every serve plan run
  (`run_plan_with_options`, ~lines 855-925) it reads Runner-v2 `RunnerEvent::GateCompleted` lines from that file.
  So serve plan runs always report "0 gate results" and an empty `gate_results`, even when gates ran.
- `roko-serve/src/routes/runs.rs` (~line 759), which reads the per-run index derived from that file.
- `roko doctor`, and `roko-fs` GC and log rotation, which expect the file.

The regression test for this, `crates/roko-cli/tests/default_engine.rs::default_engine_does_real_work`, is
`#[ignore]`d with "Graph engine events.jsonl not yet produced by mock plan runs".

## Why it matters

- Goal `visibility`: runs must be observable live in serve, the TUI and the portal. A dashboard in a second
  terminal, or serve's plan-run result, is blind to Graph runs.
- `gap-09e478` (the dogfood evidence bundle) wants a durable per-run record.
- Related:
  - `gap-8a1fb3`: the event stream lacks run completion, heartbeats and similar.
  - `bug-165b22`: `roko diagnose` reads removed Runner-v2 state.
  - `bug-f7943a` (done): `dev.sh fast` used the removed engine.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan` (line ~701):
  - With `--log-file`, it returns early into `event_log::run_recorded`, which records and then calls
    `run_graph_plan` again with `log_file` cleared.
  - It resolves the StateHub early (~line 708) so that `RunCompleted` is published on every exit path.
- `crates/roko-cli/src/graph_execution/event_log.rs`:
  - `EventTap` (line ~57) subscribes to a hub and folds its events on its own task. Reuse it.
  - `RunEventLog` (line ~156) is the `--log-file` recorder. It uses an envelope format,
    `{"type":"dashboard.<kind>","run_id":…,"event":{…}}`, which the TUI replay cannot parse.
  - The run id is `ROKO_EVIDENCE_RUN_ID` or `graph-<uuid>` (~line 167).
- `crates/roko-cli/src/runner/persist.rs`:
  - `append_run_scoped_event` (line ~691) appends one JSON line to `.roko/events.jsonl` with log rotation, plus
    the derived per-run index. It is exactly the writer needed, and it has no callers at HEAD.
  - `PersistPaths::from_workdir` (line ~64) gives `events_jsonl`.
- `crates/roko-runtime/src/state_hub.rs::replay_log_into_snapshot` (line ~1116) and `replay_events_from_reader`:
  the TUI's parsers. Each line must be a bare `serde_json` `DashboardEvent`
  (`#[serde(tag = "type", rename_all = "snake_case")]` in `roko-core/src/dashboard_snapshot.rs` ~line 77).
- `crates/roko-cli/src/serve_runtime.rs::collect_runner_gate_results`: parses `RunnerEvent`, the Runner-v2
  format, from the same file.
- `crates/roko-cli/tests/default_engine.rs`: the ignored test. It also asserts `.roko/state/run-ledger.jsonl`
  (`gate_outcome`) and `.roko/state/state-snapshot.json`. Those are Runner-v2 persist artifacts that no Graph code
  writes, so it cannot pass without being rewritten.

## Current state

- Unchanged as of `d9e79e9d8`: the test is still ignored, and `run_graph_plan` records events only with
  `--log-file`.
- `725f21e05` added `event_log.rs` (the `--log-file` recorder and `EventTap`). `default_engine.rs` has not
  changed since `a469c4de0`.
- A Graph run does write:
  - `.roko/state/graph/<plan>/{checkpoint.json, activities.jsonl, costs.json}`;
  - `.roko/episodes.jsonl`, through the feedback facade;
  - efficiency records.
- `crates/roko-cli/tests/graph_plan_callers.rs::plan_run_log_file_records_one_bracketed_run` shows that the
  StateHub carries `plan_set_loaded`, `plan_started`, `agent_spawned`, `task_completed` and `plan_completed` for
  a mock run. So the data exists; it is just not persisted.

## Plan

1. Pick the format. Recommended: bare `DashboardEvent` JSON lines, the hub's native type.
   - It is what the TUI replay and the file watcher already parse.
   - It needs no mapping layer. Do not bring back `RunnerEvent` writing: that would keep a deleted engine's
     format alive.
2. Always record. In `run_graph_plan`, after the hub is resolved and before any plan starts, spawn an `EventTap`
   on the hub. For each event, call `persist::append_run_scoped_event(&paths, &run_id, &event, …)`:
   - Use `EventDurability::Relaxed` for high-rate events (`AgentOutput`, `TaskOutputAppended`,
     `GateOutputLine`).
   - Use `EventDurability::Durable` with `flush_index = true` for lifecycle events (`PlanStarted`,
     `PlanCompleted`, `TaskStarted`, `TaskCompleted`, `GateResult`, `RunCompleted`).
   - Finish the tap on every exit path, the way `RunEventLog::finish` does, so `RunCompleted` is written.
   - The `--log-file` path goes through `run_graph_plan` too, so it gets this for free.
3. Share the run id. Pull the `ROKO_EVIDENCE_RUN_ID`/`graph-<uuid>` choice out of `RunEventLog::open` into one
   helper, and use the same id for both logs.
4. Fix serve's gate evidence. Change `collect_runner_gate_results` to read `DashboardEvent::GateResult` lines,
   filtered to the run's plan ids and read from the recorded offset. The alternative is to take gate results
   straight from the run's StateHub or checkpoint. Either way, a serve plan run must report the gates that ran.
5. Rewrite `default_engine_does_real_work` for the Graph engine and remove its `#[ignore]`:
   - Keep the bare `plan run plans` call with no `--engine`.
   - Assert that `.roko/events.jsonl` has a `"type":"task_started"` or `"type":"task_completed"` line for the
     sample task.
   - Assert a `"type":"gate_result"` line too. The sample plan in `tests/common/mod.rs` has
     `verify = [{ command = "cargo check", phase = "compile" }]`, so a gate does run.
   - Assert that `.roko/episodes.jsonl` is non-empty.
   - Assert that `.roko/state/graph/<plan>/checkpoint.json` exists.
   - Drop the `run-ledger.jsonl` and `state-snapshot.json` assertions.

## Done when

- A bare `roko plan run plans` with the mock agent leaves `DashboardEvent` lines for its plan and tasks in
  `.roko/events.jsonl`, and a per-run index under the run id.
- A second `roko dashboard` replays them.
- A serve plan run whose tasks have verify steps reports a non-zero `gate_results` count.
- `default_engine_does_real_work` runs without `#[ignore]` and passes.
- Verify:
  `cargo test -p roko-cli --test default_engine default_engine_does_real_work && ! grep -q 'ignore = ' crates/roko-cli/tests/default_engine.rs`

## Notes

- Persistence: `.roko/events.jsonl` is append-only and rotated (`log_rotation_max_mb`). Keep all writes going
  through `append_run_scoped_event` so rotation and the run index stay consistent. Never truncate the file.
- Keep the `--log-file` envelope format exactly as it is. `scripts/run_evidence.py --require-events` and
  `./dev.sh fast` check it (see `event_log.rs` lines 1-18).
- In serve, the hub is shared across the process. A per-run tap may also record unrelated hub events (feed ticks
  and so on) during the run. Filter to run, plan, task, agent and gate events if that becomes noisy.
- The test runs the real binary with mock agents (`common::setup_sample_plan_workspace`, `run_roko_isolated`). It
  must stay hermetic: isolated `HOME`, no API keys.
- Parallel safety: this edits `run_graph_plan` in `plan_runner.rs` and `serve_runtime.rs`. Coordinate with
  `gap-7c9e48` and `spec-3cb55d`, which touch the same runner, and with `gap-8a1fb3` (event stream contents).

## Original notes

default_engine_does_real_work is ignored: 'requires engine-convergence wiring: Graph engine events.jsonl not yet produced by mock plan runs'. The default engine's per-run event log (used by status/diagnose/dashboard) may be missing for Graph runs.

Imported without verification from:
- `crates/roko-cli/tests/default_engine.rs:11`

How to verify: Run the test with --ignored; check whether graph_execution writes .roko/runs/<id>/events.jsonl for mock plans.

Verified 2026-09-28: still true - tests/default_engine.rs:11 is still #[ignore]; the Graph runner only opens a GraphEventLogger when --log-file is passed (graph_execution/plan_runner.rs:1158-1172), so a bare `roko plan run` writes no .roko/events.jsonl task.attempt records.

Verified 2026-09-28 (static check against 3d0ee4d02): crates/roko-cli/tests/default_engine.rs:10-11 is still #[ignore = "...Graph engine events.jsonl not yet produced by mock plan runs"]. run_graph_plan only records events when --log-file is given (graph_execution/plan_runner.rs:646-649 hands off to event_log::run_recorded; the GraphEventLogger block is now at :1129-1143), and no Graph-path code writes .roko/events.jsonl task.attempt records on a bare `roko plan run` (only runner/types.rs event enums and TUI readers reference them).

Re-verified 2026-09-29 at d9e79e9d8: unchanged. default_engine.rs:11 is still ignored; run_graph_plan writes events only with --log-file (plan_runner.rs:702-706, logger block :1259-1273), and --log-file has no default. The verify test also asserts .roko/state/run-ledger.jsonl gate_outcome and .roko/state/state-snapshot.json, which are Runner-v2 persist artifacts (runner/persist.rs) that no graph_execution code writes, so wiring events.jsonl alone will not make it pass.
