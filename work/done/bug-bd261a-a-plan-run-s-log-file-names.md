+++
id = "bug-bd261a"
kind = "bug"
title = "A plan run's --log-file names no plans in run.started: the spec gate's events now come before PlanSetLoaded, so run.started has plan_ids [] and total_tasks 0 (since 40e48440b)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "3dd67beb5"
source = "bug-c63cca harness fix, 2026-10-26: graph_plan_callers::plan_run_log_file_records_one_bracketed_run"
discovered_from = "bug-c63cca"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::write_start", "crates/roko-cli/src/graph_execution/plan_runner.rs::SpecGateStarted"]
links = { depends_on = [], blocks = [], related = ["bug-c63cca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn with_mock_agents' crates/roko-cli/tests/common/mod.rs && cargo test -p roko-cli --test graph_plan_callers -- --exact plan_run_log_file_records_one_bracketed_run"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:31:00Z"
commit = "3dd67beb5"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T16:06:43Z"
forced = false
evidence = "run.started is written when the --log-file recorder opens and names the plans the run executes: run_recorded selects them with the new plan_runner::compute_plan_run_set (load, --only, plan-set order; serve compute_plan_run_order now reads it) and RunEventLog::open_for_run writes the line before the run publishes anything, so the spec gate events that now come before PlanSetLoaded no longer blank it (option 3; option 2 would drop the spec.gate.refused rows serve reads from a refused run index). The verify (grep with_mock_agents && cargo test -p roko-cli --test graph_plan_callers -- --exact plan_run_log_file_records_one_bracketed_run) failed before at graph_plan_callers.rs:266 (plan_ids [] vs [test-wire-xyz]) and passes on 3dd67beb5 with no claude CLI and no keys. Also green: 18 event_log and plan-set unit tests, among them the new run_started_names_the_plans_before_the_spec_gate_events and compute_plan_run_set_selects_and_orders_plans_as_the_run_does; the --log-file integration tests graph_plan_callers (2), graph_timeout_matrix (7), supervision_canary (2) and honest_verdicts_canary (1); cargo clippy -p roko-cli --no-deps -- -D warnings as CI runs it. clippy --all-targets still fails only on the pre-existing test-code findings gap-7bf92d tracks, none in changed lines."
+++

## Problem

Since 40e48440b (2026-10-19), every `roko plan run --log-file <path>` writes `run.started` with `"plan_ids": []` and
`"total_tasks": 0`. The recorder (`EventLogWriter::record`) writes `run.started` on the run's first hub event and
takes the plan set only from a `PlanSetLoaded` event. Its own doc says "The plan set comes from the run's
`PlanSetLoaded` event, its first". 40e48440b made the plan-load spec gate publish `SpecGateStarted` and
`SpecGateFinished` before `PlanSetLoaded`, so `run.started` is now written from `SpecGateStarted`, with no plan set.

How to see it:
- `graph_plan_callers::plan_run_log_file_records_one_bracketed_run` fails
  `assert_eq!(lines[0]["plan_ids"], json!(["test-wire-xyz"]))`, with left `[]`.
- By hand on 15335e2f9, a one-task plan `p1` with a mock agent and no keys gives:
  - line 0: `run.started` with `plan_ids` `[]`;
  - line 1: `dashboard.spec_gate_started`, whose event names `["p1"]`;
  - line 2: `spec_gate_finished`;
  - line 3: `plan_set_loaded`.

## Why it matters

`run.started` is the `--log-file` contract that `scripts/run_evidence.py --require-events` checks for `./dev.sh fast`.
Its `plan_ids` feed the "no Graph state evidence for plan X named by run.started" cross-check, which is only a warning
today. Its `total_tasks` gives the run's size. A reader can no longer tell from the first line which plans a run ran.
The failing test also keeps CI's Test step red once bug-c63cca lets it past the model preflight.

## Where

- `crates/roko-cli/src/graph_execution/event_log.rs`: `EventLogWriter::record` and `EventLogWriter::write_start`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `SpecGateStarted` is published, about line 1324, before
  `PlanSetLoaded`, about line 1571.

## Current state

Broken since 40e48440b ("telemetry: per-try phases, the checkout, merges and the spec gate on the hub"). The release
check on 2026-10-18 predates that commit. Until bug-c63cca, the test failed earlier on any machine without `claude`, at
the model preflight, which hid this failure.

## Plan

There is a real design choice:
1. The recorder holds lines back until `PlanSetLoaded`, or until the run ends, and then writes `run.started` with the
   plan set first. This keeps the hub's order, but a spec gate that runs for minutes shows in the log only once the
   plans load.
2. Publish `PlanSetLoaded` before the spec gate. This changes what the TUI and `.roko/events.jsonl` show for a run the
   gate refuses.
3. `run_recorded` works out the plan set itself before the run starts, from the plans directory and `--only` in its
   params, and hands it to the writer.

Chosen (2026-10-26): option 3. `run_recorded` selects the plans with `plan_runner::compute_plan_run_set`, the selection
serve's `compute_plan_run_order` already made before a run (load, `--only`, plan-set order), and
`RunEventLog::open_for_run` writes `run.started` as the log opens, before the run publishes anything. The line no longer
depends on the hub's event order, survives a process killed during the spec gate, and names the plans of a run the gate
refuses. Option 2 was rejected: once `PlanSetLoaded` comes first, `WorkspaceLogWriter::owns` drops the gate's
`spec.gate.refused` rows from the run's index, because they name the plan's `tasks.toml` path rather than its id, and
serve reads a refused run's reasons from those rows (`roko_serve::run_end::spec_gate_refusals`, bug-9a5970). Option 1
would keep the log empty for the whole spec gate and lose those lines if the process died.

## Done when

`plan_run_log_file_records_one_bracketed_run` passes with no keys and no `claude` on PATH. That is the `[[verify]]`;
bug-c63cca's test harness must be merged first, or the test fails at the model preflight instead.

## Notes

Found by bug-c63cca (2026-10-26), after the harness fix let the test reach its assertions.
