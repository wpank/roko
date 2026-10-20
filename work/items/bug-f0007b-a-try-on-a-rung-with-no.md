+++
id = "bug-f0007b"
kind = "bug"
title = "A try on a rung with no max_usd has no time or spend bound of its own, so one slow mid try (glm-4.7, 13 min of streamed reasoning) spent the run's whole cap and the task never climbed again"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-17
updated = 2026-10-17
source = "round-v6-dogfood-desktop#L1-notch"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L1, 2026-10-17)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/try_allowance.rs::RungAllowance", "crates/roko-core/src/config/routing.rs::LadderConfig", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::StallWatch"]
links = { depends_on = [], blocks = [], related = ["gap-2c9412"], supersedes = [], duplicate_of = "" }
+++

## Problem

One try on a middle rung can spend a run's whole cap, so the task never climbs again. A try is bounded by turns and,
only when `[routing.ladder] max_usd` names its rung, by spend. It has no time bound. A slow model that keeps
streaming reasoning is never silent, so the stall watchdog never fires, and a rung with no `max_usd` is bounded in
money only by the plan's budget.

Seen in round v6's showcase take L1 (2026-10-17, roko e659d2882), run `573df19c-3a49-4d0d-906b-86951c0ea0e7` on
pocket-synth. It was a follow-up started from the roko page's composer ("run it": the notch output), with a $0.25 cap,
a focused task, and the showcase ladder (cheap gpt-oss-120b, mid glm-4.7, strong gpt-5.4-mini; top is not for chat
runs).

| Try | Rung | Duration | Spend | How it ended |
|---|---|---|---|---|
| 1 | cheap | 21.1 s | $0.0456 | hit its 15-turn bound, then its check failed (`0.2217 not less than 0.2`) |
| 2 | cheap | 25.5 s | $0.0544 | its check failed (`0.399 not greater than 0.4`); climbed |
| 3 | mid | 776.5 s (12 min 56 s) | $0.1331 | `budget_exhausted`, blame harness, with $0.0012 of the cap left; its check never ran |

Try 3 made about 17 tool calls (34 `tool_start` and 17 `tool_result` stream events) against its 30-turn bound. Its
usage was 64,684 input, 28,162 output, 14,794 reasoning and 293,504 cache-read tokens. The stream was mostly
reasoning deltas: 14,638 of 14,980 `agent_output` events.

## Why it matters

The pitch is "a failing check climbs the ladder". Here the task failed twice on cheap, climbed, and then one mid try
used the rest of the cap, so it never reached strong. The person waited 13 minutes and got nothing: no check ran on
mid, and no work was kept. The run reported it honestly ("stopped at cap"), but the ladder's promise didn't happen.

## Where

- `crates/roko-cli/src/graph_task_dispatch/try_allowance.rs` (`RungAllowance`): turns, plus USD when `max_usd` names
  the rung. There's no time bound.
- `crates/roko-core/src/config/routing.rs` (`LadderConfig::try_turns`, `try_usd`): with `max_usd` unset, a rung has no
  spend allowance of its own.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs`: stalls are counted only while the agent is silent
  (`task_stall_secs = 300`), and streamed reasoning counts as progress.
- The task had `timeout_secs = 0` (`.roko/runs/573df19c…/tasks.toml`), so no task wall-time applied either.

## Current state

The bounds that applied to try 3:
- turns: 30 (focused tier 60, one climb left);
- spend: none of its own (`[routing.ladder] max_usd` unset);
- time: none.

Only the plan's $0.25 budget stopped it.

## Plan

roko-caps chooses the fix. Options:
- (a) a per-rung time bound, for example `[routing.ladder] max_secs = { cheap = 60, mid = 180 }`, ending a try as
  turns and spend do, judged by its checks (gap-2c9412);
- (b) when `max_usd` leaves a rung out, a default per-try share of what is left of the budget, so one try can't take
  all of it while rungs remain above it;
- or both.

## Done when

A try that reaches its rung's time or spend bound is judged by its checks and, failing them, climbs while budget is
left for the next rung. A replay of this run's shape under a $0.25 cap reaches strong instead of stopping on mid.

## Notes

- Evidence: `tmp/hermes-roko-v6/findings/dogfood-desktop.md`.
  - The take is `tmp/hermes-roko-v6/media/pipeline/work/takes/l1-g2-page-2134`; the climb video is the guide's climb slot.
  - The run's records are in `/Users/Shared/pocket-synth/.roko/runs/573df19c…/attempts.jsonl` and `events-by-run/`.
- The showcase's next climb take sets `[routing.ladder] max_usd = { cheap = 0.04, mid = 0.10, strong = 0.25 }` by
  hand. That is the workaround this item would make unnecessary.
