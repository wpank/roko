# 61 — Agent Dispatch Consolidation

**Priority**: P2 — architecture debt; every new cross-cutting feature must be wired N times
**Size**: XL (3-4 weeks)
**Crates**: `crates/roko-cli/src/runner/`, `crates/roko-cli/src/dispatch/`, `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-acp/src/runner.rs`, `crates/roko-serve/src/dispatch.rs`, `crates/roko-graph/src/cells/`
**Depends on**: None (but may be simplified if done after item 60)

---

## Background

Roko has five independent implementations of the "pick a model, assemble a prompt, call a provider, run safety checks, record the outcome" pattern. These grew in parallel and each integrates a different subset of the available subsystems. The consequence is that every new cross-cutting feature — a new feedback store, a new safety check, a new enrichment source — must be wired independently into each path, and currently most paths are missing most features.

The five paths are:
1. **Runner-v2** (`event_loop.rs`) — the CLI plan executor. The reference implementation; integrates all subsystems.
2. **ACP** (`roko-acp/src/runner.rs`) — the IDE integration (Cursor). Hard-wired to Claude CLI; missing most learning signals.
3. **Serve** (`roko-serve/src/dispatch.rs`) — webhook and subscription-driven dispatch. Missing all safety checks.
4. **Graph `AgentCell`** (`roko-graph/src/cells/agent.rs`) — cell-based DAG execution. No safety, no enrichment, no feedback.
5. **`dispatch_via_model_call_service`** (`roko-cli/src/dispatch_v2.rs`) — single-prompt dispatch for `roko run` and `roko chat`. Constructs a fresh `CascadeRouter` per call; no safety, no episode recording.

The goal is not to merge these into one monolithic function. The goal is to extract the shared pipeline steps into a reusable `DispatchPipeline` that each caller composes from, so adding a new subsystem requires one wiring change instead of five.

## Current State

### A1. Runner-v2 entry point

`dispatch_action` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs:8602` (file is 23,154 lines total). Called from the `tokio::select!` loop at line 4922.

**Integrated subsystems** (verified against source):
1. Full `ModelRouter` with `CascadeRouter`, `RoutingBias`, `RoutingContext` (task complexity, budget remaining, conductor signals, provider health).
2. `PromptAssembler` (9-layer builder in `crates/roko-cli/src/dispatch/prompt_builder.rs`, 3,014 lines) with knowledge store bidders, playbook injection, code-index context, gate feedback, dependency outputs, group pheromone injection.
3. Safety pre-dispatch: `if let Some(ref safety) = ctx.config.safety_layer` at line 10123 — calls `pre_dispatch_check_with_context` with per-task `AgentContract`.
4. Safety post-dispatch: `if let Some(ref safety) = config.safety_layer` at line 3035 — calls `post_dispatch_check` with worktree diff inspection.
5. `SharedAgentFactory` → `AgentDispatcherV2` → `ProviderDispatchResolver` with shared semaphores, MCP runtime, rate limiter, health registry (via `crates/roko-cli/src/dispatch/factory.rs`, 377 lines).
6. `EpisodeLogger::append()` with HDC fingerprints.
7. `AgentEfficiencyEvent` per turn.
8. `PlaybookStore::record_outcome` at line 1537-1540.
9. `SectionOutcomeRecord` at lines 3949-3964.
10. `PostGateReflection` at lines 4558-4585.
11. `ErrorPatternStore` gate failure observations at lines 71-73.
12. Cascade router feedback at lines 2051-2065.
13. `DaimonState` affect with full `AffectEngine`.
14. Knowledge store tier progression at lines 63, 82-84, 1853.

### A2. ACP pipeline

`run_agent_phase` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs:1955` (file is 2,496 lines total).

**Integrated**: Safety pre-dispatch (line 1967) and post-dispatch (line 1996) via `safety_layer_for_pipeline_role_with_sandbox` (line 1075). Provider-health is not recorded. Model routing comes from a pre-computed `model_slug` passed in, derived from `cascade_select_model` bridge events.

**Missing**: `PromptAssembler` (uses hardcoded `build_review_prompt` (line 1736) and `prepend_context` with a flat `knowledge_context` string). Episode recording is read-only (context only, no `append`). No `AgentEfficiencyEvent`, `PlaybookStore`, `SectionOutcomeRecord`, `PostGateReflection`, `ErrorPatternStore`, prompt experiment receipts, DaimonState modulation, or knowledge store tier progression.

Hard-wired to `run_claude_cli_via_agent` at line 1990-1991 — ignores `SharedAgentFactory` and rate limiters.

### A3. Serve dispatch

`dispatch_agent` at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/dispatch.rs:1695` (file is 3,305 lines total). Called from `dispatch_loop` at line 1546.

**Integrated**: Full `EpisodeLogger` with HDC fingerprints and distillation spawn (lines 2366-2478). `AgentEfficiencyEvent` (lines 40, 259, 2568). Affect PAD values written to episodes at lines 2591-2628. Cascade router feedback at lines 2639-2701. Provider health recording at lines 2013-2016.

**Missing**: Safety is entirely absent. `agent_contract: None` is set at line 1970. `dangerously_skip_permissions: true` is set at line 1979. No `SafetyLayer` instantiation, no pre-dispatch check, no post-dispatch check. Prompt assembly uses `build_template_system_prompt` (line 2106) which wraps `SystemPromptBuilder::new()` with experiment variant injection but no knowledge bidders, no playbook injection, no code-index context. No `PlaybookStore`, `SectionOutcomeRecord`, `PostGateReflection`, `ErrorPatternStore`, or knowledge store tier progression.

### A4. Graph `AgentCell`

`AgentCell` at `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/agent.rs:127`. Dispatches via `dyn AgentDispatcher` (trait at line 134) — a different trait from `roko-serve`'s `AgentDispatcher` at `dispatch.rs:66`, despite sharing the name.

The `AgentCell` dispatcher trait signature (line 135-145) takes raw `model`, `provider`, `system_prompt`, `user_message`, `tools`, `max_tokens`, `temperature` parameters and returns `AgentResponse`. No safety, no enrichment, no episode recording, no cascade feedback, no affect integration. The graph engine has no integration with `SafetyLayer`, `EpisodeLogger`, `CascadeRouter`, `PlaybookStore`, `DaimonState`, `KnowledgeStore`, or `AgentEfficiencyEvent`.

`TaskExecutorCell` (`crates/roko-graph/src/cells/task_executor.rs:91-101`) delegates via `dyn TaskDispatcher` injected by the CLI host, which inherits whatever the host applies — but `AgentCell` does not use this path.

### A5. `dispatch_via_model_call_service`

`dispatch_via_model_call_service` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs:66`. Used by `roko run` and `roko chat`.

Constructs a fresh `CascadeRouter` per call (line 104+). Records cascade and provider health feedback. No safety layer, no episode recording, no `PromptAssembler`, no playbook/knowledge enrichment.

## Implementation Plan

This is a large architectural refactor. Work in phases so each phase compiles and passes tests independently.

### Phase 1: Define a `DispatchPipeline` value object (1-2 days)

Create a new module `crates/roko-cli/src/dispatch/pipeline.rs` (or a new crate `crates/roko-dispatch/` if cross-crate sharing is needed).

Define:

```rust
/// Input to a single dispatch call.
pub struct DispatchRequest {
    pub system_prompt: String,
    pub user_messages: Vec<ChatMessage>,
    pub model: String,           // resolved model slug
    pub role: String,
    pub workdir: PathBuf,
    pub budget_remaining_usd: f64,
    pub attempt: u32,
    pub agent_contract: Option<AgentContract>,
    pub gate_feedback: Option<String>,
    pub dependency_outputs: Vec<String>,
    pub routing_context: Option<RoutingContext>,
    pub plan_id: String,
    pub task_id: String,
}

/// Output from a single dispatch call.
pub struct DispatchOutcome {
    pub content: String,
    pub cost_usd: f64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model_used: String,
    pub provider_used: String,
    pub changed_files: Vec<PathBuf>,
    pub session_id: String,
}

/// Composable pipeline for route → enrich → spawn → record.
pub struct DispatchPipeline {
    pub model_router: Option<Arc<ModelRouter>>,
    pub prompt_enricher: Option<Arc<PromptAssemblyService>>,
    pub safety: Option<SafetyLayer>,
    pub agent_factory: Arc<SharedAgentFactory>,
    pub episode_logger: Option<Arc<EpisodeLogger>>,
    pub cascade_router: Option<Arc<CascadeRouterHandle>>,
    pub playbook_store: Option<Arc<PlaybookStore>>,
    pub daimon_state: Option<Arc<Mutex<DaimonState>>>,
    // ... other optional subsystems
}

impl DispatchPipeline {
    pub async fn run(&self, req: DispatchRequest) -> Result<DispatchOutcome>;
}
```

The `run` method applies each optional stage in order: model routing (if `model_router` is set), prompt enrichment (if `prompt_enricher` is set), safety pre-check (if `safety` is set), agent spawn (always via `agent_factory`), safety post-check (if `safety` is set), and all feedback recording.

### Phase 2: Wire serve dispatch through `DispatchPipeline` (2-3 days)

Priority: serve has zero safety checks, making it the highest-priority path to migrate.

In `crates/roko-serve/src/dispatch.rs`:
1. Construct a `DispatchPipeline` in `TemplateAgentDispatcher::new` (line 510) with `safety: Some(SafetyLayer::from_config(&roko_config))`.
2. In `dispatch_agent` (line 1695), build a `DispatchRequest` and call `pipeline.run(req)` instead of the current inline sequence.
3. Remove `agent_contract: None` (line 1970) and `dangerously_skip_permissions: true` (line 1979). The pipeline handles contracts through its `safety` field.
4. The existing episode recording, efficiency events, affect, and cascade feedback at lines 2366-2701 move into `DispatchPipeline::run`.

### Phase 3: Wire `dispatch_via_model_call_service` through `DispatchPipeline` (1 day)

In `crates/roko-cli/src/dispatch_v2.rs:66`, construct a `DispatchPipeline` with:
- A shared `CascadeRouter` (read from the global learn path, not freshly constructed per call).
- `episode_logger: None`, `safety: None`, `prompt_enricher: None` (lightweight path).
- Cascade and provider health feedback (already exists, move into pipeline's feedback stage).

### Phase 4: Wire ACP through `DispatchPipeline` (2-3 days)

In `crates/roko-acp/src/runner.rs`:
1. Replace the `run_claude_cli_via_agent` call at line 1990 with a `DispatchPipeline` call that uses `SharedAgentFactory`.
2. Replace `build_review_prompt` + `prepend_context` at lines 1736-1757 with the pipeline's `prompt_enricher` stage (knowledge context as a bidder input).
3. Add `episode_logger` and `playbook_store` to the pipeline for ACP dispatch.

The ACP session lifecycle and protocol (model selection via bridge events, ACP wire format) remain unchanged.

### Phase 5: Wire runner-v2 through `DispatchPipeline` (2-3 days)

In `dispatch_action` at `event_loop.rs:8602`, replace the inline route-enrich-spawn-record sequence with a `DispatchPipeline::run` call. The event loop retains ownership of the executor state machine, worktree management, merge queue, and cancellation; it delegates the dispatch sequence to the pipeline.

This is the largest phase because `dispatch_action` is deeply interleaved with the event loop. Work incrementally: extract one subsystem at a time (cascade feedback first, then episodes, then enrichment) and keep tests green after each extraction.

### Phase 6: Consolidate the two `AgentDispatcher` traits (1 day)

`roko-serve::dispatch::AgentDispatcher` at `dispatch.rs:66`:
```rust
pub trait AgentDispatcher: Send + Sync {
    async fn dispatch(&self, template: AgentTemplate, signal: Signal) -> Result<AgentResult>;
}
```

`roko-graph::cells::agent::AgentDispatcher` at `agent.rs:134`:
```rust
pub trait AgentDispatcher: Send + Sync {
    async fn dispatch(&self, model: &str, provider: &str, system_prompt: &str, user_message: &str, tools: &[String], max_tokens: u32, temperature: f32) -> Result<AgentResponse, String>;
}
```

These are fundamentally different shapes. Options:
- Rename the graph trait to `GraphAgentDispatcher` (smaller change, avoids confusion).
- Make both implement a common base via the `DispatchPipeline` (larger, cleaner long-term).

The minimum: rename the graph trait to `GraphAgentDispatcher` to eliminate the naming collision and add a doc comment explaining the relationship.

## Acceptance Criteria

1. A `DispatchPipeline` type exists with optional stage composition for model routing, prompt enrichment, safety, episode recording, cascade feedback, playbook feedback, and affect.
2. `TemplateAgentDispatcher` (serve) constructs a `DispatchPipeline` with `safety: Some(SafetyLayer::from_config(...))` and removes the `agent_contract: None` / `dangerously_skip_permissions: true` pattern.
3. Serve dispatch applies safety pre-dispatch and post-dispatch checks on every agent invocation.
4. `dispatch_via_model_call_service` reuses the shared `CascadeRouter` from the workspace learn path instead of constructing one per call.
5. ACP dispatch calls `SharedAgentFactory` instead of the hard-wired `run_claude_cli_via_agent`.
6. Episode recording with HDC fingerprints happens in at least serve and ACP dispatch paths (in addition to runner-v2 which already does it).
7. `PlaybookStore` outcome feedback is recorded by serve dispatch.
8. The graph `AgentDispatcher` trait is renamed to `GraphAgentDispatcher` or consolidated with the serve trait.
9. Adding a new feedback subsystem requires wiring it in `DispatchPipeline::run` only — not in each caller independently.
10. All existing tests pass; no behavioral regression in the runner-v2 event loop.

## Out of Scope

- Merging runner-v2 and the graph engine executors (separate item).
- ACP wire protocol changes.
- New LLM provider backends.
- `DaimonState` write access from ACP.
- Prompt experiment assignment parity in ACP.

## Verification Checklist

- [ ] Define `DispatchRequest`, `DispatchOutcome`, and `DispatchPipeline` structs and confirm `cargo build` is clean
- [ ] Wire serve dispatch through `DispatchPipeline`; confirm `cargo test -p roko-serve` still passes
- [ ] Confirm serve dispatch now applies safety pre-dispatch check (add a test or trace log)
- [ ] Wire `dispatch_via_model_call_service` through `DispatchPipeline`; confirm no new `CascadeRouter::load_or_new` per call
- [ ] Wire ACP through `DispatchPipeline` (at minimum: replace `run_claude_cli_via_agent` with factory)
- [ ] Rename graph `AgentDispatcher` to `GraphAgentDispatcher`; update all references; `cargo build` clean
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/dispatch/pipeline.rs` (new) | `DispatchRequest`, `DispatchOutcome`, `DispatchPipeline` struct and `run` method |
| `crates/roko-cli/src/dispatch/mod.rs` | Export new `pipeline` module |
| `crates/roko-serve/src/dispatch.rs` | Construct `DispatchPipeline` in `TemplateAgentDispatcher::new`; replace inline dispatch with pipeline call; remove `agent_contract: None` and `dangerously_skip_permissions: true` |
| `crates/roko-cli/src/dispatch_v2.rs` | Replace per-call `CascadeRouter` construction with pipeline-managed shared router |
| `crates/roko-acp/src/runner.rs` | Replace `run_claude_cli_via_agent` with factory-based dispatch; wire episode logger and playbook feedback |
| `crates/roko-cli/src/runner/event_loop.rs` | Replace inline dispatch sequence in `dispatch_action` with `DispatchPipeline::run` (incremental) |
| `crates/roko-graph/src/cells/agent.rs` | Rename `AgentDispatcher` to `GraphAgentDispatcher` |
