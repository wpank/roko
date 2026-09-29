+++
id = "gap-4e35b0"
kind = "gap"
title = "No alerting when provider transitions to Open (tripped) state"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
anchors = ["crates/roko-learn/src/provider_health.rs::ProviderHealth::record_failure", "crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry::record_failure"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/pub fn record_failure(&self, provider_id/,/^    }/p' crates/roko-learn/src/provider_health.rs | grep -q 'warn!'"
+++
When the circuit breaker trips to `Open` state after 3 consecutive failures, no alert, log at warn level, or push notification is emitted. Operators have no way to know a provider has been automatically disabled unless they poll the health endpoint.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: No alerting when provider transitions to Open (tripped) state

Verified 2026-09-28: ProviderHealth::record_failure flips state to Open (crates/roko-learn/src/provider_health.rs:221-226) without logging or emitting an event. ProviderHealthRegistry::record_failure only emits a per-failure info! 'provider failure recorded' (:452-457). No warn and no StateHub/Bus event marks the Closed->Open transition. Severity p2 (observability).

Rechecked 2026-09-29: unchanged. Graph dispatch failover (crates/roko-cli/src/graph_task_dispatch.rs::blocked_provider) now reports 'circuit open after repeated failures' when it skips a provider whose circuit is open, but nothing warns or emits an event at the moment the breaker trips.
