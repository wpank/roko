+++
id = "gap-7a8474"
kind = "gap"
title = "10 of 23 AgentEfficiencyEvent fields always zero/empty in primary live path"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "learning"
subsystem = ["roko-learn/efficiency"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a08f9e2c"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback", "crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent", "crates/roko-core/src/chat_types.rs::Usage::fill_cost_from_pricing", "crates/roko-cli/src/dispatch_v2.rs::dispatch_events_from_result", "crates/roko-agent/src/safety/contract.rs::AgentContract", "crates/roko-agent/src/agent.rs::AgentResult", "crates/roko-agent/src/immune_boundary.rs::is_model_output"]
links = { depends_on = [], blocks = [], related = ["bug-f9ae3e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/async fn emit_feedback/,/^    }/p' crates/roko-cli/src/graph_task_dispatch/feedback.rs | grep -Eq 'reasoning_tokens: 0,|time_to_first_token_ms: 0,|cost_usd_without_cache: cost_usd,|tools_available: eff_tool_calls.len'"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T07:43:01Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-02T01:20:13Z"
forced = false
evidence = "the per-dispatch efficiency event fills reasoning tokens, time to first token (measured at the immune boundary), cache-free cost and tools available (wk-settle f08f64380 plus the batch's channel type fix f0d640b9e); gate 6h2 passed at 285282248 (cargo check, clippy -D warnings, 11,366 lib tests in roko-agent/cli/core/fs/gate/graph/learn/serve, canaries C1-C8 plus integration tests, 446 roko-cli bin tests, run_evidence py, portal tsc and 809 vitest); merged in 6a08f9e2c"
+++

## Problem

Every Graph plan dispatch appends one `AgentEfficiencyEvent` row to `.roko/learn/efficiency.jsonl`. The row is
built in `graph_task_dispatch.rs::emit_feedback` (the "W05: Efficiency event" block, lines ~1591-1632 at HEAD).
Five of its fields are still hardcoded instead of measured:

| Field | Written as | Should be |
|---|---|---|
| `reasoning_tokens` | `0` | thinking/reasoning tokens the provider reported |
| `cost_usd_without_cache` | `cost_usd` (a copy) | what the turn would have cost with no prompt caching |
| `tools_available` | `eff_tool_calls.len()` (the number of calls made) | the number of tools offered to the agent |
| `time_to_first_token_ms` | `0` | ms from dispatch start to the first streamed output |
| `strategy_attempted` | `""` | the retry/replan strategy of this attempt, if any |

Observable effects: `cache_savings_usd()` (`cost_usd_without_cache - cost_usd`) is always 0 on Graph rows;
`tool_utilization()` (`tools_used / tools_available`) is always 1.0 whenever any tool ran; reasoning spend
and latency are invisible in `roko learn efficiency` and in serve's projections of these rows.

## Why it matters

- Goal `learning` (learning loops on the Graph path). Efficiency rows are the raw data for cost and latency
  analysis. Rows that look valid but carry fabricated values are worse than missing ones: they pass schema
  checks and skew aggregates silently.
- `cost_usd_without_cache` feeds cache-savings reporting; `tools_available` feeds tool-utilization; TTFT is the
  first input of `LatencyRegistry::record` (`crates/roko-learn/src/runtime_feedback/routing.rs:107-125`).
- Related: `bug-f9ae3e` (same block: tool calls over-counted from delta events and all marked succeeded; its
  notes also flag `tools_available`). Coordinate so only one change edits `tools_available`.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::emit_feedback` (~line 1458): post-dispatch feedback for Graph
  tasks. Reached from `roko plan run <dir>` via the Graph engine's per-task dispatch. The per-dispatch event
  literal is at ~1591-1632.
- Same file, ~2433 (gate-fail event) and ~2706 (gate-pass event): outcome-only rows with zero tokens and
  zero cost. Their zeros are consistent (no usage on those rows) and need no change.
- `crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent` (line 92): the struct; `tool_utilization()` and
  `cache_savings_usd()` at ~266-276.
- `crates/roko-core/src/chat_types.rs::Usage` (line 125): `dispatch.result.usage`; has `reasoning_tokens: u32`
  and `fill_cost_from_pricing` (line 195), the canonical cost formula
  (`input*in + output*out + cache_read*cache_r + cache_create*cache_w`; input excludes cached tokens).
- `crates/roko-cli/src/dispatch_v2.rs::fill_cost_from_profile` (~2093): pricing sources, the target's
  `model_profile` (`cost_input_per_m`, `cost_output_per_m`, ...) then
  `roko_core::config::model_registry::builtin_pricing(slug)`.
- `crates/roko-cli/src/dispatch_v2.rs::dispatch_events_from_result` (~2116): synthesises a
  `DispatchEvent::TokenUsage` with `reasoning_tokens: 0` even though `result.usage.reasoning_tokens` exists.
- `crates/roko-agent/src/safety/contract.rs::AgentContract::allowed_tools` (line 153): optional tool
  allowlist; `effective_agent_contract(role, &task)` builds the contract in graph dispatch (~3625, ~4271).

## Current state

- Fixed since the audit: `attempt_id` is populated (`3d0637232`); cache read/write tokens, `prompt_sections`,
  `system_prompt_tokens`, `tools_used` and `tool_calls` are populated; gate outcome fields live on separate
  gate-fail/gate-pass rows. The audited Runner-v2 `event_loop.rs` was deleted on 2026-09-06.
- Remaining: the five fields in the table above.
- Data sources that exist today:
  - Reasoning: `dispatch.result.usage.reasoning_tokens`; `dispatch.result.usage_obs.reasoning_tokens` (OpenAI
    compat parses `completion_tokens_details.reasoning_tokens`, `crates/roko-agent/src/translate/openai.rs:322`);
    Codex CLI streams `AgentRuntimeEvent::TokenUsage { reasoning_tokens }`. The Claude CLI adapter reports 0.
  - Cost without cache: pricing per model via the two sources above. roko-acp already computes it with its own
    helper `calculate_cost_without_cache_for_model_slug` (`crates/roko-acp/src/bridge_events/cost.rs:65`, crate-private).
  - TTFT: not captured anywhere reachable from `emit_feedback`. `dispatch.events` carry no timestamps. The
    OpenAI-compat backend measures TTFT (`crates/roko-agent/src/openai_compat_backend.rs:709`) but only into a
    metrics registry. The live-output sink (`request.live_output`) is attached only when a TUI bridge is set.
  - Tools offered: `AgentContract.allowed_tools` when set; otherwise the provider's own default tool set,
    which roko does not know for CLI providers.
- Side note: the Graph path writes the row directly with `append_jsonl_line_async`; it does not call
  `LearningRuntime::append_efficiency_event`, so the latency registry is not fed from Graph runs at all
  (only `bench.rs:722` calls it). Out of scope here; file separately if it matters.

## Plan

1. Reasoning tokens: take `u64::from(dispatch.result.usage.reasoning_tokens)`; if 0, use
   `dispatch.result.usage_obs.as_ref().and_then(|o| o.reasoning_tokens)`; if still 0, sum
   `AgentRuntimeEvent::TokenUsage { reasoning_tokens, .. }` over `dispatch.events`. Also make
   `dispatch_events_from_result` forward `result.usage.reasoning_tokens` instead of 0.
2. Cost without cache: add a shared helper next to `Usage::fill_cost_from_pricing`, e.g.
   `Usage::cost_without_cache(input_per_m, output_per_m) -> Option<f64>` =
   `(input + cache_read + cache_create) * input_per_m + output * output_per_m`. In `emit_feedback`, resolve
   pricing the same way as `fill_cost_from_profile` (profile first, then `builtin_pricing`). When no pricing is
   known, fall back to `cost_usd` (savings 0) and say so in the field's doc comment. Keep `cost_usd` as-is: the
   Claude CLI reports it natively and it is authoritative. Optionally switch roko-acp to the shared helper.
3. Tools available: use `effective_agent_contract(role, task).allowed_tools.map(|t| t.len())`, else 0.
   Document on the struct that 0 means "unknown" (then `tool_utilization()` returns 0 instead of a fake 1.0).
4. TTFT. Options:
   - A (recommended): add `ttft_ms: Option<u64>` to what the adapters return (e.g. on `UsageObservation` or
     `AgentResult`), set it in the streaming adapters that already see the first delta (OpenAI-compat at
     `openai_compat_backend.rs:709`, Claude CLI and Codex CLI stream parsers), and write `unwrap_or(0)`.
     Real data for the main providers; touches roko-agent.
   - B: always attach a private live-output sink in graph dispatch and record the first event's `Instant`.
     Stays in roko-cli, but changes dispatch behaviour even when no TUI is attached, and its timing includes
     screening delays.
   - C: leave it unmeasured but explicit (`Option<u64>` on the struct). This touches every struct literal in
     the workspace; not recommended.
5. Strategy: the Graph engine retries a failed task through `max_retries`; the gate-fail row writes
   `"replan"` when `self.feedback.replan_on_gate_failure` is set (~2411-2466). For the per-dispatch row, derive a
   label from the attempt counter behind `next_attempt_id` (`<task>/a0` is the first attempt): `"initial"` for
   a0, `"retry"` after a failed attempt, `"replan"` when replan-on-gate-failure is on. Whether a richer strategy
   name is available at the call site is unknown; check before inventing labels.
6. Add a unit test in `graph_task_dispatch.rs` (for example `graph_efficiency_event_populates_usage_fields`)
   that feeds an `AgentResultDispatch` with reasoning tokens, cache tokens and a priced model, and asserts the
   row's `reasoning_tokens`, `cost_usd_without_cache > cost_usd`, and `tools_available`.

## Done when

- A Graph dispatch with cache reads on a priced model writes `cost_usd_without_cache > cost_usd`.
- A Codex or OpenAI-compat reasoning run writes non-zero `reasoning_tokens`.
- `tools_available` is the offered-tool count or 0 (unknown), never the call count.
- `time_to_first_token_ms` is measured for at least one streaming provider (or the chosen option's
  equivalent), and `strategy_attempted` is set on retries.
- The verify command passes:
  `! sed -n '/async fn emit_feedback/,/^    }/p' crates/roko-cli/src/graph_task_dispatch.rs | grep -Eq 'reasoning_tokens: 0,|time_to_first_token_ms: 0,|cost_usd_without_cache: cost_usd,|tools_available: eff_tool_calls.len'`

## Notes

- Telemetry only: no auth, persistence format or migration risk. The struct has a manual `Serialize` impl with
  a field list that must stay in sync (`efficiency_event_serialization_roundtrip` test); adding a field to
  `AgentEfficiencyEvent` means updating that impl and every struct literal, so prefer not to.
- Not parallel-safe with `bug-f9ae3e` (same lines of `emit_feedback`). Safe alongside other work.
- Option A of step 4 touches the provider adapters in roko-agent; if that grows, split TTFT into its own item
  and land steps 1-3 and 5 first.

- 2026-10-01 (wk-settle): PARTIAL on work/bug-f9ae3e; cargo verification deferred to the batch check. Plan steps
  1, 2, 3 and 5 landed in `graph_task_dispatch/feedback.rs::emit_feedback`:
  - `reasoning_tokens` comes from the usage, else its observation, else the streamed `TokenUsage` events
    (`reported_reasoning_tokens`), and `dispatch_events_from_result` and the streaming path forward the result's
    reasoning tokens instead of 0.
  - `cost_usd_without_cache` uses `Usage::cost_without_cache` priced like `fill_usage_cost_from_pricing` (profile,
    then built-in pricing; `dispatch_v2::usage_cost_without_cache`), never below `cost_usd`.
  - `tools_available` is the contract allowlist's length, or 0 (unknown).
  - `strategy_attempted` is `initial`, `retry`, or `replan` with `replan_on_gate_failure`.
  - Test: `graph_efficiency_event_populates_usage_fields`. The verify command's `sed` now reads `feedback.rs`.
- Left: step 4, `time_to_first_token_ms`. Nothing reachable from `emit_feedback` timestamps the first output, and
  option A needs the provider adapters in roko-agent to report it. The verify command fails on that pattern until
  it lands.
- 2026-10-02 (wk-settle): step 4 implemented on work/bug-f9ae3e (option A); cargo verification deferred to the
  batch check. `AgentResult` gains `ttft_ms`: ms from the call's start to its first streamed model output. The
  provider boundary every provider's stream passes through (`ImmuneScreenedAgent::drive_streaming_inner`, for the
  API tool loop, the Claude CLI and the Codex CLI alike) times the first text, reasoning or tool-call event, not a
  usage update, unless the provider set its own. Since bug-3a3b0f every Graph attempt streams through it, and
  `emit_feedback` writes `dispatch.result.ttft_ms.unwrap_or(0)` (0 = unknown). The verify command passes.
  Tests: `streamed_first_output_sets_time_to_first_token` (roko-agent),
  `cli_attempt_records_its_time_to_first_token` (fake Claude CLI that answers after 100 ms, end to end).
- The S01 verdict's `first_token_at` stays unset (`ttft_source = "unavailable"`): the boundary measures from its own
  call start, not as an epoch time, so it can't be placed on the attempt's clock without a further field.

## Original notes

The `AgentEfficiencyEvent` struct has 23 fields. In the primary `EfficiencyEventWriter` path used by the runner, at least 10 fields are never populated and remain at their zero-value defaults: `cost_usd_without_cache`, `cache_write_tokens`, `reasoning_tokens`, `tool_call_count`, `tool_success_cou...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F037`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Confirm in crates/roko-learn/src/efficiency.rs, crates/roko-cli/src/runner/event_loop.rs whether still true: 10 of 23 `AgentEfficiencyEvent` fields always zero/empty in primary live path Runner-v2 event_loop.rs (the audited location) was deleted 2026-09-06; check whether the Graph path (graph_task_dispatch.rs / graph_execution/) has the same behavior.

Verified 2026-09-28: checked against graph_task_dispatch.rs, which has a large uncommitted diff from a concurrent session. The Graph primary-path event (:1450-1485) still hardcodes attempt_id "", reasoning_tokens 0, time_to_first_token_ms 0, gate_passed None, gate_errors [] and strategy_attempted "", and sets cost_usd_without_cache = cost_usd and tools_available = tool_calls.len(). Now populated: cache read/write tokens, prompt_sections, system_prompt_tokens, tools_used and tool_calls. The Runner-v2 anchor is gone. Severity p2 (telemetry quality).

Rechecked 2026-09-29: attempt_id is now populated (3d0637232), and gate outcome fields are written on separate gate-fail/gate-pass efficiency events (graph_task_dispatch.rs:2436, :2709). Remaining on the per-dispatch event in graph_task_dispatch.rs::emit_feedback: reasoning_tokens 0, time_to_first_token_ms 0, cost_usd_without_cache copied from cost_usd, tools_available set to the number of tool calls rather than the tools offered, and empty strategy_attempted; the gate events also hardcode reasoning_tokens 0, time_to_first_token_ms 0 and cost_usd_without_cache 0.0.
