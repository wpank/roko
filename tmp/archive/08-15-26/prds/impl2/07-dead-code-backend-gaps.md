# PRD 07: Dead code cleanup and agent backend gaps

**Branch:** `wp-demo`
**Status:** Draft
**Date:** 2026-04-22

---

## Scope

Nine gaps covering dead infrastructure, unconnected backends, and duplicated
code paths. These are structural hygiene issues: each gap represents code that
either exists but is never called, or is connected via a hardcoded bypass that
skirts the intended architecture.

| Gap | Area | Severity | Net change |
|-----|------|----------|-----------|
| A | MultiAgentPool dead in dispatch path | High — pool infrastructure built and tested but zero agents ever pooled | Wire or remove; recommend wire |
| B | 10 dead modules in roko-orchestrator | Medium — dead code confuses contributors | Document + wire audit_chain; remove or gate the rest |
| C | CustodyLogger writer never called | High — custody CLI reports nothing because no records are ever written | Wire into ToolDispatcher or SafetyLayer |
| D | Dead scaffolding in roko-serve | Low — unused types, no runtime impact | Remove dead types; decide on truth_map |
| E | Ollama bypasses provider system | Medium — hardcoded branch skips adapters, no MCP support | Add ProviderKind::Ollama + OllamaAdapter |
| F | Perplexity tool loop backend not wired | Medium — PerplexityToolLoopBackend implements LlmBackend but create_tool_loop_backend rejects it | Wire the existing implementation |
| G | Codex JSON-RPC/WebSocket protocol not implemented | Low — HTTP fallback works, full protocol deferred | Document accurately; file follow-up |
| H | Scheduler code duplication | Medium — two independent cron startup paths produce identical behavior | Consolidate to one |
| I | Provider health not wired in roko-serve dispatch | High — health tracking works in CLI but serve dispatch ignores it | Wire route_with_health and record_success/failure |

No new crates. No new external dependencies.

**What is NOT in scope here:**
- Knowledge-informed model selection in CascadeRouter (separate concern)
- Chain runtime integration (covered by PRD 01)
- Config unification (covered by PRD 02)
- Gate rung oracle wiring (covered by PRD 04)

---

## Implementation checklist

### Gap A: Wire MultiAgentPool into the dispatch path (or remove it)

**Why it matters:** `PlanRunner` in `crates/roko-cli/src/orchestrate.rs` holds
`agent_pool: MultiAgentPool` (line 3041). The pool is initialized at
construction (lines 4723, 4881, 5041) with
`MultiAgentPool::new().with_default_concurrency(max_concurrent)`.
`cleanup_plan_pool_agents()` calls `agent_pool.kill_plan_agents()`. But the
dispatch path at line 14088 creates a fresh agent every call. The pool methods
`pre_spawn_warm()`, `promote_warm()`, `add_active()`, and `run_task()` are
never called. The pool always has 0 agents.

The pool implementation at `crates/roko-agent/src/pool.rs` and
`crates/roko-agent/src/multi_pool.rs` is well-tested with comprehensive unit
tests.

**Option 1 (recommended): Wire the pool into dispatch.**

Pre-warm agents at plan start and promote them on dispatch, avoiding the
per-task agent construction overhead.

**Option 2: Remove the pool.**

Delete `pool.rs`, `multi_pool.rs`, and the `agent_pool` field. Simpler, but
loses the concurrency-limiting and pre-warming infrastructure.

**Pros/cons:**

| | Wire (Option 1) | Remove (Option 2) |
|---|---|---|
| Pro | Reduces agent startup latency; pool tests already pass | Less code to maintain |
| Pro | Concurrency limiting already implemented | No risk of pool state bugs |
| Con | Must validate pool lifecycle matches PlanRunner lifecycle | Loses tested infrastructure |
| Con | Pool state must be cleaned up on plan failure | Harder to add back later |

**Recommendation:** Wire. The implementation is complete and tested.

#### A-1: Pre-warm agents at plan start

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- In `PlanRunner::run_plan` (or the plan initialization block), after the plan
  is loaded and the task graph is known, call:
  ```rust
  // Pre-warm one agent per expected concurrent task, up to max_concurrent.
  let warm_count = std::cmp::min(plan.tasks.len(), self.max_concurrent);
  for i in 0..warm_count {
      let model = self.resolve_model_for_task(&plan.tasks[i]);
      self.agent_pool.pre_spawn_warm(&model, &self.workdir).await?;
  }
  ```
- The `pre_spawn_warm` method already exists and spawns an idle agent in the
  background. No new pool code is needed.
- Anti-pattern: do not pre-warm more agents than `max_concurrent`. The pool
  already enforces concurrency limits but pre-warming beyond the limit wastes
  resources.

#### A-2: Promote warm agents on dispatch

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- In `dispatch_agent_with` (the function at line 14088 that creates a fresh
  agent), check the pool first:
  ```rust
  let agent = if let Some(warm) = self.agent_pool.promote_warm(&model_name) {
      tracing::debug!(model = %model_name, "promoted warm agent from pool");
      warm
  } else {
      tracing::debug!(model = %model_name, "no warm agent available, creating fresh");
      self.create_agent_for_task(&task, &model_name).await?
  };
  self.agent_pool.add_active(&plan_id, &task_id, agent.clone());
  ```
- After the agent finishes its task, call:
  ```rust
  self.agent_pool.release_active(&plan_id, &task_id);
  ```
- Anti-pattern: do not keep agents active after task completion. The pool's
  `release_active` handles cleanup. Holding agents prevents new tasks from
  claiming a concurrency slot.

#### A-3: Validate pool cleanup on plan failure

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- Confirm that `cleanup_plan_pool_agents()` is called in ALL plan exit paths:
  success, failure, cancellation, and panic (via a drop guard or `scopeguard`).
- Currently it is called only on the success path. Add calls to the error
  and cancellation paths.
- Anti-pattern: do not silently leak pool agents on plan failure. Leaked agents
  consume memory and may hold file locks.

#### A-4: Add integration test for pool-based dispatch

- [ ] File: `crates/roko-cli/tests/pool_dispatch_integration.rs` (new file)
- Test `warm_agent_promoted_on_dispatch`:
  - Create a `MultiAgentPool` with `max_concurrent = 2`.
  - Pre-warm one agent for model `"test-model"`.
  - Call `promote_warm("test-model")` and assert it returns `Some`.
  - Call `promote_warm("test-model")` again and assert it returns `None`.
- Test `pool_cleanup_on_failure`:
  - Add an active agent to the pool.
  - Call `kill_plan_agents()`.
  - Assert the pool is empty.

---

### Gap B: 10 dead modules in roko-orchestrator

**Why it matters:** `crates/roko-orchestrator/src/` exports 10 modules from
`lib.rs` that have zero production callers outside the crate. Contributors
reading the code assume these modules are load-bearing when they are not.

| # | Module | What | Assessment |
|---|--------|------|-----------|
| 1 | `merge_queue.rs` | `MergeQueue`, `MergeRequest` | Dead. PlanRunner calls `merge_branch()` directly without queue serialization. |
| 2 | `mesh_relay.rs` | `MeshRelay` for multi-node pheromone sync | Dead. Zero callers. Phase 2+. |
| 3 | `repair.rs` | `RepairEngine` | Dead. Zero callers. |
| 4 | `progress.rs` | `ProgressTracker` | Dead. Zero callers. PlanRunner has its own progress tracking. |
| 5 | `safety/loop_guard.rs` | `LoopGuard`, `LoopVerdict`, `LoopGuardConfig` | Dead. Zero callers. |
| 6 | `safety/capability_tokens.rs` | Capability token types | Dead. Zero callers. |
| 7 | `safety/sandboxing.rs` | Sandboxing primitives | Dead. Zero callers. |
| 8 | `safety/taint_propagation.rs` | Taint propagation types | Dead. Only a comment in roko-core references it. |
| 9 | `safety/permit.rs` | Permit types | Dead. Zero callers. |
| 10 | `safety/audit_chain.rs` | `AuditChain` | Nearly wired. `ParallelExecutor` has `audit_chain: Option<AuditChain>` slot + `with_audit_chain()` builder (lines 249, 379). But `orchestrate.rs` never calls `with_audit_chain()`. Always None. |

**Note:** The real safety enforcement lives in `crates/roko-agent/src/safety/`
(SafetyLayer, ToolDispatcher, authz, contracts, hooks). The orchestrator safety
modules are a parallel implementation that was never connected.

#### B-1: Wire `AuditChain` into the executor

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- Where `ParallelExecutor` is constructed (find the builder chain), add:
  ```rust
  .with_audit_chain(AuditChain::new(workdir.join(".roko/audit-chain.jsonl")))
  ```
- The `AuditChain` will then receive entries from the executor's existing
  `audit_chain.as_ref().map(|ac| ac.record(...))` call sites.
- File: `crates/roko-orchestrator/src/lib.rs` -- ensure `AuditChain` is
  re-exported.
- Anti-pattern: do not create a new `AuditChain` per task. One chain per
  executor run is the design intent.

#### B-2: Document dead modules with `#[deprecated]` or feature-gate

- [ ] For modules 1-9, add `#[deprecated(note = "Not connected to runtime. See PRD 07-B.")]`
  to each module's primary public type.
- [ ] Alternatively, gate them behind a `dead-modules` Cargo feature (disabled
  by default) so they do not appear in docs or IDE completion.
- [ ] File: `crates/roko-orchestrator/src/lib.rs` -- add the deprecation
  annotations or feature gates.
- Anti-pattern: do not delete these modules without checking whether any
  downstream crate (even unused ones like roko-dreams) imports them. Run
  `cargo build --workspace` after changes.

#### B-3: Add a manifest comment in lib.rs

- [ ] File: `crates/roko-orchestrator/src/lib.rs`
- Add a top-level doc comment:
  ```rust
  //! # Module status
  //!
  //! The following modules are exported but have no production callers as of
  //! 2026-04-22. They are retained for potential future use:
  //! - `merge_queue` — queue serialization for merge requests
  //! - `mesh_relay` — multi-node pheromone sync (Phase 2+)
  //! - `repair` — repair engine
  //! - `progress` — progress tracking (PlanRunner has its own)
  //! - `safety::loop_guard` — loop detection
  //! - `safety::capability_tokens` — capability tokens
  //! - `safety::sandboxing` — sandboxing primitives
  //! - `safety::taint_propagation` — taint propagation
  //! - `safety::permit` — permit types
  //!
  //! `safety::audit_chain` is wired via `ParallelExecutor::with_audit_chain()`.
  ```

---

### Gap C: Wire CustodyLogger writer into the tool dispatch path

**Why it matters:** `crates/roko-cli/src/custody.rs` provides CLI commands
`roko custody list`, `roko custody show`, and `roko custody verify`. These are
readers. But `CustodyLogger::log()` is never called in production.
`SafetyLayer::authorize_call_with_taint()` performs authorization but never
writes custody records. `ToolDispatcher` also does not write them. Running
`roko custody list` always reports "No custody records found."

#### C-1: Locate CustodyLogger and understand the record format

- [ ] File: `crates/roko-cli/src/custody.rs`
- Read the `CustodyLogger` struct and its `log()` method signature to
  understand what data it expects (tool name, agent id, authorization decision,
  timestamp, taint label).
- Read the `CustodyRecord` type to understand the serialization format.

#### C-2: Wire CustodyLogger into ToolDispatcher

- [ ] File: `crates/roko-agent/src/dispatcher/mod.rs`
- Add an optional `custody_logger: Option<Arc<CustodyLogger>>` field to
  `ToolDispatcher` (or accept it via a builder method
  `with_custody_logger(logger: Arc<CustodyLogger>)`).
- After each `authorize_call_with_taint()` call (or `check_pre_execution()`
  call that replaces it per PRD 04 Gap B-2), write a custody record:
  ```rust
  if let Some(ref logger) = self.custody_logger {
      logger.log(CustodyRecord {
          tool_name: call.name.clone(),
          agent_id: self.agent_id.clone(),
          decision: authz_decision.clone(),
          taint: taint_label.clone(),
          timestamp: chrono::Utc::now(),
      }).await;
  }
  ```
- Anti-pattern: do not make custody logging synchronous or blocking. The
  logger should append to `.roko/custody/records.jsonl` asynchronously. If
  `CustodyLogger::log()` is already async, use it directly. If not, spawn a
  background task.

#### C-3: Wire CustodyLogger into SafetyLayer for Claude CLI path

- [ ] File: `crates/roko-agent/src/safety/mod.rs`
- Since the Claude CLI backend does not go through `ToolDispatcher` (it runs
  its own internal tool loop), custody records for Claude CLI calls must be
  written at the SafetyLayer level.
- Add an optional `custody_logger` field to `SafetyLayer`.
- In `authorize_call_with_taint()`, after computing the `AuthzDecision`, write
  the custody record.
- This ensures custody records are written regardless of which backend is used.

#### C-4: Pass CustodyLogger from PlanRunner to the dispatch path

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- Create `CustodyLogger::new(workdir.join(".roko/custody/records.jsonl"))` in
  `PlanRunner::new`.
- Pass it to the agent creation path so it reaches both `ToolDispatcher` and
  `SafetyLayer`.
- Anti-pattern: do not create a new logger per task. One logger per PlanRunner
  is sufficient; the append-only JSONL format handles concurrent writes.

#### C-5: Verify with the CLI reader

- [ ] After wiring, run a plan that dispatches at least one agent.
- [ ] Run `roko custody list` and confirm records appear.
- [ ] Run `roko custody show <record-id>` and confirm the record contains
  the expected fields.
- [ ] Run `roko custody verify` and confirm the integrity check passes.

#### C-6: Add integration test

- [ ] File: `crates/roko-agent/tests/custody_integration.rs` (new file)
- Test `custody_record_written_on_tool_dispatch`:
  - Create a `ToolDispatcher` with a `CustodyLogger` pointing at a temp file.
  - Dispatch a tool call.
  - Read the temp file and assert one `CustodyRecord` was written.
- Test `custody_record_written_on_denied_tool`:
  - Create a `ToolDispatcher` with a `SafetyLayer` that denies the tool.
  - Dispatch a tool call.
  - Assert a `CustodyRecord` with `decision: Deny` was written.

---

### Gap D: Remove dead scaffolding in roko-serve

**Why it matters:** Dead types in production crates confuse contributors and
inflate compile times.

#### D-1: Remove or gate `RelayHealth` in relay.rs

- [ ] File: `crates/roko-serve/src/relay.rs`
- The `RelayHealth` struct has a comment "Exposed via GET /api/relay/health"
  but no such route exists in `crates/roko-serve/src/routes/mod.rs`.
- **Option 1:** Delete `relay.rs` entirely if it contains only dead types.
  Remove `pub mod relay;` from `lib.rs`.
- **Option 2:** If `relay.rs` contains other live code, remove only the dead
  `RelayHealth` struct and its associated impls.
- Before deleting, run `cargo build --workspace` to confirm nothing references
  the types.
- Anti-pattern: do not add the missing `/api/relay/health` route just to
  justify keeping the type. The route was never needed; adding it creates API
  surface debt.

#### D-2: Decide on truth_map.rs

- [ ] File: `crates/roko-serve/src/truth_map.rs`
- Not declared as `pub mod` in `lib.rs`. Runtime documentation registry with
  zero production callers. `truth_map()` and `entity_source()` are never called.
- **Option 1 (recommended):** Delete the file. It provides no runtime value
  and is not referenced.
- **Option 2:** If the runtime documentation concept is desired, declare it as
  `pub mod truth_map;` in `lib.rs` and wire it into the serve startup so
  `/api/docs` or similar exposes it.
- Anti-pattern: do not leave undeclared modules in the source tree. They create
  confusion about what is active.

#### D-3: Verify no broken references

- [ ] After removing the dead types/files, run:
  ```bash
  cargo build --workspace
  cargo test --workspace
  ```
- [ ] Confirm zero compilation errors and zero test regressions.

---

### Gap E: Add ProviderKind::Ollama and OllamaAdapter

**Why it matters:** Ollama dispatch in `crates/roko-cli/src/orchestrate.rs` is
a hardcoded `command == "ollama"` branch that bypasses `adapter_for_kind`
entirely. No `ProviderKind::Ollama` exists. No MCP support for Ollama. The
branch builds its own `ToolLoop` directly instead of going through the provider
adapter system.

#### E-1: Add `ProviderKind::Ollama` variant

- [ ] File: `crates/roko-agent/src/provider/mod.rs` (or wherever `ProviderKind`
  is defined)
- Add `Ollama` to the `ProviderKind` enum.
- Update all `match` arms on `ProviderKind` to handle the new variant. Run
  `cargo build --workspace` to find exhaustive match sites.
- Anti-pattern: do not add `_ => ...` catch-all arms. Every match on
  `ProviderKind` must be exhaustive so new variants cause compile errors.

#### E-2: Implement `OllamaAdapter`

- [ ] File: `crates/roko-agent/src/provider/ollama.rs` (new file, or extend
  existing Ollama code)
- Implement the `ProviderAdapter` trait (or equivalent) for Ollama.
- The adapter should:
  1. Construct the `ToolLoop` the same way the hardcoded branch does.
  2. Support MCP by accepting `McpConfig` in its constructor.
  3. Delegate to the existing Ollama client code in
     `crates/roko-agent/src/ollama/`.
- Anti-pattern: do not duplicate the Ollama HTTP client code. The adapter
  wraps the existing client, it does not replace it.

#### E-3: Wire `OllamaAdapter` into `adapter_for_kind`

- [ ] File: `crates/roko-agent/src/provider/mod.rs` (or wherever
  `adapter_for_kind` is defined)
- Add:
  ```rust
  ProviderKind::Ollama => Box::new(OllamaAdapter::new(config)),
  ```

#### E-4: Remove the hardcoded Ollama branch from orchestrate.rs

- [ ] File: `crates/roko-cli/src/orchestrate.rs`
- Find the `command == "ollama"` branch in the dispatch path.
- Replace it with the standard `adapter_for_kind(ProviderKind::Ollama, ...)`
  call that all other providers use.
- Anti-pattern: do not leave the old branch as a fallback. The adapter must
  be the single code path.

#### E-5: Add test for Ollama provider routing

- [ ] File: `crates/roko-agent/tests/ollama_provider_test.rs` (new file)
- Test `ollama_routes_through_adapter`:
  - Construct a provider config with `kind: ProviderKind::Ollama`.
  - Call `adapter_for_kind` and assert it returns an `OllamaAdapter`.
  - Confirm the adapter creates a `ToolLoop` (mock the HTTP endpoint).

---

### Gap F: Wire PerplexityToolLoopBackend in create_tool_loop_backend

**Why it matters:** `PerplexityToolLoopBackend` at
`crates/roko-agent/src/perplexity/tool_loop.rs` implements `LlmBackend`. But
`create_tool_loop_backend()` at
`crates/roko-agent/src/tool_loop/backends/mod.rs:72-74` returns an error for
`ProviderKind::PerplexityApi`:
```rust
ProviderKind::PerplexityApi => Err(AgentCreationError::MissingConfig(
    "Perplexity tool-loop backend is not implemented yet".into(),
)),
```
The error message is wrong: the implementation exists, it just is not wired.

#### F-1: Wire the existing backend

- [ ] File: `crates/roko-agent/src/tool_loop/backends/mod.rs`
- Replace the error branch at lines 72-74 with:
  ```rust
  ProviderKind::PerplexityApi => {
      let api_key = config.api_key.clone().ok_or_else(|| {
          AgentCreationError::MissingConfig("PERPLEXITY_API_KEY required".into())
      })?;
      let base_url = config.base_url.clone()
          .unwrap_or_else(|| "https://api.perplexity.ai".to_string());
      Ok(Box::new(PerplexityToolLoopBackend::new(api_key, base_url)))
  }
  ```
- Add the import: `use crate::perplexity::tool_loop::PerplexityToolLoopBackend;`
- Anti-pattern: do not change the `PerplexityToolLoopBackend` implementation.
  It already implements `LlmBackend` correctly. Only the factory function
  needs updating.

#### F-2: Update the PerplexityAdapter to support tool loop mode

- [ ] File: `crates/roko-agent/src/provider/perplexity.rs` (or wherever
  `PerplexityAdapter` is defined)
- Currently the adapter creates single-shot chat agents only. Add a path that
  creates a `ToolLoop`-backed agent when tool use is requested.
- This may require a `supports_tool_loop() -> bool` method on the adapter
  trait, returning `true` for Perplexity now that the backend is wired.

#### F-3: Add test for Perplexity tool loop creation

- [ ] File: `crates/roko-agent/tests/perplexity_tool_loop_test.rs` (new file)
- Test `perplexity_tool_loop_backend_created`:
  - Construct a config with `kind: ProviderKind::PerplexityApi` and a dummy
    API key.
  - Call `create_tool_loop_backend` and assert it returns `Ok`.
  - Assert the returned backend is a `PerplexityToolLoopBackend`.
- Test `perplexity_tool_loop_fails_without_key`:
  - Construct a config without an API key.
  - Call `create_tool_loop_backend` and assert it returns
    `Err(AgentCreationError::MissingConfig(...))`.

---

### Gap G: Document Codex JSON-RPC/WebSocket protocol status

**Why it matters:** `crates/roko-agent/src/codex_agent.rs` only implements the
HTTP `/v1/chat/completions` fallback. The JSON-RPC/WebSocket
`AppServerConnection` protocol (from the reference Mori codebase at
`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs`) is not
implemented. A comment in the file acknowledges: "A full JSON-RPC app-server
implementation lands in a later wave."

This gap is documentation-only. The HTTP fallback works for all current use
cases.

#### G-1: Add accurate doc comment to codex_agent.rs

- [ ] File: `crates/roko-agent/src/codex_agent.rs`
- Add a module-level doc comment:
  ```rust
  //! # Codex Agent
  //!
  //! Current implementation: HTTP `/v1/chat/completions` fallback only.
  //!
  //! The full JSON-RPC/WebSocket `AppServerConnection` protocol from the
  //! reference Mori codebase (`/Users/will/dev/uniswap/bardo/apps/mori/
  //! src/agent/connection.rs`, lines 2444-2620) is not yet implemented.
  //!
  //! The HTTP fallback covers all current use cases. The JSON-RPC protocol
  //! adds: bidirectional streaming, tool-call routing, and session
  //! persistence. Filed as a follow-up task.
  ```

#### G-2: Update CLAUDE.md if Codex is mentioned

- [ ] File: `/Users/will/dev/nunchi/roko/roko/CLAUDE.md`
- If the status table mentions Codex, ensure it says `Partial — HTTP fallback
  only; JSON-RPC/WebSocket protocol not implemented`.
- If Codex is not mentioned, add it to the crates table with the correct
  status.

---

### Gap H: Consolidate duplicate scheduler startup paths

**Why it matters:** `roko serve` starts cron via
`start_builtin_event_sources()` in `crates/roko-serve/src/lib.rs:508`.
`roko daemon` starts cron via `start_scheduler()` in
`crates/roko-serve/src/scheduler.rs:13`, called from
`crates/roko-cli/src/daemon.rs:361`. Two independent code paths produce
identical behavior. This means:
1. Bug fixes to one path do not propagate to the other.
2. Cron jobs may fire twice if both `roko serve` and `roko daemon` run
   simultaneously.

#### H-1: Identify the canonical startup path

- [ ] Read both startup paths and determine which is more complete.
- `start_builtin_event_sources()` in `lib.rs:508` is likely the canonical
  path since it runs inside the server's tokio runtime with access to
  `AppState`.
- `start_scheduler()` in `scheduler.rs:13` may be a standalone daemon path
  that predates the serve integration.

#### H-2: Consolidate to one path

- [ ] File: `crates/roko-cli/src/daemon.rs`
- Replace the direct `start_scheduler()` call with a call to
  `start_builtin_event_sources()` (or extract the shared logic into a common
  function that both paths call).
- If `roko daemon` needs the scheduler without the full HTTP server, extract
  the cron logic from `start_builtin_event_sources()` into a shared
  `start_cron_jobs(config: &CronConfig, state: Arc<AppState>)` function.
- Anti-pattern: do not merge `roko daemon` and `roko serve` into one command.
  They serve different deployment models (daemon = background process, serve =
  foreground HTTP server). Only the cron startup should be shared.

#### H-3: Add guard against double-firing

- [ ] File: `crates/roko-serve/src/lib.rs` (or the shared cron function)
- Add an `AtomicBool` guard that prevents cron jobs from being registered
  twice:
  ```rust
  static CRON_STARTED: std::sync::atomic::AtomicBool =
      std::sync::atomic::AtomicBool::new(false);
  if CRON_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
      tracing::warn!("cron jobs already started, skipping duplicate registration");
      return;
  }
  ```
- Anti-pattern: do not use `OnceLock` for this. The guard must be resettable
  for testing.

#### H-4: Add test for deduplication

- [ ] File: `crates/roko-serve/tests/scheduler_dedup_test.rs` (new file)
- Test `cron_jobs_not_registered_twice`:
  - Call the shared cron startup function twice.
  - Assert the second call returns without registering duplicate jobs.

---

### Gap I: Wire provider health tracking in roko-serve dispatch

**Why it matters:** Provider health tracking works in the CLI orchestrator
path: `orchestrate.rs:13453` checks `is_healthy()`, and
`crates/roko-learn/src/runtime_feedback.rs:832` records success/failure. But
the roko-serve HTTP dispatch loop (`crates/roko-serve/src/dispatch.rs`) never
calls `record_success()` or `record_failure()` after agent runs. Further,
`route_with_health()` exists but is unused in the serve path.
`dispatch.rs:2326` uses `CascadeRouter::load_or_new().route()` (without
health) instead of `route_with_health()`.

#### I-1: Replace `route()` with `route_with_health()` in serve dispatch

- [ ] File: `crates/roko-serve/src/dispatch.rs`
- At line 2326 (or the equivalent routing call), replace:
  ```rust
  let model = router.route(&role, &category);
  ```
  with:
  ```rust
  let model = router.route_with_health(&role, &category, &provider_health);
  ```
- The `provider_health` object must be accessible from the serve dispatch
  context. If it is not currently in `AppState`, add it.

#### I-2: Record success/failure after agent runs in serve dispatch

- [ ] File: `crates/roko-serve/src/dispatch.rs`
- After each agent run completes (success or failure), call:
  ```rust
  provider_health.record_success(&provider_name); // on success
  provider_health.record_failure(&provider_name, &error_message); // on failure
  ```
- The `record_success` and `record_failure` methods already exist on the
  health tracker.

#### I-3: Add ProviderHealth to AppState if missing

- [ ] File: `crates/roko-serve/src/state.rs`
- If `AppState` does not already have a `provider_health` field, add:
  ```rust
  pub provider_health: Arc<ProviderHealth>,
  ```
- Initialize it in `AppState::new` from the config or with defaults.
- Anti-pattern: do not create a new `ProviderHealth` per request. It must be
  shared across all requests so health data accumulates.

#### I-4: Add test for health recording in serve dispatch

- [ ] File: `crates/roko-serve/tests/dispatch_health_test.rs` (new file)
- Test `serve_dispatch_records_provider_health`:
  - Set up an `AppState` with a `ProviderHealth` instance.
  - Dispatch a mock agent run that succeeds.
  - Assert `provider_health.is_healthy("test-provider")` returns true.
  - Dispatch a mock agent run that fails 5 times.
  - Assert `provider_health.is_healthy("test-provider")` returns false.

---

## Concrete file touchpoints

| Gap | File | Change |
|-----|------|--------|
| A | `crates/roko-cli/src/orchestrate.rs` | Pre-warm agents at plan start; promote on dispatch; release after task; cleanup on all exit paths |
| A | `crates/roko-agent/src/pool.rs` | No changes needed (already implemented) |
| A | `crates/roko-agent/src/multi_pool.rs` | No changes needed (already implemented) |
| B | `crates/roko-orchestrator/src/lib.rs` | Add deprecation annotations or feature gates; add module status doc comment |
| B | `crates/roko-cli/src/orchestrate.rs` | Wire `AuditChain` into `ParallelExecutor` construction |
| C | `crates/roko-agent/src/dispatcher/mod.rs` | Add `custody_logger` field; write records after authorization |
| C | `crates/roko-agent/src/safety/mod.rs` | Add `custody_logger` field; write records in `authorize_call_with_taint` |
| C | `crates/roko-cli/src/orchestrate.rs` | Create `CustodyLogger` and pass to dispatch path |
| D | `crates/roko-serve/src/relay.rs` | Remove `RelayHealth` or delete file |
| D | `crates/roko-serve/src/truth_map.rs` | Delete file |
| D | `crates/roko-serve/src/lib.rs` | Remove `pub mod relay;` and/or `mod truth_map;` |
| E | `crates/roko-agent/src/provider/mod.rs` | Add `ProviderKind::Ollama`; wire in `adapter_for_kind` |
| E | `crates/roko-agent/src/provider/ollama.rs` (new) | `OllamaAdapter` implementing `ProviderAdapter` |
| E | `crates/roko-cli/src/orchestrate.rs` | Remove hardcoded `command == "ollama"` branch |
| F | `crates/roko-agent/src/tool_loop/backends/mod.rs` | Wire `PerplexityToolLoopBackend` in factory |
| F | `crates/roko-agent/src/provider/perplexity.rs` | Add tool loop mode support |
| G | `crates/roko-agent/src/codex_agent.rs` | Add accurate doc comment |
| G | `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` | Correct Codex status if present |
| H | `crates/roko-cli/src/daemon.rs` | Replace `start_scheduler()` with shared cron function |
| H | `crates/roko-serve/src/lib.rs` | Extract shared cron startup; add double-fire guard |
| H | `crates/roko-serve/src/scheduler.rs` | Delegate to shared function or remove |
| I | `crates/roko-serve/src/dispatch.rs` | Replace `route()` with `route_with_health()`; add `record_success`/`record_failure` |
| I | `crates/roko-serve/src/state.rs` | Add `provider_health` to `AppState` if missing |

---

## Verification checklist

Run each command from the workspace root `/Users/will/dev/nunchi/roko/roko/`.

### Build and lint (all gaps)

```bash
cargo build --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo +nightly fmt --all -- --check
```

### After Gap A

```bash
# Pool promotion test
cargo test -p roko-agent pool -- --nocapture

# Integration test
cargo test -p roko-cli warm_agent_promoted_on_dispatch -- --nocapture

# Confirm pool is non-empty during plan run
RUST_LOG=debug cargo run -p roko-cli -- plan run plans/ 2>&1 | grep "promoted warm agent"
```

### After Gap B

```bash
# Confirm AuditChain receives entries during plan run
RUST_LOG=debug cargo run -p roko-cli -- plan run plans/ 2>&1 | grep "audit_chain"

# Confirm deprecated warnings appear for dead modules
cargo build -p roko-orchestrator 2>&1 | grep "deprecated"

# Workspace still builds clean
cargo build --workspace
```

### After Gap C

```bash
# Run a plan that dispatches at least one agent
cargo run -p roko-cli -- plan run plans/

# Verify custody records were written
cargo run -p roko-cli -- custody list

# Integration test
cargo test -p roko-agent custody_record_written_on_tool_dispatch -- --nocapture
```

### After Gap D

```bash
# Workspace builds without the removed files
cargo build --workspace
cargo test --workspace
```

### After Gap E

```bash
# Confirm Ollama routes through the adapter
cargo test -p roko-agent ollama_routes_through_adapter -- --nocapture

# Manual: run with Ollama configured and verify MCP tools are available
RUST_LOG=debug cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -i "ollama.*mcp"
```

### After Gap F

```bash
# Perplexity tool loop creation test
cargo test -p roko-agent perplexity_tool_loop_backend_created -- --nocapture

# Failure without key
cargo test -p roko-agent perplexity_tool_loop_fails_without_key -- --nocapture
```

### After Gap G

```bash
# Doc comment present
grep -n "JSON-RPC" crates/roko-agent/src/codex_agent.rs
```

### After Gap H

```bash
# Deduplication test
cargo test -p roko-serve cron_jobs_not_registered_twice -- --nocapture

# Manual: start both serve and daemon, confirm cron fires once
```

### After Gap I

```bash
# Health recording test
cargo test -p roko-serve serve_dispatch_records_provider_health -- --nocapture

# Manual: dispatch an agent via HTTP, confirm health is tracked
curl http://localhost:6677/api/health/providers
```

### Full pre-commit gate

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Acceptance criteria

**Gap A is closed when:**
1. `MultiAgentPool.count()` returns > 0 during an active plan run.
2. `promote_warm()` is called at least once per plan run (visible in debug logs).
3. `cleanup_plan_pool_agents()` is called on all plan exit paths (success, failure, cancellation).
4. Existing pool unit tests continue to pass.

**Gap B is closed when:**
1. `ParallelExecutor` is constructed with `with_audit_chain(...)` in orchestrate.rs.
2. `.roko/audit-chain.jsonl` contains entries after a plan run.
3. Dead modules are annotated with `#[deprecated]` or gated behind a feature.
4. `cargo build --workspace` produces no errors.

**Gap C is closed when:**
1. `roko custody list` returns records after a plan run.
2. Records include tool name, agent id, authorization decision, and timestamp.
3. Both the ToolDispatcher path (non-Claude backends) and the SafetyLayer path (Claude CLI) produce records.
4. Integration test passes.

**Gap D is closed when:**
1. `relay.rs` and `truth_map.rs` dead types are removed.
2. `cargo build --workspace` and `cargo test --workspace` pass.

**Gap E is closed when:**
1. `ProviderKind::Ollama` exists and `adapter_for_kind` returns an `OllamaAdapter`.
2. The hardcoded `command == "ollama"` branch is removed from orchestrate.rs.
3. Ollama dispatch works through the adapter system (manual test with a running Ollama instance).
4. MCP tools are available to Ollama agents.

**Gap F is closed when:**
1. `create_tool_loop_backend(ProviderKind::PerplexityApi, config)` returns `Ok(...)` with a valid API key.
2. The error message "Perplexity tool-loop backend is not implemented yet" no longer exists in the codebase.
3. `PerplexityAdapter` can create tool-loop-backed agents.

**Gap G is closed when:**
1. `codex_agent.rs` has an accurate module doc comment describing current vs planned capabilities.
2. CLAUDE.md reflects the partial status if Codex is mentioned.

**Gap H is closed when:**
1. One shared cron startup function exists, called from both `roko serve` and `roko daemon`.
2. The double-fire guard prevents duplicate cron registration.
3. Running `roko serve` and `roko daemon` simultaneously does not produce duplicate cron jobs.

**Gap I is closed when:**
1. `route_with_health()` is called in the serve dispatch path.
2. `record_success()` and `record_failure()` are called after agent runs in the serve dispatch path.
3. `AppState` contains a shared `ProviderHealth` instance.
4. Provider health data accumulates across requests (observable via health API or logs).
