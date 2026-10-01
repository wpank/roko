+++
id = "gap-28ceb9"
kind = "gap"
title = "dispatch_v2 has no provider failover and no usage-exhausted error class"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/dispatch_v2"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::classify_provider_error", "crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover"]
links = { depends_on = [], blocks = [], related = ["bug-35379d", "find-229e9c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'detect_provider_exhaustion' crates/roko-cli/src/dispatch_v2.rs && cargo test -p roko-cli --lib classify_provider_error_detects_usage_exhaustion"
+++

Graph plan runs switch to a fallback model when the planned provider is blocked or exhausted (`run_bridge_with_failover`). `dispatch_v2` (`AgentDispatcherV2`) has no failover, and its `classify_provider_error` knows only `insufficient_credits`, `rate_limit`, `timeout`, `server_error` and `unknown`, with no class for session or usage-limit exhaustion. Callers that dispatch through it, which the e5 audit identified as the serve chat and ACP paths, fail on an exhausted provider instead of moving to a configured fallback.

Fix: share the Graph path's exhaustion detection and failover policy, recording the substitution (see bug-35379d).

## Notes

- 2026-10-01 (wk-tiers): partial, on work/bug-7cdce7; cargo verification deferred to the batch check.
  - Done (the error class): `dispatch_v2::classify_provider_error` checks `detect_provider_exhaustion` first and
    returns `provider_exhausted`, which the registry records as `ErrorClass::Exhausted`. So a session or
    usage-limit refusal trips the breaker at once. Before, it counted as `unknown`, or as billing when it mentioned
    a quota. On the Graph path, `run_bridge_with_failover` still refines the quarantine to the reported reset.
    Test: `classify_provider_error_detects_usage_exhaustion`.
  - Left (the failover): serve chat and ACP still dispatch through `AgentDispatcherV2` with no fallback. Sharing
    the Graph policy means lifting `failover_candidates`/`failover_model` out of `GraphTaskDispatcher` into
    dispatch_v2 (or a shared module), then calling it from those entry points and recording the substitution
    (bug-35379d). Those entry points are in serve/ACP, outside this packet.
