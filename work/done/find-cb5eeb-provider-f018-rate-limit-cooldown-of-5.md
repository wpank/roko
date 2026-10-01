+++
id = "find-cb5eeb"
kind = "finding"
title = "Rate limit cooldown of 5 s is too short for actual provider rate windows"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
anchors = ["crates/roko-learn/src/provider_health.rs::cooldown_ms", "crates/roko-learn/src/provider_health.rs::record_exhaustion"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/fn cooldown_ms/,/^    }/p' crates/roko-learn/src/provider_health.rs | grep -q 'ErrorClass::RateLimit => 5_000'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:57Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:13Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
