+++
id = "gap-e32873"
kind = "gap"
title = "Provider-owned internal tool calls bypass per-call taint, corrigibility and immune screening"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#agentcontract-and-strict-e34-enforcement-are-live-broader-security-coverage-remains-partial"
anchors = ["crates/roko-agent/src/safety", "crates/roko-core/src/corrigibility.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

E34 is accepted 8/8. Host-visible tool results, provider primary outputs and structured tools pass through taint tracking, corrigibility and the immune Graph. Tool calls and results that happen inside an opaque provider (for example Claude CLI or Codex running their own tools) are still invisible. They get no per-result taint, no per-call corrigibility decision and no immune screening, and provider trace Signals are not ingested. GAPS.md recorded this as a residual outside strict E34 acceptance.

Fix: ingest provider trace and tool events wherever the provider exposes them (stream-json, hooks, ACP tool notifications) and run them through the same taint, corrigibility and immune chain. For providers that expose nothing, fail closed or flag the run.
