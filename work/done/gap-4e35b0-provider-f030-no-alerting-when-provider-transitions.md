+++
id = "gap-4e35b0"
kind = "gap"
title = "No alerting when provider transitions to Open (tripped) state"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "2f82da96a"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030"
anchors = ["crates/roko-learn/src/provider_health.rs::ProviderHealth::record_failure", "crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry::record_failure"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/pub fn record_failure(&self, provider_id/,/^    }/p' crates/roko-learn/src/provider_health.rs | grep -q 'warn!'"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T08:50:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:13Z"
forced = false
evidence = "implemented by wk-tiers: ProviderHealthRegistry::record_failure logs one warn! naming the provider, error class, failure count and cooldown end when a circuit trips to Open. The verify passes; roko-learn lib tests pass (gate 6i at f4347b8eb). A StateHub or Bus event for the dashboard is not done (roko-learn can't reach it)"
+++
When the circuit breaker trips to `Open` state after 3 consecutive failures, no alert, log at warn level, or push notification is emitted. Operators have no way to know a provider has been automatically disabled unless they poll the health endpoint.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F030`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: No alerting when provider transitions to Open (tripped) state

Verified 2026-09-28: ProviderHealth::record_failure flips state to Open (crates/roko-learn/src/provider_health.rs:221-226) without logging or emitting an event. ProviderHealthRegistry::record_failure only emits a per-failure info! 'provider failure recorded' (:452-457). No warn and no StateHub/Bus event marks the Closed->Open transition. Severity p2 (observability).

Rechecked 2026-09-29: unchanged. Graph dispatch failover (crates/roko-cli/src/graph_task_dispatch.rs::blocked_provider) now reports 'circuit open after repeated failures' when it skips a provider whose circuit is open, but nothing warns or emits an event at the moment the breaker trips.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7cdce7; cargo verification deferred to the batch check.
  - `ProviderHealthRegistry::record_failure` emits one `warn!` when a failure moves a provider's circuit into Open,
    from Closed or from a failed half-open probe. The warning names the provider, error class, consecutive failures
    and cooldown end. The per-failure `info!` stays.
  - No test: roko-learn has no log-capture helper, and the transition itself is covered by the existing breaker
    tests. Usage exhaustion still logs its quarantine at `info!` (`record_exhaustion`).
  - Not done: there is no StateHub or Bus event for the trip; roko-learn cannot reach the dashboard.
