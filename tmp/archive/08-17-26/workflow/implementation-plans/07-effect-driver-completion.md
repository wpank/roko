# 07 — EffectDriver: Multi-Task Actions, Gate Feedback, Safety, Persistence

> Phase 1.3 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. The integration plan that connects 01–06.

---

## Status (2026-05-01)

**PARTIAL.** Single-task driver works. Multi-task action variants missing. Gate feedback always passed empty. Safety not threaded.

**What's done:**

- `roko_runtime::effect_driver::EffectDriver` — `crates/roko-runtime/src/effect_driver.rs`
- `EffectServices` aggregating `model_caller`, `prompt_assembler`, `feedback_sink`, `gate_runner`, `affect_policy`
- Implements: `spawn_agent(role, user_prompt, context)`, `run_gates(enabled_gates, shell_gates)`, `commit(message)`, `save_checkpoint(state, path)`
- Emits `RuntimeEvent`s on every action
- Cancellation handled in `WorkflowEngine`

**What's not:**

- **Gate feedback always empty:** `EffectDriver::spawn_agent` passes `gate_feedback: Vec::new()` to `PromptSpec`. Audit doc 11 § 7 calls this out.
- No multi-task action variants: `SpawnImplementerForTask`, `RunGateForTask`, `RunVerifyStepsForTask`, `SubmitMerge`, `SpawnScribeForTask`
- Save_checkpoint exists but `WorkflowEngine` does not call it in production loop (per plan 04)
- `SafetyLayer` not wired (per plan 09)
- `RunVerifySteps` action handling — missing
- Multi-task fanout: when `step_actions(...)` returns `[SpawnA, SpawnB, SpawnC]`, today the driver executes them serially; need parallel `join_all`
- Cancel handling for in-flight agents: not implemented (current code waits for in-flight to complete)
- `merge_service` and `worktree_service` don't exist — `MergeStrategy` action has no handler

---

## Goal

`EffectDriver` is the complete side-effect executor for the unified `WorkflowEngine`:

1. Handles all multi-task `PipelineAction` variants (per plan 05)
2. Plumbs gate failure context into the **next** prompt (closing the feedback loop)
3. Calls `PersistenceService::checkpoint` after each phase transition
4. Calls `SafetyLayer` pre/post for every agent spawn
5. Executes multiple actions concurrently where the FSM emits a wave
6. Cooperatively cancels in-flight agents on `CancelToken` flip
7. Owns `MergeService` + `WorktreeService` for plan-execution merges

---

## Why This Exists (Anti-Patterns Eliminated)

- **#4 Features in Wrong Layer** — adding feedback / safety / merge to the runner instead of the driver
- **#7 Copy-Paste** — runner has its own merge code, ACP has its own commit code
- **#10 God file** — `runner/event_loop.rs` (3K LOC) does what driver should do

---

## Existing Code — Read These First

```rust
// crates/roko-runtime/src/effect_driver.rs (current)
pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<dyn AffectPolicy>>,
}

pub struct EffectDriver {
    services: EffectServices,
    run_id: String,
    workdir: PathBuf,
    feedback_totals: WorkflowFeedbackTotals,
}

impl EffectDriver {
    pub async fn spawn_agent(...) -> Result<PipelineInput>;
    pub async fn run_gates(...) -> Result<PipelineInput>;
    pub async fn commit(...) -> Result<PipelineInput>;
    pub async fn save_checkpoint(...) -> Result<()>;
    pub fn emit(&self, event: RuntimeEvent);
}
```

The `spawn_agent` body around line 122–140 builds `PromptSpec` with **`gate_feedback: Vec::new()`** — this is the bug.

---

## Implementation Steps

### Step 1 — Plumb gate feedback into next agent prompt

The pipeline already records the last gate failure in its state (after plan 05, this is `pipeline.last_gate_failure: Option<GateFailureRecord>`). `WorkflowEngine` passes it to the driver as part of the action context:

```rust
// New in pipeline_state.rs
pub enum PipelineAction {
    SpawnImplementer { task_id: Option<String>, gate_feedback: Vec<GateFeedback>, review_findings: Vec<String> },
    SpawnAutoFixer { task_id: Option<String>, error_context: GateFailureRecord },
    // ...
}
```

```rust
// crates/roko-runtime/src/effect_driver.rs (updated spawn_agent)
async fn spawn_implementer(
    &self,
    task_id: Option<String>,
    gate_feedback: Vec<GateFeedback>,    // was: Vec::new()
    review_findings: Vec<String>,
) -> Result<PipelineInput> {
    let assembled = self.services.prompt_assembler.assemble(PromptSpec {
        role: Some("implementer".to_string()),
        task: Some(self.task_for(task_id.as_deref())?.title.clone()),
        workdir: Some(self.workdir.clone()),
        gate_feedback,                   // NOW POPULATED
        review_findings,
        attempt: self.attempt_for(task_id.as_deref()),
        ..Default::default()
    }).await?;

    self.emit(RuntimeEvent::AgentSpawned { ... });
    let resp = self.services.model_caller.call(ModelCallRequest {
        system: Some(assembled.system),
        prompt_section_ids: assembled.diagnostics.included_sections.iter().map(|s| s.id.clone()).collect(),
        knowledge_ids: assembled.diagnostics.knowledge_ids.clone(),
        ...
    }).await?;
    self.emit(RuntimeEvent::AgentCompleted { ... });
    Ok(PipelineInput::AgentCompleted)
}
```

The audit specifically called out the `Vec::new()` hardcode (`crates/roko-runtime/src/effect_driver.rs:~125-129`). After this step, gate failure errors / warnings / suggestions flow into the retry prompt as Layer 4b (per plan 02 § PromptAssembly).

### Step 2 — Add multi-task action handlers

For each new variant from plan 05's `PipelineAction`:

```rust
// crates/roko-runtime/src/effect_driver.rs
impl EffectDriver {
    pub async fn execute(&self, action: PipelineAction) -> EffectOutcome {
        match action {
            PipelineAction::SpawnEnricher { context } => self.spawn_enricher(context).await,
            PipelineAction::SpawnImplementer { task_id, gate_feedback, review_findings } =>
                self.spawn_implementer(task_id, gate_feedback, review_findings).await,
            PipelineAction::SpawnImplementerForTask { task_id, prompt } =>
                self.spawn_implementer_for_task(task_id, prompt).await,
            PipelineAction::SpawnAutoFixerForTask { task_id, error_context } =>
                self.spawn_autofixer_for_task(task_id, error_context).await,
            PipelineAction::RunGateForTask { task_id, rung } =>
                self.run_gate_for_task(task_id, rung).await,
            PipelineAction::RunVerifyStepsForTask { task_id, steps } =>
                self.run_verify_steps(task_id, steps).await,
            PipelineAction::SpawnReviewerForTask { task_id, context } =>
                self.spawn_reviewer_for_task(task_id, context).await,
            PipelineAction::SpawnScribeForTask { task_id, context } =>
                self.spawn_scribe_for_task(task_id, context).await,
            PipelineAction::CommitForTask { task_id, message } =>
                self.commit_for_task(task_id, message).await,
            PipelineAction::SubmitMerge { plan_id } =>
                self.submit_merge(plan_id).await,
            PipelineAction::EmitWarning(w) => {
                self.emit(RuntimeEvent::WarningRaised { run_id: self.run_id.clone(), warning: w });
                EffectOutcome::Done
            }
            PipelineAction::NoOp => EffectOutcome::Done,
            // existing variants
            PipelineAction::SpawnStrategist => self.spawn_strategist().await,
            // ...
        }
    }
}
```

`spawn_implementer_for_task`, `spawn_reviewer_for_task`, `spawn_scribe_for_task` all reuse the same `spawn_agent` helper internally — only the role and prompt context differ. Resist creating six near-identical functions.

```rust
async fn spawn_for_role(
    &self,
    role: &str,
    task_id: Option<String>,
    extra_spec: PromptSpec,           // role-specific extras
) -> EffectOutcome {
    let mut spec = self.base_prompt_spec(task_id.clone());
    spec.role = Some(role.to_string());
    spec.merge(extra_spec);
    let assembled = self.services.prompt_assembler.assemble(spec).await?;
    let req = self.build_model_request(role, task_id, &assembled);
    let response = self.services.model_caller.call(req).await?;
    EffectOutcome::AgentDone {
        agent_id: task_id.unwrap_or(role.into()),
        output: response.content,
        tokens_used: response.usage.total_tokens,
        cost_usd: response.usage.cost_usd,
        files_changed: detect_files_changed(&response.content, &self.workdir),
    }
}
```

### Step 3 — Concurrent action fanout

When the FSM emits multiple actions in one `step_actions` call (e.g. wave dispatch):

```rust
// crates/roko-runtime/src/workflow_engine.rs
let actions = pipeline.step_actions(input);
let outcomes = futures::future::join_all(
    actions.into_iter().map(|action| {
        let driver = driver.clone();              // Arc<EffectDriver>
        let token = token.clone();
        async move {
            tokio::select! {
                outcome = driver.execute(action) => outcome,
                _ = token.cancelled() => EffectOutcome::Failed { error: "cancelled".into() },
            }
        }
    })
).await;

for outcome in outcomes {
    let next_input = outcome.into_input();
    pipeline.step(next_input);                    // collapse multiple outcomes into FSM history
}
```

Bound concurrency by `max_concurrent_tasks` (from `PlanExecutionConfig`). Use `futures::stream::iter(actions).buffer_unordered(N).collect()` instead of `join_all` for safety.

### Step 4 — Wire `PersistenceService::checkpoint`

```rust
// crates/roko-runtime/src/workflow_engine.rs (per plan 04)
loop {
    let actions = pipeline.step_actions(input);
    let outcomes = drive_concurrent(&driver, actions, &token).await;
    for outcome in outcomes {
        input = outcome.into_input();
        pipeline.step(input);
    }
    services.persistence.checkpoint(&snapshot_from(&pipeline, &ledger)).await?;
    if pipeline.is_terminal() { break; }
}
```

Frequency: every phase transition by default; configurable via `[runtime].checkpoint_interval_ms`.

### Step 5 — Add `MergeService` + `WorktreeService`

```rust
// crates/roko-runtime/src/merge_service.rs
#[async_trait]
pub trait MergeService: Send + Sync {
    async fn merge(&self, request: MergeRequest) -> Result<MergeOutcome>;
}

pub struct GitMergeService {
    workdir: PathBuf,
    strategy: MergeStrategy,                  // PullRequest | DirectCommit | Worktree
    github_client: Option<Arc<dyn GitHubClient>>,
}

#[async_trait]
impl MergeService for GitMergeService {
    async fn merge(&self, request: MergeRequest) -> Result<MergeOutcome> {
        match self.strategy {
            MergeStrategy::DirectCommit => self.direct_commit(request).await,
            MergeStrategy::Worktree => self.merge_worktree(request).await,
            MergeStrategy::PullRequest => self.open_pr(request).await,
        }
    }
}

// crates/roko-runtime/src/worktree_service.rs
pub trait WorktreeService: Send + Sync {
    async fn create_for_plan(&self, plan_id: &str) -> Result<PathBuf>;
    async fn cleanup_idle(&self, ttl_secs: u64) -> Result<u32>;
}
```

Implementations:

- `GitMergeService` — extracted from `crates/roko-orchestrator/src/merge_queue.rs` (which audit doc 15 § 7 calls out as fully built but never called)
- `WorktreeService` — extracted from `crates/roko-orchestrator/src/worktree.rs` (~42K LOC; trim to essentials)

Add `MergeService` + `WorktreeService` to `EffectServices`:

```rust
pub struct EffectServices {
    // existing
    pub merge_service: Arc<dyn MergeService>,         // NEW
    pub worktree_service: Option<Arc<dyn WorktreeService>>,  // NEW; None for non-plan workflows
    pub persistence: Arc<dyn PersistenceService>,
}
```

### Step 6 — Cooperative cancellation of in-flight agents

Today `CancelToken` is checked at iteration boundaries. In-flight agent calls run to completion. To cancel mid-call:

```rust
// crates/roko-runtime/src/effect_driver.rs
async fn spawn_for_role(&self, role: &str, ...) -> EffectOutcome {
    // ...
    let model_call = self.services.model_caller.stream(req);
    let mut stream = model_call.await?;

    let mut content = String::new();
    let mut usage = TokenUsage::default();
    while let Some(event) = stream.next().await {
        if self.cancel_token.is_cancelled() {
            return EffectOutcome::Failed { error: "cancelled mid-stream".into() };
        }
        match event {
            ModelStreamEvent::ContentDelta { text } => content.push_str(&text),
            ModelStreamEvent::Usage { usage: u } => usage = u,
            ModelStreamEvent::Completed { .. } => break,
            ModelStreamEvent::Failed { error } => return EffectOutcome::Failed { error },
            _ => {}
        }
    }
    EffectOutcome::AgentDone { ... }
}
```

This requires plan 01 step 1 (true streaming) to land first. Without it, `stream()` is post-hoc chunked and cancellation is no better than today.

### Step 7 — Wire `SafetyLayer` (per plan 09)

Add to `EffectServices`:

```rust
pub safety: Arc<SafetyLayer>,
```

Call `safety.pre_dispatch_check(...)` before every agent spawn; `safety.post_dispatch_check(...)` after. Block / warn per the result.

This is shared with plan 09; do not duplicate the logic — just consume the trait.

### Step 8 — Tests

```rust
#[tokio::test]
async fn gate_feedback_flows_into_retry_prompt() {
    let driver = test_driver_with_recording_assembler();
    driver.execute(PipelineAction::SpawnImplementer {
        task_id: None,
        gate_feedback: vec![GateFeedback {
            gate_name: "compile".into(), rung: 0, passed: false,
            errors: vec!["error[E0382]: borrow of moved value".into()],
            warnings: vec![], suggestions: vec![],
        }],
        review_findings: vec![],
    }).await;
    let captured = driver.last_assembled_prompt();
    assert!(captured.system.contains("error[E0382]"));
    assert!(captured.system.contains("Gate Feedback"));
}

#[tokio::test]
async fn concurrent_actions_run_in_parallel() {
    let driver = test_driver_with_slow_agent(Duration::from_millis(500));
    let start = Instant::now();
    let outcomes = drive_concurrent(&driver, vec![
        PipelineAction::SpawnImplementerForTask { task_id: "A".into(), prompt: spec() },
        PipelineAction::SpawnImplementerForTask { task_id: "B".into(), prompt: spec() },
        PipelineAction::SpawnImplementerForTask { task_id: "C".into(), prompt: spec() },
    ], &token, max_concurrent: 3).await;
    assert!(start.elapsed() < Duration::from_millis(800));   // parallel, not 1500ms
    assert_eq!(outcomes.len(), 3);
}

#[tokio::test]
async fn cancellation_aborts_in_flight() {
    let driver = test_driver_with_slow_agent(Duration::from_secs(10));
    let token = CancelToken::new();
    let task = tokio::spawn(driver.execute(PipelineAction::SpawnImplementer { ... }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    token.cancel();
    let outcome = task.await.unwrap();
    assert!(matches!(outcome, EffectOutcome::Failed { .. }));
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #4 Features in wrong layer | Adding retry logic in the driver (belongs in scheduler) or classification (belongs in FSM) | Driver only **executes** — FSM decides; scheduler queues |
| #7 Copy-paste | Six near-identical `spawn_X` methods | One `spawn_for_role(role, ...)` helper |
| #10 God file | Driver growing past 1.5K LOC | Extract `merge_service.rs`, `worktree_service.rs`, `commit_service.rs` |

---

## Things NOT To Do

1. **Don't await `feedback_sink.record(...)` in the agent hot path.** Spawn it: `tokio::spawn(feedback.record(event))`. Sinks are designed to be fire-and-forget.
2. **Don't put per-role business logic in the driver.** Role-specific behavior (e.g. "scribe writes docs" / "auditor checks security") lives in templates (plan 02), not driver code.
3. **Don't call `git` directly.** Use `MergeService` and `WorktreeService` traits. `git2` or shell out happens inside one place.
4. **Don't hard-code `max_concurrent_tasks = 1`.** Default 1, but read from config and respect.
5. **Don't ignore `EffectOutcome::Failed`.** Every failure must produce a `PipelineInput` so the FSM can decide next steps.
6. **Don't call `PersistenceService::checkpoint` between every individual model call.** Once per phase transition is the right cadence; otherwise, IO dominates.
7. **Don't forget about `caller` strings.** Each spawn function in the driver passes `caller: Some("workflow_engine")` to ModelCallRequest. Helper functions / sub-agents may need different caller strings (e.g. `caller: Some("workflow_engine.scribe")`).
8. **Don't share mutable state across concurrent action handlers without `Mutex`.** When fanning out, `EffectDriver` is `Arc<EffectDriver>`; any mutable field needs interior mutability.

---

## Tests / Proof Criteria

```bash
# 1. Gate feedback no longer hardcoded empty
rg 'gate_feedback: Vec::new\(\)' crates/roko-runtime/src/effect_driver.rs
# expected: 0 matches

# 2. EffectServices includes new services
rg 'merge_service|persistence|safety:' crates/roko-runtime/src/effect_driver.rs
# expected: present in EffectServices

# 3. Driver's execute matches all PipelineAction variants exhaustively
# (rust enum exhaustiveness is enforced by compiler; verify no _ pattern)
rg '_ =>|_ => ' crates/roko-runtime/src/effect_driver.rs
# expected: 0 wildcard arms in execute()

# 4. Concurrent fanout uses stream + buffer_unordered
rg 'buffer_unordered|join_all' crates/roko-runtime/src/workflow_engine.rs
```

Functional proofs:

- [ ] All 3 unit tests above pass
- [ ] `roko run "fix the broken test"` after a failed gate retries with the gate output in its prompt (verify visually via prompt diagnostic)
- [ ] `roko plan run` 5-task plan with `max_concurrent_tasks = 3` shows 3 concurrent agents in `roko dashboard`
- [ ] `kill -2` (Ctrl+C) during multi-task run cancels all in-flight agents within 1 second
- [ ] Merge step uses `MergeService::PullRequest` strategy when configured
- [ ] CR/persistence checkpoint test (plan 04 Step 6) passes for every action type

---

## Dependencies

- **Plan 01 (ModelCallService)** — for true streaming + cancellation mid-call
- **Plan 02 (PromptAssembly)** — for `gate_feedback` to actually appear in Layer 4b
- **Plan 03 (FeedbackService)** — for the sink to consume `record()` calls
- **Plan 04 (PersistenceService)** — for `checkpoint()` API
- **Plan 05 (PipelineState multi-task)** — for the new `PipelineAction` variants
- **Plan 06 (TaskScheduler)** — for the scheduler the driver coordinates with
- **Plan 09 (Safety wiring)** — for `SafetyLayer` API to thread through

This is the **integration plan** that makes 01-06 + 09 actually run. Tackle last.

---

## Estimated Effort

**L.** ~1.5-2 weeks.

- Step 1 (gate feedback) — S (1 day)
- Step 2 (multi-task handlers) — M (3 days)
- Step 3 (concurrent fanout) — M (2 days; backpressure tricky)
- Step 4 (persistence wiring) — S (half day, mostly already in plan 04)
- Step 5 (MergeService + WorktreeService) — L (4-5 days; extracting from orchestrator)
- Step 6 (cancellation) — M (2 days; depends on streaming)
- Step 7 (safety wiring) — S (1 day; integrates plan 09)
- Step 8 (tests) — M (2 days)
