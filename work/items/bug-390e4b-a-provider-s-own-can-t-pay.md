+++
id = "bug-390e4b"
kind = "bug"
title = "A provider's own 'can't pay' answer is read as a rate limit or a plain failure, not as exhausted: z.ai's 429 code 1113 ('Insufficient balance or no resource package') is retried and waited out, every later try calls z.ai first, and Cerebras' 402 Payment Required spends the try"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7 roko-speed, z.ai 1113 since 06:13 on 2026-10-21 (run 7b8cb121), the lead's ask"
discovered_from = "agent:roko-speed (round v7 glm study probe)"
anchors = ["crates/roko-agent/src/provider/error_classify.rs::classify_rate_limited", "crates/roko-agent/src/provider/error_classify.rs::detect_provider_exhaustion", "crates/roko-agent/src/openai_compat_backend.rs::classify_http_error", "crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover"]
links = { depends_on = [], blocks = [], related = ["bug-f9ca2f", "bug-62ae1e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_provider_that_cannot_bill_is_skipped_at_its_first_answer' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_provider_that_cannot_bill_is_skipped_at_its_first_answer"

[[verify]]
command = "grep -rqw 'fn a_can_t_pay_answer_is_exhaustion_not_a_rate_limit' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_can_t_pay_answer_is_exhaustion_not_a_rate_limit"
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
