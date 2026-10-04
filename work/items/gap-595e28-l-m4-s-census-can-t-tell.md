+++
id = "gap-595e28"
kind = "gap"
title = "L-M4's census can't tell a lost exclusion decision from no opportunity, and its DP3 feedback isn't on run rows"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop-audit", "roko-learn/telemetry"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-10 follow-up reports 2026-10-04 (PK65 gap-4cbd80)"
discovered_from = "gap-4cbd80"
anchors = ["crates/roko-learn/src/loop_audit/loops.toml", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::load_audit_trust", "crates/roko-learn/src/telemetry/records.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn l_m4_census_distinguishes_unlogged_from_no_opportunity' crates/roko-learn/ && cargo test -p roko-learn l_m4_census_distinguishes_unlogged_from_no_opportunity"
+++

## Problem

Two gaps in L-M4's census coverage, from PK65's work (gap-4cbd80, done):

1. **L-M4's census "opportunity" exists only when DP4's exclusion is successfully logged.** `loops.toml`'s
   `L-M4` entry (`crates/roko-learn/src/loop_audit/loops.toml:414-423`): "S05's feedback shows in later route
   rows: DP4's audit trust leaves a candidate out with `ineligible_reason audit_trust` (7131); the census
   measures ε from those rows." The census's whole measurement of L-M4's activity depends on finding a route
   row carrying that `ineligible_reason`. If DP4 decides to exclude a model but, for any reason, that decision
   never reaches a route row (a write failure, an early return, a path that skips logging), the census sees zero
   opportunities and reports "no opportunity," not "unlogged" — the same *symptom* as the loop genuinely never
   having anything to do, indistinguishable from a real logging gap. (The census framework already has a
   distinct `ReasonCode::Unlogged` for exactly this class of problem elsewhere, e.g. L-know's retrieval count —
   L-M4 has no equivalent cross-check.)
2. **L-M4's feedback through DP3's verify-depth ladder isn't on run rows yet.** No `verify_depth`/`rung_depth`-
   style field exists anywhere in `crates/roko-learn/src/telemetry/records.rs` (confirmed: zero matches). So
   even once DP4's exclusion decisions reach route rows correctly, there's no way for a run's own rows to show
   whether/how that fed into DP3's verify-depth ladder (V0-V4) for the affected task — the loop's downstream
   effect on verify depth is invisible at the run-row level.

## Why it matters

Goal: cybernetic, M4 deep audits (S05). (1) means L-M4's census reading can't currently distinguish "healthy,
nothing to exclude" from "broken, losing exclusion decisions" — exactly the ambiguity the census's `Unlogged`
reason code exists to resolve for other loops. (2) means the DP3/DP4 feedback relationship (audit trust
informing verify depth) has no observable trace in the per-run record, making it hard to audit or debug after
the fact.

## Where

- `crates/roko-learn/src/loop_audit/loops.toml::L-M4` (the opportunity definition).
- `crates/roko-learn/src/loop_audit/census.rs` (where an `Unlogged`-style cross-check would need to live, same
  pattern as other loops' checks).
- `crates/roko-cli/src/graph_task_dispatch/routing_context.rs::load_audit_trust`,
  `crates/roko-learn/src/cascade_router.rs::set_audit_trust` (where the exclusion decision is made and would
  need to be independently cross-checked for logging gaps).
- `crates/roko-learn/src/telemetry/records.rs` (where a verify-depth field would need to be added for (2)).

## Current state

Both confirmed as described; neither addressed.

## Plan

1. For (1): find an independent signal that a DP4 exclusion decision was *made* (even if the route-row write
   failed), so the census can tell "no opportunity" apart from "opportunity lost before logging" — mirroring
   whatever mechanism backs L-know's `Unlogged` check.
2. For (2): add a verify-depth field to the run-row schema, populated wherever DP3's ladder actually sets a
   task's verify depth, so L-M4's downstream effect becomes visible per run.

## Done when

- L-M4's census can distinguish "no opportunity" from "an exclusion decision was lost before logging."
- A run's rows show the verify depth DP3's ladder set for each task.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK65's work (gap-4cbd80, done).
