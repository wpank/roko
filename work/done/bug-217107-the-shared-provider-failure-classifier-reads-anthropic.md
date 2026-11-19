+++
id = "bug-217107"
kind = "bug"
title = "The shared provider-failure classifier reads Anthropic's authentication_error (the Claude CLI's expired OAuth token) as unknown, not an auth failure"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "b0cd36b7f"
source = "gap-466060 (worker, 2026-10-27)"
discovered_from = "gap-466060: provider_error_kind's copy named it auth_failure through any 'authentication'; the shared classifier it now delegates to does not"
anchors = ["crates/roko-agent/src/provider/error_classify.rs::is_auth_failure_text"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-466060"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn anthropic_authentication_error_is_an_auth_failure' crates/roko-agent/ && cargo test -p roko-agent --lib anthropic_authentication_error_is_an_auth_failure"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T10:40:48Z"
commit = "b0cd36b7f"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-27T10:25:54Z"
forced = false
evidence = "b0cd36b7f: error_classify::is_auth_failure_text counts authentication_error, the error type Anthropic sends with a 401 for a bad or expired credential and the Claude CLI passes on as API Error: 401 {...\"type\":\"authentication_error\"...} (the text of its expired OAuth token). classify_failure_text names that text auth_failure now, and so does model_call_service::provider_error_kind, which delegates to it through classify_call_failure_text (gap-466060). A permission_error 403 stays unknown (bug-80f0d0). The verify passes: the grep finds fn anthropic_authentication_error_is_an_auth_failure, and cargo test -p roko-agent --lib anthropic_authentication_error_is_an_auth_failure ran 1 test, passed, exit 0. provider_error_kind_delegates_to_the_shared_classifier moves both texts (the expired-token text and authentication_error: invalid x-api-key) from the cases where the old copy answered otherwise to those where it agrees. roko-agent lib: 2433 passed, 4 ignored. clippy -p roko-agent --no-deps -- -D warnings and nightly fmt are clean."
+++

## Problem

`classify_failure_text` (`crates/roko-agent/src/provider/error_classify.rs`) does not treat Anthropic's
`authentication_error` as an auth failure. That is the error type Anthropic's API returns with a 401 for a bad or
expired credential, and the Claude CLI passes it on in its `API Error: <status> <body>` form (the runner's
classifier already matches `api error: 529`, `crates/roko-cli/src/runner/types.rs`). The CLI's text for an expired
OAuth token is
`API Error: 401 {"type":"error","error":{"type":"authentication_error","message":"OAuth token has expired. …"}}`.
None of `is_auth_failure_text`'s words match it. `mentions_http_401` needs `http`, `status`, `code`, `unauthorized`,
`request` or `returned` within three words of the 401, and `api error: 401` has none of them. So the class is
`unknown`. `authentication_error: invalid x-api-key` is `unknown` too.

## Why it matters

Goal: truth (provider-health accuracy). An auth failure opens the provider's circuit at once for 24 hours
(`ProviderHealth::record_failure`, backlog 1115). An unknown failure is retried, and it opens the circuit only after
three in a row. Graph dispatch (`dispatch_v2::classify_provider_error`), the ladder, the rung probe and ACP learning
all read this classifier.

Until gap-466060, the provider-call boundary (`model_call_service::provider_error_kind`) kept its own copy of the
classifier, which matched any `authentication` and so named these texts `auth_failure`. It delegates now, and
`model_call_service::tests::provider_error_kind_delegates_to_the_shared_classifier` pins the shared answer
(`unknown`) for both texts.

## Where

- `crates/roko-agent/src/provider/error_classify.rs::is_auth_failure_text`: the auth words.
- `crates/roko-agent/src/provider/mod.rs::map_provider_error` already reads `authentication_error` as an invalid
  key, for the message it shows the user.

## Current state

The words that count as auth are `not logged in`, `please run /login`, `invalid api key`, `invalid_api_key`,
`unauthorized`, `authentication failed` (the rendering of `ProviderError::AuthFailure`), an HTTP 401 (bug-e03f92),
and `403` together with `forbidden`.

## Plan

1. Add `authentication_error` to `is_auth_failure_text`.
2. Pin the expired-token text above as `auth_failure` in a test in error_classify's tests
   (`anthropic_authentication_error_is_an_auth_failure`).
3. Move its two cases in `provider_error_kind_delegates_to_the_shared_classifier` from the "copy answered otherwise"
   table to the agreeing one.

## Done when

The Claude CLI's expired-token text classifies as `auth_failure` through `classify_failure_text` and through
`provider_error_kind`, and the `[[verify]]` passes.

## Notes

Anthropic's 403 type, `permission_error`, is not a credential failure. It stays `unknown`, as bug-80f0d0 decided for
a 403 that does not say the key is bad.
