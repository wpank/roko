+++
id = "gap-addf2a"
kind = "gap"
title = "Loop-audit census re-reads all history each close, and LoopAuditor's enforcement never reaches arm assignment"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop-audit", "roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK43 gap-c1d920)"
discovered_from = "gap-c1d920"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::read_runs", "crates/roko-learn/src/loop_audit/mod.rs::LoopAuditor", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::RunAttempts"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn arm_assignment_consults_the_loop_auditor' crates/roko-cli/ && cargo test -p roko-cli arm_assignment_consults_the_loop_auditor"
+++

## Problem

Two structural gaps in the loop-audit census, surfaced by PK43's work (gap-c1d920, done):

1. **The census tick re-reads every run from scratch at each plan-run close.** `crates/roko-learn/src/loop_audit/census.rs::read_runs` reads every directory under `runs_dir` fresh on every call (`std::fs::read_dir(runs_dir)`, no filtering by what's already been processed, no cached/incremental state). As runs accumulate under `.roko/runs/`, each plan-run close pays the cost of re-reading and re-scoring the *entire* history again, not just the newly-closed run.
2. **Arm assignment at attempt open doesn't consult `LoopAuditor`.** `LoopAuditor::layer_spec` and `::executed_policy` (`crates/roko-learn/src/loop_audit/mod.rs:354,384`) are the methods that should tell a run, at the point it's assigning a chain's arms, what the auditor currently says about each loop (its live/observe-only/retired state, and whether it's tripped — see the companion item for (b), the "nothing clears a trip" half of this same report). Confirmed: neither the arm-assignment path (`crates/roko-cli/src/graph_task_dispatch/attempt.rs::arm_set`/`ArmSet::assign`, from earlier batches' research) nor `crates/roko-cli/src/dispatch/model_routing.rs` calls either method — nothing in production actually enforces a demotion the auditor has decided on.

## Why it matters

Goal: cybernetic (M2 loop liveness, S02/S03). (1) means the census's cost grows with total run history, not with new activity — a scaling problem for any long-lived workspace. (2) means the auditor's own enforcement decisions (which loop should be demoted, retired, or held) have no actual effect on what arms get assigned at dispatch time — the auditor can conclude a loop should stop acting, but nothing wires that conclusion back into the one place (arm assignment) that would make it stick.

## Where

- `crates/roko-learn/src/loop_audit/census.rs::read_runs`.
- `crates/roko-learn/src/loop_audit/mod.rs::LoopAuditor::layer_spec`, `::executed_policy`.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::arm_set` (where a chain's arms get assigned, and where
  the auditor's decision would need to be consulted).

## Current state

Both confirmed as described; neither addressed.

## Plan

1. Give the census a way to process only runs newer than its last processed state (a watermark, or per-run
   processed-marker files), so a plan-run close's cost scales with new activity, not total history.
2. Wire `arm_set` (or wherever arms are actually chosen) to call `LoopAuditor::layer_spec`/`executed_policy`
   before assigning, so a demoted/retired/tripped loop's enforcement decision actually changes what happens at
   dispatch time, not just what the census reports after the fact.

## Done when

- A plan-run close's census cost does not grow with total run history under `.roko/runs/`.
- Arm assignment reflects what `LoopAuditor` currently says about the loop it's drawing for.
- The `[[verify]]` command passes.

## Notes

- Related but distinct: the companion item for sub-finding (b) of the same PK43 report — a tripped audit state
  (SRM alarm, placebo move) has no way to clear once set. That's about the trip/clear lifecycle; this item is
  about the census's read cost and about enforcement actually reaching arm assignment.
