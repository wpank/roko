+++
id = "reg-e2038b"
kind = "regression"
title = "A workspace's own models lose to the default ladder's built-in rungs, so its tasks go to gpt-5.4-mini on a key from ~/.roko/.env"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/routing"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "roko-delivery, 2026-10-15 (team lead): fake claude_cli fixtures dispatched to real gpt-5.4-mini with Will's HOME"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs::builtin_rung_model"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn explicit_models_win_over_builtin_rungs' crates/roko-cli/ && cargo test -p roko-cli --lib explicit_models_win_over_builtin_rungs"
+++

## Problem

Since bug-4e336b (merged 4aa4224bb), a ladder rung may name a built-in model (`gpt-5.4-mini`,
`claude-sonnet-4-6`). With no `[models.*]` entry, it binds once the built-in's key is set. A workspace with no
`[routing.ladder]` of its own uses roko's default ladder, whose `strong` rung names `gpt-5.4-mini`. So in a
workspace that configures its own model, its tasks went to `gpt-5.4-mini` on OpenAI whenever `OPENAI_API_KEY`
was set, for example from `~/.roko/.env`. That workspace might have, say, a `claude_cli` provider and a
`[models.*]` entry on it.

The ladder start rung for a `focused` task is `cheap`. That rung can't run, so the task starts on the next one
up, `strong`.

Found 2026-10-15 by roko-delivery: roko-cli integration tests that spawn `roko` with a fixture's fake
`claude_cli` provider, run with Will's HOME, logged `creating agent via provider adapter slug=gpt-5.4-mini
provider=openai_compat base_url=https://api.openai.com/v1`. One of them is
`worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing`. The result was real spend, and tests that
pass or fail at random on that machine.

## Why it matters

A person's explicit choice of model and provider must win. Here roko silently moved their tasks to another
vendor and spent on a key they keep for other uses.

## Where

- `crates/roko-cli/src/dispatch/model_routing.rs::builtin_rung_model`, reached from `rung_dispatch_model` for a
  rung no `[models.*]` entry has.
- `roko_core::config::routing::LadderConfig::default`: the default ladder (D11's rungs).

## Plan

1. A rung binds a built-in model only when the workspace makes no model choice of its own that the rung could
   override. That means its ladder is its own (its rungs differ from roko's default ladder, or it has role
   ladders), or it has no `[models.*]` entry and no `[agent] default_model` other than roko's default. A model
   pin (`--model`) and a task's `model_hint` already win over the ladder.
2. The skip reason says so, and `roko doctor`'s `routing_ladder` line shows it.
3. Test: a workspace with only a fake `claude_cli` model never routes a task to a built-in API model, even with
   every provider available (OPENAI_API_KEY set).

## Done when

`explicit_models_win_over_builtin_rungs` passes (the `[[verify]]` command).
