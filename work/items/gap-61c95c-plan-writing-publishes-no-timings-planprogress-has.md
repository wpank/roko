+++
id = "gap-61c95c"
kind = "gap"
title = "Plan writing publishes no timings: PlanProgress has no elapsed time or phase split, so a host can't say how long planning has run or where the time went"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#telemetry"
discovered_from = "agent:roko-speed (round v7, split out of gap-6a319c, 2026-10-19)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::PlanProgress"]
links = { depends_on = [], blocks = [], related = ["gap-6a319c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_progress_reports_its_elapsed_time_and_phases' crates/roko-cli/src && cargo test -p roko-cli --lib plan_progress_reports_its_elapsed_time_and_phases"
+++

## Problem

Plan writing measures its phases but publishes none of them. `generate_plan` times its init, context, prompt, agent
and post-processing phases and logs them (`plan generate: phase timing` in `plan_generate/pipeline.rs`).
`PlanProgress`, which roko serve shows in the operation's status, carries the stage, the attempt, the model, the calls
and the cost. It carries no time.

On round v7's Undertow phases, writing a plan took 16-90 s from Hermes. A host that shows the operation can say
"planning, call 2", but not how long the call has run or where the earlier time went.

## Why it matters

Plan writing is the first thing the person waits on in the dogfood loop, and the round's speed work needs to know
what to cut: the planner call, the repair pass or the validation. The Hermes plugin's plan-writing card wants the
same split it now gets for a try (gap-6a319c's `phases`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`:
  - `PlanProgress` and the `emit(PlanProgress { … })` calls;
  - the `plan generate: phase timing` log in `generate_plan`.
- roko serve's operation status, which serializes `PlanProgress`.

## Plan

1. Add `elapsed_ms` to `PlanProgress`: the time since the generation started.
2. Add `phases`, cumulative and null until a phase has run: `{lift_ms, planner_ms, repair_ms, validate_ms}`. These
   are the names agreed with plugin-wow in round v7.
3. Emit a last progress with the final phases when the plan is written, so a host can show the split after the
   operation ends.

## Done when

A generation's progress reports carry `elapsed_ms`, and `phases` that add up to at most `elapsed_ms`. The last report
names the planner's share. Checked by the test in `[[verify]]`.

## Notes

- Split out of gap-6a319c, whose per-try timing merged first. This part waited for roko-planner-2's `work/bug-27f00b`,
  which changed `pipeline.rs` and has merged.
- Field names are agreed with the Hermes plugin. Rename nothing without telling plugin-wow.
