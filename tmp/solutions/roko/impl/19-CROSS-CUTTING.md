# Cross-Cutting Concerns: Error Handling, Logging, Events, Shutdown, Cleanup, Deployment

> Cross-cutting improvements that span multiple crates. Each task touches
> infrastructure that underlies every subsystem: error propagation, logging
> consistency, event bus reliability, cancellation, shutdown, resource cleanup,
> API stability, and deployment. These tasks are designed for parallel execution
> by independent agents after dependency edges are resolved.
>
> Core thesis: **standardize the plumbing so feature work stops reinventing it.**

---

## Section A: Error Handling Standardization (Tasks 19.01--19.05)

### Task 19.01: Audit and Migrate anyhow Usage at Crate Boundaries

**Problem**: `anyhow::Error` appears in 181+ call sites across 30+ files. Inside
crates this is acceptable, but at public API boundaries it erases error structure,
preventing callers from matching on specific failure modes or building retry logic.
The error philosophy in `roko-core/src/error/mod.rs` explicitly names this as an
anti-pattern.

**Files**:
- Audit: all `pub fn` / `pub async fn` in `crates/roko-neuro/src/lib.rs`,
  `crates/roko-dreams/src/cycle.rs`, `crates/roko-dreams/src/runner.rs`,
  `crates/roko-index/src/workspace.rs`, `crates/roko-demo/src/deploy.rs`
- Modify: each file's public function signatures

**Steps**:
1. Run `grep -rn 'anyhow::Result\|anyhow::Error\|anyhow::bail\|anyhow::anyhow' crates/ --include='*.rs' | grep -v target/ | grep -v test | grep 'pub '` to find public API sites
2. For each crate with public `anyhow::Result` returns, create a crate-level error enum deriving `thiserror::Error` with `#[non_exhaustive]`
3. Convert public signatures from `anyhow::Result<T>` to `Result<T, CrateError>` where `CrateError` implements `From<anyhow::Error>` for internal fallback
4. Implement `From<CrateError>` for `RokoError` on each new error type to maintain the unified taxonomy from `roko-core/src/error/mod.rs`
5. Keep `anyhow` for internal-only functions -- only public boundaries matter

**Acceptance criteria**:
- `grep -rn 'pub.*fn.*anyhow' crates/roko-neuro/ crates/roko-dreams/ crates/roko-index/ --include='*.rs' | grep -v test | grep -v target/` returns 0 results
- All new error types implement `Into<RokoError>` via the `ErrorKind` discriminant
- `cargo test --workspace` passes without regression

**Dependencies**: None

---

### Task 19.02: Implement Structured Error Context Chain

**Problem**: Error messages like `"substrate error: failed"` lose the causal chain.
When a gate failure causes a replan, the replan agent sees the final error string
but not the nested cause (e.g., clippy lint X in file Y at line Z). The `RokoError`
variants carry `String` payloads, not structured context.

**Files**:
- `crates/roko-core/src/error/mod.rs` (extend `RokoError`)
- `crates/roko-gate/src/gate_service.rs` (gate error enrichment)
- `crates/roko-cli/src/runner/gate_dispatch.rs` (propagation)

**Steps**:
1. Add `ErrorContext` struct to `roko-core/src/error/mod.rs`:
   ```rust
   pub struct ErrorContext {
       pub subsystem: &'static str,  // "gate", "agent", "substrate"
       pub operation: String,         // "clippy", "compile", "dispatch"
       pub detail: String,            // human-readable
       pub source_file: Option<String>,
       pub source_line: Option<u32>,
       pub suggestions: Vec<String>,
   }
   ```
2. Add `#[error("...")]` variant `RokoError::Contextual { context: ErrorContext, #[source] source: Box<RokoError> }`
3. Add `RokoError::with_context(self, subsystem, operation)` builder method
4. In `GateService::run_gates()`, wrap gate errors with `ErrorContext` containing the gate name, rung index, and stderr snippet
5. In `gate_dispatch.rs`, propagate the structured context to the autofix agent's prompt

**Acceptance criteria**:
- Gate failure errors contain subsystem, operation, and detail fields
- Autofix agent receives structured error context (not just a flat string)
- Error display still produces a human-readable chain via `Display` impl
- `cargo clippy --workspace -- -D warnings` passes

**Dependencies**: 19.01

---

### Task 19.03: Add ErrorKind Coverage for Missing Subsystems

**Problem**: `ErrorKind` in `roko-core/src/error/mod.rs` covers core subsystems
but is missing discriminants for ACP, MCP, deployment, and daemon errors. These
subsystems currently map to `ErrorKind::Internal`, losing the ability to build
subsystem-specific retry policies.

**Files**:
- `crates/roko-core/src/error/mod.rs` (extend `ErrorKind`)

**Steps**:
1. Add variants to `ErrorKind`: `Acp`, `Mcp`, `Deploy`, `Daemon`, `Tui`, `Learning`
2. Map existing `RokoError` variants to the new discriminants in `RokoError::kind()`
3. Extend `RokoError::is_transient()` with transient classifications for each new kind (e.g., MCP server crashes are transient; deployment auth failures are not)
4. Add doc comments with retry guidance per kind

**Acceptance criteria**:
- All 18 crates' public errors map to a specific `ErrorKind` (not `Internal`)
- `ErrorKind` implements `Display` with stable string labels suitable for metrics
- `cargo test -p roko-core` passes

**Dependencies**: None

---

### Task 19.04: Standardize Error Logging with Span Context

**Problem**: Errors are logged inconsistently: some use `tracing::error!`, some use
`eprintln!`, some are silently swallowed. 66+ files use `tracing` but with
inconsistent span structures. Error events lack the run ID, task ID, and agent ID
needed to correlate failures in multi-agent runs.

**Files**:
- `crates/roko-runtime/src/workflow_engine.rs` (add tracing spans)
- `crates/roko-cli/src/runner/event_loop.rs` (add tracing spans)
- `crates/roko-acp/src/runner.rs` (add tracing spans)
- `crates/roko-serve/src/routes/*.rs` (standardize error responses)

**Steps**:
1. Define a standard span hierarchy in `roko-runtime`:
   ```
   roko.run[run_id] -> roko.task[task_id] -> roko.agent[agent_id] -> roko.gate[gate_name]
   ```
2. In `workflow_engine.rs`, wrap the main run loop in `tracing::info_span!("roko.run", run_id = %run_id)`
3. In `event_loop.rs`, wrap each task dispatch in `tracing::info_span!("roko.task", task_id = %task_id)`
4. In ACP `runner.rs`, add `roko.acp.session[session_id]` span
5. Replace all `eprintln!` error reporting in `crates/roko-cli/src/` with `tracing::error!`
6. In serve routes, ensure all error responses include `request_id` from middleware

**Acceptance criteria**:
- `RUST_LOG=roko=debug roko plan run` produces structured tracing output with nested spans
- Every error event in the log has `run_id` and `task_id` fields when applicable
- Zero `eprintln!` calls remain in `roko-cli/src/` (excluding test code and the TUI raw terminal output)
- `cargo clippy --workspace -- -D warnings` passes

**Dependencies**: None

---

### Task 19.05: Wire RPC Error Codes Across All JSON-RPC Surfaces

**Problem**: The `roko-core/src/error/rpc.rs` module defines `RpcError` with
standard JSON-RPC error codes, but ACP and MCP surfaces define their own inline
error codes. ACP uses `-32001` for `SESSION_BUSY` without going through the
standard mapping. MCP servers use ad-hoc error codes.

**Files**:
- `crates/roko-core/src/error/rpc.rs` (extend codes)
- `crates/roko-acp/src/handler.rs` (use RpcError)
- `crates/roko-mcp-stdio/src/lib.rs` (use RpcError)

**Steps**:
1. Add ACP-specific error codes to `RpcError`: `SessionBusy(-32001)`, `SessionNotFound(-32002)`, `PromptTooLong(-32003)`
2. Add MCP-specific error codes: `ToolNotFound(-32010)`, `ToolTimeout(-32011)`, `ScriptFailed(-32012)`
3. In ACP `handler.rs`, replace inline `serde_json::json!({"code": -32001, ...})` with `RpcError::SessionBusy.to_response(id)`
4. In MCP `serve_stdio()`, replace inline error construction with `RpcError` conversions
5. Document all codes in `rpc.rs` module docs

**Acceptance criteria**:
- All JSON-RPC error responses across ACP and MCP go through `RpcError`
- `grep -rn '"code".*-320' crates/roko-acp/ crates/roko-mcp-*/ --include='*.rs' | grep -v rpc.rs` returns 0 inline codes
- Error code documentation is complete in `rpc.rs`

**Dependencies**: 19.03

---

## Section B: Logging and Observability Consistency (Tasks 19.06--19.09)

### Task 19.06: Unify Logging Initialization Across Entry Points

**Problem**: `roko serve`, `roko plan run`, `roko chat`, and `roko agent serve` each
initialize logging differently. Some use `tracing_subscriber` with `EnvFilter`,
some use a JSONL logger, some use both. The ACP server has its own log file
configuration at `crates/roko-acp/src/config.rs`.

**Files**:
- `crates/roko-cli/src/main.rs` (centralize logging init)
- `crates/roko-runtime/src/lib.rs` (export shared init function)

**Steps**:
1. Create `roko_runtime::logging::init(config: &LogConfig)` that handles all logging setup
2. `LogConfig` reads from `roko.toml` `[logging]` section: `level`, `format` (text/json), `file` (optional path), `otel_endpoint` (optional)
3. The init function sets up: `tracing_subscriber::fmt` layer + optional JSONL file layer + optional OTel layer
4. Replace all ad-hoc logging init in `main.rs`, `daemon.rs`, `serve_runtime.rs`, and ACP `config.rs` with the single `init()` call
5. Ensure `RUST_LOG` env var overrides take precedence over config

**Acceptance criteria**:
- All CLI entry points use `roko_runtime::logging::init()`
- `roko serve` and `roko plan run` produce identically structured log output at the same level
- `[logging] format = "json"` in roko.toml produces structured JSON log lines
- `cargo test --workspace` passes

**Dependencies**: None

---

### Task 19.07: Add Request-Scoped Correlation IDs

**Problem**: When `roko serve` handles concurrent HTTP requests, log lines from
different requests interleave without correlation. The SSE and WebSocket routes
emit events without request IDs, making it impossible to trace a dashboard
update back to its originating API call.

**Files**:
- `crates/roko-serve/src/routes/middleware.rs` (add correlation ID middleware)
- `crates/roko-core/src/runtime_event.rs` (add `correlation_id` to envelope)

**Steps**:
1. Add Axum middleware that generates or extracts `X-Request-ID` header, stores in request extensions
2. Create a tracing span `roko.http[request_id]` per request in the middleware
3. Add optional `correlation_id: Option<String>` to `RuntimeEventEnvelope`
4. When events are emitted from an HTTP-initiated context, populate `correlation_id` from the request extension
5. SSE/WebSocket routes include `correlation_id` in event payloads

**Acceptance criteria**:
- `curl -H "X-Request-ID: test-123" http://localhost:6677/api/status` logs contain `request_id=test-123`
- Without the header, a UUID is auto-generated
- RuntimeEvents emitted from HTTP routes carry the correlation ID

**Dependencies**: 19.06

---

### Task 19.08: Emit gen_ai.* OpenTelemetry Semantic Conventions

**Problem**: Per the research synthesis (section 2.3), native gen_ai.* OTel emission
would give six vendor integrations for ~200 LOC. Currently, runtime events go to
JSONL only. No OTel spans are emitted, meaning no integration with Datadog,
Honeycomb, Langfuse, Phoenix, Langtrace, or Grafana.

**Files**:
- `crates/roko-runtime/src/otel.rs` (new file)
- `crates/roko-runtime/src/workflow_engine.rs` (emit spans)
- `crates/roko-agent/src/model_call_service.rs` (emit per-call spans)
- `crates/roko-runtime/Cargo.toml` (add `opentelemetry`, `opentelemetry-otlp`)

**Steps**:
1. Create `otel.rs` module with `init_otel(endpoint: &str, protocol: &str) -> TracerProvider`
2. Define attribute mapping to gen_ai.* v1.37+ conventions:
   - `gen_ai.provider.name` from `ProviderKind::label()`
   - `gen_ai.operation.name` = "chat" | "execute_tool" | "retrieval"
   - `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`
   - `gen_ai.usage.cache_read.input_tokens`
   - `gen_ai.conversation.id` from session ID
3. In `ModelCallService::call()`, create a span per model call with gen_ai.* attributes
4. In `WorkflowEngine`, create parent spans for workflow runs
5. Add `[observability]` section to roko.toml config: `provider`, `endpoint`, `protocol`
6. Ensure OTel export is off by default (opt-in via config)

**Acceptance criteria**:
- With `[observability] endpoint = "http://localhost:4317"`, spans are exported via OTLP
- Each model call produces a span with `gen_ai.provider.name` and `gen_ai.usage.*` attributes
- Without config, zero overhead (no OTel initialization)
- `cargo test --workspace` passes (OTel disabled in tests)

**Dependencies**: 19.06

---

### Task 19.09: Gate Results as Structured Compliance Events

**Problem**: Gate results are logged to JSONL but not emitted as structured events
suitable for SIEM/GRC integration. Per the research synthesis (section 4.1),
transforming the gate pipeline from internal QA to externally consumable compliance
stream is a revenue-generating surface, especially with EU AI Act Article 50
enforcement approaching.

**Files**:
- `crates/roko-gate/src/gate_service.rs` (emit compliance events)
- `crates/roko-core/src/runtime_event.rs` (add `GateCompliance` variant)

**Steps**:
1. Add `RuntimeEvent::GateCompliance { gate_name, rung, verdict, detail, duration_ms, agent_id, task_id }` variant
2. In `GateService::run_gates()`, after each gate execution emit a `GateCompliance` event to the event bus
3. If OTel is configured (task 19.08), also emit an OTel span per gate with attributes: `roko.gate.name`, `roko.gate.rung`, `roko.gate.verdict`, `roko.gate.duration_ms`
4. Add `[gates] compliance_events = true` config flag (default false)
5. When enabled, gate events flow through EventBus -> SSE/WebSocket for external consumers

**Acceptance criteria**:
- With `compliance_events = true`, each gate execution produces a `GateCompliance` RuntimeEvent
- Events are visible via SSE at `/api/events`
- Without the flag, no extra event overhead
- Gate events include the full verdict (pass/fail/skip) with detail

**Dependencies**: 19.08

---

## Section C: StateHub and Event Bus Improvements (Tasks 19.10--19.13)

### Task 19.10: Unify DashboardEvent and RuntimeEvent Types

**Problem**: Two parallel event systems exist: `DashboardEvent` (in
`roko-core/src/dashboard_snapshot.rs`) drives the TUI via StateHub, and
`RuntimeEvent` (in `roko-core/src/runtime_event.rs`) drives the workflow engine's
observers. They overlap (both have agent start/complete, gate results) but use
different types, causing the same event to be emitted twice through different
channels with different shapes.

**Files**:
- `crates/roko-core/src/runtime_event.rs` (extend)
- `crates/roko-core/src/dashboard_snapshot.rs` (derive from RuntimeEvent)
- `crates/roko-core/src/state_hub.rs` (accept RuntimeEvent, project to snapshot)

**Steps**:
1. Add `impl From<RuntimeEvent> for DashboardEvent` that maps runtime events to their dashboard equivalents
2. Add missing variants to `RuntimeEvent` for events that only exist in `DashboardEvent` (e.g., `AgentOutput`, `TokenUpdate`)
3. Modify `StateHub::publish()` to accept `RuntimeEvent` and auto-convert
4. Keep `DashboardEvent` as the TUI-facing type but derive it from `RuntimeEvent`
5. Remove duplicate event emission sites where both event types are emitted for the same occurrence

**Acceptance criteria**:
- Single event emission point per occurrence (not two parallel emits)
- TUI still receives `DashboardEvent` via `watch` channel
- SSE/WebSocket still receive events via broadcast channel
- `cargo test --workspace` passes
- Event emission count does not increase (no regression in event volume)

**Dependencies**: None

---

### Task 19.11: Add Event Bus Backpressure and Overflow Handling

**Problem**: The `EventBus<DashboardEvent>` broadcast channel silently drops events
when consumers lag. During fast multi-agent runs, the TUI or SSE clients can miss
events with no indication. The ring buffer (1024 entries in StateHub) provides
replay but late joiners still lose events beyond the ring.

**Files**:
- `crates/roko-runtime/src/event_bus.rs` (add backpressure)
- `crates/roko-core/src/state_hub.rs` (add overflow tracking)

**Steps**:
1. In `EventBus`, add an overflow counter: `Arc<AtomicU64>` tracking total dropped events
2. Add `EventBus::overflow_count() -> u64` public method
3. In `StateHub`, log a warning when overflow count increases: `tracing::warn!(overflow = count, "event bus overflow: {count} events dropped")`
4. Add a `DashboardEvent::Overflow { dropped_count }` variant so the TUI can display a "missed N events" indicator
5. Increase the default broadcast channel capacity from 256 to 2048 for multi-agent runs
6. Add `[dashboard] event_buffer_size = 2048` config option

**Acceptance criteria**:
- Overflow events are tracked and logged
- TUI displays "N events missed" when overflow occurs
- Default capacity handles 10 concurrent agents at 10 events/second without overflow
- `cargo test --workspace` passes

**Dependencies**: 19.10

---

### Task 19.12: Add Event Bus Filtering and Subscription Topics

**Problem**: All consumers receive all events. The TUI does not need gate compliance
events. The SSE learning endpoint does not need agent output chunks. Broadcasting
everything wastes CPU on serialization and deserialization for events consumers
discard.

**Files**:
- `crates/roko-runtime/src/event_bus.rs` (add topic filtering)

**Steps**:
1. Add `EventTopic` enum: `Agent`, `Gate`, `Learning`, `System`, `Compliance`, `All`
2. Add `RuntimeEvent::topic() -> EventTopic` method that classifies each variant
3. Add `EventBus::subscribe_filtered(topics: &[EventTopic]) -> FilteredReceiver` that only delivers matching events
4. Keep `EventBus::subscribe()` as the unfiltered path for backward compatibility
5. Migrate SSE route to use filtered subscription (exclude `Agent` output chunks)
6. Migrate TUI bridge to use filtered subscription (exclude `Compliance` events)

**Acceptance criteria**:
- Filtered subscribers only receive events matching their topic set
- Unfiltered subscribers still receive everything
- No performance regression for the unfiltered path
- `cargo test --workspace` passes

**Dependencies**: 19.10

---

### Task 19.13: Make StateHub Snapshot Serializable for REST API

**Problem**: `DashboardSnapshot` is served via the REST API at `/api/status` but the
serialization is ad-hoc: some fields are skipped, some are transformed inline in the
route handler. The snapshot should be directly serializable with a stable schema.

**Files**:
- `crates/roko-core/src/dashboard_snapshot.rs` (add serde derives, schema version)
- `crates/roko-serve/src/routes/status/mod.rs` (use direct serialization)

**Steps**:
1. Ensure all fields in `DashboardSnapshot` implement `Serialize + Deserialize`
2. Add `schema_version: u8` field (start at 1) for forward compatibility
3. Add `generated_at: DateTime<Utc>` field
4. In the `/api/status` route, return `Json(snapshot)` directly instead of constructing an ad-hoc response object
5. Document the snapshot schema in `DashboardSnapshot` doc comments

**Acceptance criteria**:
- `GET /api/status` returns the full `DashboardSnapshot` with `schema_version: 1`
- `serde_json::to_string(&snapshot)` round-trips cleanly
- Existing TUI rendering is unaffected
- `cargo test --workspace` passes

**Dependencies**: None

---

## Section D: Cancellation and Graceful Shutdown (Tasks 19.14--19.18)

### Task 19.14: Propagate CancelToken Through All Dispatch Paths

**Problem**: `CancelToken` from `roko-runtime/src/cancel.rs` supports hierarchical
cancellation (parent cancels children), but not all dispatch paths thread it through.
The `WorkflowEngine` passes a `CancelToken` to the effect driver, but `chat_inline.rs`,
`dispatch_direct.rs`, and `run.rs` create their own ad-hoc cancellation or none at all.

**Files**:
- `crates/roko-cli/src/chat_inline.rs` (accept CancelToken)
- `crates/roko-cli/src/dispatch_direct.rs` (accept CancelToken)
- `crates/roko-cli/src/run.rs` (accept CancelToken)
- `crates/roko-cli/src/run_inline.rs` (accept CancelToken)

**Steps**:
1. In `main.rs`, create a root `CancelToken` and wire SIGINT/SIGTERM to call `root.cancel()`
2. Pass `root.child()` to every dispatch entry point: `run()`, `chat_inline()`, `dispatch_direct()`
3. In each dispatch function, check `cancel.is_cancelled()` before each major phase (agent spawn, gate run, merge)
4. Replace any `tokio::signal::ctrl_c().await` with `cancel.cancelled().await` in select loops
5. Ensure the ACP server's per-session `CancelToken` (already in `session.rs`) is a child of the root token

**Acceptance criteria**:
- Ctrl-C in `roko run "hello"` cancels within 2 seconds (no orphan agent processes)
- Ctrl-C in `roko chat` cancels the current agent call and returns to the prompt
- Ctrl-C in `roko plan run` cancels the current task, persists state, and exits
- `cancel.is_cancelled()` is checked in all dispatch paths

**Dependencies**: None

---

### Task 19.15: Wire GracefulShutdown into roko serve

**Problem**: `GracefulShutdown` in `roko-core/src/shutdown.rs` is fully implemented
with hook registration, concurrent drain, and hard deadline. But `roko serve` in
`crates/roko-serve/src/lib.rs` uses ad-hoc `force_shutdown` signaling and does not
register subsystem hooks. WebSocket connections are dropped without drain. SSE
streams cut off mid-event.

**Files**:
- `crates/roko-serve/src/lib.rs` (wire GracefulShutdown)
- `crates/roko-serve/src/state.rs` (add GracefulShutdown to AppState)
- `crates/roko-serve/src/routes/ws.rs` (register drain hook)
- `crates/roko-serve/src/routes/sse.rs` (register drain hook)

**Steps**:
1. Add `GracefulShutdown` to `AppState` alongside the existing `CancelToken`
2. Register shutdown hooks for each subsystem:
   - `"ws-drain"`: send `subscription_ended` to all WebSocket clients, wait up to 1s
   - `"sse-drain"`: send final SSE event to all clients, close connections
   - `"state-flush"`: flush StateHub snapshot to disk
   - `"metrics-flush"`: flush any buffered OTel/metrics data
3. On SIGTERM/SIGINT, call `shutdown.drain().await` instead of the current `force_shutdown` flag
4. Log the `ShutdownReport` (drained_hooks, timed_out_hooks, elapsed_ms)
5. Use `GracefulShutdown::wait_started()` in the Axum `into_make_service()` to stop accepting new connections during drain

**Acceptance criteria**:
- `kill -TERM <roko-serve-pid>` produces a `ShutdownReport` in the log
- WebSocket clients receive `subscription_ended` before disconnection
- SSE streams receive a final event before closing
- Shutdown completes within the 5-second deadline
- No orphan background tasks after shutdown

**Dependencies**: None

---

### Task 19.16: Wire GracefulShutdown into roko daemon

**Problem**: `roko daemon stop` sends SIGTERM but does not coordinate with the
daemon's internal subsystems. The daemon may be mid-plan-run when killed, leaving
worktrees in dirty state, agent processes orphaned, and learning files unflushed.

**Files**:
- `crates/roko-cli/src/daemon.rs` (wire GracefulShutdown)
- `crates/roko-runtime/src/lifecycle.rs` (lifecycle coordination)

**Steps**:
1. In daemon main loop, create `GracefulShutdown::with_deadline(Duration::from_secs(10))`
2. Register hooks:
   - `"plan-runner"`: signal CancelToken, wait for current task to checkpoint
   - `"agent-processes"`: call `ProcessSupervisor::shutdown_all()` with 5-second timeout
   - `"learning-flush"`: flush episode logger, cascade router, efficiency writer
   - `"worktree-cleanup"`: ensure all worktrees have their latest changes committed
3. Wire SIGTERM handler to `shutdown.drain().await`
4. Add `roko daemon stop --timeout <seconds>` flag to override the default deadline
5. After drain, log a summary of what was flushed and what timed out

**Acceptance criteria**:
- `roko daemon stop` drains all subsystems before exiting
- Agent processes are killed if they do not exit within 5 seconds
- Learning files are flushed to disk before exit
- `roko daemon stop --timeout 2` uses a 2-second deadline

**Dependencies**: 19.14, 19.15

---

### Task 19.17: Add Shutdown Hooks for ACP Server

**Problem**: The ACP server (`crates/roko-acp/src/handler.rs`) runs as a subprocess
of editors (Zed, JetBrains). When the editor sends EOF on stdin, the ACP server
should drain active sessions, flush episodes, and persist cascade router state.
Currently it exits immediately on EOF.

**Files**:
- `crates/roko-acp/src/handler.rs` (add shutdown path)
- `crates/roko-acp/src/session.rs` (add session drain)
- `crates/roko-acp/src/bridge_events.rs` (flush pending episodes)

**Steps**:
1. In `run_acp_server()`, when the transport returns `None` (EOF), enter shutdown
2. Create `GracefulShutdown::with_deadline(Duration::from_secs(3))` for the ACP server
3. Register hooks:
   - `"active-sessions"`: cancel all active session CancelTokens, wait for prompts to complete
   - `"episode-flush"`: flush any buffered episode data to `.roko/episodes.jsonl`
   - `"router-save"`: persist cascade router to `.roko/learn/cascade-router.json`
   - `"session-persist"`: save all session state to disk for later `session/load`
4. After drain, exit cleanly with code 0

**Acceptance criteria**:
- ACP server flushes episodes on editor close
- Active prompts are cancelled (not left running as orphan processes)
- Session state is persisted for later resume
- Shutdown completes within 3 seconds

**Dependencies**: 19.14

---

### Task 19.18: Implement Force-Kill Escalation for Agent Processes

**Problem**: `ProcessSupervisor` in `roko-runtime` sends SIGTERM to agent processes
but has no escalation path. If an agent's Claude CLI subprocess is stuck in a long
model call, SIGTERM may not work. The dogfood session revealed agents surviving
force_shutdown because SIGTERM was ignored by the subprocess tree.

**Files**:
- `crates/roko-runtime/src/process.rs` (add force-kill escalation)

**Steps**:
1. Add `ProcessSupervisor::shutdown_with_escalation(timeout: Duration)`:
   - Phase 1 (0-timeout/2): SIGTERM to process group
   - Phase 2 (timeout/2-timeout): SIGKILL to process group
   - Phase 3 (after timeout): kill each PID individually if group kill failed
2. Use `nix::sys::signal::killpg()` for process group signaling (already used in the codebase)
3. Track process group IDs at spawn time via `setsid()` or `process_group(0)`
4. Log each escalation step: `tracing::warn!("agent {} did not respond to SIGTERM, escalating to SIGKILL", pid)`
5. After kill, verify process is dead with `waitpid(WNOHANG)`

**Acceptance criteria**:
- Agent processes that ignore SIGTERM are killed within `timeout` seconds
- Process group kill catches child processes spawned by the agent
- Log output shows the escalation progression
- No zombie processes remain after shutdown

**Dependencies**: None

---

## Section E: Resource Cleanup (Tasks 19.19--19.22)

### Task 19.19: Implement Worktree Cleanup Policy

**Problem**: Per MEMORY.md and project rules, worktrees are never deleted
automatically. Over time, this accumulates hundreds of worktrees (visible in the
glob output: 20+ worktrees under `.claude/worktrees/` and `.roko/worktrees/`). There
is no visibility into worktree age, size, or relationship to completed plans.

**Files**:
- `crates/roko-orchestrator/src/worktree.rs` (add status reporting)
- `crates/roko-cli/src/commands/util.rs` (add `roko util worktrees` subcommand)

**Steps**:
1. Add `WorktreeManager::status() -> Vec<WorktreeInfo>` that reports: path, branch, age, disk size, plan association, clean/dirty
2. Add `roko util worktrees list` subcommand that displays the status table
3. Add `roko util worktrees gc --older-than 30d --dry-run` that identifies candidates for cleanup
4. Without `--dry-run`, prompt for confirmation (never auto-delete per project rules)
5. Add `roko util worktrees archive <path>` that creates a tarball of the worktree before removal
6. Track worktree creation time in `.roko/state/worktrees.json`

**Acceptance criteria**:
- `roko util worktrees list` shows all worktrees with age and size
- `roko util worktrees gc --older-than 30d --dry-run` lists candidates without deleting
- Actual deletion requires explicit user confirmation
- Worktree registry persists across restarts

**Dependencies**: None

---

### Task 19.20: Implement Temp File Cleanup on Startup

**Problem**: Failed runs leave temporary files: partial executor snapshots, lock
files, MCP config fragments, and partial JSONL entries. These accumulate in `.roko/`
and can cause issues on restart (e.g., stale lock files blocking new runs).

**Files**:
- `crates/roko-cli/src/commands/init.rs` (add cleanup on init)
- `crates/roko-runtime/src/lib.rs` (add cleanup function)

**Steps**:
1. Create `roko_runtime::cleanup::startup_cleanup(roko_dir: &Path)`:
   - Remove stale `.lock` files older than 1 hour
   - Remove `.tmp` files (partial atomic writes that never completed rename)
   - Truncate corrupt JSONL files at the last valid line boundary
   - Remove empty directories in `.roko/state/tasks/`
2. Call `startup_cleanup()` at the beginning of `plan run`, `serve`, and `daemon start`
3. Log each cleanup action: `tracing::info!("cleaned up stale lock: {}", path)`
4. Add `--no-cleanup` flag to skip startup cleanup for debugging

**Acceptance criteria**:
- Stale `.lock` files are removed on startup
- Corrupt JSONL files are truncated to valid state (not deleted)
- `--no-cleanup` skips all cleanup
- `cargo test --workspace` passes

**Dependencies**: None

---

### Task 19.21: Add Process Orphan Detection

**Problem**: When `roko plan run` is killed (SIGKILL, OOM, power failure), spawned
agent processes may survive as orphans. There is no mechanism to detect and clean up
orphans from a previous run on restart.

**Files**:
- `crates/roko-runtime/src/process.rs` (add orphan detection)
- `crates/roko-cli/src/runner/event_loop.rs` (call on startup)

**Steps**:
1. On agent spawn, write PID to `.roko/state/pids/<run_id>/<task_id>.pid`
2. On clean agent exit, remove the PID file
3. On `plan run` startup, scan `.roko/state/pids/` for PID files from previous runs
4. For each PID file, check if the process is still running (`kill(pid, 0)`)
5. If running, log a warning and offer to kill: `tracing::warn!("orphan process {} from previous run {} still running", pid, run_id)`
6. Add `roko util cleanup-orphans` subcommand that kills orphaned processes
7. In non-interactive mode (daemon, CI), auto-kill orphans from the same run directory

**Acceptance criteria**:
- PID files are created on agent spawn and removed on clean exit
- `roko plan run` warns about orphans from previous runs
- `roko util cleanup-orphans` kills orphaned agent processes
- `cargo test --workspace` passes

**Dependencies**: 19.18

---

### Task 19.22: Periodic Learning File Compaction

**Problem**: Append-only JSONL files grow without bound: `episodes.jsonl`,
`efficiency.jsonl`, `costs.jsonl`. After weeks of use, these files can reach
hundreds of MB, slowing startup (cascade router loads and parses the full file).

**Files**:
- `crates/roko-learn/src/lib.rs` (add compaction)
- `crates/roko-cli/src/commands/learn.rs` (add `roko learn compact` subcommand)

**Steps**:
1. Add `compact_episodes(path: &Path, retention_days: u32)` that:
   - Reads the JSONL file
   - Removes entries older than `retention_days`
   - Computes aggregate statistics for removed entries (total tokens, total cost, pass rate by model)
   - Writes a summary entry and the retained entries to a new file
   - Atomic rename to replace the original
2. Add `compact_cascade_router(path: &Path)` that prunes observations older than the confidence window
3. Add `roko learn compact --retention-days 30` subcommand
4. Add optional auto-compaction on startup when file exceeds 50MB (configurable)
5. Always preserve the aggregate summary so historical trends are not lost

**Acceptance criteria**:
- `roko learn compact --retention-days 30` reduces file size by removing old entries
- Aggregate statistics are preserved in a summary entry
- CascadeRouter observations are pruned without losing learned routing weights
- `cargo test --workspace` passes

**Dependencies**: None

---

## Section F: API Versioning and Backward Compatibility (Tasks 19.23--19.26)

### Task 19.23: Add API Version Header to roko serve

**Problem**: The HTTP control plane has ~85 routes with no versioning. Breaking
changes to response schemas will silently break dashboard clients, CLI callers, and
external integrations.

**Files**:
- `crates/roko-serve/src/routes/middleware.rs` (add version header)
- `crates/roko-serve/src/openapi.rs` (add version to OpenAPI spec)

**Steps**:
1. Add `X-Roko-API-Version: 1` response header to all routes via middleware
2. Add `Accept-Version: 1` request header support (optional; defaults to latest)
3. In the OpenAPI spec, set `info.version` to `"1.0.0"`
4. Add version negotiation: if client sends `Accept-Version: 2` and server only supports 1, return `406 Not Acceptable`
5. Document versioning policy: breaking changes require a new version; additive changes do not

**Acceptance criteria**:
- All responses include `X-Roko-API-Version: 1`
- `Accept-Version: 999` returns 406
- OpenAPI spec has version `1.0.0`
- Existing clients without version headers work unchanged

**Dependencies**: None

---

### Task 19.24: Add Schema Version to All Persisted JSON Files

**Problem**: Persisted files (executor.json, cascade-router.json, gate-thresholds.json,
experiments.json, episodes.jsonl) have no schema version. Format changes cause silent
parse failures or data loss.

**Files**:
- `crates/roko-orchestrator/src/runtime_snapshot.rs` (already has `schema_version`)
- `crates/roko-learn/src/feedback_service.rs` (add schema version)
- `crates/roko-learn/src/playbook.rs` (add schema version)

**Steps**:
1. For each persisted JSON file, ensure the root object has `"schema_version": N`
2. On load, check the schema version. If unknown (newer than expected), log a warning and attempt best-effort parsing
3. If the version is older, apply migration functions:
   - `migrate_v0_to_v1(data: Value) -> Value` per file type
4. Add `roko config migrate` subcommand that migrates all persisted files to the latest schema
5. Document schema changes in each file's module docs

**Acceptance criteria**:
- All persisted JSON files have `schema_version` field
- Older schemas are auto-migrated on load
- Newer schemas produce a warning, not a crash
- `roko config migrate` succeeds on a workspace with v0 files

**Dependencies**: None

---

### Task 19.25: Add roko.toml Config Schema Validation

**Problem**: roko.toml accepts any keys without validation. Typos like
`defalt_model` instead of `default_model` are silently ignored. The config schema
divergence between `[[gate]]` and `[gates]` (documented in 06-IMPLEMENTATION-PLANS.md,
Plan 5) is one symptom of a broader lack of validation.

**Files**:
- `crates/roko-core/src/config/mod.rs` (add validation)
- `crates/roko-cli/src/commands/config_cmd.rs` (add `roko config validate`)

**Steps**:
1. After deserializing `RokoConfig` from TOML, collect unknown keys using `serde_ignored`
2. For each unknown key, compute Levenshtein distance to known keys
3. If distance <= 2, suggest the correct key: `warning: unknown key 'defalt_model', did you mean 'default_model'?`
4. If distance > 2, warn: `warning: unknown key 'xyz' in section [agent]`
5. Add `roko config validate` subcommand that runs validation and reports all issues
6. On `roko plan run`, validate config and warn (do not fail) for unknown keys
7. Accept both `[[gate]]` and `[gates]` formats per Plan 5, with deprecation warning for `[[gate]]`

**Acceptance criteria**:
- `roko config validate` detects typos and suggests corrections
- `defalt_model` in roko.toml produces: `warning: unknown key 'defalt_model', did you mean 'default_model'?`
- Both gate config formats are accepted with deprecation warning
- Validation is non-blocking (warns, does not fail)

**Dependencies**: None

---

### Task 19.26: Add Deprecation Warnings for Config Migrations

**Problem**: Config format changes (e.g., `[[gate]]` -> `[gates]`, old field names)
need deprecation warnings so users know to update. Currently, old formats either
silently work (confusing) or silently break (worse).

**Files**:
- `crates/roko-core/src/config/mod.rs` (add deprecation tracking)

**Steps**:
1. Add `DeprecationWarning { field: String, message: String, since_version: String, removal_version: Option<String> }` struct
2. During config parsing, collect deprecation warnings into `Vec<DeprecationWarning>`
3. Return warnings alongside the parsed config: `fn load_config() -> Result<(RokoConfig, Vec<DeprecationWarning>)>`
4. In CLI entry points, display warnings with color: yellow for deprecations, red for upcoming removals
5. Add `#[deprecated_field(since = "0.5", use_instead = "gates")]` attribute macro for config struct fields

**Acceptance criteria**:
- `[[gate]]` format produces a deprecation warning naming `[gates]` as the replacement
- Warnings include the version where the old format will be removed
- `roko config show` displays active deprecation warnings
- Warnings do not prevent execution

**Dependencies**: 19.25

---

## Section G: Deployment Improvements (Tasks 19.27--19.31)

### Task 19.27: Improve Dockerfile with Multi-Stage Caching

**Problem**: The current Dockerfile copies the entire workspace before building,
invalidating the Docker layer cache on every source change. This makes builds
take 10-20 minutes instead of 1-2 minutes for incremental changes.

**Files**:
- `Dockerfile` (rewrite)

**Steps**:
1. Add a dependency-cache stage:
   ```dockerfile
   FROM rust:1.91-bookworm AS deps
   WORKDIR /app
   COPY Cargo.toml Cargo.lock ./
   COPY crates/*/Cargo.toml ./crates/
   # Create empty lib.rs files so cargo can resolve the workspace
   RUN find crates -name Cargo.toml -exec sh -c 'mkdir -p $(dirname {})/src && echo "" > $(dirname {})/src/lib.rs' \;
   RUN cargo build --release -p roko-cli 2>/dev/null || true
   ```
2. Add the source copy stage that benefits from cached dependencies:
   ```dockerfile
   FROM deps AS builder
   COPY . .
   RUN cargo build --release -p roko-cli
   ```
3. Add a runtime stage with non-root user:
   ```dockerfile
   FROM debian:bookworm-slim AS runtime
   RUN groupadd -r roko && useradd -r -g roko roko
   # ... install deps ...
   USER roko
   ```
4. Add health check: `HEALTHCHECK --interval=30s CMD curl -f http://localhost:6677/api/health || exit 1`
5. Add `.dockerignore` with `target/`, `.roko/`, `tmp/`, `.git/`

**Acceptance criteria**:
- Source-only changes rebuild in < 2 minutes (dependency cache hit)
- Runtime image runs as non-root user
- Health check passes within 30 seconds of container start
- `.dockerignore` prevents sending 2GB+ of target/ to Docker daemon

**Dependencies**: None

---

### Task 19.28: Add Docker Compose for Development

**Problem**: No development setup for running roko serve alongside supporting
services (mock LLM for testing, OTel collector, dashboard). Developers must
manually start each component.

**Files**:
- `docker-compose.yml` (new file)
- `docker-compose.dev.yml` (new file, extends for development)

**Steps**:
1. Create `docker-compose.yml` with:
   - `roko-serve`: builds from Dockerfile, exposes 6677, mounts `.roko/` volume
   - `otel-collector`: `otel/opentelemetry-collector:latest`, receives OTLP on 4317
   - `jaeger`: `jaegertracing/all-in-one:latest`, trace visualization on 16686
2. Create `docker-compose.dev.yml` with:
   - Hot reload via mounted source volume
   - Debug logging level
   - Exposed debug ports
3. Add `scripts/dev-up.sh` that runs `docker compose -f docker-compose.yml -f docker-compose.dev.yml up`
4. Ensure OTel collector forwards to Jaeger for local trace viewing

**Acceptance criteria**:
- `docker compose up` starts roko-serve + OTel collector + Jaeger
- `http://localhost:6677/api/health` returns healthy
- `http://localhost:16686` shows Jaeger UI (when OTel is configured)
- `docker compose down` stops all services cleanly

**Dependencies**: 19.27

---

### Task 19.29: Improve Railway Deployment

**Problem**: `roko deploy railway` exists but does not auto-provision auth (Plan 3.3
in 06-IMPLEMENTATION-PLANS.md), does not set appropriate resource limits, and does
not configure health checks.

**Files**:
- `crates/roko-cli/src/commands/deploy.rs` (or equivalent deploy module)

**Steps**:
1. Auto-generate a 32-byte hex API key on Railway deploy, set as `ROKO_API_KEY` env var
2. Set `api_auth.enabled = true` in the deployed config
3. Print the API key to stdout once: "Save this API key -- it will not be shown again"
4. Configure Railway health check to `GET /api/health` with 30-second interval
5. Set Railway memory limit to 2GB (sufficient for roko-serve + 4 concurrent agents)
6. Add `--region` flag for Railway region selection
7. Validate `RAILWAY_TOKEN` is set before attempting deployment

**Acceptance criteria**:
- `roko deploy railway` prints an API key
- Deployed service rejects unauthenticated requests (returns 401)
- Health check is configured and passing within 60 seconds of deploy
- Missing `RAILWAY_TOKEN` produces a clear error message

**Dependencies**: None

---

### Task 19.30: Add roko deploy docker Subcommand

**Problem**: Users who want to self-host need to manually build the Docker image,
configure volumes, set environment variables, and manage the container. A `roko
deploy docker` command should generate a ready-to-run configuration.

**Files**:
- `crates/roko-cli/src/commands/deploy.rs` (add Docker subcommand)

**Steps**:
1. Add `roko deploy docker` subcommand that:
   - Builds the Docker image using the workspace Dockerfile
   - Generates `docker-compose.prod.yml` with:
     - roko-serve container with configured ports, volumes, and env vars
     - Auto-generated API key in `.env` file
     - Restart policy: `unless-stopped`
     - Log driver configuration: `json-file` with max-size and max-file
   - Prints instructions: "Run: docker compose -f docker-compose.prod.yml up -d"
2. Add `--port` flag (default 6677)
3. Add `--data-dir` flag for `.roko/` volume mount location
4. Add `--tls` flag that generates a self-signed cert and configures HTTPS

**Acceptance criteria**:
- `roko deploy docker` produces a `docker-compose.prod.yml` ready to run
- The generated compose file includes API auth, health check, restart policy, and log limits
- `docker compose -f docker-compose.prod.yml up -d` starts a working roko-serve instance
- `--tls` flag configures HTTPS with a self-signed certificate

**Dependencies**: 19.27

---

### Task 19.31: Add Container Health Monitoring Endpoint

**Problem**: The `/api/health` endpoint returns 200 OK but does not report subsystem
health. In a containerized deployment, operators need to know if the learning
subsystem is healthy, if the event bus is overflowing, if the file system has space,
etc.

**Files**:
- `crates/roko-serve/src/routes/status/health.rs` (extend health check)

**Steps**:
1. Extend `/api/health` to return subsystem health:
   ```json
   {
     "status": "healthy",
     "version": "0.5.0",
     "uptime_seconds": 3600,
     "subsystems": {
       "event_bus": { "status": "healthy", "overflow_count": 0 },
       "learning": { "status": "healthy", "episodes_count": 142 },
       "filesystem": { "status": "healthy", "free_bytes": 10737418240 },
       "agents": { "status": "healthy", "active_count": 2, "orphan_count": 0 }
     }
   }
   ```
2. Return HTTP 200 if all subsystems are healthy, 503 if any are degraded
3. Add `/api/health/ready` (readiness probe) that returns 200 only when the server is fully initialized and accepting requests
4. Add `/api/health/live` (liveness probe) that returns 200 as long as the process is running
5. Configure the Kubernetes/Docker health check to use `/api/health/ready`

**Acceptance criteria**:
- `/api/health` returns subsystem-level health information
- `/api/health/ready` returns 503 during initialization, 200 when ready
- `/api/health/live` always returns 200
- Subsystem health reflects actual state (not hardcoded)

**Dependencies**: 19.11

---

## Priority Matrix

| Priority | Tasks | Impact | Effort | Parallel? |
|---|---|---|---|---|
| **P0** | 19.14 (CancelToken), 19.18 (Force-kill) | Critical | 1-2 days | Yes (with each other) |
| **P1** | 19.04 (Span context), 19.06 (Logging init), 19.15 (Serve shutdown) | High | 2-3 days | Yes (all independent) |
| **P2** | 19.01 (anyhow audit), 19.10 (Event unification), 19.20 (Temp cleanup) | High | 2-3 days | Yes (all independent) |
| **P3** | 19.08 (OTel), 19.23 (API version), 19.27 (Dockerfile) | High | 3-4 days | Yes (all independent) |
| **P4** | 19.19 (Worktree status), 19.21 (Orphan detection), 19.25 (Config validation) | Medium | 2-3 days | Yes (all independent) |
| **P5** | 19.02 (Error context), 19.07 (Correlation IDs), 19.09 (Compliance events) | Medium | 2-3 days | After P1 deps |
| **P6** | 19.11 (Backpressure), 19.12 (Filtering), 19.22 (Compaction) | Medium | 2-3 days | Yes (after 19.10) |
| **P7** | 19.03 (ErrorKind), 19.05 (RPC codes), 19.13 (Snapshot serial.) | Low | 1-2 days | Yes (all independent) |
| **P8** | 19.16 (Daemon shutdown), 19.17 (ACP shutdown) | Medium | 1-2 days | After 19.14, 19.15 |
| **P9** | 19.24 (Schema versions), 19.26 (Deprecation warnings) | Low | 1-2 days | After 19.25 |
| **P10** | 19.28 (Compose), 19.29 (Railway), 19.30 (Docker deploy), 19.31 (Health) | Medium | 3-4 days | After 19.27 |

### Dependency Graph

```
19.01 ──> 19.02  (error context needs typed errors)
19.03 ──> 19.05  (RPC codes need ErrorKind variants)
19.06 ──> 19.07  (correlation IDs need unified logging)
19.06 ──> 19.08  (OTel needs logging infrastructure)
19.08 ──> 19.09  (compliance events use OTel spans)
19.10 ──> 19.11  (backpressure needs unified events)
19.10 ──> 19.12  (filtering needs unified events)
19.14 ──> 19.16  (daemon shutdown needs CancelToken)
19.14 ──> 19.17  (ACP shutdown needs CancelToken)
19.15 ──> 19.16  (daemon shutdown needs serve shutdown)
19.18 ──> 19.21  (orphan detection needs force-kill)
19.25 ──> 19.26  (deprecation warnings need config validation)
19.27 ──> 19.28  (compose needs Dockerfile)
19.27 ──> 19.29  (Railway needs Dockerfile)
19.27 ──> 19.30  (Docker deploy needs Dockerfile)
19.11 ──> 19.31  (health endpoint needs overflow tracking)
```

### Fast Track (1-2 hours each, do immediately)

| Fix | Time | Task |
|---|---|---|
| Propagate CancelToken to chat_inline.rs | 1 hour | 19.14 |
| Replace eprintln! with tracing in roko-cli | 1 hour | 19.04 (partial) |
| Add .dockerignore file | 15 min | 19.27 (partial) |
| Add API version header middleware | 30 min | 19.23 |
| Startup cleanup for stale .lock files | 1 hour | 19.20 |

### Total Estimated Effort

31 tasks. At 4-8 hours per task with 60% parallelism: **10-15 working days** for
full implementation. Fast-track items (5 tasks) can land in 1 day.
