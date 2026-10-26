+++
id = "gap-ece700"
kind = "gap"
title = "Telemetry gaps from round v7's run analysis: provider waits inside model_ms, tool time by kind, CPU time of tool calls and checks, a build's output head, how a try ended, untimed pre-open work, the pre-check and rung probes in the run manifest, the merge onto the person's branch, and plan-writing calls per operation"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "L"
subsystem = ["roko-cli/telemetry"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, sections 5 and 7), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/call_cost.rs::MeteredCalls", "crates/roko-agent/src/openai_compat_backend.rs::admit", "crates/roko-agent/src/tool_loop/mod.rs::open_stream", "crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs::ToolTiming", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::attempt_phases", "crates/roko-std/src/tool/builtin/bash.rs::execute", "crates/roko-cli/src/graph_task_dispatch/tui_forward.rs::forward_dispatch_events_to_tui", "crates/roko-learn/src/telemetry/records.rs::ExecutedModel", "crates/roko-learn/src/telemetry/records.rs::RunInvocation", "crates/roko-cli/src/graph_task_dispatch.rs::dispatch", "crates/roko-serve/src/routes/run_result.rs", "crates/roko-serve/src/operations.rs"]
links = { depends_on = [], blocks = [], related = ["bug-8bcd98", "gap-6a319c", "gap-5c3347", "gap-61c95c", "gap-143b5c", "gap-8b917d", "gap-618a75", "gap-1ba0ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw provider_wait_ms crates/roko-core/src/dashboard_snapshot.rs && grep -qw held_ms crates/roko-core/src/dashboard_snapshot.rs && grep -qw tools_by_kind crates/roko-core/src/dashboard_snapshot.rs && grep -qw sibling_wait_ms crates/roko-core/src/dashboard_snapshot.rs && grep -rqw cpu_ms crates/roko-std/src/tool/builtin && grep -rqw units_compiled crates/roko-cli/src/graph_task_dispatch && grep -qw send_backs crates/roko-learn/src/telemetry/records.rs && grep -qw pre_check crates/roko-learn/src/telemetry/records.rs && grep -qw rung_probes crates/roko-learn/src/telemetry/records.rs && grep -rqw delivery_merged crates/roko-core/src && grep -qw rejected_by crates/roko-serve/src/operations.rs"
+++

## Problem

The analysis of round v7's Undertow runs (481 run-minutes, 76 tries, 1,048 model calls, 1,864 tool calls) had to
rebuild by hand, or couldn't rebuild at all:
- where a call's time went;
- whether a slow tool call was waiting or working;
- how a try ended;
- a run's pre-check, probes and merge.

The records support most splits: phases per try, tool spans from the event index, checks, merges, plan checks,
delivery. These are the ones they don't. Each row names the file where the field belongs.

| # | What can't be measured today | Where it would be recorded | Field to add |
|---|---|---|---|
| 1 | A call's own duration and TTFT. `call_cost` has only its end time and token counts. `Usage` carries `wall_ms`, and the loop measures `turn_duration_ms`, but neither reaches the row. | `graph_task_dispatch/call_cost.rs::MeteredCalls`; `roko-core/src/dashboard_snapshot.rs` (`DashboardEvent::CallCost`); set `usage.wall_ms` from `turn_duration_ms` in `tool_loop/mod.rs::run_inner` | `call_cost.wall_ms`, `call_cost.ttft_ms` |
| 2 | A call's wait for a provider slot or a 429 backoff. The learned-limit hold (`openai_compat_backend.rs::admit`) logs no duration: 16 holds in phases 2.1 and 2.2. 429 retries (`tool_loop/mod.rs::open_stream`) reach only the tracing log: 16 retries, 24 s. Both sit inside `model_ms`. | those two sites, then `graph_task_dispatch/attempt.rs::attempt_phases` | per call `held_ms`, `retries`, `retry_wait_ms`; per try `timing.phases.provider_wait_ms`, taken out of `model_ms` |
| 3 | Build, test and configure time inside a turn. `tools_by_name` keeps the top 5 tool names, and `bash` mixes builds, tests and `ls`. `run_tests` returns each check's `secs`, and the verdict drops them. | `graph_task_dispatch/live_tool_calls.rs` (`ToolTiming`); `attempt.rs::attempt_phases`; `dashboard_snapshot.rs` (`AttemptPhases`) | `timing.phases.tools_by_kind` {build, test, configure, run_tests, git, read, write, other}, classing bash by the command after `cd … &&`; `timing.phases.self_checks` {count, ms, last_passed} |
| 4 | Waiting against working inside a tool call or check step. A build queued on a slot or lock looks like one compiling. Phase 4's three 200–214 s first builds and 6059db44's 15-min `run_tests` can't be explained. | `roko-std/src/tool/builtin/bash.rs` and `child_group.rs`; `task_checks.rs` and `run_tests.rs`; `graph_task_dispatch/verification.rs::run_verify_steps`; `spec_red_on_base.rs` | `cpu_ms` (the child's user plus system time from rusage) beside each call's and step's wall `ms` |
| 5 | What a build or test did. Output over 2,048 bytes keeps only its last 1,024 (`graph_task_dispatch/tui_forward.rs::forward_dispatch_events_to_tui`), so a build's head is lost: the units compiled, heavy.sh's slot line. | `tui_forward.rs`, on the `tool_result` stream event | a summary parsed before the cut: `exit_code`, `units_compiled`, `tests` {cases, assertions, failed} |
| 6 | How a try ended: whether the agent stopped with its own last check red, green or never run, and how many send-backs (bug-cbbaa8) it got. 17 of the round's 28 agent-blamed failures were "stopped, own test red". Telling them apart took parsing tool output. | `graph_task_dispatch/attempt.rs` (settle), counting at bug-cbbaa8's send-back site; `roko-learn/src/telemetry/records.rs::ExecutedModel` | `executed.self_check` (red, green, none), `executed.self_checks`, `executed.send_backs` |
| 7 | Work between the workspace and the attempt's open: `restore_best_try`, clearing a stale index lock, and the sibling-settle wait (gap-1920ba). It is untimed, and falls into gaps. | `roko-cli/src/graph_task_dispatch.rs::dispatch` (about lines 1318–1347) | `timing.phases.restore_ms`, `timing.phases.sibling_wait_ms` |
| 8 | The pre-check and the rung probes in the run manifest. The invocation's `started_at` is stamped after the pre-check, so manifest times miss 0.2–134 s. Phases 0–1 left no pre-check record at all: phase 1's 2 min 56 s is only in the pilot's log. Probe results live only in a 24 h cache. | `graph_execution/plan_runner.rs::run_graph_plan_body`; `roko-learn/src/telemetry/records.rs` (`RunInvocation`), written through `manifest.rs::begin_invocation` | `invocations[].pre_check` {started_at, elapsed_ms, steps_run, cached}; `invocations[].rung_probes` [{rung, model, ok, error_class, ms}] |
| 9 | The merge onto the person's branch. Delivery ends at `roko/batch/<run>`. Undertow main was fast-forwarded in the plugin's dialog, with no record and no promotion tag. | `roko-serve/src/routes/run_result.rs` (it already proposes `merge_command`), or the route a merge goes through | a `delivery_merged` event {target, commit, at, fast_forward} |
| 10 | Plan-writing calls per operation. The planner rows in `learn/costs.jsonl` carry no operation id, no draft number and no outcome, and all say `cost_source: unknown`. Matching 32 planner calls to 17 writes took slugs, timestamps and the rejected drafts' times. | `roko-serve/src/operations.rs` (the operation's progress), from the plan_generate pipeline | `progress.calls[]` {kind (draft, regeneration or coverage), rejected_by, usd, ms, model} |

## Why it matters

Visibility. Speed work needs these splits.
- The round's biggest unknowns were exactly here: whether phase 4's slow builds were queueing or compiling, how
  much of `model_ms` was waiting, and why a write took 3 planner calls.
- Each field is small and local, and most data is already in hand at the site named.

## Where

The anchors and the table above.

## Current state

- gap-6a319c (per-try phases), gap-5c3347 (`call_cost`) and gap-61c95c (plan-writing phases) made most of the
  round's analysis possible.
- This item is the remainder.
- Not here:
  - `reasoning_tokens` and `finish_reason` on `call_cost`, which roko-effort takes with gap-1ba0ac;
  - failed calls' `call_cost` rows and the verdict's refused spend, in bug-8bcd98;
  - the distiller's cost in the plan-writing operation (gap-143b5c).

## Plan

Each row is independent and can be its own commit. Suggested order, by how much each one explains:
- 2 and 4 together, because they explain `model_ms` and the slow tools;
- then 1, 3 and 6;
- then 8 and 10;
- then 5, 7 and 9.

Each field gets a test at its site, and the run summary or serve reads it where a reader exists.

## Done when

- Every field in the table exists, is written at its site, and has a test.
- The `[[verify]]` static check, which finds each field's name in its file, passes.

## Notes

- Evidence and method: `tmp/hermes-roko-v7/findings/run-analyst.md`, sections 1 ("Which splits the records
  support"), 5 and 7.
- The scripts are in `tmp/hermes-roko-v7/run-analyst/`.
