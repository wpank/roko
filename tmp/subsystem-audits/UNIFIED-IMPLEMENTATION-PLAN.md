# Unified Runtime: Implementation Plan

One runtime. Every feature works. Designed from scratch.

This plan converges runner v2 (`runner/event_loop.rs`), the dead `orchestrate.rs`, and the ACP pipeline (`roko-acp/pipeline.rs`) into a single, clean execution engine.

## Design Principles

1. **Pure state machine + side-effect driver** -- The ACP pipeline got this right. The state machine has zero I/O. A separate driver performs effects and feeds results back. This pattern is the foundation.
2. **One dispatch path** -- Every model call (runner, ACP, HTTP, research, dreams, neuro) goes through one `ModelCallService`. No more 4 different ways to spawn claude.
3. **Structured prompt assembly** -- The 9-layer `SystemPromptBuilder` from `roko-compose` is the one prompt path. No ad-hoc string concatenation.
4. **Feedback as event stream** -- One normalized event type. All learning, knowledge, dreams, and routing observations consume from it.
5. **Preserve what's valuable, delete what's overengineered** -- CascadeRouter yes, VCG auction payments no. Gate replan yes, daimon PAD model no.

## Architecture

```
                        WorkflowEngine
                             |
                    ┌────────┴────────┐
                    |                 |
              PipelineState      ExecutionPolicy
              (pure FSM)         (decisions only)
                    |                 |
                    └────────┬────────┘
                             |
                      EffectDriver
                             |
            ┌────────┬───────┼───────┬────────┐
            |        |       |       |        |
       ModelCall  GateRunner  Merge  Persist  Feedback
       Service    Service    Service Service  Service
            |
     ┌──────┼──────┐
     |      |      |
   Claude  OpenAI  Ollama
    CLI    Compat   HTTP
```

## Reference: What Each Runtime Does Today

| Capability | runner v2 (`event_loop.rs`) | orchestrate.rs (dead) | ACP pipeline |
|---|---|---|---|
| State machine | `ParallelExecutor` (external) | `PlanRunner` (monolithic) | `PipelineState` (pure, elegant) |
| Agent spawn | `spawn_agent()` via `CliProviderConfig` | `run_prepared_agent()` | `run_claude_cli()` (bare) |
| Prompt assembly | `PromptAssembler` (knowledge, playbooks, effectiveness) | 9-layer `SystemPromptBuilder` + `PromptComposer` + scorers | Ad-hoc string concat |
| Model routing | `ModelRouter` (always returns default) | `CascadeRouter` (LinUCB bandit, 17 features) | None (hardcoded claude) |
| Gates | `rung_dispatch::run_rung()` + verify steps | Gate pipeline + replan | `CompileGate`/`TestGate`/`ClippyGate` direct |
| Retry/replan | Exponential backoff, prompt enrichment on 3rd attempt | Classification, dedup, escalation ladder (retry→escalate→decompose) | AutoFix → re-implement, max_iterations |
| Feedback | Episodes, efficiency, cascade obs, bandit, neuro, thresholds | 18 hooks (episode, cascade, playbook, HDC, affect, attribution, etc.) | None |
| Inter-agent context | Task output persistence | Pheromones, prior outputs, gate feedback, knowledge | Strategy brief, error context, review findings |
| Safety | Extension chain hooks | SafetyLayer (pre/post, scrub, contracts, rate limit) | None |
| Observability | `RunnerEvent` → `StateHub` → TUI/SSE/WS | Scattered | `CognitiveEvent` → ACP `session/update` |
| Resume | Strict fingerprint validation, JSONL recovery | Snapshot + PID registry | None |

---

## Phase 0: Foundation Services

Everything else depends on these. Build them first, test them in isolation.

### 0.1 ModelCallService

One service for every model call in the system. Replaces 4 spawn paths.

- [ ] **0.1.1** Define `ModelCallService` trait in `roko-runtime` (or new `roko-inference`):
  ```rust
  pub trait ModelCallService: Send + Sync {
      async fn complete(&self, req: ModelCallRequest) -> Result<ModelCallResponse>;
      async fn stream(&self, req: ModelCallRequest) -> Result<ModelCallStream>;
      async fn probe(&self, provider_id: &str) -> ProviderProbeResult;
  }
  ```
- [ ] **0.1.2** Define `ModelCallRequest` with: `caller` (Runner/Acp/Http/Research/Dream/Neuro), `model_key`, `system_prompt`, `messages`, `tools`, `max_tokens`, `temperature`, `operation_id`, `credential_scope` (no raw API keys)
- [ ] **0.1.3** Define `ModelCallResponse` with: `content`, `tool_calls`, `usage` (input/output/cache tokens), `cost_usd`, `model_used`, `provider_used`, `duration_ms`, `stop_reason`
- [ ] **0.1.4** Define `ModelCallStream` wrapping `tokio::sync::mpsc::Receiver<StreamChunk>` where `StreamChunk` = `ContentDelta | ThinkingDelta | ToolCallDelta | Usage | Done | Error`
- [ ] **0.1.5** Implement `InferenceGateway` (production `ModelCallService`):
  - Resolve model → provider via config registry
  - Resolve credentials via `SecretService` (only component reading env vars)
  - Dispatch to correct provider adapter (`ClaudeCliAdapter`, `OpenAiCompatAdapter`, `OllamaAdapter`, etc.)
  - Emit `model_call.requested` event before, `model_call.completed`/`failed`/`cancelled` after
  - Record cost via provider-specific cost calculator
- [ ] **0.1.6** Implement `ClaudeCliAdapter`: spawn `claude --print --output-format stream-json --model <model> --system-prompt <prompt> --permission-mode bypassPermissions`, parse stream-json. Unify the two existing claude spawn paths.
- [ ] **0.1.7** Implement `OpenAiCompatAdapter`: HTTP SSE streaming to `/chat/completions`. Covers OpenAI, Moonshot, ZhiPu, Gemini, Ollama.
- [ ] **0.1.8** Implement `AnthropicApiAdapter`: HTTP streaming to Anthropic Messages API with tool loop.
- [ ] **0.1.9** Implement `PerplexityAdapter`: HTTP to Perplexity Sonar API.
- [ ] **0.1.10** Test: each adapter against a mock server or live provider. Probe returns classified status.

**Source reference**: `bridge_events.rs:run_claude_cognitive_task()` (lines 545-773) and `run_openai_compat_cognitive_task()` (lines 780-980) for the two streaming implementations. `agent_stream.rs:spawn_agent()` for the CLI spawn. `dispatch_v2.rs:CliProviderConfig::build_invocation()` for invocation building.

### 0.2 PromptAssemblyService

One prompt path. Merges runner v2's `PromptAssembler` with orchestrate.rs's 9-layer builder.

- [ ] **0.2.1** Define `PromptAssemblyRequest` with: `role`, `task`, `plan_context`, `attempt` (for retry feedback), `gate_feedback: Vec<GateFeedback>`, `review_feedback: Vec<String>`, `strategy_brief: Option<String>`, `workdir`, `token_budget`
- [ ] **0.2.2** Define `AssembledPrompt` with: `system_prompt`, `user_prompt`, `tool_allowlist`, `diagnostics: PromptDiagnostics`
- [ ] **0.2.3** Implement `PromptAssemblyService` using `roko-compose::SystemPromptBuilder`:
  - Layer 1 (Role): From role manifest in `core_roles.toml` → `RolePromptTemplate::role_identity()`
  - Layer 2 (Conventions): From `roko.toml` project conventions
  - Layer 3 (Domain): From `roko-neuro::KnowledgeStore` query (relevant knowledge for this task)
  - Layer 4 (Task): Task description, acceptance criteria, file context, prior task outputs
  - Layer 4b (Gate feedback): Structured `GateFeedback` (compile errors, test failures, clippy warnings) — not raw text
  - Layer 5 (Tools): Tool manifest filtered by role manifest `tools.capabilities`
  - Layer 6 (Skills): From `PlaybookStore` (keyword + success scoring, same as runner v2's `WorkdirPlaybookSource`)
  - Layer 7 (Anti-patterns): From safety layer constraints + past failure patterns
  - Layer 8 (Warnings): Plain `Vec<String>` of active warnings (replaces pheromones — gate failures, system issues, etc.)
- [ ] **0.2.4** Implement token budget enforcement: sections have `drop_priority`, sorted by priority, truncated to budget. Keep the greedy knapsack from `PromptComposer`, drop VCG payment computation.
- [ ] **0.2.5** Record `PromptDiagnostics`: included/dropped sections, knowledge IDs, playbook IDs, estimated tokens, section sources
- [ ] **0.2.6** Add section effectiveness tracking: record which sections were included in successful vs failed runs, adjust priorities over time (keep from runner v2's `SectionEffectivenessSource`)
- [ ] **0.2.7** Test: snapshot tests for implementer, reviewer, strategist, and retry prompts. Verify budget enforcement drops low-priority sections first.

**Source reference**: `dispatch/prompt_builder.rs:PromptAssembler::assemble()` for the current runner prompt path. `orchestrate.rs:dispatch_agent_with()` lines 14600-14970 for the 9-layer assembly. `roko-compose/src/system_prompt_builder.rs` for the builder API.

### 0.3 FeedbackService

One event stream, all learning consumes from it.

- [ ] **0.3.1** Define `ExecutionOutcome` with: `operation_id`, `task_id`, `plan_id`, `role`, `model`, `provider`, `success`, `usage` (tokens, cost, duration), `gate_verdicts`, `prompt_diagnostics_id`
- [ ] **0.3.2** Implement `FeedbackService` that receives `ExecutionOutcome` and fans out to:
  - **EpisodeSink**: Write `Episode` to `.roko/episodes.jsonl` (keep from runner v2)
  - **EfficiencySink**: Write `AgentEfficiencyEvent` to `.roko/learn/efficiency.jsonl` (keep)
  - **RoutingSink**: `CascadeRouter::observe_multi_objective()` with actual model/role/category from the outcome (fix the hardcoded values in runner v2)
  - **KnowledgeSink**: `RuntimeKnowledgeLifecycle::ingest_episode()` for neuro store (keep)
  - **ThresholdSink**: `AdaptiveThresholds::observe(rung, passed)` for gate EMA (keep)
  - **PlaybookSink**: `PlaybookStore::record(task_id, success)` (keep)
- [ ] **0.3.3** Remove the 12 feedback hooks that aren't pulling their weight: HDC fingerprinting, affect stamping, crate familiarity, anomaly detection, context attribution, section effectiveness per-episode (keep aggregate only), strategy metadata, force-backend override learning, somatic markers, emotional tags, predictive calibration, enriched run recording.
- [ ] **0.3.4** Ensure second run demonstrably uses first run's feedback: CascadeRouter picks different model after observing failure, knowledge store provides relevant context, playbook influences prompt assembly.
- [ ] **0.3.5** Test: two-run proof — first run records episode + routing observation, second run's prompt includes knowledge from first.

**Source reference**: `event_loop.rs:emit_feedback()` (lines 2292-2517) for current runner feedback. `orchestrate.rs:record_task_success()` (line 10241) for the 18-hook version.

### 0.4 PersistenceService

Crash-safe state management.

- [ ] **0.4.1** Define `RunState` snapshot schema: `run_id`, completed tasks, in-flight tasks, plan phases, iteration counts, task failure counts, merge queue state, task fingerprints
- [ ] **0.4.2** Implement atomic snapshot writes (tmp + rename) to `.roko/state/run-state.json`
- [ ] **0.4.3** Implement strict resume validation: reject if task definitions changed (fingerprint mismatch), reject stale plan IDs, reject unsupported schema version
- [ ] **0.4.4** Implement JSONL crash recovery: detect truncated last line, truncate to last complete record before appending
- [ ] **0.4.5** Save CascadeRouter state and adaptive gate thresholds alongside run state
- [ ] **0.4.6** Test: crash at every phase (active agent, post-agent/pre-gate, in-gate, post-gate/pre-snapshot), verify resume produces no duplicate completions

**Source reference**: `runner/persist.rs` for current snapshot/JSONL logic. `runner/resume.rs` for strict validation. `orchestrate.rs` `ExecutorSnapshot` for the older schema.

---

## Phase 1: The Execution Engine

The unified state machine. Merges the ACP pipeline's clean design with runner v2's DAG scheduling.

### 1.1 PipelineState (Pure State Machine)

Extend the ACP pipeline's `PipelineState` to handle multi-task plans, not just single prompts.

- [ ] **1.1.1** Define `PipelinePhase` enum (merge all three):
  ```
  Pending → Enriching → Implementing → Gating → AutoFixing →
  Verifying → Reviewing → DocRevision → Merging → Complete | Halted | Cancelled
  ```
- [ ] **1.1.2** Define `PipelineEvent` enum (inputs):
  ```
  Start, EnrichmentDone { brief }, EnrichmentSkipped,
  TaskCompleted { task_id, output }, TaskFailed { task_id, error },
  GatePassed { rung }, GateFailed { rung, output },
  AutoFixCompleted, AutoFixFailed { error },
  VerifyPassed, VerifyFailed { failures },
  ReviewApproved, ReviewRevised { findings },
  DocRevisionDone,
  MergeSucceeded, MergeFailed { conflict },
  Timeout, BudgetExceeded, UserCancel
  ```
- [ ] **1.1.3** Define `PipelineAction` enum (outputs):
  ```
  SpawnEnricher { prompt },
  SpawnImplementer { task_id, prompt, context },
  SpawnAutoFixer { task_id, error_context },
  RunGate { rung },
  RunVerifySteps { task_id },
  SpawnReviewer { context },
  SpawnScribe { context },
  Commit { message },
  SubmitMerge { plan_id },
  Done, Halt { reason }
  ```
- [ ] **1.1.4** Define `WorkflowTemplate` (merge ACP + mori tiers):
  ```
  Express:       Implement → Gate → Commit
  Standard:      Implement → Gate → QuickReview → Commit
  Full:          Enrich → Implement → Gate → FullReview → DocRevision → Commit
  PlanExecution: (DAG-driven, iterates tasks) Enrich → [Task → Gate → AutoFix]* → Verify → Review → Merge
  ```
- [ ] **1.1.5** Implement `PipelineState::step(event) -> Vec<PipelineAction>`:
  - Pattern match on `(phase, event)` exhaustively
  - Track `iteration` vs `max_iterations` per task
  - Track `gate_rung` advancement (0→1→2 for standard rungs)
  - On gate fail: if iterations remain → AutoFix. If autofix fails → re-implement with combined error context. If iterations exhausted → classify failure and decide: retry/escalate/decompose/halt (from orchestrate.rs's replan ladder)
  - On review revise: if iterations remain → re-implement with findings. If exhausted → commit with caveats.
  - **Zero I/O, zero async, fully deterministic**
- [ ] **1.1.6** Implement `FailureClassifier` (from orchestrate.rs `gate_failure_next_action`):
  - `role_tool_permission` → NeedsHuman
  - `external_environment` → Blocked
  - `architectural_conflict` → NeedsReplan
  - `simple_compile_error` → AutoFix
  - `test_failure` → Retry with context
  - Track failure dedup (same failure hash → don't retry same way)
  - Cap replans per plan (`replan_max_per_plan`)
- [ ] **1.1.7** Test: unit tests for every `(phase, event)` transition. Express path, standard path, full path, plan execution path. Gate failure → autofix → retry → escalate → halt. Review revise → retry → commit with caveats.

**Source reference**: `pipeline.rs` for the clean state machine pattern. `event_loop.rs:dispatch_action()` + `apply_agent_completion()` for runner transitions. `orchestrate.rs:gate_failure_next_action()` and `attempt_replan()` for failure classification.

### 1.2 TaskScheduler

DAG-aware task scheduling, extracted from the event loop.

- [ ] **1.2.1** Define `TaskScheduler` with: task graph, completed set, running set, failed set, skipped set, retry cooldowns
- [ ] **1.2.2** Implement `ready_tasks()`: returns tasks whose `depends_on` are all completed, not currently running, not in cooldown, not skipped
- [ ] **1.2.3** Implement wave computation: BFS layering for parallel dispatch (wave 0 = no deps, wave 1 = depends on wave 0 only)
- [ ] **1.2.4** Implement file-overlap serialization (opt-in): tasks touching same files cannot run concurrently
- [ ] **1.2.5** Implement retry cooldown: exponential backoff per task (from runner v2's approach)
- [ ] **1.2.6** Implement skip propagation: if task A fails permanently, all tasks depending on A are skipped
- [ ] **1.2.7** Test: 3-task chain A→B→C runs in order. Two independent tasks run in parallel. Failed task skips dependents.

**Source reference**: `runner/task_dag.rs` for current DAG logic. `roko-orchestrator/src/dag.rs` for `UnifiedTaskDag`.

### 1.3 EffectDriver

The side-effect executor that drives the pipeline. Replaces `event_loop.rs`'s 3000-line `tokio::select!` loop.

- [ ] **1.3.1** Define `EffectDriver` struct owning: `ModelCallService`, `GateService`, `MergeService`, `PersistenceService`, `FeedbackService`, `TaskScheduler`, cancel token
- [ ] **1.3.2** Implement main loop:
  ```rust
  loop {
      let actions = pipeline.step(event);
      for action in actions {
          let result = self.execute(action).await;
          event = result.into_event();
      }
      self.persist.checkpoint().await;
  }
  ```
- [ ] **1.3.3** Implement `execute(action)` dispatch:
  - `SpawnImplementer` → `prompt_service.assemble()` → `model_call_service.stream()` → parse output → `TaskCompleted/TaskFailed`
  - `SpawnAutoFixer` → same dispatch with auto-fixer role and error context
  - `SpawnReviewer` → same dispatch with reviewer role, parse verdict → `ReviewApproved/ReviewRevised`
  - `SpawnEnricher` → same dispatch with strategist role → `EnrichmentDone`
  - `RunGate` → `gate_service.run_rung()` → `GatePassed/GateFailed`
  - `RunVerifySteps` → `gate_service.run_verify()` → `VerifyPassed/VerifyFailed`
  - `SubmitMerge` → `merge_service.merge()` → `MergeSucceeded/MergeFailed`
  - `Commit` → git add + commit → `CommitDone`
- [ ] **1.3.4** After each agent completion, call `feedback_service.record(outcome)`
- [ ] **1.3.5** After each checkpoint, persist run state atomically
- [ ] **1.3.6** On cancellation, kill active agents, save state, emit cancelled event
- [ ] **1.3.7** Emit normalized `RuntimeEvent`s to `StateHub` at every phase transition for TUI/HTTP/SSE

**Source reference**: `runner.rs` (ACP) for the clean driver pattern. `event_loop.rs` for the tokio::select loop that this replaces.

### 1.4 Multi-Task Plan Support

Extend the single-task pipeline to handle plans with DAG-ordered tasks.

- [ ] **1.4.1** In `PlanExecution` template, the Implementing phase iterates:
  ```
  while scheduler.has_ready_tasks():
      tasks = scheduler.ready_tasks()
      for task in tasks:
          emit SpawnImplementer { task_id, prompt }
      await all completions
      for completed:
          scheduler.mark_complete(task_id)
          run gates for task
  ```
- [ ] **1.4.2** Support `max_concurrent_tasks` (default 1 for safety, configurable up to 8)
- [ ] **1.4.3** Track per-task state: attempt count, last failure, gate rung reached
- [ ] **1.4.4** Support task-level model hints from `tasks.toml` → passed to `ModelCallService`
- [ ] **1.4.5** Support task-level verify steps → run after task-level gates pass
- [ ] **1.4.6** Test: 5-task plan with diamond dependency (A→B, A→C, B→D, C→D, D→E). Verify execution order and parallelism.

---

## Phase 2: Model Routing

### 2.1 CascadeRouter Integration (Simplified)

- [ ] **2.1.1** Simplify the routing context from 17 features to 6:
  ```rust
  pub struct RoutingContext {
      task_tier: Tier,          // mechanical/focused/integrative/architectural
      role: AgentRole,
      attempt: u32,             // retry count
      budget_pressure: f64,     // 0.0 = flush, 1.0 = broke
      prior_failure: bool,
      task_category: TaskCategory,
  }
  ```
- [ ] **2.1.2** Wire CascadeRouter into ModelCallService: `router.select(context, candidates)` → model selection before provider dispatch
- [ ] **2.1.3** Wire CascadeRouter observation into FeedbackService: after each completion, `router.observe(context, model_idx, success, cost, duration)`
- [ ] **2.1.4** Implement tier-based defaults as fallback: mechanical→haiku, focused→sonnet, integrative→sonnet, architectural→opus
- [ ] **2.1.5** Support `force_backend` override from config: bypasses router, records as override observation
- [ ] **2.1.6** Test: after 5 failures on sonnet, router selects opus. After budget pressure rises, router prefers haiku.

**Source reference**: `orchestrate.rs:cascade_routing_context()` (line 2648) for the original 17-feature context. `dispatch/model_routing.rs:ModelRouter` for the current (non-functional) wrapper.

### 2.2 TaskRequirements Matching

- [ ] **2.2.1** Define `TaskRequirements`: `needs_web_search`, `needs_code_execution`, `needs_thinking`, `min_context_window`, `max_cost`
- [ ] **2.2.2** Filter candidate models by capability before scoring (from orchestrate.rs `score_model_for_task`)
- [ ] **2.2.3** Test: task requiring web search doesn't get dispatched to local ollama model.

---

## Phase 3: Safety

### 3.1 SafetyLayer

- [ ] **3.1.1** Preserve `SafetyLayer` from `roko-agent/src/safety/`: pre-dispatch check, post-dispatch check, scrub policy, path policy, git policy
- [ ] **3.1.2** Wire pre-dispatch into EffectDriver: before every `ModelCallService.stream()`, run `safety.pre_dispatch_check()`
- [ ] **3.1.3** Wire scrub into PromptAssemblyService: scrub assembled prompt before dispatch
- [ ] **3.1.4** Wire role tool allowlists: from role manifest `tools.capabilities` → filter tool list in prompt
- [ ] **3.1.5** Make `dangerously_skip_permissions` opt-in per role (not global default)
- [ ] **3.1.6** Drop: `TemporalMonitor`, `AgentWarrant` (OCaps), daimon tool filtering. Replace daimon tool filtering with simple rule: if `consecutive_failures >= 3`, strip network/git tools.
- [ ] **3.1.7** Test: implementer has write access, reviewer has read-only. Scrubber removes API keys from prompts.

### 3.2 Extension Hooks

- [ ] **3.2.1** Preserve extension chain from runner v2: `pre_inference`, `post_inference`, `on_gate`, `on_error`, `on_shutdown`
- [ ] **3.2.2** Wire into EffectDriver at the correct points
- [ ] **3.2.3** Test: extension receives correct hook data for each event type

---

## Phase 4: Observability

### 4.1 Normalized Runtime Events

- [ ] **4.1.1** Define `RuntimeEvent` enum:
  ```
  PlanStarted, PlanCompleted, PlanFailed,
  TaskStarted, TaskCompleted, TaskFailed, TaskSkipped,
  AgentSpawned, AgentCompleted, AgentFailed,
  GateStarted, GatePassed, GateFailed,
  MergeStarted, MergeSucceeded, MergeFailed,
  AutoFixStarted, AutoFixCompleted, AutoFixFailed,
  ReviewStarted, ReviewApproved, ReviewRevised,
  PromptAssembled { diagnostics },
  ModelCallStarted, ModelCallCompleted { usage },
  ```
  Every event carries: `timestamp`, `run_id`, `plan_id`, `task_id`, `operation_id`
- [ ] **4.1.2** EffectDriver emits `RuntimeEvent` to `StateHub` at every transition
- [ ] **4.1.3** StateHub fans out to: TUI (via `DashboardEvent`), HTTP SSE, WebSocket, tracing
- [ ] **4.1.4** Persist `RuntimeEvent`s to `.roko/events.jsonl` for query

### 4.2 Projection Layer

- [ ] **4.2.1** `RuntimeProjection` maintains: current run state, active agents, active gates, cost/token totals, plan progress percentages
- [ ] **4.2.2** TUI reads from projection (not from raw state files)
- [ ] **4.2.3** HTTP endpoints read from projection:
  - `GET /api/runs/{id}` → run state
  - `GET /api/runs/{id}/events` → filtered events
  - `GET /api/runs/{id}/plans/{id}` → plan state with task progress
- [ ] **4.2.4** Test: start `roko serve`, run a plan, query events endpoint, verify matches `.roko/events.jsonl`

---

## Phase 5: Entry Point Convergence

### 5.1 WorkflowEngine Facade

Every CLI command, HTTP route, and ACP prompt goes through one engine.

- [ ] **5.1.1** Define `WorkflowEngine` in `roko-runtime` (or `roko-workflow`):
  ```rust
  pub struct WorkflowEngine {
      model_call: Arc<dyn ModelCallService>,
      prompt: PromptAssemblyService,
      feedback: FeedbackService,
      persist: PersistenceService,
      safety: SafetyLayer,
  }
  ```
- [ ] **5.1.2** Implement `WorkflowEngine::run_prompt(prompt, template) -> RunReport`:
  - Creates `PipelineState` with template
  - Creates `EffectDriver` with all services
  - Drives to completion
  - Returns report with cost, tokens, success, gate verdicts
- [ ] **5.1.3** Implement `WorkflowEngine::run_plan(plans_dir, config) -> RunReport`:
  - Discovers plans, parses tasks
  - Creates `TaskScheduler` from DAG
  - Creates `PipelineState` with `PlanExecution` template
  - Drives to completion with multi-task iteration
- [ ] **5.1.4** Implement `WorkflowEngine::resume(snapshot_path) -> RunReport`:
  - Loads and validates snapshot
  - Rebuilds scheduler state from completed tasks
  - Continues from last checkpoint

### 5.2 CLI Wiring

- [ ] **5.2.1** `roko run "<prompt>"` → `engine.run_prompt(prompt, auto_select(prompt))`
- [ ] **5.2.2** `roko plan run plans/` → `engine.run_plan(plans_dir, config)`
- [ ] **5.2.3** `roko plan run plans/ --resume` → `engine.resume(snapshot)`
- [ ] **5.2.4** `roko acp` workflow mode → `engine.run_prompt(prompt, template)` with ACP event bridging

### 5.3 HTTP Wiring

- [ ] **5.3.1** `POST /api/inference` → `engine.run_prompt(prompt, Express)`
- [ ] **5.3.2** `POST /api/plans/{id}/run` → `engine.run_plan(plan_dir, config)`
- [ ] **5.3.3** `POST /api/runs/{id}/cancel` → cancel token

---

## Phase 6: Retirement

### 6.1 Kill the Dead Code

- [ ] **6.1.1** Add `#[deprecated]` banner to `orchestrate.rs`
- [ ] **6.1.2** Verify no CLI path calls `PlanRunner` (already true)
- [ ] **6.1.3** Extract any remaining unique behavior into the new services (check all features against parity matrix)
- [ ] **6.1.4** Move `orchestrate.rs` to `orchestrate_legacy.rs` (or delete if all behavior migrated)
- [ ] **6.1.5** Delete `runner/event_loop.rs` (replaced by EffectDriver)
- [ ] **6.1.6** Delete `roko-acp/src/runner.rs` bare claude spawn (replaced by ModelCallService)

### 6.2 Simplifications

- [ ] **6.2.1** Delete VCG auction payment computation (keep greedy knapsack)
- [ ] **6.2.2** Delete daimon PAD model, somatic markers, strategy spaces, dream depotentiation. Replace with `FailureTracker { consecutive_failures: u32, last_failure_kind: FailureKind }` with rules: 3 failures → restrict tools, 5 failures → escalate model
- [ ] **6.2.3** Delete pheromone system. Replace with `warnings: Vec<String>` injected into prompt layer 8
- [ ] **6.2.4** Delete HDC fingerprinting (add back when retrieval use case exists)
- [ ] **6.2.5** Delete context bidder subsystem allocation tracking (keep labels only)
- [ ] **6.2.6** Simplify CascadeRouter feature vector from 17 to 6 dimensions

---

## Phase 7: Proof Runs

Each proof must pass before claiming the feature works.

- [ ] **7.1** One-task implementation plan: `roko plan run` completes, episode recorded, state persisted
- [ ] **7.2** Multi-task dependency plan: A→B→C runs in order, parallel tasks run concurrently
- [ ] **7.3** Gate failure → auto-fix: compile gate fails, auto-fixer patches, re-gate passes
- [ ] **7.4** Gate failure → replan: 3 consecutive failures trigger decomposition
- [ ] **7.5** Reviewer flow: reviewer rejects, implementer retries with findings, reviewer approves
- [ ] **7.6** Crash + resume: kill during gate, resume from snapshot, no duplicate completions
- [ ] **7.7** Model routing observation: first run records cascade observation, second run selects different model
- [ ] **7.8** Knowledge reuse: first run's episode appears in second run's prompt
- [ ] **7.9** Provider matrix: each configured provider produces artifact or classified non-success status
- [ ] **7.10** HTTP query proof: `roko serve` + plan run + query events/projection endpoint
- [ ] **7.11** ACP proof: editor sends `session/prompt` with workflow=standard, pipeline runs implement→gate→review→commit
- [ ] **7.12** Single prompt proof: `roko run "add health check"` runs express pipeline end-to-end

---

## Implementation Order

| Priority | Phase | Dependency |
|---|---|---|
| **First** | 0.1 ModelCallService | None — everything needs this |
| **Second** | 0.2 PromptAssemblyService | 0.1 (prompts feed into model calls) |
| **Third** | 0.3 FeedbackService | 0.1 (records model call outcomes) |
| **Fourth** | 0.4 PersistenceService | None (independent) |
| **Fifth** | 1.1 PipelineState | None (pure, no deps) |
| **Sixth** | 1.2 TaskScheduler | None (pure, no deps) |
| **Seventh** | 1.3 EffectDriver | 0.1, 0.2, 0.3, 0.4, 1.1, 1.2 |
| **Eighth** | 1.4 Multi-Task Plan | 1.2, 1.3 |
| **Ninth** | 2.1 CascadeRouter | 0.1, 0.3 |
| **Tenth** | 3.1 Safety | 0.2, 1.3 |
| **Eleventh** | 4.1-4.2 Observability | 1.3 |
| **Twelfth** | 5.1-5.3 Entry Points | All above |
| **Thirteenth** | 6.1-6.2 Retirement | All above + proof runs |
| **Last** | 7.1-7.12 Proof Runs | All above |

---

## What Gets Deleted

| Component | Reason |
|---|---|
| `orchestrate.rs` (21K lines) | Dead code, all valuable features extracted |
| `runner/event_loop.rs` (3K lines) | Replaced by EffectDriver |
| `roko-acp/src/runner.rs` bare `run_claude_cli()` | Replaced by ModelCallService |
| VCG auction payments | Greedy knapsack is sufficient |
| Daimon PAD/somatic/strategy | Replaced by FailureTracker with simple rules |
| Pheromone system | Replaced by Vec<String> warnings |
| HDC fingerprinting | No retrieval consumer exists |
| 12 of 18 per-task feedback hooks | Noise reduction |
| CascadeRouter 17-dim features | Simplified to 6-dim |

## What Gets Preserved

| Component | Where It Came From | Why |
|---|---|---|
| Pure state machine pattern | ACP pipeline | Clean, testable, correct |
| PromptAssembler with knowledge/playbooks/effectiveness | runner v2 dispatch/ | Proven prompt quality |
| 9-layer SystemPromptBuilder | orchestrate.rs via roko-compose | Structured, cache-aligned |
| CascadeRouter (simplified) | orchestrate.rs | Adaptive model selection |
| Gate failure classification + replan ladder | orchestrate.rs | Essential for autonomy |
| SafetyLayer (pre/post/scrub/contracts) | orchestrate.rs via roko-agent | Required for safe execution |
| Strict resume validation | runner v2 | Crash safety |
| Episode + efficiency + threshold feedback | runner v2 | Proven learning loop |
| Extension chain hooks | runner v2 | Extensibility |
| Role manifests in TOML | roko-core | Declarative role definitions |
| Adaptive gate thresholds | roko-gate/roko-learn | Skip gates that always pass |
