+++
id = "bug-07bc75"
kind = "bug"
title = "The provider bridge still trains the cascade router on pre-gate provider success for every Graph dispatch"
status = "open"
triage = "verified"
severity = "p1"
goal = "cybernetic"
size = "S"
subsystem = ["learn", "dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (22:20, wk-settle's report on gap-8f6206)"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::record_agent_dispatch_feedback", "crates/roko-learn/src/model_call_feedback.rs::observe_model_call_on_router"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = ["gap-8f6206"], blocks = [], related = ["gap-8f6206", "bug-c34782"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_dispatch_router_learns_only_from_settled_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_dispatch_router_learns_only_from_settled_verdicts"
+++

## Problem

After gap-8f6206, every learner behind `FeedbackEvent` reads the settled verdict's `learning_label`. But the provider bridge, `dispatch_v2::record_agent_dispatch_feedback` → `observe_model_call`, still updates `cascade-router.json` from the provider call's pre-gate success on every Graph dispatch, so the router is trained twice, once on the wrong signal.

## Why it matters

The cybernetic claim is that loops learn from verified outcomes, and this path contradicts it. Epic spec-6ac537.

## Where

`crates/roko-cli/src/dispatch_v2.rs` and the model-call feedback path in roko-learn.

## Current state

Graph dispatches update the router from provider success.

## Plan

1. On the Graph path, skip router learning in the provider bridge. The settled verdict already feeds the router through `RoutingObservationSink`.
2. Keep the bridge's cost and efficiency recording.
3. Other paths (chat, serve) keep their current learning until they settle verdicts.
4. Add the test the verify names.

## Done when

- [ ] A Graph dispatch updates the router only from its settled verdict.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-07bc75` at `57ea5133f`; cargo verification deferred to the batch check. The branch is
  `work/gap-8f6206` (e23fe1990) with `df18d9ece` merged in (`fe0f5e6e5`), because gap-8f6206 was not merged yet
  and MAIN had rewritten `model_call_feedback.rs`. In the worktree's own target clone, `cargo check -p roko-cli --lib
  --tests` and the targeted tests passed: `graph_dispatch_router_learns_only_from_settled_verdicts`, `dispatch_v2::`,
  `graph_task_dispatch::feedback::` and `::attempt::`.
- **Decision (2026-09-29):** every production `AgentDispatchRequest` comes from Graph dispatch: the batch path, the
  streaming path and the cheap helper agent. So the bridge (`record_agent_dispatch_feedback`) drops the router
  observation entirely, through `ModelCallFeedbackRecorder::without_cascade_router`, rather than taking a
  per-request flag that would also touch `streaming.rs` and `routing_context.rs`. It still writes the `model_call`
  efficiency row and the provider's health. Chat, serve, `dispatch_via_model_call_service`, ACP and the vision
  loop keep their own recorders. After a Graph run, `cascade-router.json` is the run's in-memory router, saved at
  run end and trained only by `RoutingObservationSink`.
