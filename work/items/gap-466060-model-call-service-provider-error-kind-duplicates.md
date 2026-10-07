+++
id = "gap-466060"
kind = "gap"
title = "model_call_service::provider_error_kind duplicates the unified provider-error classifier"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/model-call-service"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK01 gap-625195)"
discovered_from = "gap-625195 (backlog task 1113, implemented at d562b56b2, did not cover this call site)"
anchors = ["crates/roko-agent/src/model_call_service.rs::provider_error_kind", "crates/roko-agent/src/provider/error_classify.rs::classify_failure_text"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn provider_error_kind_delegates_to_the_shared_classifier' crates/roko-agent/ && cargo test -p roko-agent provider_error_kind_delegates_to_the_shared_classifier"
+++

## Problem

`crates/roko-agent/src/model_call_service.rs::provider_error_kind` (line 2148) is a third, independent
implementation of provider-error classification. Backlog task 1113 ("classify_provider_error reuses the shared
classifier and names auth failures", part of gap-625195, implemented at commit d562b56b2) unified
`crates/roko-cli/src/dispatch_v2.rs::classify_provider_error`, which now only delegates:
`roko_agent::provider::error_classify::classify_failure_text(output_text_lower)` (`dispatch_v2.rs:1958-1961`,
comment: "One classifier for every provider-health caller (backlog 1113)"). `model_call_service::provider_error_kind`
was not touched by that unification and still hand-rolls its own string matching, despite 1113's comment claiming
coverage of "every provider-health caller."

## Why it matters

Goal: truth / provider-health accuracy. `provider_error_kind`'s own doc comment says it exists "for circuit-breaker
outcome recording" (`ProviderOutcomeRecorder::record_provider_failure`) — the same kind of decision (quarantine vs.
retry) the unified classifier exists to make consistent across providers. A second, independently maintained copy
can silently drift from the canonical one — it already has: it classifies `context_overflow` and `content_policy`,
which `classify_failure_text` doesn't label at all, while `classify_failure_text` has `empty_response`, which
`provider_error_kind` doesn't check for.

## Where

- `crates/roko-agent/src/model_call_service.rs:2148-2199` (`provider_error_kind`): hand-rolled `if`/`else` chain over
  `message.to_ascii_lowercase()`, called from `openai_compat_backend.rs:389`,
  `provider/anthropic_api/tool_loop.rs:596`, `tool_loop/backends/gemini_native.rs:111`.
- `crates/roko-agent/src/provider/error_classify.rs:291` (`classify_failure_text`): the canonical classifier task
  1113 pointed `dispatch_v2.rs` at. Returns the same kind of `&'static str` label set
  (`"provider_exhausted"`, `"insufficient_credits"`, `"auth_failure"`, `"rate_limit"`, `"timeout"`, `"server_error"`,
  `"empty_response"`, `"unknown"`) that `provider_error_kind` does, minus `context_overflow`/`content_policy`, plus
  `empty_response`.
- `crates/roko-cli/src/dispatch_v2.rs:1958-1961` (`classify_provider_error`): already unified, the model to follow.

## Current state

`provider_error_kind` already calls into `error_classify`'s helpers for three cases (`detect_provider_exhaustion`,
`is_billing_message`, `mentions_http_401`), so it's partially aligned, but keeps its own control flow and two extra
categories (`context_overflow`, `content_policy`) that `classify_failure_text` has no equivalent for.

## Plan

1. Add `context_overflow` and `content_policy` cases to `classify_failure_text` (or a variant of it) in
   `error_classify.rs`.
2. Replace `model_call_service::provider_error_kind`'s body with a delegation to the extended shared classifier, the
   same pattern `dispatch_v2.rs::classify_provider_error` uses.
3. Keep `provider_error_kind`'s signature (`&str -> &'static str`) so its three call sites don't need to change.
4. Add/extend a test exercising a context-overflow and a content-policy message through the shared classifier to
   confirm parity with the old `provider_error_kind` behavior.

## Done when

- `model_call_service::provider_error_kind` delegates to the shared classifier in `error_classify.rs` instead of
  duplicating its own matching.
- The `[[verify]]` command passes.

## Notes

Check call sites expect exactly the same label strings after the change (`"context_overflow"`, `"content_policy"`,
etc.) — this is used for circuit-breaker bookkeeping, not just logging, so a label rename would change behavior
downstream.
