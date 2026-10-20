+++
id = "bug-a35286"
kind = "bug"
title = "A failover substitute's verified outcome never reaches the self-model's estimate for the model that ran it (runtime_feedback/self_model.rs unit_of: \"nobody chose it\"), so gpt-5.4-mini stays at 0 of 2 after Undertow phase 1's 44 s, $0.02 pass and is forecast at P(VS) 0.15"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/runtime_feedback", "roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#H-self-model"
discovered_from = "agent:hermes-pilot (round v7, self-model shadow review of Undertow phases 0-1)"
anchors = ["crates/roko-cli/src/runtime_feedback/self_model.rs::unit_of", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::settle"]
links = { depends_on = [], blocks = [], related = ["find-816b4b", "gap-74893d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failover_substitutes_verified_pass_counts_in_its_models_estimate' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_failover_substitutes_verified_pass_counts_in_its_models_estimate"
+++

## Problem

Round v7, Undertow phase 1 (run `0bed5c58`, roko f33fd217c; the code is unchanged on main). T1's first three tries on
mid (glm-4.7) were lost to Z.ai 429s. Its fourth try failed over to gpt-5.4-mini, the strong rung's model, and passed
its checks: 10 turns, 44 s, $0.0214. The self-model (M3, `[self_model] mode = "shadow"`) learned nothing from it.
`unit_of` in `runtime_feedback/self_model.rs` returns `None` for any verdict with a non-empty `failover_chain`: "neither
does an attempt a provider failover ran on a substitute model, since nobody chose it".

After phases 0 and 1, the saved state (`.roko/learn/self-model/state-v1.json`) holds 14 outcomes:
- the gpt-5.4-mini stratum is n 2.0, k 0: phase 0 run 1's T3 tries 3 and 4 on the JUCE plugin task, both failed;
- `history` has integrative|gpt-5.4-mini = 0.0.

So every phase 1 forecast after T1 put strong at P(VS) 0.15–0.19 (LCB 0.04–0.06), against mid's 0.67–0.78. The same
forecasts put strong at lat_q50 40–67 s and $0.05–0.06 a try, against mid's 418–695 s and $0.13–0.14.

## Why it matters

Keeping a substitute out of routing propensity and the policy's own statistics is right, since nobody chose it, so it
says nothing about a choice. But it is real evidence of what that model can do on this task family. Leaving it out
blinds the per-model success estimate to exactly the outcomes failover produces. Here it holds the fastest model that
passed at 0 of 2, so an active self-model would steer away from it. The start rule's items are gap-74893d (latency)
and find-816b4b (the fallback).

## Where

- `crates/roko-cli/src/runtime_feedback/self_model.rs`: `unit_of`, which returns `None` when `failover_chain` isn't
  empty, and `SelfModelOutcomeSink::on_event`.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs`: `settle`, `observe` and `record_outcome`. They feed both
  the per-model success estimate (the state's prior `strata` and `history`) and the routing and policy statistics (the
  calibration window, the `routed` flag and the policy's propensity-weighted updates).

## Current state

This is by design, from S01, per the comment at the top of `runtime_feedback/self_model.rs`. No part of the model sees
a failover substitute's outcome.

## Plan

1. Settle a failover substitute's verified outcome into the success estimate of the model that ran: its arm, as
   provider and slug as dispatched. Give it weight 1, since there is no propensity to divide by.
2. Keep it out of everything that assumes a routed choice: the calibration window, the `routed` flag and the policy's
   propensity-weighted statistics.
3. Mark the unit as a substitute, so a later fit can tell it apart.

## Done when

Replaying phase 1's records (run `0bed5c58`, after phase 0's runs) gives the gpt-5.4-mini stratum 1 of 3, and the
policy statistics (calibration window, routed counts) are unchanged. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/self-model-shadow-p0-p1.txt`. For every forecast in phases 0 and 1 it
  has P(VS), LCB, cost and latency per arm, beside the rung roko used and the result. Also Undertow's
  `.roko/learn/self-model/state-v1.json`.
- Since bug-4b6844, 429s no longer spend tries, but failover still happens when a provider is down.
