# Cross-Cutting Concerns: Task Breakdown

> Standardize the plumbing across 18 crates: error handling, logging,
> event systems, cancellation, shutdown, resource cleanup, API versioning,
> and deployment. 31 tasks across 7 sections.
>
> Sources: `impl/19-CROSS-CUTTING.md`, `11-CURRENT-STATE-GROUND-TRUTH.md`,
> `06-IMPLEMENTATION-PLANS.md`, codebase analysis

---

## Overview

Cross-cutting concerns are infrastructure that every subsystem touches.
The codebase has grown to ~177K LOC across 18 crates, and each growth
phase introduced its own conventions for error handling, logging, event
emission, and shutdown. The result is:

| Concern | Current State | Problem |
|---|---|---|
| Error types | `anyhow::Result` at 218+ call sites across 30+ files; `RokoError` taxonomy covers 17 kinds but misses ACP/MCP/Deploy/Daemon | Callers cannot match on error kinds; retry logic impossible at crate boundaries |
| Logging | 179 `eprintln!` calls in `roko-cli/src/`; 356 `tracing::*` calls in 10 files; zero structured spans with run/task IDs | Log interleaving in multi-agent runs; no correlation between error and its run |
| Events | Two parallel systems: `DashboardEvent` (23 files) for TUI + `RuntimeEvent` (34 files) for workflow engine; overlap on agent/gate events; same occurrence emitted twice | Double event emission, inconsistent shapes, wasted serialization |
| StateHub | `#[path]` include creates two copies: `roko-cli/src/lib.rs:36` and `roko-serve/src/lib.rs:68` both include `roko-core/src/state_hub.rs` | CLI and serve cannot share state; `roko run --serve` SSE broken |
| Cancellation | `CancelToken` exists in 20 files but `chat_inline.rs`, `dispatch_direct.rs`, `run.rs` create ad-hoc cancellation or none | Ctrl-C leaves orphan agent processes; no hierarchical cancel |
| Shutdown | `GracefulShutdown` fully built in `roko-core/src/shutdown.rs` (hook registration, concurrent drain, deadline) but only imported by 1 file | `roko serve` uses ad-hoc `force_shutdown`; WS/SSE connections dropped without drain |
| Agent kill | `ProcessSupervisor` has `shutdown_all()` and `kill_all()` but no SIGTERM->SIGKILL escalation; dogfood showed agents surviving force_shutdown | Zombie processes after crashes |
| API versioning | ~85 routes in `roko-serve` with zero versioning headers; no `X-Roko-API-Version` | Breaking changes silently break dashboards |
| Config validation | `roko.toml` accepts any keys; typos like `defalt_model` silently ignored; `[[gate]]` vs `[gates]` format mismatch between init and runtime | Config that does nothing; user confusion |
| Persisted schemas | 34 files use `schema_version` but coverage is partial; no migration functions for older formats | Format changes cause silent parse failures |
| Dockerfile | Single-stage build copies everything; no layer cache; runs as root; no health check | 10-20 min rebuilds for source-only changes |
| Deployment auth | `dangerously_skip_permissions: true` always set in plan mode (line 394 of `plan.rs`); cloud deploy has no auto-provisioned auth | Agents run with full permissions; deployed services unauthenticated |

**Target state**: one error taxonomy, one logging init, one event type,
one cancel hierarchy, one shutdown coordinator, one versioned API surface,
one validated config schema, one optimized container image.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-ANYHOW | `anyhow::Result` at public crate boundaries | 218+ sites across `roko-neuro`, `roko-dreams`, `roko-index`, `roko-demo`, `roko-mcp-*` | High |
| AP-EPRINT | 179 `eprintln!` calls instead of structured logging | `crates/roko-cli/src/` -- 22 files, heaviest: `commands/prd.rs` (18), `main.rs` (17), `commands/util.rs` (18) | High |
| AP-2EVENT | Two parallel event types for same occurrences | `DashboardEvent` in `crates/roko-core/src/dashboard_snapshot.rs` vs `RuntimeEvent` in `crates/roko-core/src/runtime_event.rs` | High |
| AP-2HUB | `#[path]` include creates two incompatible StateHub types | `crates/roko-cli/src/lib.rs:36` and `crates/roko-serve/src/lib.rs:68` | Critical |
| AP-NOCANCEL | Chat/run/dispatch paths lack CancelToken propagation | `crates/roko-cli/src/chat_inline.rs`, `dispatch_direct.rs`, `run.rs`, `run_inline.rs` | High |
| AP-NOSHUT | GracefulShutdown built but not wired into serve/daemon/ACP | `crates/roko-core/src/shutdown.rs` exists; `crates/roko-serve/src/lib.rs` uses ad-hoc flag | High |
| AP-NOESCAL | No SIGTERM->SIGKILL escalation for stuck agents | `crates/roko-runtime/src/process.rs` -- `shutdown_all` sends SIGTERM only | Medium |
| AP-NOVERSION | Zero API versioning on 85 routes | `crates/roko-serve/src/routes/middleware.rs` has no version header | Medium |
| AP-SETVAR | `unsafe { std::env::set_var() }` for provider override | `crates/roko-cli/src/commands/util.rs:236` | Medium |
| AP-NOVALID | Config accepts unknown keys silently | `crates/roko-core/src/config/mod.rs` -- no `serde_ignored` or typo detection | Medium |
| AP-DOCKER | Single-stage Dockerfile, runs as root, no health check | `/Users/will/dev/nunchi/roko/roko/Dockerfile` -- `COPY . .` invalidates all caches | Medium |
| AP-RPCINLINE | ACP/MCP use inline JSON-RPC error codes instead of `RpcError` | 10 files use `RpcError` but ACP `handler.rs` has ad-hoc codes | Low |

---

## Section A: Error Handling Standardization

### Task 19.1: Audit and Migrate anyhow Usage at Crate Boundaries
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/cycle.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/deploy.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-scripts/src/main.rs`
**Depends On**: none

#### Context
`anyhow::Error` appears at 218+ call sites across 30+ files in `crates/`. The error philosophy in `crates/roko-core/src/error/mod.rs` explicitly defines a structured `RokoError` enum with 17 `ErrorKind` discriminants and `thiserror` derivation. At public API boundaries, `anyhow::Result` erases this structure, preventing callers from matching on failure modes or building retry logic. Internal crate usage of `anyhow` is acceptable; only `pub fn` / `pub async fn` return types need migration.

The heaviest offenders: `roko-demo` (41 occurrences across 14 files, but demo code is lower priority), `roko-mcp-scripts` (7 occurrences), `roko-dreams` (cycle.rs + runner.rs), `roko-neuro`, `roko-index`.

#### Implementation Steps
1. For each crate with public `anyhow::Result` returns, create a crate-level error enum deriving `thiserror::Error` with `#[non_exhaustive]`. Example for `roko-neuro`:
   ```rust
   #[derive(Debug, thiserror::Error)]
   #[non_exhaustive]
   pub enum NeuroError {
       #[error("knowledge store: {0}")]
       Store(String),
       #[error("query failed: {0}")]
       Query(String),
       #[error(transparent)]
       Io(#[from] std::io::Error),
       #[error(transparent)]
       Other(#[from] anyhow::Error),
   }
   ```
2. Convert public signatures from `anyhow::Result<T>` to `Result<T, CrateError>`.
3. Implement `From<CrateError>` for `RokoError` on each new error type to maintain the unified taxonomy.
4. Keep `anyhow` for internal-only functions -- only public boundaries matter.
5. Verify no public function in the target crates returns `anyhow::Result`.

#### Design Guidance
The `From<anyhow::Error>` variant provides a migration escape hatch -- internal functions that still use `anyhow` can be wrapped with `?` at the boundary. Over time, these `Other` variants should be replaced with specific error kinds. Do not attempt to migrate all 218 sites at once; focus on the 5 crates listed above which are imported by `roko-cli` and `roko-serve`.

#### Verification Criteria
- [ ] Zero `pub` functions in `roko-neuro`, `roko-dreams`, `roko-index` return `anyhow::Result`
- [ ] All new error types implement `Into<RokoError>`
- [ ] `cargo test --workspace` passes without regression
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

---

### Task 19.2: Implement Structured Error Context Chain
**Priority**: P5
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/error/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`
**Depends On**: Task 19.1

#### Context
Error messages like `"substrate error: failed"` lose the causal chain. When a gate failure triggers an autofix agent, the agent sees the final error string but not nested cause information (e.g., clippy lint X in file Y at line Z). The `RokoError` variants carry `String` payloads, not structured context. The gate dispatch in Runner v2 (`crates/roko-cli/src/runner/gate_dispatch.rs`) propagates error strings that the autofix agent must parse.

#### Implementation Steps
1. Add `ErrorContext` struct to `crates/roko-core/src/error/mod.rs`:
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
2. Add `RokoError::Contextual { context: ErrorContext, #[source] source: Box<RokoError> }` variant.
3. Add `RokoError::with_context(self, subsystem, operation)` builder method.
4. In `GateService::run_gates()`, wrap gate errors with `ErrorContext` containing gate name, rung index, and stderr snippet.
5. In `gate_dispatch.rs`, propagate structured context to the autofix agent prompt.

#### Verification Criteria
- [ ] Gate failure errors contain subsystem, operation, and detail fields
- [ ] Autofix agent receives structured error context (not just flat string)
- [ ] Error `Display` impl still produces a human-readable chain
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

---

### Task 19.3: Add ErrorKind Coverage for Missing Subsystems
**Priority**: P7
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/error/mod.rs`
**Depends On**: none

#### Context
`ErrorKind` in `crates/roko-core/src/error/mod.rs` (line 337) has 17 variants covering core subsystems: Store, NotFound, BodyEncode, BodyDecode, Rejected, BudgetExceeded, Io, Json, Invalid, Planning, Agent, Verify, Tool, Chain, Config, Transport, User, Timeout, Cancelled, PermissionDenied, RateLimited. Missing discriminants for ACP, MCP, deployment, and daemon errors. These subsystems currently map to `ErrorKind::Internal` (which does not even exist -- they fall through to generic handling), losing subsystem-specific retry policy capability.

The `kind()` method at line 266 is exhaustive over `RokoError` variants and maps each to an `ErrorKind`. The `is_transient()` method at line 382 uses `ErrorKind` to classify whether retry is appropriate.

#### Implementation Steps
1. Add variants to `ErrorKind`: `Acp`, `Mcp`, `Deploy`, `Daemon`, `Tui`, `Learning`.
2. Map existing `RokoError` variants to the new discriminants in `kind()`.
3. Extend `is_transient()` with classifications: MCP server crashes = transient, deployment auth failures = not transient, ACP session busy = transient.
4. Add doc comments with retry guidance per kind.
5. Update the exhaustive test at line 526 (`example()` function) to cover new kinds.

#### Verification Criteria
- [ ] All 18 crates' public errors map to a specific `ErrorKind` (not generic)
- [ ] `ErrorKind` implements `Display` with stable string labels suitable for metrics
- [ ] Exhaustive test in `error/mod.rs` covers all new kinds
- [ ] `cargo test -p roko-core` passes

---

### Task 19.4: Standardize Error Logging with Span Context
**Priority**: P1
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`
**Depends On**: none

#### Context
Errors are logged inconsistently across `roko-cli/src/`: 179 `eprintln!` calls across 22 files. The heaviest files: `commands/util.rs` (18), `commands/prd.rs` (18), `main.rs` (17), `chat.rs` (16), `commands/plan.rs` (15), `orchestrate.rs` (15), `prd.rs` (13), `auth_detect.rs` (9). Meanwhile, structured `tracing::*` calls exist in only 10 files (356 total) with no consistent span hierarchy.

Error events lack run ID, task ID, and agent ID needed to correlate failures in multi-agent runs. The workflow engine (`crates/roko-runtime/src/workflow_engine.rs`) creates `RuntimeEventEnvelope` with `run_id` but does not create tracing spans.

#### Implementation Steps
1. Define a standard span hierarchy documented in `roko-runtime`:
   ```
   roko.run[run_id] -> roko.task[task_id] -> roko.agent[agent_id] -> roko.gate[gate_name]
   ```
2. In `workflow_engine.rs`, wrap the main run loop in `tracing::info_span!("roko.run", run_id = %run_id)`.
3. In `event_loop.rs`, wrap each task dispatch in `tracing::info_span!("roko.task", task_id = %task_id)`.
4. In ACP `runner.rs`, add `roko.acp.session[session_id]` span.
5. Replace `eprintln!` with `tracing::error!` / `tracing::warn!` in the target files. Preserve TUI raw terminal output (those `eprintln!` calls are intentional for direct terminal rendering -- skip `chat_inline.rs` line 1 and similar TUI paths).
6. In serve routes middleware, ensure all error responses include `request_id`.

#### Design Guidance
Not all 179 `eprintln!` calls should be converted. Some are intentional user-facing CLI output (e.g., `eprintln!("Error: {e}")` in command handlers that format errors for the terminal). Convert error/warning paths; leave user-facing output that is part of CLI UX. Use `tracing::error!` for errors that should be machine-parseable, `eprintln!` only for formatted terminal output that bypasses the log layer.

#### Verification Criteria
- [ ] `RUST_LOG=roko=debug roko plan run` produces structured tracing output with nested spans
- [ ] Every error event in the log has `run_id` and `task_id` fields when applicable
- [ ] `eprintln!` count in `roko-cli/src/` reduced by at least 50% (from 179 to <90)
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

---

### Task 19.5: Wire RPC Error Codes Across All JSON-RPC Surfaces
**Priority**: P7
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/error/rpc.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/transport.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-github/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-slack/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-scripts/src/main.rs`
**Depends On**: Task 19.3

#### Context
`crates/roko-core/src/error/rpc.rs` defines `RpcError` with standard JSON-RPC codes (PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND, INVALID_PARAMS, INTERNAL_ERROR) and custom Roko codes (AGENT_FAILURE=-32000, GATE_FAILURE=-32001, TIMEOUT=-32002, BUDGET_EXCEEDED=-32003). But 10 files use `RpcError` with varying completeness, and ACP defines its own inline error codes (`SESSION_BUSY` as -32001 which collides with GATE_FAILURE).

#### Implementation Steps
1. Add ACP-specific error codes to `rpc.rs`: `SESSION_BUSY=-32010`, `SESSION_NOT_FOUND=-32011`, `PROMPT_TOO_LONG=-32012` (shifted to avoid collision with existing -32001 GATE_FAILURE).
2. Add MCP-specific error codes: `TOOL_NOT_FOUND=-32020`, `TOOL_TIMEOUT=-32021`, `SCRIPT_FAILED=-32022`.
3. In ACP `types.rs` and `transport.rs`, replace inline `serde_json::json!({"code": ..., ...})` with `RpcError::new(SESSION_BUSY, ...)`.
4. In each MCP crate's `main.rs` / `lib.rs`, replace inline error construction with `RpcError` conversions.
5. Document all codes in `rpc.rs` module docs.

#### Verification Criteria
- [ ] All JSON-RPC error responses across ACP and MCP go through `RpcError`
- [ ] No inline error code constants remain in ACP/MCP crates
- [ ] Error code documentation is complete in `rpc.rs`
- [ ] `cargo test --workspace` passes

---

## Section B: Logging and Observability Consistency

### Task 19.6: Unify Logging Initialization Across Entry Points
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/config.rs`
**Depends On**: none

#### Context
`roko serve`, `roko plan run`, `roko chat`, and `roko agent serve` each initialize logging differently. The CLI main.rs sets up `tracing_subscriber` with `EnvFilter`; the ACP server has its own log file configuration in `crates/roko-acp/src/config.rs`. The daemon has yet another initialization path.

#### Implementation Steps
1. Create `roko_runtime::logging::init(config: &LogConfig)` that handles all logging setup.
2. `LogConfig` reads from `roko.toml` `[logging]` section: `level`, `format` (text/json), `file` (optional path), `otel_endpoint` (optional).
3. The init function sets up: `tracing_subscriber::fmt` layer + optional JSONL file layer + optional OTel layer.
4. Replace all ad-hoc logging init in `main.rs`, `daemon.rs`, and ACP `config.rs` with the single `init()` call.
5. Ensure `RUST_LOG` env var overrides take precedence over config.

#### Verification Criteria
- [ ] All CLI entry points use `roko_runtime::logging::init()`
- [ ] `roko serve` and `roko plan run` produce identically structured log output at the same level
- [ ] `[logging] format = "json"` in roko.toml produces structured JSON log lines
- [ ] `cargo test --workspace` passes

---

### Task 19.7: Add Request-Scoped Correlation IDs
**Priority**: P5
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs`
**Depends On**: Task 19.6

#### Context
`crates/roko-serve/src/routes/middleware.rs` has no correlation ID middleware. When `roko serve` handles concurrent HTTP requests, log lines from different requests interleave without correlation. `RuntimeEventEnvelope` (line 11 of `runtime_event.rs`) has `run_id`, `seq`, `ts`, `schema_version`, `source`, and `payload` but no `correlation_id`. SSE and WebSocket routes emit events without request IDs.

#### Implementation Steps
1. Add Axum middleware that generates or extracts `X-Request-ID` header, stores in request extensions.
2. Create a tracing span `roko.http[request_id]` per request in the middleware.
3. Add optional `correlation_id: Option<String>` to `RuntimeEventEnvelope`.
4. When events are emitted from an HTTP-initiated context, populate `correlation_id` from the request extension.
5. SSE/WebSocket routes include `correlation_id` in event payloads.

#### Verification Criteria
- [ ] `curl -H "X-Request-ID: test-123" http://localhost:6677/api/status` logs contain `request_id=test-123`
- [ ] Without the header, a UUID is auto-generated
- [ ] `RuntimeEventEnvelope` emitted from HTTP routes carry the correlation ID
- [ ] `cargo test --workspace` passes

---

### Task 19.8: Emit gen_ai.* OpenTelemetry Semantic Conventions
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/otel.rs` (new file)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/Cargo.toml`
**Depends On**: Task 19.6

#### Context
No OpenTelemetry dependency exists anywhere in the workspace (zero `opentelemetry` references in any `Cargo.toml`). Runtime events go to JSONL only via `crates/roko-runtime/src/jsonl_logger.rs`. Native gen_ai.* OTel emission would give six vendor integrations (Datadog, Honeycomb, Langfuse, Phoenix, Langtrace, Grafana) for ~200 LOC.

#### Implementation Steps
1. Add `opentelemetry = "0.28"` and `opentelemetry-otlp = "0.28"` to `crates/roko-runtime/Cargo.toml` behind an `otel` feature flag.
2. Create `otel.rs` module with `init_otel(endpoint: &str, protocol: &str) -> TracerProvider`.
3. Define attribute mapping to gen_ai.* v1.37+ conventions:
   - `gen_ai.provider.name` from `ProviderKind::label()`
   - `gen_ai.operation.name` = "chat" | "execute_tool" | "retrieval"
   - `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`
   - `gen_ai.usage.cache_read.input_tokens`
   - `gen_ai.conversation.id` from session ID
4. In `ModelCallService::call()`, create a span per model call with gen_ai.* attributes.
5. In `WorkflowEngine`, create parent spans for workflow runs.
6. Add `[observability]` section to `roko.toml` config schema: `provider`, `endpoint`, `protocol`.
7. Ensure OTel export is off by default (opt-in via config or feature flag).

#### Design Guidance
Use a Cargo feature flag `otel` so the dependency is optional. This keeps the default binary lean. The `init_otel` function should be called from `roko_runtime::logging::init()` when the OTel config section is present. The `TracerProvider` should be stored in a global `OnceLock` for access from `ModelCallService`.

#### Verification Criteria
- [ ] With `[observability] endpoint = "http://localhost:4317"` and `--features otel`, spans are exported via OTLP
- [ ] Each model call produces a span with `gen_ai.provider.name` and `gen_ai.usage.*` attributes
- [ ] Without the feature or config, zero overhead (no OTel initialization)
- [ ] `cargo test --workspace` passes (OTel disabled in tests)
- [ ] `cargo check --workspace` passes without `otel` feature (no compile error)

---

### Task 19.9: Gate Results as Structured Compliance Events
**Priority**: P5
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs`
**Depends On**: Task 19.8

#### Context
Gate results are logged to JSONL but not emitted as structured events. `RuntimeEvent` (line 56 of `runtime_event.rs`) has `GateStarted`, `GatePassed`, and `GateFailed` variants but no compliance-specific event with the detail needed for SIEM/GRC integration. The gate pipeline produces structured `GateVerdict` results internally but flattens them to pass/fail strings in the event.

#### Implementation Steps
1. Add `RuntimeEvent::GateCompliance { gate_name, rung, verdict, detail, duration_ms, agent_id, task_id }` variant.
2. In `GateService::run_gates()`, after each gate execution emit a `GateCompliance` event to the event bus.
3. If OTel is configured (Task 19.8), also emit an OTel span per gate with attributes: `roko.gate.name`, `roko.gate.rung`, `roko.gate.verdict`, `roko.gate.duration_ms`.
4. Add `[gates] compliance_events = true` config flag (default false).
5. When enabled, gate events flow through EventBus to SSE/WebSocket for external consumers.
6. Update `RuntimeEvent::run_id()` and `RuntimeEvent::kind()` match arms for the new variant.

#### Verification Criteria
- [ ] With `compliance_events = true`, each gate execution produces a `GateCompliance` RuntimeEvent
- [ ] Events are visible via SSE at `/api/events`
- [ ] Without the flag, no extra event overhead
- [ ] Gate events include the full verdict (pass/fail/skip) with detail

---

## Section C: StateHub and Event Bus Improvements

### Task 19.10: Unify DashboardEvent and RuntimeEvent Types
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/state_hub.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/run.rs`
**Depends On**: none

#### Context
Two parallel event systems exist. `DashboardEvent` (23 files, defined in `crates/roko-core/src/dashboard_snapshot.rs` line 23) has ~15 variants driving the TUI via StateHub. `RuntimeEvent` (34 files, defined in `crates/roko-core/src/runtime_event.rs` line 56) has 12 variants driving the workflow engine's observers. They overlap on agent start/complete, gate results, and phase transitions but use different types with different field sets.

For example, agent completion is `DashboardEvent::AgentSpawned { agent_id, role, model }` vs `RuntimeEvent::AgentSpawned { run_id, agent_id, role, model }`. Gate results are `DashboardEvent::GateResult { plan_id, task_id, gate, passed }` vs `RuntimeEvent::GatePassed { run_id, gate_name, duration_ms }`. The same occurrence is emitted twice through different channels.

#### Implementation Steps
1. Add `impl From<RuntimeEvent> for Option<DashboardEvent>` that maps runtime events to their dashboard equivalents. Not all runtime events have dashboard counterparts (e.g., `FeedbackRecorded` has no TUI equivalent), so the conversion returns `Option`.
2. Add missing variants to `RuntimeEvent` for events that only exist in `DashboardEvent`: `EfficiencyMetric`, `Diagnosis`, `ExperimentWinnersUpdated`, `CFactorTrendUpdated`, `EpisodeRecorded`.
3. Modify `StateHub::publish()` to accept `RuntimeEvent` and auto-convert via the `From` impl.
4. Keep `DashboardEvent` as the TUI-facing type but derive it from `RuntimeEvent`.
5. Remove duplicate event emission sites where both event types are emitted for the same occurrence. Search for patterns where `state_hub.push_dashboard_event(...)` and `event_bus.emit(RuntimeEvent::...)` appear near each other for the same logical event.

#### Design Guidance
This is a wide-reaching change that touches 23+ files. Implement the `From` conversion first, then gradually migrate emission sites. Use a two-phase approach: Phase 1 adds the conversion and keeps both emission paths. Phase 2 removes the `DashboardEvent` emission from sites that now emit `RuntimeEvent`. This allows incremental verification.

#### Verification Criteria
- [ ] Single event emission point per occurrence (not two parallel emits)
- [ ] TUI still receives `DashboardEvent` via `watch` channel
- [ ] SSE/WebSocket still receive events via broadcast channel
- [ ] `cargo test --workspace` passes
- [ ] Event emission count does not increase (no regression in event volume)

---

### Task 19.11: Add Event Bus Backpressure and Overflow Handling
**Priority**: P6
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/state_hub.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`
**Depends On**: Task 19.10

#### Context
`EventBus` in `crates/roko-runtime/src/event_bus.rs` uses `tokio::sync::broadcast` which silently drops events when consumers lag. The bus has a bounded `VecDeque` ring (replay buffer) but the broadcast channel capacity is not configurable. During fast multi-agent runs, TUI or SSE clients can miss events with no indication. The `Envelope<E>` wrapper at line 63 includes `seq` for gap detection but no overflow tracking.

#### Implementation Steps
1. In `EventBus`, add an overflow counter: `Arc<AtomicU64>` tracking total dropped events.
2. Add `EventBus::overflow_count() -> u64` public method.
3. In `StateHub`, log a warning when overflow count increases: `tracing::warn!(overflow = count, "event bus overflow: {count} events dropped")`.
4. Add a `DashboardEvent::Overflow { dropped_count }` variant so the TUI can display a "missed N events" indicator.
5. Increase the default broadcast channel capacity to 2048 for multi-agent runs.
6. Add `[dashboard] event_buffer_size = 2048` config option.

#### Verification Criteria
- [ ] Overflow events are tracked and logged
- [ ] TUI displays "N events missed" when overflow occurs
- [ ] Default capacity handles 10 concurrent agents at 10 events/second without overflow
- [ ] `cargo test --workspace` passes

---

### Task 19.12: Add Event Bus Filtering and Subscription Topics
**Priority**: P6
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs`
**Depends On**: Task 19.10

#### Context
All consumers receive all events. The TUI does not need gate compliance events. The SSE learning endpoint does not need agent output chunks. Broadcasting everything wastes CPU on serialization/deserialization for events consumers discard.

#### Implementation Steps
1. Add `EventTopic` enum: `Agent`, `Gate`, `Learning`, `System`, `Compliance`, `All`.
2. Add `RuntimeEvent::topic() -> EventTopic` method that classifies each variant.
3. Add `EventBus::subscribe_filtered(topics: &[EventTopic]) -> FilteredReceiver` that only delivers matching events.
4. Keep `EventBus::subscribe()` as the unfiltered path for backward compatibility.
5. Migrate SSE route to use filtered subscription (exclude `Agent` output chunks).
6. Migrate TUI bridge to use filtered subscription (exclude `Compliance` events).

#### Verification Criteria
- [ ] Filtered subscribers only receive events matching their topic set
- [ ] Unfiltered subscribers still receive everything
- [ ] No performance regression for the unfiltered path
- [ ] `cargo test --workspace` passes

---

### Task 19.13: Make StateHub Snapshot Serializable for REST API
**Priority**: P7
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
**Depends On**: none

#### Context
`DashboardSnapshot` is served via the REST API at `/api/status` (in `crates/roko-serve/src/routes/status/health.rs`) but the serialization is ad-hoc: some fields are skipped, some are transformed inline in the route handler. The snapshot already has `Serialize + Deserialize` on `DashboardEvent` (line 23 of `dashboard_snapshot.rs`) but the `DashboardSnapshot` struct itself may need verification that all fields are serializable.

#### Implementation Steps
1. Ensure all fields in `DashboardSnapshot` implement `Serialize + Deserialize`.
2. Add `schema_version: u8` field (start at 1) for forward compatibility.
3. Add `generated_at: DateTime<Utc>` field.
4. In the `/api/status` route, return `Json(snapshot)` directly instead of constructing an ad-hoc response.
5. Document the snapshot schema in `DashboardSnapshot` doc comments.

#### Verification Criteria
- [ ] `GET /api/status` returns the full `DashboardSnapshot` with `schema_version: 1`
- [ ] `serde_json::to_string(&snapshot)` round-trips cleanly
- [ ] Existing TUI rendering is unaffected
- [ ] `cargo test --workspace` passes

---

## Section D: Cancellation and Graceful Shutdown

### Task 19.14: Propagate CancelToken Through All Dispatch Paths
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run_inline.rs`
**Depends On**: none

#### Context
`CancelToken` from `crates/roko-runtime/src/cancel.rs` supports hierarchical cancellation (parent cancels children). It is used in 20 files. However, key dispatch paths lack it:

- `crates/roko-cli/src/chat_inline.rs` -- no CancelToken; Ctrl-C during chat leaves orphan agent processes.
- `crates/roko-cli/src/run.rs` -- creates its own ad-hoc cancellation via `tokio::signal::ctrl_c()`.
- `crates/roko-cli/src/run_inline.rs` -- similar ad-hoc cancellation.

The WorkflowEngine (`crates/roko-runtime/src/workflow_engine.rs`) does accept a `CancelToken` in its run method. The ACP server (`crates/roko-acp/src/session.rs`) creates per-session `CancelToken`s. The plan runner (`crates/roko-cli/src/runner/event_loop.rs`) uses `CancelToken`. The gap is the CLI entry points that should create a root token.

#### Implementation Steps
1. In `main.rs`, create a root `CancelToken` and wire SIGINT/SIGTERM to call `root.cancel()`.
2. Pass `root.child()` to every dispatch entry point: `run()`, `chat_inline()`.
3. In each dispatch function, check `cancel.is_cancelled()` before each major phase (agent spawn, gate run, merge).
4. Replace any `tokio::signal::ctrl_c().await` with `cancel.cancelled().await` in select loops.
5. Ensure the ACP server's per-session `CancelToken` is a child of the root token.

#### Design Guidance
The root `CancelToken` should be created once in `main()` and passed as an argument to command dispatch functions. Do not store it in a static or global; the hierarchical parent-child relationship is the correct pattern. When a child is cancelled, it does not cancel the parent, but when the root is cancelled, all children are cancelled.

#### Verification Criteria
- [ ] Ctrl-C in `roko run "hello"` cancels within 2 seconds (no orphan agent processes)
- [ ] Ctrl-C in `roko chat` cancels the current agent call and returns to the prompt
- [ ] Ctrl-C in `roko plan run` cancels the current task, persists state, and exits
- [ ] `cancel.is_cancelled()` is checked in all dispatch paths
- [ ] `cargo test --workspace` passes

---

### Task 19.15: Wire GracefulShutdown into roko serve
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`
**Depends On**: none

#### Context
`GracefulShutdown` in `crates/roko-core/src/shutdown.rs` is fully implemented: hook registration, concurrent drain with `join_all`, hard deadline, `ShutdownReport` with drained/timed-out counts. But it is imported by only 1 file in the codebase. `roko-serve` in `crates/roko-serve/src/lib.rs` uses ad-hoc `force_shutdown` signaling (found in `daemon.rs`, `agent_serve.rs`). WebSocket connections in `crates/roko-serve/src/routes/` are dropped without drain. SSE streams cut off mid-event.

#### Implementation Steps
1. Add `GracefulShutdown` to `AppState` in `crates/roko-serve/src/state.rs` alongside the existing `CancelToken`.
2. Create `GracefulShutdown::with_deadline(Duration::from_secs(5))` during server startup.
3. Register shutdown hooks for each subsystem:
   - `"ws-drain"`: send `subscription_ended` to all WebSocket clients, wait up to 1s.
   - `"sse-drain"`: send final SSE event to all clients, close connections.
   - `"state-flush"`: flush StateHub snapshot to disk.
   - `"metrics-flush"`: flush any buffered OTel/metrics data.
4. On SIGTERM/SIGINT, call `shutdown.drain().await` instead of the current `force_shutdown` flag.
5. Log the `ShutdownReport` (drained_hooks, timed_out_hooks, elapsed_ms).

#### Verification Criteria
- [ ] `kill -TERM <roko-serve-pid>` produces a `ShutdownReport` in the log
- [ ] WebSocket clients receive `subscription_ended` before disconnection
- [ ] SSE streams receive a final event before closing
- [ ] Shutdown completes within the 5-second deadline
- [ ] No orphan background tasks after shutdown

---

### Task 19.16: Wire GracefulShutdown into roko daemon
**Priority**: P8
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs`
**Depends On**: Task 19.14, Task 19.15

#### Context
`roko daemon stop` sends SIGTERM but does not coordinate with the daemon's internal subsystems. The daemon may be mid-plan-run when killed, leaving worktrees in dirty state, agent processes orphaned, and learning files unflushed. `crates/roko-cli/src/daemon.rs` references `force_shutdown` (one of the 9 files matching that pattern).

#### Implementation Steps
1. In daemon main loop, create `GracefulShutdown::with_deadline(Duration::from_secs(10))`.
2. Register hooks:
   - `"plan-runner"`: signal CancelToken, wait for current task to checkpoint.
   - `"agent-processes"`: call `ProcessSupervisor::shutdown_all()` with 5-second timeout.
   - `"learning-flush"`: flush episode logger, cascade router, efficiency writer.
   - `"worktree-cleanup"`: ensure all worktrees have their latest changes committed.
3. Wire SIGTERM handler to `shutdown.drain().await`.
4. Add `roko daemon stop --timeout <seconds>` flag to override the default deadline.
5. After drain, log a summary of what was flushed and what timed out.

#### Verification Criteria
- [ ] `roko daemon stop` drains all subsystems before exiting
- [ ] Agent processes are killed if they do not exit within 5 seconds
- [ ] Learning files are flushed to disk before exit
- [ ] `roko daemon stop --timeout 2` uses a 2-second deadline
- [ ] `cargo test --workspace` passes

---

### Task 19.17: Add Shutdown Hooks for ACP Server
**Priority**: P8
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
**Depends On**: Task 19.14

#### Context
The ACP server runs as a subprocess of editors (Zed, JetBrains). When the editor sends EOF on stdin, the ACP server should drain active sessions, flush episodes, and persist cascade router state. Currently it exits immediately on EOF. `crates/roko-acp/src/session.rs` already creates per-session `CancelToken`s but there is no coordinated shutdown sequence.

#### Implementation Steps
1. In `run_acp_server()` handler, when the transport returns `None` (EOF), enter shutdown.
2. Create `GracefulShutdown::with_deadline(Duration::from_secs(3))`.
3. Register hooks:
   - `"active-sessions"`: cancel all active session CancelTokens, wait for prompts to complete.
   - `"episode-flush"`: flush any buffered episode data to `.roko/episodes.jsonl`.
   - `"router-save"`: persist cascade router to `.roko/learn/cascade-router.json`.
   - `"session-persist"`: save all session state to disk for later `session/load`.
4. After drain, exit cleanly with code 0.

#### Verification Criteria
- [ ] ACP server flushes episodes on editor close
- [ ] Active prompts are cancelled (not left running as orphan processes)
- [ ] Session state is persisted for later resume
- [ ] Shutdown completes within 3 seconds

---

### Task 19.18: Implement Force-Kill Escalation for Agent Processes
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs`
**Depends On**: none

#### Context
`ProcessSupervisor` at line 839 of `crates/roko-runtime/src/process.rs` has `shutdown_all()` (line 951) and `kill_all()` (line 1033) but no SIGTERM-to-SIGKILL escalation. The dogfood session (documented in `tmp/dogfood/CONTEXT.md`) revealed agents surviving `force_shutdown` because SIGTERM was ignored by the subprocess tree. The `Drop` impl at line 1248 force-kills on drop, but this is a last resort.

`crates/roko-agent/src/process/kill.rs` has kill logic for individual processes. `crates/roko-agent/src/process/mod.rs` and `registry.rs` also reference force_shutdown/SIGKILL. These need to be coordinated with the supervisor's escalation.

#### Implementation Steps
1. Add `ProcessSupervisor::shutdown_with_escalation(timeout: Duration)`:
   - Phase 1 (0 to timeout/2): SIGTERM to process group.
   - Phase 2 (timeout/2 to timeout): SIGKILL to process group.
   - Phase 3 (after timeout): kill each PID individually if group kill failed.
2. Use `nix::sys::signal::killpg()` for process group signaling (already used in the codebase).
3. Track process group IDs at spawn time via `process_group(0)` in the `Command` builder.
4. Log each escalation step: `tracing::warn!("agent {} did not respond to SIGTERM, escalating to SIGKILL", pid)`.
5. After kill, verify process is dead with `waitpid(WNOHANG)`.

#### Verification Criteria
- [ ] Agent processes that ignore SIGTERM are killed within `timeout` seconds
- [ ] Process group kill catches child processes spawned by the agent
- [ ] Log output shows the escalation progression
- [ ] No zombie processes remain after shutdown
- [ ] `cargo test -p roko-runtime` passes

---

## Section E: Resource Cleanup

### Task 19.19: Implement Worktree Cleanup Policy
**Priority**: P4
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/worktree.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (add subcommand)
**Depends On**: none

#### Context
Per MEMORY.md and project rules, worktrees are never deleted automatically. The glob shows 20+ worktrees under `.claude/worktrees/` and `.roko/worktrees/`. There is no visibility into worktree age, size, or relationship to completed plans. `WorktreeManager` at `crates/roko-orchestrator/src/worktree.rs` has `create_for_plan()`, `remove()`, `touch()`, `reclaim_idle()`, `health()`, `clear_stale_locks()`, `prune()` but no status reporting.

**CRITICAL**: Never auto-delete worktrees. This is a hard project rule. The GC command must always prompt for confirmation.

#### Implementation Steps
1. Add `WorktreeManager::status() -> Vec<WorktreeInfo>` that reports: path, branch, age, disk size, plan association, clean/dirty.
2. Add `roko util worktrees list` subcommand that displays the status table.
3. Add `roko util worktrees gc --older-than 30d --dry-run` that identifies candidates for cleanup.
4. Without `--dry-run`, prompt for confirmation (never auto-delete).
5. Add `roko util worktrees archive <path>` that creates a tarball of the worktree before removal.
6. Track worktree creation time in `.roko/state/worktrees.json`.

#### Verification Criteria
- [ ] `roko util worktrees list` shows all worktrees with age and size
- [ ] `roko util worktrees gc --older-than 30d --dry-run` lists candidates without deleting
- [ ] Actual deletion requires explicit user confirmation (never auto-delete)
- [ ] Worktree registry persists across restarts
- [ ] `cargo test --workspace` passes

---

### Task 19.20: Implement Temp File Cleanup on Startup
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs`
**Depends On**: none

#### Context
Failed runs leave temporary files: partial executor snapshots, lock files, MCP config fragments, and partial JSONL entries (truncated mid-line). These accumulate in `.roko/` and can cause issues on restart (e.g., stale lock files blocking new runs, corrupt JSONL causing parse failures on cascade router load).

#### Implementation Steps
1. Create `roko_runtime::cleanup::startup_cleanup(roko_dir: &Path)`:
   - Remove stale `.lock` files older than 1 hour.
   - Remove `.tmp` files (partial atomic writes that never completed rename).
   - Truncate corrupt JSONL files at the last valid line boundary.
   - Remove empty directories in `.roko/state/tasks/`.
2. Call `startup_cleanup()` at the beginning of `plan run`, `serve`, and `daemon start`.
3. Log each cleanup action: `tracing::info!("cleaned up stale lock: {}", path.display())`.
4. Add `--no-cleanup` flag to skip startup cleanup for debugging.

#### Design Guidance
The JSONL truncation must be careful: read the file line by line, find the last line that parses as valid JSON, and truncate the file at that point. Do not delete the file. Use atomic write (write to `.tmp`, rename) for the truncated output to avoid data loss if the truncation itself is interrupted.

#### Verification Criteria
- [ ] Stale `.lock` files are removed on startup
- [ ] Corrupt JSONL files are truncated to valid state (not deleted)
- [ ] `--no-cleanup` skips all cleanup
- [ ] `cargo test --workspace` passes

---

### Task 19.21: Add Process Orphan Detection
**Priority**: P4
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (add subcommand)
**Depends On**: Task 19.18

#### Context
When `roko plan run` is killed (SIGKILL, OOM, power failure), spawned agent processes may survive as orphans. There is no mechanism to detect and clean up orphans from a previous run on restart. `ProcessSupervisor` at line 839 of `process.rs` tracks handles in memory but does not persist PIDs to disk.

#### Implementation Steps
1. On agent spawn, write PID to `.roko/state/pids/<run_id>/<task_id>.pid`.
2. On clean agent exit, remove the PID file.
3. On `plan run` startup, scan `.roko/state/pids/` for PID files from previous runs.
4. For each PID file, check if the process is still running (`kill(pid, 0)` or equivalent).
5. If running, log a warning: `tracing::warn!("orphan process {} from previous run {} still running", pid, run_id)`.
6. Add `roko util cleanup-orphans` subcommand that kills orphaned processes.
7. In non-interactive mode (daemon, CI), auto-kill orphans from the same run directory.

#### Verification Criteria
- [ ] PID files are created on agent spawn and removed on clean exit
- [ ] `roko plan run` warns about orphans from previous runs
- [ ] `roko util cleanup-orphans` kills orphaned agent processes
- [ ] `cargo test --workspace` passes

---

### Task 19.22: Periodic Learning File Compaction
**Priority**: P6
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
**Depends On**: none

#### Context
Append-only JSONL files grow without bound: `episodes.jsonl`, `efficiency.jsonl`, `costs.jsonl`, `routing.jsonl`. After weeks of use, these files can reach hundreds of MB. The cascade router loads and parses the full `cascade-router.json` on startup, slowing initialization. No compaction or retention policy exists.

#### Implementation Steps
1. Add `compact_episodes(path: &Path, retention_days: u32)` that:
   - Reads the JSONL file line by line.
   - Removes entries older than `retention_days`.
   - Computes aggregate statistics for removed entries (total tokens, total cost, pass rate by model).
   - Writes a summary entry and the retained entries to a `.tmp` file.
   - Atomic rename to replace the original.
2. Add `compact_cascade_router(path: &Path)` that prunes observations older than the confidence window.
3. Add `roko learn compact --retention-days 30` subcommand.
4. Add optional auto-compaction on startup when file exceeds 50MB (configurable via `[learning] max_file_mb = 50`).
5. Always preserve the aggregate summary so historical trends are not lost.

#### Verification Criteria
- [ ] `roko learn compact --retention-days 30` reduces file size by removing old entries
- [ ] Aggregate statistics are preserved in a summary entry
- [ ] CascadeRouter observations are pruned without losing learned routing weights
- [ ] `cargo test --workspace` passes

---

## Section F: API Versioning and Config Validation

### Task 19.23: Add API Version Header to roko serve
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/openapi.rs`
**Depends On**: none

#### Context
The HTTP control plane has ~85 routes with zero versioning. `crates/roko-serve/src/routes/middleware.rs` has no `X-Roko-API-Version` header, no `Accept-Version` handling, no version negotiation. Breaking changes to response schemas silently break dashboard clients, CLI callers, and external integrations.

#### Implementation Steps
1. Add `X-Roko-API-Version: 1` response header to all routes via Axum middleware layer.
2. Add `Accept-Version: 1` request header support (optional; defaults to latest).
3. In the OpenAPI spec (`openapi.rs`), set `info.version` to `"1.0.0"`.
4. Add version negotiation: if client sends `Accept-Version: 999` and server only supports 1, return `406 Not Acceptable`.
5. Document versioning policy: breaking changes require a new version; additive changes do not.

#### Verification Criteria
- [ ] All responses include `X-Roko-API-Version: 1`
- [ ] `Accept-Version: 999` returns 406
- [ ] OpenAPI spec has version `1.0.0`
- [ ] Existing clients without version headers work unchanged
- [ ] `cargo test --workspace` passes

---

### Task 19.24: Add Schema Version to All Persisted JSON Files
**Priority**: P9
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/contextual_bandit.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/section_outcome.rs`
**Depends On**: none

#### Context
34 files reference `schema_version` but coverage is partial. `RuntimeEventEnvelope` has it (line 15 of `runtime_event.rs`). `RuntimeSnapshot` has it (`crates/roko-orchestrator/src/runtime_snapshot.rs`). But persisted learning files (`cascade-router.json`, `gate-thresholds.json`, `experiments.json`) may lack it. The `crates/roko-cli/src/snapshot_migrate.rs` file exists, suggesting migration infrastructure is partially built.

#### Implementation Steps
1. For each persisted JSON file type, ensure the root object has `"schema_version": N`.
2. On load, check the schema version. If unknown (newer than expected), log a warning and attempt best-effort parsing.
3. If the version is older, apply migration functions: `migrate_v0_to_v1(data: Value) -> Value` per file type.
4. Add `roko config migrate` subcommand that migrates all persisted files to the latest schema.
5. Document schema changes in each file's module docs.

#### Verification Criteria
- [ ] All persisted JSON files have `schema_version` field
- [ ] Older schemas are auto-migrated on load
- [ ] Newer schemas produce a warning, not a crash
- [ ] `roko config migrate` succeeds on a workspace with v0 files
- [ ] `cargo test --workspace` passes

---

### Task 19.25: Add roko.toml Config Schema Validation
**Priority**: P4
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`
**Depends On**: none

#### Context
`roko.toml` accepts any keys without validation. The `serde_ignored` crate is not in any `Cargo.toml` in the workspace. Typos like `defalt_model` instead of `default_model` are silently ignored. The config schema divergence between `[[gate]]` (written by `roko init`) and `[gates]` (read by `roko plan run`) documented in `06-IMPLEMENTATION-PLANS.md` Plan 5 is one symptom. `crates/roko-core/src/config/compat.rs` exists (references `schema_version`), suggesting some migration infrastructure is present.

#### Implementation Steps
1. Add `serde_ignored` dependency to `roko-core/Cargo.toml`.
2. After deserializing `RokoConfig` from TOML, collect unknown keys using `serde_ignored::deserialize()`.
3. For each unknown key, compute Levenshtein distance to known keys.
4. If distance <= 2, suggest the correct key: `warning: unknown key 'defalt_model', did you mean 'default_model'?`.
5. If distance > 2, warn: `warning: unknown key 'xyz' in section [agent]`.
6. Add `roko config validate` subcommand that runs validation and reports all issues.
7. On `roko plan run`, validate config and warn (do not fail) for unknown keys.
8. Accept both `[[gate]]` and `[gates]` formats per Plan 5, with deprecation warning for `[[gate]]`.

#### Verification Criteria
- [ ] `roko config validate` detects typos and suggests corrections
- [ ] `defalt_model` in roko.toml produces: `warning: unknown key 'defalt_model', did you mean 'default_model'?`
- [ ] Both gate config formats are accepted with deprecation warning
- [ ] Validation is non-blocking (warns, does not fail)
- [ ] `cargo test --workspace` passes

---

### Task 19.26: Add Deprecation Warnings for Config Migrations
**Priority**: P9
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: Task 19.25

#### Context
Config format changes need deprecation warnings so users know to update. Currently, old formats either silently work (confusing) or silently break (worse). The `[[gate]]` to `[gates]` migration is the first concrete case.

#### Implementation Steps
1. Add `DeprecationWarning { field: String, message: String, since_version: String, removal_version: Option<String> }` struct.
2. During config parsing, collect deprecation warnings into `Vec<DeprecationWarning>`.
3. Return warnings alongside the parsed config: `fn load_config() -> Result<(RokoConfig, Vec<DeprecationWarning>)>`.
4. In CLI entry points, display warnings with color: yellow for deprecations, red for upcoming removals.
5. Add `#[deprecated_field(since = "0.5", use_instead = "gates")]` attribute macro for config struct fields (or use a simpler HashMap-based approach).

#### Verification Criteria
- [ ] `[[gate]]` format produces a deprecation warning naming `[gates]` as the replacement
- [ ] Warnings include the version where the old format will be removed
- [ ] `roko config show` displays active deprecation warnings
- [ ] Warnings do not prevent execution
- [ ] `cargo test --workspace` passes

---

## Section G: Deployment Improvements

### Task 19.27: Improve Dockerfile with Multi-Stage Caching
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Dockerfile`
**Depends On**: none

#### Context
The current Dockerfile is minimal: 23 lines, single builder stage that copies everything (`COPY . .`) before building, invalidating Docker layer cache on every source change. It runs as root in the runtime stage. No health check. The `.dockerignore` exists and is comprehensive (67 lines), which is good.

Current Dockerfile:
```dockerfile
FROM rust:1.91-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p roko-cli
FROM debian:bookworm-slim AS runtime
# ... apt-get, COPY binary, VOLUME, ENTRYPOINT
```

#### Implementation Steps
1. Add a dependency-cache stage that copies only `Cargo.toml` and `Cargo.lock` first:
   ```dockerfile
   FROM rust:1.91-bookworm AS deps
   WORKDIR /app
   COPY Cargo.toml Cargo.lock ./
   COPY crates/*/Cargo.toml ./crates/
   RUN find crates -name Cargo.toml -exec sh -c 'mkdir -p $(dirname {})/src && echo "" > $(dirname {})/src/lib.rs' \;
   RUN cargo build --release -p roko-cli 2>/dev/null || true
   ```
2. Add the source copy stage that benefits from cached dependencies:
   ```dockerfile
   FROM deps AS builder
   COPY . .
   RUN cargo build --release -p roko-cli
   ```
3. Add non-root user to the runtime stage:
   ```dockerfile
   RUN groupadd -r roko && useradd -r -g roko roko
   USER roko
   ```
4. Add health check: `HEALTHCHECK --interval=30s CMD curl -f http://localhost:6677/api/health || exit 1`.
5. Verify `.dockerignore` covers `target/`, `.roko/`, `tmp/`, `.git/` (it does -- confirmed).

#### Verification Criteria
- [ ] Source-only changes rebuild in < 3 minutes (dependency cache hit)
- [ ] Runtime image runs as non-root user `roko`
- [ ] Health check passes within 30 seconds of container start
- [ ] `docker build .` succeeds
- [ ] Image size is reasonable (< 200MB for runtime stage)

---

### Task 19.28: Add Docker Compose for Development
**Priority**: P10
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/docker/docker-compose.yml` (exists, may need update)
- `/Users/will/dev/nunchi/roko/roko/docker/docker-compose.dev.yml` (new file)
**Depends On**: Task 19.27

#### Context
A `docker/docker-compose.yml` already exists in the workspace. No dev-specific compose file exists. No OTel collector or Jaeger service is configured for local trace viewing.

#### Implementation Steps
1. Review existing `docker/docker-compose.yml` and extend with:
   - `otel-collector`: `otel/opentelemetry-collector:latest`, receives OTLP on 4317.
   - `jaeger`: `jaegertracing/all-in-one:latest`, trace visualization on 16686.
2. Create `docker/docker-compose.dev.yml` with:
   - Hot reload via mounted source volume.
   - Debug logging level.
   - Exposed debug ports.
3. Add `scripts/dev-up.sh` that runs `docker compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up`.
4. Ensure OTel collector forwards to Jaeger for local trace viewing.

#### Verification Criteria
- [ ] `docker compose -f docker/docker-compose.yml up` starts roko-serve
- [ ] `http://localhost:6677/api/health` returns healthy
- [ ] `http://localhost:16686` shows Jaeger UI (when OTel collector is configured)
- [ ] `docker compose down` stops all services cleanly

---

### Task 19.29: Improve Railway Deployment with Auth Provisioning
**Priority**: P10
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/deploy.rs` (or equivalent deploy module)
**Depends On**: none

#### Context
`roko deploy railway` exists but does not auto-provision auth. The ground truth document (section 7.4) notes that `dangerously_skip_permissions: true` is always set in plan mode (line 394 of `plan.rs`). Cloud deployments are unauthenticated by default. Plan 3.3 in `06-IMPLEMENTATION-PLANS.md` specifies the exact fix.

#### Implementation Steps
1. Auto-generate a 32-byte hex API key on Railway deploy, set as `ROKO_API_KEY` env var.
2. Set `api_auth.enabled = true` in the deployed config.
3. Print the API key to stdout once: "Save this API key -- it will not be shown again".
4. Configure Railway health check to `GET /api/health` with 30-second interval.
5. Set Railway memory limit to 2GB.
6. Add `--region` flag for Railway region selection.
7. Validate `RAILWAY_TOKEN` is set before attempting deployment.

#### Verification Criteria
- [ ] `roko deploy railway` prints an API key
- [ ] Deployed service rejects unauthenticated requests (returns 401)
- [ ] Health check is configured and passing
- [ ] Missing `RAILWAY_TOKEN` produces a clear error message

---

### Task 19.30: Add roko deploy docker Subcommand
**Priority**: P10
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/deploy.rs`
**Depends On**: Task 19.27

#### Context
Users who want to self-host need to manually build the Docker image, configure volumes, set environment variables, and manage the container. A `roko deploy docker` command should generate a ready-to-run configuration.

#### Implementation Steps
1. Add `roko deploy docker` subcommand that:
   - Builds the Docker image using the workspace Dockerfile.
   - Generates `docker-compose.prod.yml` with:
     - roko-serve container with configured ports, volumes, and env vars.
     - Auto-generated API key in `.env` file.
     - Restart policy: `unless-stopped`.
     - Log driver configuration: `json-file` with max-size and max-file.
   - Prints instructions: "Run: docker compose -f docker-compose.prod.yml up -d".
2. Add `--port` flag (default 6677).
3. Add `--data-dir` flag for `.roko/` volume mount location.
4. Add `--tls` flag that generates a self-signed cert and configures HTTPS.

#### Verification Criteria
- [ ] `roko deploy docker` produces a `docker-compose.prod.yml` ready to run
- [ ] The generated compose file includes API auth, health check, restart policy, and log limits
- [ ] `docker compose -f docker-compose.prod.yml up -d` starts a working roko-serve instance
- [ ] `--tls` flag configures HTTPS with a self-signed certificate

---

### Task 19.31: Add Container Health Monitoring Endpoint
**Priority**: P10
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
**Depends On**: Task 19.11

#### Context
The `/api/health` endpoint exists in `crates/roko-serve/src/routes/status/health.rs` and returns 200 OK, but does not report subsystem health. In containerized deployments, operators need to know event bus overflow state, learning subsystem health, filesystem space, and active agent count.

#### Implementation Steps
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
2. Return HTTP 200 if all subsystems are healthy, 503 if any are degraded.
3. Add `/api/health/ready` (readiness probe) that returns 200 only when the server is fully initialized.
4. Add `/api/health/live` (liveness probe) that returns 200 as long as the process is running.
5. Configure Kubernetes/Docker health check to use `/api/health/ready`.

#### Verification Criteria
- [ ] `/api/health` returns subsystem-level health information
- [ ] `/api/health/ready` returns 503 during initialization, 200 when ready
- [ ] `/api/health/live` always returns 200
- [ ] Subsystem health reflects actual state (not hardcoded)
- [ ] `cargo test --workspace` passes

---

## Priority Matrix

| Priority | Tasks | Impact | Effort | Parallel? |
|---|---|---|---|---|
| **P0** | 19.14 (CancelToken), 19.18 (Force-kill) | Critical | 8 hours | Yes (independent) |
| **P1** | 19.4 (Span context), 19.6 (Logging init), 19.15 (Serve shutdown) | High | 14 hours | Yes (all independent) |
| **P2** | 19.1 (anyhow audit), 19.10 (Event unification), 19.20 (Temp cleanup) | High | 17 hours | Yes (all independent) |
| **P3** | 19.8 (OTel), 19.23 (API version), 19.27 (Dockerfile) | High | 12 hours | Yes (all independent) |
| **P4** | 19.19 (Worktree status), 19.21 (Orphan detection), 19.25 (Config validation) | Medium | 12 hours | Yes (all independent) |
| **P5** | 19.2 (Error context), 19.7 (Correlation IDs), 19.9 (Compliance events) | Medium | 10 hours | After P1 deps |
| **P6** | 19.11 (Backpressure), 19.12 (Filtering), 19.22 (Compaction) | Medium | 11 hours | After 19.10 |
| **P7** | 19.3 (ErrorKind), 19.5 (RPC codes), 19.13 (Snapshot serial.) | Low | 7 hours | Yes (all independent) |
| **P8** | 19.16 (Daemon shutdown), 19.17 (ACP shutdown) | Medium | 6 hours | After 19.14, 19.15 |
| **P9** | 19.24 (Schema versions), 19.26 (Deprecation warnings) | Low | 7 hours | After 19.25 |
| **P10** | 19.28 (Compose), 19.29 (Railway), 19.30 (Docker deploy), 19.31 (Health) | Medium | 12 hours | After 19.27 |

## Dependency Graph

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
19.27 ──> 19.30  (Docker deploy needs Dockerfile)
19.11 ──> 19.31  (health endpoint needs overflow tracking)
```

## Fast Track (1-2 hours each, do immediately)

| Fix | Time | Task |
|---|---|---|
| Propagate CancelToken to chat_inline.rs and run.rs | 2 hours | 19.14 (partial) |
| Replace eprintln! with tracing in top 5 files | 2 hours | 19.4 (partial) |
| Add API version header middleware | 30 min | 19.23 |
| Startup cleanup for stale .lock files | 1 hour | 19.20 (partial) |
| Add non-root user and health check to Dockerfile | 30 min | 19.27 (partial) |

## Total Estimated Effort

31 tasks. At 2-8 hours per task: ~116 hours total.
With 60% parallelism across independent tasks: **10-15 working days**.
Fast-track items (5 partial tasks) can land in 1 day.
