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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ea5fbe4b2"
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

## Notes

- Premise held at ea5fbe4b2: the two rules counted `orchestrator_actions`, which excludes the `roko.tool_history` service, the only production writer of `ToolContext` external actions, and that history recorded only successful calls.
- Decided: count tool history, per model turn. Through `contract::begin_tool_turn` and `contract::record_tool_result`, the dispatcher records a turn marker and the calls admitted in it, in the model's order and before any runs, at each `dispatch_batch` (one model turn) and each single `dispatch` (a turn of its own), and every call's result, failures included. `MaxToolCallsPerTurn` counts a call by its place in its turn, so parallel calls cannot slip past the limit; `MaxConsecutiveFailures` counts trailing failed results (with orchestrator actions); `RequireToolBeforeEdit` now needs a call that ran and succeeded, not one only admitted or failed; cost and gate rules still see no tool history.
- `tool_history_satisfies_read_before_edit_without_counting_as_actions` keeps its assertion, now deliberately: earlier results are no calls of the turn an edit starts. Its entries take the recorded shape.
- Effect: the bundled limits now apply on roko's dispatcher (Claude CLI agents run their own tools). Calls past a role's `MaxToolCallsPerTurn` in one model turn (implementer 8, auto-fixer 6, reviewer 4) get a contract error, rewritten by the role's recovery action; the run goes on. The auto-fixer's calls are refused after three failed calls in a row, a `bash` command exiting non-zero being a failure. The restricted fallback contract allows no tools, so its limits change nothing.
- `dispatcher/mod.rs` gets three small hunks (`dispatch`, `dispatch_batch`, the history block), kept minimal for find-f489db; the test lives in `contract.rs`.
- Tests: `max_tool_calls_per_turn_counts_dispatched_tools` (this item's verify): three parallel calls under a limit of two refuse the third by order, a new turn counts afresh, and after two failed calls the next is refused. Also `read_before_edit_needs_a_successful_read`.
- Checked without cargo: static review and nightly fmt only; roko-agent was not compiled.
- Implemented on `work/find-8416f2` at `4165f6485`; cargo verification deferred to the batch check.
