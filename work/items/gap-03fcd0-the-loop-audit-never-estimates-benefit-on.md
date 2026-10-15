+++
id = "gap-03fcd0"
kind = "gap"
title = "The loop audit never estimates benefit β on any path, so no loop can go live, null or harm and the census can never say live"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "L"
subsystem = ["roko-learn/loop_audit", "roko-cli/learn"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-10"
anchors = ["crates/roko-learn/src/loop_audit/mod.rs::beta_fields", "crates/roko-learn/src/loop_audit/mod.rs::loop_evidence", "crates/roko-learn/src/loop_audit/mod.rs::observe_census", "crates/roko-learn/src/loop_audit/estimators.rs", "crates/roko-learn/src/loop_audit/cs.rs::ipw_difference", "crates/roko-learn/src/loop_audit/census.rs::census_row"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = ["gap-6f31c3"], blocks = [], related = ["spec-c6e21b", "gap-0132c2", "gap-50346f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn census_estimates_beta_once_n_beta_opportunities' crates/roko-learn/ && cargo test -p roko-learn census_estimates_beta_once_n_beta_opportunities"

[[verify]]
command = "grep -rqw 'fn beta_takes_an_oracle_loop_live_and_a_harmful_one_to_harm' crates/roko-learn/ && cargo test -p roko-learn beta_takes_an_oracle_loop_live_and_a_harmful_one_to_harm"
+++

## Problem

The M2 loop audit never estimates benefit β on any code path. `beta_fields` (`crates/roko-learn/src/loop_audit/
mod.rs:735-749`) always returns `est: None`, `lcb: None`, `ucb: None` with a reason (`eps_below_min`,
`n_below_n_beta` or `not_estimated`), whatever n is. `loop_evidence` (:697) leaves `benefit` at its default, and
`observe_census` says so in its doc (:563-566: "β is not estimated yet, so no loop goes live, `null` or `harm`
here"). The benefit estimators built by 5113 (`loop_audit/estimators.rs` `dim`/`cuped`/`aipw` :468-480,
`cs.rs::ipw_difference` :280) have no caller outside their tests and the E2 simulator. `census_row` only ever assigns
`Flagged` or `Probation`.

Seen live: every `loop.health` row in the 7 real workspaces (2026-10-09..14) has β null, all in probation, zero
transitions (A06 §5, E2b RG4).

## Why it matters

Goal `cybernetic` (S03 §4.4; paper 9-16, 9-18, 9-20 and D7 A.10, D.14). NEW-FINDINGS finding 10. Without β: no loop can
be judged to help or harm; the census can never say `live`; ViabilityBench's `requires_live`
(`benchmarks/viabilitybench/driver/campaign.py:69-80`) refuses every live manifest; E3/C5 (oracle playbook live,
harmful one demoted, placebo still) cannot pass. Stage 3 of the epic: measure before anyone turns enforce on.

## Where

- `crates/roko-learn/src/loop_audit/mod.rs::beta_fields`, `loop_evidence`, `observe_census`.
- `crates/roko-learn/src/loop_audit/estimators.rs` (`dim`, `cuped`, `aipw`) and `cs.rs::ipw_difference`.
- `crates/roko-learn/src/loop_audit/census.rs::census_row` and the tallies' opportunities (they carry the arm and its
  propensity; the outcome label comes from the run's verdicts).
- Stale CLI text: `crates/roko-cli/src/commands/learn.rs` (`--census` "until the measured audit (5123) lands", and the
  "not built yet" log), `learn_loops.rs` "time to detection: not measured".

## Current state

ε and ι are measured in production. 5113 (estimators) and 5126 (the tick) were each done without β; no item owned
the join.

## Plan

1. Join each opportunity to its attempt's learning label (the verdict's label; no label, no outcome) in the census
   fold.
2. Estimate β per loop as the learned-minus-default difference with IPW at the logged propensities (Hájek), CUPED when
   the covariate is present, and a confidence sequence at α/K for `lcb`/`ucb` (S03 §4.4). Guard 2 stays: no estimate
   below ε_min or N_β, with that reason.
3. Fill `LoopEvidence::benefit` so the state machine can reach `live`, `null` and `harm`. Enforce stays off (D10), so
   nothing is demoted; the ledger records the states.
4. Fix the stale CLI text.
5. Tests: `census_estimates_beta_once_n_beta_opportunities` (synthetic runs: none below N_β with its reason; an estimate
   with an interval above it) and `beta_takes_an_oracle_loop_live_and_a_harmful_one_to_harm` ($0 version of E3/C5, with
   the placebo staying `null`).

## Done when

Both `[[verify]]` commands pass, and `roko learn loops --json` on a workspace with enough synthetic runs shows a
non-null β.

## Notes

- Depends on gap-6f31c3: the placebo check must work before β can move a loop.
- gap-0132c2: at default settings the EB radius stays near 2300/n, so real transitions take a long time; tuning power is
  that item's job, not this one's.
- `h5_live.toml`/`h6_live.toml` require loop ids that are not registered (L-audit, L-gate-depth, L-route-trust); that is
  slice 33's (A04), not this item's.
- Loop-audit enforce is stage 4 and Will's call.
