+++
id = "gap-f439c1"
kind = "gap"
title = "A held outbound effect and its decision publish no event, so a run's stream cannot show what waits on the person or when it was decided"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
source = "roko-loops worker for the hermes-plugin-roko live UI, 2026-10-14 (roko main 216f88c84)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched", "crates/roko-serve/src/routes/effects.rs::decide_effect", "crates/roko-agent/src/dispatcher/mod.rs::stage_effect"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn staged_effects_are_published' crates/roko-cli/ && cargo test -p roko-cli staged_effects_are_published"

[[verify]]
command = "grep -rqw 'fn a_decided_effect_is_published' crates/roko-serve/ && cargo test -p roko-serve a_decided_effect_is_published"
+++

## Problem

When a run's agent proposes an outbound call (a send, a post, a payment) and roko holds it for approval, nothing is
published: the hold is a file under `.roko/state/effect-holds/<run>/`, an audit signal and the agent's tool result
("staged for approval as <id>; it has not run"). Its decision (MCP `effect_decide`, `POST /api/effects/{id}/decide`)
publishes nothing either.

## Why it matters

Held effects are loop L0 handing a decision to the person. A host has to poll `effects_pending` to learn that
something waits, and cannot show when it was decided (goal `visibility`).

## Where

- `crates/roko-agent/src/dispatcher/mod.rs::stage_effect` writes the hold (`roko_agent::safety::effects::EffectHold`).
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched`: the attempt's watch loop, which ticks during the
  provider call.
- `crates/roko-serve/src/routes/effects.rs::decide_effect`: every decision path of `roko serve`.

## Current state

No event exists for either step.

## Plan

1. Add `DashboardEvent::EffectStaged {effect_id, run_id, plan_id, task_id, attempt, tool, server, proposed_at}` and
   `DashboardEvent::EffectDecided {effect_id, run_id, plan_id, task_id, tool, server, outcome, decided_by}`.
2. While an attempt's provider call runs, publish `effect_staged` for each new hold of the attempt (its run's hold
   directory, read on the watch loop's tick), and once more after the call for holds the last tick missed.
3. After `roko serve` decides an effect, publish `effect_decided` on its hub.

## Done when

`[[verify]]` passes: a hold an attempt writes is published as `effect_staged`, and a decision through `roko serve` as
`effect_decided`.
