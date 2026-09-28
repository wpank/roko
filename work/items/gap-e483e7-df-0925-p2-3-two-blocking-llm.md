+++
id = "gap-e483e7"
kind = "gap"
title = "Two blocking LLM calls on every gate failure; cheap-agent timeout hardcoded"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
anchors = ["graph_task_dispatch.rs:2466 judge_quality", "graph_task_dispatch.rs:2665 enrich_error_digest", "CheapFactoryAgent", "crates/roko-cli/src/graph_task_dispatch.rs:2190", "crates/roko-cli/src/graph_task_dispatch.rs:2374", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
quality_judge and enrich_error_digest are awaited inline (~43s per failure, ~86 of 96s overhead) though neither gates the retry; CheapFactoryAgent hardcodes a 30s timeout ignoring timeouts.llm_call_secs.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure`

How to verify: Time retry latency after a gate failure.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed parts (committed at HEAD): judge_quality now runs in a detached tokio::spawn (graph_task_dispatch.rs:2190, call at :2197), so it no longer blocks the retry. CheapFactoryAgent's timeout now comes from timeouts.llm_call_secs, not a hardcoded 30s (cheap_agent :1296-1307, test :6420; 725f21e05). Remaining: enrich_error_digest is still awaited inline on every gate failure, bounded by timeouts.llm_call_secs (:2371-2380). The code keeps it inline on purpose because the diagnosis feeds the retry prompt, so one blocking LLM call per failure remains.
