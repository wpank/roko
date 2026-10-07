+++
id = "bug-b087ea"
kind = "bug"
title = "Streaming dispatch still passes DispatchContext attempt: 0, so the self-model treats every retry as a first attempt"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-17b follow-up reports 2026-10-04 (bug-78e5ce, work/backlog-batch-17b)"
discovered_from = "bug-78e5ce (open; own Progress note names this residual of facet 2)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs::dispatch_streaming", "crates/roko-cli/src/graph_task_dispatch/self_model.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_retry_self_model_features_show_prior_failure' crates/roko-cli/ && cargo test -p roko-cli streaming_retry_self_model_features_show_prior_failure"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:35Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:05Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (streaming_retry_self_model_features_show_prior_failure). Streaming passes the real attempt number, so a streamed retry's self-model features show the prior failure."
+++

## Problem

The streaming dispatch path passes `DispatchContext { attempt: 0, .. }` unconditionally
(`crates/roko-cli/src/graph_task_dispatch/streaming.rs:184`), so the self-model's features treat
every streaming retry as a first attempt. The non-streaming (batch) path passes the real count
(`crates/roko-cli/src/graph_task_dispatch.rs:1226`: `attempt: attempt_number`). The self-model
reads this field directly: `has_prior_failure: inputs.attempt > 0`
(`crates/roko-cli/src/graph_task_dispatch/self_model.rs:1123`) — with `attempt` hardcoded 0,
`has_prior_failure` is always `false` for a streamed attempt no matter how many times it has
actually retried.

This field's staleness was already known and explicitly accepted twice before, each time on the
grounds that nothing read it yet: `bug-cae1e1` (done, 2026-09-30) fixed streaming's retry
marking and `preferred_provider` handling but left `DispatchContext.attempt` at 0, noting "nothing
in routing reads it, and gap-460230 adds its ladder step on that line." `gap-460230` (done,
2026-09-30) added ladder/escalation logic with its own test covering the streaming path
(`two_failed_attempts_escalate_one_rung_when_streaming`) — but that fix tracks retries through
its own mechanism, not by making `DispatchContext.attempt` itself correct; the field is still 0
today. `bug-78e5ce`'s facet 2 (done on `work/backlog-batch-17b`) just added the self-model's
`forecast_attempt` call to the streaming path for the first time — which is what newly makes
this field matter for a third consumer, and its own Progress note flags it: "The streaming
`DispatchContext` still passes `attempt: 0` ..., so a streaming retry's features say first
attempt."

## Why it matters

Goal: cybernetic, M3 self-model (S04), same goal as `bug-78e5ce`. `has_prior_failure` is a real
input to the self-model's forecast — now that streaming dispatch actually forecasts (per
`bug-78e5ce`'s own fix), every streamed retry's forecast is computed as if it were a first
attempt, systematically misinforming the self-model's prediction exactly on the chains where a
prior failure most matters.

## Where

- `crates/roko-cli/src/graph_task_dispatch/streaming.rs:171-184` (`dispatch_streaming`'s
  `DispatchContext` construction; the fix site).
- `crates/roko-cli/src/graph_task_dispatch.rs:1226` (the batch path's correct construction; the
  pattern to match).
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs:1123` (`has_prior_failure`, the
  consumer that makes this matter now).

## Current state

Unfixed, confirmed on `work/backlog-batch-17b` (tip `00a28ea0c`): `attempt: 0` is still
hardcoded in the streaming path's `DispatchContext` literal.

## Plan

1. Thread the real attempt number into `dispatch_streaming`'s `DispatchContext` construction,
   the same way the batch path does.
2. Regression test: a streamed second attempt's self-model forecast has
   `has_prior_failure: true`, not `false`.

## Done when

- A streaming retry's self-model features reflect its real attempt count.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-17b follow-up, bug-78e5ce, work/backlog-batch-17b not yet merged): confirmed
  directly. `bug-78e5ce` is still open and its Progress note already names this exact residual;
  filed separately since that item's single named `[[verify]]` command covers only its original
  facet 2 (the forecast call itself), not this newly-surfaced consequence of it. A matching note
  has been added to `bug-78e5ce`.

## Progress

- bug-b087ea: implemented at e4e039248 on `work/bug-7dff88`; cargo verification deferred to the batch gate.
  `dispatch_streaming` sets `DispatchContext.attempt` to `attempt_number` (the attempts before this one, from
  `next_retry_attempt`, as on the batch path). Ladder routing does not read it; it moves the self-model's
  features and the prompt's retry gate (streaming sends no gate feedback). Test:
  `streaming_retry_self_model_features_show_prior_failure`: two streamed attempts are forecast as (1, no prior
  failure) and (2, prior failure).
