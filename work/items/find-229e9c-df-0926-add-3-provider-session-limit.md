+++
id = "find-229e9c"
kind = "finding"
title = "Provider session-limit refusals retried as ordinary task failures"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-agent/provider"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop"
anchors = ["error_classify.rs", "claude_cli", "crates/roko-agent/src/provider/error_classify.rs::detect_provider_exhaustion", "crates/roko-cli/src/graph_task_dispatch.rs::run_bridge_with_failover"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Claude CLI 'session limit' exits (exit 1 in ~2s) were retried three times, then FailFast skipped dependents; refusal/limit errors should be classified and paused rather than consuming retries.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop`

How to verify: Check error classification for session/rate-limit messages.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): 725f21e05 classifies session and usage-limit refusals as provider exhaustion (roko-agent/src/provider/error_classify.rs:14-18, :62-66 detect_provider_exhaustion). On the Graph path, run_bridge_with_failover (graph_task_dispatch.rs:4223-4325) quarantines the provider until its reset via health_registry.record_exhaustion (:4292-4295) and fails over to the next candidate in the same attempt, so no retry is burned. What remains: when no other usable provider exists (or --model pins one), failover_model returns an error (:4324) and the attempt fails with what the doc calls a non-retryable error (:4220-4222). The run does not pause until the reset window, so FailFast still skips dependents.
