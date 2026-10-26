+++
id = "bug-374384"
kind = "bug"
title = "A rung whose probe failed on billing stays skipped for a day after a recharge: probe_ladder trusts every fresh verdict, so a resume after recharging z.ai or Cerebras is still skipped or refused (gap-20301d, gap-c96097)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-lead#billing-reprobe"
discovered_from = "agent:team-lead and agent:roko-exact (round v7, 2026-10-26: gap-20301d merged without the billing re-probe the lead asked for)"
anchors = ["crates/roko-cli/src/dispatch/rung_probe.rs::probe_ladder", "crates/roko-cli/src/graph_execution/plan_runner.rs::billing_refusal", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::unroutable"]
links = { depends_on = [], blocks = [], related = ["gap-20301d", "gap-c96097", "bug-e33b7e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_rung_refused_on_billing_before_this_run_is_probed_again' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_rung_refused_on_billing_before_this_run_is_probed_again a_billing_failure_stands_only_in_its_run"
+++

## Problem

`probe_ladder` reuses every fresh rung-probe verdict for 24 h (`PROBE_TTL_MS`), billing failures included. So a rung
skipped because its provider refused on billing stays skipped for a day after a person recharges the account:
- a 402 from Cerebras;
- z.ai's 1113, insufficient balance;
- an exhausted plan.

The likeliest fix for such a rung is to recharge and press resume, and that resume:
- skips the rung again (gap-20301d's `rung_skipped`);
- with every rung refused on billing, refuses every task the ladder routes (gap-20301d);
- with no billing case, stops early (gap-c96097).

Earlier this round the pilot hand-edited `.roko/learn/rung-probes.json` to clear a bad gpt-5.4 verdict. The skip and
refusal messages also don't say how to probe a rung again now.

## Where

- `crates/roko-cli/src/dispatch/rung_probe.rs`: `probe_ladder` and `RungProbes::fresh`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the plan-start probe, `unrunnable_ladder` (gap-c96097), and
  `billing_refusal`.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs`: `unroutable`, the gap-20301d refusal.

## Plan

1. `billing_refusal` moves to `rung_probe` as the shared check. It covers an `insufficient_credits` or
   `provider_exhausted` class and `is_billing_message`, which matches a 402, z.ai's 1113 and an insufficient
   balance.
2. `probe_ladder` takes the run's start time.
   - A cached billing failure probed before the run started is probed again.
   - That gives one probe per run per rung: gap-c96097's early stop probes first, and the plan-start probe then
     reads this run's verdict.
   - A non-billing failure, such as a model that answers blank or refuses a parameter, still stands for a day.
3. The skip and the refusal say how to probe a rung again now.
   - **A billing failure:** "probed again when the next run starts: recharge and run again". Its `rung_skipped` has
     `billing: true` and no `until_ms`.
   - **Any other failure:** remove the model's entry from `.roko/learn/rung-probes.json`.

## Done when

A cached billing failure from before the run, on a provider that now answers the probe's tool call, is probed again
and the rung is used. A second probe in the same run reads that verdict without a call. A billing failure probed
within this run is trusted.
