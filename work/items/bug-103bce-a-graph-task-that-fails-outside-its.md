+++
id = "bug-103bce"
kind = "bug"
title = "A Graph task that fails outside its gates never says why: its error reaches only a warn log and a hash, so a chat host reports 'engine failed'"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko docs walkthrough 2026-10-13, fresh roko init workspace with only the Claude CLI (roko 5de43f9ab)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn failed_tasks_say_why_in_the_runs_events' crates/roko-cli/ && cargo test -p roko-cli failed_tasks_say_why_in_the_runs_events"
+++

## Problem

When a Graph task fails outside its gates, the reason never leaves the process. Observed on 2026-10-13 (roko
`5de43f9ab`) in a fresh `roko init` workspace whose only provider is the Claude CLI: every run started from Hermes
(MCP `run_prompt`) failed about 4 ms after dispatch. The Graph engine had the reason:

```
no usable provider for this task: `claude-sonnet-4-6` on `claude_cli`: provider `claude_cli` (claude_cli) runs its
own tools, so it cannot honour the task's `stage` policy for tool calls that act on the outside world: only roko's
own tool loop can stage or refuse them. … (claude-sonnet-4-6@anthropic: ANTHROPIC_API_KEY is not set). …
```

But it went only to a `tracing::warn!`, which `roko serve` does not print by default, and to the attempt record as
a sha256. The run's events had `task_completed {outcome: failed}` and `error: "Graph Engine: 1 plan(s), 0
succeeded, 1 failed"`, so Hermes told the person "roko's engine itself failed", and `roko diagnose` said "no task
failure was recorded".

## Why it matters

The person, or the chat host acting for them, cannot fix what the run does not say. roko's own message here names
the fix: put the API key in `~/.roko/.env`. Every first run from a chat host in a workspace that has only CLI agents
fails like this, because a chat host's run holds outbound actions (`stage`, decisions 9107/9131), which only roko's
own tool loop can do.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: logs each failed node's `error` with
  `tracing::warn!`, and only when not `--quiet`/`--json`.
- `crates/roko-graph/src/engine.rs`: keeps it in `NodeResult.error`.

## Current state

Unfixed at `5de43f9ab`.

## Plan

1. When a plan ends, publish one `error` event per failed task that has an error (not a task blocked by
   another): `plan '<plan>': task '<task>' failed: <error>`, clipped. The run index, `/api/runs/{id}/events`, the
   per-run stream and hosts' cards then carry it (hermes-plugin-roko already shows a run's `error` events).
2. Unit test the messages from a `GraphOutput`.

## Done when

`failed_tasks_say_why_in_the_runs_events` passes (the `[[verify]]` command).

## Notes

Found by following the plugin's getting-started steps in a fresh workspace (hermes-plugin-roko docs walkthrough).
Related: an up-front refusal of such runs, before anything starts, is a separate question.
