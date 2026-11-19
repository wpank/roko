+++
id = "bug-bfb75a"
kind = "bug"
title = "plan_total_secs is dead on Graph runs: phase 1 ran 73.5 min under plan_total_secs = 3600 and was never stopped"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "cdcab3318"
source = "round-v7-hermes-pilot#p1-0bed5c58"
discovered_from = "agent:hermes-pilot (round v7, phase 1 run 0bed5c58, 2026-10-19)"
anchors = ["crates/roko-cli/src/runner/types.rs::effective_plan_timeout_secs", "crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "crates/roko-core/src/config/timeouts.rs::TimeoutConfig"]
links = { depends_on = [], blocks = [], related = ["bug-f0007b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_stops_at_its_time_limit' crates/roko-cli/src && cargo test -p roko-cli --lib a_run_stops_at_its_time_limit"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T10:54:17Z"
commit = "cdcab3318"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T09:42:54Z"
forced = false
evidence = "Every Graph plan run now arms [timeouts] plan_total_secs (graph_execution/time_limit.rs, armed in run_graph_plan_body after the config loads; a FAST run keeps its own deadline). At the limit the run stops through its interrupt as PlanRunInterrupt::TimeLimit: label time-limit, exit 143, checkpoint interrupted and resumable. 0 sets no limit (effective_plan_timeout_secs no longer floors it to 1 s). Verify a_run_stops_at_its_time_limit passes (the limit counts from the run's start, a dropped limit disarms, 0 arms nothing). $0 e2e (bench/bounds-bench.sh, plan_total_secs = 10, 6 s turns): the run stopped at 10 s with interrupted_by time-limit and exit 143, 'the run's time limit ([timeouts] plan_total_secs) elapsed; stopping the plan run, which --resume-plan continues'; --resume-plan then finished both tasks, exit 0."
+++

## Problem

`[timeouts] plan_total_secs` ("Total plan execution timeout", default 3600, written into every `roko.toml` by
`roko init`) does nothing on a Graph run. Round v7's phase 1 run of Undertow (`0bed5c58`, roko 3b34aed92, hermes-pilot)
ran 73.5 minutes with `plan_total_secs = 3600` in its `roko.toml`, and was never stopped.

The setting was read by the Runner-v2 event loop, deleted on 2026-09-06 (`6b5da8616`). Since then
`effective_plan_timeout_secs` fills `RunConfig.plan_timeout_secs`, which nothing reads outside tests and `Debug`. The
legacy `[runner] plan_timeout_secs` fallback is dead the same way.

## Why it matters

A person who sets a wall-clock limit for a run, next to its spend cap, expects it to hold. A documented control that
silently does nothing is worse than none: the run goes on past the time the person allowed.

## Where

- `crates/roko-core/src/config/timeouts.rs`: `TimeoutConfig::plan_total_secs`, `plan_total()`.
- `crates/roko-cli/src/runner/types.rs`: `effective_plan_timeout_secs` (its `.max(1)` would make `0` a 1 s limit),
  `RunConfig.plan_timeout_secs`.
- `crates/roko-cli/src/graph_execution/fast_lane.rs`: `arm_plan_deadline` already stops a FAST run at its deadline
  through the run's `PlanRunInterruptHandle` (`PlanRunInterrupt::Deadline`: the graph is cancelled, the checkpoint is
  finalized as interrupted and resumable, exit 143).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `run_graph_plan_body` arms only the FAST deadline.

## Plan

1. Arm a run deadline from `effective_plan_timeout_secs` for every Graph plan run, through the same interrupt as the
   FAST deadline. `0` turns it off. A FAST run keeps its own, shorter deadline.
2. Say what stopped the run: the run's time limit and the setting that set it, not "the FAST deadline".
3. The stopped run stays resumable (`--resume-plan`), as after SIGTERM.

## Done when

A plan run stops at `plan_total_secs`, as interrupted and resumable, and says so; `plan_total_secs = 0` arms no
deadline. Checked by the test in `[[verify]]`.

## Notes

- Found by hermes-pilot (round v7): "`plan_total_secs = 3600` is not enforced on Graph runs: the run went past 62
  min without being stopped" (`tmp/hermes-roko-v7/findings/hermes-pilot.md`).
- Other `[timeouts]` run-level fields are schema-only too: `hard_run_secs`, `task_attempt_secs`, `gate_effect_secs`,
  `agent_silence_secs`, `scheduler_no_progress_secs` (set only by the config loader's schema sentinel).
- Taken with bug-f0007b (a per-try time bound), on the lead's ask.
