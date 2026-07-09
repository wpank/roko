# Orchestration Rearchitecture: Implementation Plan

> Converge 3 runtimes into 1 WorkflowEngine. Decompose the 22K LOC god object.
> Wire parallel execution, worktree isolation, wave gating, cumulative context
> handoff, failure recovery, and resume. 52 tasks across 9 phases.
>
> Sources: 04-ORCHESTRATION-AND-GATES-AUDIT.md, 17-ORCH-{AUDIT,GOALS,ISSUES,PLAN,PATTERNS}.md,
> 06-IMPLEMENTATION-PLANS.md, 22-RUNNER-LESSONS.md

---

## Current State

| Runtime | LOC | Status | Architecture |
|---|---|---|---|
| orchestrate.rs | 22,522 | Legacy dead code | 80+ field monolith, batch-only |
| Runner v2 | ~8,100 (15 files) | Active (CLI default) | Event-driven, streaming, serial |
| WorkflowEngine | ~4,022 (4 files) | Active (run/chat/ACP) | Pure FSM + EffectDriver |

**Target**: WorkflowEngine absorbs Runner v2's operational features and
orchestrate.rs's learning features. One runtime, one state machine, one dispatch
path. orchestrate.rs is deleted.

**Key numbers from mega-parity runner (195 batches, 177K LOC)**:
- Per-task compile: 15-40 min. Wave gate: 3-8 min. No gate: 0 min.
- Parallel (15 concurrent) vs serial: 10-15x speedup.
- Cumulative context reduced merge conflicts from ~50% to ~30%.
- Agent write speed (no build): 1-5 min/task.

---

## Phase 1: Worktree Integration (Foundation)

Everything parallel depends on isolation. No parallel execution is safe without
per-task worktrees. This phase wires the existing WorktreeManager and MergeQueue
into WorkflowEngine.

### TASK-O01: Add WorktreeManager to EffectServices
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-runtime/Cargo.toml`
**What**: Add optional WorktreeManager and MergeQueue to EffectServices so the
EffectDriver can allocate per-task worktrees.
**Steps**:
1. Add `roko-orchestrator` dependency to `roko-runtime/Cargo.toml`
2. Add `pub worktree_manager: Option<Arc<WorktreeManager>>` to `EffectServices`
3. Add `pub merge_queue: Option<Arc<MergeQueue>>` to `EffectServices`
4. Update all `EffectServices` construction sites to pass `None` (backward compatible)
5. In `EffectDriver::spawn_agent()`, when worktree_manager is Some, allocate a
   worktree via `create_for_plan(task_id)` and use its path as workdir
**Acceptance**: `cargo test -p roko-runtime` passes. Existing serial execution
unchanged (worktree_manager=None path tested).
**Depends on**: none
**Effort**: M

### TASK-O02: Wire WorktreeManager construction in ServiceFactory
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-orchestrator/src/service_factory.rs`
**What**: ServiceFactory constructs WorktreeManager and MergeQueue when parallel
execution is configured, and passes them into EffectServices.
**Steps**:
1. Read `max_parallel_tasks` from WorkflowConfig (added in TASK-O05)
2. If `max_parallel_tasks > 1`, construct WorktreeManager with workdir as base
3. Construct MergeQueue with file-overlap detection enabled
4. Pass both into EffectServices
5. If `max_parallel_tasks == 1`, pass None (serial mode, no worktree overhead)
**Acceptance**: ServiceFactory builds with and without worktree support.
Integration test constructs services with `max_parallel_tasks=4` and verifies
WorktreeManager is non-None.
**Depends on**: TASK-O01
**Effort**: S

### TASK-O03: Worktree lifecycle management in EffectDriver
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: EffectDriver manages worktree creation before agent spawn and cleanup
after task completion, including the merge-back step.
**Steps**:
1. Before `spawn_agent()`, if worktree_manager is Some: call `create_for_plan(task_id)`
   to get an isolated worktree path
2. Pass worktree path as the agent's working directory
3. After agent completes successfully and gates pass: enqueue merge via MergeQueue
4. Wait for merge slot (MergeQueue handles file-overlap serialization)
5. Execute git merge from worktree branch into integration branch
6. On merge failure: record attempt, retry up to 3 times with rebase
7. On merge success: optionally run post-merge regression gate
8. On task failure: preserve worktree for inspection (never auto-delete)
**Acceptance**: Integration test: spawn 2 tasks with worktree isolation, both
complete, both merge into integration branch without conflict.
**Depends on**: TASK-O01, TASK-O02
**Effort**: L

### TASK-O04: Post-merge regression gate
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: After merging a task's worktree into the integration branch, run a
lightweight regression gate (compile-only by default) to catch cross-task
integration errors.
**Steps**:
1. After successful merge, run `cargo check --workspace` in the integration branch
2. If regression gate fails, identify the merge that caused it (last merge commit)
3. Revert the merge, mark task for retry with regression error context
4. Make post-merge gate configurable: `post_merge_gate = ["compile"]` in config
5. For serial execution (max_parallel=1), skip post-merge gate (redundant)
**Acceptance**: Test: merge two tasks where the second breaks compilation of the
first. Post-merge gate catches the break. Offending task is retried.
**Depends on**: TASK-O03
**Effort**: M

---

## Phase 2: Parallel Execution

With worktree isolation in place, enable concurrent task dispatch. This is the
single highest-impact change: plans with 20 tasks in 4 waves go from 20x to ~5x
wall-clock time.

### TASK-O05: Add parallel config to WorkflowConfig
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/pipeline_state.rs`
**What**: Add `max_parallel_tasks` and `worktree_isolation` to WorkflowConfig,
parseable from TOML.
**Steps**:
1. Add `pub max_parallel_tasks: usize` to `WorkflowConfig` (default: 1)
2. Add `pub worktree_isolation: bool` to `WorkflowConfig` (default: false)
3. Parse from TOML: `[workflow] max_parallel_tasks = 4` and `worktree_isolation = true`
4. Validate: if `max_parallel_tasks > 1` and `worktree_isolation == false`, warn
   (parallel without isolation is unsafe)
5. Wire into `WorkflowRunConfig` so WorkflowEngine reads it
**Acceptance**: `WorkflowConfig::from_toml_str()` round-trips the new fields.
Default values preserve serial behavior.
**Depends on**: none
**Effort**: S

### TASK-O06: Concurrent task dispatch in WorkflowEngine
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Modify WorkflowEngine's run loop to dispatch multiple tasks from
TaskScheduler::next_batch() concurrently using tokio JoinSet.
**Steps**:
1. In the multi-task run loop, call `scheduler.next_batch()` to get ready tasks
2. For each task in batch, spawn into a `JoinSet` with its own worktree
3. Use `tokio::select!` to race task completions against cancellation
4. As each task completes, call `scheduler.mark_completed()` or `mark_failed()`
5. After each completion, re-check `scheduler.next_batch()` for newly unblocked tasks
6. Pass `max_parallel_tasks` from config to TaskScheduler
7. Ensure per-task PipelineStateV2 instances (one FSM per task, not shared)
**Acceptance**: Test plan with 4 independent tasks completes in ~1x single-task
time (not 4x). Test plan with serial dependencies still runs correctly.
**Depends on**: TASK-O03, TASK-O05
**Effort**: L

### TASK-O07: TaskScheduler supports dynamic max_parallel
**Priority**: P1
**Category**: improvement
**Files**: `crates/roko-runtime/src/task_scheduler.rs`
**What**: Allow TaskScheduler's max_parallel to be updated at runtime for
adaptive parallelism (Phase 7).
**Steps**:
1. Add `pub fn set_max_parallel(&mut self, n: usize)` method
2. `next_batch()` uses current max_parallel value
3. Add `pub fn running_count(&self) -> usize` for callers to check load
4. Add `pub fn stats(&self) -> SchedulerStats` returning completed, failed,
   running, pending counts
**Acceptance**: Unit test: change max_parallel mid-run, verify batch sizes change.
**Depends on**: none
**Effort**: S

### TASK-O08: Wire CLI plan-run to parallel config
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/main.rs`
**What**: `roko plan run` reads parallel config from roko.toml and CLI flags,
passes to WorkflowEngine.
**Steps**:
1. Add `--parallel <N>` CLI flag to `plan run` subcommand (overrides config)
2. Add `--worktree` CLI flag (enables worktree isolation, default when parallel>1)
3. Read `[workflow] max_parallel_tasks` from roko.toml as fallback
4. Construct WorkflowRunConfig with parallel settings
5. Pass to WorkflowEngine (which passes to ServiceFactory, which builds worktrees)
**Acceptance**: `roko plan run plans/ --parallel 4` dispatches up to 4 tasks
concurrently. `roko plan run plans/` (no flag) runs serially as before.
**Depends on**: TASK-O05, TASK-O06
**Effort**: M

---

## Phase 3: Context Handoff

The mega-parity runner proved that context handoff is the #1 factor in reducing
merge conflicts and improving agent output quality. This phase implements
cumulative context, gate failure context, and structured agent-to-agent handoff.

### TASK-O09: CumulativeContext struct and builder
**Priority**: P0
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs` (new submodule or inline)
**What**: Define a CumulativeContext struct that accumulates per-task change
summaries and renders them as a prompt section.
**Steps**:
1. Define `CumulativeContext` with `Vec<TaskChangeSummary>` and `token_count`
2. Define `TaskChangeSummary` with task_id, files_changed, diff_stat,
   functions_added, functions_modified
3. Implement `fn add_task(&mut self, summary: TaskChangeSummary)` with token budget
4. Implement `fn render(&self) -> String` producing markdown "## What Changed Before You"
5. Token budget: cap at 4000 tokens. Truncate oldest summaries when exceeding.
6. For large files, use signature-only views (function name + params, no body)
**Acceptance**: Unit test: add 5 task summaries, render produces valid markdown.
Token budget truncation tested with oversize input.
**Depends on**: none
**Effort**: M

### TASK-O10: Compute git diff summary after task completion
**Priority**: P0
**Category**: feature
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: After a task completes and its worktree is ready for merge, compute
a diff summary and add it to the plan's CumulativeContext.
**Steps**:
1. After agent completion, run `git diff --stat HEAD` in the task's worktree
2. Parse diff stat output into `TaskChangeSummary` fields
3. Optionally extract function signatures via `git diff HEAD` and regex for
   `fn ` / `pub fn ` / `struct ` / `impl ` patterns
4. Call `cumulative_context.add_task(summary)`
5. Pass cumulative_context through WorkflowEngine to subsequent task dispatches
**Acceptance**: After task T1 completes, task T2's prompt contains a
"What Changed Before You" section listing T1's changed files and functions.
**Depends on**: TASK-O09, TASK-O03
**Effort**: M

### TASK-O11: Inject cumulative context into agent prompts
**Priority**: P0
**Category**: feature
**Files**: `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: When dispatching an agent, include the rendered CumulativeContext as
a Layer 3 domain context section in the system prompt.
**Steps**:
1. Add `cumulative_context: Option<String>` parameter to EffectDriver::spawn_agent()
2. In PromptAssemblyService, accept cumulative context as a Layer 3c section
3. Insert after domain context, before task context (Layer 4)
4. When token budget is tight, cumulative context is trimmed before task context
5. Section ID: "cumulative_changes" for section effectiveness tracking
**Acceptance**: Agent receives cumulative context in system prompt. Section
appears between domain and task layers. Verified via episode metadata.
**Depends on**: TASK-O09, TASK-O10
**Effort**: M

### TASK-O12: Structured gate failure context for retries
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/pipeline_state.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: When a task fails gating and is retried, include structured per-gate
failure breakdowns instead of raw error strings.
**Steps**:
1. Define `StructuredFailureContext` with attempt, gate_name, gate_output,
   diff_from_prior, error_pattern, suggested_fix
2. Replace `last_gate_failure: Option<String>` in PipelineStateV2 with
   `last_gate_failures: Vec<StructuredFailureContext>`
3. After gate failure, populate StructuredFailureContext from GateVerdict fields
4. On retry dispatch, render failures as "## Previous Attempt Failed" section
   with per-gate error details
5. Include `diff --stat` of what the failed attempt changed
**Acceptance**: Retry agent's prompt includes structured failure context. Test:
compile failure shows error output; clippy failure shows warning list. Both
visible in prompt, not just raw strings.
**Depends on**: none
**Effort**: M

### TASK-O13: Structured strategy-to-implementer handoff
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/pipeline_state.rs`
**What**: Replace unstructured `strategist_brief: Option<String>` with a
structured handoff document.
**Steps**:
1. Define `StrategyBrief` struct: approach, key_constraints, files_to_modify,
   files_to_avoid, estimated_complexity, open_questions
2. Replace `strategist_brief` field with `strategy_brief: Option<StrategyBrief>`
3. Parse strategist agent's output into StrategyBrief fields (best-effort,
   fall back to raw string if parsing fails)
4. Render as structured markdown for implementer prompt
5. Include in PipelineStateV2 checkpoint serialization
**Acceptance**: Strategist -> implementer handoff includes structured fields.
Checkpoint round-trips correctly with the new struct.
**Depends on**: none
**Effort**: S

---

## Phase 4: Feature Extraction from orchestrate.rs

The 22K LOC monolith contains features that no other runtime has. This phase
extracts the most valuable ones into service traits that WorkflowEngine uses.
Each trait goes into roko-core/src/foundation.rs with concrete implementations
in their respective crates.

### TASK-O14: Extract EpisodeRecorder trait
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`, `crates/roko-learn/src/lib.rs`,
`crates/roko-runtime/src/effect_driver.rs`
**What**: Define an EpisodeRecorder trait and wire it into EffectDriver so every
model call and gate result is recorded as an episode.
**Steps**:
1. Define trait in foundation.rs: `record_turn(turn)`, `record_gate(result)`,
   `finalize_episode(outcome)`, all returning `Result<()>`
2. Implement trait on EpisodeLogger in roko-learn
3. Add `pub episode_recorder: Option<Arc<dyn EpisodeRecorder>>` to EffectServices
4. In EffectDriver, after each model call: `episode_recorder.record_turn()`
5. After each gate: `episode_recorder.record_gate()`
6. After task completion: `episode_recorder.finalize_episode()`
7. Wire in ServiceFactory: construct EpisodeLogger from `.roko/episodes.jsonl`
**Acceptance**: `roko plan run` produces entries in `.roko/episodes.jsonl`. Each
entry has non-zero token counts and gate verdicts.
**Depends on**: none
**Effort**: M

### TASK-O15: Extract KnowledgeRouter trait
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`, `crates/roko-neuro/src/knowledge_store.rs`,
`crates/roko-runtime/src/effect_driver.rs`
**What**: Define a KnowledgeRouter trait that queries the neuro store for context
relevant to the current task, injected into the system prompt.
**Steps**:
1. Define trait: `fn route(&self, task_description: &str, role: &str) -> Vec<KnowledgeEntry>`
2. Implement on KnowledgeStore in roko-neuro
3. Add `pub knowledge_router: Option<Arc<dyn KnowledgeRouter>>` to EffectServices
4. In EffectDriver, before prompt assembly: query knowledge router
5. Pass results to PromptAssemblyService as Layer 3 domain context
6. Port logic from orchestrate.rs `build_knowledge_routing_advice()`
**Acceptance**: Knowledge store entries appear in agent prompts when relevant
knowledge exists. Empty when store is empty (no crash, no noise).
**Depends on**: none
**Effort**: M

### TASK-O16: Extract PlaybookQuery trait
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-core/src/foundation.rs`, `crates/roko-learn/src/playbook.rs`,
`crates/roko-runtime/src/effect_driver.rs`
**What**: Define a trait for querying playbooks (proven action sequences) and
inject matching playbooks into agent prompts as Layer 6 techniques.
**Steps**:
1. Define trait: `fn query(&self, task_category: &str, role: &str) -> Vec<Playbook>`
2. Implement on PlaybookStore in roko-learn
3. Add `pub playbook_query: Option<Arc<dyn PlaybookQuery>>` to EffectServices
4. In EffectDriver, before prompt assembly: query matching playbooks
5. Pass results to PromptAssemblyService for Layer 6 injection
6. Already partially wired in ServiceFactory -- verify and complete
**Acceptance**: Matching playbooks appear in agent prompts. `roko learn all`
shows playbook data after runs that generate successful tool sequences.
**Depends on**: none
**Effort**: M

### TASK-O17: Wire CascadeRouter observations in EffectDriver
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: After each model call, record a CascadeRouter observation so the
router learns which models work best for which task types.
**Steps**:
1. FeedbackSink trait already supports `record_model_call()` -- verify it calls
   CascadeRouter internally via FeedbackService
2. Ensure EffectDriver calls `feedback_sink.record_model_call()` after every
   agent completion (verify this path exists end-to-end)
3. Ensure FeedbackService persists CascadeRouter to `.roko/learn/cascade-router.json`
   at run end
4. Verify observations include quality, cost, and duration dimensions
5. Test: run twice, verify observation count increases in cascade-router.json
**Acceptance**: `.roko/learn/cascade-router.json` has observations after
`roko plan run`. Observation count grows across runs.
**Depends on**: none
**Effort**: S

### TASK-O18: Wire AdaptiveThreshold observations in EffectDriver
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-gate/src/gate_service.rs`
**What**: After each gate rung executes, call `thresholds.observe(rung, passed)`
so adaptive skip can work. Persist thresholds at run end.
**Steps**:
1. Add `pub adaptive_thresholds: Option<Arc<Mutex<AdaptiveThresholds>>>` to
   EffectServices
2. In EffectDriver gate execution, after each GateVerdict: call
   `thresholds.observe(rung, verdict.passed)`
3. Before running a gate, check `thresholds.should_skip_rung_adaptively(rung)`
   -- skip if true (except rung 0, never skip compile)
4. At run end, persist to `.roko/learn/gate-thresholds.json`
5. Wire in ServiceFactory: load or construct AdaptiveThresholds
**Acceptance**: `.roko/learn/gate-thresholds.json` updated after runs. After 20+
consecutive clippy passes, clippy is skipped on next run.
**Depends on**: none
**Effort**: M

### TASK-O19: Wire SectionEffectiveness recording
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: After each task, record which prompt sections contributed to success
or failure so the prompt builder can optimize section priority.
**Steps**:
1. Add `pub section_effectiveness: Option<Arc<Mutex<SectionEffectivenessRegistry>>>`
   to EffectServices
2. PromptAssemblyService already returns section IDs in its output -- capture these
3. After task success: record section IDs as positive
4. After task failure: record section IDs as negative
5. At run end: persist to `.roko/learn/section-effects.json`
6. PromptAssemblyService already reads section effectiveness for priority weighting
   -- verify this path works end-to-end
**Acceptance**: `.roko/learn/section-effects.json` has section entries after runs.
Positive/negative counts reflect task outcomes.
**Depends on**: TASK-O14
**Effort**: S

### TASK-O20: Wire replan-on-gate-failure
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: When a task exhausts its autofix budget, trigger architectural
replanning instead of immediately failing.
**Steps**:
1. Import ReplanStrategy and PlanRevisionRequest from roko-orchestrator
2. Track gate_failure_count per task in WorkflowEngine
3. After autofix exhaustion (PipelineStateV2 would transition to Halted):
   intercept and check `config.learning.replan_on_gate_failure` (default true)
4. If true and replan_attempts < 2: construct PlanRevisionRequest with gate error
   context, spawn strategist agent to generate revised approach
5. If revision succeeds, reset task PipelineStateV2 to Implementing with new prompt
6. If revision fails or attempts >= 2, proceed to Halted as before
7. Port logic from orchestrate.rs `build_gate_failure_plan_revision()`
**Acceptance**: Task that consistently fails gates triggers replan (strategist
spawned). Replan capped at 2 attempts. After exhaustion, task halts normally.
**Depends on**: none
**Effort**: L

---

## Phase 5: State Machine Convergence

Two state machines (PipelineStateV2 with 10 states, PlanPhase with 14 states)
model the same concept. This phase defines a unified state model that covers
both single-prompt and multi-task workflows.

### TASK-O21: Define UnifiedPhase superset
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-core/src/phase.rs`
**What**: Define a phase enum that is the superset of PipelineStateV2 and
PlanPhase, with optional phases that can be skipped based on workflow config.
**Steps**:
1. Define `UnifiedPhase` covering all phases from both state machines:
   Pending, Strategizing, Enriching, Implementing, Gating, AutoFixing,
   Verifying, Reviewing, DocRevision, Committing/Merging, Complete, Failed,
   Skipped, Cancelled
2. Define `FailureKind` superset from both (typed enum, not string)
3. Define `fn valid_transitions(from: &UnifiedPhase) -> Vec<UnifiedPhase>`
4. Add Serialize/Deserialize derives
5. Implement `From<PipelinePhase>` and `From<PlanPhase>` for migration
**Acceptance**: UnifiedPhase covers every state from both existing machines.
Conversion from either existing enum is lossless. Serialization round-trips.
**Depends on**: none
**Effort**: M

### TASK-O22: Migrate PipelineStateV2 to UnifiedPhase
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/pipeline_state.rs`
**What**: Replace PipelineStateV2's internal phase type with UnifiedPhase. The
step() function dispatches on the same variants but uses the unified enum.
**Steps**:
1. Replace `phase: PipelinePhase` with `phase: UnifiedPhase`
2. Update all match arms in `step()` to use UnifiedPhase variants
3. Phases not used by single-prompt workflows (Enriching, Verifying, DocRevision,
   Merging) are simply never entered -- no code change needed for them
4. Update checkpoint serialization to use UnifiedPhase
5. Add migration: if checkpoint JSON has old PipelinePhase names, map to UnifiedPhase
**Acceptance**: All existing PipelineStateV2 tests pass unchanged. Checkpoint
format backward-compatible (old checkpoints load into new code).
**Depends on**: TASK-O21
**Effort**: M

### TASK-O23: Migrate Runner v2 to UnifiedPhase
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/state.rs`
**What**: Replace PlanPhase usage in Runner v2 with UnifiedPhase so Runner v2
and WorkflowEngine report the same phase taxonomy.
**Steps**:
1. Replace `PlanPhase` imports with `UnifiedPhase`
2. Update RunState tracking to use UnifiedPhase
3. Update TUI bridge events to report UnifiedPhase
4. Update persist/resume to serialize UnifiedPhase
5. Add migration for existing executor.json snapshots
**Acceptance**: `roko plan run` reports phases using UnifiedPhase names. TUI
displays consistent phase names. Resume from old snapshots works.
**Depends on**: TASK-O21, TASK-O22
**Effort**: M

---

## Phase 6: DAG Improvements

The DAG infrastructure is sophisticated (2,557 LOC with CPM, waves, mutations,
fusion, culling) but underutilized. This phase enables cost-aware scheduling,
chain fusion for mechanical tasks, and live DAG mutations.

### TASK-O24: Cost-aware task priority scoring
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/task_scheduler.rs`
**What**: Score ready tasks by priority before selection, considering critical
path membership, downstream dependency count, and estimated cost.
**Steps**:
1. Define `fn priority_score(task: &SchedulableTask) -> f64`
2. Scoring formula: `critical_path_bonus * 2.0 + downstream_dependents * 0.5 +
   (1.0 / estimated_cost) * 0.3`
3. Determine critical path via slack computation: zero-slack tasks get bonus
4. Count downstream dependents via BFS from task
5. Estimated cost from task tier: mechanical=1, focused=1.5, integrative=2.5,
   architectural=4.0
6. In `next_batch()`, sort candidates by priority before applying file exclusion
**Acceptance**: Unit test: task with 4 downstream dependents dispatched before
task with 1. Critical path task dispatched before non-critical.
**Depends on**: none
**Effort**: M

### TASK-O25: Wire chain fusion for mechanical tasks
**Priority**: P2
**Category**: improvement
**Files**: `crates/roko-runtime/src/task_scheduler.rs`, `crates/roko-orchestrator/src/dag.rs`
**What**: Before execution, fuse linear chains of mechanical-tier tasks to reduce
wave count and scheduling overhead.
**Steps**:
1. Call `dag.fuse_linear_chains(config)` after DAG construction
2. Set `same_tier_only = true` (only fuse tasks at the same complexity tier)
3. Set `max_chain_length = 3` (don't create mega-tasks)
4. Preserve average parallelism: `ave_width` threshold prevents over-fusion
5. Report fusion count in run summary
**Acceptance**: Plan with A->B->C (all mechanical) fuses to ABC. Plan with
A->B (architectural) ->C (mechanical) does not fuse. Average parallelism
maintained above threshold.
**Depends on**: none
**Effort**: S

### TASK-O26: Live DAG mutation for task injection
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Support adding tasks to the DAG mid-execution, e.g., when a replan
generates new subtasks.
**Steps**:
1. Expose `TaskScheduler::add_task(id, task, depends_on)` method
2. Internally, validate no cycles via clone-and-validate pattern
3. New tasks are immediately eligible for scheduling if deps are met
4. Update checkpoint to include dynamically added tasks
5. Wire to replan: when TASK-O20's replan generates subtasks, inject them via
   `add_task()`
**Acceptance**: Test: add a task mid-execution, it runs after its deps complete.
Cycle injection rejected. Checkpoint includes dynamic tasks.
**Depends on**: TASK-O20
**Effort**: M

### TASK-O27: Target culling for partial plan execution
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Support `roko plan run plans/ --target T5,T8` to run only the specified
tasks and their transitive dependencies.
**Steps**:
1. Add `--target <task_ids>` CLI flag (comma-separated)
2. After DAG construction, call `dag.cull(targets)` to remove unnecessary tasks
3. Report culled task count in run summary
4. Culled tasks marked as Skipped in the snapshot
**Acceptance**: `--target T5` in a 10-task plan runs only T5 and its deps.
Unrelated tasks are Skipped. Snapshot reflects culling.
**Depends on**: none
**Effort**: S

---

## Phase 7: Wave Gating

Per-task compilation is 15-40 minutes. Wave gates (compile after a batch of
tasks) are 3-8 minutes. This phase implements configurable gate timing.

### TASK-O28: Gate timing configuration
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/pipeline_state.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: Support three gate timing modes: per-task (default), per-wave, and
deferred. Configurable in TOML.
**Steps**:
1. Define `enum GateTiming { PerTask, PerWave, Deferred }`
2. Add `gate_timing: GateTiming` to WorkflowConfig
3. Parse from TOML: `[workflow] gate_timing = "per-wave"`
4. PerTask: run gates after each task (current behavior)
5. PerWave: skip gates per-task, run at wave boundaries
6. Deferred: skip all gates, run once at plan completion
7. Anti-pattern checks always run regardless of timing mode (millisecond cost)
**Acceptance**: `gate_timing = "per-wave"` runs gates 3 times for a 10-task
3-wave plan (not 10 times). Total gate time measured and logged.
**Depends on**: TASK-O05
**Effort**: M

### TASK-O29: Wave boundary detection and gate dispatch
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: When gate_timing is PerWave, detect wave boundaries (all tasks in
current wave completed) and run gates on the merged integration branch.
**Steps**:
1. Track current wave via TaskScheduler (wave number from DAG)
2. After each task completion, check if all tasks in current wave are done
3. If wave complete: merge all wave tasks into integration branch, then run
   full gate pipeline on the integration branch
4. If wave gate fails: identify offending task via git log of wave merges
5. Retry offending task with gate failure context, other tasks' work preserved
6. If wave gate passes: advance to next wave
**Acceptance**: Test: 3 tasks in wave 0, gates run once (not 3 times). Wave gate
failure identifies which task caused regression.
**Depends on**: TASK-O06, TASK-O28
**Effort**: L

### TASK-O30: Wave gate bisection for failure identification
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: When a wave gate fails, binary search across the wave's merge commits
to identify which task caused the regression.
**Steps**:
1. Record merge commit hashes for each task in the wave
2. On wave gate failure: checkout mid-point commit, run gate
3. If mid-point passes: regression is in later merges
4. If mid-point fails: regression is in earlier merges
5. Recurse until single offending merge identified
6. Report offending task ID and gate error
7. Retry only the offending task(s), not the entire wave
**Acceptance**: Wave with 4 tasks, task 3 breaks compile. Bisection finds task 3
in 2 steps (not 4). Only task 3 is retried.
**Depends on**: TASK-O29
**Effort**: M

### TASK-O31: No-build prompt injection for deferred/wave gating
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: When gate_timing is not PerTask, inject a "do not compile" section
into agent prompts to prevent agents from running expensive builds.
**Steps**:
1. When gate_timing is PerWave or Deferred, add a prompt section:
   "## Build Policy\nDo NOT run any compilation or test commands. Focus on writing
   correct code. Verification will happen at wave boundaries."
2. Inject as a high-priority section (before tool instructions)
3. Optionally: set `denied_tools = ["cargo"]` in agent spawn config to
   restrict tool access as a fallback for non-compliant agents
4. Monitor agent duration: if >3x expected for tier, likely building despite
   instructions (warn in run summary)
**Acceptance**: With `gate_timing = "per-wave"`, agent prompt contains build
policy section. Agent that ignores policy detected by duration monitoring.
**Depends on**: TASK-O28
**Effort**: S

---

## Phase 8: Resume and Failure Recovery

Robust resume is essential for any long-running process. This phase unifies
the three resume mechanisms into one, adds fingerprint validation, and
implements configurable failure routing.

### TASK-O32: Unified checkpoint format
**Priority**: P0
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/workflow_engine.rs`, `crates/roko-runtime/src/task_scheduler.rs`
**What**: Extend WorkflowEngine checkpoint to include TaskScheduler state so
crash recovery preserves task-level progress.
**Steps**:
1. Add `Serialize + Deserialize` derives to TaskStatus in task_scheduler.rs
2. Define `WorkflowCheckpoint` struct containing:
   - `pipeline_states: HashMap<String, PipelineStateV2>` (per-task FSM)
   - `task_statuses: HashMap<String, TaskStatus>` (scheduler state)
   - `cumulative_context: CumulativeContext`
   - `gate_failure_counts: HashMap<String, u32>`
   - `schema_version: u32`
   - `timestamp_ms: u64`
3. Atomic write via tmp + rename (existing pattern in EffectDriver)
4. Checkpoint after every task completion (crash-safe)
5. Path: `.roko/state/workflow-checkpoint.json`
**Acceptance**: Kill WorkflowEngine mid-run (10-task plan at task 5). Resume.
Tasks 1-5 are not re-executed. Task 6 starts from scratch.
**Depends on**: TASK-O06
**Effort**: M

### TASK-O33: Fingerprint validation on resume
**Priority**: P0
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: On resume, hash each task definition and compare against the checkpoint.
Reject resume if tasks have been edited since the run started.
**Steps**:
1. Define `TaskDefFingerprint` as SHA-256 of task title + description + files +
   depends_on (same fields Runner v2 uses)
2. Record fingerprints at run start, include in checkpoint
3. On resume: recompute fingerprints, compare against checkpoint
4. If any task fingerprint changed: report which tasks diverged, abort resume
   with actionable error message
5. Allow `--force-resume` flag to override fingerprint validation
**Acceptance**: Resume after editing a task definition: error message names the
changed task. Resume with unedited tasks: succeeds. `--force-resume`: bypasses.
**Depends on**: TASK-O32
**Effort**: M

### TASK-O34: JSONL truncation recovery
**Priority**: P1
**Category**: improvement
**Files**: `crates/roko-runtime/src/jsonl_logger.rs`
**What**: On resume, detect and recover from truncated JSONL files (e.g.,
episodes.jsonl cut mid-line by a crash).
**Steps**:
1. On file open for append, read the last line
2. If last line is not valid JSON, truncate the file to the last valid newline
3. Log a warning with the truncated content
4. Continue appending from the recovered position
5. Port logic from Runner v2's `prepare_resume()` JSONL recovery
**Acceptance**: Write 5 entries to JSONL, corrupt the last one (truncate mid-line).
Recovery removes the corrupt entry. Subsequent appends work correctly.
**Depends on**: none
**Effort**: S

### TASK-O35: Configurable per-gate failure routing
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-runtime/src/pipeline_state.rs`
**What**: Replace hardcoded failure routing with per-gate configurable policies
parsed from TOML.
**Steps**:
1. Define `FailurePolicy` struct: action (autofix/reimplement/escalate/skip/halt/replan),
   max_attempts, escalation_model
2. Add `failure_policies: HashMap<String, FailurePolicy>` to WorkflowConfig
3. Parse from TOML: `[workflow.failure.compile] action = "autofix"` etc.
4. In PipelineStateV2::step(), on gate failure: lookup policy for the failed gate
5. Route to the policy's action instead of the hardcoded autofix->reimplement->halt
6. Default policy: autofix(2) for compile/clippy/fmt, reimplement(2) for test,
   halt for judge
**Acceptance**: `[workflow.failure.clippy] action = "skip"` causes clippy failures
to be skipped (not autofix). `[workflow.failure.test] action = "reimplement"`
causes test failures to re-run implementation (not autofix).
**Depends on**: TASK-O05
**Effort**: M

### TASK-O36: Model escalation on repeated failure
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: When a task fails repeatedly with the same model, escalate to a
stronger model for the next attempt.
**Steps**:
1. Track `(task_id, model) -> failure_count` in WorkflowEngine
2. After N failures with the same model (N configurable, default 2):
   request a stronger model from CascadeRouter
3. CascadeRouter already supports tier-based routing -- call with higher tier
4. Log the escalation event for learning
5. If the stronger model succeeds, record a positive observation for that model
   at this task type
**Acceptance**: Task fails twice with gpt-5.4-mini, third attempt uses
claude-sonnet-4-20250514 (or configured fallback). Escalation logged.
**Depends on**: TASK-O17
**Effort**: M

### TASK-O37: Per-task status files for manual intervention
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Write per-task status files alongside the checkpoint for human-readable
run state and manual override capability.
**Steps**:
1. Write `.roko/state/tasks/{plan_id}/{task_id}.status` containing phase name
2. Update on every task phase transition
3. On resume: if status file says "complete" but checkpoint says "failed", trust
   the file (manual override)
4. `echo "complete" > .roko/state/tasks/plan-1/T3.status` skips T3 and unblocks
   its dependents
5. `echo "skip" > .roko/state/tasks/plan-1/T3.status` skips without running
6. `ls .roko/state/tasks/` gives immediate visibility into run state
**Acceptance**: `ls .roko/state/tasks/plan/` shows one file per task with current
status. Manual override to "complete" unblocks downstream tasks on resume.
**Depends on**: TASK-O32
**Effort**: M

---

## Phase 9: Adaptive Parallelism and Speculative Execution

Advanced scheduling optimizations that dynamically adjust concurrency based on
runtime signals and pre-dispatch tasks likely to become ready.

### TASK-O38: Adaptive parallelism controller
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Dynamically adjust max_parallel_tasks based on error rate, disk
pressure, and merge conflict rate.
**Steps**:
1. Define `AdaptiveParallelism` struct with error_window (VecDeque<bool>),
   base_max_parallel, current_max_parallel
2. `adjust(task_succeeded: bool) -> usize`:
   - Error rate > 30%: halve current_max_parallel (min 1)
   - Error rate < 10%: increment current_max_parallel (up to base)
3. Disk pressure check: `statvfs` available bytes < 5GB: pause dispatch
4. Merge conflict rate > 20%: reduce current_max_parallel
5. Feed current_max_parallel to TaskScheduler before each next_batch()
6. Log all adjustments as RuntimeEvents
**Acceptance**: Inject 3 consecutive failures: parallelism drops. Inject 10
consecutive successes: parallelism recovers to base. Disk check prevents
dispatch when free space is low.
**Depends on**: TASK-O06, TASK-O07
**Effort**: M

### TASK-O39: Speculative execution trigger
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Pre-dispatch agents for critical-path tasks when their dependencies
are nearly complete.
**Steps**:
1. After each task completion, identify critical-path tasks (zero slack)
2. For each critical-path task: count completed deps / total deps
3. If ratio >= 0.8 and task not already running/speculative: spawn speculatively
4. Track speculative tasks in a HashSet
5. If a dependency fails: cancel speculative agent via CancelToken, clean up
   worktree, record wasted cost
6. If all deps complete: promote speculative to real (no restart needed)
7. Cost tracking: `speculative_hit_count`, `speculative_miss_count`,
   `speculative_wasted_cost_usd`
**Acceptance**: Plan where T5 depends on T1,T2,T3,T4. After T1,T2,T3 complete
(80%), T5 is speculatively started. If T4 completes, T5 continues seamlessly.
If T4 fails, T5 is cancelled.
**Depends on**: TASK-O06, TASK-O24
**Effort**: L

### TASK-O40: Speculative execution cost guard
**Priority**: P2
**Category**: improvement
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Only speculate when the expected wasted cost is below a budget
threshold.
**Steps**:
1. Read `speculative_threshold_multiplier` from config (default 0.2 = 20% of
   remaining budget)
2. Estimate speculative task cost from tier multiplier
3. Compute expected waste: `cost * (1 - dep_completion_ratio)`
4. Only speculate if `expected_waste < threshold * remaining_budget`
5. Track speculative cost separately in run summary
**Acceptance**: With tight budget, speculation is suppressed. With generous
budget, speculation proceeds. Run summary shows speculative cost breakdown.
**Depends on**: TASK-O39
**Effort**: S

---

## Phase 10: Anti-Pattern Pre-Gate

Fast grep-based checks that catch LLM code generation mistakes in milliseconds,
before any expensive compilation. Derived from the mega-parity runner's AP checks.

### TASK-O41: Anti-pattern check registry
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-gate/src/antipattern.rs` (new file)
**What**: Define a registry of fast anti-pattern checks that run via regex/grep,
catching common LLM mistakes without compilation.
**Steps**:
1. Define `AntiPatternCheck` struct: id, name, pattern (Regex), file_glob,
   exclude_test, severity (Error/Warning), message
2. Define `AntiPatternRegistry` with `Vec<AntiPatternCheck>` and `fn check(workdir)
   -> Vec<AntiPatternViolation>`
3. Implement 10 initial checks from mega-parity runner:
   - AP-1: Stub gate returning pass (`GateVerdict.*passed.*true` outside test)
   - AP-2: `block_on` in async code
   - AP-3: Duplicate trait definitions
   - AP-5: Raw `Command::new("claude")` (should use dispatcher)
   - AP-6: Inline prompt strings (`format!("You are a"`)
   - AP-7: `std::sync::Mutex` held across `.await`
   - AP-8: Empty function bodies (`{}` immediately after fn signature)
   - AP-9: `unimplemented!()` / `unreachable!()` left behind
   - AP-10: Hardcoded localhost/port in non-test code
   - AP-11: `#[allow(unused)]` blanket suppression
4. Each check runs in milliseconds (regex, no cargo)
**Acceptance**: Registry with 10 checks. `check()` on a file with AP-7 violation
returns the violation with correct line number and message.
**Depends on**: none
**Effort**: M

### TASK-O42: Integrate anti-pattern checks as pre-gate
**Priority**: P1
**Category**: feature
**Files**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: Run anti-pattern checks before any compilation gate. If AP checks fail
with Error severity, short-circuit to autofix without wasting compile time.
**Steps**:
1. Implement `GateRunner` trait on `AntiPatternGate` wrapping AntiPatternRegistry
2. In GateService, run AP gate at rung -1 (before compile) when configured
3. AP Error violations -> GateVerdict with passed=false
4. AP Warning violations -> GateVerdict with passed=true (warning only)
5. AP gate runs in <100ms -- verify via test timing assertion
6. Add `enabled_ap_checks` to gate config (default: all enabled)
7. Per-task exemptions via `ap_exempt = ["AP-10"]` in tasks.toml
**Acceptance**: Task with AP-7 violation fails AP gate before compile runs (saves
3-8 min). Exempt task with `ap_exempt = ["AP-10"]` passes AP gate despite AP-10
match. AP gate completes in <100ms.
**Depends on**: TASK-O41
**Effort**: M

---

## Phase 11: Remaining Learning Features

Features from orchestrate.rs that are lower priority but needed for full
self-hosting fidelity.

### TASK-O43: Wire EfficiencyWriter into EffectDriver
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: After each agent turn, emit an AgentEfficiencyEvent with token/cost/tool
data to `.roko/learn/efficiency.jsonl`.
**Steps**:
1. Add `pub efficiency_writer: Option<Arc<Mutex<EfficiencyWriter>>>` to EffectServices
2. After each model call completion, construct AgentEfficiencyEvent from response
3. Include: input_tokens, output_tokens, cost_usd, wall_ms, tools_available,
   tools_used, letter_grade
4. Call `efficiency_writer.append(&event)` (includes flush)
5. Wire in ServiceFactory: construct from `.roko/learn/efficiency.jsonl`
**Acceptance**: `.roko/learn/efficiency.jsonl` has entries after `roko plan run`.
Each entry has non-zero token counts. File grows per-agent-call (not buffered).
**Depends on**: none
**Effort**: S

### TASK-O44: Wire DaimonState affect engine
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-orchestrator/src/service_factory.rs`
**What**: Load DaimonState from disk and use it as the AffectPolicy in
EffectDriver, instead of the default policy.
**Steps**:
1. In ServiceFactory, load DaimonState from `.roko/state/daimon.json` via
   `DaimonState::load_or_new()`
2. Construct DaimonPolicy from loaded state
3. Pass as `affect_policy` in EffectServices
4. After each task outcome, update DaimonState with outcome signal
5. Persist DaimonState at run end
**Acceptance**: Run with prior DaimonState: affect modulation uses loaded state
(not default). Temperature, exploration_rate, tier_bias reflect affect state.
**Depends on**: none
**Effort**: M

### TASK-O45: Wire dream consolidation trigger
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: After plan completion, optionally trigger dream consolidation to
distill knowledge from the run.
**Steps**:
1. Import DreamRunner and DreamLoopConfig from roko-dreams
2. After successful plan completion: check `config.dreams.auto_consolidate`
3. If true: spawn DreamRunner with episodes from this run
4. DreamRunner distills patterns, updates knowledge store
5. Log dream consolidation results in run summary
**Acceptance**: After plan completion with `auto_consolidate = true`, dream
consolidation runs. Knowledge store has new entries from the run.
**Depends on**: TASK-O14
**Effort**: M

### TASK-O46: Wire custody audit chain
**Priority**: P2
**Category**: rearchitecture
**Files**: `crates/roko-runtime/src/effect_driver.rs`
**What**: Record custody events for every agent dispatch and gate execution,
providing an audit trail of who did what and when.
**Steps**:
1. Define `CustodyRecorder` trait in foundation.rs: `record_dispatch(agent, task,
   model, timestamp)`, `record_gate(task, gate, verdict, timestamp)`
2. Implement on CustodyLogger from roko-agent
3. Add `pub custody_recorder: Option<Arc<dyn CustodyRecorder>>` to EffectServices
4. Call from EffectDriver at dispatch and gate points
5. Wire in ServiceFactory: construct CustodyLogger writing to `.roko/custody.jsonl`
**Acceptance**: `.roko/custody.jsonl` has entries after runs. Each dispatch and
gate execution has a custody record with timestamp and actor.
**Depends on**: none
**Effort**: M

---

## Phase 12: Runtime Convergence and Cleanup

Final phase: retire orchestrate.rs, align Runner v2 with WorkflowEngine, and
clean up the codebase.

### TASK-O47: Route Runner v2 through WorkflowEngine
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-runtime/src/workflow_engine.rs`
**What**: Replace Runner v2's custom event loop with WorkflowEngine calls. Runner
v2 becomes a thin CLI adapter that constructs WorkflowRunConfig and calls
WorkflowEngine::run().
**Steps**:
1. WorkflowEngine must support all Runner v2 features by this point (Phases 1-11)
2. Replace Runner v2's main loop with:
   `let result = WorkflowEngine::run(config, services, cancel).await`
3. Keep Runner v2's CLI argument parsing, plan loading, and TUI setup
4. Keep Runner v2's result reporting and exit code logic
5. Remove Runner v2's duplicate state machine, gate dispatch, and agent dispatch
6. Verify: all Runner v2 tests pass through WorkflowEngine path
**Acceptance**: `roko plan run` uses WorkflowEngine internally. All existing
Runner v2 tests pass. Learning files populated. TUI updates work.
**Depends on**: TASK-O06, TASK-O14, TASK-O17, TASK-O18, TASK-O32, TASK-O33
**Effort**: L

### TASK-O48: Deprecate orchestrate.rs behind feature flag
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-cli/Cargo.toml`
**What**: Gate orchestrate.rs behind `#[cfg(feature = "legacy-orchestrate")]`
and remove from default compilation.
**Steps**:
1. Add `legacy-orchestrate = []` feature to roko-cli/Cargo.toml
2. Wrap entire orchestrate.rs with `#[cfg(feature = "legacy-orchestrate")]`
3. Remove orchestrate.rs from CLI dispatch (no code path reaches it)
4. Update any remaining imports that reference orchestrate.rs types
5. `cargo check --workspace` clean without the feature
**Acceptance**: `cargo check --workspace` passes without legacy-orchestrate.
No code path reaches orchestrate.rs in default builds.
**Depends on**: TASK-O47
**Effort**: M

### TASK-O49: Delete orchestrate.rs
**Priority**: P1
**Category**: rearchitecture
**Files**: `crates/roko-cli/src/orchestrate.rs`
**What**: Remove the 22K LOC file entirely after verifying all features are
ported and no tests depend on it.
**Steps**:
1. Verify all features from the audit's "Features Only in Dead Code" table are
   either ported or explicitly deprecated
2. Delete `orchestrate.rs`
3. Remove the `legacy-orchestrate` feature from Cargo.toml
4. Remove unused dependencies from roko-cli/Cargo.toml
5. Run `cargo test --workspace` to verify nothing breaks
6. Update CLAUDE.md to remove orchestrate.rs references
**Acceptance**: `cargo test --workspace` passes. `orchestrate.rs` gone. 22K LOC
removed from workspace. No orphaned imports.
**Depends on**: TASK-O48
**Effort**: S

### TASK-O50: De-duplicate gate rung mapping
**Priority**: P1
**Category**: improvement
**Files**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-runtime/src/effect_driver.rs`
**What**: Export `rung_for_gate_name` as a public function from roko-gate and
remove the duplicate in EffectDriver.
**Steps**:
1. In roko-gate/src/gate_service.rs: make `rung_for_gate_name` public
2. Export via roko-gate/src/lib.rs
3. In roko-runtime/src/effect_driver.rs: replace local `rung_for_gate_name` with
   import from roko-gate
4. Remove the TODO comment acknowledging the duplication
5. Add GateVerdict fields: `rung: u8` and `confidence: f64` (from ORCH-018)
**Acceptance**: Only one `rung_for_gate_name` in the codebase (in roko-gate).
EffectDriver imports it. GateVerdict carries rung and confidence.
**Depends on**: none
**Effort**: S

### TASK-O51: Set ProcessSupervisor restart default to 1
**Priority**: P2
**Category**: improvement
**Files**: `crates/roko-runtime/src/process.rs`
**What**: Change SupervisionStrategy default from OneForOne(max_restarts=0) to
OneForOne(max_restarts=1) so transient failures get one automatic retry.
**Steps**:
1. In `SupervisionStrategy::default()`, set `max_restarts: 1`
2. Set `within_ms: 60_000` (1 restart per minute window)
3. Ensure restart uses the same SpawnConfig (same args, env, workdir)
4. Log restart events at WARN level
5. After restart exhaustion, behave as before (process failure is terminal)
**Acceptance**: Agent process that crashes once is automatically restarted.
Agent that crashes twice within 60 seconds is not restarted.
**Depends on**: none
**Effort**: S

### TASK-O52: Disk pressure monitoring
**Priority**: P2
**Category**: feature
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Monitor available disk space and pause task dispatch when space is low.
**Steps**:
1. Before each task dispatch, check available disk space via `statvfs`
2. If available < 5GB: pause dispatch, log warning, emit RuntimeEvent
3. Check every 60 seconds while paused
4. Resume dispatch when space is freed (worktree cleanup, cargo clean, etc.)
5. Make threshold configurable: `[workflow] min_disk_gb = 5`
6. For worktree cleanup: offer to remove worktrees from completed tasks older
   than a configurable TTL
**Acceptance**: With <5GB free: dispatch pauses, warning logged. After freeing
space: dispatch resumes. Threshold configurable.
**Depends on**: none
**Effort**: S

---

## Dependency Graph

```
Phase 1 (Worktree)     Phase 3 (Context)     Phase 4 (Features)
O01 -> O02 -> O03      O09 --------+         O14 (episodes)
       O03 -> O04      O10 -> O11  |         O15 (knowledge)
                        O12        |         O16 (playbooks)
Phase 2 (Parallel)     O13        |         O17 (cascade)
O05 --------+                     |         O18 (thresholds)
O06 --------|----+                |         O19 (effectiveness)
O07         |    |                |         O20 (replan)
O08 --------|    |                |
             |    |    Phase 5 (FSM)
             |    |    O21 -> O22 -> O23
             |    |
Phase 7 (Wave)    |    Phase 8 (Resume)
O28 -> O29 -> O30 |    O32 -> O33
O31               |    O34
                  |    O35
Phase 6 (DAG)    |    O36
O24              |    O37
O25              |
O26              |    Phase 9 (Adaptive)
O27              +--- O38
                      O39 -> O40

Phase 10 (AP)         Phase 11 (Learning)    Phase 12 (Converge)
O41 -> O42            O43                    O47 -> O48 -> O49
                      O44                    O50
                      O45                    O51
                      O46                    O52
```

---

## Priority Summary

| Priority | Tasks | What |
|---|---|---|
| **P0** | O01-O03, O05-O06, O08-O11, O14-O15, O17, O32-O33 | Parallel execution + worktrees + context + learning + resume |
| **P1** | O04, O07, O16, O18, O20-O24, O28-O29, O31, O34-O35, O41-O43, O47-O50 | Wave gating + failure recovery + state convergence + AP checks + cleanup |
| **P2** | O12-O13, O19, O25-O27, O30, O36-O40, O44-O46, O51-O52 | Speculative exec + adaptive parallelism + affect + custody + dreams |

---

## Effort Summary

| Size | Count | Tasks |
|---|---|---|
| S (< 1 day) | 16 | O02, O05, O07, O13, O17, O19, O25, O27, O31, O34, O40, O43, O49, O50, O51, O52 |
| M (1-2 days) | 26 | O01, O03, O04, O08-O12, O14-O16, O18, O21-O24, O26, O28, O30, O32-O33, O35-O38, O42, O44-O46 |
| L (2-3 days) | 4 | O06, O20, O29, O39 |
| **XL** | 6 | O03, O47 (large M/L boundary, complex integration) |

**Total estimated effort**: ~50-65 person-days across all phases.

**Critical path**: Phase 1 (O01-O03) -> Phase 2 (O06) -> Phase 8 (O32) ->
Phase 12 (O47-O49). Minimum ~12-15 days for the critical path.

**Independent tracks** (can run in parallel with critical path):
- Phase 4 (O14-O20): Feature extraction from orchestrate.rs
- Phase 5 (O21-O23): State machine convergence
- Phase 10 (O41-O42): Anti-pattern pre-gate
- Phase 11 (O43-O46): Remaining learning features

---

## Verification Plan

### Per-Phase Smoke Tests

| Phase | Test |
|---|---|
| 1 (Worktree) | 2 tasks with worktree isolation complete and merge without conflict |
| 2 (Parallel) | 4 independent tasks complete in ~1x single-task time |
| 3 (Context) | Task B's prompt includes "What Changed Before You" from task A |
| 4 (Features) | `.roko/episodes.jsonl` has entries after `roko plan run` |
| 5 (FSM) | TUI shows UnifiedPhase names consistently across all entry points |
| 6 (DAG) | High-dependency task dispatched before isolated task |
| 7 (Wave) | 10-task plan: gates run 3 times (not 10) |
| 8 (Resume) | Kill at task 5, resume, tasks 1-5 not re-executed |
| 9 (Adaptive) | 3 failures: parallelism drops. 10 successes: parallelism recovers |
| 10 (AP) | AP-7 violation detected in <100ms before compile gate |
| 11 (Learning) | Efficiency and episode files populated per-run |
| 12 (Converge) | `roko plan run` uses WorkflowEngine. orchestrate.rs deleted. |

### End-to-End Validation

```bash
# Create a plan with 10 tasks, 3 waves, file overlaps
cargo run -p roko-cli -- plan create test-parallel

# Run with parallel execution + wave gating
cargo run -p roko-cli -- plan run plans/test-parallel --parallel 4

# Verify:
# 1. Tasks in wave 0 ran in parallel (check timestamps in episodes.jsonl)
# 2. File-overlapping tasks serialized (no merge conflicts)
# 3. Cumulative context injected (check prompt sections in episodes)
# 4. Gates ran at wave boundaries (3 gate runs, not 10)
# 5. Post-merge regression passed (no compile errors on integration branch)
# 6. Learning files populated (cascade-router.json, gate-thresholds.json)
# 7. Total time < 4x sequential (with 4 parallel slots)

# Resume test
cargo run -p roko-cli -- plan run plans/test-parallel --parallel 4
# Kill at task 5 (Ctrl+C)
cargo run -p roko-cli -- plan run plans/test-parallel --parallel 4 --resume
# Tasks 1-5 not re-executed

# Manual intervention test
echo "complete" > .roko/state/tasks/test-parallel/T7.status
cargo run -p roko-cli -- plan run plans/test-parallel --parallel 4 --resume
# T7 skipped, downstream tasks unblocked
```

---

## Sources

All source files under `/Users/will/dev/nunchi/roko/roko/`. Key references:

| File | LOC | Role |
|---|---|---|
| `crates/roko-runtime/src/workflow_engine.rs` | 1,678 | Target: unified runtime |
| `crates/roko-runtime/src/pipeline_state.rs` | 1,109 | Pure FSM |
| `crates/roko-runtime/src/effect_driver.rs` | 857 | Side-effect executor |
| `crates/roko-runtime/src/task_scheduler.rs` | 378 | DAG scheduler |
| `crates/roko-cli/src/orchestrate.rs` | 22,522 | Legacy monolith (to delete) |
| `crates/roko-cli/src/runner/event_loop.rs` | 3,136 | Runner v2 core loop |
| `crates/roko-orchestrator/src/dag.rs` | 2,557 | DAG + waves + CPM |
| `crates/roko-orchestrator/src/worktree.rs` | 1,203 | Worktree isolation |
| `crates/roko-orchestrator/src/merge_queue.rs` | 921 | File-overlap-aware merge |
| `crates/roko-orchestrator/src/service_factory.rs` | 312 | Service construction |
| `crates/roko-gate/src/gate_service.rs` | 679 | Gate execution |
| `crates/roko-gate/src/adaptive_threshold.rs` | 957 | Adaptive gate skip |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 1,048 | 9-layer prompt builder |
| `crates/roko-core/src/foundation.rs` | 509 | Service traits |
| `crates/roko-learn/src/runtime_feedback.rs` | 5,084 | Learning runtime |
| `crates/roko-learn/src/feedback_service.rs` | 1,156 | Feedback service |
| `crates/roko-orchestrator/src/replan.rs` | 446 | Replan infrastructure |
