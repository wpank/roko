+++
id = "gap-74893d"
kind = "gap"
title = "The self-model's cascade start rule (cascade::start_rung) weighs cost only, though every forecast carries latency (Undertow phase 1: strong lat_q50 40-67 s, mid 418-695 s; measured mid tries of 7.5-46 min against a 44 s strong pass); design first, owner roko-speed"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#H-self-model"
discovered_from = "agent:hermes-pilot (round v7, self-model shadow review of Undertow phases 0-1)"
anchors = ["crates/roko-learn/src/self_model/cascade.rs::start_rung", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::decide"]
links = { depends_on = [], blocks = [], related = ["bug-a35286", "find-816b4b"], supersedes = [], duplicate_of = "" }
+++

## Problem

The cascade start rule, `cascade::start_rung`, picks the first rung j whose P(VS) LCB exceeds
`expected_cost(j) / expected_cost(j+1) + BREAK_EVEN_MARGIN` (0.05), else the last rung. It weighs cost only. Each
candidate's forecast carries latency (`lat_q50_s`, `lat_q90_s`), but the start rule never reads it.

Undertow's numbers, from round v7:
- In phase 1 the self-model forecast strong (gpt-5.4-mini) at lat_q50 40–67 s and mid (glm-4.7) at 418–695 s.
- Measured: mid tries of 7.5–46 min (T3 744 s and 461 s, T4 1,347 s, T2 2,781 s). They were 96–99% model time, with
  build and check at most 28 s of each, against a 44 s, $0.02 strong pass (T1).

Since bug-f0007b (7268ba404), a mid try stops at 900 s. Even so, the start rule cannot trade one minute on strong for
15–46 minutes on mid.

## Why it matters

The pitch is that cheap, quick models do as much as possible. A start rule that can't see time starts a slow model
whenever it is cheaper per success, however long it takes, and the person waits.

## Where

- `crates/roko-learn/src/self_model/cascade.rs`: `start_rung`, `cost_to_vs` and `BREAK_EVEN_MARGIN`.
- `CandidateForecast`: the latency quantiles.
- The active start path, `crates/roko-cli/src/graph_task_dispatch/self_model.rs`: `decide` and `active_start`.

## Plan

Owner: roko-speed (round v7). Design comes first: before any code, a short note with the options and a proposed
objective. For example:
- (a) expected cost plus a price on time (USD per minute of wall time, set in `[self_model]`), so break-even compares
  cost + λ·latency;
- (b) a latency cap: skip a start rung whose lat_q90 exceeds a bound, unless its P(VS) margin is large;
- (c) minimise the expected time to a verified success within the cost budget, as a cascade over expected time with
  the same LCB test.

The verify command is written with the chosen objective.

## Done when

A design note, with the options and the chosen objective, is linked here. Then, on Undertow's phase-1 forecasts, the
start rule's choice reflects the latency the forecasts carry, checked by a test written with the design.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/self-model-shadow-p0-p1.txt`. Phase 1's per-try times are in
  `tmp/hermes-roko-v7/findings/hermes-pilot.md`, section G (from `tmp/hermes-roko-v7/pilot/tooling/trytimes.py`).
- Both linked items bear on this: strong's P(VS) is understated (bug-a35286), and the fallback is the open finding
  (find-816b4b).
