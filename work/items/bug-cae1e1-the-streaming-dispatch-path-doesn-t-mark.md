+++
id = "bug-cae1e1"
kind = "bug"
title = "The streaming dispatch path doesn't mark retries in the routing context and ignores preferred_provider"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/streaming"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "7490cb94b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-taskdef's report, checked on work/gap-0f3980 at b27c02717)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-0f3980"], blocks = [], related = ["gap-0f3980", "gap-b62e95"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_dispatch_marks_retries_and_honours_preferred_provider' crates/roko-cli/src/ && cargo test -p roko-cli --lib streaming_dispatch_marks_retries_and_honours_preferred_provider"
+++

## Problem

On gap-0f3980's branch, the batch dispatch path honours a task's `preferred_provider` (3 uses in `graph_task_dispatch.rs`) and marks retries in the routing context. The streaming path (`graph_task_dispatch/streaming.rs`) does neither: it never reads `preferred_provider`, and it doesn't mark the attempt as a retry in the routing context (wk-taskdef). So a streamed task routes as a first attempt on the default provider, whatever its TaskDef says.

## Why it matters

Tier ladder and escalation (epic spec-98f76d): retry-aware routing and provider preferences only work on one of the two dispatch paths. gap-b62e95 is the router side of the retry context.

## Where

The routing-context construction in `streaming.rs`, compared with the batch path.

## Plan

1. Build the streaming path's routing context the same way as the batch path, sharing one function, with the retry state and `preferred_provider`.
2. Add `streaming_dispatch_marks_retries_and_honours_preferred_provider`.

## Done when

- [ ] Both paths route a retry, and a task with `preferred_provider`, the same way.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-0f3980's branch.
- Implemented on `work/bug-cae1e1` at `05fc821cc`; cargo verification deferred to the batch check.
- `dispatch_streaming` marks its routing context, and the feedback copy, with `mark_attempt` right after the attempt
  opens (`next_retry_attempt` only reads the retry book). Both paths pick the model to dispatch with
  `GraphTaskDispatcher::dispatch_model_key` (routing_context.rs).
- The streaming `DispatchContext.attempt` stays 0: nothing in routing reads it, and gap-460230 adds its ladder step on
  that line.
- The test checks the provider named by each attempt's terminal receipt, not a log the fake providers write: after the
  failed gate, the helper calls (bug-62e3f4) run on the cheap helper model (`select_cheap_model_key`), which here is the
  other provider. Helpers are their own cost line, so they don't follow the task's `preferred_provider`.
