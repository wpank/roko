+++
id = "bug-57ae52"
kind = "bug"
title = "UX34: Force-Backend Overrides Corrupt Cascade Router Learning"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/90-ux34-override-learning-isolation.md#90 — UX34: Force-Backend Overrides Corrupt Cascade Router Learning"
discovered_from = "audit:tmp/backlog/archive/90-ux34-override-learning-isolation.md#90 — UX34: Force-Backend Overrides Corrupt Cascade Router Learning"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs:88", "crates/roko-learn/src/cascade_router.rs::CascadeRouter::record_override_outcome", "crates/roko-cli/src/dispatch/model_routing.rs::ModelChoiceSource", "crates/roko-cli/src/graph_execution/feedback.rs:377"]
links = { depends_on = [], blocks = [], related = ["find-85a998", "gap-479bbb", "gap-ffac87"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli override_source_routes_through_dampened_path"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Present at HEAD 91b4745f8: crates/roko-cli/src/runtime_feedback/routing.rs:88-93 routes ModelChoiceSource::Override outcomes through the dampened CascadeRouter::record_override_outcome (test override_source_routes_through_dampened_path); the Graph path tags forced dispatches (dispatch_plan.forced / cli_model_override) as ModelChoiceSource::Override in graph_task_dispatch.rs, and graph_execution/feedback.rs:377 does the same for ChoiceSource::ManualOverride. gap-479bbb and gap-ffac87 describe the same UX34 issue."
+++
correctness (bandit statistics are poisoned by outcomes the router did not choose). The cascade router is a bandit-based model selection system that learns from outcomes: when a task succeeds or fails, it records which model was used and updates its probability weights accordingly. This is how…

Imported without verification from:
- `tmp/backlog/archive/90-ux34-override-learning-isolation.md#90 — UX34: Force-Backend Overrides Corrupt Cascade Router Learning`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `src/runner/event_loop.rs`.

How to verify: Check: `RunnerEvent::TaskAttemptCompleted` carries a `model_source: ModelChoiceSource` field.; The `task_attempt_completed` and `task_attempt_completed_with_timing` constructors in `types.rs` accept and store `model_source`.; All call sites of… [evidence: 00-INDEX historical claim: Archived 2026-09-07; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 2 |]

Verified 2026-09-28: closed as done; see [closed].evidence.
