+++
id = "bug-8b0d0a"
kind = "bug"
title = "One gateway call can be observed up to three times on serve's shared cascade router"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-serve", "roko-gateway", "roko-agent/model_call_service"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report on bug-605a8a, branch work/bug-605a8a)"
anchors = ["crates/roko-gateway/src/gateway.rs", "crates/roko-learn/src/model_call_feedback.rs", "crates/roko-agent/src/model_call_service.rs", "crates/roko-serve/src/service_factory.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = ["bug-605a8a"], blocks = [], related = ["bug-605a8a", "bug-84de98", "bug-7a2630", "bug-c34782"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_gateway_call_is_observed_once_on_the_shared_router' crates/roko-serve/src/ && cargo test -p roko-serve --lib a_gateway_call_is_observed_once_on_the_shared_router"
+++

## Problem

On `work/bug-605a8a`, d32a4609c gives serve one journaled cascade router, shared by the gateway, feedback and dispatch. Three paths can now record the same gateway call on it:

- the gateway itself (`record_observation`, `crates/roko-gateway/src/gateway.rs:523` and :663 on the branch);
- the FeedbackService, through the inference observer that `service_factory.rs` attaches (:334, :516), which calls `observe_model_call` (`model_call_feedback.rs:233` onward);
- the force-backend override, `record_override_outcome` (`model_call_service.rs:869`).

A single call can therefore update the router up to three times.

## Why it matters

Cybernetic core (epic spec-6ac537): routing statistics count some calls three times, and others once, depending on the path. That skews the arms and the exploration bonus.

## Where

The three observation sites above, and the shared router in serve's state (`state.rs`, `lib.rs` on the branch).

## Current state

The shared router exists only on the branch (not merged at ad391f99a). Before it, the paths wrote to separate routers.

## Plan

1. Pick one owner for each call's observation (probably the FeedbackService, which sees the settled outcome), and have the other paths skip the shared router, or pass a call id that the router deduplicates on.
2. Add `a_gateway_call_is_observed_once_on_the_shared_router`: one gateway call, including a forced backend, changes the router's counts by exactly one observation.

## Done when

- [ ] Each model call is observed once on the shared router.
- [ ] The `[[verify]]` command passes.

## Notes

- Land with or after bug-605a8a.
- Implemented on `work/bug-84de98` at `1df06a526`; cargo verification deferred to the batch check. The model-call feedback is the one observer; the gateway only routes, and serve attaches no override recorder.
