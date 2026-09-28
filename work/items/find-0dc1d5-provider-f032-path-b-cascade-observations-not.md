+++
id = "find-0dc1d5"
kind = "finding"
title = "[provider F032] Path B cascade observations not covered by WAL (write-ahead log)"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F032"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F032"
anchors = ["crates/roko-learn/src/feedback_service.rs::observe_model_call", "crates/roko-learn/src/model_call_feedback.rs::observe_model_call_on_router", "crates/roko-learn/src/runtime_feedback/mod.rs::replay_and_open_wal", "crates/roko-execution/src/builder.rs:603"]
links = { depends_on = [], blocks = [], related = ["bug-012303"], supersedes = [], duplicate_of = "" }
+++
Cascade router observations from feedback Path B (dispatch_v2) are not durably logged before being applied. If the process crashes after a routing decision but before the observation persists, the observation is lost. This biases the learning signal over time.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F032`

How to verify: Confirm in crates/roko-learn/src/cascade_router.rs, crates/roko-cli/src/dispatch_v2.rs whether still true: Path B cascade observations not covered by WAL (write-ahead log)

Verified 2026-09-28: only the runtime_feedback path journals cascade observations (WalWriter; replay_and_open_wal at crates/roko-learn/src/runtime_feedback/mod.rs:181). FeedbackService::observe_model_call (feedback_service.rs:695) calls observe_model_call_on_router (model_call_feedback.rs:212), which mutates the router without a WAL entry. That path is live via crates/roko-cli/src/chat_session.rs:87, serve_runtime.rs:898 and crates/roko-execution/src/builder.rs:603. dispatch_v2.rs::dispatch_via_model_call_service (:94) itself has no callers. Severity p2 (learning-signal durability).
