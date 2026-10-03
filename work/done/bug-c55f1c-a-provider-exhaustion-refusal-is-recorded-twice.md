+++
id = "bug-c55f1c"
kind = "bug"
title = "A provider-exhaustion refusal is recorded twice: the bridge's classifier, then failover's record_exhaustion"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-learn/provider-health"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "730b43d91"
source = "wave-4 follow-up reports 2026-10-02 (PK02)"
discovered_from = "gap-e00238 (task 1114 fixed the bridge's own internal double-write; this is a third, separate call site)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-agent/src/model_call_service.rs::provider_error_kind", "crates/roko-learn/src/provider_health.rs::record_exhaustion"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn one_exhaustion_counts_as_one_failure_record' crates/roko-cli/ && cargo test -p roko-cli one_exhaustion_counts_as_one_failure_record"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T02:27:42Z"
commit = "730b43d91"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T01:27:39Z"
forced = false
evidence = "Gate 6a (merged into main as 730b43d91, tree identical to work/backlog-batch-6a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 7,078 passed (roko-agent, roko-cli, roko-learn), roko-cli bin + golden-path canaries + operator_checkout_clean 442/442, hub_ipc 7/7, roko-learn legacy_rule_live + loop_audit_cs_reference; every [[verify]] passes."
+++

## Problem

A single provider-exhaustion refusal is recorded twice toward the provider's health state, through two independent
paths that both react to the same `AgentResult`:

1. **The bridge's generic classifier.** `crates/roko-agent/src/model_call_service.rs::provider_error_kind` (line
   2148) classifies the refusal's error text and returns `"provider_exhausted"` when
   `error_classify::detect_provider_exhaustion` matches. This feeds the bridge's own result recording (e.g.
   `recorder.record_provider_failure(provider_id, provider_error_kind(&message))`, line 1991), which ultimately
   calls into the health registry's generic failure path.
2. **Failover's dedicated exhaustion quarantine.** `crates/roko-cli/src/graph_task_dispatch/failover.rs` (around
   lines 273-304), running immediately after the SAME bridge call returns with `dispatch.result.success == false`,
   independently re-detects the exhaustion from `dispatch.result.output.body.as_text()` via the identical
   `detect_provider_exhaustion` check, computes its own `until_ms`, and calls
   `self.factory.health_registry.record_exhaustion(&provider_id, until_ms)` directly on the registry.

`ProviderHealthRegistry::record_exhaustion` (`crates/roko-learn/src/provider_health.rs:551`, delegating to the
per-provider `record_exhaustion` at line 265) itself calls `self.record_failure(ErrorClass::Exhausted, now_ms)` —
i.e. it performs a full failure recording (incrementing `consecutive_failures`, `total_failures`, pushing a
`FailureRecord` onto `failure_window`, pushing `false` onto `recent_outcomes`) **in addition to** whatever the
bridge's own generic recording already did for the identical refusal in step 1. One real refused call therefore
contributes two entries to `total_failures`/`failure_window`/`recent_outcomes`.

## Why it matters

Goal: truth (honest provider health, so the circuit breaker and routing act on real failure rates, not inflated
ones). `ProviderHealth::record_failure`'s own rolling-window trip condition ("rolling success rate < 30% over the
last 10 requests", `provider_health.rs:198-237`) is sensitive to exactly this kind of double counting: one real
exhaustion event consumes two of the ten rolling-window slots, pushing a provider toward (or needlessly re-opening)
a circuit twice as fast as the real failure rate warrants. It also means `total_failures`/`time_weighted_error_rate`
over-report a provider's unreliability to anything that reads them (dashboards, `roko diagnose`, future audits).

## Where

- `crates/roko-agent/src/model_call_service.rs::provider_error_kind` (line 2148) and its caller around line 1991
  (`recorder.record_provider_failure`) — the bridge's generic classified-failure path.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs` (lines ~273-304) — the dedicated exhaustion handling that
  runs after the bridge call returns, calling `self.factory.health_registry.record_exhaustion` directly.
- `crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry::record_exhaustion` (line 551) and
  `ProviderHealth::record_exhaustion` (line 265), which both delegate to `record_failure`.
- Distinct from `gap-e00238`/task 1114 (done): that fix made `record_provider_outcome` and
  `record_agent_dispatch_feedback` (both inside `dispatch_v2.rs`, "the bridge") write exactly once between
  themselves. It did not touch `failover.rs`'s separate, direct call into `provider_failover`'s exhaustion handling,
  which is a third call site task 1114's anchors never named.

## Current state

Unfixed. Confirmed by reading the current call chain end to end: the bridge dispatch call in `failover.rs` (via
`run_shared_agent_bridge`/`run_shared_agent_bridge_with_config`) already goes through the bridge's own
classified-failure recording for a failed result; `failover.rs` then performs its own, separate
`record_exhaustion` call on the same registry for the same result.

## Plan

1. Decide which path owns exhaustion recording. Recommended: `failover.rs`'s dedicated `record_exhaustion` call
   already sets the correct until-ms quarantine (from the provider's own reported reset time, not just a generic
   cooldown default) — keep that one, and make the bridge's generic path skip recording when the result is already
   tagged/classified as exhaustion (mirroring how task 1114 made `record_provider_outcome` skip `immune_denied`
   results), since `failover.rs` will record it anyway.
2. Alternatively, have the bridge call `roko_learn::provider_failover::record_exhaustion` itself (instead of its own
   generic failure path) and have `failover.rs` stop re-detecting/re-recording when the bridge already did.
3. Test: one refused call classified as exhaustion increments `total_failures`/`failure_window` by exactly one, and
   the registry's final `cooldown_until` matches `failover.rs`'s computed `until_ms` (not an earlier, shorter
   generic-cooldown value that a first write might have set).

## Done when

- A fake provider call refused with an exhaustion message leaves the registry's `total_failures` incremented by
  exactly one and `failure_window` holding exactly one `FailureRecord` for that call.
- The `[[verify]]` command passes.

## Notes

- Related but distinct: `gap-e00238` (done, task 1114) fixed the bridge's OWN internal double-write
  (`record_provider_outcome` + `record_agent_dispatch_feedback`); this item is about a THIRD call site
  (`failover.rs`'s direct `record_exhaustion`) that task 1114's anchors never covered.
- Do not weaken `failover.rs`'s own quarantine `until_ms` computation — it is more accurate (provider-reported
  reset time) than the bridge's generic classified-failure path would produce on its own.

## Progress

- 2026-10-03: implemented on `work/gap-d90a93` at 5cff574dd; `ProviderHealth::record_exhaustion` only moves the quarantine end of a provider already held open for an exhaustion, which also covers the serve and ACP paths. Cargo verification deferred to the batch gate.
