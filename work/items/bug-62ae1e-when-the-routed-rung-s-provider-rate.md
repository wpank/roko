+++
id = "bug-62ae1e"
kind = "bug"
title = "When the routed rung's provider rate-limits every try, each try fails over to the rung above under the routed rung's 20-turn bound and its failure never counts toward a climb, so the task loops on substitutes until exhausted (phase 2.2 run 20c780ae: 5 substitute tries on gpt-5.4-mini, all at mid's bound, none counted; T2 passed on its last try)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#L-p2-2-run-20c780ae"
discovered_from = "agent:hermes-pilot (round v7 phase 2.2 run 20c780ae; phase 2.1 run d4a4810d)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::note_ladder_outcome", "crates/roko-agent/src/rate_limit.rs::ProviderRateLimiter"]
links = { depends_on = [], blocks = [], related = ["bug-4b6844", "gap-ff2c38", "gap-460230"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_rate_limit_failover_try_runs_under_its_substitute_rungs_bounds' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_rate_limit_failover_try_runs_under_its_substitute_rungs_bounds"

[[verify]]
command = "grep -rqw 'fn repeated_rate_limit_failovers_count_toward_the_climb' crates/roko-cli/src && cargo test -p roko-cli --lib -- repeated_rate_limit_failovers_count_toward_the_climb"

[[verify]]
command = "grep -rqw 'fn a_held_call_waits_for_a_slot_instead_of_a_substitute' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_held_call_waits_for_a_slot_instead_of_a_substitute"
+++

## Problem

When the routed rung's provider rate-limits every try, the task loops on failover substitutes until it is exhausted.
Each try fails over to the rung above but keeps the routed rung's tight bound. Its failure doesn't count toward a
climb. So the task never gets its own try on the rung above, with that rung's bounds, and it never waits for the
model it was routed to.

Round v7, Undertow phase 2.2, run `20c780ae` (roko e4643dafd, plan `p2-2-undertow-gain-staging`, 3 parallel tasks,
all routed to mid = glm-4.7 on Z.ai). Z.ai refused T2's and T3's calls with 429s, then roko opened its circuit
("glm-4.7 on zai passed over (circuit_open)"). Every refused try failed over within the try to strong
(gpt-5.4-mini):

| Try | Ran | Turns | Ended |
|---|---|---|---|
| T3#1 | strong / failover | 20 (mid's bound) | gate_failed, 4 of 12 assertions |
| T3#2 | strong / failover | 20 | gate_failed, 10 of 12 |
| T2#1 | 2 min on glm-4.7, then strong / failover | 20 | gate_failed, 16 of 18 |
| T2#2 | strong / failover | 20 | gate_failed, 16 of 18 |
| T3#3 | mid / start, glm-4.7 | 19 | passed |
| T2#3 | strong / failover (circuit_open) | 20 | gate_failed: broke P1's filter decay |
| T1#1 | mid / start, glm-4.7 | 28 | passed |
| T2#4 | mid / start, glm-4.7 | 13 | passed, its last try (max_retries 3) |

- The summary's `rungs[]` reads strong 5 tries, 0 passes, 5 agent failures. All five were substitutes at mid's 20-turn
  bound, and none counted toward a climb. T2 and T3 stayed routed to mid, and passed only when Z.ai let glm-4.7
  through. One more refusal and T2 would have been exhausted.
- **Z.ai seems to take about two sessions at once:**
  - In phase 2.1 (run `d4a4810d`), 3 of 5 parallel starts got 429 within 2 s.
  - Here T1 held a slot and ran for 19 min, while T2 and T3 could not get in.
  - roko's limiter learns this (`ProviderRateLimiter::admit` holds a call back as a rate limit while the provider is
    at the calls it showed it takes). The failover then switches models instead of waiting for the slot.

## Where

- `crates/roko-cli/src/graph_task_dispatch/failover.rs` (~line 540): a rate-limited attempt "fails over within it"
  whenever a candidate exists. It waits ("failover has nowhere to send it; waiting inside the attempt") only when none
  does. Failover goes up the role's runnable rungs (decision 1119, backlog 1120).
- `crates/roko-agent/src/rate_limit.rs` (`ProviderRateLimiter::admit`, the learned calls-at-once limit,
  `LEARNED_LIMIT_HOLDS_FOR`) and `crates/roko-agent/src/openai_compat_backend.rs` (~line 427, "holding a call back:
  its provider rate-limited one…"): a held-back call surfaces as a rate limit, so dispatch fails it over at once.
- `crates/roko-cli/src/graph_task_dispatch/streaming.rs` (~line 224) and `graph_task_dispatch.rs` (~line 1506):
  `rung_allowance` is computed from the routed `dispatch_plan` before dispatch. So a failover substitute runs under
  the routed rung's turns and time (`max_turns = max_turns.min(turns)`).
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::note_ladder_outcome` (~line 577): "attempts a failover
  substitute ran in place of the rung's model (backlog 1118), change nothing", so the task never climbs.

## Plan

roko-ladder's design, approved by the lead on 2026-10-21. From the workspace's `.roko/roko.log.2026-10-20`:
- Only T3's first refusal was a real 429.
- After it the limiter learned zai's limit of 2 calls at once (bug-4b6844). Every later refusal was our own hold
  ("holding a call back ... in_flight=2 limit=2", 16 times), while T1's and another task's glm-4.7 sessions were in
  flight.
- The tool loop's three quick retries ran out in about 2 s, so dispatch failed over at once.
- The holds also counted as provider failures, which is why zai's circuit opened for T2's try 3.

1. **Wait for a slot instead of switching.** In `run_bridge_with_failover`'s rate-limit branch, a new limiter query,
   `ProviderRateLimiter::own_calls(endpoint)`, says whether the run's own calls are in flight to the routed model's
   endpoint while a rate limit from the last minute says how many it takes at once. If they fill it, the model frees
   up as soon as one of them ends, between their turns. The try waits for a free slot, polling every 2 s and marked
   queued, so neither the stall watchdog nor the try's time bound counts the wait. Then it calls the same model
   again (at once, if one of them ended already).
   - Bound: `[routing.ladder] rate_limit_wait_secs`, default 600 s, over the attempt. Only after it does the try
     fail over.
   - A refusal with nothing else of ours in flight (an account limit) keeps today's Retry-After / failover behaviour.
2. **A held call is not a provider failure.** A rate limit while the run's own calls are in flight to the provider
   under such a limit is not recorded against its circuit (`record_provider_outcome`), so it can't open the circuit.
3. **The substitute's own bounds.** When failover hands the try to a model that is a ladder rung, the try's turns,
   spend and time allowance are recomputed for that rung (as `rung_allowance` does, for the rung the substitute
   is), and the try's bounded-stop check uses them. A substitute that isn't on the ladder keeps the routed rung's
   bounds.
4. **Rate-limit failovers count toward the climb.** In `note_ladder_outcome`, a failed try whose routed model was
   refused for `rate_limited` or `circuit_open`, and that a substitute ran, counts as a failure on the routed rung. Two
   of them climb the task. Other refusal classes (exhausted, billing, credentials) keep today's rule.

## Done when

A task whose routed rung's provider rate-limits every try either waits for that model (bounded) or climbs, and a
failover try runs under its substitute rung's bounds. Checked by the tests in `[[verify]]`.
