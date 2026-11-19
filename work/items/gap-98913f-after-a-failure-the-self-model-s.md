+++
id = "gap-98913f"
kind = "gap"
title = "After a failure the self-model's cascade still weighs cost alone (cascade::after_failure, 6131: E[c]/p), while its start rule weighs time since gap-74893d: on Undertow's phase-1 forecasts a task that fails on mid retries mid ($0.27 against $0.53 to a verified pass) where, with time priced at $3/h, it would climb to strong ($0.99 against $1.19)"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-speed#C-follow-up"
discovered_from = "agent:roko-speed (round v7, gap-74893d design note; the lead approved it as a follow-up)"
anchors = ["crates/roko-learn/src/self_model/cascade.rs::after_failure", "crates/roko-learn/src/self_model/cascade.rs::cost_to_vs"]
links = { depends_on = [], blocks = [], related = ["gap-74893d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn after_failure_weighs_time_like_the_start_rule' crates/roko-learn/src && cargo test -p roko-learn --lib after_failure_weighs_time_like_the_start_rule"
+++

## Problem

Since gap-74893d, the self-model's cascade starts a task on the rung with the lowest expected total, cost plus λ·time,
to a verified pass (`[self_model] start_rule = "expected_total"`, λ = `time_value_usd_per_hour`, default $3). After
an agent-blamed failure, though, `cascade::after_failure` (6131) still picks retry, climb or skip by expected cost to
VS alone: `cost_to_vs` = E[c]/p.

The two rules then disagree on the same forecasts. On Undertow's phase-1 forecasts for T3 (round v7, run `0bed5c58`):
- the start rule at $3/h starts on strong;
- a task the ladder started on mid that fails there retries mid (E[c]/p $0.27) instead of climbing to strong ($0.53);
- with time priced the same way, the climb wins: $0.99 against mid's $1.19 (g/p, with mid tries of about 12 minutes
  against strong's 1.4).

## Why it matters

In active mode the self-model decides both steps for the chains it starts. A start rule that weighs time, followed by
a climb rule that doesn't, can start a task fast and then keep it on a slow rung after a failure. It is one cascade
and should have one objective.

## Where

- `crates/roko-learn/src/self_model/cascade.rs`: `after_failure` and `cost_to_vs`. `expected_total` (g = E[c] + λ·E[t])
  exists since gap-74893d.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs`: `decide` and `post_failure_step`, which call
  `after_failure`.
- `[self_model] start_rule`: `break_even` restores S04 §4.4's start rule exactly. The same switch, or one beside it,
  should restore today's E[c]/p step.

## Plan

1. Give `after_failure` the price on time: compare g/p (with the conditional P(VS) where the data hold it) instead of
   E[c]/p, at the same λ.
2. Keep today's step under `start_rule = "break_even"`, so one value still restores S04's cascade as written.
3. Log both steps' picks in the prediction row, as gap-74893d does for the start.

## Done when

On Undertow's phase-1 T3 forecasts, a failure on mid climbs to strong at $3/h and retries mid at λ = 0 and under
`break_even`. Checked by the test in `[[verify]]`.

## Notes

- Approved as a follow-up by the round v7 lead on 2026-10-20, when gap-74893d kept `after_failure` out of scope.
- The replay (`roko learn self-model replay`) keeps S04's rules: its matrix has no time.
