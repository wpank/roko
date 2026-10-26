+++
id = "gap-d91691"
kind = "gap"
title = "Follow-ups the lead approved for the ladder stack: c96097's early stop still exempts billing failures that bug-374384 now re-probes, an all_rungs_skipped try still counts as a rung try and folds into M1, and bug-bce0ed's prior fallback goes unrecorded and an abandon needs no agreement from the prior"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-learn/self_model"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-lead#ladder-stack-followups"
discovered_from = "agent:team-lead and agent:roko-exact (round v7, 2026-10-26: approvals of bug-374384, gap-b856d3 and bug-bce0ed that crossed with merge c8f9bff80)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::unrunnable_ladder", "crates/roko-learn/src/telemetry/rung_stats.rs::tally_run", "crates/roko-cli/src/runtime_feedback/homeostasis.rs::settle", "crates/roko-learn/src/self_model/policy.rs"]
links = { depends_on = [], blocks = [], related = ["bug-374384", "gap-c96097", "gap-b856d3", "bug-bce0ed"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_abandon_needs_the_prior_to_agree' crates/roko-learn/src && cargo test -p roko-learn --lib -- an_abandon_needs_the_prior_to_agree a_runaway_retry_forecast_is_recorded_as_a_prior_fallback a_try_refused_for_no_rung_is_no_rung_try"

[[verify]]
command = "grep -rqw 'fn the_rung_probes_run_before_the_red_on_base_pre_check' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_rung_probes_run_before_the_red_on_base_pre_check"
+++

## Problem

The ladder stack merged in c8f9bff80 (bug-374384, bug-e33b7e, gap-b856d3, bug-bce0ed). The approvals of the lead and
roko-exact crossed with that merge, and asked for four things the merge doesn't do.

1. **c96097's early stop still exempts billing failures.** `unrunnable_ladder` skips its stop when any rung failed
   on billing, because a day-old verdict could outlive a recharge. Since bug-374384, a billing failure from an earlier
   run is probed again before the early stop reads it. A billing failure the stop sees is now this run's, and
   stopping before the red-on-base pre-check is right: the person has to recharge either way (roko-exact).
2. **An `all_rungs_skipped` try is still counted somewhere.** Every learner that reads `learning_label` already skips
   it: the self-model, routing, and prompt experiments. Audits draw only gate verdicts. Two places still count it:
   - the rung tallies (`rung_stats::rows`, the run summary's `rungs`, the end-of-run readout), where it is a try in a
     "(no rung)" row with no model;
   - M1's chain fold (`runtime_feedback::homeostasis`), where a chain that never ran would read as a failed
     resolution.
3. **bug-bce0ed's prior fallback is unrecorded.** A retry whose raw L1 forecast ran away takes L0's rate, but the
   prediction row says `basis: prior`, the same as a first try.
4. **An abandon needs no agreement from the prior.** The lead approved "abandon only when the prior agrees". Today
   an abandon follows L1 alone on a retry.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `unrunnable_ladder` and its test.
- `crates/roko-learn/src/telemetry/rung_stats.rs`: `tally_run`.
- `crates/roko-cli/src/runtime_feedback/homeostasis.rs`: `settle`.
- `crates/roko-learn/src/telemetry/records.rs`: `ForecastBasis`.
- `crates/roko-learn/src/self_model/model.rs`: `forecast_arm`.
- `crates/roko-learn/src/self_model/policy.rs` and `cascade.rs`: where they decide to abandon.

## Plan

1. Drop the billing exemption from `unrunnable_ladder`. Its test's billed case, a billing failure probed in this run,
   then expects the stop.
2. Leave `all_rungs_skipped` tries out of the rung tallies and out of M1's fold.
3. Add `ForecastBasis::PriorFallback` (`prior_fallback`) for a retry whose L1 forecast ran away.
4. Give each candidate its prior's P(VS), `prior_p_vs`.
   - An abandon then needs the prior to agree too: no arm's prior P(VS) reaches p_abandon. Otherwise the arm the
     prior likes best runs, flagged below target.
   - The policy and the cascade check the same.
   - The agreement uses the prior's point forecast, not its lower bound. A lower bound sits below p_abandon whenever
     data is thin, so it would agree with almost any abandon.

## Done when

- The early stop stops a run whose every rung failed this run on billing.
- An `all_rungs_skipped` try is in no rung row and resolves no M1 chain.
- A runaway forecast is recorded as `prior_fallback`.
- A retry the prior still rates at 0.6 is not abandoned however low L1 goes.
