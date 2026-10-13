+++
id = "bug-369c21"
kind = "bug"
title = "agent_spawned and agent_completed events report attempt 0 for every retry of a Graph task"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher", "crates/roko-cli/src/graph_task_dispatch/failover.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn retried_attempts_publish_their_attempt_number' crates/roko-cli/ && cargo test -p roko-cli retried_attempts_publish_their_attempt_number"
+++

## Problem

When a Graph task is retried, every attempt's `agent_spawned` and `agent_completed` events carry `attempt: 0` and
the same `agent_id` (`<plan>/<task>`). In run `e7fef800-c269-44c2-b4db-7548f4207898` (Hermes dogfood,
2026-10-13), T1 ran six attempts, and all twelve rows say attempt 0.

`GraphTaskDispatcher::dispatch` calls `tui.agent_spawned(&pre_dispatch_agent_id, …, 0, …)` with a literal `0`,
although `attempt_number` is in scope a few lines earlier.

## Why it matters

Hosts cannot number attempts ("T1, attempt 3 of 6"), cannot tell one attempt's output from the next, and
undercount retries in timelines (goal `visibility`).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher`: the pre-dispatch `agent_spawned` call in
  `dispatch`.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs`: the failover `agent_spawned` call.
- The publisher of `AgentCompleted`.

## Plan

Pass the attempt number (numbered as the attempt key numbers it) to `agent_spawned` and `agent_completed`. Give
each attempt an agent id of its own, or keep the id and rely on the attempt field, but say which in
`DashboardEvent`'s docs.

## Done when

`[[verify]]` passes: a task that runs twice publishes two `agent_spawned` rows with different attempt numbers.
