+++
id = "find-cb5eeb"
kind = "finding"
title = "[provider F018] Rate limit cooldown of 5 s is too short for actual provider rate windows"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018"
anchors = ["crates/roko-learn/src/provider_health.rs::cooldown_ms", "crates/roko-learn/src/provider_health.rs::record_exhaustion"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The circuit breaker applies a 5 s cooldown for `ErrorClass::RateLimit`. Actual provider rate limit windows are typically 60 s to 900 s for Anthropic, OpenAI, and Gemini. A 5 s cooldown causes the agent to immediately retry and receive another 429, rapidly exhausting retries.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F018`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: Rate limit cooldown of 5 s is too short for actual provider rate windows

Verified 2026-09-28: cooldown_ms still maps ErrorClass::RateLimit => 5_000, as does the default arm (crates/roko-learn/src/provider_health.rs:370). Only usage-window exhaustion honours a provider-reported reset (record_exhaustion, ~:229-237 and :474-495). The breaker trips only after 3 consecutive failures, which softens but does not fix this. Severity p2 (tuning default).
