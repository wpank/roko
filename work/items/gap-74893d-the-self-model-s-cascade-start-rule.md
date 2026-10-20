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

[[verify]]
command = "grep -rqw 'fn the_start_rule_weighs_time_on_phase_one_forecasts' crates/roko-learn/src && cargo test -p roko-learn --lib the_start_rule_weighs_time_on_phase_one_forecasts"
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

Owner: roko-speed (round v7). The design note is in `tmp/hermes-roko-v7/findings/roko-speed.md`, "Design note C"
(2026-10-20). The lead approved the objective: the exact cascade cost, with a price on time.

The rule it changes is S04 §4.4's start rule, as decision 6101 (B) adopted it: the first rung j with
p_vs_lcb(j) > c_j/c_{j+1} + m_b, else the last rung.

**The objective:** start on the rung with the lowest expected total, cost plus λ·time, to a verified pass.
- One try on rung k costs g_k = E[c_k] + λ·E[t_k], from the log-normal means of the forecast's q50 and q90.
- A task started on rung j climbs one rung per failure, at most K_max = 2 times. The highest rung it can reach is
  retried until it passes: G_j = g_j + (1 − p_j)·G_{j+1}, ending in g/p, with p the forecast P(VS).
- The lowest G_j wins, and ties go to the higher rung. With no finite G, it is the last rung.

**Config** (`[self_model]`):
- `start_rule = "expected_total"` (the default), or `"break_even"`, which restores today's rule exactly;
- `time_value_usd_per_hour` (λ, default 3). `0` weighs cost alone.

**Not in this item:** `after_failure` (6131) keeps E[c]/p, which is a follow-up. The replay keeps S04's rule, since
its matrix has no time.

## Done when

On Undertow's phase-1 forecasts for T3's first try (run `0bed5c58`), `break_even` and λ = 0 start on cheap. λ =
$3/hour starts on strong: $0.99 to a verified pass, against cheap's $1.08 and mid's $1.12. Checked by the test in
`[[verify]]`. The start-rung demo harness (round v6) passes the same conditions before and after, or the change is
explained.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/self-model-shadow-p0-p1.txt`. Phase 1's per-try times are in
  `tmp/hermes-roko-v7/findings/hermes-pilot.md`, section G (from `tmp/hermes-roko-v7/pilot/tooling/trytimes.py`).
- Both linked items bear on this: strong's P(VS) is understated (bug-a35286), and the fallback is the open finding
  (find-816b4b).
