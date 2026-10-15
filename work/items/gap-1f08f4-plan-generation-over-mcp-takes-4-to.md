+++
id = "gap-1f08f4"
kind = "gap"
title = "Plan generation over /mcp takes 4 to 9 minutes for a five-task plan, reports a timed-out attempt as a crash, and reports no cost"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::generate_plan", "crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/agent_exec.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_generation_status_reports_its_cost' crates/roko-serve/ && cargo test -p roko-serve plan_generation_status_reports_its_cost"
+++

## Problem

Plan generation over `/mcp` `plan_generate` was slow and its cost was invisible in the Hermes dogfood
(2026-10-13; a five-task plan for a small Python project):

- **Op `0af7526b`.** The strategist (claude-sonnet-4-6 via `claude_cli`) ran its first attempt for 301.6 s and
  failed with 25 bytes of output. serve's log reports it as "plan generation agent Agent crashed for an unknown
  reason; inspect the full stderr output (attempt 1/3)", although it was the five-minute timeout. The retry took
  252 s and wrote 19,999 output tokens: `tasks.toml` is 13 KB for 5 tasks. In all, about 555 s.
- **Op `92204a56`.** The same work took 232 s and wrote 17,412 output tokens.
- **Cost.** An episode-distiller call on sonnet ($0.07 to $0.12) follows each generation. The two plans cost $1.33,
  as recorded in `.roko/learn/efficiency.jsonl`.
- **What the host sees.** The op's status carries no cost or tokens, so the Hermes plugin shows planning as "—"
  and leaves it out of the day's spend and limit. There is no progress beyond "running" during the 4 to 9 minute
  wait, and the op is forgotten (404) about a minute after it ends.
- **The plan itself.** `plan.md`'s body is just "Generated plan.". The plan id comes from the first words of the
  host's rewritten prompt: `project-tally-a-small-python-cli-expense-ledger`.

## Why it matters

The person waits up to nine minutes with no sign of progress, roko's spend is understated to hosts that enforce
limits, and the plan has no readable overview for the person to review (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/mcp.rs::generate_plan`: the op and its status.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: the strategist dispatch, retry and `plan.md` writing.
- `crates/roko-cli/src/agent_exec.rs`: the "crashed for an unknown reason" classification.

## Plan

1. Report the op's cost, tokens and stage (strategist, distiller, writing) in its status, and keep finished ops
   long enough to be read, or persist them.
2. Classify a timed-out strategist attempt as a timeout.
3. Find out why a five-task plan needs about 18k output tokens, and tighten the prompt or the format.
4. Write a real `plan.md` summary: the goal, the waves and the checks.
5. Derive the slug from the feature, not from the prompt's first words.

## Done when

`[[verify]]` passes: a finished plan generation's status reports its cost.

## Notes

- 2026-10-15 (bug-59ed90): a planner call that keeps failing, or fails auth, now ends its error with "The planner
  said: …" (the provider's own message, first 500 characters), where it used to say only "Agent crashed for an
  unknown reason; inspect the full stderr output" with nothing to inspect. A timed-out attempt is still classed as a
  crash (Plan step 2 remains), and the op's status still carries no cost.
