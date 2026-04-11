# Roko Workflow — Issue Tracker

> Auto-generated from `tmp/workflow/implementation-plans/` on 2026-05-01. Covers every open issue from all 18 plans. Each checkbox is a discrete unit of work. Check a box only after the change lands in `main` and the stated verification passes.

---

## How To Use

1. Work top-down within a plan (they're dependency-ordered)
2. Before starting a group, verify its dependencies are checked off
3. After checking a box, run the verification command listed
4. When all boxes in a plan are checked, update `00-INDEX.md` status to COMPLETE

---

## Plan 01 — ModelCallService Completion

### 01-A: True Streaming

- [ ] **01-A-1** Define `StreamingProviderAdapter` trait in `crates/roko-agent/src/provider/mod.rs` with `stream(req, ctx) -> BoxAdapterStream`
  - Verify: `rg 'trait StreamingProviderAdapter' crates/roko-agent/src/provider/ --type rust` returns 1
- [ ] **01-A-2** Define `AdapterStreamChunk` enum (`Started`, `ContentDelta`, `ToolCallDelta`, `Usage`, `Done`, `Error`)
  - Verify: `rg 'enum AdapterStreamChunk' crates/roko-agent/ --type rust` returns 1
- [ ] **01-A-3** Implement `StreamingProviderAdapter` for `ClaudeCliAdapter` (reuse `parse_stream_line`)
  - File: `crates/roko-agent/src/provider/claude_cli/mod.rs`
  - Verify: unit test `claude_cli_streams_content_deltas` passes
- [ ] **01-A-4** Implement `StreamingProviderAdapter` for `AnthropicApiAdapter` (SSE from Messages API)
  - File: `crates/roko-agent/src/claude_agent.rs` or new `provider/anthropic_api/stream.rs`
  - Verify: unit test with mock SSE server
- [ ] **01-A-5** Implement `StreamingProviderAdapter` for `OpenAiCompatAdapter` (SSE deltas)
  - File: `crates/roko-agent/src/openai_compat_backend.rs`
  - Verify: unit test with mock SSE server
- [ ] **01-A-6** Implement `StreamingProviderAdapter` for `GeminiAdapter`
  - File: `crates/roko-agent/src/gemini/native.rs`
  - Verify: unit test
- [ ] **01-A-7** Override `ModelCallService::stream()` — resolve → adapter.streaming() → bridge_stream OR fallback
  - File: `crates/roko-agent/src/model_call_service.rs`
  - Verify: `rg 'fn stream' crates/roko-agent/src/model_call_service.rs` returns override (not default trait impl)
- [ ] **01-A-8** `bridge_stream` helper: maps `AdapterStreamChunk` → `ModelStreamEvent`, records `FeedbackEvent::ModelCall` on `Done/Error`
  - Verify: integration test asserting `ContentDelta` arrives before `Done`
- [ ] **01-A-9** Verify chat path delivers live tokens — `chat_session.rs:send_turn_streaming`
  - Verify: manual test in `roko` interactive; first delta within 500ms

### 01-B: Migrate `roko run`

- [ ] **01-B-1** Replace `spawn_agent_scoped` in `crates/roko-cli/src/run.rs` with `ServiceFactory.model_call_service().call(...)`
  - Verify: `rg 'spawn_agent_scoped' crates/roko-cli/src/run.rs` returns 0
- [ ] **01-B-2** Ensure `RunReport` contract preserved (`output_text`, `gate_verdicts`, `episode_id`)
  - Verify: existing `roko run` integration tests pass
- [ ] **01-B-3** Remove `TODO(gateway)` comment
  - Verify: `rg 'TODO.*gateway' crates/roko-cli/src/run.rs` returns 0

### 01-C: Migrate `roko plan run` (behind flag)

- [ ] **01-C-1** Add `[runner].use_workflow_engine` config flag to `roko-core` config schema
  - File: `crates/roko-core/src/config/schema.rs`
- [ ] **01-C-2** In `event_loop.rs`, if flag = true, route to `ModelCallService::call` instead of `Dispatcher::spawn_agent`
  - File: `crates/roko-cli/src/runner/event_loop.rs`
  - Verify: with flag true, `roko plan run plans/sample` records episodes via `ModelCallService`
- [ ] **01-C-3** With flag false (default), existing behavior unchanged
  - Verify: existing plan run integration tests still pass

### 01-D: Migrate `agent_exec.rs`

- [ ] **01-D-1** Refactor `run_agent`, `run_agent_capture`, `run_agent_logged` to use `ModelCallService`
  - File: `crates/roko-cli/src/agent_exec.rs`
  - Verify: `rg 'spawn_agent_scoped|create_agent_for_model' crates/roko-cli/src/agent_exec.rs` returns 0
- [ ] **01-D-2** Remove `persist_capture_episode` helper (FeedbackService records episodes)
  - Verify: `rg 'persist_capture_episode' crates/roko-cli/src/ --type rust` returns 0 outside tests
- [ ] **01-D-3** Lift `role_allows_dangerous_skip_permissions` to shared util
  - Verify: function exists in `crates/roko-agent/src/safety/` not just `run.rs`

### 01-E: Migrate LLM Judge Oracle

- [ ] **01-E-1** Create `crates/roko-gate/src/llm_judge_oracle.rs` with `LlmJudgeOracle { service: Arc<dyn ModelCaller> }`
  - Verify: file exists and compiles
- [ ] **01-E-2** Wire `LlmJudgeOracle` into `gate_service.rs` via `GateRunContext.judge_oracle`
  - Verify: `rg 'LlmJudgeOracle' crates/roko-gate/ --type rust` returns 2+ (def + usage)
- [ ] **01-E-3** `ServiceFactory::build` injects `Arc<dyn ModelCaller>` into `LlmJudgeOracle`
  - File: `crates/roko-orchestrator/src/service_factory.rs`
- [ ] **01-E-4** Delete `AgentJudgeOracle` from `orchestrate.rs` (or verify it's unreachable from default build)
  - Verify: `rg 'AgentJudgeOracle' crates/ --type rust | grep -v '#\[cfg(feature'` returns 0

### 01-F: Migrate Distillation, Dreams, Web-Search

- [ ] **01-F-1** Refactor `Distiller` in `crates/roko-neuro/src/episode_completion.rs` to take `Arc<dyn ModelCaller>`
  - Verify: `rg 'ANTHROPIC_API_KEY' crates/roko-neuro/ --type rust` returns 0
- [ ] **01-F-2** Refactor dream runner in `crates/roko-dreams/src/runner.rs` to use `ModelCallService`
  - Verify: `rg 'create_agent_for_model' crates/roko-dreams/ --type rust` returns 0
- [ ] **01-F-3** Refactor web_search tool in `crates/roko-std/src/tool/builtin/web_search.rs` to use `ModelCallService`
  - Verify: `rg 'PERPLEXITY_API_KEY' crates/roko-std/ --type rust` returns 0

### 01-G: Cleanup

- [ ] **01-G-1** Delete or feature-gate `extract_clean_text` in `crates/roko-cli/src/chat.rs`
  - Verify: `rg 'fn extract_clean_text' crates/ --type rust | grep -v '#\[cfg(feature'` returns 0
- [ ] **01-G-2** Verify `dispatch_direct.rs` unreachable from default build
  - Verify: `cargo build --bin roko --no-default-features` succeeds; `rg 'dispatch_direct' crates/roko-cli/src/ --type rust | grep -v '#\[cfg(feature'` returns 0
- [ ] **01-G-3** No bare `Command::new("claude")` outside `provider/claude_cli/`
  - Verify: `rg 'Command::new\("claude"\)' crates/ --type rust | grep -v 'provider/claude_cli' | grep -v test` returns 0
- [ ] **01-G-4** No direct API-key env reads outside the gateway
  - Verify: `rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v 'roko-agent/src/(provider|secret)' | grep -v test` returns 0
- [ ] **01-G-5** Add caller surface coverage test
  - Verify: `cargo test --package roko-cli feedback_caller_surfaces` passes

### 01-PROOF: Functional Proofs

- [ ] `roko run "add a comment to README"` → episode with `caller: "cli"` in `.roko/episodes.jsonl`
- [ ] `roko plan run plans/sample` (flag=true) → one episode per task
- [ ] `roko prd refine prds/sample.md` → episode with `caller: "research"`
- [ ] `roko knowledge dream run` → episode with `caller: "dreams"`
- [ ] Gate judge → episode with `caller: "gate.judge"`, `cache_policy: bypass`
- [ ] Streaming: `ContentDelta` arrives before `Done` in `roko` chat

---

## Plan 02 — Prompt Assembly Completion

### 02-A: Type Extensions

- [ ] **02-A-1** Extend `PromptSpec` with `plan_context`, `prior_task_outputs`, `strategy_brief`, `review_findings`, `attempt`, `token_budget`, `tool_allowlist`, `warnings` fields
  - File: `crates/roko-core/src/foundation.rs`
  - Verify: all new fields exist on `PromptSpec`
- [ ] **02-A-2** Convert `gate_feedback: Vec<String>` to `Vec<GateFeedback>` (structured)
  - Verify: `GateFeedback` struct defined in `foundation.rs`
- [ ] **02-A-3** Add `PromptDiagnostics` + `AssembledPrompt` to foundation trait
  - Verify: `rg 'struct AssembledPrompt' crates/roko-core/ --type rust` returns 1
- [ ] **02-A-4** Update `PromptAssembler` trait signature: `assemble(&self, spec: PromptSpec) -> Result<AssembledPrompt>`
  - Verify: trait updated, all impls compile

### 02-B: Naming Collision

- [ ] **02-B-1** Rename `roko_cli::dispatch::prompt_builder::PromptAssembler` → `TaskPromptComposer`
  - Verify: `rg 'pub (struct|trait) PromptAssembler' crates/ --type rust` returns exactly 1 (the trait)
- [ ] **02-B-2** Rename `roko_compose::templates::assembly::PromptAssembler` → `TemplateAssembler`

### 02-C: Migrate Entry Points

- [ ] **02-C-1** `roko run` uses `PromptAssemblyService::assemble()` via `ServiceFactory`
  - File: `crates/roko-cli/src/run.rs`
  - Verify: `rg 'build_system_prompt_with_context_validated' crates/roko-cli/src/run.rs` returns 0
- [ ] **02-C-2** `roko plan run` dispatch uses `PromptAssemblyService` instead of `TaskPromptComposer`
  - File: `crates/roko-cli/src/dispatch/prompt_builder.rs`
- [ ] **02-C-3** ACP `runner.rs` review prompts migrated to templates
  - Verify: `rg 'build_review_prompt' crates/roko-acp/src/runner.rs` returns 0
- [ ] **02-C-4** ACP `session.rs` inline role prompts migrated to templates
  - Verify: `rg 'You are' crates/roko-acp/src/session.rs --type rust` returns 0 (outside templates)
- [ ] **02-C-5** Chat `build_chat_system_prompt` migrated to `PromptAssemblyService`
  - File: `crates/roko-cli/src/chat_session.rs`
- [ ] **02-C-6** Add `interactive_chat` role to `core_roles.toml`
- [ ] **02-C-7** HTTP `/api/inference/complete` supports optional `assembly` body field
  - File: `crates/roko-serve/src/routes/gateway.rs`

### 02-D: VCG Deletion

- [ ] **02-D-1** Delete `crates/roko-compose/src/auction.rs` (or feature-gate behind `experimental-vcg`)
- [ ] **02-D-2** Remove `VcgWelfare` variant from strategy enum
- [ ] **02-D-3** Remove `vcg_allocate` calls from `prompt.rs`
- [ ] **02-D-4** Remove re-exports from `crates/roko-compose/src/lib.rs`
  - Verify: `rg 'vcg_allocate|VcgWelfare|VcgAuction' crates/ --type rust | grep -v test | grep -v '#\[cfg('` returns 0

### 02-E: Section Effectiveness

- [ ] **02-E-1** Verify loop closes: run twice, `.roko/learn/section-effects.json` has non-zero entries on 2nd run
  - Verify: integration test `section_effectiveness_persists_and_influences` passes
- [ ] **02-E-2** Add snapshot tests for implementer/reviewer/strategist/retry prompts
  - Verify: 4 snapshot tests in `crates/roko-compose/tests/snapshots/`

### 02-PROOF: Functional Proofs

- [ ] No inline role identity strings outside templates: `rg 'You are (the|a|an) \*\*' crates/ --type rust | grep -v 'roko-compose/src/templates' | grep -v test` returns 0
- [ ] `PromptAssembler` trait collision resolved: exactly 1 trait definition
- [ ] Token budget: prompts > 100K truncate low-priority sections (test)

---

## Plan 03 — Feedback Service Completion

### 03-A: Canonical Event

- [ ] **03-A-1** Extend `FeedbackEvent` with `TaskStarted`, `TaskCompleted`, `TaskFailed`, `PlanStarted`, `PlanCompleted` variants
  - File: `crates/roko-core/src/foundation.rs`
- [ ] **03-A-2** Rename CLI `runtime_feedback::FeedbackEvent` → `RuntimeFeedbackEvent` (temp)
  - File: `crates/roko-cli/src/runtime_feedback/mod.rs`

### 03-B: Move Sinks to `roko-learn`

- [ ] **03-B-1** Move `EpisodeSink` → `crates/roko-learn/src/sinks/episodes.rs`
- [ ] **03-B-2** Move `RoutingObservationSink` → `crates/roko-learn/src/sinks/routing.rs`
- [ ] **03-B-3** Move `KnowledgeIngestionSink` → `crates/roko-learn/src/sinks/knowledge.rs`
- [ ] **03-B-4** Move `ConductorObservationSink` → `crates/roko-learn/src/sinks/conductor.rs`
- [ ] **03-B-5** Move `DreamTriggerSink` → `crates/roko-learn/src/sinks/dreams.rs`
- [ ] **03-B-6** Create `MultiSink` fanout in `crates/roko-learn/src/multi_sink.rs`
  - Verify: `rg 'struct MultiSink' crates/roko-learn/ --type rust` returns 1

### 03-C: New Sinks

- [ ] **03-C-1** Create `ThresholdSink` in `crates/roko-learn/src/sinks/threshold.rs`
- [ ] **03-C-2** Create `PlaybookSink` in `crates/roko-learn/src/sinks/playbook.rs`
- [ ] **03-C-3** Wire `ServiceFactory::build` to construct `MultiSink` with all sinks
  - File: `crates/roko-orchestrator/src/service_factory.rs`

### 03-D: Chat Feedback

- [ ] **03-D-1** Attach `FeedbackService` to chat `ModelCallService` via `ServiceFactory`
  - File: `crates/roko-cli/src/chat_session.rs`
  - Verify: `roko "hello"` writes episode to `.roko/episodes.jsonl` with `caller: "cli"`
- [ ] **03-D-2** Add `[learn].chat_episode_recording` config knob (default true)

### 03-E: Multi-Objective Routing

- [ ] **03-E-1** Replace `router.observe(...)` with `router.observe_multi_objective(...)` in `FeedbackService`
  - File: `crates/roko-learn/src/feedback_service.rs`
  - Verify: `rg 'observe_multi_objective' crates/roko-learn/src/feedback_service.rs` returns 1+

### 03-F: Prune 12 Hooks

- [ ] **03-F-01** Delete HDC fingerprinting (`hdc.rs`)
- [ ] **03-F-02** Remove affect stamping call site (leave function for old episode reads)
- [ ] **03-F-03** Delete crate familiarity (`crate_familiarity_score`)
- [ ] **03-F-04** Delete anomaly detection write path (`anomaly_detector.rs`)
- [ ] **03-F-05** Delete context attribution (`context_attribution.rs`)
- [ ] **03-F-06** Keep section effectiveness aggregate; delete per-episode tracking
- [ ] **03-F-07** Delete strategy metadata (`strategy_metadata.rs`)
- [ ] **03-F-08** Delete force-backend override learning (`force_backend_override.rs`)
- [ ] **03-F-09** Delete somatic markers (`somatic_markers.rs`)
- [ ] **03-F-10** Stop populating emotional tags (set `None`)
- [ ] **03-F-11** Delete predictive calibration (`calibration.rs`)
- [ ] **03-F-12** Delete enriched run recorder (`enriched_run_recorder.rs`)
  - Verify per hook: `ls crates/roko-learn/src/${file}.rs` returns "not found" for each deleted file

### 03-G: Two-Run Proof

- [ ] **03-G-1** Integration test `second_run_uses_first_runs_feedback` passes
- [ ] **03-G-2** `cascade_router_changes_choice_after_failure` test passes
- [ ] **03-G-3** `playbook_records_after_successful_task_with_passing_gates` test passes
- [ ] **03-G-4** `gate_threshold_ema_decreases_after_repeated_failures` test passes

### 03-H: Delete CLI Facade

- [ ] **03-H-1** Delete `crates/roko-cli/src/runtime_feedback/` directory
  - Verify: `ls crates/roko-cli/src/runtime_feedback/` returns "not found"
- [ ] **03-H-2** Replace `FeedbackFacade` in `plan.rs` with `ServiceFactory.feedback_sink()`
- [ ] **03-H-3** One canonical `FeedbackSink` trait: `rg 'pub trait FeedbackSink' crates/ --type rust` returns exactly 1
- [ ] **03-H-4** One canonical `FeedbackEvent` enum: `rg 'pub enum FeedbackEvent' crates/ --type rust` returns exactly 1

---

## Plan 04 — Persistence Service

- [ ] **04-01** Define `PersistenceService` trait in `crates/roko-runtime/src/persistence.rs`
- [ ] **04-02** Define `RunStateV2` schema with `schema_version`, embedded `cascade_router_state`, `adaptive_thresholds_state`, task fingerprints
- [ ] **04-03** Define `WriteBatch` for transactional multi-file writes
- [ ] **04-04** Implement `FsPersistenceService` in `crates/roko-runtime/src/persistence_fs.rs`
- [ ] **04-05** Implement atomic checkpoint (tmp + rename) using `roko_fs::atomic::write`
- [ ] **04-06** Implement `transactional_write` (phase 1: tmp files → phase 2: JSONL appends → phase 3: run state commit → phase 4: rename)
- [ ] **04-07** Implement `validate_resume` with schema version + fingerprint checks
- [ ] **04-08** Implement `recover_logs` (truncate partial trailing JSONL lines)
- [ ] **04-09** Add `persistence` to `EffectServices` struct
  - File: `crates/roko-runtime/src/effect_driver.rs`
- [ ] **04-10** Wire `persistence.checkpoint()` into `WorkflowEngine::run_with_cancel` after each phase transition
  - File: `crates/roko-runtime/src/workflow_engine.rs`
- [ ] **04-11** Add `[runtime].checkpoint_interval_ms` config knob
- [ ] **04-12** Migrate runner `RunStateSnapshot` callers to `RunStateV2`
  - File: `crates/roko-cli/src/runner/persist.rs`, `resume.rs`
- [ ] **04-13** Consolidate `.roko/episodes.jsonl` vs `.roko/learn/episodes.jsonl` — one canonical location
  - Verify: `rg 'learn/episodes.jsonl' crates/ --type rust | grep -v test` returns 0
- [ ] **04-14** Crash-recovery test matrix (8 crash points × success)
  - Verify: `cargo test --features proof crash_recovery` passes
- [ ] **04-15** Transactional CascadeRouter + thresholds save via `RunStateV2` embedded fields
- [ ] **04-16** Projection TTL enforcement: `ProjectionEnvelope::is_stale(now)` + `load_or_refresh`

---

## Plan 05 — Pipeline Multi-Task

- [ ] **05-01** Extend `Phase` enum with `Enriching`, `DispatchingWave`, `Verifying`, `DocRevision`, `Merging`, `AwaitingReplan`
- [ ] **05-02** Add `task_id: Option<String>` to `Implementing`, `AutoFixing`, `Gating`, `Reviewing`, `Committing`
- [ ] **05-03** Add multi-task `PipelineInput` variants: `EnrichmentDone`, `TaskCompleted`, `TaskFailed`, `WaveCompleted`, `VerifyPassed/Failed`, `DocRevisionDone`, `MergeSucceeded/Failed`, `ReplanRequested/Approved/Rejected`
- [ ] **05-04** Add `GateFailureRecord` struct with `kind: FailureClass`, `stderr`, `exit_code`
- [ ] **05-05** Add `step_actions()` returning `Vec<PipelineAction>` (wave dispatch support)
- [ ] **05-06** Add multi-task `PipelineAction` variants: `SpawnEnricher`, `SpawnImplementerForTask`, `RunGateForTask`, `RunVerifyStepsForTask`, `SpawnReviewerForTask`, `SpawnScribeForTask`, `CommitForTask`, `SubmitMerge`, `EmitWarning`
- [ ] **05-07** Implement `FailureClassifier` (pure, no I/O) in `crates/roko-runtime/src/failure_classifier.rs`
  - Verify: `rg 'use tokio|use std::fs|async fn' crates/roko-runtime/src/failure_classifier.rs` returns 0
- [ ] **05-08** Implement `classify_failure_shape` (RoleToolPermission, ExternalEnvironment, ArchitecturalConflict, SimpleCompileError, TestFailure, Unknown)
- [ ] **05-09** Implement replan ladder: classify → AutoFix → Retry → Escalate → Decompose → Halt
- [ ] **05-10** Enforce `replan_max_per_plan` cap in `FailureClassifier`
- [ ] **05-11** Define `WorkflowTemplate::PlanExecution(PlanExecutionConfig)` variant
- [ ] **05-12** Implement `PlanExecution` phase transitions: Enriching → DispatchingWave → per-task loops → Merging → Complete
- [ ] **05-13** Unit tests for every `(Phase, PipelineInput)` transition (~80 cases)
- [ ] **05-14** Diamond DAG integration test (A → {B,C} → D → E)
- [ ] **05-15** Failure dedup test (same stderr twice → EscalateModel)
- [ ] **05-16** Old checkpoint migration test (single-prompt still resumes)
- [ ] **05-17** Mermaid diagram in `crates/roko-runtime/docs/pipeline-state.md`

---

## Plan 06 — TaskScheduler Integration

- [ ] **06-01** Add `compute_waves()` API returning `Vec<Vec<String>>`
- [ ] **06-02** Add `TaskRetryState` with exponential backoff + `ready_tasks_at(now_ms)`
- [ ] **06-03** Add `mark_failed` with retry count tracking + `skip_dependents` when exhausted
- [ ] **06-04** Add `DependencyRef::parse("plan:task")` for cross-plan dependencies
- [ ] **06-05** Extend `SchedulableTask` with `plan_id` field
- [ ] **06-06** Add cycle detection in `TaskScheduler::new` (return `Err(Cycle(...)`)
- [ ] **06-07** Wire `TaskScheduler` into `WorkflowEngine` for `PlanExecution` template
  - File: `crates/roko-runtime/src/workflow_engine.rs`
  - Verify: `rg 'TaskScheduler' crates/roko-runtime/src/workflow_engine.rs` returns 1+
- [ ] **06-08** Migrate `roko plan run` default to `WorkflowEngine` (behind `--use-event-loop` fallback)
  - File: `crates/roko-cli/src/commands/plan.rs`
- [ ] **06-09** Tests: diamond DAG, file-overlap serialization, retry backoff, cross-plan deps
- [ ] **06-10** Audit feature parity vs `event_loop.rs` (speculative exec = removed, document)

---

## Plan 07 — EffectDriver Completion

- [ ] **07-01** Plumb `gate_feedback: Vec<GateFeedback>` from pipeline state into `spawn_implementer`
  - Verify: `rg 'gate_feedback: Vec::new\(\)' crates/roko-runtime/src/effect_driver.rs` returns 0
- [ ] **07-02** Add `spawn_for_role(role, task_id, extra_spec)` helper (deduplicate 6 similar spawn functions)
- [ ] **07-03** Add `execute(PipelineAction)` dispatch for all new variants from Plan 05
  - Verify: no `_ =>` wildcard arm in `execute()` match
- [ ] **07-04** Implement concurrent action fanout via `buffer_unordered(N)` in `WorkflowEngine`
  - Verify: `rg 'buffer_unordered' crates/roko-runtime/src/workflow_engine.rs` returns 1+
- [ ] **07-05** Wire `PersistenceService::checkpoint` after each phase transition in run loop
- [ ] **07-06** Create `MergeService` trait + `GitMergeService` (extract from `merge_queue.rs`)
  - File: `crates/roko-runtime/src/merge_service.rs`
- [ ] **07-07** Create `WorktreeService` trait (extract from `worktree.rs`)
- [ ] **07-08** Add `MergeService` + `WorktreeService` to `EffectServices`
- [ ] **07-09** Implement cooperative cancellation: check `CancelToken` per stream chunk
- [ ] **07-10** Wire `SafetyLayer` per plan 09

---

## Plan 08 — CascadeRouter Integration

- [ ] **08-01** Define 6-feature `RoutingContext` in `crates/roko-learn/src/routing_context.rs`
  - Verify: struct has exactly `task_tier`, `role`, `attempt`, `budget_pressure`, `prior_failure`, `task_category`
- [ ] **08-02** Define `TaskRequirements` + `filter_candidates` in `crates/roko-learn/src/task_requirements.rs`
- [ ] **08-03** Wire `CascadeRouter::select_for_frequency_among` into `ModelCallService::resolve`
  - File: `crates/roko-agent/src/model_call_service.rs`
- [ ] **08-04** Add `build_routing_context(req)` and `build_task_requirements(req)` helpers
- [ ] **08-05** Implement `force_backend` override from `[runtime.routing]` config
  - Verify: `rg 'force_backend' crates/roko-agent/src/model_call_service.rs` returns 1+
- [ ] **08-06** Implement tier-based fallback (`mechanical→haiku`, `architectural→opus`)
- [ ] **08-07** Tests: 5 failures → router avoids model; budget_pressure → cheaper; filter by requirements

---

## Plan 09 — Safety Layer Wiring

- [ ] **09-01** Add `safety: Arc<SafetyLayer>` to `EffectServices`
- [ ] **09-02** Call `safety.pre_dispatch_check(...)` before every `spawn_for_role`
  - Verify: `rg 'pre_dispatch_check' crates/roko-runtime/src/effect_driver.rs` returns 1+
- [ ] **09-03** Call `safety.scrub()` on assembled prompt before dispatch
- [ ] **09-04** Call `safety.post_dispatch_check(...)` after every agent completion
- [ ] **09-05** Migrate `dangerously_skip_permissions` to contract YAML field
  - Verify: `rg 'dangerously_skip_permissions' crates/ --type rust | grep -v 'safety/contract' | grep -v test` returns 0
- [ ] **09-06** Implement `PerTurnSpend` + `record_call_cost` + cumulative enforcement
  - File: `crates/roko-agent/src/safety/budget.rs`
- [ ] **09-07** Implement agent-level `check_agent_recovery` (consecutive failures → Alert/Downgrade)
- [ ] **09-08** Make MCP misconfiguration loud (`McpExpectation::Required` → error; `Optional` → warn)
  - File: `crates/roko-agent/src/provider/mod.rs`
- [ ] **09-09** Tests: path escape blocked, cumulative cost enforced, contract-sourced permissions

---

## Plan 10 — Observability / Projection

- [ ] **10-01** Extend `RuntimeEvent` with: `ToolCallStarted`, `ToolCallCompleted`, `ToolOutputDelta`, `AgentThinkingDelta`, `PlanStarted/Completed`, `TaskStarted/Completed`, `MergeStarted/Succeeded/Failed`, `ReviewStarted/Approved/Revised`, `AutoFixStarted/Completed`, `ModelCallStarted/Completed`, `SafetyAlert/Warning`, `AgentBlocked`, `CostUpdate`, `PromptAssembled`
- [ ] **10-02** Migrate `DashboardEvent` consumers (TUI, SSE, WS) to `RuntimeEvent`
  - Verify: `rg 'DashboardEvent' crates/ --type rust | grep -v '#\[deprecated\]'` returns 0
- [ ] **10-03** Merge `.roko/runtime-events.jsonl` into `.roko/events.jsonl` (one canonical JSONL)
  - Verify: no references to `runtime-events.jsonl` in code
- [ ] **10-04** Consolidate HTTP routes to `/api/runs`, `/api/runs/{id}`, `/api/runs/{id}/events`, `/api/runs/{id}/transcript`
- [ ] **10-05** Add TTL enforcement: `ProjectionEnvelope::is_stale` + `load_or_refresh`
- [ ] **10-06** TUI dashboard reads from `RuntimeProjection::dashboard_view()` instead of disk
  - Verify: `rg 'load_from_disk' crates/roko-cli/src/tui/ --type rust` returns 0
- [ ] **10-07** SSE end-to-end test: events arrive within 100ms

---

## Plan 11 — Entry Point Convergence

- [ ] **11-01** `roko plan run` routes to `WorkflowEngine::run(PlanExecution)` by default
  - Verify: `rg 'runner::event_loop::run' crates/roko-cli/src/commands/plan.rs` gated behind `--use-event-loop` only
- [ ] **11-02** `agent_exec.rs` callers use `WorkflowEngine::run(Express)` or one-shot `ModelCallService`
- [ ] **11-03** ACP default mode routed through `WorkflowEngine::run(Express)` + `AcpEventBridge`
  - Verify: `rg 'run_anthropic_cognitive_task|run_openai_compat_cognitive_task' crates/roko-acp/ --type rust` returns 0 callers
- [ ] **11-04** `roko chat` deleted or thinned to 30-LOC wrapper
  - Verify: `wc -l crates/roko-cli/src/chat.rs` < 50
- [ ] **11-05** HTTP `/api/inference/complete` supports `assembly` body field
- [ ] **11-06** `WorkflowConfig::auto_select(prompt)` shared by ACP, HTTP, CLI
  - File: `crates/roko-runtime/src/pipeline_state.rs`
- [ ] **11-07** Smoke tests for every entry point (run, plan run, prd, ACP, HTTP)

---

## Plan 12 — Retirement / Deletion

- [ ] **12-01** Delete 12 noisy feedback hooks (per 03-F-01 through 03-F-12)
- [ ] **12-02** Delete `crates/roko-cli/src/runtime_feedback/` (per 03-H-1)
- [ ] **12-03** Delete `DashboardEvent` enum (per 10-02)
- [ ] **12-04** Delete `extract_clean_text` (per 01-G-1)
- [ ] **12-05** Delete `crates/roko-cli/src/runner/event_loop.rs`
- [ ] **12-06** Delete `crates/roko-cli/src/runner/task_dag.rs`
- [ ] **12-07** Delete `crates/roko-cli/src/runner/persist.rs::RunStateSnapshot`
- [ ] **12-08** Delete `crates/roko-cli/src/dispatch_direct.rs`
- [ ] **12-09** Extract remaining unique features from `orchestrate.rs` (knowledge routing, skill extraction, gate classifier)
- [ ] **12-10** Delete `crates/roko-cli/src/orchestrate.rs` → `tmp/legacy/`
- [ ] **12-11** Remove `legacy-orchestrate` feature from `Cargo.toml`
  - Verify: `rg 'cfg.*legacy-orchestrate' crates/ --type rust` returns 0
- [ ] **12-12** Delete `roko-orchestrator/src/dag.rs::UnifiedTaskDag` (replaced by `TaskScheduler`)
- [ ] **12-13** Delete `roko-orchestrator/src/executor/mod.rs::ParallelExecutor`
- [ ] **12-14** Move `merge_queue.rs` content into `roko-runtime/src/merge_service.rs`; delete original
- [ ] **12-15** Delete `roko-orchestrator/src/coordination.rs` (pheromones; per plan 15)
- [ ] **12-16** Delete `crates/roko-daimon/` (per plan 15)
- [ ] **12-17** Delete `crates/roko-compose/src/auction.rs` (VCG; per plan 02-D)
- [ ] **12-18** Delete `roko-orchestrator/src/replan.rs` (replaced by `FailureClassifier`)
- [ ] **12-19** Delete `RunStateSnapshot`, `ExecutorSnapshot`, `OrchestratorSnapshot` (keep `RunStateV2`)
- [ ] **12-20** Verify: `cargo build --workspace && cargo test --workspace`
- [ ] **12-21** Verify: `find crates/ -name '*.rs' | xargs wc -l | tail -1` shows ≥100K LOC reduction

---

## Plan 13 — Gate Pipeline Unification

- [ ] **13-01** Align `GateService` rung map with `registry.rs::GATE_SPECS` canonical
- [ ] **13-02** Add `GateRunContext` with optional inputs (symbol_manifest, oracle, artifact_store)
- [ ] **13-03** Migrate ACP `run_gates` to call `GateService`
  - Verify: `rg 'CompileGate|TestGate|ClippyGate' crates/roko-acp/ --type rust` returns 0
- [ ] **13-04** Migrate `runner/gate_dispatch.rs` to call `GateService`
- [ ] **13-05** Wire `LlmJudgeOracle` into `GateService` rung 6 (per 01-E)
- [ ] **13-06** Plumb `feedback_for_agent()` output into retry prompts (per 07-01)
- [ ] **13-07** `ServiceFactory::build` calls `GateService::with_adaptive_thresholds`
  - Verify: `rg 'with_adaptive_thresholds' crates/roko-orchestrator/src/service_factory.rs` returns 1+
- [ ] **13-08** Share `Arc<Mutex<AdaptiveThresholds>>` between `GateService` (reads) and `ThresholdSink` (writes)
- [ ] **13-09** Wire or delete 6 standalone gates (DiffGate, CodeExec, Benchmark, FormatCheck, SecurityScan)
- [ ] **13-10** Full 7-rung integration test with all inputs provided

---

## Plan 14 — Providers / Action Plan

- [ ] **14-01** Define `ResolvedRuntimeConfig` fully in `crates/roko-core/src/config/provenance.rs`
- [ ] **14-02** Thread `ResolvedRuntimeConfig` from `main.rs` through every CLI command
  - Verify: `rg 'ResolvedRuntimeConfig' crates/roko-cli/src/ --type rust` returns 5+
- [ ] **14-03** `detect_auth(config)` accepts `ResolvedRuntimeConfig`; checks `default_backend` first
  - File: `crates/roko-cli/src/auth_detect.rs`
- [ ] **14-04** Add `--fallback-model` to all Claude CLI spawns (from config)
  - Verify: `rg 'fallback.model' crates/roko-agent/src/provider/claude_cli/ --type rust` returns 1+
- [ ] **14-05** Create `StderrClassifier` (`Benign` / `Important` / `Error`) in `crates/roko-agent/src/stderr_classifier.rs`
- [ ] **14-06** Wire stderr classifier into Claude CLI and all subprocess spawn paths
- [ ] **14-07** Create `spawn_with_doa_detection(invocation, threshold)` in `crates/roko-agent/src/spawn_wrapper.rs`
- [ ] **14-08** Wire DOA detection into subprocess spawn paths
- [ ] **14-09** Unify per-role tool policy: `policy_for_role(role, contract)` replaces both `claude_tool_allowlist` and `resolve_tool_policy`
  - Verify: `rg 'claude_tool_allowlist|resolve_tool_policy' crates/roko-cli/src/ --type rust` returns 0
- [ ] **14-10** Session resume: pass `session_id` via `routing_hints`; Claude adapter reads `claude:resume:<id>`
- [ ] **14-11** Add `roko config doctor` command
  - File: `crates/roko-cli/src/commands/config_cmd.rs`

---

## Plan 15 — Cognitive Layer Cleanup

- [ ] **15-01** Delete pheromones: `crates/roko-orchestrator/src/coordination.rs` (and all call sites)
  - Verify: `rg 'PheromoneStore|active_pheromone_chunks' crates/ --type rust` returns 0
- [ ] **15-02** Create `WarningStore` in `crates/roko-runtime/src/warning_store.rs`
- [ ] **15-03** Wire `warnings.snapshot()` into `PromptSpec.warnings` in `EffectDriver`
- [ ] **15-04** Delete `crates/roko-daimon/` crate entirely
  - Verify: `ls crates/roko-daimon` returns "not found"
- [ ] **15-05** Create `FailureTracker` in `crates/roko-runtime/src/failure_tracker.rs`
- [ ] **15-06** Create `FailureTrackerSink` in `crates/roko-learn/src/sinks/failure_tracker_sink.rs`
- [ ] **15-07** Refactor `Distiller` to accept `Arc<dyn ModelCaller>` (per 01-F-1)
- [ ] **15-08** Refactor dream runner to use `ModelCallService` (per 01-F-2)
- [ ] **15-09** Add periodic dream schedule in `roko serve` if `[learn].auto_dream_interval_hours > 0`
- [ ] **15-10** Delete HDC fingerprinting (per 03-F-01)
- [ ] **15-11** Knowledge round-trip test: run 1 creates entry; run 2's prompt includes it

---

## Plan 16 — CLI / TUI Rendering Convergence

- [ ] **16-01** Define `ResponseRenderer` trait in `crates/roko-cli/src/render/mod.rs`
- [ ] **16-02** Implement `InlineRenderer` (uses `inline/primitives/`)
- [ ] **16-03** Implement `PlainRenderer` (no styling)
- [ ] **16-04** Extract one `run_chat_loop` function with `ChatBackend` + `ResponseRenderer` generics
  - Verify: `wc -l crates/roko-cli/src/chat_inline.rs` < 1500
- [ ] **16-05** Wire `ToolCallBlock`, `CostWaterfall`, `DiffBlock`, `ReplanBlock`, `SessionSummary` primitives into `InlineRenderer`
  - Verify: `rg 'ToolCallBlock' crates/roko-cli/src/render/ --type rust` returns 1+
- [ ] **16-06** TUI `DashboardData` reads from `RuntimeProjection` not disk
  - Verify: `rg 'load_from_disk' crates/roko-cli/src/tui/ --type rust` returns 0
- [ ] **16-07** TUI Agents tab F3 renders `ToolCallBlock`
- [ ] **16-08** Delete `extract_clean_text` (per 01-G-1)
- [ ] **16-09** Delete or thin `roko chat` REPL (per 11-04)

---

## Plan 17 — Demo Completion

- [ ] **17-01** Create `crates/roko-cli/src/output_format/mod.rs` with `RunOutputFormatter` trait
- [ ] **17-02** Implement `ClackStyle` formatter (Plan/Predict/Knowledge/Run/Gates/Done blocks)
- [ ] **17-03** Add `PredictBlock` source (model, route_source, estimated_cost)
- [ ] **17-04** Add `KnowledgeBlock` source (entry_count, sources, relevance)
- [ ] **17-05** Add `DoneBlock` (actual vs predicted delta)
- [ ] **17-06** Verify `roko run --resume <id>` works after Ctrl+C
- [ ] **17-07** Verify `roko run --share` prints `nunchi://` and HTTPS URLs
- [ ] **17-08** Add Tokyo Night theme via `ROKO_THEME=tokyo-night`
- [ ] **17-09** Create `scripts/demo-rehearsal.sh`
- [ ] **17-10** (Web) Decide on 3-5 demo-critical scenarios
- [ ] **17-11** (Web, optional) Implement Pulse Globe
- [ ] **17-12** (Web, optional) Implement Terrain knowledge viz
- [ ] **17-13** (Web) Add scripted resume-checkpoint scenario
- [ ] **17-14** (Web) Adopt Tokyo Night + Geist on web

---

## Plan 18 — Proof Runs

- [ ] **18-01** Proof 7.1: one-task plan executes, episode written, state persisted
- [ ] **18-02** Proof 7.2: multi-task diamond DAG executes in correct order with parallelism
- [ ] **18-03** Proof 7.3: gate failure → autofix → gate pass
- [ ] **18-04** Proof 7.4: gate failure exhausted → replan or halt
- [ ] **18-05** Proof 7.5: reviewer rejects → implementer retries with findings → approved
- [ ] **18-06** Proof 7.6: crash + resume → no duplicate task completions
- [ ] **18-07** Proof 7.7: routing learns (5 failures → router avoids model)
- [ ] **18-08** Proof 7.8: knowledge reuse (run 1 episode → run 2 prompt includes it)
- [ ] **18-09** Proof 7.9: provider matrix (each provider: success or classified DOA)
- [ ] **18-10** Proof 7.10: HTTP query (serve + plan run + events endpoint matches JSONL)
- [ ] **18-11** Proof 7.11: ACP workflow (implement → gate → review → commit)
- [ ] **18-12** Proof 7.12: single-prompt express pipeline end-to-end
- [ ] **18-CC1** No bare `Command::new("claude")` outside adapter
- [ ] **18-CC2** One canonical `FeedbackEvent` enum
- [ ] **18-CC3** Total LOC reduced by ≥100K from baseline
- [ ] **18-CC4** All required files present (persistence.rs, failure_tracker.rs, etc.)
- [ ] **18-CC5** All retired files absent (orchestrate.rs, daimon, etc.)

---

## Summary Stats

| Metric | Count |
|--------|-------|
| Total issues | ~200 |
| Plans | 18 |
| Critical path items (Plans 01-07) | ~90 |
| Deletion items (Plans 12, 15) | ~35 |
| Proof items (Plan 18) | ~17 |
