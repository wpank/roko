+++
id = "find-cb5eeb"
kind = "finding"
title = "Rate limit cooldown of 5 s is too short for actual provider rate windows"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
anchors = ["crates/roko-learn/src/provider_health.rs::cooldown_ms", "crates/roko-learn/src/provider_health.rs::record_exhaustion"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/fn cooldown_ms/,/^    }/p' crates/roko-learn/src/provider_health.rs | grep -q 'ErrorClass::RateLimit => 5_000'"
+++
The circuit breaker applies a 5 s cooldown for `ErrorClass::RateLimit`. Actual provider rate limit windows are typically 60 s to 900 s for Anthropic, OpenAI, and Gemini. A 5 s cooldown causes the agent to immediately retry and receive another 429, rapidly exhausting retries.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: Rate limit cooldown of 5 s is too short for actual provider rate windows

Verified 2026-09-28: cooldown_ms still maps ErrorClass::RateLimit => 5_000, as does the default arm (crates/roko-learn/src/provider_health.rs:370). Only usage-window exhaustion honours a provider-reported reset (record_exhaustion, ~:229-237 and :474-495). The breaker trips only after 3 consecutive failures, which softens but does not fix this. Severity p2 (tuning default).

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7cdce7; cargo verification deferred to the batch check.
  - `ProviderHealth::cooldown_ms` gives `RateLimit` 60 s, one provider rate window, instead of 5 s.
  - Raising one class above `ServerError` exposed a second problem: a later failure replaced the running cooldown,
    so a server error after a rate limit would have cut the window to 30 s. `record_failure` now keeps the later of
    the running cooldown and the new one, which also stops a stray failure shortening a 24 h billing quarantine.
  - Tests updated for 60 s. `additional_failures_extend_cooldown` now also checks that a shorter class leaves the
    cooldown alone and a longer one extends it.
  - Not done: the provider's `retry-after` is still not honoured for rate limits. Only `record_exhaustion` takes a
    reported reset.
