+++
id = "gap-143b5c"
kind = "gap"
title = "A generated plan's slug comes from the request's first words, a small plan runs to thousands of output tokens, and the distiller's cost is not in the operation"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/plans", "roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-11d870's steps 4 and 5 and leftovers (roko-planner, 2026-10-16)"
discovered_from = "gap-11d870"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/agent_exec.rs::persist_capture_episode_distilled_by"]
links = { depends_on = [], blocks = [], related = ["gap-11d870", "gap-1f08f4", "gap-c2efe7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_generated_plan_is_named_after_its_proposed_slug' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_generated_plan_is_named_after_its_proposed_slug"
+++

## Problem

Steps 4 and 5 of gap-11d870's plan, left when gap-11d870 was closed (the stage while running, timeouts and the
plan.md overview landed; see its evidence):

- **The slug is the request's first words.** `roko serve` derives a plan's slug before the planner runs, from the
  first line of the request (`derive_unique_slug` in `routes/plans/authoring.rs::start_plan_generation`), unless the
  host passes a `title` (gap-c2efe7). In the 2026-10-13 Hermes dogfood that gave
  `project-tally-a-small-python-cli-expense-ledger`. `roko plan generate` slugs the prompt the same way. The plan's
  own `[meta] title`, when the planner writes one, plays no part.
- **Output volume.** A five-task plan took about 18k output tokens (a 13 KB `tasks.toml`) in that dogfood, which is
  most of a generation's time and cost. Nobody has measured what the tokens go to (repeated defaults, long verify
  commands, context blocks).
- **Cost not counted.** A generation's `cost_usd` covers the planner's calls. The episode distillation that follows
  each generation runs in the background after the operation ends, and is recorded on its own
  (`roko_neuro::DistillationSpend`), so a host never sees it.
- **A tool-loop planner's timeout.** gap-11d870 classes a call the CLI adapters stopped at roko's limit
  (`"timed out after <N> ms"`) as a timeout. A planner on an API model runs in roko's own tool loop, whose
  request timeout is `tool_loop::LlmError::Timeout` (`"timeout: …"`), not marked that way, so
  `classify_agent_crash` most likely calls it a network error (not checked live).

## Why it matters

A host shows the slug as the plan's name and the person reads it in every list; the output volume is what makes
plan writing take minutes; and an uncounted call understates what a request cost (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation`: the slug, which the 202 response and the
  operation's `kind` carry before the planner runs.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the planner prompt and its output.
- `crates/roko-cli/src/agent_exec.rs::persist_capture_episode_distilled_by`: where the background distillation
  starts.

## Plan

1. Slug: either have the planner propose a short slug in `[meta]` and move the plan to it before the operation
   completes (the result's `slug` is then the one to open; a host must not rely on the 202's), or ask hosts to pass
   `title` and say so in the `plan_generate` tool's description. The first changes a published contract; decide
   with plugin-runs.
2. Volume: generate plans for three sample requests, count the output tokens by section, and drop what the parser
   fills in anyway (`status = "ready"`, `done = 0`, empty lists) from the format the prompt asks for.
3. Cost: either wait for the distillation before the operation ends and add its cost, or report it later as a
   separate figure.

## Done when

`[[verify]]` passes: a generated plan whose planner proposes a slug is named after it, and the result's `slug` is
that name.
