+++
id = "gap-3e1428"
kind = "gap"
title = "A chat host's run checks none of the request's explicit constraints against the change: the LLM judge is off by default, and its helper model failed every call in the 2026-10-13 dogfood"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "find-2ba86d (hermes-plugin-roko dogfood 2026-10-13, run 7557c030), split out 2026-10-27"
anchors = ["crates/roko-cli/src/graph_task_dispatch/judge_step.rs::judge_attempt", "crates/roko-core/src/config/gates.rs"]
links = { depends_on = [], blocks = [], related = ["find-2ba86d"], supersedes = [], duplicate_of = "" }
+++

## Problem

A chat host's run (Hermes through `roko serve`) never checks the request's explicit constraints against the change.
In the 2026-10-13 Hermes dogfood (find-2ba86d, run `7557c030-2046-473f-b42b-155f1a8ad4e0`), the request said
"exactly the Gregorian rule, no special cases for particular years, no global or mutable state", and the agent kept
a call counter as an attribute of `is_leap_year` and inverted the answer for 1900 on its third call. The workspace
test rung passed, and the verdict was `succeeded`.

The only check that reads a request's words against a diff is the opt-in LLM judge (`[gates] llm_judge`,
`crates/roko-cli/src/graph_task_dispatch/judge_step.rs`), and it is off by default. On that machine its helper
model (kimi-k2.5 through moonshot) failed every call in the same session (`success: false`, `error_class: unknown`),
so turning it on would not have judged either attempt.

## Why it matters

Goal `truth`. find-2ba86d's static gaming screen (`roko_gate::attempt_diff::gaming_findings`) now makes such a pass
`unverified`, but only for the shapes it knows (call-count state, special cases the tests contradict, computed
imports) and only in Python. A request's explicit constraint ("no global state", "must not import X", "implement the
algorithm yourself") is the most direct statement of what the change must not do, and nothing reads it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/judge_step.rs::judge_attempt`: the judge step; returns early when
  `[gates] llm_judge` is off or no helper model can run (`cheap_agent()`).
- `crates/roko-core/src/config/gates.rs`: `llm_judge`, `llm_judge_min_score`, `llm_judge_blocking`.
- `judge_task_description` in the same file: what the judge is told about the task.
- The chat-host entry: `roko serve`'s run creation (`crates/roko-serve/src/routes/`), which knows a run came from a
  host.

## Current state

The judge is advisory unless `llm_judge_blocking = true`. It is told the task and the diff, not the request's
explicit constraints as a list to check one by one. A helper model that fails every call leaves the attempt unjudged
with a log line only.

## Plan

1. For runs a chat host starts, turn the judge on, or a constraint-only variant of it: the request's explicit
   constraints (sentences with "must not", "no …", "without …", "only …") checked one by one against the diff.
2. Use a helper model that answers: fall back to the run's own model when the configured helper fails, and record
   which model judged.
3. A constraint the judge finds broken settles the attempt `unverified` with its reason, as the gaming screen does
   (`red_flags::settle_suspect`), rather than failing it on a model's word alone.

## Done when

A chat-host run of the `is_leap_year` request above, with a scripted helper that reports the broken constraint,
settles `unverified` with the constraint named in the verdict; and a helper that fails falls back to the run's model.

## Notes

Split out of find-2ba86d on 2026-10-27: the coordinator kept the LLM judge out of that item's scope.
