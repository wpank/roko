# ACP: Integration Gaps with Roko Subsystems

> **Source**: Cross-crate analysis of `crates/roko-acp/src/` against all roko subsystems
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `.roko/GAPS.md`
> **Created**: 2026-08-15

## Wired Integrations

- **Cascade Router (model selection)**: ACP loads `CascadeRouter`, calls `route_with_health_scored()`, and records observations after each dispatch via `record_cascade_observation()`. The reward function (`compute_acp_reward`) factors success, latency, and token count. Persisted to `.roko/learn/cascade-router.json` with IO lock.
- **Knowledge Store (neuro) for dispatch context**: `query_dispatch_knowledge()` queries both `KnowledgeStore` (roko-neuro) and `PlaybookStore` (roko-learn) per prompt. Results are injected into the system prompt as context and shown as a visible knowledge card in the editor.
- **Episode Logger**: Every ACP dispatch appends a full `Episode` to `.roko/episodes.jsonl` via `append_acp_episode()`. Episodes include model, backend, usage, timing, input/output signal hashes, and failure reasons.
- **Episode Distillation**: After each episode, `roko_neuro::spawn_episode_distillation()` is called in a background task, feeding the knowledge store from ACP interactions.
- **Dream Consolidation**: `maybe_spawn_dream_consolidation()` fires after each episode. Checks episode count since last dream report against `DREAM_EPISODE_THRESHOLD` (10). Spawns a background `DreamRunner::consolidate_now()` with full `DreamLoopConfig`.
- **Dream Routing Advice**: `load_dream_routing_advice()` and `relevant_pattern_summaries()` are consumed during provenance chain construction. Dream patterns appear in the provenance card shown to the user.
- **Efficiency Events**: `acp_efficiency_event()` builds a per-dispatch `AgentEfficiencyEvent` and `emit_acp_efficiency_event()` persists it to `.roko/learn/efficiency.jsonl`. Events include cost, tokens, latency, model, and session ID.
- **Prompt Experiments (A/B)**: `assign_acp_experiment()` selects a running experiment from `ExperimentStore`, applies the variant system prompt, and `record_outcome_for_experiment()` records outcomes. IO-locked for concurrent safety.
- **Provider Health**: ACP loads and shares `ProviderHealthRegistry` across sessions. Health scores feed into cascade routing via `route_with_health_scored()`. Rate limiting via `ProviderRateLimiter` is wired with RPM/TPM awareness.
- **Gate Pipeline (rungs 0-2)**: `run_gates()` in `runner.rs` uses `roko_gate::CompileGate`, `TestGate`, `ClippyGate` with the `Verify` trait. `AdaptiveThresholds` are loaded, observed, and persisted. Individual gate results are emitted as ACP tool_call events visible in the editor.
- **Event Bus (runtime_event_bus)**: `spawn_runtime_event_bridge()` subscribes to the global `runtime_event_bus::<RuntimeDriverEvent>()` and forwards relevant events to the ACP session. `AcpWorkflowEventConsumer` implements `EventConsumer` for the `WorkflowEngine` path.
- **Event Forwarding to roko-serve**: `AcpEventForwarder` maps `CognitiveEvent` to `RuntimeEvent` and forwards via `HttpEventSink`. Enabled when `ROKO_SERVE_URL` is set. Covers token chunks, tool calls, completions, failures, plan updates, MCP status.
- **SystemPromptBuilder (roko-compose)**: `session.rs` uses `SystemPromptBuilder::new()` with role identity and appends knowledge context, pinned files, and conversation history layers.
- **WorkflowEngine (roko-runtime)**: `run_with_workflow_engine()` builds services via `ServiceFactory::build()` from `roko-serve`, creates a `WorkflowEngine`, adds `JsonlLogger` and `AcpWorkflowEventConsumer` as event consumers, and executes the full workflow pipeline.
- **Safety Layer**: `SafetyLayer` and `AgentContract` are loaded and enforced per dispatch. `DispatchSafetyContext` is built with the resolved model, role, and action context.
- **Config Hot-Reload**: `ConfigWatcher` detects file changes via `notify::RecommendedWatcher`. On change, `replace_roko_config()` updates the `SessionManager`, revalidates all active sessions, and rebuilds the rate limiter if providers changed. `ConfigCache` provides zero-copy reads via `ArcSwap`.
- **Budget Guardrails**: Session cost tracking via `record_efficiency_cost()`. Budget enforcement returns `SESSION_BUDGET_EXCEEDED` JSON-RPC error when the ceiling is hit.
- **DaimonState (affect routing)**: `acp_routing_context()` reads `.roko/daimon/affect.json` (or legacy path) to build `DaimonPolicy` for affect-aware model routing.

## Partial Integrations

- **Gate Pipeline (rungs 3-6)**: ACP uses rungs 0 (compile), 1 (clippy), and 2 (test) from the 7-rung pipeline. Rungs 3-6 (diff review, semantic analysis, integration tests, oracle gates) are not wired. The runner's `WorkflowEngine` path has gate support but the direct `run_gates()` only runs the first three.
- **Event Bus Emission (ACP as producer)**: ACP subscribes to the event bus as a consumer and forwards events via HTTP. However, ACP itself does not publish its own lifecycle events (session created, session closed, prompt started, prompt completed) to the bus. Other subsystems cannot discover ACP activity by subscribing to the bus.
- **Config Synchronization (bidirectional)**: Config changes on disk are detected and applied to ACP sessions (file -> ACP). But changes made via `roko serve` API (e.g., PUT /config) are not automatically pushed to running ACP processes. The ACP process only watches files, not the HTTP API. Direction is file-to-ACP only, not HTTP-to-ACP.
- **Knowledge-Informed Model Selection**: The neuro store is queried for dispatch context (system prompt enrichment) but is NOT consulted for model selection in the CascadeRouter. The routing context (`RoutingContext`) uses task category, complexity band, role, daimon policy, and conductor load -- but no knowledge store signal. This is the exact gap identified in CLAUDE.md item 13.

## Missing Integrations

- **TUI Dashboard Visibility**: The roko TUI (`crates/roko-cli/src/tui/`) has no ACP integration. There is zero grep-matching of "acp" in the TUI source. ACP sessions, their progress, and their metrics are invisible in `roko dashboard`. The TUI would need to either (a) read ACP episodes/efficiency logs, or (b) receive ACP events via the event bus or StateHub.
- **roko-serve Session Visibility**: `roko-serve` has no routes or handlers for querying ACP sessions. The only ACP-related mention in roko-serve is a comment in `service_factory.rs` ("for CLI, server, and ACP"). There are no `/api/acp/sessions`, `/api/acp/metrics`, or similar endpoints. ACP sessions are entirely opaque to the HTTP control plane.
- **force_backend Override Learning**: When a user explicitly selects a model/provider in the ACP session config (`model_selection_explicit = true`), this override is not fed back to the cascade router as a learning signal. The router only learns from dispatches it selected itself. This is the exact gap identified in CLAUDE.md item 15.
- **Cold Substrate Archival Trigger**: ACP does not trigger cold archival. The `roko-neuro` cold store exists but has no runtime trigger from ACP (or anywhere else -- CLAUDE.md item 14). Episodes and knowledge accumulate without pruning.

## Per-Integration Details

### 1. Cascade Router Learning

- **Status**: Wired (route + observe), but force_backend learning is missing
- **What works**: `CascadeRouter::load_or_new()` loads the router on each dispatch. `route_with_health_scored()` selects a model with provider health and rate-limit awareness. After dispatch, `record_cascade_observation()` computes a reward (`compute_acp_reward()` using success/latency/tokens), calls `router.observe()`, and persists back to disk. IO-locked via `CASCADE_ROUTER_IO_LOCK`. The router path is `.roko/learn/cascade-router.json`.
- **What's missing**: When `session.model_selection_explicit == true`, the user chose a specific model. This override is NOT recorded as a positive signal for that model in the cascade router. The router cannot learn that the user prefers certain models for certain tasks. (CLAUDE.md item 15: "force_backend override learning")
- **Fix**: In the post-dispatch path (around line 2300 of bridge_events.rs), check `model_selection_explicit`. If true, call `record_cascade_observation()` with the user-selected model and a boosted reward (e.g., 1.0) so the router learns from manual overrides.
- **Effort**: 2h
- **File**: `crates/roko-acp/src/bridge_events.rs` lines 2270-2330

### 2. Knowledge-Informed Model Routing

- **Status**: Partial -- knowledge is used for prompt context but NOT for model selection
- **What works**: `query_dispatch_knowledge()` queries both `KnowledgeStore` and `PlaybookStore` per prompt. Results are injected into the system prompt and shown as a knowledge card in the editor. Dream routing advice is loaded for provenance.
- **What's missing**: The `RoutingContext` built by `acp_routing_context()` does not include any signal from the knowledge store. The cascade router selects models based on task category, complexity, role, daimon affect, and conductor load -- but not on whether prior knowledge suggests a specific model performed well for this type of task. (CLAUDE.md item 13)
- **Fix**: After querying knowledge, extract relevant heuristics about model performance (e.g., "model X works well for concurrency fixes") and pass them as a field on `RoutingContext` (or as optional biases to `route_with_health_scored()`). This requires changes in both roko-learn (extend `RoutingContext` or the router API) and roko-acp (pass knowledge hits into routing).
- **Effort**: 4h
- **Files**: `crates/roko-acp/src/bridge_events.rs` (routing context construction), `crates/roko-learn/src/cascade_router.rs` (accept knowledge biases)

### 3. Dream Consolidation

- **Status**: Wired
- **What works**: `maybe_spawn_dream_consolidation()` is called after every episode. It checks whether the number of episodes since the last dream report exceeds `DREAM_EPISODE_THRESHOLD` (10). If so, it spawns a background `DreamRunner::consolidate_now()` with the workspace's agent config. Dream routing advice is loaded and used for provenance chain construction. Dream pattern summaries are surfaced via `relevant_pattern_summaries()`.
- **What's missing**: The threshold is hardcoded (10 episodes). No config knob exists to tune it per workspace. The dream cycle runs synchronously in a blocking thread (`spawn_blocking` + `consolidate_now`) which internally calls `block_on`, so it is incompatible with nested async runtimes. This is fragile but works in practice because `spawn_blocking` runs on the blocking thread pool.
- **Fix**: Add `dreams.episode_threshold` to `roko.toml` config schema. Consider making `DreamRunner` fully async to avoid the `block_on` hazard.
- **Effort**: 2h (config knob), 6h (async DreamRunner)
- **File**: `crates/roko-acp/src/bridge_events.rs` lines 486-562

### 4. Event Bus Integration

- **Status**: Partial -- ACP consumes events but does not produce them
- **What works**: `spawn_runtime_event_bridge()` subscribes to `runtime_event_bus::<RuntimeDriverEvent>()` and forwards matching events to the ACP session. The `AcpWorkflowEventConsumer` implements `CoreEventConsumer` and receives workflow engine events (agent spawned/completed/failed, gate started/passed/failed, inference started/completed/failed, phase transitions, workflow completed). These are mapped to `CognitiveEvent` variants.
- **What's missing**: ACP does not publish its own lifecycle events to the bus. Other subsystems (TUI, roko-serve, roko-conductor) cannot discover that an ACP session was created, that a prompt started, or that a session closed. The bus is one-directional from ACP's perspective.
- **Fix**: Add `RuntimeEvent::AcpSessionCreated`, `AcpPromptStarted`, `AcpPromptCompleted`, `AcpSessionClosed` variants to `roko-core::runtime_event`. Emit them from `SessionManager` and `handle_session_prompt()`. This unblocks TUI integration and roko-serve visibility.
- **Effort**: 4h
- **Files**: `crates/roko-core/src/runtime_event.rs` (new variants), `crates/roko-acp/src/session.rs` (emit on lifecycle), `crates/roko-acp/src/bridge_events.rs` (emit on prompt start/end)

### 5. Gate Integration

- **Status**: Partial -- rungs 0-2 wired, rungs 3-6 not accessible
- **What works**: `run_gates()` in `runner.rs` runs `CompileGate` (rung 0), `TestGate` (rung 2), and `ClippyGate` (rung 1) using the `Verify` trait. `AdaptiveThresholds` are loaded from `.roko/learn/gate-thresholds.json`, observed after each gate, and saved. Individual results are emitted as ACP tool_call events with start/complete status. The `/gate` slash command triggers the full pipeline.
- **What's missing**: Rungs 3-6 (diff review, semantic analysis, integration tests, oracle gates) are not accessible from the ACP runner. The `WorkflowEngine` path partially supports them via `ServiceFactory` but the direct ACP `run_gates()` only knows about compile/test/clippy. The review phase in the pipeline state machine is separate from the gate pipeline.
- **Fix**: Extend `run_gates()` to optionally run higher rungs. The `roko-gate` crate already has the infrastructure (`GatePayload`, rung index system). Wire the reviewer gate (rung 3) and diff gate into the pipeline runner.
- **Effort**: 4h
- **File**: `crates/roko-acp/src/runner.rs` lines 2057-2145

### 6. TUI Integration

- **Status**: Missing
- **What works**: Nothing. Zero ACP references in `crates/roko-cli/src/tui/`.
- **What's missing**: ACP sessions, their progress, costs, and metrics are completely invisible in `roko dashboard`. The TUI has tabs for plans, agents, episodes, gates, and knowledge -- but no ACP tab or sidebar.
- **Fix**: Two approaches: (a) If ACP publishes lifecycle events to the event bus (see gap 4), the TUI can subscribe and display them in a new ACP tab. (b) Alternatively, the TUI could poll `.roko/episodes.jsonl` and `.roko/learn/efficiency.jsonl` for ACP-originated entries (filter by `kind == "acp"` or `trigger_kind == "acp_prompt"`).
- **Effort**: 6h (new TUI tab with bus subscription), 3h (read-only episode/efficiency display)
- **Files**: `crates/roko-cli/src/tui/` (new tab), depends on gap 4 for real-time

### 7. HTTP Control Plane (roko-serve)

- **Status**: Missing
- **What works**: `AcpEventForwarder` maps cognitive events to `RuntimeEvent` and posts them to `roko-serve` via `HttpEventSink` (when `ROKO_SERVE_URL` is set). `ServiceFactory` is shared between CLI, serve, and ACP.
- **What's missing**: `roko-serve` has no routes for querying ACP sessions. There are no endpoints for listing active ACP sessions, viewing session history, retrieving ACP metrics, or controlling ACP sessions via HTTP. The forwarded events arrive as generic `RuntimeEvent`s -- roko-serve cannot distinguish them from CLI runner events.
- **Fix**: Add ACP-specific routes: `GET /api/acp/sessions` (list active), `GET /api/acp/sessions/:id` (session details), `GET /api/acp/metrics` (cost/token aggregates). Either query ACP episode logs or have the ACP process register itself with roko-serve via a registration endpoint.
- **Effort**: 6h (routes + ACP registration protocol)
- **Files**: `crates/roko-serve/src/routes/` (new acp.rs module), `crates/roko-acp/src/handler.rs` (self-registration on startup)

### 8. Config Synchronization

- **Status**: Partial -- file-to-ACP only
- **What works**: `ConfigWatcher` uses `notify::RecommendedWatcher` to detect changes to roko.toml, global config, and ROKO_CONFIG env path. On change, `replace_roko_config()` updates the `SessionManager`, revalidates all active sessions (model/provider/effort), and rebuilds rate limiters if providers changed. `ConfigCache` provides zero-copy reads via `ArcSwap`.
- **What's missing**: Changes made via `roko serve` API (e.g., `PUT /config/set`) write to files on disk. The ACP `ConfigWatcher` should pick these up via filesystem events. However, if roko-serve modifies config in memory without writing to disk (e.g., runtime-only overrides), ACP will not see those changes. Additionally, there is no mechanism for ACP to push config changes back to roko-serve (e.g., when a user changes effort level in the editor).
- **Fix**: Clarify the config source of truth. If roko-serve always writes to disk, the current file-watcher approach is sufficient. If runtime-only overrides exist, add a pub/sub channel (e.g., via the event bus) for config change notifications.
- **Effort**: 2h (audit + document), 4h (add bus-based config sync if needed)
- **Files**: `crates/roko-acp/src/config_watch.rs`, `crates/roko-serve/src/routes/config.rs`

## Summary Table

| Integration | Status | Blocks Self-Hosting? | Effort |
|---|---|---|---|
| Cascade Router Learning | Wired (missing force_backend) | No | 2h |
| Knowledge-Informed Routing | Partial | No | 4h |
| Dream Consolidation | Wired | No | 2h (config), 6h (async) |
| Event Bus Integration | Partial (consume-only) | Blocks TUI + serve | 4h |
| Gate Pipeline (rungs 0-2) | Wired | No | -- |
| Gate Pipeline (rungs 3-6) | Missing | No | 4h |
| TUI Dashboard | Missing | No (cosmetic) | 3-6h |
| HTTP Control Plane | Missing | No (cosmetic) | 6h |
| Config Synchronization | Partial (file-to-ACP) | No | 2-4h |
| force_backend Learning | Missing | No | 2h |
| Episode Logger | Wired | No | -- |
| Episode Distillation | Wired | No | -- |
| Efficiency Events | Wired | No | -- |
| Prompt Experiments | Wired | No | -- |
| Provider Health | Wired | No | -- |
| SystemPromptBuilder | Wired | No | -- |
| Safety Layer | Wired | No | -- |
| Budget Guardrails | Wired | No | -- |
| DaimonState | Wired | No | -- |
| WorkflowEngine | Wired | No | -- |

**Total incremental effort for all gaps**: ~30-40h

**Priority order**:
1. Event bus emission (unblocks TUI + serve visibility) -- 4h
2. force_backend override learning (CLAUDE.md item 15) -- 2h
3. Knowledge-informed routing (CLAUDE.md item 13) -- 4h
4. TUI dashboard tab (user-facing visibility) -- 3-6h
5. HTTP control plane routes (dashboard/API visibility) -- 6h
6. Higher gate rungs (quality improvement) -- 4h
7. Dream config knob (polish) -- 2h
8. Config sync audit (correctness) -- 2h
