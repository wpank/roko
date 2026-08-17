# Master Implementation Plan

Architecture-first approach. Build the right abstractions, then the UX falls out naturally through adapters. Every consumer (ACP editor, TUI dashboard, HTTP/SSE, future integrations) gets the same data through the same interfaces.

**Core design principle**: Traits define boundaries. Adapters connect producers to consumers. Adding a new consumer = one adapter. Adding a new event = all adapters can opt in. No wiring features into specific runtimes.

> **Status (2026-04-28)**: Phases 0-3 completed via architecture runner (16 batches, all successful). Branch: `codex/arch-run-20260428-012508`. Foundation traits, services, execution engine, and adapter layer are all in place. Phases 4-6 and proof runs remain.

---

## The Architecture

```
                    ┌─────────────────────┐
                    │   Entry Points      │
                    │  CLI / ACP / HTTP   │
                    └────────┬────────────┘
                             │
                    ┌────────▼────────────┐
                    │   WorkflowEngine    │
                    │  (single facade)    │
                    └────────┬────────────┘
                             │
              ┌──────────────▼──────────────┐
              │        EffectDriver          │
              │  step(event) → actions       │
              │  execute(action) → event     │
              └──┬───┬───┬───┬───┬──────────┘
                 │   │   │   │   │
    ┌────────────┘   │   │   │   └────────────┐
    ▼                ▼   ▼   ▼                ▼
┌────────┐  ┌──────┐ ┌────┐ ┌──────────┐ ┌────────┐
│ModelCall│  │Prompt│ │Gate│ │Persistence│ │Feedback│
│Service │  │Assembly│Service│ │Service   │ │Service │
└───┬────┘  └──┬───┘ └──┬─┘ └────┬─────┘ └───┬────┘
    │          │        │        │            │
    │   ┌──────┘        │        │     ┌──────┘
    ▼   ▼               ▼        ▼     ▼
┌──────────────────────────────────────────┐
│              EventBus                     │
│  (normalized RuntimeEvents from all       │
│   services, fan-out to all consumers)     │
└──┬────┬────┬────┬────┬───────────────────┘
   │    │    │    │    │
   ▼    ▼    ▼    ▼    ▼
  ACP  TUI  SSE  JSONL  Future
  Adpt Adpt Adpt Logger (Slack,
                         Webhook,
                         ...)
```

**Key traits** (defined in `roko-core`, implemented in domain crates):

| Trait | Purpose | Implementors |
|---|---|---|
| `InferenceProvider` | Execute a model call against a specific backend | Claude CLI, Claude API, OpenAI, Ollama, Gemini, etc. |
| `ContextSource` | Supply context for prompt assembly | NeuroStore, EpisodeStore, PlaybookStore, GateFeedback, CodeIndex |
| `FeedbackSink` | Record outcomes from model calls | EpisodeSink, RouterSink, ThresholdSink, EfficiencySink |
| `EventConsumer` | Receive normalized RuntimeEvents | AcpAdapter, TuiAdapter, SseAdapter, JsonlLogger |
| `PermissionProvider` | Check whether an action is allowed | ScopePermissions, RoleContract, WorktreePolicy |
| `GateRunner` | Execute a verification step | CompileGate, TestGate, ClippyGate, LlmJudgeGate, CustomShellGate |

---

## Phase 0: Core Traits & Event System

> **Goal**: Define the boundaries. No implementations yet — just the trait contracts and event types that everything else builds on. This is the API design phase.
> **Crate**: `roko-core` (traits), new `roko-events` or in `roko-core` (event types)

### 0.1 RuntimeEvent Types

> **Status**: Completed 2026-04-28 via arch runner batch P0A (branch: codex/arch-run-20260428-012508)

The universal event vocabulary. Every service emits these. Every consumer subscribes to these.

- [x] **0.1.1** Define `RuntimeEvent` enum:
  ```
  // Lifecycle
  RunStarted { run_id, plan_id, template }
  RunCompleted { run_id, report }
  RunFailed { run_id, error }
  RunCancelled { run_id }

  // Task-level
  TaskStarted { task_id, role }
  TaskCompleted { task_id, outcome }
  TaskFailed { task_id, error, attempt }
  TaskSkipped { task_id, reason }

  // Agent / inference
  AgentSpawned { agent_id, role, model }
  AgentStreaming { agent_id, chunk }
  AgentCompleted { agent_id, usage, cost }
  AgentFailed { agent_id, error }

  // Routing
  RouterDecision { policy, candidates, chosen, tier_confidence }
  RouterEscalation { from_model, to_model, reason }
  RouterOverride { task_pattern, from, to }

  // Prompt
  PromptAssembled { layers, total_tokens, knowledge_hits, diagnostics }

  // Gates
  GateStarted { gate, rung }
  GatePassed { gate, rung, duration, detail }
  GateFailed { gate, rung, duration, error, line_marks }
  GateSkipped { gate, reason }

  // Knowledge
  KnowledgeQueried { hits, latency }
  KnowledgeInjected { hits, layer, total_layers }

  // Cost
  CostUpdated { turn_cost, session_cost, turn_budget, session_budget, tokens, sparkline }

  // Phase / workflow
  PhaseChanged { from, to }
  ModeChanged { from, to }
  PlanUpdated { entries, replanned }
  CheckpointCreated { hash, files_changed, iterations }

  // Multi-agent
  SwarmUpdated { agents: Vec<AgentProgress> }
  AgentChatMessage { from_role, to_role, text }

  // Permissions
  PermissionRequested { scope, title, description, tags }
  PermissionGranted { scope }
  PermissionDenied { scope }

  // Editor
  AgentCursorMoved { file, line, action }
  AgentNavigated { from_file, to_file, reason }

  // MCP
  McpCallStarted { server, tool }
  McpCallCompleted { server, tool, duration }

  // Episodes
  EpisodeRecorded { episode_id, timeline }
  ```
- [x] **0.1.2** Every event carries: `timestamp`, `run_id`, `sequence_number`
- [x] **0.1.3** Events are `Clone + Send + Sync + Serialize`

### 0.2 Core Traits

> **Status**: Completed 2026-04-28 via arch runner batch P0B (branch: codex/arch-run-20260428-012508)

- [x] **0.2.1** `InferenceProvider` trait:
  ```rust
  trait InferenceProvider: Send + Sync {
      fn id(&self) -> &str;
      fn capabilities(&self) -> ProviderCapabilities; // models, streaming, tools, thinking
      async fn stream(&self, req: InferenceRequest) -> Result<InferenceStream>;
      async fn call(&self, req: InferenceRequest) -> Result<InferenceResponse>;
      fn health(&self) -> ProviderHealth; // for circuit breaker
  }
  ```
- [x] **0.2.2** `ContextSource` trait:
  ```rust
  trait ContextSource: Send + Sync {
      fn name(&self) -> &str;
      fn priority(&self) -> u32; // higher = injected first
      async fn query(&self, ctx: &TaskContext) -> Vec<ContextEntry>;
  }
  // ContextEntry { source_path, text, score, tier }
  ```
- [x] **0.2.3** `FeedbackSink` trait:
  ```rust
  trait FeedbackSink: Send + Sync {
      fn name(&self) -> &str;
      async fn record(&self, outcome: &InferenceOutcome);
  }
  // InferenceOutcome { model, role, success, cost, duration, tokens, task_category }
  ```
- [x] **0.2.4** `EventConsumer` trait:
  ```rust
  trait EventConsumer: Send + Sync {
      fn name(&self) -> &str;
      fn interests(&self) -> EventFilter; // which events this consumer cares about
      async fn on_event(&self, event: &RuntimeEvent);
  }
  ```
- [x] **0.2.5** `PermissionProvider` trait:
  ```rust
  trait PermissionProvider: Send + Sync {
      async fn check(&self, action: &ToolAction, scope: &AgentScope) -> PermissionResult;
      // PermissionResult: Allowed | NeedsApproval { title, description, tags } | Denied { reason }
  }
  ```
- [x] **0.2.6** `GateRunner` trait:
  ```rust
  trait GateRunner: Send + Sync {
      fn name(&self) -> &str;
      async fn run(&self, ctx: &GateContext) -> GateResult;
      // GateResult { passed, duration, detail, line_marks, error }
  }
  ```

### 0.3 EventBus

> **Status**: Completed 2026-04-28 via arch runner batch P0C (branch: codex/arch-run-20260428-012508)

The fan-out mechanism. All services emit events. All consumers receive them.

- [x] **0.3.1** `EventBus` struct: `tokio::broadcast::Sender<RuntimeEvent>` (or custom channel for filtering)
- [x] **0.3.2** `EventBus::emit(event)` — sends to all subscribers
- [x] **0.3.3** `EventBus::subscribe(filter) → Receiver<RuntimeEvent>` — consumers get filtered stream
- [x] **0.3.4** `EventBus::register_consumer(impl EventConsumer)` — managed subscriptions
- [x] **0.3.5** Back-pressure handling: slow consumers get latest-only (no unbounded queuing)

**Effort**: Small. Trait definitions + event enum + broadcast channel. This is API design, not implementation.

---

## Phase 1: Foundation Services

> **Goal**: Implement the core services behind the traits. Each service emits RuntimeEvents to the EventBus.
> **Crates**: `roko-agent` (ModelCallService), `roko-compose` (PromptAssembly), `roko-learn` (Feedback), `roko-fs` (Persistence)

### 1.1 ModelCallService

> **Status**: Completed 2026-04-28 via arch runner batch P1A (branch: codex/arch-run-20260428-012508)

Single dispatch for all inference. Wraps provider selection + budget + safety + events.

- [x] **1.1.1** `ModelCallService` struct: owns `Vec<Arc<dyn InferenceProvider>>`, `CascadeRouter`, `EventBus`, budget state
- [x] **1.1.2** `stream(req: ModelCallRequest) → InferenceStream`:
  1. `CascadeRouter::select(context, providers)` → chosen provider + emit `RouterDecision`
  2. `PermissionProvider::check()` if applicable
  3. Budget check → reject if over limit
  4. `provider.stream(req)` → wrap stream to extract usage
  5. On completion: emit `AgentCompleted` with usage/cost, emit `CostUpdated`
  6. On failure: classify error, emit `AgentFailed`, maybe escalate (emit `RouterEscalation`)
  7. Fan out to all `FeedbackSink`s
- [x] **1.1.3** Provider registry: register `InferenceProvider` impls, query by capability
- [x] **1.1.4** Circuit breaker per provider: consecutive failures → mark unhealthy → failover
- [x] **1.1.5** Migrate existing 8 backends to `InferenceProvider` trait impls (Claude CLI, Claude API, OpenAI, Ollama, Gemini, Perplexity, Codex, Cursor)
- [x] **1.1.6** Budget accumulation: per-task and per-session, with configurable limits

### 1.2 PromptAssemblyService

> **Status**: Completed 2026-04-28 via arch runner batch P1B (branch: codex/arch-run-20260428-012508)

Builds prompts from pluggable context sources. Wraps SystemPromptBuilder.

- [x] **1.2.1** `PromptAssemblyService` struct: owns `Vec<Arc<dyn ContextSource>>`, `SystemPromptBuilder`, `EventBus`
- [x] **1.2.2** `assemble(role, task_context) → AssembledPrompt`:
  1. Load role identity (from config or template)
  2. For each `ContextSource` in priority order: `source.query(task_context)` → collect entries
  3. Feed entries into `SystemPromptBuilder` layers (knowledge=L7, episodes=L6, playbooks=L5, etc.)
  4. Budget enforcement via DensityGreedy
  5. Emit `PromptAssembled` event with layer diagnostics
  6. Emit `KnowledgeInjected` if knowledge hits > 0
- [x] **1.2.3** Implement `ContextSource` for existing stores:
  - `NeuroContextSource` — queries roko-neuro knowledge store
  - `EpisodeContextSource` — queries recent relevant episodes
  - `PlaybookContextSource` — queries playbook store
  - `GateFeedbackContextSource` — injects structured errors from prior gate failures
  - `CodeIndexContextSource` — queries code-intelligence index
- [x] **1.2.4** Section effectiveness tracking: after each run, record which sources correlated with success → adjust priorities

### 1.3 FeedbackService

> **Status**: Completed 2026-04-28 via arch runner batch P1C (branch: codex/arch-run-20260428-012508)

Records outcomes. Pluggable sinks.

- [x] **1.3.1** `FeedbackService` struct: owns `Vec<Arc<dyn FeedbackSink>>`, `EventBus`
- [x] **1.3.2** `record(outcome: InferenceOutcome)` → fans out to all sinks
- [x] **1.3.3** Implement `FeedbackSink` for existing systems:
  - `EpisodeSink` — writes to `.roko/episodes.jsonl`
  - `RouterSink` — calls `CascadeRouter::observe()`
  - `ThresholdSink` — updates adaptive gate thresholds
  - `EfficiencySink` — writes to `.roko/learn/efficiency.jsonl`
  - `SectionEffectivenessSink` — tracks prompt section correlation with success

### 1.4 GateService

> **Status**: Completed 2026-04-28 via arch runner batch P1D (branch: codex/arch-run-20260428-012508)

Runs verification steps. Pluggable gate runners.

- [x] **1.4.1** `GateService` struct: owns `Vec<Arc<dyn GateRunner>>`, `EventBus`
- [x] **1.4.2** `run_gates(gates: &[GateConfig], ctx: &GateContext) → Vec<GateResult>`:
  1. For each gate: emit `GateStarted`, run gate, emit `GatePassed`/`GateFailed`
  2. Parse line-level errors → include as `line_marks` in result (for editor gutter)
  3. Aggregate pass/fail
- [x] **1.4.3** Implement `GateRunner` for existing gates:
  - `CompileGate`, `TestGate`, `ClippyGate` — shell commands, parse output
  - `DiffGate` — checks diff is non-empty and relevant
  - `LlmJudgeGate` — uses `ModelCallService` (not bare claude), emits its own inference events
  - `CustomShellGate` — user-defined shell command from config
- [x] **1.4.4** Adaptive thresholds: `ThresholdSink` updates per-gate EMA, `GateService` reads thresholds to decide skip

### 1.5 PersistenceService

> **Status**: Completed 2026-04-28 via arch runner batches P2A/P3C (branch: codex/arch-run-20260428-012508)

Atomic state management.

- [x] **1.5.1** Atomic JSON writes (formalize existing pattern)
- [x] **1.5.2** Checkpoint/resume with fingerprint validation
- [x] **1.5.3** Deduplicated file layout (resolve .roko/learn/ vs .roko/memory/)
- [x] **1.5.4** Event journal: append RuntimeEvents to `.roko/events.jsonl` (via `JsonlLoggerConsumer`)

**Effort**: Medium-Large. This is the real work, but it's building the right thing once instead of patching features into three runtimes.

---

## Phase 2: Execution Engine

> **Goal**: One state machine, one driver, one facade. All entry points converge.
> **Crate**: `roko-runtime` (or new `roko-engine`)

### 2.1 PipelineState (Pure State Machine)

> **Status**: Completed 2026-04-28 via arch runner batch P2A (branch: codex/arch-run-20260428-012508)

No I/O. No async. Fully unit-testable. Config-driven phase transitions.

- [x] **2.1.1** Phases: Pending → Strategizing → Implementing → Gating → AutoFixing → Reviewing → Committing → Complete/Failed
- [x] **2.1.2** Events → state transitions → actions (the `step()` function)
- [x] **2.1.3** Workflow templates drive which phases are active:
  - Express: Implementing → Gating → Committing
  - Standard: + Reviewing
  - Full: + Strategizing
  - PlanExecution: multi-task DAG iteration
  - Custom: from workflow TOML (Phase 4)
- [x] **2.1.4** Failure classification: simple_compile→AutoFix, complex→Replan, review_revise→Retry, consecutive_failures→Halt
- [x] **2.1.5** Unit tests for every `(phase, event) → (phase, actions)` transition

### 2.2 TaskScheduler

> **Status**: Completed 2026-04-28 via arch runner batch P2B (branch: codex/arch-run-20260428-012508)

DAG-aware scheduling for multi-task plans.

- [x] **2.2.1** Task graph with state sets (completed/running/failed/skipped)
- [x] **2.2.2** `ready_tasks()`: deps satisfied, not running, not in cooldown
- [x] **2.2.3** Wave computation for parallel dispatch
- [x] **2.2.4** Skip propagation (failed → skip dependents)
- [x] **2.2.5** File-overlap serialization (opt-in safety)

### 2.3 EffectDriver

> **Status**: Completed 2026-04-28 via arch runner batch P2C (branch: codex/arch-run-20260428-012508)

Executes actions from the state machine. Uses all foundation services.

- [x] **2.3.1** Owns: `ModelCallService`, `PromptAssemblyService`, `GateService`, `PersistenceService`, `FeedbackService`, `TaskScheduler`, `EventBus`
- [x] **2.3.2** Main loop:
  ```
  loop {
      let actions = pipeline.step(event);
      for action in actions {
          let result = self.execute(action).await;  // delegates to services
          event = result.into_event();
      }
      self.persistence.checkpoint().await;
  }
  ```
- [x] **2.3.3** Action dispatch → service calls:
  - `SpawnAgent{role}` → `prompt_assembly.assemble(role, ctx)` → `model_call.stream(req)`
  - `RunGates{rungs}` → `gate_service.run_gates(config, ctx)`
  - `Commit` → git add + commit
  - `Replan` → decompose failed task
- [x] **2.3.4** Every action emits RuntimeEvents via EventBus (services do this internally)
- [x] **2.3.5** Cancellation: kill agents, save state, emit RunCancelled

### 2.4 WorkflowEngine Facade

> **Status**: Completed 2026-04-28 via arch runner batch P2D (branch: codex/arch-run-20260428-012508)

Single entry point for all callers.

- [x] **2.4.1** `WorkflowEngine::run_prompt(prompt, template) → RunReport`
- [x] **2.4.2** `WorkflowEngine::run_plan(plans_dir, config) → RunReport`
- [x] **2.4.3** `WorkflowEngine::resume(snapshot) → RunReport`
- [x] **2.4.4** Wire CLI: `roko run` → `engine.run_prompt()`
- [x] **2.4.5** Wire CLI: `roko plan run` → `engine.run_plan()`
- [x] **2.4.6** Wire ACP: `session/prompt` → `engine.run_prompt()` + AcpAdapter on EventBus
- [x] **2.4.7** Wire HTTP: `POST /api/inference` → `engine.run_prompt()` + SseAdapter on EventBus

**Effort**: Medium-Large. But this replaces 3 runtimes with 1. The state machine (2.1) and scheduler (2.2) are pure and can be built/tested independently.

---

## Phase 3: Adapters (The Rendering Layer)

> **Goal**: Every consumer gets RuntimeEvents through an adapter. Adding a new consumer = implementing `EventConsumer`. This is where the UX comes alive — but it's a thin translation layer on top of a solid foundation.
> **Crates**: `roko-acp` (AcpAdapter), `roko-cli` (TuiAdapter), `roko-serve` (SseAdapter, RestHandlers)

### 3.1 AcpAdapter (Editor Integration)

> **Status**: Completed 2026-04-28 via arch runner batch P3A (branch: codex/arch-run-20260428-012508)

Translates RuntimeEvents → ACP JSON-RPC notifications that the editor renders.

- [x] **3.1.1** Implement `EventConsumer` for `AcpAdapter`
- [x] **3.1.2** Event → ACP notification mapping (the 16 message types):

  | RuntimeEvent | ACP Notification | Message Type |
  |---|---|---|
  | `AgentStreaming` | `acp/message` | `agent_text` |
  | `AgentStreaming` (thinking) | `acp/message` | `thinking` |
  | `AgentCompleted` (tool call) | `acp/message` | `tool` (enriched: kind, cost, mcpServer) |
  | `PhaseChanged` | `acp/message` | `phase_change` |
  | `ModeChanged` | `acp/message` | `mode_change` |
  | `GateStarted/Passed/Failed` | `acp/message` | `gate_row` |
  | `PlanUpdated` | `acp/message` | `plan` |
  | `PromptAssembled` (knowledge) | `acp/message` | `knowledge` |
  | `RouterDecision` | `acp/message` | `router_trace` |
  | `SwarmUpdated` | `acp/message` | `swarm` |
  | `AgentChatMessage` | `acp/message` | `agent_chat` |
  | `CheckpointCreated` | `acp/message` | `checkpoint` |
  | `AgentCursorMoved` | `acp/message` | `callgraph` / cursor |
  | `PermissionRequested` | `acp/message` | `permission` |
  | Diff from tool output | `acp/message` | `diff` |
  | Terminal from tool output | `acp/message` | `terminal` |
  | Step from strategist | `acp/message` | `step` |

- [x] **3.1.3** Panel update notifications (right-rail):

  | RuntimeEvent(s) | ACP Panel Notification |
  |---|---|
  | `CostUpdated` | `acp/panel/cost` |
  | `RouterDecision` + history | `acp/panel/router` |
  | `KnowledgeInjected` | `acp/panel/knowledge` |
  | `McpCallCompleted` + registry | `acp/panel/mcp` |
  | `PermissionRequested/Granted` | `acp/panel/permissions` |
  | `EpisodeRecorded` | `acp/panel/episode` |

- [x] **3.1.4** Define JSON schemas for each notification type (the wire format contract)
- [x] **3.1.5** ACP request handlers: `acp/permissionResponse`, `acp/episodeBranch`, `acp/overrideModel`

### 3.2 TuiAdapter (Terminal Dashboard)

> **Status**: Completed 2026-04-28 via arch runner batches P3A/P3B (branch: codex/arch-run-20260428-012508)

Translates RuntimeEvents → ratatui DashboardEvents.

- [x] **3.2.1** Implement `EventConsumer` for `TuiAdapter`
- [x] **3.2.2** Map RuntimeEvents → existing `DashboardEvent` variants (extend where needed)
- [x] **3.2.3** Rendering primitives for all 16 message types (TUI equivalents of ACP cards)
- [x] **3.2.4** Converge the 2 chat loops in `chat_inline.rs` (share rendering with fullscreen TUI)

### 3.3 SseAdapter + REST Handlers (Web Dashboard)

> **Status**: Completed 2026-04-28 via arch runner batch P3B (branch: codex/arch-run-20260428-012508)

- [x] **3.3.1** Implement `EventConsumer` for `SseAdapter` — emits Server-Sent Events
- [x] **3.3.2** SSE event types: `cost_update`, `router_decision`, `gate_update`, `phase_change`, `plan_update`, `agent_chat`, `knowledge_injected`, `permission_request`, `swarm_update`
- [x] **3.3.3** REST endpoints (read from RuntimeProjection):
  - `GET /session/:id/cost`
  - `GET /session/:id/router`
  - `GET /session/:id/knowledge`
  - `GET /session/:id/mcp`
  - `GET /session/:id/permissions`
  - `GET /session/:id/gates`
  - `GET /episodes/:id/timeline`
  - `GET /session/:id/swarm`

### 3.4 JsonlLogger

> **Status**: Completed 2026-04-28 via arch runner batch P3C (branch: codex/arch-run-20260428-012508)

- [x] **3.4.1** Implement `EventConsumer` for `JsonlLogger` — appends every event to `.roko/events.jsonl`
- [x] **3.4.2** Event replay: read `.roko/events.jsonl` → emit to any adapter (powers episode replay)

### 3.5 RuntimeProjection

> **Status**: Completed 2026-04-28 via arch runner batch P3C (branch: codex/arch-run-20260428-012508)

Materialized view of current state, built from events.

- [x] **3.5.1** `RuntimeProjection` struct: active agents, cost totals, plan progress, gate results, current phase
- [x] **3.5.2** Implement `EventConsumer` — updates projection on each event
- [x] **3.5.3** REST handlers and TUI read from projection (not raw files)

**Effort**: Medium. Each adapter is a focused translation layer. The AcpAdapter is the largest (16 message types + 6 panels), but each mapping is straightforward once the events exist.

---

## Phase 4: Config-Driven Extensibility

> **Goal**: Roles, workflows, gates, context sources all defined in config. Drop a TOML file to add new ones. No code changes.
> **Crate**: `roko-core` (schemas + loaders), domain crates (registries)

### 4.1 Role Config System
- [ ] **4.1.1** Role TOML schema: identity, rules, tools (allowed/denied), budget, model_hint, prompt sections
- [ ] **4.1.2** Role loader + registry (replaces 28-variant enum)
- [ ] **4.1.3** Hot-reload on file change
- [ ] **4.1.4** Ship defaults as `roles/*.toml`
- [ ] **4.1.5** Role → `PermissionProvider` mapping (tool permissions from role config)
- [ ] **4.1.6** Role → `ContextSource` priority overrides (architect gets more code context, researcher gets more knowledge)

### 4.2 Workflow Config System
- [ ] **4.2.1** Workflow TOML schema: steps with role, gates, on_success, on_failure
- [ ] **4.2.2** Workflow → PipelineState generation (TOML drives the state machine)
- [ ] **4.2.3** Built-in templates as TOML (express, standard, full, plan-execution)
- [ ] **4.2.4** Custom failure recovery per step
- [ ] **4.2.5** Workflow-installed slash commands (workflows contribute their own commands)

### 4.3 Gate Config System
- [ ] **4.3.1** Gate TOML schema: type, threshold, timeout, custom_command
- [ ] **4.3.2** `CustomShellGate` impl: runs user-defined shell command, parses exit code
- [ ] **4.3.3** Per-workflow gate configuration
- [ ] **4.3.4** `CustomHttpGate` impl: calls external URL, checks response
- [ ] **4.3.5** `CustomMcpGate` impl: calls MCP tool, checks result

### 4.4 Context Source Registration
- [ ] **4.4.1** Plugin ContextSources registered at startup
- [ ] **4.4.2** MCP servers as ContextSources (query MCP tools for context)
- [ ] **4.4.3** External doc injection (embed files from workspace as context)

### 4.5 Slash Command Registry
- [ ] **4.5.1** Builtin commands (47)
- [ ] **4.5.2** User-defined from roko.toml (brand-colored, dashed border in UI)
- [ ] **4.5.3** Workflow-installed commands
- [ ] **4.5.4** Search/filter palette with keyboard nav

**Effort**: Medium. Schema definitions + loaders + registries. The runtime already has most of the concepts, just hardcoded.

---

## Phase 5: Advanced Features

> **Goal**: The flashy showcase features. Built on the solid Phase 0-3 foundation.

### 5.1 Parallel Agents (Tournament)

Uses WorkflowEngine + EventBus + SwarmUpdated events. No ad-hoc wiring.

- [ ] **5.1.1** `ParallelWorkflowTemplate`: spawns N agents with different approaches
- [ ] **5.1.2** Worktree isolation: `git worktree add` per agent, cleanup on completion
- [ ] **5.1.3** Per-agent `EffectDriver` instance, all sharing the same `EventBus`
- [ ] **5.1.4** `SwarmProjection`: aggregates per-agent RuntimeEvents → `SwarmUpdated` events
- [ ] **5.1.5** Winner selection: compare gate results + metrics
- [ ] **5.1.6** Synthesis step: synthesis agent receives all N diffs, produces comparison + recommendation
- [ ] **5.1.7** Merge winner: apply winning worktree's changes

### 5.2 Episode Replay

Uses JsonlLogger + EventBus for replay.

- [ ] **5.2.1** Record: JsonlLogger already persists all events (Phase 3.4)
- [ ] **5.2.2** Replay: load events.jsonl → re-emit to EventBus at scrub position
- [ ] **5.2.3** Branch-from-here: reconstruct context up to scrub position, create new session
- [ ] **5.2.4** Auto-extract learnings: pattern match on events → `[{ finding, action, target }]`
- [ ] **5.2.5** Post-replay updates: feed learnings back through FeedbackService

### 5.3 Editor Integration

Uses AgentCursorMoved + GateFailed events.

- [ ] **5.3.1** Tool call interception: when agent reads/writes a file, emit `AgentCursorMoved`
- [ ] **5.3.2** Gate gutter marks: `GateFailed.line_marks` → `textDocument/publishDiagnostics`
- [ ] **5.3.3** CallGraph trace: code-intelligence index provides fn → file:line tree
- [ ] **5.3.4** AGENT NAVIGATES dividers: `AgentNavigated` → rendered in chat stream
- [ ] **5.3.5** EditorPeek hints: suggest file to show based on current AgentCursorMoved

### 5.4 Permission Scoping

Uses PermissionProvider trait.

- [ ] **5.4.1** 11 permission scopes (file_reads, searches, edits_src, shell_safe, git_commit, etc.)
- [ ] **5.4.2** Per-scope state: `auto | ask | deny` + call_count
- [ ] **5.4.3** Mode → safety mapping (architect = read-only, research = no writes)
- [ ] **5.4.4** Per-worktree scope for parallel agents
- [ ] **5.4.5** Auto-revert scope (gate failure → rollback)

### 5.5 Cascade Learning UX

Uses RouterDecision + RouterOverride events.

- [ ] **5.5.1** Override recording: user forces model → `RouterOverride` event → FeedbackService → RouterSink
- [ ] **5.5.2** Cumulative savings display: "87% savings vs always-opus" computed from CostUpdated history
- [ ] **5.5.3** Learning indicator: router panel shows "LEARNING" badge when actively recording

**Effort**: Large for parallel agents (5.1), medium for everything else.

---

## Phase 6: Retirement

> **Goal**: Delete dead code. Only after Phase 2 proof runs pass.

- [ ] **6.1** Retire `orchestrate.rs` (21K lines)
- [ ] **6.2** Delete `runner/event_loop.rs` (3K lines, replaced by EffectDriver)
- [ ] **6.3** Delete `roko-acp/runner.rs` bare spawn (replaced by ModelCallService)
- [ ] **6.4** Replace daimon (40K) with `FailureTracker` (2 rules)
- [ ] **6.5** Delete pheromone system (68K). Replace with `warnings: Vec<String>` in prompt layer 8
- [ ] **6.6** Delete VCG auction (keep greedy knapsack)
- [ ] **6.7** Simplify CascadeRouter 17-dim → 6-dim features
- [ ] **6.8** `cargo build --workspace` clean, `cargo test --workspace` passes

---

## Proof Runs

Each proof validates end-to-end through the new architecture.

> **Note (2026-04-28)**: With Phases 0-3 complete, the infrastructure for P.1-P.8 is now in place. These proof runs can proceed once the services are wired into the live runtime paths.

- [ ] **P.1** `roko run "add health check"` → express pipeline → episode recorded → cost tracked
- [ ] **P.2** Same prompt via ACP → identical behavior, AcpAdapter emits all message types
- [ ] **P.3** Same prompt via HTTP → SseAdapter streams same events
- [ ] **P.4** Multi-task DAG plan → parallel tasks, correct ordering
- [ ] **P.5** Gate failure → auto-fix → re-gate
- [ ] **P.6** Crash + resume → no duplicate work
- [ ] **P.7** CascadeRouter learns: first run records, second run selects differently
- [ ] **P.8** Knowledge reuse: first run's episode in second run's prompt
- [ ] **P.9** Tournament: 3 parallel agents, synthesis, winner selection (via adapters, no ad-hoc wiring)
- [ ] **P.10** Episode replay: load past run, scrub, branch-from-here

---

## Dependency Graph

```
Phase 0 (Traits + Events)
    │
    ▼
Phase 1 (Foundation Services)
    │
    ├──→ Phase 2 (Execution Engine) ──→ Phase 6 (Retirement)
    │         │
    │         ▼
    └──→ Phase 3 (Adapters) ──→ Phase 5 (Advanced Features)
              │
              └──→ Phase 4 (Config-Driven) ──→ Phase 5
```

- Phase 0 + 1 can overlap (define traits, then implement)
- Phase 2 and 3 can overlap (engine and adapters develop in parallel)
- Phase 4 is independent of 3, can start once Phase 1 services exist
- Phase 5 needs Phase 2 (engine) + Phase 3 (adapters)
- Phase 6 only after proof runs pass

## Minimum Viable Demo Path

Fastest path to something impressive:

1. **Phase 0** — Define RuntimeEvent enum + EventConsumer trait
2. **Phase 1.1** — ModelCallService (wraps existing providers, emits events)
3. **Phase 1.4** — GateService (wraps existing gates, emits events)
4. **Phase 3.1** — AcpAdapter (translates events → the 16 message types)
5. **Phase 3.5** — RuntimeProjection (materialized view for panels)

This gets you: rich message cards + cost panel + gate strip + router decisions — all flowing through the proper architecture. Every subsequent feature is additive, not a rewrite.

---

## Crate Mapping

| Crate | New Code | What |
|---|---|---|
| `roko-core` | Traits + RuntimeEvent enum | Phase 0 |
| `roko-agent` | `InferenceProvider` impls, `ModelCallService` | Phase 1.1 |
| `roko-compose` | `PromptAssemblyService`, `ContextSource` impls | Phase 1.2 |
| `roko-learn` | `FeedbackService`, `FeedbackSink` impls | Phase 1.3 |
| `roko-gate` | `GateService`, `GateRunner` impls | Phase 1.4 |
| `roko-fs` | `PersistenceService` | Phase 1.5 |
| `roko-runtime` | `PipelineState`, `EffectDriver`, `WorkflowEngine` | Phase 2 |
| `roko-acp` | `AcpAdapter` (EventConsumer) | Phase 3.1 |
| `roko-cli` | `TuiAdapter` (EventConsumer) | Phase 3.2 |
| `roko-serve` | `SseAdapter`, REST handlers, `RuntimeProjection` | Phase 3.3, 3.5 |

---

## What This Architecture Gives You

| Concern | Old (3 runtimes) | New (traits + adapters) |
|---|---|---|
| Add new LLM provider | Modify 4 spawn sites | Implement `InferenceProvider` (1 file) |
| Add new context source | Modify prompt builder | Implement `ContextSource` (1 file) |
| Add new feedback sink | Modify each runtime | Implement `FeedbackSink` (1 file) |
| Add new UI consumer | Wire into specific runtime | Implement `EventConsumer` (1 file) |
| Add new gate type | Modify gate dispatch | Implement `GateRunner` (1 file) |
| Add new permission rule | Modify safety layer | Implement `PermissionProvider` (1 file) |
| Add new workflow | Modify state machine code | Drop a TOML file |
| Add new role | Modify enum + 5 files | Drop a TOML file |

Future integrations (Slack notifications, webhook alerts, VS Code extension, Neovim plugin, CI integration) each = one `EventConsumer` implementation.
