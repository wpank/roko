+++
id = "gap-b856d3"
kind = "gap"
title = "The all-rungs refusal is only a RokoError string: give the refused try a typed outcome, all_rungs_skipped, in its verdict and the run summary, so a host can show it and offer a resume only when a recharge can fix it (gap-20301d's follow-up)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-serve/routes"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-lead#all-rungs-skipped-kind"
discovered_from = "agent:team-lead (round v7, 2026-10-26: gap-20301d's approval, item 2)"
anchors = ["crates/roko-learn/src/telemetry/records.rs::AttemptOutcome", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::unroutable", "crates/roko-serve/src/routes/runs.rs::run_summary"]
links = { depends_on = [], blocks = [], related = ["gap-20301d", "bug-374384"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_the_ladder_routes_is_refused_when_every_rung_failed_its_probe' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_task_the_ladder_routes_is_refused_when_every_rung_failed_its_probe"

[[verify]]
command = "grep -rqw 'fn a_runs_summary_names_an_all_rungs_skipped_refusal' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_runs_summary_names_an_all_rungs_skipped_refusal"
+++

## Problem

gap-20301d refuses a task the ladder routes when every rung failed its tool-use probe at plan start. That refusal is
only a `RokoError::Rejected` string. The try's verdict records it as `harness_error`, the same as any harness fault,
so nothing typed tells a host what happened:
- plugin-wow can't show it the way its P-49 handles a refused run;
- the plugin can't tell whether a resume can work. Since bug-374384, a rung refused on billing is probed again on the
  next run, so a resume after a recharge works. Any other probe failure stands for a day.

## Where

- `crates/roko-learn/src/telemetry/records.rs`: `AttemptOutcome` and its blame.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs`: `unroutable`.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs`: `Settlement::harness_failure`.
- `crates/roko-serve/src/routes/runs.rs`: `run_summary`, which reads the `attempt_settled` events.

## Plan

1. **A new outcome.** `AttemptOutcome::AllRungsSkipped`, `all_rungs_skipped` on the wire. It is charged to the
   harness and carries no learning label, as `harness_error` does now. The refused try's verdict and its
   `attempt_settled` carry it.
2. **The run summary.** When a try was refused this way, the run's `refused` names it:
   - `stage: "ladder"`, `kind: "all_rungs_skipped"`, `reason`: the refusal's first line;
   - `billing_only`: whether every skipped rung failed on its provider's billing, so that a recharge and a resume
     fix it.

## Done when

The refusal test shows the try's verdict with outcome `all_rungs_skipped`. A run summary over such a try names it
under `refused`, with `billing_only`.
