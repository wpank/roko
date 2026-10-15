+++
id = "gap-6f31c3"
kind = "gap"
title = "L-sec and L-placebo draw arms on every attempt but the census never folds their rows, so the placebo check that flags a broken audit never runs"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-18"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::fold_run", "crates/roko-learn/src/loop_audit/census.rs::content_loop", "crates/roko-learn/src/loop_audit/census.rs::RunRecords", "crates/roko-learn/src/loop_audit/mod.rs::observe_census", "crates/roko-learn/src/loop_audit/arm_set.rs::ARM_SET_LOOPS"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["spec-c6e21b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn census_measures_l_sec_and_the_placebo' crates/roko-learn/ && cargo test -p roko-learn census_measures_l_sec_and_the_placebo"

[[verify]]
command = "grep -rqw 'fn placebo_influence_trips_audit_broken' crates/roko-learn/ && cargo test -p roko-learn placebo_influence_trips_audit_broken"
+++

## Problem

Every Graph attempt draws arms for L-sec and L-placebo (`crates/roko-learn/src/loop_audit/arm_set.rs:28`
`ARM_SET_LOOPS`), and the run logs their rows, but the M2 census never measures either loop, so neither gets a
`loop.health` row and the placebo check never runs.

- `census.rs::fold_run` folds `decisions`, `content_decisions` and `harness_decisions`, but never
  `RunRecords::placebo_decisions` (one row per attempt, `decision_point = "placebo"`, `loop_id = "L-placebo"`;
  `graph_task_dispatch/decision_log.rs` test `placebo_decisions_have_identical_proposals`).
- `census.rs::content_loop` maps only `Knowledge` to L-know and `Playbooks` to L-play; `Sections` rows map to no loop
  unless the row names one, so L-sec has no tally.
- `LoopAuditor::observe_census` (`loop_audit/mod.rs`) builds `AuditorSignals` from the SRM alarm only; nothing turns a
  placebo that shows influence or benefit into `audit_broken`.

Seen live (E2b RG4): the 10-14 ledger has 44 placebo decision rows and section draws, but `loop.health` rows only for
L-know and L-route, in all 7 real workspaces.

## Why it matters

Goal `cybernetic` (S03 §4.3/§4.6: the placebo is the auditor's own control; a placebo that "helps" means the audit is
broken and must freeze every transition). NEW-FINDINGS finding 18. Before β is estimated (gap-03fcd0) and long before
enforce, the placebo check has to work, or a broken estimator could demote a good loop. Stage 1 of the epic.

## Where

- `crates/roko-learn/src/loop_audit/census.rs::fold_run`, `content_loop`, `RunRecords` (`placebo_decisions`).
- `crates/roko-learn/src/loop_audit/mod.rs::observe_census` (`AuditorSignals`, `audit_broken`).
- `crates/roko-learn/src/loop_audit/arm_set.rs::ARM_SET_LOOPS`; the registry entries `L-sec` and `L-placebo` in
  `crates/roko-learn/src/loop_audit/loops.toml`.
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs`: writes the rows.

## Current state

5108 built the layers and the placebo; 5123 built the measured census; 5135 measures L-M1/L-M3/L-M4. The placebo
and section rows were never added to the fold.

## Plan

1. Fold `placebo_decisions` into an L-placebo tally (exposure, ι over the identical proposals, which should be 0).
2. Give section rows their loop: map `ContentDecisionPoint::Sections` to L-sec in `content_loop`, or have the writer set
   `audit.loop_id = "L-sec"`, and fold them like L-know.
3. In `observe_census`, set the placebo signal: when L-placebo's influence (and, after gap-03fcd0, benefit) interval
   excludes 0, record `audit_broken` (the ledger's `*` row) and write `placebo_ok: false` on every health row.
4. Tests `census_measures_l_sec_and_the_placebo` (the loop-census fixture or a synthetic run gives both a health row)
   and `placebo_influence_trips_audit_broken`.

## Done when

Both `[[verify]]` commands pass, and a fake-provider plan run's ledger has `loop.health` rows for L-sec and L-placebo.

## Notes

Do not change what either loop executes; this is measurement only. Enforce stays off (D10).
