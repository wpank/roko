+++
id = "find-f489db"
kind = "finding"
title = "Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "visibility"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-21
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "df746d76c"
source = "tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
discovered_from = "audit:tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:868", "crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::with_tool_audit", "crates/roko-cli/src/dispatch_v2.rs::AgentDispatcherV2::with_tool_audit", "crates/roko-fs/src/tool_audit.rs::ScrubAuditAdapter", "crates/roko-fs/src/observability.rs::FsObservabilitySinks", "crates/roko-agent/src/tool_loop/context_factory.rs::ToolExecutionContextFactory::new", "crates/roko-agent/src/dispatcher/mod.rs::emit_terminal_audit", "crates/roko-agent/src/provider/mod.rs::AgentOptions"]
links = { depends_on = [], blocks = [], related = ["bug-7debad"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'ToolAuditLog::open|ScrubAuditAdapter::new' crates/roko-cli/src/graph_execution && grep -rqE 'trace_sink_dyn|JsonlTraceSink::new' crates/roko-cli/src && grep -rqw 'fn graph_run_writes_tool_audit_admit_and_result' crates/roko-cli/ && cargo test -p roko-cli graph_run_writes_tool_audit_admit_and_result"
+++

## Problem

Tool calls in real runs leave no durable per-call record, and the per-call trace and metrics sinks are no-ops. Two of the original three findings are still open:

- **TD-003, durable tool audit log not written.** `ToolAuditLog` (`crates/roko-fs/src/tool_audit.rs`) writes paired `admit`/`result` records to `.roko/tool_audit.jsonl`, and `ScrubAuditAdapter` scrubs secrets and truncates arguments before writing. The plumbing from the CLI down to the dispatcher exists (see Where), but production code never constructs a `ToolAuditLog` or `ScrubAuditAdapter`, and nobody calls `SharedAgentFactory::with_tool_audit` or `AgentDispatcherV2::with_tool_audit`. Only roko-fs unit tests and `crates/roko-cli/tests/tool_audit_evidence.rs` write the file. So after `roko plan run <plan>` with an API-backed provider that calls tools, `.roko/tool_audit.jsonl` is absent or unchanged, and `roko learn tools` (which reads that file, `commands/learn.rs:1966`) has nothing to show.
- **TD-005, trace and metrics sinks are no-ops in production.** `ToolDispatcher::emit_terminal_audit` now calls `ctx.trace_sink.append(...)` and `ctx.metrics_sink.record(...)` (`crates/roko-agent/src/dispatcher/mod.rs:1130`, `:1141`). But every production `ToolContext` is built by `ToolExecutionContextFactory`, which defaults to `NoopAuditSink`, `NoopTraceSink` and `NoopMetricsSink` (`tool_loop/context_factory.rs:74-76`), and no provider overrides them. The values sent are also placeholders, so a real sink would record wrong data:
  - `HandlerFinished { exit_ms: timeout_ms }` records the configured timeout, not the elapsed time;
  - metrics are always `ToolMetrics::empty()`, with the role hard-coded to `AgentRole::Implementer` and the format to `ToolFormat::OpenAiJson`.

Expected: every tool call dispatched by roko's own tool loop in a plan run produces a scrubbed admit record and a terminal result record in `.roko/tool_audit.jsonl`, carrying the task/run correlation IDs. A trace event and a metrics record with real duration and success go to the JSONL sinks in `.roko/traces/` and `.roko/metrics/tool_metrics.jsonl`.

## Why it matters

- Goal `visibility`: runs must be observable. Without these records nobody can reconstruct which tool calls were admitted, how long they took, or what came back. `roko learn tools` and any latency or failure dashboard have no data source.
- Incident review cannot prove which call produced which result. This also affects safety review, since denials and results are not durably recorded per call.
- Related:
  - `bug-7debad`: the tool-audit umbrella bug. Its remaining unique scope is TD-006 (parallel result preview misattribution). It points TD-003 at this item.
  - `gap-ff95f5`: persistent taint/witness/custody provenance. Also a dispatcher side channel; coordinate so there is one terminal observation, not two parallel logs.

## Where

- `crates/roko-fs/src/tool_audit.rs`:
  - `ToolAuditLog::open(root)` (:123) writes `<root>/.roko/tool_audit.jsonl` (`DEFAULT_AUDIT_PATH`, :32) under a file lock;
  - `ScrubAuditAdapter::new(log, scrubber)` (:268) exposes `record_admit` and `record_result`.
- `crates/roko-fs/src/observability.rs`:
  - `FsObservabilitySinks::for_workdir` (:51) and `trace_sink_dyn()`/`metrics_sink_dyn()` (:127, :133) build the ready-made `JsonlTraceSink` (`.roko/traces/`) and `JsonlMetricsSink` (`.roko/metrics/tool_metrics.jsonl`);
  - `RunScrubber::build` (:196) builds a `LogScrubber` for a run.
  - Today only `bootstrap_observability_dirs` in `crates/roko-cli/src/main.rs:4415` uses it, and only to create the directories.
- Audit plumbing, top to bottom (all present, never fed):
  - `crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::with_tool_audit` (:278) passes it into `AgentDispatcherV2::with_tool_audit` (:484, :535, :608);
  - `crates/roko-cli/src/dispatch_v2.rs::AgentDispatcherV2::with_tool_audit` (:1519) passes it into `AgentOptions.tool_audit` (:1870);
  - `crates/roko-agent/src/provider/mod.rs` has `AgentOptions.tool_audit` (:872) and `build_tool_dispatcher_with_audit` (:459). The latter is called by the Anthropic API, OpenAI-compatible, Gemini, Cerebras and Perplexity tool loops;
  - `crates/roko-agent/src/dispatcher/mod.rs::ToolDispatcher::dispatch` (:531) calls `file_audit.record_admit` before execution and `record_result` after (:554-580).
- Trace/metrics plumbing:
  - `crates/roko-agent/src/tool_loop/agent_wrapper.rs::ToolLoopAgent::with_trace_sink`/`with_metrics_sink`/`with_audit_sink` (:160-180) feed the factory (:196-201). No provider builder calls them; see for example `provider/anthropic_api/tool_loop.rs:71-88`;
  - `AgentOptions` has no trace or metrics field.
- The Graph entry point to wire: `crates/roko-cli/src/graph_execution/plan_runner.rs:868`, where `roko plan run` builds its `SharedAgentFactory` (`.with_health_registry(...)`, `.with_error_patterns_from_disk(workdir)`) and never calls `.with_tool_audit(...)`.

## Current state

- TD-004 (per-tool timeout) is fixed. `dispatch_unfinalized` clamps the deadline with `ToolDef::timeout_ms` (dispatcher/mod.rs:663-672, "T029"). It arrived in batch commit `72e0a76b8`, the same commit that added the trace/metrics calls in `emit_terminal_audit`.
- TD-003 is still open. `grep -rnE 'ToolAuditLog::open|ScrubAuditAdapter::new' crates/roko-cli/src crates/roko-serve/src crates/roko-execution/src` finds nothing.
- TD-005 is half done. The dispatcher calls the sinks, but production sinks are no-ops and the values are placeholders.
- `crates/roko-cli/src/runner/types.rs:2470` still has an `obs_sinks: Option<FsObservabilitySinks>` field from the deleted Runner-v2 loop, with no users. Do not build on it.
- CLI-backed providers (`ClaudeCli`, `CodexCli`) run their own tool loops in a subprocess, and `claude_cli.rs:421` sets `tool_audit: None`. Their tool calls never reach `ToolDispatcher`, so this item covers roko's own tool loop only.

## Plan

1. TD-003, audit wiring:
   - In `plan_runner.rs` near :868, open `ToolAuditLog::open(workdir).await?` and build `ScrubAuditAdapter::new(Arc::new(log), RunScrubber::build(&[]))`, or a scrubber seeded with the run's configured secrets if they are available there;
   - chain `.with_tool_audit(Arc::new(adapter))` on the `SharedAgentFactory`;
   - decide what happens when the log cannot be opened. Recommendation: warn and continue, because the audit is observability, not a safety gate.
   - Do the same wherever `roko run`/`roko do` and `roko serve` build an `AgentDispatcherV2` or `SharedAgentFactory`, if it is cheap. `dispatch/mod.rs:440` (`spawn_agent_result_bridge`) is one such site.
2. Correlation: check that admit/result records carry task identity. `ToolCall` and `ToolContext.correlation` (`CorrelationEnvelope`) hold the agent/run IDs. If `ScrubAuditAdapter::record_admit` does not write them, add fields from `ctx.correlation` (run id, task id, attempt) to the record, bounded, with no raw arguments.
3. TD-005, sinks:
   - add `trace_sink: Option<Arc<dyn TraceSink>>` and `metrics_sink: Option<Arc<dyn MetricsSink>>` to `AgentOptions`, next to `tool_audit`;
   - in each provider builder that creates a `ToolLoopAgent` (`anthropic_api/tool_loop.rs:71`, `openai_compat.rs:522`, `gemini/adapter.rs:81`/`:125`, `cerebras.rs:89`), call `.with_trace_sink`/`.with_metrics_sink` when they are set;
   - thread them from `AgentDispatcherV2`, the same way `tool_audit` is threaded, and create them in `plan_runner.rs` from `FsObservabilitySinks::for_workdir(workdir)` (`trace_sink_dyn()`, `metrics_sink_dyn()`).
4. Fix the placeholder values in `emit_terminal_audit`:
   - measure the elapsed handler time and use it for `exit_ms`, keeping the timeout in the audit details if needed;
   - fill `ToolMetrics` with duration, success/failure and bytes out;
   - take the role and format from the context or call instead of hard-coding `Implementer`/`OpenAiJson`. If the context has no role, add one or record "unknown".
5. Flush: call `FsObservabilitySinks::flush_traces()` at run end, if the JSONL trace sink buffers.
6. Tests:
   - a roko-cli test, e.g. `graph_run_writes_tool_audit_admit_and_result`, that runs one tool call through a factory built the way `plan_runner` builds it (fake provider/tool) and asserts one admit and one result line in `<tmp>/.roko/tool_audit.jsonl`, with the correlation ID and no raw secret;
   - a roko-agent unit test asserting `emit_terminal_audit` records a non-empty `ToolMetrics` and an elapsed time that differs from the timeout.

## Done when

- After `roko plan run` on a plan whose task makes at least one tool call through an API provider, `.roko/tool_audit.jsonl` has a scrubbed admit and result pair per call with run/task correlation, and `roko learn tools` lists them.
- `.roko/traces/` and `.roko/metrics/tool_metrics.jsonl` receive per-call records with real duration and success.
- Verify. The current command requires a `.with_trace_sink(`/`.with_metrics_sink(` call inside roko-cli/serve/execution, which the natural fix (setting fields on `AgentOptions`, calling the setters inside `roko-agent` providers) does not produce. Suggested replacement:
  `grep -rqE 'ToolAuditLog::open|ScrubAuditAdapter::new' crates/roko-cli/src/graph_execution && grep -rqE 'trace_sink_dyn|JsonlTraceSink::new' crates/roko-cli/src && grep -rqw 'fn graph_run_writes_tool_audit_admit_and_result' crates/roko-cli/ && cargo test -p roko-cli graph_run_writes_tool_audit_admit_and_result`

## Notes

- Never persist raw tool arguments or raw results. `ScrubAuditAdapter` is the only allowed writer; keep its scrub and truncate step.
- The audit file is shared by concurrent tasks and processes. `ToolAuditLog` already takes an advisory lock (`tool_audit.jsonl.lock`), so do not bypass it with a second writer.
- TD-006 (result/call misattribution in turn traces) belongs to `bug-7debad`, not here.
- Touches `plan_runner.rs`, `dispatch_v2.rs`, `provider/mod.rs` and the provider tool-loop builders. Moderate conflict risk with other Graph-runner or provider items, so avoid running it in parallel with items anchored in `plan_runner.rs` or `provider/*`.
- Size M.

- Implemented on `work/find-f489db` at `df746d76c`; cargo verification deferred to the batch check.
- Premise re-checked at a962bcab9; it still held. Two further gaps appeared:
  - Every production `ToolContext` had an empty correlation, because nothing called `ToolLoopAgent::with_correlation`.
  - Wiring `JsonlTraceSink` as it was would have leaked a file descriptor per tool call: the dispatcher appended to a
    fresh trace id per call and never finished it, and the sink keeps an open writer per unfinished trace.
- How each plan step was done:
  - TD-003. `attach_tool_observability` (`plan_runner.rs`) attaches the audit and the sinks to the run's factory.
    Opening the log can fail; the run then warns and goes on. The audit scrubs with the process's secret scrubber
    (`roko_core::obs::secret_scrubber`), else the built-in patterns. `roko run` goes through the same plan runner.
    Not wired: `dispatch/mod.rs::spawn_agent_result_bridge`, and serve paths that do not run a Graph plan.
  - Correlation. Each `AuditLine` gains an optional `correlation`: run, task, attempt key and agent, each cut to
    256 bytes. `AgentDispatcherV2` derives it from the request's attempt key and passes it as
    `AgentOptions.tool_correlation`.
  - TD-005. The sinks go through `SharedAgentFactory` and `AgentDispatcherV2` (`with_observability_sinks`) into
    `AgentOptions` (`trace_sink`, `metrics_sink`). Every provider tool loop applies them, Perplexity included.
  - Placeholders. `HandlerFinished.exit_ms` is now the call's wall-clock time: safety checks, handler and screening.
    The timeout stays in the audit details. `ToolMetrics` holds quality rates, not durations, so each call records a
    one-call sample (known tool, schema check, completion), and duration and success go into the closed trace's
    outcome. Model, role and format come from the dispatcher's `ToolCallIdentity`
    (`build_provider_tool_dispatcher`): the model slug, the contract's role (default implementer, since `AgentRole`
    has no unknown) and the translator's format.
  - Flush. Not needed: each trace is finished per call, which flushes and closes it.
- Tests: `graph_run_writes_tool_audit_admit_and_result` (roko-cli lib; a real Graph task dispatch against the
  OpenAI-compatible mock) and `terminal_observation_records_elapsed_time_and_metrics` (roko-agent dispatcher).
- `ScrubAuditAdapter::record_admit`/`record_result` now take the correlation. Their callers in roko-fs's tests and
  `tests/tool_audit_evidence.rs` pass an empty one. Four full `AgentOptions` literals gained the three new fields.

## Original notes

durable tool audit log has no production caller; timeout not enforced. Tool-audit dispatch contract findings TD-003 through TD-005 are still open: (TD-003) Durable ToolAuditLog has no production caller — only test code writes to it. (TD-004) ToolDef::timeout_ms is advertised in tool definitions…

Imported without verification from:
- `tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)`

How to verify: Check whether the gap described in tmp/backlog/archive/389-tool-dispatch-observability-gaps.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: TD-003 and TD-005 are still true, and TD-004 is fixed. TD-003: nothing in production opens `ToolAuditLog` or builds a `ScrubAuditAdapter` (only roko-fs tests and crates/roko-cli/tests/tool_audit_evidence.rs do), and the `with_tool_audit` setters (crates/roko-cli/src/dispatch/factory.rs:278, dispatch_v2.rs:1519) have no callers. TD-005: `ToolExecutionContextFactory` defaults to Noop audit, trace and metrics sinks (tool_loop/context_factory.rs:74-77), and the trace/metrics setters are called only in roko-core tests (tool/handler.rs:657-658). TD-004 fixed: dispatcher/mod.rs:652-660 (T029) enforces `ToolDef::timeout_ms`.
