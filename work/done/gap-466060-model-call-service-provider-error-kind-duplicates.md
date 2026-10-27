+++
id = "gap-466060"
kind = "gap"
title = "model_call_service::provider_error_kind duplicates the unified provider-error classifier"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/model-call-service"]
created = 2026-10-02
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "e1050464d"
source = "backlog wave reports 2026-10-02 (PK01 gap-625195)"
discovered_from = "gap-625195 (backlog task 1113, implemented at d562b56b2, did not cover this call site)"
anchors = ["crates/roko-agent/src/model_call_service.rs::provider_error_kind", "crates/roko-agent/src/provider/error_classify.rs::classify_failure_text"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = [], blocks = [], related = ["bug-217107", "gap-354ea9", "bug-4cde49"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn provider_error_kind_delegates_to_the_shared_classifier' crates/roko-agent/ && cargo test -p roko-agent provider_error_kind_delegates_to_the_shared_classifier"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T09:41:43Z"
commit = "e1050464d"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-27T08:30:55Z"
forced = false
evidence = "e1050464d: model_call_service::provider_error_kind is one line now, error_classify::classify_call_failure_text(&message.to_ascii_lowercase()). classify_call_failure_text (error_classify.rs) returns the class classify_failure_text gives, except that text it calls unknown is context_overflow (context window, context length, context overflow) or content_policy (content policy, safety policy). So the two extra classes of the copy survive, and the other callers of classify_failure_text (Graph dispatch, failover, the ladder, the rung probe, ACP learning) are unchanged. A comparison over 130 failure texts found ten cases where the copy answered otherwise. The item lists them, and provider_error_kind_delegates_to_the_shared_classifier pins examples of each to the shared answer next to the old one: model not found is model_not_found; login and key failures are auth_failure, ahead of rate limit and timeout; a bare 403, a lone forbidden, permission denied and authentication_error are no longer auth failures; broken streams are server_error; refused connections and DNS failures are unknown instead of timeout; 504 is unknown; 502 or 503 digits outrank context words; empty_response and provider_glitch keep their names. The verify passes: cargo test -p roko-agent provider_error_kind_delegates_to_the_shared_classifier ran 1 test in the lib target and passed, 0 ran elsewhere, exit 0. call_failure_text_names_context_overflow_and_content_policy covers the variant. roko-agent lib: 2432 passed, 4 ignored. clippy -p roko-agent --no-deps -- -D warnings and nightly fmt --check are clean. Filed bug-217107 (authentication_error reads as unknown, p2), gap-354ea9 (no class for a connection that never opened, p3) and bug-4cde49 (bare 429/502/503 digits, and 504 has no class, p3)."
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

## Where the copy and the shared classifier disagreed (2026-10-27)

`provider_error_kind` now delegates to `error_classify::classify_call_failure_text`, a variant of the shared
classifier. It returns `classify_failure_text`'s class, except for text that `classify_failure_text` calls `unknown`:
such text is a `context_overflow` when it says "context window", "context length" or "context overflow", and a
`content_policy` when it says "content policy" or "safety policy". Those are the copy's own words for its two extra
classes.

The other callers of `classify_failure_text` are unchanged and still read both as `unknown`: Graph dispatch's provider
health, failover, the ladder, the rung probe, ACP learning and `ErrorClass::from_failure_text`. Adding the two classes
to `classify_failure_text` itself would have changed them. ACP learning, for one, would stop counting a context
overflow against the model.

The class `provider_error_kind` returns is used in three places:

- the circuit breaker, through `ProviderCallCell::record_provider_attempt`;
- the API backends' outcome recorders, which call it only for `ProviderError::Other`, `LlmError::Backend` and
  `LlmError::Network` (typed errors map straight to their class);
- the efficiency log, through the `error_class` of `ModelCallService::call`'s feedback for a failed call.

Exhaustion, billing, a refused request's status, rate limits, timeouts, 502, 503 and "server error" get the same
answer as before. So do the context and content-policy words, in text the shared classifier calls `unknown`.

The copy and the delegation were compared over 130 failure texts: each typed provider error as an attempt's text
carries it, the transport's errors, the CLIs' words and `map_provider_error`'s messages. The disagreements are below.
`model_call_service::tests::provider_error_kind_delegates_to_the_shared_classifier` pins examples of each case to the
shared answer, with the copy's answer beside them.

1. **"model not found"** (`provider error: model not found`, or `map_provider_error`'s "Model not found on provider
   …"). The copy said `unknown` (`timeout` if the text also said "timed out"). Now `model_not_found`, which never
   counts toward the provider's circuit (bug-80f0d0).
2. **Login and key failures** (`Not logged in · Please run /login`, `Invalid API key · …`, `invalid_api_key`). The
   copy said `unknown` (`timeout` with "timed out"). Now `auth_failure`, which opens the circuit at once (backlog
   1113).
3. **An auth failure that also mentions a rate limit or a timeout** (`401 Unauthorized (request id req_429170)`,
   `unauthorized: request timed out`). The copy said `rate_limit` or `timeout`. Now `auth_failure`, because the
   shared classifier checks auth first.
4. **Auth words only the copy had**:
   - a bare `403` (`HTTP 403: Your organization does not have access to this model`);
   - `forbidden` without 403;
   - `permission denied`, including a tool refusal, which `ToolError::PermissionDenied` renders as
     `permission denied: …`;
   - a 403 inside a number (`stream ended after 1403 tokens`).

   The copy said `auth_failure`, which kept the provider out for 24 hours. Now `unknown`: a 403 that does not say
   forbidden is most often scoped to the model (bug-80f0d0).
5. **Anthropic's `authentication_error`** (the Claude CLI's `API Error: 401 {…}` for an expired OAuth token, and
   `authentication_error: invalid x-api-key`). The copy said `auth_failure`. Now `unknown`. The boundary loses a
   right answer here; filed as bug-217107 against the shared classifier.
6. **A stream broken mid-response** (`network error: read chunk failed: error decoding response body`,
   `connection reset by peer`, `broken pipe`). The copy said `timeout` (`unknown` when the text had no "network").
   Now `server_error` (bug-9b65dc).
7. **A connection that never opened** (`tcp connect error: Connection refused`, `dns error: …`,
   `network is unreachable`, `ECONNREFUSED`, `ENOTFOUND`). The copy said `timeout` (its P0-16 branch). Now `unknown`.
   Filed as gap-354ea9. The openai-compatible backend's own refused-connection message ("Cannot reach provider …")
   was already `unknown` in the copy.
8. **A 504 without "timeout"** (`upstream returned 504`, nginx's `504 Gateway Time-out`), and a 504 inside a number
   (`wrote 15040 bytes`). The copy said `server_error`. Now `unknown`. Filed in bug-4cde49.
9. **Context words beside a class the shared classifier names first**. Examples: `server error: context window
   exceeded`, or 502 or 503 inside a request id or a token count (`context length exceeded (request id req_5031)`,
   `… resulted in 150234 tokens`). The copy said `context_overflow`. Now `server_error`. The digit cases are
   bug-4cde49.
10. **`empty_response …` and `provider_glitch: …`**. The copy said `unknown`. Now `empty_response` and
    `provider_glitch`. `ErrorClass::from_kind` maps both to `Unknown`, so the circuit is unchanged and only the label
    differs.
