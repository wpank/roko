# Orchestration & Plan Execution Audit

3 runtimes, 2 state machines, 1 dead monolith -- the plan execution subsystem
has everything built but spread across incompatible implementations. This audit
documents the exact state of every component, grounded in source code analysis.

## Architecture Runner Status (2026-04-29)

**3 runtimes converging to 1 WorkflowEngine.** Phase 2 of the architecture
runner completed:
- `WorkflowEngine` facade unifies all entry points
  (`crates/roko-runtime/src/workflow_engine.rs`, 1,678 LOC)
- `PipelineStateV2` replaces per-runtime state machines with config-driven pure FSM
  (`crates/roko-runtime/src/pipeline_state.rs`, 1,110 LOC)
- `EffectDriver` executes side effects, keeping decisions in state machine
  (`crates/roko-runtime/src/effect_driver.rs`, 858 LOC)
- `TaskScheduler` handles DAG scheduling within WorkflowEngine
  (`crates/roko-runtime/src/task_scheduler.rs`, 379 LOC)
- CLI wired via `run_with_workflow_engine()`
- ACP wired via `run_with_workflow_engine()` with AcpAdapter
- **Remaining**: proof runs, `orchestrate.rs` retirement (Phase 6), HTTP entry
  point wiring

---

## 1. The Three Runtimes

| Runtime | Files | LOC | Status | Architecture |
|---|---|---|---|---|
| ACP pipeline | `roko-acp/src/{pipeline,runner,workflow}.rs` | ~1,651 | Active (any ACP editor) | Pure state machine + effect driver |
| Runner v2 | `roko-cli/src/runner/` (15 files) | ~9,300 | Active (CLI) | Event-driven tokio `select!` executor |
| Orchestrate.rs | `roko-cli/src/orchestrate.rs` | 22,522 | Legacy | Batch monolith |

### 1.1 ACP Pipeline (Lightweight, Newest)

**State machine:** `PipelinePhase` -- 10 states:
```
Pending -> Strategizing -> Implementing -> Gating -> AutoFixing
  -> Reviewing -> Committing -> Complete/Halted/Cancelled
```

**Actions emitted:** `SpawnStrategist`, `SpawnImplementer`, `SpawnAutoFixer`,
`RunGates`, `SpawnReviewer`, `Commit`, `Done`, `Halt`

**Workflow templates (3 presets + TOML config):**
- Express: implement -> gate -> commit (max_iterations=1, max_autofix=1)
- Standard: + review (max_iterations=2, max_autofix=2)
- Full: + strategist (max_iterations=3, max_autofix=2)

**Source-verified strengths:**
- Pure `step(event) -> action` -- side-effect free state machine
  (PipelineStateV2::step in pipeline_state.rs:598-767)
- Auto-selects template based on prompt complexity
- Iteration loop on gate/review failures with backoff
- Multi-role review for "thorough" mode
- TOML-configurable workflow definitions via `WorkflowConfig::from_toml_str()`
  supporting `[workflow]` tables and `[[workflow.steps]]` arrays
- Checkpoint/resume via `checkpoint()` / `from_checkpoint()` -- JSON
  serialization of full state (phase, iteration, review findings, gate failures)

**Source-verified weaknesses:**
- Serial agents only (one role per phase)
- No DAG execution, no parallel tasks
- No worktree isolation -- single working directory

### 1.2 Runner v2 (Streaming, Active)

**State machine:** `PlanPhase` -- 14 states:
```
Queued -> Enriching -> Implementing -> Gating -> Verifying
  -> Reviewing -> DocRevision -> AutoFixing -> RegeneratingVerify
  -> Merging -> Complete/Done/Failed/Skipped
```

**Key features (source-verified):**
- Line-by-line streaming output parsing (stream-json)
- Task DAG with dependency-driven wave scheduling
- Real-time TUI updates via StateHub
- Per-plan merge queue with conflict detection (via PlanMerger)
- Strict resume via fingerprint validation (TaskDefFingerprint in persist.rs)
- Dream consolidation triggered on plan completion (event_loop.rs:668+)
- Speculative execution infrastructure built in roko-orchestrator but not yet
  wired in event loop

**Concurrency settings (from event_loop.rs):**
```rust
max_concurrent_plans: 4    // default
max_concurrent_tasks: 1    // bottleneck -- serial by task
```

### 1.3 Orchestrate.rs (Batch, Legacy -- 22,522 LOC)

**Same PlanPhase** as Runner v2, but implements 11,000+ lines of features never
ported. From the import block alone (lines 1-210), orchestrate.rs depends on:

| Subsystem | Import Count | What |
|---|---|---|
| roko_agent | 16 items | SafetyLayer, CustodyLogger, MultiAgentPool, Gemini, Perplexity |
| roko_compose | 8 items | AttentionBidder, PromptComposer, SectionScorer |
| roko_conductor | 5 items | Conductor, StuckDetector, DiagnosisEngine |
| roko_core | 30+ items | CFactorSummary, DaimonPolicy, PredictiveScorer |
| roko_daimon | 6 items | AffectEngine, DaimonState, StrategyCoordinates |
| roko_dreams | 3 items | DreamRunner, DreamLoopConfig, DreamAgentConfig |
| roko_gate | 20+ items | GatePipeline, AdaptiveThresholds, RungExecutionConfig |
| roko_learn | 25+ items | EpisodeLogger, PlaybookStore, SkillLibrary, CostRecord |
| roko_neuro | 8 items | KnowledgeStore, ContextAssembler, TierProgression |
| roko_orchestrator | 15+ items | ParallelExecutor, WorktreeManager, MergeQueue |
| roko_runtime | 5 items | ProcessSupervisor, CancelToken, RuntimeEventBus |

Features only in orchestrate.rs:
- SystemPromptBuilder 9-layer enrichment (30+ steps via EnrichmentPipeline)
- DaimonState affect engine integration (load_or_new at line 299)
- Dream consolidation (hypnagogia loops)
- Knowledge routing (build_knowledge_routing_advice)
- VCG auction composition (vcg_allocate)
- C-factor computation and regression detection (CFactorSummary, CFactor)
- Anophily detection + remediation (pre_agent_remediation_log_path)
- Custody audit chain (CustodyLogger)
- Skill extraction + SkillLibrary (SkillExtractionRequest)
- Full learning feedback loop (publish_turn_learning_feedback)
- Predictive calibration (CalibrationTracker, PredictionPolicy)
- Section effectiveness tracking (SectionEffectivenessRegistry)
- Model experiments (ModelExperimentStore)
- Error pattern store (ErrorPatternStore, FailurePatternQuery)
- Routing decision logging (RoutingDecisionLog)
- Latency registry (LatencyRegistry)
- Heartbeat monitoring (HeartbeatClock, HeartbeatSnapshot)

---

## 2. State Machine Comparison

### 2.1 PlanPhase (roko-core, used by Runner v2 + orchestrate.rs)

14 states with typed failure kinds. Transitions enforced by `valid_transitions`
function at `roko-core/src/phase.rs:238-257`.

```
Queued | Enriching | Implementing | Gating | Verifying | Reviewing
| DocRevision | AutoFixing | RegeneratingVerify | Merging
| Complete | Done | Failed { reason: FailureKind } | Skipped
```

**FailureKind enum:** AutoFixExhausted, AllTasksFailed, TaskRetriesExhausted,
SetupFailed, MaxIterations, SpawnFailures, Deadlock, WorktreeMissing,
VacuousImplementation, VerifyScriptBroken, Other(String)

**PlanStateMachine** (roko-orchestrator/src/executor/state_machine.rs):
- Stateless: all mutable state lives in PlanState
- MAX_AUTO_FIX_ITERATIONS = 5
- MAX_MERGE_ATTEMPTS = 3
- Gating -> AutoFixing loops until iteration >= MAX_AUTO_FIX_ITERATIONS
- ReviewRejected -> Implementing (back to implementation)
- MergeFailed with attempts >= MAX_MERGE_ATTEMPTS -> Failed(Deadlock)
- Fatal event from any non-terminal phase -> Failed if valid, else TransitionError

### 2.2 PipelineStateV2 (roko-runtime, used by WorkflowEngine)

10 states -- simpler subset, **not compatible** with PlanPhase:

```
Pending | Strategizing | Implementing | Gating | AutoFixing
| Reviewing | Committing | Complete | Halted { reason } | Cancelled
```

**Key differences from PlanPhase:**
- ACP `Strategizing` has no PlanPhase equivalent
- ACP lacks `Enriching`, `Verifying`, `DocRevision`, `RegeneratingVerify`,
  `Merging`, `Done`
- ACP `Committing` loosely maps to PlanPhase `Merging`
- ACP `Halted` is not the same as PlanPhase `Failed` -- Halted carries a
  reason string, Failed carries a typed FailureKind
- ACP `Cancelled` exists; PlanPhase has no equivalent
- PipelineStateV2 is Serialize/Deserialize; PlanPhase transition enforcement
  is via a separate PlanStateMachine struct

**Convergence problem:** Two state machines for the same concept. Converging
them requires mapping ACP phases into PlanPhase or vice versa. The current
approach is to keep PipelineStateV2 for single-prompt workflows and PlanPhase
for multi-task plan execution.

---

## 3. Plan Format (tasks.toml)

```toml
[meta]
plan = "plan-id"
iteration = 1
total = N
done = M
status = "implementing"
max_parallel = 1

[[task]]
id = "T1"
title = "Implement feature X"
description = "..."
role = "implementer"
tier = "mechanical|focused|integrative|architectural"
model_hint = "claude-opus-4-6"
status = "pending|active|done|blocked"
files = ["src/main.rs"]
depends_on = ["T0"]
depends_on_plan = ["00-foundation"]
allowed_tools = ["bash", "edit"]
denied_tools = ["rm", "git"]
timeout_secs = 600
max_retries = 3
acceptance = ["All tests pass"]

[task.verify]
[task.verify.compile]
required = true
rung = 0

[task.verify.test]
gate = "test"
required = true
rung = 2
```

**Parsing:** `task_parser.rs` -> `TasksFile` -> `Vec<TaskDef>`. Validates
cycles via `detect_cycle_nodes()`. Defaults: status=pending, tier=mechanical,
timeout=600, max_retries=2.

**ACP does not use tasks.toml** -- it works with single prompts, not plan files.
WorkflowEngine supports both modes: single-prompt via PipelineStateV2, and
multi-task via TaskScheduler.

---

## 4. Agent Dispatch (3 Implementations)

| Runtime | Dispatch Path | LOC | Features |
|---|---|---|---|
| ACP | `runner.rs` -> `run_claude_cli()` | ~30 | Fast, minimal, no safety layer |
| Runner v2 | Dispatcher facade -> AgentDispatcherV2 | ~3,040 | Multi-provider, budget, tool translation, warm pool |
| Orchestrate.rs | `dispatch_agent()` + `dispatch_agent_with()` | ~2,142 | Safety, MCP, warm reuse, custody, enrichment |
| WorkflowEngine | EffectDriver -> ModelCaller trait | ~510 | Foundation services, affect modulation |

**What WorkflowEngine/EffectDriver adds (source-verified):**
- AffectPolicy integration: `modulate_dispatch()` adjusts temperature, token
  limits, and cache policy based on behavioral state
- Modulated temperature: BASE=0.2, EXPLORATION_RANGE=0.6, TIER_RANGE=0.1
- Cache policy: exploration_rate >= 0.5 -> bypass cache
- Token budget: turn_limit_factor clamp(0.25, 2.0) * DEFAULT_MAX_OUTPUT=2048
- PromptAssembler trait: role-aware prompt assembly with section IDs
- FeedbackSink trait: structured model call and gate verdict recording
- GateRunner trait: configurable gate execution with ShellGateCommand support

**What is in orchestrate.rs dispatch but NOT in EffectDriver:**
- SafetyLayer with provenance tracking
- MultiAgentPool for warm agent reuse
- Custody audit chain (CustodyLogger)
- Gemini cache client (GeminiCacheClient)
- Perplexity search integration
- DaimonState affect engine (full -- Runner v2 only uses DaimonPolicy::default())
- Knowledge routing (build_knowledge_routing_advice)
- SkillLibrary extraction
- CFactorSummary computation
- Predictive calibration sections
- Section effectiveness tracking
- Model experiment routing
- Error pattern store queries

**Anti-Pattern #7:** Four dispatch implementations with different error handling,
timeout logic, and token counting. Bug fixes only land in one.

---

## 5. DAG Execution (roko-orchestrator/src/dag.rs, 2,557 LOC)

### 5.1 UnifiedTaskDag

The DAG is the scheduling backbone. It supports four kinds of edges:

1. **Intra-plan depends_on**: `t1 -> t2` inside one plan
2. **Cross-plan depends_on**: `"09-foo:t3"` adds an edge from plan 09-foo's t3
3. **Plan-level depends_on**: plan B depends on plan A -> all tasks in A before
   any task in B
4. **File-overlap inference** (opt-in via `DagConfig::infer_file_overlap`): two
   tasks touching the same file get serialized; lexicographically earlier
   GlobalTaskId runs first

```rust
pub struct DagConfig {
    pub infer_file_overlap: bool,   // default: true
    pub max_wave_width: usize,      // 0 = unbounded
}
```

### 5.2 Wave Scheduling

`UnifiedTaskDag::waves()` partitions the DAG via BFS layering:
- Wave 0: tasks with no open dependencies
- Wave 1: depends only on wave 0
- Wave N: depends only on waves 0..N-1
- `max_wave_width`: overflow spills to next wave

Within a wave, tasks sort by GlobalTaskId for deterministic output.

### 5.3 Critical Path Method (CPM)

The DAG computes CPM analysis:
- `earliest_start(task)` -- earliest time a task can begin
- `latest_start(task)` -- latest start without extending the plan
- `slack(task)` -- difference between latest and earliest start
- `critical_path()` -- zero-slack tasks that determine total duration

```rust
pub struct DagStats {
    pub nodes: usize,
    pub edges: usize,
    pub waves: usize,
    pub critical_path_minutes: u32,
}
```

### 5.4 Live DAG Mutations

The DAG supports mid-execution mutations via `DagMutation` enum:

- `AddTask { task_id, task, depends_on }` -- insert a new task
- `RemoveTask { task_id }` -- remove and reconnect dependents
- `SplitTask { task_id, into }` -- replace one task with a serial chain
- `AddDependency { from, to }` -- add an edge
- `UpdateTaskMetadata { task_id, task }` -- replace a task spec

Mutations are validated: they reject cycles, completed tasks, and structural
errors. Applied via clone-and-validate pattern (clone the DAG, apply, check
for cycles, swap on success).

### 5.5 Chain Fusion

`fuse_linear_chains(config)` collapses eligible linear chains in place:
- Chains where A->B->C with single-dependent, single-dependency nodes
- Configurable: `max_chain_length`, `same_tier_only`, `ave_width` threshold
- Guards against reducing average parallelism below threshold
- Skips chains containing completed tasks
- Returns the number of fusions performed

### 5.6 Target Culling

`cull(targets)` removes tasks not required to produce given target task IDs:
- Backward BFS from targets collects all transitive dependencies
- Everything NOT in that set is removed
- Supports both qualified ("plan:task") and bare ("task") target references

### 5.7 Execution Snapshot

`DagExecutionSnapshot` provides a serializable runtime view:

```rust
pub struct DagExecutionSnapshot {
    pub schema_version: u32,
    pub tasks: BTreeMap<String, DagTaskExecutionMetadata>,
    pub topological_order: Vec<GlobalTaskId>,
    pub waves: Vec<ExecutionWave>,
    pub timestamp_ms: u64,
}
```

`DagTaskExecutionStatus` tracks per-task runtime state:
`Pending | Ready | Running | Gating | Passed | Retrying { attempt, backoff_until_ms } | Exhausted { attempts, last_error } | Skipped`

`ready_tasks()` returns tasks that are dispatchable and whose deps all passed.

### 5.8 Speculative Execution

`SpeculativeExecution` struct and `speculative_threshold_multiplier` config
exist in `roko-orchestrator/src/executor/mod.rs` and are persisted in
snapshots, but **no code in the Runner v2 event loop actually triggers
speculative spawns**. Built but not wired.

### 5.9 The Bottleneck

`max_concurrent_tasks: 1` means despite the full DAG and wave infrastructure,
only one task runs at a time by default. This is the single most impactful
configuration change available.

---

## 6. Persistence & Resume

### 6.1 ExecutorSnapshot (.roko/state/executor.json)

```rust
ExecutorSnapshot {
    schema_version: u32,                              // current = 1
    plan_states: HashMap<String, PlanState>,
    queue_order: Vec<String>,
    conductor_circuit_breaker: Option<PersistedCircuitBreakerState>,
    speculative_executions: HashMap<String, SpeculativeExecution>,
    timestamp_ms: u64,
}
```

### 6.2 PipelineStateV2 Checkpoint

The WorkflowEngine uses JSON-serialized PipelineStateV2:
```rust
pub fn checkpoint(&self) -> Result<String> {
    Ok(serde_json::to_string(self)?)
}
pub fn from_checkpoint(json: &str) -> Result<Self> {
    Ok(serde_json::from_str(json)?)
}
```

Preserves: phase, config, iteration, autofix_attempts, original_prompt,
strategist_brief, review_findings, last_gate_failure, files_changed,
commit_hash.

EffectDriver::save_checkpoint writes atomically via tmp + rename pattern.

### 6.3 Resume by Runtime

| Runtime | Resume Support | Mechanism | Strictness |
|---|---|---|---|
| ACP | Partial (via PipelineStateV2) | JSON checkpoint | Phase + iteration |
| Runner v2 | Full | Fingerprint validation + JSONL recovery | Strict -- hash-validates every task definition |
| Orchestrate.rs | Full | ExecutorSnapshot load | Legacy -- allows task edits between runs |
| WorkflowEngine | Partial | PipelineStateV2::from_checkpoint | Restores phase, findings, counters |

---

## 7. Process Supervision (roko-runtime/src/process.rs, 1,354 LOC)

### 7.1 ProcessSupervisor

Manages a pool of `ProcessHandle`s with bulk operations:

```rust
pub struct SpawnConfig {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub grace_period: Duration,           // default: 5s
    pub cancellation: Option<CancelToken>,
    pub session: Option<ProcessSessionConfig>,
    pub label: String,
}
```

### 7.2 Supervision Strategies (Erlang/OTP-style)

Three strategies directly inspired by Erlang supervisors:

```rust
pub enum SupervisionStrategy {
    OneForOne { max_restarts, within_ms, fallback_tier },
    OneForAll { max_restarts },
    RestForOne { max_restarts },
}
```

- **OneForOne**: restart only the failed process (with sliding window rate limit)
- **OneForAll**: restart every managed process when one fails
- **RestForOne**: restart the failed process and those started after it

### 7.3 Process Session Config

Durable metadata for resume and interruption diagnosis:

```rust
pub struct ProcessSessionConfig {
    pub session_id: String,
    pub invocation_id: String,
    pub backend_id: String,
    pub task_id: Option<String>,
    pub reuse_policy_id: Option<String>,
    pub resumable: bool,
    pub timeout_ms: Option<u64>,
}
```

Session ledger persisted at `.roko/state/process-sessions.json`.

---

## 8. WorkflowEngine Architecture (roko-runtime/src/workflow_engine.rs, 1,678 LOC)

The WorkflowEngine ties together PipelineStateV2 (decisions) and EffectDriver
(effects) into a run loop. It is the shared entry point for CLI, ACP, and HTTP.

### 8.1 Configuration

```rust
pub struct WorkflowRunConfig {
    pub prompt: String,
    pub workdir: PathBuf,
    pub workflow: WorkflowConfig,
    pub enabled_gates: Vec<String>,
    pub shell_gates: Vec<ShellGateCommand>,
    pub commit_prefix: Option<String>,
}
```

### 8.2 Effect Services

The EffectDriver requires five services:

```rust
pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
}
```

### 8.3 Affect Modulation

The EffectDriver applies affect modulation to every model call:

- `pre_dispatch()` gets AffectContext (behavioral state, PAD values, emotional tag)
- `modulate_dispatch()` adjusts turn_limit_factor, exploration_rate, tier_bias
- turn_limit_factor <= 0.0 -> agent dispatch deferred entirely
- Temperature: 0.2 + (exploration * 0.6) + (tier_bias.max(0) * 0.1)
- Cache: exploration >= 0.5 -> bypass cache
- Token budget: factor * 2048, clamped to [0.25, 2.0] range

### 8.4 Gate Rung Mapping

EffectDriver maps gate names to rungs (duplicated from roko-gate):
- Rung 0: compile, compile:cargo
- Rung 1: clippy, clippy:cargo
- Rung 2: test, test:cargo
- Rung 3: diff, diff:git
- Rung 4: fmt, fmt:cargo, format
- Rung 5: custom, custom:shell, shell
- Rung 6: judge, llm-judge

Deterministic gates (rung 0-4) get confidence=1.0; heuristic gates (rung 5+)
get confidence=0.5.

---

## 9. TaskScheduler (roko-runtime/src/task_scheduler.rs, 379 LOC)

Pure DAG dependency resolver for multi-task plan execution within WorkflowEngine.

### 9.1 Scheduling Logic

```rust
pub struct TaskScheduler {
    tasks: HashMap<String, SchedulableTask>,
    status: HashMap<String, TaskStatus>,
    max_parallel: usize,
}
```

`next_batch()` respects three constraints:
1. **Dependency satisfaction**: all depends_on must be Completed
2. **File exclusion**: tasks touching same files as running tasks are deferred
3. **Parallelism cap**: batch.len() <= max_parallel - running_count

### 9.2 Failure Propagation

`mark_failed()` cascades skips to all transitive dependents via BFS. A single
failed task can skip an entire dependency subtree.

---

## 10. Features Only in Dead Code

| Feature | Orchestrate.rs Location | Runner v2 | WorkflowEngine | ACP |
|---|---|---|---|---|
| Dream consolidation | Line 7589+ | Present | Missing | Missing |
| Daimon affect engine (full) | Line 266+ | Partial | Partial (AffectPolicy trait) | Missing |
| Knowledge routing | build_knowledge_routing_advice() | Missing | Missing | Missing |
| VCG auction | vcg_allocate() | Missing | Missing | Missing |
| Custody audit chain | CustodyLogger | Missing | Missing | Missing |
| Skill extraction | SkillLibrary::extract() | Missing | Missing | Missing |
| Anophily remediation | pre_agent_remediation_log_path() | Missing | Missing | Missing |
| C-factor computation | CFactorSummary | Missing | Missing | Missing |
| 30+ enrichment steps | estimate_enrichment | Partial | Partial (PromptAssembler) | None |
| Predictive calibration | CalibrationTracker | Missing | Missing | Missing |
| Section effectiveness | SectionEffectivenessRegistry | Missing | Missing | Missing |
| Error pattern queries | ErrorPatternStore | Missing | Missing | Missing |
| Model experiments | ModelExperimentStore | Missing | Missing | Missing |
| Heartbeat monitoring | HeartbeatClock | Missing | Missing | Missing |
| Routing decision log | RoutingDecisionLog | Missing | Missing | Missing |

---

## 11. Merge & Worktree Orchestration

### 11.1 Worktree Manager (roko-orchestrator/src/worktree.rs, 1,203 LOC)

Per-plan isolation:
- Each plan gets its own git worktree (isolated branch)
- `WorktreeManager::create_for_plan(plan_id)` -> branch from configured base
- Idle TTL cleanup (configurable; test fixtures use 3600s)
- Health checks verify branch exists and is reachable
- Budget enforcement via `max_live: Option<usize>` cap
- Branch naming via `format_branch_name()`

### 11.2 Merge Queue (roko-orchestrator/src/merge_queue.rs, 924 LOC)

File-conflict-aware serialized merging:
- `MergeQueue` tracks concurrent merge requests
- File overlap detection prevents parallel merges touching same files
- `MergeQueueSnapshot` for persistence
- `MergeQueueMetrics` for monitoring
- `DEFAULT_MAX_MERGE_RETRIES` constant for retry policy

### 11.3 Post-Merge Validation

`PostMergeRunner` runs regression gates after each merge:
- Validates the integration branch did not regress
- Reports failure back to the executor state machine
- `PostMergeCheck` + `PostMergeResult` types for structured feedback

---

## 12. Anti-Patterns

| Anti-Pattern | Where | Severity |
|---|---|---|
| **#10 God file** | orchestrate.rs is 22,522 lines -- features that belong in 6+ crates | Critical |
| **#7 Copy between runtimes** | 4 dispatch implementations, 3 gate dispatch paths, 2 state machines | High |
| **#3 Build another runtime** | Each runtime reimplements plan execution instead of sharing | High |
| **#4 Features in wrong layer** | Enrichment, custody, skill extraction all inline in orchestrate.rs | Medium |
| State machine mismatch | PipelineStateV2 (10 states) vs PlanPhase (14 states) -- not interoperable | Medium |
| Bottleneck default | max_concurrent_tasks: 1 despite full DAG infrastructure | High |
| Gate rung duplication | rung_for_gate_name in EffectDriver duplicates GateService mapping | Low |
| Affect policy gap | EffectDriver supports AffectPolicy but Runner v2 only uses default | Medium |

---

## 13. Entry Point Summary

| Command | Primary File | State Machine | Concurrency | Resume |
|---|---|---|---|---|
| `roko run <prompt>` | run.rs | None (linear) | Serial | None |
| `roko plan run <dir>` v2 | runner/event_loop.rs | PlanPhase + PlanStateMachine | DAG waves | Strict fingerprints |
| `roko plan run <dir>` legacy | orchestrate.rs | PlanPhase + Executor | Wave iteration | Legacy |
| ACP `/workflow` | acp/runner.rs | PipelinePhase | Serial agents | None |
| WorkflowEngine | workflow_engine.rs | PipelineStateV2 | TaskScheduler DAG | JSON checkpoint |

---

## 14. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-cli/src/orchestrate.rs` | 22,522 | Dead monolith |
| `roko-runtime/src/workflow_engine.rs` | 1,678 | Active -- unified facade |
| `roko-runtime/src/pipeline_state.rs` | 1,110 | Active -- pure FSM |
| `roko-runtime/src/effect_driver.rs` | 858 | Active -- side-effect executor |
| `roko-runtime/src/task_scheduler.rs` | 379 | Active -- DAG scheduler |
| `roko-runtime/src/process.rs` | 1,354 | Active -- process supervision |
| `roko-cli/src/runner/event_loop.rs` | 3,035 | Active -- Runner v2 core loop |
| `roko-cli/src/runner/types.rs` | 1,560 | Active -- event/type protocol |
| `roko-cli/src/runner/merge.rs` | 776 | Active -- merge queue integration |
| `roko-cli/src/runner/resume.rs` | 406 | Active -- fingerprint resume |
| `roko-cli/src/runner/state.rs` | 511 | Active -- mutable run state |
| `roko-cli/src/runner/task_dag.rs` | 554 | Active -- per-plan DAG |
| `roko-cli/src/runner/projection.rs` | 554 | Active -- event projection |
| `roko-cli/src/runner/persist.rs` | 475 | Active -- atomic snapshots |
| `roko-cli/src/runner/gate_dispatch.rs` | 323 | Active -- gate routing |
| `roko-cli/src/runner/agent_stream.rs` | 347 | Active -- streaming output parser |
| `roko-cli/src/run.rs` | 1,555 | Active -- oneshot universal loop |
| `roko-acp/src/pipeline.rs` | 539 | Active -- ACP state machine |
| `roko-acp/src/runner.rs` | 969 | Active -- ACP executor |
| `roko-acp/src/workflow.rs` | 143 | Active -- workflow run wrapper |
| `roko-acp/src/session.rs` | 1,539 | Active -- ACP session management |
| `roko-acp/src/bridge_events.rs` | 1,855 | Active -- ACP event bridge |
| `roko-orchestrator/src/executor/state_machine.rs` | ~630 | Active -- PlanStateMachine |
| `roko-orchestrator/src/dag.rs` | 2,557 | Active -- DAG + wave scheduler |
| `roko-orchestrator/src/worktree.rs` | 1,203 | Active -- worktree isolation |
| `roko-orchestrator/src/merge_queue.rs` | 924 | Active -- file-overlap-aware merge queue |
| `roko-orchestrator/src/replan.rs` | varies | Active -- replanning on failure |
| `roko-cli/src/dispatch_v2.rs` | 946 | Active -- provider resolver |
| `roko-cli/src/dispatch/mod.rs` | 405 | Active -- Dispatcher facade |
| `roko-cli/src/dispatch/prompt_builder.rs` | 993 | Active -- prompt assembly |
| `roko-cli/src/task_parser.rs` | ~400 | Active -- TOML parsing |
| `roko-core/src/phase.rs` | ~397 | Core -- PlanPhase enum + transitions |

---

## Sources

All source files read for this audit are under
`/Users/will/dev/nunchi/roko/roko/`. Key files:

- `crates/roko-runtime/src/pipeline_state.rs` -- PipelineStateV2 (10 phases, TOML config, checkpoint/resume)
- `crates/roko-runtime/src/effect_driver.rs` -- EffectDriver (affect modulation, gate rung mapping, model calls)
- `crates/roko-runtime/src/workflow_engine.rs` -- WorkflowEngine facade
- `crates/roko-runtime/src/task_scheduler.rs` -- TaskScheduler (DAG resolver, file exclusion, failure cascade)
- `crates/roko-runtime/src/process.rs` -- ProcessSupervisor (Erlang strategies, SpawnConfig, sessions)
- `crates/roko-runtime/src/lib.rs` -- Module structure and re-exports
- `crates/roko-orchestrator/src/dag.rs` -- UnifiedTaskDag (waves, CPM, mutations, fusion, culling)
- `crates/roko-orchestrator/src/executor/state_machine.rs` -- PlanStateMachine (14-phase transitions)
- `crates/roko-orchestrator/src/lib.rs` -- Orchestrator module exports
- `crates/roko-cli/src/orchestrate.rs` -- Legacy monolith (22,522 LOC, import analysis)
- `crates/roko-core/src/phase.rs` -- PlanPhase enum, FailureKind, valid_transitions
