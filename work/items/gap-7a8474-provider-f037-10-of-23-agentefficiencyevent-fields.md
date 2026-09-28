+++
id = "gap-7a8474"
kind = "gap"
title = "10 of 23 AgentEfficiencyEvent fields always zero/empty in primary live path"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/efficiency"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037"
anchors = ["crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent", "crates/roko-cli/src/graph_task_dispatch.rs:1450"]
links = { depends_on = [], blocks = [], related = ["bug-f9ae3e"], supersedes = [], duplicate_of = "" }
+++
The `AgentEfficiencyEvent` struct has 23 fields. In the primary `EfficiencyEventWriter` path used by the runner, at least 10 fields are never populated and remain at their zero-value defaults: `cost_usd_without_cache`, `cache_write_tokens`, `reasoning_tokens`, `tool_call_count`, `tool_success_cou...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Confirm in crates/roko-learn/src/efficiency.rs, crates/roko-cli/src/runner/event_loop.rs whether still true: 10 of 23 `AgentEfficiencyEvent` fields always zero/empty in primary live path Runner-v2 event_loop.rs (the audited location) was deleted 2026-09-06; check whether the Graph path (graph_task_dispatch.rs / graph_execution/) has the same behavior.

Verified 2026-09-28: checked against graph_task_dispatch.rs, which has a large uncommitted diff from a concurrent session. The Graph primary-path event (:1450-1485) still hardcodes attempt_id "", reasoning_tokens 0, time_to_first_token_ms 0, gate_passed None, gate_errors [] and strategy_attempted "", and sets cost_usd_without_cache = cost_usd and tools_available = tool_calls.len(). Now populated: cache read/write tokens, prompt_sections, system_prompt_tokens, tools_used and tool_calls. The Runner-v2 anchor is gone. Severity p2 (telemetry quality).
