+++
id = "gap-25d9cc"
kind = "gap"
title = "MCP confirm_pending omits the confirm rung's timeout, so hosts guess when an unanswered sign-off is skipped"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/mcp"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::confirm_holds", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::confirm_rung"]
links = { depends_on = [], blocks = [], related = ["gap-7c6663"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn confirm_pending_reports_the_rungs_deadline' crates/roko-serve/ && cargo test -p roko-serve confirm_pending_reports_the_rungs_deadline"
+++

## Problem

A `confirm` rung writes its review hold with `timeout_secs`, taken from the rung (`confirm_rung` in
`attempt_workspace.rs`). But `/mcp` `confirm_pending` (`confirm_holds`) returns only `plan_id`, `task_id`,
`title`, `attempt_key`, `question`, `summary` and `held_at`.

A host therefore cannot tell how long the person has before the rung gives up and the task ends unverified. In the
Hermes dogfood (2026-10-13, run `1a778f62-e889-4344-93ee-b4d21d4b92ae`, assistant pack with
`timeout_secs = 1800`), the plugin assumed roko's default of 120 s and told the person "answer by
2026-10-13T14:30:53+00:00 or the check is skipped", which is two minutes for a rung that waits thirty.

## Why it matters

The person sees false urgency. With a rung shorter than the host's guess, they would miss the deadline and get an
unverified task without warning (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/mcp.rs::confirm_holds`: builds the items from the hold files.
- `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::confirm_rung`: writes `timeout_secs` and
  `held_at`.

## Current state

The data is in the hold file. Only the projection drops it.

## Plan

Include `timeout_secs` and `deadline_at` (`held_at` plus `timeout_secs`, RFC 3339) in each `confirm_pending` item,
and document them where the `/mcp` tools are documented. Hosts should then show the real deadline.

## Done when

`[[verify]]` passes: a hold with `timeout_secs = 1800` is listed with that timeout and a `deadline_at` 30 minutes
after `held_at`.
