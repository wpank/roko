+++
id = "gap-e483e7"
kind = "gap"
title = "DF-0925 P2-3: Two blocking LLM calls on every gate failure; cheap-agent timeout hardcoded"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
anchors = ["graph_task_dispatch.rs:2466 judge_quality", "graph_task_dispatch.rs:2665 enrich_error_digest", "CheapFactoryAgent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
quality_judge and enrich_error_digest are awaited inline (~43s per failure, ~86 of 96s overhead) though neither gates the retry; CheapFactoryAgent hardcodes a 30s timeout ignoring timeouts.llm_call_secs.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure`

How to verify: Time retry latency after a gate failure.
