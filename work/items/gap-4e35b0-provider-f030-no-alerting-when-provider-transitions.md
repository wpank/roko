+++
id = "gap-4e35b0"
kind = "gap"
title = "[provider F030] No alerting when provider transitions to Open (tripped) state"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
anchors = ["crates/roko-learn/src/provider_health.rs::record_failure"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
When the circuit breaker trips to `Open` state after 3 consecutive failures, no alert, log at warn level, or push notification is emitted. Operators have no way to know a provider has been automatically disabled unless they poll the health endpoint.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: No alerting when provider transitions to Open (tripped) state

Verified 2026-09-28: ProviderHealth::record_failure flips state to Open (crates/roko-learn/src/provider_health.rs:221-226) without logging or emitting an event. ProviderHealthRegistry::record_failure only emits a per-failure info! 'provider failure recorded' (:452-457). No warn and no StateHub/Bus event marks the Closed->Open transition. Severity p2 (observability).
