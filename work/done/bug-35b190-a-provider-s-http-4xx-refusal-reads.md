+++
id = "bug-35b190"
kind = "bug"
title = "A provider's HTTP 4xx refusal reads as a network error: OpenAI's 400 invalid_request_error on reasoning_effort in phase 4's run 16fc88ae came out as 'network error: http 400', failure class provider_error:unknown, and a 403 or 404 reads the same"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "7440f7be7"
source = "round-v7 phase 4 run 16fc88ae (Undertow .roko/roko.log 08:06Z), the lead's ask"
discovered_from = "agent:hermes-pilot (round v7 phase 4)"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs::classify_http_error", "crates/roko-agent/src/provider/mod.rs::ProviderError", "crates/roko-agent/src/provider/error_classify.rs::classify_failure_text", "crates/roko-cli/src/dispatch/rung_probe.rs::failure_reason"]
links = { depends_on = [], blocks = [], related = ["bug-29014c", "bug-390e4b", "gap-466060"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_providers_4xx_refusal_reads_as_a_refusal_not_a_network_error' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_providers_4xx_refusal_reads_as_a_refusal_not_a_network_error"

[[verify]]
command = "grep -rqw 'fn a_refused_request_is_not_sent_again' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_refused_request_is_not_sent_again"

[[verify]]
command = "grep -rqw 'fn a_rung_probe_records_a_refusals_words' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_rung_probe_records_a_refusals_words"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T10:27:22Z"
commit = "7440f7be7"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-21T09:37:11Z"
forced = false
evidence = "7440f7be7. openai_compat_backend::classify_http_error now takes the provider (send_turn and stream_turn pass self.provider_id, the SSE error path its provider) and sends every 4xx but 401 (AuthFailure, unchanged) and 429 (rate limit) through error_classify::classify_http_refusal: a 402 or a 4xx whose words or code say the account can't pay is bug-390e4b's billing refusal, a 400 about the context (context_length_exceeded) stays ContextOverflow, a 408 is Timeout, and any other 4xx becomes ProviderError::RequestRefused { provider, status, message }: the provider's own words (error.message, message or error, whitespace collapsed, at most 400 characters) with the param and code it named. It renders as \"openai refused the request (HTTP 400): Function tools with reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions. (param reasoning_effort)\". Transport errors (no status) stay LlmError::Network. refusal_class(status) names the class: auth_failure for 401/403, model_not_found for 404, else request_refused; classify_failure_text (via refused_request_status, so the ladder's infra class reads provider_error:request_refused), model_call_service::provider_error_kind and the openai_compat, gemini_native and anthropic tool-loop circuit-breaker kinds use it, and classify_failure_text names \"model not found\" as model_not_found. RetryPolicy::should_retry never sends a RequestRefused again (ErrorClass::RequestRefused). The rung probe's failure_reason keeps a refusal from the provider's name on. Circuit-breaker accounting is unchanged: request_refused and model_not_found count as Unknown, as a 400 did. Tests: a_providers_4xx_refusal_reads_as_a_refusal_not_a_network_error (OpenAI's 400 body: RequestRefused with those words and class request_refused; a 401 stays AuthFailure, auth_failure; a 403 no-access refusal is auth_failure with its words; a 404 is model_not_found; a context-length 400 stays ContextOverflow; transport errors stay Network, a timed-out one classed timeout; a 408 is Timeout), a_refused_request_is_not_sent_again (one call, the run ends in the provider's words; an untyped provider error is still retried), a_rung_probe_records_a_refusals_words (end to end over the OpenAI-compatible mock: one request, reason \"request_refused: mid_api refused the request (HTTP 400): Function tools ... (param reasoning_effort)\", also in the probe cache); the classify_http_error, retry, billing and probe tests beside them pass. fmt and clippy -D warnings clean; lib suites roko-agent 2397, roko-cli 3988, roko-core 2047, roko-learn 1393, roko-runtime 213, roko-serve 1081, all passing."
+++

## Problem

roko's OpenAI-compatible tool-loop backend files a provider's HTTP 4xx refusal as a network error. Phase 4's run
16fc88ae (Undertow `.roko/roko.log`, 08:06Z) shows it. OpenAI answered 400 `invalid_request_error`, `param:
reasoning_effort`, "Function tools with reasoning_effort are not supported…" (from bug-29014c's unmerged binary).
roko reported:

    agent error (openai): network error: http 400: {…}

- The failure class came out `provider_error:unknown`: the ladder's `infra_failure_class` and the shared
  `classify_failure_text` know no refused request.
- The plugin, and anyone reading roko's text, says "the network failed".
- A 403 (no access to the model) or a 404 (no such model) reads the same, and sends people to their connection
  instead of their key or model name.
- A rung probe that gets one records a cut-off "unknown: agent error (openai): network error: http 400: {"error":
  {…" with the provider's words lost.

## Where

- `crates/roko-agent/src/openai_compat_backend.rs::classify_http_error`: 429/529 rate limited, 5xx server error, 401
  auth failure, 402 billing refusal (bug-390e4b); every other status, 400, 403, 404 and 422 included, becomes
  `LlmError::Network(e.to_string())`. `sse_error_to_llm_error` routes a streamed error object with a status through
  it too.
- `crates/roko-agent/src/provider/mod.rs::ProviderError`: no variant carries a refused request's status and words.
- `crates/roko-agent/src/retry.rs::RetryPolicy::should_retry`: an untyped provider error is retried twice.
- `crates/roko-agent/src/provider/error_classify.rs::classify_failure_text` and
  `crates/roko-agent/src/model_call_service.rs::provider_error_kind`: the failure classes.
- `crates/roko-cli/src/dispatch/rung_probe.rs::failure_reason`: keeps the first 160 characters of the error text.

## Plan

- A 4xx other than the 401, 402 and 429 cases becomes a typed `ProviderError::RequestRefused { provider, status,
  message }`. Its text reads e.g. `openai refused the request (HTTP 400): Function tools with reasoning_effort are not
  supported… (param reasoning_effort)`, with the provider's own message from the body.
  - A 400 whose words say the context is too long stays `ContextOverflow`, as the shared HTTP classifier has it.
  - A 408 is a timeout. Any 4xx whose words say the account can't pay is a billing refusal (bug-390e4b).
- The failure class distinguishes the refusals: `request_refused`, `auth_failure` for a 401 or 403 (a bad key or no
  access), `model_not_found` for a 404. The class comes from the status in the text, in the shared classifier and
  in the circuit breaker's kinds.
- A refused request is never sent again: the same request gets the same answer.
- A rung probe that gets one records the provider's words.
- Real transport errors (no status) stay network errors.
- Whether a refusal should count against the provider's circuit is left as it is: a 400 counted as an unknown
  failure before, and still does.

## Done when

A 400 with OpenAI's body reads as `openai refused the request (HTTP 400): …`, classifies as `request_refused` and is
not retried; a 401 stays an auth failure; a transport error and a timeout stay network and timeout errors; and a rung
probe records the refusal's words. Checked by the tests in `[[verify]]`.
