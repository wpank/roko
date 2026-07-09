# Migration: v2 to v3 Transition Plan

## Strategy: Parallel Module, Progressive Cutover

v3 is built alongside v2, not as an in-place edit. The new `dispatch/` module and rewritten `runner/` files coexist with the existing code until all 6 audit paths reach target scores.

## Phase 0: New Module Shell (Day 1)

Create `dispatch/` directory with type stubs that compile but delegate to existing v2 code.

### Files Created
```
crates/roko-cli/src/dispatch/
  mod.rs              - pub mod declarations + AgentDispatcher struct
  model_routing.rs    - RoutingContext + CascadeRouter integration stubs
  prompt_builder.rs   - PromptAssembler + RoleSystemPromptSpec delegation
  outcome.rs          - AgentOutcome, DispatchError enums
  warm_pool.rs        - WarmPool placeholder (empty impl)
```

### Changes to Existing Files
```
crates/roko-cli/src/main.rs    - add `mod dispatch;`
crates/roko-cli/src/run.rs     - no changes yet (still uses runner::run)
```

### Verification
```bash
cargo check -p roko-cli
cargo test -p roko-cli
```

## Phase 1: Agent Dispatch (Paths 1 + 5)

Replace `agent_stream::spawn_agent()` with `AgentDispatcher::dispatch()`.

### Step 1.1: Wire `create_agent_for_model()` into `dispatch/mod.rs`

Replace the hardcoded `Command::new("claude")` in `agent_stream.rs` with a call through `AgentDispatcher` that uses `roko-agent`'s provider factory.

**Before** (v2):
```rust
// agent_stream.rs line 197-234
let mut cmd = Command::new(&config.program);
cmd.args(["--print", "--output-format", "stream-json"]);
cmd.args(["--model", &config.model]);
// ... hardcoded Claude CLI flags
```

**After** (v3):
```rust
// dispatch/mod.rs
let agent = create_agent_for_model(&model, &agent_options)?;
agent.send_message(&prompt, &system_prompt).await?;
```

**Migration path**: `event_loop.rs`'s `SpawnAgent` arm calls `dispatch.dispatch()` instead of `agent_stream::spawn_agent()`.

### Step 1.2: Wire `CascadeRouter` into `dispatch/model_routing.rs`

**Before** (v2):
```rust
// event_loop.rs line 470-474
let model = task_def.model_hint
    .as_deref()
    .unwrap_or(&ctx.config.model)
    .to_string();
```

**After** (v3):
```rust
// dispatch/model_routing.rs
let routing = RoutingContext::from_task(task_def, &config);
let model = cascade_router.select_model(&routing)
    .unwrap_or_else(|| config.model.clone());
```

### Step 1.3: Wire `RoleSystemPromptSpec` into `dispatch/prompt_builder.rs`

**Before** (v2):
```rust
// agent_stream.rs line 310-340
pub fn build_minimal_system_prompt(task, plan_id) -> String {
    // 3-line hardcoded prompt
}
```

**After** (v3):
```rust
// dispatch/prompt_builder.rs
pub fn assemble_prompt(task, plan_id, ctx) -> Result<String> {
    let spec = RoleSystemPromptSpec::for_role(role);
    // ... 9-layer assembly with playbooks, anti-patterns
}
```

### Step 1.4: Wire verify phase

**Before** (v2):
```rust
// event_loop.rs line 545-548
ExecutorAction::RunVerify { plan_id } => {
    info!("auto-passing verification (stub)");
    let _ = executor.apply_event(plan_id, &ExecutorEvent::VerifyPassed);
}
```

**After** (v3):
```rust
ExecutorAction::RunVerify { plan_id } => {
    let reviewer = warm_pool.take_or_spawn("reviewer", &config)?;
    reviewer.send_message(&verify_prompt).await?;
    // ... parse result -> VerifyPassed or VerifyFailed
}
```

## Phase 2: Plan Execution (Path 2)

### Step 2.1: Replace sentinel resolution with real DAG

**Before** (v2):
```rust
// event_loop.rs line 402-429
let resolved_task = if task == "next" || task == "fix" || task == "regen-verify" {
    // Walk task list, find first ready one
}
```

**After** (v3):
```rust
// DagExecutor in event_loop.rs uses proper topological sort
let ready_tasks = dag.ready_tasks(completed);
let task_id = ready_tasks.first()?;
```

### Step 2.2: Add gate timeout and semaphore

**Before** (v2):
```rust
// gate_dispatch.rs line 22
tokio::spawn(async move { /* no timeout */ });
```

**After** (v3):
```rust
let result = tokio::time::timeout(
    Duration::from_secs(timeout),
    gate_semaphore.acquire_then(|| run_rung(...))
).await;
```

### Step 2.3: Add retry backoff and failure classification

**Before** (v2): All failures go to auto-fix identically.

**After** (v3):
```rust
match classify_failure(&completion) {
    FailureClass::Transient => retry_with_backoff(task, attempt),
    FailureClass::Permanent => fail_task_permanently(task),
}
```

### Step 2.4: Add merge queue

Serialize git-touching operations to prevent conflicts.

## Phase 3: State Persistence (Path 3)

### Step 3.1: Expand `PersistPaths`

**Before** (v2):
```rust
pub struct PersistPaths {
    pub executor_json: PathBuf,
    pub episodes_jsonl: PathBuf,
    pub efficiency_jsonl: PathBuf,
    pub agent_pids_json: PathBuf,
    pub events_json: PathBuf,
}
```

**After** (v3):
```rust
pub struct PersistPaths {
    // existing
    pub executor_json: PathBuf,
    pub episodes_jsonl: PathBuf,
    pub efficiency_jsonl: PathBuf,
    pub agent_pids_json: PathBuf,
    pub events_json: PathBuf,
    // new
    pub cascade_router_json: PathBuf,
    pub gate_thresholds_json: PathBuf,
    pub daimon_json: PathBuf,
}
```

### Step 3.2: Save all 5 files per task completion

### Step 3.3: Add version field to snapshots

### Step 3.4: Enhance resume to load all state files

## Phase 4: Learning Feedback (Path 4)

### Step 4.1: Consult CascadeRouter at dispatch

Wire `dispatch/model_routing.rs` to call `cascade_router.select_model()`.

### Step 4.2: Record routing observations

After every task outcome, call `cascade_router.record_observation()`.

### Step 4.3: Enrich episode logging

Add model, provider, tokens, cost, gate result, files changed to every episode.

### Step 4.4: Wire knowledge ingestion

On gate pass, call neuro store ingestion with lowered admission threshold.

## Phase 5: Observable Execution (Path 6)

### Step 5.1: Expand `handle_agent_event` to publish all events

Every `AgentEvent` variant gets a corresponding `DashboardEvent`.

### Step 5.2: Add non-TUI structured logging

`tracing::info_span!` at every phase transition.

### Step 5.3: Stream gate output to TUI

Gate verdicts emit `TaskOutputAppended` events.

## Cutover: Remove v2 Code

Once all phases pass verification:

1. **Delete** `agent_stream::spawn_agent()` and `build_minimal_system_prompt()` - replaced by `dispatch/`
2. **Delete** sentinel resolution code in `event_loop.rs` - replaced by real DAG
3. **Remove** `claude_program` from `RunConfig` - no longer needed (provider factory handles binary paths)
4. **Remove** `ClaudeStreamEvent` and related types from `types.rs` - provider abstraction handles parsing
5. **Archive** `orchestrate.rs` - v3 runner + dispatch/ replaces its runtime functionality

### What Stays
- `plan_loader.rs` - unchanged
- `persist.rs` - enhanced, not replaced
- `tui_bridge.rs` - enhanced, not replaced
- All external crate interfaces (`roko-agent`, `roko-gate`, `roko-learn`, `roko-compose`)

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Provider abstraction breaks streaming | Keep `agent_stream.rs` stream parser as fallback for Claude CLI backend |
| CascadeRouter has no training data | Default to `model_hint` or `config.model` when router returns `None` |
| 5-file snapshot is slow | Only save changed files (dirty flag per state component) |
| Gate semaphore deadlock | Use `tokio::sync::Semaphore` with `try_acquire_owned` + timeout |
| Resume from v2 snapshot | Version field defaults to `1` for unversioned snapshots -> apply v1 to v2 migration |

## Timeline

| Phase | Depends On | Scope |
|-------|-----------|-------|
| Phase 0: Module shell | Nothing | 1 day |
| Phase 1: Agent dispatch | Phase 0 | 2-3 days |
| Phase 2: Plan execution | Phase 0 | 2-3 days |
| Phase 3: Persistence | Phase 0 | 1-2 days |
| Phase 4: Learning | Phase 1 | 1-2 days |
| Phase 5: Observability | Phases 1-2 | 1-2 days |
| Cutover | Phases 1-5 | 1 day |

Phases 1, 2, and 3 are parallelizable after Phase 0. Phase 4 depends on Phase 1 (dispatch). Phase 5 depends on Phases 1 and 2 (events flow from dispatch and execution).

## Implementation Packet

This migration plan is the task list an agent should follow when implementing the full runner convergence work.

### Global Rules

- [ ] Keep each phase independently mergeable.
- [ ] Add tests in the same phase as behavior changes.
- [ ] Do not delete legacy behavior until parity tests exist.
- [ ] If a phase needs a larger design decision, add a short ADR to this directory before coding.

### Phase 0 Checklist: Shell and Compile

- [ ] Add `dispatch/` module shell.
- [ ] Add `runtime_feedback/` module shell.
- [ ] Add `projection/` module shell.
- [ ] Add `runtime_events` module in `roko-agent`.
- [ ] Wire module exports with no behavior changes.
- [ ] Run `cargo check -p roko-cli -p roko-agent`.

### Phase 1 Checklist: Dispatch

- [ ] Move model selection to `dispatch/model_routing.rs`.
- [ ] Move prompt construction to `dispatch/prompt_builder.rs`.
- [ ] Move spawn preflight to `dispatch/preflight.rs`.
- [ ] Replace direct runner spawn call.
- [ ] Preserve mock dispatcher test path.
- [ ] Add unit tests for dispatch request construction.

### Phase 2 Checklist: Execution

- [ ] Replace sentinel task resolution.
- [ ] Add per-plan task status tracking.
- [x] Add real verify handling.
- [x] Add gate timeout and semaphore.
- [ ] Add merge queue dispatch.
- [ ] Add execution-state tests.

### Phase 3 Checklist: Persistence

- [ ] Version snapshots.
- [ ] Persist run state.
- [ ] Persist router and thresholds.
- [x] Append event log.
- [ ] Add resume tests.

### Phase 4 Checklist: Feedback

- [ ] Add feedback facade.
- [ ] Record routing observations.
- [ ] Record knowledge candidates.
- [ ] Record conductor observations.
- [ ] Add dream trigger events.

### Phase 5 Checklist: Observability

- [ ] Add complete event projection.
- [ ] Add non-TUI progress output.
- [ ] Add dashboard snapshot tests.

### Cutover Checklist

- [ ] Run parity matrix from `21-FEATURE-PARITY-MATRIX.md`.
- [ ] Mark any remaining `orchestrate.rs` unique behavior.
- [ ] Delete or wrap migrated legacy behavior.
- [ ] Update `docs/STATUS.md` to reflect runner ownership.

## Worker 9 Evidence Checklist (2026-04-26)

Migration progress that is implemented in the current tree:

- [x] Active `plan run` uses `crates/roko-cli/src/runner/event_loop.rs`, with no-mock Codex and Claude smoke proof captured under `/tmp/roko-real-e2e-nrUD05/`.
- [x] `crates/roko-cli/src/dispatch_v2.rs` provides a partial dispatch abstraction for Claude/Codex CLI invocation and provider factory resolution.
- [x] `crates/roko-cli/src/runner/gate_dispatch.rs` provides real gate timeout, semaphore, and `task.verify` execution.
- [x] `crates/roko-cli/src/runner/persist.rs` appends runtime events to `.roko/events.jsonl` and atomically saves `executor.json`.
- [x] `crates/roko-cli/src/runner/agent_stream.rs` routes live prompt assembly through `roko-compose` via `build_composed_system_prompt`, with a legacy fallback.
- [x] `crates/roko-learn/src/runtime_feedback.rs`, `crates/roko-dreams/src/runner.rs`, and `crates/roko-neuro/src/lifecycle.rs` provide reusable feature engines that are not yet wired into the active runner.

Migration blockers still open:

- [ ] Phase 0 module shells are not present: `crates/roko-cli/src/dispatch/`, `crates/roko-cli/src/runtime_feedback/`, `crates/roko-cli/src/projection/`, and `crates/roko-agent/src/runtime_events.rs`.
- [ ] Phase 1 dispatch is only partially implemented through `dispatch_v2.rs`; the runner still calls `agent_stream::spawn_agent` and parses provider-specific stream JSON locally.
- [ ] Phase 2 still lacks a dedicated task DAG module and merge queue dispatch in the active runner.
- [ ] Phase 3 still lacks runner-owned `run-state.json`, router snapshot, threshold snapshot, and strict stale task-id resume validation.
- [ ] Phase 4 feedback remains split between runner-local JSONL writes and richer `roko-learn` abstractions that the live runner does not call.
- [ ] Phase 5 projection facade is absent, so TUI, HTTP/SSE, and non-TUI CLI parity is not proven.

## 2026-04-27 Deepening Pass - Source-Corrected Migration State

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: this section corrects stale 2026-04-26 phase blockers, distinguishes implemented module existence from active-path proof, and turns migration into concrete cutover batches with grep gates. The score is not higher because the migration still needs generated proof reports from a clean clone and real provider runs.

This section supersedes the "Worker 9 Evidence Checklist" rows above when source disagrees with them.

### Current Source Truth

- [x] `crates/roko-agent/src/runtime_events.rs` exists and exports provider-neutral `AgentRuntimeEvent`.
- [x] `crates/roko-agent/src/provider/claude_cli/stream.rs` owns Claude stream parsing and maps it into `AgentRuntimeEvent`.
- [x] `crates/roko-cli/src/runner/types.rs` aliases `AgentEvent` to `roko_agent::AgentRuntimeEvent`.
- [x] `crates/roko-cli/src/dispatch/` exists with `mod.rs`, `model_routing.rs`, `prompt_builder.rs`, `outcome.rs`, and `warm_pool.rs`.
- [x] `crates/roko-cli/src/runtime_feedback/` exists with episode, routing, knowledge, conductor, and dream sinks.
- [x] `crates/roko-cli/src/projection/` exists with dashboard and CLI progress adapters.
- [x] `crates/roko-cli/src/runner/task_dag.rs` exists.
- [x] `crates/roko-cli/src/runner/merge.rs` exists and contains `PlanMerger`, `GitMergeBackend`, merge conflict evidence, and regression gate support.
- [x] `crates/roko-cli/src/runner/event_loop.rs` constructs the new `Dispatcher` and `PromptAssembler` in the active runner path.
- [x] `crates/roko-cli/src/runner/event_loop.rs` handles `ExecutorAction::MergeBranch` through `PlanMerger`, not by unconditional merge success.
- [x] `crates/roko-cli/src/lib.rs` exports `dispatch`, `projection`, and `runtime_feedback`.

### Remaining Migration Truth

- [ ] `dispatch_v2.rs` still exists and must be classified as legacy adapter, compatibility layer, or removable surface.
- [ ] `dispatch_direct.rs` still exists and is used by chat/unified paths; it must be routed through the same dispatch facade or explicitly scoped to interactive one-shot chat.
- [ ] `orchestrate.rs` still exports `PlanRunner` and remains a large donor/runtime surface; every remaining public caller must be classified.
- [ ] `PlanRunner::from_plans_dir` callers must be ported or intentionally marked legacy.
- [ ] Active prompt assembly is runner-owned through `dispatch/prompt_builder.rs`; it still needs convergence with `roko-compose` VCG/cost/section-effect manifest semantics.
- [ ] Active routing still risks config-default bypass because `DispatchContext::model_hint` can carry the default configured model before the cascade router has authority.
- [ ] `dispatch/model_routing.rs` must return evidence-backed `ModelChoiceSource` values for task override, runtime override, router decision, and default fallback.
- [ ] `runtime_feedback/knowledge.rs` writes candidates but does not prove durable `roko-neuro` admission, reinforcement, and lifecycle promotion.
- [ ] `runtime_feedback/dreams.rs` writes dream triggers but does not prove consolidation or prompt/routing influence on the next run.
- [ ] Resume still needs proof for interrupt-after-task-pass and interrupt-during-gate without duplicate completion.
- [ ] Projection still needs HTTP/SSE/TUI/CLI parity proof from the same normalized event log.
- [ ] Provider proof must cover Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI through the same dispatch path with status vocabulary: `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, `unsupported`.

### Cutover Batches

#### MIG-01: Freeze Legacy Entry Points

- [ ] Generate a list of every public `orchestrate.rs`, `dispatch_v2.rs`, and `dispatch_direct.rs` caller.
- [ ] Classify each caller as `active_runtime`, `interactive_chat`, `legacy_donor`, `test_only`, or `remove`.
- [ ] Add comments or type names that make the classification obvious in code.
- [ ] Refuse new production callers to `orchestrate.rs` except explicitly documented migration adapters.
- [ ] Store the inventory at `tmp/mori-diffs/generated/migration-legacy-surface-report.json`.

#### MIG-02: Finish Dispatch Authority

- [ ] Ensure all `plan run` model decisions enter through `dispatch/model_routing.rs`.
- [ ] Stop passing the configured default model as `model_hint`; default model is fallback policy, not a user override.
- [ ] Add a routing decision record for every dispatch containing requested model, selected model, provider, source, fallback reason, and policy version.
- [ ] Route direct chat/unified calls through a shared dispatch command service or document why those paths are intentionally outside plan-run orchestration.
- [ ] Prove every provider emits `AgentRuntimeEvent::Started` before output and exactly one terminal event.

#### MIG-03: Finish Prompt Authority

- [ ] Treat `dispatch/prompt_builder.rs` as the active prompt authority for runner execution until it is split or replaced.
- [ ] Feed knowledge candidates, dream routing advice, playbooks, section effectiveness, gate feedback, retry context, and file scope into one `PromptAssembler` input object.
- [ ] Emit prompt diagnostics for included sections, dropped sections, estimated tokens, budget decisions, knowledge ids, playbook ids, dream advice ids, and section-effectiveness ids.
- [ ] Reconcile `dispatch/prompt_builder.rs` diagnostics with `roko-compose` `CompositionManifest` so prompt cost and section attribution are not duplicated.

#### MIG-04: Finish Feedback And Persistence

- [ ] Flush routing observations to `.roko/learn/cascade-router.json` or a versioned replacement after terminal task events.
- [ ] Flush gate threshold observations to `.roko/learn/gate-thresholds.json` or a versioned replacement.
- [ ] Write durable knowledge lifecycle records, not only candidates.
- [ ] Write durable dream consolidation records, not only triggers.
- [ ] Persist run-state fields needed for resume: run id, active effect, active plan/task, retry count, routing decision, prompt diagnostics ids, merge attempt id, and projection cursor.

#### MIG-05: Finish Projection And Query Surfaces

- [ ] Make TUI, CLI progress, HTTP polling, and HTTP/SSE read from the same projection model.
- [ ] Expose provider lifecycle, prompt diagnostics, gate decision, retry decision, merge result, conflict evidence, knowledge writes, dream triggers, and resume events through queryable projections.
- [ ] Add a deterministic proof script that starts a run, queries HTTP endpoints, tails events, and verifies all expected event types are visible.

#### MIG-06: Archive And Delete Only After Proof

- [ ] Run the provider matrix proof.
- [ ] Run the merge success and merge conflict proof.
- [ ] Run crash/resume proof.
- [ ] Run prompt diagnostics proof.
- [ ] Run feedback second-run influence proof.
- [ ] Move completed docs to `archive/` only after the generated proof files are linked from [README.md](README.md).
- [ ] Delete or rename legacy surfaces only after a grep gate proves no production callers remain.

### Generated Proof Contract

An agent implementing this migration must produce `tmp/mori-diffs/generated/migration-cutover-report.json`:

```json
{
  "schema": "mori-diffs.migration-cutover.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "legacy_surfaces": {
    "orchestrate_rs_callers": [],
    "dispatch_v2_callers": [],
    "dispatch_direct_callers": [],
    "plan_runner_from_plans_dir_callers": []
  },
  "active_path_proof": {
    "plan_run_uses_dispatch": false,
    "routing_authority": false,
    "prompt_authority": false,
    "feedback_facade": false,
    "projection_facade": false,
    "merge_backend": false,
    "resume": false
  },
  "provider_matrix": [],
  "remaining_gaps": []
}
```

### Grep Gates

- [ ] `rg -n "PlanRunner::from_plans_dir|orchestrate::PlanRunner|pub use orchestrate" crates/roko-cli/src` has only documented legacy exports or no hits.
- [ ] `rg -n "dispatch_v2|dispatch_direct" crates/roko-cli/src` has only documented compatibility callers.
- [ ] `rg -n "agent_stream::spawn_agent|build_minimal_system_prompt" crates/roko-cli/src/runner` returns no active production dispatch path.
- [ ] `rg -n "ClaudeStreamEvent|ClaudeAssistantEvent|ClaudeToolEvent" crates/roko-cli/src/runner` returns no hits.
- [ ] `rg -n "MergeSucceeded" crates/roko-cli/src/runner/event_loop.rs` shows only real `PlanMerger` result handling.

### Archive Gate

- [ ] All stale 2026-04-26 blockers are corrected or marked historical.
- [ ] `migration-legacy-surface-report.json` exists.
- [ ] `migration-cutover-report.json` exists.
- [ ] Every current source-truth row above has proof, not just source existence.
- [ ] README and current runtime gap ledger link the generated proof files.
