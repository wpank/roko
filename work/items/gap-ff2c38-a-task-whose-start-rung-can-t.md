+++
id = "gap-ff2c38"
kind = "gap"
title = "A task whose start rung can't run starts on the highest runnable rung, with no rung left to climb: a chat run's architectural tasks (start top, claude_cli) ran all tries on strong and exhausted (Undertow phase 2 run 82b7e069: T2, T3 0/8 on gpt-5.4-mini)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-core/config", "roko-cli/dispatch"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-ladder#start-unreachable"
discovered_from = "agent:roko-ladder (round v7, run 82b7e069 analysis)"
anchors = ["crates/roko-core/src/config/routing.rs::LadderConfig::resolve", "crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::GraphTaskDispatcher::record_attempt_ladder"]
links = { depends_on = [], blocks = [], related = ["gap-cf427e", "gap-460230", "gap-3217a0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_unreachable_start_keeps_one_climb' crates/roko-core/ && grep -rqw 'fn a_try_on_an_unreachable_start_says_so' crates/roko-cli/ && cargo test -p roko-core --lib -- an_unreachable_start_keeps_one_climb && cargo test -p roko-cli --lib -- a_try_on_an_unreachable_start_says_so"
+++

## Problem

A task starts on the first runnable rung at or above its tier's start rung. When none at or above it can run, it
starts on the highest runnable rung, and then it has no rung left to climb.

A chat host's run drops every rung whose agent runs its own tools (`RoutingLadder::in_own_tool_loop`, gap-cf427e),
because its tasks hold outbound tool calls for approval. That drops `top` (claude-sonnet-4-6 through claude_cli). So
in a chat run, every architectural task (start `top`) lands on `strong`, the highest runnable rung, and spends all
of its tries there.

Undertow phase 2 (run `82b7e069`, roko d7d1fbb8b, from the Hermes chat):
- T2 and T3, both architectural, ran 4 tries each on strong (gpt-5.4-mini). Every verdict says
  `ladder.rung = "strong"`, `reason = "start"`, `step = 0`, and the last one `exhausted = true`.
- Both exhausted with `escalations: 0`, so T4 was blocked.
- Nothing in the records says the configured start couldn't run.

## Why it matters

The ladder's whole point is to escalate when a cheaper rung fails. An unreachable start silently turns it into "all
tries on one model", on a rung that has done worst on this work. On Undertow's run records (phases 0-2):
- mid (glm-4.7): 11 of 13 tries passed, and every integrative task passed on its first try with a verdict.
- strong (gpt-5.4-mini): 1 of 11, and 0 of 8 on phase 2's T2 and T3.
- The self-model's own shadow forecasts for T2 and T3's first try agree: P(verified success) 0.77 on glm-4.7, 0.44
  on gpt-5.4-mini (`predictions.jsonl`).

Starting one rung lower keeps one climb and starts on the cheaper model.

## Where

- `crates/roko-core/src/config/routing.rs::LadderConfig::resolve`: picks the start rung. It takes the first usable
  index at or above the requested one, else `usable.last()`.
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder`: `start`, `climb`, and the narrowing that drops
  rungs (`from_config_with`, `without_models`, `in_own_tool_loop`).
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::GraphTaskDispatcher::record_attempt_ladder`: the try's ladder
  record (`reason`).
- `crates/roko-learn/src/telemetry/records.rs::LadderReason`: the recorded reasons.

## Current state

`resolve`'s doc says "When that rung cannot run, the task starts on the next runnable rung above it, else the nearest
one below." The nearest one below is the highest runnable rung, from which `can_climb` is false (ladder.rs).

## Plan

1. In `resolve`, when no runnable rung is at or above the requested start, start on the runnable rung below the
   highest one (when at least two can run), so the task can still climb once. With one runnable rung, start there.
   A reachable start changes nothing. `ResolvedLadder` remembers the requested start and says whether it was
   unreachable.
2. `LadderReason::StartUnreachable` (`"start_unreachable"`): the reason a try records when it runs on the fallback
   start (step 0, not pinned, not the self-model's).
3. One log line wherever the run's ladder is set or narrowed, naming the tiers whose start can't run, where they
   start instead, and the rung they can climb to. For example: architectural tasks start on top, which can't run
   here; they start on mid and can climb to strong.

## Done when

- A task whose start rung and every rung above it can't run starts one runnable rung below the highest. Its first
  try records `reason = "start_unreachable"`, and it climbs once after two failures.
- A reachable start is unchanged and records `start`.
- Checked by the `[[verify]]` tests.

## Notes

- Asked by the lead on 2026-10-20 (round v7, roko-ladder's analysis of run 82b7e069) as a p2 to do at once. The lead
  is making the matching config change in Undertow for phase 2's resume.
- Alternatives the lead may still choose: an API-backed Sonnet rung so top is reachable in chat runs (needs an
  Anthropic key); or reorder the workspace's ladder from rung stats (gap-3217a0).
