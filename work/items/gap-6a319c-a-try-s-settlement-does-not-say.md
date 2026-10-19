+++
id = "gap-6a319c"
kind = "gap"
title = "A try's settlement does not say where its time went: model, tool, check, queue, checkout and merge time are not split, gate_result has no duration, and the plan-load spec gate publishes no timings"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#telemetry"
discovered_from = "agent:roko-speed (round v7, with plugin-wow, 2026-10-19)"
anchors = ["crates/roko-learn/src/telemetry/records.rs::AttemptTiming", "crates/roko-core/src/dashboard_snapshot.rs::AttemptSettlement", "crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs::LiveToolCalls", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::AttemptContext"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "gap-e02ec4", "gap-4e852d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_phases_add_up_to_its_duration' crates/roko-cli/src && cargo test -p roko-cli --lib attempt_phases"
+++

## Problem

A try's settlement says how long it took (`attempt_settled.duration_ms`, and timestamps in the `roko.verdict/1` row),
but not where the time went:
- Model time and tool time are one number: the dispatch window.
- A check's duration is not on its `gate_result`.
- The time before the try opens is not recorded: waiting for budget, disk or a slot, and making the checkout.
- Accepting a passed try onto the plan branch, after settlement, is not recorded.
- The plan-load spec gate can run for minutes before any agent starts (171 s of a 283 s run on round v7's JUCE
  fixture) and publishes nothing until it ends.
- Plan writing logs its phase timings (`plan generate: phase timing`) but publishes none.

So the Hermes plugin's run page can only estimate a try's split from event timestamps. It cannot say "the build took
40 s of this 45 s try", or "this try was faster because its build tree was warm".

## Why it matters

Round v7 builds a VST3 synth from Hermes as many small C++ tasks. On this workload the time goes to builds, the
spec gate and model calls in shares that change from task to task. The person watching a long multi-plan build, and
the round's speed work, both need the real split: what to fix next, and evidence that a fix worked. plugin-wow builds
the page's per-try timing strip and run timeline on these fields. The names were agreed with it on 2026-10-19 (round v7
findings, roko-speed, "Telemetry schema").

## Where

- `crates/roko-learn/src/telemetry/records.rs::AttemptTiming`: the verdict row's `timing`. It gains `phases`, and the
  verdict gains `workspace`.
- `crates/roko-core/src/dashboard_snapshot.rs::AttemptSettlement`: the hub event. It gains the same `phases` and
  `workspace`. `GateResult` gains `duration_ms`. New events: `AttemptMerged`, `SpecGateStarted`, `SpecGateFinished`.
- `crates/roko-cli/src/graph_task_dispatch.rs` (`dispatch`): queue, workspace and accept timing.
  `graph_task_dispatch/attempt.rs`: the phases are computed at settle. `graph_task_dispatch/live_tool_calls.rs`: tool
  intervals and names.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the spec gate events around `spec_gate::gate_plans`.

## Current state

- `AttemptTiming` has these timestamps: attempt start, prompt assembled, dispatch start/end, first token, verify
  start/end, settled.
- Verify steps have `steps[].duration_ms`.
- `LiveToolCalls` pairs tool steps with results, without times.
- Plan generation measures init/context/prompt/agent/extraction/post in `generate_plan` and only logs them.

## Plan

1. One `AttemptPhases` type (roko-core), used by both the verdict's `timing.phases` and `attempt_settled.phases`:
   - durations: `workspace_ms`, `queue_ms`, `prompt_ms`, `ttft_ms`, `model_ms`, `tools_ms`, `checks_ms`, `other_ms`;
   - counts: `model_calls`, `tool_calls`;
   - `tools_by_name`: at most 5 `{tool, calls, ms}`;
   - `checks`: `[{name, ms, passed}]`.

   Rules: null when unknown, never 0. Invariant: prompt + model + tools + checks + other = `duration_ms`.
2. `workspace {reused, warm_build_trees}` on both.
3. `LiveToolCalls` records when each tool call starts and ends, and its name. `tools_ms` merges overlapping calls.
   `model_ms` is the dispatch window minus `tools_ms`.
4. `gate_result.duration_ms`. `attempt_merged` after the accept. `spec_gate_started` / `spec_gate_finished` around the
   plan-load gate.
5. Tests:
   - a fake-provider attempt's verdict has `phases` that add up to `duration_ms`, with a tool call's time in
     `tools_ms`;
   - the hub event carries the same object;
   - `gate_result` carries its step's duration.

## Done when

- A Graph attempt's `attempts.jsonl` row and its `attempt_settled` event carry `phases` and `workspace`, and the
  phases add up to `duration_ms`.
- The `gate_result`, `attempt_merged` and spec gate events carry their durations.
- `cargo test -p roko-cli --lib attempt_phases` passes.

## Notes

- Per-step spec-gate progress (`spec_gate_step`) needs a progress callback in `spec_red_on_base.rs`, which gap-d21da9
  is changing now. It lands with gap-e02ec4.
- Plan-writing phases are left out of this item: `plan_generate/pipeline.rs` is being changed on `work/bug-27f00b`
  (roko-planner-2). The agreed fields are `PlanProgress.elapsed_ms` and `PlanProgress.phases` {lift_ms, planner_ms,
  repair_ms, validate_ms}, cumulative. They go in a follow-up once that branch merges.
- Field names are agreed with the Hermes plugin. Rename nothing without telling plugin-wow.
