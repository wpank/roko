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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::classify_provider_error", "crates/roko-cli/src/graph_task_dispatch.rs::run_bridge_with_failover"]
links = { depends_on = [], blocks = [], related = ["bug-35379d", "find-229e9c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'detect_provider_exhaustion' crates/roko-cli/src/dispatch_v2.rs && cargo test -p roko-cli --lib classify_provider_error_detects_usage_exhaustion"
+++

Graph plan runs switch to a fallback model when the planned provider is blocked or exhausted (`run_bridge_with_failover`). `dispatch_v2` (`AgentDispatcherV2`) has no failover, and its `classify_provider_error` knows only `insufficient_credits`, `rate_limit`, `timeout`, `server_error` and `unknown`, with no class for session or usage-limit exhaustion. Callers that dispatch through it, which the e5 audit identified as the serve chat and ACP paths, fail on an exhausted provider instead of moving to a configured fallback.

Fix: share the Graph path's exhaustion detection and failover policy, recording the substitution (see bug-35379d).
