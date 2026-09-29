+++
id = "bug-1948c9"
kind = "bug"
title = "Contract rules MaxToolCallsPerTurn and MaxConsecutiveFailures never fire: nothing records the actions they count"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-agent/src/safety/contract.rs::orchestrator_actions", "crates/roko-agent/src/dispatcher/mod.rs:567"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn max_tool_calls_per_turn_counts_dispatched_tools" crates/roko-agent && cargo test -p roko-agent max_tool_calls_per_turn_counts_dispatched_tools'
+++

`MaxToolCallsPerTurn` and `MaxConsecutiveFailures` (and the cost total at contract.rs:861) count `orchestrator_actions(ctx)`, which excludes external actions whose service is `roko.tool_history`. The only production writer of `ToolContext` external actions is the dispatcher's per-call tool history (dispatcher/mod.rs:567), which uses exactly that service; `with_external_actions` is called only in tests. These rules therefore always see zero actions: for example, the researcher contract's `MaxToolCallsPerTurn: 12` can never trigger. `RequireToolBeforeEdit` is unaffected because it reads the tool history directly.

Fix: record orchestrator actions where the orchestrator performs them, or count tool history in the rules that are meant to limit tool calls. Add a dispatcher-level test named `max_tool_calls_per_turn_counts_dispatched_tools`.

Re-checked 2026-09-29: still open. The unit test tool_history_satisfies_read_before_edit_without_counting_as_actions (contract.rs:1388) asserts that tool history does not count toward MaxToolCallsPerTurn(1), so a fix that counts tool history must change that test deliberately.
