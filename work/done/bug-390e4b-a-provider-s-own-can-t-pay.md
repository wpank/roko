+++
id = "bug-390e4b"
kind = "bug"
title = "A provider's own 'can't pay' answer is read as a rate limit or a plain failure, not as exhausted: z.ai's 429 code 1113 ('Insufficient balance or no resource package') is retried and waited out, every later try calls z.ai first, and Cerebras' 402 Payment Required spends the try"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "40c3c4a7e"
source = "round-v7 roko-speed, z.ai 1113 since 06:13 on 2026-10-21 (run 7b8cb121), the lead's ask"
discovered_from = "agent:roko-speed (round v7 glm study probe)"
anchors = ["crates/roko-agent/src/provider/error_classify.rs::classify_rate_limited", "crates/roko-agent/src/provider/error_classify.rs::detect_provider_exhaustion", "crates/roko-agent/src/openai_compat_backend.rs::classify_http_error", "crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover"]
links = { depends_on = [], blocks = [], related = ["bug-f9ca2f", "bug-62ae1e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_provider_that_cannot_bill_is_skipped_at_its_first_answer' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_provider_that_cannot_bill_is_skipped_at_its_first_answer"

[[verify]]
command = "grep -rqw 'fn a_can_t_pay_answer_is_exhaustion_not_a_rate_limit' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_can_t_pay_answer_is_exhaustion_not_a_rate_limit"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T05:53:36Z"
commit = "40c3c4a7e"
by = "roko-speed"
executor = "unknown"
forced = false
evidence = "40c3c4a7e. error_classify::billing_refusal turns a can't-pay body (JSON error.message, message or error, with error.code or code; else the text) into ProviderExhausted { resets_at_ms: None, message: \"<words> (code <code>)\" }; is_billing_message knows z.ai's \"insufficient balance\" and \"no resource package\"; classify_rate_limited returns it for a billing 429, and openai_compat_backend::classify_http_error for any 402 (a network error before). InsufficientCredits' text starts with BILLING_REFUSAL_MARKER, which is_exhaustion_text reads, so detect_provider_exhaustion takes both. Failover's exhaustion branch (record_exhaustion, the provider skipped for [routing] exhaustion_cooldown_secs, failover within the try) publishes DashboardEvent::ProviderExhausted {plan_id, task_id, attempt, provider, model, billing, reason, until_ms, detail}; serve logs it as provider_exhausted, health lists it, and GET /api/runs/{id}/summary lists provider_exhausted, one per provider with its latest refusal; docs/v3/26-HTTP-API.md. Tests: a_provider_that_cannot_bill_is_skipped_at_its_first_answer (the mid rung's provider answers 429 code 1113, then in a second case 402: one request, no retry or wait, the strong rung runs the task in the same try, the hub hears one provider_exhausted with billing=true, the provider's words and \"recharge, or the ladder runs without it until ...\", and the next try runs without calling the provider), a_can_t_pay_answer_is_exhaustion_not_a_rate_limit (an ordinary 429, code 1302, stays a rate limit), classify_http_error_402_is_a_billing_refusal, a_runs_summary_names_the_providers_it_went_without; the 429 failover tests beside them pass. fmt and clippy -D warnings clean; lib suites: roko-agent 2392, roko-cli 3963 of 3964 (the one failure, plan_check_publishes_start_heartbeat_and_finish, is bug-8d8f98's load flake at load about 100 in a file this change leaves alone; it passes alone 3 of 3), roko-core 2047, roko-learn 1391, roko-runtime 213, roko-serve 1079."
+++

## Problem

A provider that answers that it can't bill the account is treated as rate-limited, or as an ordinary failure, not as
exhausted:

- z.ai's balance ran out during Undertow phase 3 (run 7b8cb121, 06:13). Every call since gets HTTP 429 with code 1113,
  "Insufficient balance or no resource package. Please recharge." `classify_rate_limited` reads it as a transient
  rate limit, since `is_billing_message` doesn't know z.ai's wording. So the tool loop retries it, failover takes it
  for a rate limit (bug-62ae1e's own-calls wait can apply), the provider's circuit stays closed, and every later try
  calls z.ai first and fails over again. The shared rate limiter also learns a concurrency limit from it.
- Cerebras answered HTTP 402 Payment Required to every call in phase 2 (run 82b7e069, bug-f9ca2f). The
  OpenAI-compatible backend's `classify_http_error` maps a 402 to a network error, so the try that met it failed and
  was spent.
- `detect_provider_exhaustion` deliberately skips billing errors, so failover never treats them as exhaustion.

## Where

- `crates/roko-agent/src/provider/error_classify.rs`: `is_billing_message`, `classify_rate_limited`,
  `detect_provider_exhaustion`.
- `crates/roko-agent/src/openai_compat_backend.rs::classify_http_error`: 402.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover`: the exhaustion branch quarantines
  the provider for `[routing] exhaustion_cooldown_secs` and fails over within the try. Nothing on the hub says why.

## Plan

- A provider's own "can't pay" answer (a 402, or a 429 whose body says so, z.ai's 1113 included) is a
  `ProviderExhausted` refusal with the provider's words and code. Failover quarantines the provider for its
  exhaustion cooldown with no retry, no rate-limit wait and no try spent on it, and moves to the next model.
- Its other billing refusals (`InsufficientCredits`) are exhaustion to failover too.
- The hub hears a `provider_exhausted` event, and the run summary lists it, in plain words, for example "zai:
  Insufficient balance or no resource package. Please recharge. (code 1113): recharge, or the ladder runs without it".
- An ordinary 429 behaves as before.

## Done when

A 1113 answer and a 402 each mark the provider exhausted at their first answer, the try fails over without retrying
it, the next try routes past it without calling it, and the event and summary say why. Checked by the tests in
`[[verify]]`.
