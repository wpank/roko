+++
id = "gap-11d870"
kind = "gap"
title = "Plan generation calls a timed-out planner call a crash, says only running while it works, and writes a plan.md with no overview"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-serve/plans"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-1f08f4's leftovers after bug-a0432b (Hermes dogfood 2026-10-13, ops 0af7526b and 92204a56)"
discovered_from = "gap-1f08f4"
anchors = ["crates/roko-cli/src/agent_exec.rs::classify_agent_crash", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::slugify"]
links = { depends_on = [], blocks = [], related = ["gap-1f08f4", "bug-a0432b", "bug-a0f01e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_timed_out_planner_call_is_reported_as_a_timeout' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_timed_out_planner_call_is_reported_as_a_timeout"

[[verify]]
command = "grep -rqw 'fn plan_generation_status_reports_its_stage' crates/roko-serve/src && cargo test -p roko-serve --lib -- plan_generation_status_reports_its_stage"

[[verify]]
command = "grep -rqw 'fn a_plan_without_a_plan_md_block_gets_an_overview' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_plan_without_a_plan_md_block_gets_an_overview"
+++

## Problem

What gap-1f08f4 found in the Hermes dogfood of 2026-10-13 (ops `0af7526b` and `92204a56`, a five-task plan for a
small Python project) and bug-a0432b left alone. bug-a0432b made a finished plan generation's status report its
cost (`result.cost_usd`), which closed gap-1f08f4. The rest is still true:

- **A timed-out attempt is called a crash.** A CLI planner call is stopped after 300 s
  (`agent_exec.rs`, `timeout_ms: Some(300_000)`). The output it hands back does not say "timeout", so
  `classify_agent_crash` returns `Unknown`, and serve's log says "Agent crashed for an unknown reason; inspect the
  full stderr output (attempt 1/3)". Op `0af7526b`'s first attempt was exactly that timeout (301.6 s, 25 bytes of
  output).
- **No stage while it runs.** For the 4 to 9 minutes a plan takes, `run_status` says only `running`: not which
  attempt, whether it is regenerating or distilling, the time so far or the spend so far. A failed op's error
  names the cost only when the plan was written but cannot run ("Writing the plan cost $X."); a planner that
  fails outright reports none. And `cost_usd` counts the planner's calls only: the episode distillation after
  each generation runs in the background and is recorded on its own (`roko_neuro::DistillationSpend`).
- **No overview.** When the planner's reply has no `plan.md` block, `generate_plan` writes a `plan.md` that is
  the slug and "Generated plan.", so the person reviewing the plan has nothing readable.
- **The slug is the prompt's first words** (`project-tally-a-small-python-cli-expense-ledger`), not the feature.
- **Volume.** The five-task plan took about 18k output tokens (a 13 KB `tasks.toml`). Nobody has looked at why.

That a finished op is forgotten (404) about a minute after it ends is part (1) of bug-a0f01e's open work, not this
item's.

## Why it matters

The person waits minutes with no sign of progress; a timeout reads as an unexplained crash, so nobody raises the
limit or picks a faster planner; and the plan has no overview to review before it runs (goal `hermes`).

## Where

- `crates/roko-cli/src/agent_exec.rs::classify_agent_crash` and the 300 s limit in `run_agent_capture_impl`
  (`SpawnAgentSpec.timeout_ms`): the timeout and its class.
- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation`: the op and the status it records.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the minimal `plan.md` and the slug
  (`slugify` of the prompt).

Entry point: `/mcp` `plan_generate`, or `POST /api/plans/generate`, then `run_status` on the op.

## Current state

- bug-59ed90 (2026-10-15): a planner call that keeps failing ends its error with "The planner said: …".
- bug-7a08a8 (2026-10-16): a plan that cannot run fails the op with its validation errors.
- bug-a0432b (2026-10-16): a chat host's plan is written by a model roko's own tool loop runs, and the finished op's
  result carries `cost_usd` (an op whose plan cannot run ends its error with "Writing the plan cost $X."). It does
  not change the 300 s limit (`SpawnAgentSpec.timeout_ms`, set for every planner call) or how a stopped call is
  classed.

## Plan

1. Have the timed-out call say so (a marker in the output, or a typed result) and add a `Timeout` crash class
   whose hint names the limit and how to change it.
2. Report the stage in the op's status while it runs: attempt k of n, regenerating, distilling, with the elapsed
   seconds and the spend so far (a progress channel from `generate_plan` to the op), and keep the spend in a
   failed op's status.
3. When the reply has no `plan.md` block, write one from `tasks.toml`: the goal (the request's first lines), the
   tasks with their waves and checks.
4. Derive the slug from the plan's title, or a slug the planner proposes, rather than the prompt's first words.
5. Measure the output on a few small requests and tighten the prompt or the format if most of it is boilerplate.

## Done when

The `[[verify]]` commands pass: a planner call that hits the time limit is classed and reported as a timeout; a
running plan generation's status names its stage; and a plan whose reply has no `plan.md` block gets one that lists
its tasks.

## Notes

- Keep `result.slug`, `result.task_count` and `result.cost_usd` in the finished op as they are: the Hermes plugin
  reads them.
