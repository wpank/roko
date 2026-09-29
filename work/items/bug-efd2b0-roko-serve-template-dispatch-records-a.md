+++
id = "bug-efd2b0"
kind = "bug"
title = "roko-serve template dispatch records a cascade-router outcome on every TurnCompleted, on top of its journaled observation"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-serve/dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report)"
anchors = ["crates/roko-serve/src/dispatch.rs::drain_dispatch_learning_events", "crates/roko-serve/src/dispatch.rs::record_cascade_router_outcome_with_layout"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-8b0d0a", "bug-84de98", "bug-605a8a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_template_dispatch_is_observed_once_on_the_router' crates/roko-serve/src/ && cargo test -p roko-serve --lib a_template_dispatch_is_observed_once_on_the_router"
+++

## Problem

Template dispatch in roko-serve (`crates/roko-serve/src/dispatch.rs`) records its outcome on the cascade router twice over:

- it journals one observation of the dispatch on the shared router (`state.cascade_journal.observe_model_call(…, "template_dispatch", …)`, :2093);
- then `drain_dispatch_learning_events` (:2695) calls `record_cascade_router_outcome_with_layout` for every `AgentEvent::TurnCompleted` it drains (:2707-2714), keyed by `template.model`.

`template.model` can be a `[models.*]` config key rather than a model slug, so the outcome may land on an arm that doesn't match the served model. One dispatch with several turns counts once in the journal and once more per turn.

## Why it matters

Cybernetic core (epic spec-6ac537): routing learns from inflated and misattributed outcomes. bug-8b0d0a fixed the gateway's triple observation; this is the template-dispatch path.

## Where

`drain_dispatch_learning_events` and `record_cascade_router_outcome_with_layout` (:2784), next to the journaled observation at :2085-2100.

## Current state

At 7fa54b873 both recordings happen, as described.

## Plan

1. Keep one observation per dispatch: the journaled one, keyed by the resolved model slug. Drop the per-`TurnCompleted` router recording, or make it count the dispatch once.
2. Resolve a config key to its slug before any router update.
3. Add `a_template_dispatch_is_observed_once_on_the_router`: a dispatch with several turns changes the router's counts by one observation on the served model's arm.

## Done when

- [ ] A template dispatch updates the router once, on the served model's arm.
- [ ] The `[[verify]]` command passes.
