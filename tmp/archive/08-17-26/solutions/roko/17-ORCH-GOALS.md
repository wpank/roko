# Orchestration: Goals

## End State

A single `WorkflowEngine` that reads configurable workflow definitions (TOML)
and executes them as composable pipelines. Any entry point (`roko run`,
`roko plan run`, ACP) goes through the same engine. The engine supports
parallel multi-task DAG execution, per-task worktree isolation, serialized
merging, and resume from any point.

---

## 1. Primary Goals

### 1.1 Single Runtime

**Current**: 3 runtimes (ACP pipeline, Runner v2, orchestrate.rs), each with
different capabilities, state machines, and dispatch paths.

**Target**: One `WorkflowEngine` that handles every workflow type:
- Single-prompt (express/standard/full) -- currently PipelineStateV2
- Multi-task DAG (plan execution) -- currently Runner v2 + PlanPhase
- ACP editor workflows -- currently separate PipelinePhase

**What exists today**:
- `WorkflowEngine` facade in `roko-runtime/src/workflow_engine.rs` (1,678 LOC)
- `PipelineStateV2` pure FSM in `roko-runtime/src/pipeline_state.rs` (1,110 LOC)
- `EffectDriver` in `roko-runtime/src/effect_driver.rs` (858 LOC)
- `TaskScheduler` DAG resolver in `roko-runtime/src/task_scheduler.rs` (379 LOC)
- CLI wired via `run_with_workflow_engine()`
- ACP wired via AcpAdapter

**Gap**: WorkflowEngine does not yet support the full Runner v2 feature set
(worktree isolation, merge queue, dream consolidation, speculative execution).
The orchestrate.rs features (knowledge routing, VCG auction, custody chain,
skill extraction, predictive calibration) have no path into WorkflowEngine.

### 1.2 Workflow-as-Config

**Current**: Hardcoded phase sequences in both PipelineStateV2 (match arms on
phase + input) and PlanStateMachine (match arms on PhaseKind + event).

**Target**: Pipelines defined in TOML, not Rust state machines. Users drop a
`.toml` file to create new workflows.

**What exists today**:
- `WorkflowConfig::from_toml_str()` supports basic TOML:
  ```toml
  [workflow]
  template = "standard"      # express | standard | full
  has_strategy = true
  has_review = true
  max_iterations = 3
  max_autofix_attempts = 2

  [[workflow.steps]]
  name = "strategy"
  role = "strategist"

  [[workflow.steps]]
  name = "implement"
  role = "implementer"
  ```
- Steps array infers `has_strategy` and `has_review` flags from step names
- Three preset templates (express, standard, full) with override capability

**Gap**:
- No per-step gate configuration (all steps share the same gates)
- No per-step failure routing (hardcoded in PipelineStateV2::step match arms)
- No parallel step execution (all steps are serial)
- No custom step types beyond strategy/implement/review
- No synthesis/merge step for combining parallel results
- Steps array is parsed but only used to infer boolean flags -- the actual
  execution ignores step order and configuration

### 1.3 DAG-Parallel Execution

**Current**: `max_concurrent_tasks: 1` despite full DAG infrastructure.

**Target**: Configurable parallelism with wave gating, file-overlap
serialization, and cost-aware scheduling.

**What exists today**:
- `UnifiedTaskDag` with full BFS wave scheduling (dag.rs, 2,557 LOC)
- `TaskScheduler` with file-exclusion-aware `next_batch()` (task_scheduler.rs)
- `DagConfig::max_wave_width` for bounding wave parallelism
- CPM analysis (earliest/latest start, slack, critical path)
- Chain fusion for collapsing linear sequences
- Live DAG mutations (AddTask, RemoveTask, SplitTask, AddDependency)
- `DagExecutionSnapshot` with per-task status tracking

**Gap**:
- Runner v2 event loop defaults to serial (max_concurrent_tasks=1)
- WorkflowEngine's TaskScheduler exists but is not yet connected to the
  EffectDriver's agent spawning in a concurrent way
- Speculative execution structs exist but no trigger code
- No cost-aware scheduling (cheaper tasks could run first to conserve budget)
- No adaptive parallelism (adjust concurrency based on error rates)

### 1.4 Context Handoff

**Current**: Each agent gets its prompt in isolation. No cumulative state is
passed between agents in the same plan.

**Target**: Progressive refinement where each agent knows what prior agents
changed, what gates passed/failed, and what the cumulative diff looks like.

**What exists today**:
- PipelineStateV2 accumulates `review_findings` across iterations
- PipelineStateV2 carries `last_gate_failure` for retry context
- EffectDriver passes `context` parameter to `SpawnImplementer` action
- orchestrate.rs has cumulative context via `load_prior_task_outputs()` and
  `with_task_failure_context()` but these are not ported to WorkflowEngine
- The mega-parity runner uses a "cumulative section" showing what files
  changed in prior batches

**Gap**:
- No cumulative diff view for agents within the same plan
- No agent-to-agent message passing (one agent cannot directly inform another)
- No structured handoff of gate results to downstream agents
- No "what changed around you" context for parallel agents

### 1.5 Resume and Crash Recovery

**Current**: Three different resume mechanisms with different strictness.

**Target**: Unified resume that works across all entry points, with fingerprint
validation and graceful degradation.

**What exists today**:
- PipelineStateV2::checkpoint/from_checkpoint (JSON round-trip)
- EffectDriver::save_checkpoint (atomic write via tmp + rename)
- Runner v2 fingerprint validation (TaskDefFingerprint in persist.rs)
- ExecutorSnapshot with plan_states, queue_order, circuit breaker state
- ProcessSessionConfig with resumable flag and session ledger

**Gap**:
- WorkflowEngine checkpoints PipelineStateV2 but not TaskScheduler state
- No fingerprint validation in WorkflowEngine (Runner v2 has it)
- No reconciliation of checkpoint + JSONL state (Runner v2 has prepare_resume)
- No cross-process session resume (sessions.json exists but no consumer)

---

## 2. Secondary Goals

### 2.1 Configurable Failure Recovery

**Current**: Hardcoded failure routing in PipelineStateV2::step and
PlanStateMachine::transition.

**Target**: Per-step `on_failure` routing:
- `retry(max=3, backoff=exponential)` -- retry the same step
- `escalate(to="human")` -- halt and notify
- `loop_back(to="implement")` -- re-run an earlier step with failure context
- `skip()` -- mark step as skipped and continue
- `replan()` -- generate new tasks based on failure analysis

**What exists today**:
- PipelineStateV2: autofix_attempts < max -> AutoFixing, else iteration <
  max_iterations -> re-Implementing, else Halted
- PlanStateMachine: iteration >= MAX_AUTO_FIX_ITERATIONS -> Failed(AutoFixExhausted),
  merge_attempts >= MAX_MERGE_ATTEMPTS -> Failed(Deadlock)
- roko-orchestrator/replan.rs: ReplanStrategy, PlanRevisionRequest,
  PlanRevisionEvidence -- replanning infrastructure exists
- roko-orchestrator/repair.rs: RepairEngine with RepairLevel and RepairAction

### 2.2 Parallel Step Execution

**Target**: Spawn N agents in parallel within a single workflow step (e.g.,
tournament mode, swarm execution, multi-reviewer).

**Data feeds required for UX (from v2 UX showcase):**
- `SwarmState` -- per-agent: id, branch, approach_name, status, metric,
  progress, gates, winner flag
- `PlanState` -- entries with content, status, role, priority, replan flag
- `AgentChat` -- from_role, to_role, text (cross-agent messages)
- `ConvergenceMetrics` -- rounds, must_fixes, nits, meta-metric score
- `CheckpointEvent` -- hash, files_changed, iterations, restore_action

### 2.3 Worktree Integration in WorkflowEngine

**Target**: WorkflowEngine allocates worktrees for parallel tasks, manages
their lifecycle, and coordinates merge.

**What exists today**:
- `WorktreeManager` (roko-orchestrator/src/worktree.rs, 1,203 LOC)
- `MergeQueue` (roko-orchestrator/src/merge_queue.rs, 924 LOC)
- `PostMergeRunner` for regression gates
- Runner v2 uses all three via PlanMerger

**Gap**: WorkflowEngine does not reference WorktreeManager or MergeQueue.

### 2.4 Learning Integration

**Target**: WorkflowEngine feeds outcomes to the learning subsystem so that
future runs benefit from past experience.

**What exists today in orchestrate.rs:**
- EpisodeLogger for turn recording
- PlaybookStore for replay context
- CascadeRouter for model routing
- ExperimentStore for A/B testing
- AdaptiveThresholds for gate tuning
- SectionEffectivenessRegistry for prompt section evaluation
- RoutingDecisionLog for audit trail

**Gap**: WorkflowEngine's FeedbackSink trait is minimal -- it records model
calls and gate results but not episodes, playbooks, experiments, or routing
decisions.

---

## 3. Lessons From Mega-Parity Runner

The mega-parity runner (195 parallel batches, ~6 hours, 177K LOC workspace)
validated the orchestration loop at scale. Key insights that inform goals:

### 3.1 Isolation is Non-Negotiable

> "Agents must work in separate worktrees/sandboxes. Shared mutable state
> between concurrent agents is a recipe for corruption."

WorkflowEngine must integrate WorktreeManager for any parallel execution.
The mega-parity runner used one worktree per batch (~500MB each, 15 concurrent).

### 3.2 Context Handoff is the Hard Problem

> "Telling agent B what agent A changed is more important than telling agent B
> what to do. Bad context -> merge conflicts -> wasted work -> cascade failures."

The cumulative section pattern (showing what changed in prior batches) reduced
merge conflicts significantly. WorkflowEngine needs a structured mechanism
for building cumulative context across agents.

### 3.3 Gates Should Be Batched, Not Per-Agent

> "Compiling after every agent turn is too expensive. Compile after a batch of
> changes accumulates. The trade-off is delayed error detection, but the time
> savings are 10-100x."

Numbers from the runner:
- Per-batch compile: 15-40 minutes per batch, 195 batches = 50+ hours
- Wave gates only: 3-8 minutes per wave, ~20 waves = 2-3 hours
- No gates (deferred): 0 minutes during run, 30 min fix-up at end

WorkflowEngine should support configurable gate timing: per-task, per-wave,
or deferred.

### 3.4 Result Files Are the Coordination Mechanism

> "Not message passing, not shared memory -- simple files on disk that say
> 'success' or 'failed.' Any process can read them, any process can write them."

The --continue pattern: resume from disk state, not memory state. Kill and
restart freely. WorkflowEngine's checkpoint mechanism is the right approach;
it needs to extend to TaskScheduler state.

### 3.5 Manual Intervention is a Feature

> "The system should make it easy for a human to: read status, mark things as
> done, unblock dependencies, kill stalled processes, and restart."

WorkflowEngine should expose intervention points:
- Override task status (mark as completed/skipped)
- Unblock dependencies manually
- Inject failure context for retries
- Pause/resume execution

### 3.6 The Auto-Pick Pattern

> "Having a separate process that watches for completed work and integrates it
> into your branch means you can keep working while agents generate code."

A background cherry-picker that monitors completed tasks and integrates them
is more practical than synchronous merge-after-each-task. This maps to the
MergeQueue pattern but with asynchronous integration.

---

## 4. Novel Orchestration Patterns

### 4.1 Speculative Execution

Pre-dispatch agents for tasks likely to become ready, based on:
- Critical path analysis (zero-slack tasks are dispatched speculatively)
- Historical completion probability (from learning subsystem)
- Cost threshold (only speculate when cost is below budget multiplier)

Infrastructure exists (`SpeculativeExecution` struct in executor snapshots)
but no trigger code. Goal: wire speculative dispatch when a task's dependencies
are 80%+ complete and the task is on the critical path.

### 4.2 Adaptive Parallelism

Dynamically adjust concurrency based on runtime signals:
- Error rate spike -> reduce parallelism (fewer concurrent agents = less wasted work)
- Low error rate + budget headroom -> increase parallelism
- Disk pressure -> reduce parallelism
- API rate limit proximity -> reduce parallelism

DagConfig::max_wave_width supports static bounds; adaptive parallelism would
adjust this at wave boundaries based on metrics from the preceding wave.

### 4.3 Cost-Aware Scheduling

Prioritize task dispatch order by expected cost efficiency:
- Mechanical tasks (cheap, fast) first -- early wins build momentum
- High-dependency tasks (many downstream dependents) prioritized to unblock
- Critical path tasks never deferred
- Expensive tasks deferred to later waves when budget is known

The CascadeRouter already routes to cheaper models for simpler tasks. Cost-aware
scheduling extends this to task ordering, not just model selection.

### 4.4 Progressive Refinement

Multi-pass agent strategy where each pass builds on the prior:
- Pass 1: Fast model writes initial implementation (80% correct)
- Pass 2: Stronger model reviews and fixes critical issues
- Pass 3: Gate results inform targeted fixes

The mega-parity runner's two-pass model (implementation + audit) validated this
at scale. PipelineStateV2's iteration mechanism supports this natively but only
drives it via gate failures, not proactively.

### 4.5 Tournament Execution

Spawn N agents with different strategies for the same task:
- Different models (Claude vs GPT vs Gemini)
- Different approaches (refactor vs rewrite)
- Different context windows (minimal vs full)

Pick the winner based on gate results, token efficiency, or diff size.
The WorktreeManager supports isolated branches for each contestant.

### 4.6 Wave Gating

Compile/test at wave boundaries instead of per-task:
- Wave N tasks all complete -> merge all into integration branch -> run gates
- If gates fail, identify which task caused regression (git bisect on merges)
- Retry only the offending task(s)

The mega-parity runner used this pattern successfully with `--no-gate` per-batch
and `cargo check --workspace` between waves.

---

## 5. Gap Summary

| Goal | What Exists | What is Missing |
|---|---|---|
| Single runtime | WorkflowEngine + PipelineStateV2 + EffectDriver + TaskScheduler | Full Runner v2 + orchestrate.rs feature parity |
| Workflow-as-config | TOML parsing, 3 presets, step inference | Per-step gates, failure routing, custom steps |
| DAG-parallel execution | UnifiedTaskDag, TaskScheduler, CPM | Wired concurrency, speculative dispatch |
| Context handoff | review_findings, gate feedback | Cumulative diff, agent messaging |
| Resume | JSON checkpoint, fingerprint validation | Unified resume across all modes |
| Failure recovery | Autofix loop, iteration limits | Configurable per-step routing |
| Parallel steps | WorktreeManager, MergeQueue | WorkflowEngine integration |
| Learning | FeedbackSink trait | Episode, playbook, experiment recording |
| Worktree integration | WorktreeManager (1,203 LOC) | WorkflowEngine connection |
| Speculative execution | SpeculativeExecution struct | Trigger code, cost thresholds |
| Adaptive parallelism | max_wave_width | Runtime adjustment based on metrics |
| Cost-aware scheduling | CascadeRouter | Task ordering by cost efficiency |

---

## Sources

- `crates/roko-runtime/src/pipeline_state.rs` -- WorkflowConfig TOML parsing, PipelineStateV2 FSM
- `crates/roko-runtime/src/effect_driver.rs` -- EffectServices, affect modulation, gate dispatch
- `crates/roko-runtime/src/workflow_engine.rs` -- WorkflowRunConfig, WorkflowResult
- `crates/roko-runtime/src/task_scheduler.rs` -- TaskScheduler, next_batch(), file exclusion
- `crates/roko-orchestrator/src/dag.rs` -- UnifiedTaskDag, waves, CPM, mutations, fusion
- `crates/roko-orchestrator/src/worktree.rs` -- WorktreeManager
- `crates/roko-orchestrator/src/merge_queue.rs` -- MergeQueue, file-conflict-aware
- `crates/roko-orchestrator/src/replan.rs` -- ReplanStrategy, PlanRevisionRequest
- `crates/roko-orchestrator/src/repair.rs` -- RepairEngine
- `crates/roko-cli/src/orchestrate.rs` -- All legacy features (import analysis, 22,522 LOC)
- `crates/roko-cli/src/runner/event_loop.rs` -- Runner v2 concurrency defaults
- `tmp/solutions/runner/LESSONS.md` -- Mega-parity runner operational lessons
