# Execution Model Comparison: Mori vs Roko

Deep comparison of how the two systems handle plan execution, covering
queue management, dependency ordering, agent dispatch, gate pipelines,
resume/checkpoint, and operator UX.

---

## 1. Architecture at a Glance

| Dimension | Mori | Roko (runner-v2) |
|---|---|---|
| **Primary module** | `apps/mori/src/orchestrator/` (14+ files) + `app/parallel.rs` (~26K lines) | `crates/roko-cli/src/runner/` (27 files) + `crates/roko-cli/src/orchestrator/executor/` (8 files) |
| **Event loop** | `app/parallel.rs` async loop with `tokio::select!` | `runner/event_loop.rs` async loop with `tokio::select!` |
| **Scheduler** | `ParallelExecutor` in `orchestrator/executor.rs` (pure state machine) | `ParallelExecutor` in `orchestrator/executor/mod.rs` (pure state machine) + `TaskDag` in `runner/task_dag.rs` |
| **State machine granularity** | Cross-plan unified task DAG (`UnifiedTaskDag` + `PlanDag`) | Per-plan task DAG (`TaskDag` wrapping per-plan `PlanDag`) |
| **Queue manifest** | `.mori/queue.toml` with milestones, maintenance batches, run presets | No queue manifest; plans discovered from directory, ordered by DAG |
| **Agent backends** | 3 backends: Codex, Cursor, Claude | 11 provider kinds: AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw |
| **Pipeline phase count** | 26 phases in `PipelinePhase` enum | Plan phases from `PlanPhase` enum in roko-core; task DAG handles Ready/Active/Blocked/Terminal per task |
| **Gate types** | 11 gate types (Compile, Clippy, Test, Invariant, Terminal, Spec, Integration, FullLoop, Coverage, IgnoredTest, DependencyDeny) | 19 gates across 7 rungs (compile, test, clippy, diff, and oracle rungs 4-6) |

---

## 2. Queue System

### Mori: `.mori/queue.toml`

Mori has a first-class queue manifest at `.mori/queue.toml`. This is the
single file an operator edits to control what runs, in what order, and
with what settings.

**Structure:**

```toml
[run]
mode = "express"
max_agents = 20
max_parallel_plans = 6
preset = "balanced"
fast_task_model = "gpt-4o-mini"
standard_task_model = "claude-sonnet-4-20250514"
complex_task_model = "claude-opus-4-20250514"
disabled_providers = ["codex"]

[[milestone]]
name = "Minimal MVP"
description = "Core types, chain RPC, safety, inference."
tags = ["mvp", "core"]
plans = ["02", "04", "05", "06", "09"]

[milestone.maintenance]
after_batch = [
  { plans = ["02", "04", "05"], refactor = "R01", qa = "Q01", docs = "W01" },
  { plans = ["06", "09"], refactor = "R02", integration = "X01" },
]

[[milestone]]
name = "Demo Story"
description = "Terminal, trading, dreams."
tags = ["demo"]
plans = ["07", "07a", "08"]
```

**Key features of Mori's queue:**

1. **Named milestones** group plans into logical batches with descriptions
   and tags. The executor completes all plans in a milestone (including
   maintenance) before advancing to the next.

2. **Milestone progression** is automatic: `current_milestone_specs()`
   finds the first milestone with incomplete plans and returns its specs.

3. **Maintenance batches** let you declare refactor/QA/docs/integration/
   audit plans that run after their associated implementation plans
   complete. The `maintenance_dependency_map()` method injects these
   dependencies into the DAG automatically.

4. **Run settings** (`[run]`) configure the entire execution session:
   mode, agent limits, model routing per complexity band, provider
   preferences, optimization profile, context strategy, and per-plan
   routing overrides.

5. **Validation** catches duplicate plan specs and dependency inversions
   before execution starts (`validate_queue_config()`).

6. **Persistence** via `save_queue_toml()` with temp-file + rename for
   crash safety.

### Roko: No queue manifest

Roko discovers plans by scanning a directory and building a DAG from
`tasks.toml` frontmatter `depends_on` fields. There is no equivalent of
milestones, maintenance batches, or a persistent queue manifest.

**What exists:**

- `roko plan run plans/ --engine runner-v2` scans a directory
- `plan_loader::load_plans()` finds directories with `tasks.toml`
- `TaskDag` handles per-plan dependency ordering via `depends_on` and
  `depends_on_plan` fields in task definitions
- Plans are processed in whatever order the DAG permits
- Run-level config comes from `roko.toml` `[runner]`, not a per-run queue

**What is missing:**

- No milestone grouping or milestone-based progression
- No maintenance plan concept (refactor/QA/docs after a batch)
- No per-run session config file (mode, model overrides, limits)
- No queue validation before execution
- No ability to define "run these 5 plans in milestone X, then those 3
  in milestone Y"

---

## 3. Wave System (Dependency Ordering)

### Mori: Plan-level waves via Kahn's algorithm

Mori's `PlanDag` (`orchestrator/dag.rs`) builds a plan-level dependency
graph from:

1. **Frontmatter `depends_on`** fields in plan markdown files
2. **Cross-plan task references** from `tasks.toml` (`"09:T3"` format)
3. **Queue maintenance dependencies** injected from queue config
4. **Sequential fallback** for plans without frontmatter

The DAG then computes **execution waves** using Kahn's algorithm:

```
Wave 0: [01-a]                    # No deps, runs first
Wave 1: [02-b, 03-c]             # Depend on 01-a, run in parallel
Wave 2: [04-d]                    # Depends on both 02-b and 03-c
```

**Additional wave capabilities:**

- `compute_waves()` groups plans into levels by dependency depth
- `critical_path()` finds the longest weighted path through the DAG
- `split_wave()` respects `max_parallel` limits within waves
- `file_overlap_analysis()` detects crate-level conflicts between
  parallel plans
- `estimated_total_minutes()` sums wave estimates for ETA
- `can_start()` checks if a specific plan can begin given completed set

### Roko: Task-level DAG within plans

Roko's `TaskDag` (`runner/task_dag.rs`) operates at a different level.
It manages per-plan task ordering rather than cross-plan waves:

- `next_ready_task()` finds the next task whose `depends_on` are all
  completed within the plan and whose `depends_on_plan` plans are done
- `ready_tasks()` returns all ready tasks for potential parallel dispatch
- `progress_summary()` classifies tasks as Ready/Active/Blocked/Terminal

The cross-plan orchestrator (`orchestrator/executor/`) also has a
`ParallelExecutor` with `ExecutorConfig.max_concurrent_plans` and
`max_concurrent_tasks`, but it does not compute waves or group plans
into dependency levels.

**What exists in Roko:**

- Per-plan task DAG with `depends_on` (intra-plan) and `depends_on_plan`
  (cross-plan) resolution
- `DagConfig` with timeout and retry parameters
- `PlanDag` per-plan state tracking running/completed/failed/skipped
- Retry scheduling with exponential backoff
- Task-level blocking reasons with descriptive messages

**What is missing:**

- No plan-level wave computation (Kahn's algorithm across plans)
- No critical path analysis
- No file/crate overlap detection between parallel plans
- No wave splitting for parallelism limits
- No estimated total duration from wave sum

---

## 4. Parallel Execution

### Mori: Bounded agent pool with sophisticated scheduling

Mori's `ParallelExecutor` (`executor.rs`) manages a unified cross-plan
task-level DAG with these features:

- **Agent budget**: `max_concurrent_agents` (default 15) enforced across
  ALL agent types (implementers, reviewers, scribes, auto-fixers).
  `total_active_agents` is set by the event loop before `schedule_next()`
  and includes non-implementer agents.

- **Plan slot limit**: `max_parallel_plans` caps how many plans can be
  in the Implementing phase simultaneously. New plans cannot start
  if active slots are full.

- **Budget reservation**: `budget_reservations` set tracks plans that
  have a pipeline but no agent yet, preventing over-scheduling.

- **Task batching**: Multiple tasks from the same plan are grouped into
  a single `SpawnTaskAgentBatch` action because plan worktrees are shared.
  "Collapsing independent task groups into a single implementer because
  plan worktrees are shared."

- **Spawn backoff**: Exponential backoff on spawn failures (2s, 4s, 30s)
  with per-plan `spawn_blocked_until` timestamps. After 10 consecutive
  failures, the plan is failed with `FailureKind::SpawnFailures`.

- **Task-level backoff**: Individual tasks have `task_blocked_until`
  timestamps after failures.

- **Zombie reaping**: 4-hour max age for in-flight tasks
  (`reap_zombie_agents()`).

- **Wall-clock limit**: 45-minute default per plan, triggering
  `PlanTimeout` action.

- **Utility agents**: Non-task agents (pre-planners, refactorers) tracked
  separately but counted against the budget.

### Roko: Simpler concurrent task model

Roko's runner-v2 has simpler parallelism:

- **`max_concurrent_tasks`** from `RunConfig` (default from
  `DEFAULT_RUNNER_MAX_CONCURRENT_TASKS` in roko-core defaults).
  This limits how many tasks can run at once across all plans.

- **`max_concurrent_plans`** in `ExecutorConfig` (default 4) limits
  concurrent plan execution.

- **Per-task agent dispatch**: Each task gets its own agent process
  (`AgentHandle`). The runner dispatches one task at a time per plan
  in the event loop, waiting for completion before dispatching the next
  ready task.

- **Task timeout**: `agent_dispatch_timeout()` derived from
  `TimeoutConfig` in `roko.toml`. Configurable per-role and per-phase.

- **Retry**: `DagConfig` with exponential backoff
  (`DEFAULT_PLAN_RETRY_BASE_SECS`, `DEFAULT_PLAN_RETRY_MAX_SECS`),
  shift cap, and plan-level `DEFAULT_PLAN_TIMEOUT_SECS`.

**What is missing compared to Mori:**

- No agent-pool-level budget tracking across all roles
- No spawn backoff with per-plan failure counting
- No zombie agent reaping
- No budget reservation for pre-pipeline plans
- No task batching (multiple tasks to one agent instance)

---

## 5. Plan Pipeline (Per-Plan State Machine)

### Mori: 26-phase pipeline with express mode

Mori's `PlanPipeline` (`pipeline.rs`) is a full state machine per plan:

```
Preflight -> [Strategist] -> Implementer -> CompileGate
  -> DependencyDenyCheck -> TestGate -> IgnoredTestCheck
  -> SpecComplianceCheck -> [FullLoopTest] -> [Reviewing]
  -> [Verdict] -> [DocRevision] -> Committing -> Complete
```

**Key pipeline features:**

1. **Complexity-driven configuration** (`complexity.rs`):
   - Trivial: skip strategist + reviews, max 1 iteration
   - Simple: skip strategist + reviews, max 2 iterations
   - Standard: skip strategist, quick review, max 2 iterations
   - Complex: skip strategist, full review panel + critic, max 2 iterations

2. **Express mode**: Skip strategist and all reviews. On gate failure,
   use auto-fix instead of re-implementing. Max 3 auto-fix attempts.

3. **Gate failure routing**: Compile failures with simple rustc
   suggestions route to `AutoFix` (direct apply); complex failures route
   back to `Implementer`. Test failures with only warnings also route
   to `AutoFix`.

4. **Review cap**: After `max_iterations` consecutive revise cycles, emit
   `ReviewCapHit` and force-commit. Structured reviews with TOML parsing
   for smarter routing (approve/revise/skip verdicts).

5. **Iteration memory** (`iteration_memory.rs`): Previous review feedback
   is compacted and persisted for crash recovery, injected into
   re-implementation prompts.

6. **Phase timeouts**: Configurable per-phase via `ConductorConfig`.

### Roko: Plan-phase state machine in orchestrator + runner event loop

Roko's approach splits the state machine between two layers:

1. **`PlanPhase`** from roko-core: the high-level plan lifecycle
   (Pending, Implementing, Gating, Reviewing, Merging, Complete, Failed)

2. **Runner event loop** (`event_loop.rs`): The actual dispatch logic
   that sequences task DAG resolution, agent dispatch, gate execution,
   and result handling.

3. **`PlanStateMachine`** in `orchestrator/executor/state_machine.rs`:
   Phase transition rules with typed `TransitionError`.

**Key differences:**

- Roko has no complexity-based pipeline adaptation (every plan gets the
  same pipeline)
- No express mode (skip reviews, auto-fix on failure)
- No structured review routing (approve/revise/skip)
- No review cap with force-commit
- No iteration memory compaction for crash recovery
- Roko has richer gate pipeline (19 gates, 7 rungs) vs Mori's 11 gates

---

## 6. Agent Roles and Dispatch

### Mori: 22+ specialized roles

Mori defines extensive agent roles in `agent/roles.rs`:

```rust
Conductor, Strategist, Implementer, Architect, Auditor,
Scribe, Critic, Refactorer, PrePlanner, DocVerifier,
IntegrationTester, MergeResolver, TerminalValidator,
GolemLifecycleTester, SpecDriftDetector, RegressionDetector,
PerformanceSentinel, CoverageTracker, PlanLifecycleManager,
CrossSystemTester, ErrorDiagnoser, Researcher,
DependencyValidator, QuickReviewer, SnapshotComparator,
PatternExtractor
```

**Dispatch pattern:**

- **Conductor**: Supervises all agents. Monitors silence, compile
  failures, context pressure, phase timeouts. Intervenes with nudge/
  restart/abort/force-advance actions. Has watchers for stall detection,
  pattern matching, and LLM-based reasoning.

- **One implementer per plan**: Stays warm across phases so gate-failure
  fixes reuse context. `primary_agent_instance` tracked in `PlanState`.

- **Gate-review overlap**: Pre-spawn a warm reviewer alongside gate
  execution. If gates pass, the reviewer continues. If gates fail,
  cancel the reviewer (`CancelActiveReviewer`).

- **Batch spawn**: `SpawnTaskAgentBatch` groups all tasks for a plan
  into one agent instance, because worktrees are shared.

- **Routing by complexity band**: Task-level heuristic routing
  (`heuristic_routing_band_for_task()`) considers file count, estimate,
  cross-plan deps, critical surfaces, category, quality profile,
  reasoning level, and speed priority.

### Roko: Roles from roko-core, dispatched via runner

Roko's agent roles come from `roko_core::AgentRole`:

```rust
Implementer, Auditor, QuickReviewer, Critic, Strategist,
Architect, Conductor, Scribe, Refactorer, PrePlanner,
DocVerifier, SpecDriftDetector, RegressionDetector,
SnapshotComparator, PlanLifecycleManager, ...
```

**Dispatch pattern:**

- Agent dispatch via `AgentDispatchRequest` through a `SharedAgentFactory`
  that resolves the provider, model, and runtime.

- **CascadeRouter** for model selection based on learned outcomes.

- **Per-role context scoping** (`ContextScopingConfig`): Implementers
  get focused context (more error patterns, fewer episodes). Reviewers
  get broader episode recall. Strategists get plan-level only.

- **PromptCache** and `PromptExperimentContext` for A/B testing of
  prompt sections.

- **Daimon integration**: Affect engine modulates dispatch parameters.

**What Roko adds vs Mori:**

- Learned model routing (CascadeRouter + cascade persistence)
- Per-role context scoping
- Prompt experiments (A/B testing)
- Daimon affect modulation
- 11 provider backends vs Mori's 3
- Safety layer (trust-origin IFC, capability wrappers, immune graph)

**What Roko lacks vs Mori:**

- No conductor agent pattern (supervisor that monitors/intervenes)
- No gate-review overlap (pre-spawn warm reviewer)
- No warm agent reuse across phases
- No task batching to single agent
- No complexity-driven routing heuristic with 6 classification phases

---

## 7. Gate Pipeline

### Mori: Linear gate chain with auto-fix bypass

```
Implementer -> CompileGate -> DependencyDeny -> TestGate
  -> IgnoredTestCheck -> SpecCompliance -> [FullLoopTest]
```

- Simple compile errors with rustc suggestions -> auto-fix
- Warning-only test failures -> auto-fix
- Spec compliance can be non-blocking (SPEC_ISSUE)
- 3 failure threshold per gate before halting
- Express mode: auto-fix on all gate failures, no reviews

### Roko: 7-rung adaptive gate pipeline

```
Rung 0: compile (cargo check / cargo build)
Rung 1: test (cargo test)
Rung 2: clippy (cargo clippy -- -D warnings)
Rung 3: diff (structural change validation)
Rungs 4-6: oracle gates (higher-level validation)
```

- **Adaptive thresholds**: EMA per rung in
  `.roko/learn/gate-thresholds.json` with configurable flush cadence.
- **19 gate types** in roko-gate
- **Enriched rung inputs** with plan context
- **Gate failure classification** for learning
- **Gate failure replan**: `build_gate_failure_plan_revision`
  triggered by `learning_config.replan_on_gate_failure`
- **Error pattern store**: `GateFailureObservation` fed into
  `ErrorPatternStore` for future prompt enrichment
- **Post-gate reflection**: `PostGateReflectionStore` with
  promotion config for knowledge tier progression

**Roko's gate pipeline is more sophisticated in infrastructure** (adaptive
thresholds, learning integration, error pattern storage) but **Mori's
pipeline is more sophisticated in routing** (auto-fix bypass, express
mode, complexity-driven gate selection).

---

## 8. Resume / Checkpoint System

### Mori: `ExecutorSnapshot` with task-level granularity

Mori persists an `ExecutorSnapshot` containing:

- `completed_tasks: Vec<String>` (GlobalTaskId strings)
- `in_flight_tasks: HashMap<String, String>` (task -> instance)
- `completed_plans: Vec<String>`
- `plan_phases: HashMap<String, PlanPhase>`
- `plan_iterations: HashMap<String, u32>`
- `merge_queue: Vec<String>`
- `plans_since_refactor / plans_since_integration_test`
- `review_feedback: HashMap<String, Vec<String>>`
- `task_failure_counts: HashMap<String, u32>`
- `skipped_tasks: Vec<String>`
- `verify_error_signatures` and `consecutive_verify_fails` per plan

Resume works by:
1. Loading the snapshot
2. Filtering completed tasks/plans for current batch
3. Restoring `ParallelExecutor` state
4. Auto-retrying recoverable failed plans after per-kind cooldowns
5. Mid-phase recovery for plans stuck in Gating/Reviewing/AutoFixing

### Roko: Strict resume with fingerprint validation

Roko's resume (`runner/resume.rs`) adds safety on top:

- **`TaskDefFingerprint`** hashing for every task definition
- **Drift detection**: If a task's content changed since it was marked
  complete, it is flagged as `DriftedTask` for re-queuing
- **Schema versioning**: `RUN_STATE_SCHEMA_VERSION` check
- **JSONL recovery**: `episodes.jsonl`, `events.jsonl`,
  `efficiency.jsonl` are truncated after their last valid line
- **Cascade router restoration**: CascadeRouter JSON from prior snapshot
- **Conductor circuit breaker restoration**: Persisted circuit breaker
  state from prior run

Roko's resume is **stricter and safer** (fingerprint validation catches
stale completions) but **Mori's is more complete** (task-level
granularity, review feedback, verify signature persistence, auto-retry
after cooldown).

---

## 9. Express Mode

### Mori: Full express mode implementation

Express mode is a first-class execution mode in Mori:

- **No strategist**: Skip the strategist phase entirely
- **No reviews**: Skip architect/auditor/scribe/critic reviews
- **Auto-fix on gate failure**: Instead of re-running the implementer,
  spawn a lightweight auto-fixer agent
- **Max 3 auto-fix attempts** before failing the plan
- **Batch rebase**: On auto-fix exhaustion, attempt to rebase from the
  batch branch (pick up code from other merged plans) before failing
- **Configured via `[run] mode = "express"` in queue.toml**

### Roko: No express mode

Roko has no express mode concept. Every plan goes through the same
pipeline. The closest equivalent is configuring `max_retries` low and
skipping review agents via role config, but this is not a named mode
and lacks the auto-fix-on-failure routing.

---

## 10. Conductor Agent Pattern

### Mori: Sophisticated supervisor agent

The Conductor (`conductor/mod.rs`) is a persistent supervisor that:

- **Monitors silence**: If an agent has been silent for
  `silence_timeout` (180s), intervene
- **Counts compile failures**: After `compile_fail_threshold` (3),
  escalate
- **Detects stalls**: `task_stall_timeout` (300s) for stuck tasks
- **Monitors context pressure**: `context_pressure_ratio` (0.8) for
  token usage
- **Phase timeout**: `phase_timeout` (1800s) for stuck phases

**Intervention tiers:**
- **Nudge**: Send a steering message to the agent
- **Restart**: Kill and cold-start the agent
- **Abort**: Force-advance past the current plan

**Conductor actions:**
- `SendMessage`: Nudge/steer an agent
- `RestartAgent`: Kill + restart
- `ForceAdvance`: Skip to next plan
- `SkipReviews`: Commit without review
- `SpawnValidation`: Run a validation pass
- `GenerateFixPlan`: Create a plan from failures
- `InsertGate`: Add a gate to the pipeline
- `SkipValidation`: Skip a validation
- `AssignAdditionalTasks`: Inject tasks into warm agent
- `PingWarmAgent`: Keep-alive for warm agents

**Rate limiting**: `RateLimiter` with soft limit, priority queue for
pending spawns.

### Roko: Conductor adapter but no live supervisor

Roko has `runner/conductor_adapter.rs` and `roko-conductor` crate with
12 watchers and circuit breaker, but:

- The conductor is configured via `[conductor.watchers.*]` in
  `roko.toml`
- A `ConductorRingSink` feeds watcher signals into a ring buffer
- Circuit breaker state is persisted for resume
- But: **no live supervisor loop** that actively monitors and intervenes
  during execution. The conductor signals are collected but intervention
  actions are not dispatched in the runner event loop.

---

## 11. Merge and Integration

### Mori: Dependency-ordered merge queue

- **Merge queue** (`merge_queue: Vec<String>`): Plans that pass
  gates/reviews are queued for merge in dependency order
- **Serialized merges**: Only one plan merges at a time
  (`merge_in_progress` flag)
- **Batch branch**: Plans merge into a shared batch branch
- **Refactoring**: Every N plans, spawn a refactorer agent on the batch
- **Integration tests**: Every N plans, run cross-crate integration
  tests on the batch
- **Post-merge regression**: Workspace-wide regression tests after merge
- **Merge deadlock detection**: `merge_queue_entered_at` timestamps
  for detecting stuck merges

### Roko: Merge dispatch with GitHub integration

- `PlanMerger` with `MergeDispatch` in `runner/merge.rs`
- `MergeQueue` in orchestrator with `MergeRequest` abstraction
- GitHub workflow integration (`runner/github_workflow.rs`): Draft PRs,
  terminal comments, accepted-commit publication
- **But**: No refactoring agent after N merges, no integration test
  scheduling, no post-merge regression runs

---

## 12. Batch Controller

### Mori: Pause after N completions

The `BatchController` (`orchestrator/batch.rs`) is a simple mechanism:

- Track `completed_since_pause` counter
- When counter reaches `batch_size`, return `true` to signal pause
- Operator reviews completed plans, then continues
- Useful for human oversight at regular intervals

### Roko: No batch controller

Roko has no concept of "pause after N plans complete for human review."

---

## 13. UX Comparison

### Mori TUI features for execution

- **Queue overview modal** (`tui/modals/queue_overview.rs`)
- **Wave overview modal** (`tui/modals/wave_overview.rs`)
- **Wave progress widget** (`tui/widgets/wave_progress.rs`)
- **Wave bar widget** (`tui/widgets/wave_bar.rs`)
- **Parallel pool widget** (`tui/widgets/parallel_pool.rs`)
- **Agent pool modal** (`tui/modals/agent_pool_modal.rs`)
- **Phase bar/timeline widgets**
- **Task picker/detail modals**
- **Batch review modal**

### Roko TUI features for execution

- F1-F10 tabs in ratatui dashboard
- `TuiBridge` for state hub updates
- Plan progress views
- Agent output display
- But: **no queue overview, no wave visualization, no milestone
  progress**

---

## 14. Recommendations: Bringing Mori's UX to Roko

### High Priority (directly improves self-hosting)

#### R1: Queue Manifest (`queue.toml`)

Add `.roko/queue.toml` support with milestone grouping. The existing
`plans/INDEX.md` structure already has plan metadata; a queue manifest
would formalize ordering and grouping.

**Implementation path:**
- Add `QueueConfig` struct to roko-cli (port from Mori's
  `orchestrator/queue.rs`)
- Wire into `plan_loader::load_plans()` for ordering
- Add `roko plan queue show/edit/validate` CLI commands
- Runner reads queue config for milestone-based plan selection

**Effort:** Medium. The data model exists in Mori and can be ported
directly. The main work is wiring it into the runner's plan selection
and the TUI's plan list view.

#### R2: Plan-Level Wave Computation

Add Kahn's algorithm for plan-level waves, reusing Mori's `PlanDag`
approach. Roko already has per-plan task DAGs; the missing piece is the
plan-level DAG built from cross-plan dependencies.

**Implementation path:**
- Port `PlanDag::compute_waves()` from Mori's `orchestrator/dag.rs`
- Feed cross-plan `depends_on_plan` into plan-level DAG edges
- Use waves for ordering and parallelism visualization
- Display waves in TUI and CLI status output

**Effort:** Low-Medium. The algorithm is well-tested in Mori and Roko's
task definitions already carry the dependency data.

#### R3: Express Mode

Add an express execution mode that skips reviews and uses auto-fix on
gate failures. This is the single highest-impact operator UX feature
for rapid iteration.

**Implementation path:**
- Add `express` field to `RunConfig`
- Skip review dispatch in event loop when express is set
- On gate failure, attempt auto-fix before re-dispatch
- Add `--express` flag to `roko plan run`
- Configurable in `roko.toml` under `[runner]`

**Effort:** Medium. Roko already has gate failure handling and retry
logic; express mode adds conditional bypasses.

### Medium Priority (improves operator experience)

#### R4: Conductor Supervisor Loop

Wire the existing conductor watchers into a live supervision loop in the
runner event loop. Roko already has `roko-conductor` with 12 watchers
and circuit breaker; the missing piece is the periodic tick that checks
agent health and dispatches interventions.

**Implementation path:**
- Add a periodic conductor tick to the `tokio::select!` loop
- Read from the conductor ring buffer
- Dispatch intervention actions (nudge, restart, force-advance)
- Expose conductor state in TUI

**Effort:** Medium. The infrastructure exists; the gap is the dispatch
loop and action routing.

#### R5: Maintenance Plans

Add maintenance plan support (refactor/QA/docs/integration/audit)
that run after implementation plans complete. Port from Mori's
`MaintenanceBatch` in queue config.

**Implementation path:**
- Add maintenance fields to queue config
- Inject maintenance dependencies into plan-level DAG
- Queue maintenance plans after their batch completes
- Track separately in TUI (e.g., "Milestone X (maintenance)")

**Effort:** Low. This is pure DAG edge injection and queue logic.

#### R6: Batch Controller

Add "pause after N completions" for human oversight. Simple counter
with configurable batch size.

**Implementation path:**
- Add `batch_size` to `RunConfig`
- Count completions and pause event loop
- Resume on operator signal (keypress in TUI or CLI flag)

**Effort:** Low. The `BatchController` from Mori is 35 lines.

#### R7: Wave Visualization in TUI

Add wave progress display to Roko's TUI dashboard. Show which wave is
active, which plans are parallel, and estimated time remaining.

**Effort:** Medium. Depends on R2 (wave computation) being done first.

### Lower Priority (nice to have)

#### R8: Warm Agent Reuse

Keep implementer agents warm across gate-fix cycles instead of
cold-starting for every retry. Requires tracking
`primary_agent_instance` per plan.

**Effort:** High. Requires agent lifecycle changes.

#### R9: Gate-Review Overlap

Pre-spawn reviewers alongside gate execution. Cancel on gate failure,
continue on pass. Saves wall-clock time for Standard/Complex plans.

**Effort:** High. Requires concurrent agent tracking and cancellation
logic.

#### R10: File Overlap Analysis

Detect plans in the same wave that touch overlapping crates. Warn the
operator or serialize conflicting plans.

**Effort:** Low. Port `file_overlap_analysis()` from Mori's DAG module.

#### R11: Critical Path Display

Show the critical path (longest sequential chain) in the TUI and CLI
status output. Useful for identifying bottlenecks.

**Effort:** Low. Port `critical_path()` from Mori.

#### R12: Per-Run Session Config

Allow a per-run config file (separate from `roko.toml`) that specifies
model overrides, agent limits, and execution preferences for a single
run session. Similar to Mori's `[run]` section in `queue.toml`.

**Effort:** Medium. Config layering logic already exists in roko-core
(`priority/provenance, seven invariants, migrations`); the gap is a
CLI path for "this run only" overrides.

---

## 15. Summary Table

| Feature | Mori | Roko | Gap |
|---|---|---|---|
| Queue manifest | `.mori/queue.toml` | None | **R1** |
| Milestones | Named groups with progression | None | **R1** |
| Maintenance plans | Refactor/QA/docs after batch | None | **R5** |
| Wave computation | Kahn's algorithm | None | **R2** |
| Critical path | Longest weighted path | None | **R11** |
| Express mode | Full implementation | None | **R3** |
| Conductor supervisor | Live monitoring + intervention | Watchers exist, no dispatch | **R4** |
| Agent budget tracking | Global across all roles | Per-task only | Low gap |
| Spawn backoff | Per-plan exponential | Per-task retry | Low gap |
| Zombie reaping | 4-hour max age | Timeout-based | Low gap |
| Task batching | Multiple tasks to one agent | One task per agent | **R8** |
| Gate-review overlap | Pre-spawn warm reviewer | None | **R9** |
| Warm agent reuse | Primary agent persists | Cold start per retry | **R8** |
| Complexity routing | 6-phase heuristic | CascadeRouter learned | Different approach |
| Auto-fix on gate fail | Express mode auto-fixer | Replan-on-failure | Different approach |
| Batch controller | Pause after N plans | None | **R6** |
| File overlap detection | Crate-level analysis | None | **R10** |
| Merge queue | Dependency-ordered serial | MergeQueue exists | Low gap |
| Post-merge regression | Workspace tests after merge | None | Medium gap |
| Review cap | Force-commit after N cycles | None | Low gap |
| Resume fingerprinting | Basic task-level | Strict with drift detection | **Roko ahead** |
| Adaptive gate thresholds | None | EMA per rung | **Roko ahead** |
| Learned model routing | None | CascadeRouter | **Roko ahead** |
| Prompt experiments | None | A/B testing | **Roko ahead** |
| Safety layer | None | Trust-origin IFC, immune graph | **Roko ahead** |
| Provider diversity | 3 backends | 11 provider kinds | **Roko ahead** |
| Telemetry | Event log | 39-variant observable events | **Roko ahead** |
| TUI wave view | Wave overview + progress | None | **R7** |
| TUI queue view | Queue overview modal | None | **R7** |

---

## 16. Porting Priority Order

For reaching full self-hosting confidence with operator UX parity:

1. **R1 Queue Manifest** + **R2 Wave Computation** -- These two together
   give the operator control over what runs and visibility into how the
   dependency graph will be parallelized. They are prerequisites for
   milestones, maintenance, and wave visualization.

2. **R3 Express Mode** -- The single most impactful runtime optimization.
   Skipping reviews for known-good plans and auto-fixing trivial gate
   failures saves 40-60% wall-clock time on batch runs.

3. **R4 Conductor Supervisor** -- The infrastructure exists. Wiring the
   dispatch loop turns passive monitoring into active intervention,
   preventing stuck agents from burning tokens indefinitely.

4. **R5 Maintenance Plans** + **R6 Batch Controller** -- Operational
   hygiene. Maintenance plans keep code quality from degrading during
   long runs. Batch controller gives operators a natural review point.

5. **R7 Wave Visualization** -- UX polish that makes the dashboard
   useful during long multi-plan runs.

6. **R8-R12** -- Optimizations and nice-to-haves that improve efficiency
   but are not blocking for self-hosting.
