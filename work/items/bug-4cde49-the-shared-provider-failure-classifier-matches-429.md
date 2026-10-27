+++
id = "bug-4cde49"
kind = "bug"
title = "The shared provider-failure classifier matches 429, 502 and 503 anywhere in a failure's text, so a token count or request id decides its class, and 504 has none"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-466060 (worker, 2026-10-27)"
discovered_from = "gap-466060: comparing provider_error_kind's copy with the shared classifier; bug-e03f92 fixed the same flaw for 401"
anchors = ["crates/roko-agent/src/provider/error_classify.rs::classify_failure_text", "crates/roko-agent/src/provider/error_classify.rs::mentions_http_401"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-466060", "bug-e03f92"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn http_status_numbers_count_only_as_statuses' crates/roko-agent/ && cargo test -p roko-agent --lib http_status_numbers_count_only_as_statuses"
+++

## Problem

`classify_failure_text` (`crates/roko-agent/src/provider/error_classify.rs`) matches `429`, `502` and `503` as
substrings anywhere in a failure's text, so digits inside a token count, a byte count or a request id decide the
class:

- `This model's maximum context length is 128000 tokens. However, your messages resulted in 150234 tokens.`
  classifies as `server_error`, because 150234 holds 502;
- `context length exceeded (request id req_5031)` also classifies as `server_error`;
- any failure whose request id holds `429` classifies as `rate_limit`, unless an earlier class names it first.

`mentions_http_401` fixed the same flaw for 401 (bug-e03f92), and `403` counts only together with `forbidden`. The
status `504` has no class at all. `upstream returned 504` and nginx's `504 Gateway Time-out` are `unknown`; only
`Gateway Timeout`, through its word `timeout`, is a timeout.

## Why it matters

Goal: truth (provider-health accuracy). A failure with the wrong class gets the wrong cooldown and circuit treatment
(a rate limit waits 60 s, a server error 30 s), and the ladder records the wrong infra class. The provider-call
boundary (`model_call_service::provider_error_kind`) delegates to this classifier since gap-466060.
`provider_error_kind_delegates_to_the_shared_classifier` pins `server_error` for the context-length text with
`req_5031`. The copy that gap-466060 replaced named it `context_overflow`, because it checked the context window
before the 5xx digits.

## Where

- `crates/roko-agent/src/provider/error_classify.rs::classify_failure_text`: the `429`, `502` and `503` substrings.
- `crates/roko-agent/src/provider/error_classify.rs::mentions_http_401`: the status-shaped matcher to generalise.

## Plan

1. Generalise `mentions_http_401` into a status matcher (for example `mentions_http_status(lower, 429)`) and use it
   for 429, 502, 503 and 504. Keep the words that need no number: `too many requests`, `rate limit`,
   `service unavailable`, `server error`, and add `bad gateway` and `gateway time-out` (504 as a timeout or a server
   error, to be decided).
2. Test both sides: a status counts, and the same digits inside a longer number, a path or an id do not
   (`http_status_numbers_count_only_as_statuses`).
3. Update the pinned cases in `provider_error_kind_delegates_to_the_shared_classifier`.

## Done when

The texts above take the class their words give, a 504 has a class, and the `[[verify]]` passes.
