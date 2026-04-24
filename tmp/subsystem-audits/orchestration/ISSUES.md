# Orchestration: Issues

Cataloged issues from source code analysis, runtime experience, and the
mega-parity runner. Ordered by impact (critical -> medium -> low).

---

## Critical Issues

### ORCH-001: Serial Execution Default (max_concurrent_tasks=1)

**Location**: `crates/roko-cli/src/runner/event_loop.rs` concurrency defaults

**Problem**: Despite having a full DAG scheduler with wave scheduling, CPM
analysis, and file-overlap serialization, the Runner v2 defaults to executing
one task at a time. The WorkflowEngine's TaskScheduler also has `max_parallel`
but it is not yet wired to concurrent agent spawning.

**Impact**: A plan with 20 tasks in 4 waves takes 20x longer than it should.
The mega-parity runner demonstrated that parallel execution (PARALLEL=15) reduces
total runtime by 10-15x for independent tasks.

**Evidence**: Runner v2 event_loop.rs sets:
```rust
max_concurrent_plans: 4
max_concurrent_tasks: 1  // <-- the bottleneck
```

**Root cause**: Concurrency introduces merge conflicts and shared-state hazards
that were not solved when the config was introduced. The conservative default
was chosen to avoid data corruption but was never revisited.

**Fix path**: Increase `max_concurrent_tasks` to a configurable value (default 4),
gated behind worktree isolation. Only allow parallel tasks when
`WorktreeManager` is active and can provide per-task isolation.

---

### ORCH-002: 22K LOC God File (orchestrate.rs)

**Location**: `crates/roko-cli/src/orchestrate.rs` (22,522 lines)

**Problem**: The legacy orchestrator is a single file that implements features
belonging in 6+ crates: enrichment, custody, skill extraction, C-factor
computation, predictive calibration, knowledge routing, dream consolidation,
affect engine, heartbeat monitoring, and more. Its 160+ import lines pull from
every crate in the workspace.

**Impact**: Features built in orchestrate.rs cannot be used by WorkflowEngine,
ACP, or the HTTP control plane. Bug fixes in orchestrate.rs are invisible to
other runtimes. New developers cannot navigate a 22K-line file.

**Evidence**: Import block spans lines 1-210. The file imports from roko_agent
(16 items), roko_compose (8), roko_conductor (5), roko_core (30+),
roko_daimon (6), roko_dreams (3), roko_gate (20+), roko_learn (25+),
roko_neuro (8), roko_orchestrator (15+), roko_runtime (5).

**Root cause**: Organic growth during the plan-execute-gate-persist loop
development. Each feature was added to the existing entry point rather than
extracted to a crate.

**Fix path**: Extract features into EffectDriver service traits:
1. KnowledgeService (knowledge routing, neuro queries)
2. LearningService (episodes, playbooks, experiments, routing log)
3. EnrichmentService (30+ steps, section effectiveness)
4. CustodyService (audit chain, provenance tracking)
5. DiagnosisService (stuck detection, heartbeat, health monitoring)
6. SkillService (extraction, library, queries)

---

### ORCH-003: Two Incompatible State Machines

**Location**: `roko-runtime/src/pipeline_state.rs` (PipelineStateV2, 10 states)
and `roko-core/src/phase.rs` (PlanPhase, 14 states)

**Problem**: Two state machines model the same concept (workflow execution
lifecycle) but are not interoperable:
- PipelineStateV2: Pending, Strategizing, Implementing, Gating, AutoFixing,
  Reviewing, Committing, Complete, Halted, Cancelled
- PlanPhase: Queued, Enriching, Implementing, Gating, Verifying, Reviewing,
  DocRevision, AutoFixing, RegeneratingVerify, Merging, Complete, Done,
  Failed, Skipped

Differences:
- PipelineStateV2 has Strategizing, Committing, Cancelled -- PlanPhase does not
- PlanPhase has Enriching, Verifying, DocRevision, RegeneratingVerify, Merging,
  Done, Skipped -- PipelineStateV2 does not
- PipelineStateV2 Halted carries a reason String; PlanPhase Failed carries
  a typed FailureKind enum

**Impact**: WorkflowEngine and Runner v2 cannot share state. A workflow
starting as a simple run (PipelineStateV2) cannot be promoted to a plan
execution (PlanPhase) without losing state. Monitoring tools must handle
both state machines.

**Fix path**: Define a superset state machine that covers both, with optional
phases that can be skipped based on workflow configuration. Alternatively,
define a phase adapter that maps between the two.

---

### ORCH-004: Four Agent Dispatch Implementations

**Location**:
- ACP: `roko-acp/src/runner.rs` -> `run_claude_cli()` (~30 LOC)
- Runner v2: `roko-cli/src/dispatch/mod.rs` + `dispatch_v2.rs` (~3,040 LOC)
- Orchestrate.rs: `dispatch_agent()` + `dispatch_agent_with()` (~2,142 LOC)
- WorkflowEngine: `effect_driver.rs` -> `spawn_agent()` (~510 LOC)

**Problem**: Each dispatch path has different features, error handling, timeout
logic, token counting, and safety checks. Bug fixes land in one implementation
and are not propagated. The EffectDriver has affect modulation but not safety
layers; orchestrate.rs has safety layers but different affect integration;
Runner v2 has budget tracking but not custody logging.

**Impact**: Agents dispatched from different surfaces behave differently.
A safety bug fixed in one dispatch path remains open in three others.

**Fix path**: Consolidate into EffectDriver's ModelCaller + PromptAssembler
trait pattern. Add service traits for safety, custody, and knowledge routing
that can be composed into the EffectServices struct.

---

## High Issues

### ORCH-005: Speculative Execution Built But Not Wired

**Location**: `roko-orchestrator/src/executor/mod.rs` (SpeculativeExecution
struct), `roko-orchestrator/src/executor/snapshot.rs` (serialized in
ExecutorSnapshot)

**Problem**: The SpeculativeExecution infrastructure (struct, config field
`speculative_threshold_multiplier`, snapshot persistence) exists but no code
in Runner v2's event loop or WorkflowEngine's run loop triggers speculative
task spawns.

**Impact**: Tasks on the critical path are not pre-dispatched even when their
dependencies are nearly complete. This adds unnecessary latency to plan
execution.

**Fix path**: In the event loop, when a task's dependencies are 80%+ complete
(by count or by estimated time) and the task is on the critical path
(`dag.slack(task).is_zero()`), spawn the agent speculatively. If dependencies
fail, cancel the speculative agent. Track speculative cost separately.

---

### ORCH-006: No Cumulative Context for Parallel Agents

**Location**: `roko-runtime/src/effect_driver.rs` spawn_agent, line 143

**Problem**: EffectDriver passes a flat `context` string to agents:
```rust
let user_content = context.map_or_else(
    || user_prompt.to_string(),
    |ctx| format!("{user_prompt}\n\n## Additional Context\n\n{ctx}"),
);
```

There is no mechanism to build cumulative context showing what other agents in
the same plan have changed. The mega-parity runner identified this as the
single most impactful context improvement: "Telling agent B what agent A
changed is more important than telling agent B what to do."

**Impact**: Parallel agents write conflicting code. Sequential agents repeat
work. Merge conflicts increase linearly with parallelism.

**Evidence from mega-parity runner**: The cumulative section (showing files
changed in prior batches) reduced merge conflicts from ~50% to ~30% of
cherry-picks. Signature-only views of changed files kept token overhead
manageable.

**Fix path**:
1. After each task completes, compute a git diff summary (files changed,
   functions added/modified)
2. Maintain a cumulative context buffer per plan
3. Before dispatching a new task, inject the cumulative context as a
   structured section (e.g., "## What Changed Before You")
4. Use signature-only views for large files to stay within token budget
5. orchestrate.rs has `load_prior_task_outputs()` and
   `with_task_failure_context()` -- port these to EffectDriver

---

### ORCH-007: WorkflowEngine Missing Worktree Integration

**Location**: `roko-runtime/src/workflow_engine.rs`

**Problem**: WorkflowEngine operates on a single `workdir: PathBuf` and has
no reference to WorktreeManager or MergeQueue. When TaskScheduler dispatches
parallel tasks, they all operate in the same working directory, causing
file conflicts.

**Impact**: Parallel execution in WorkflowEngine is unsafe without worktree
isolation. This blocks ORCH-001.

**Fix path**: Add optional `WorktreeManager` to EffectServices. When parallel
tasks are dispatched, allocate a worktree per task via
`WorktreeManager::create_for_plan()`. After task completion, merge via
MergeQueue with file-overlap detection.

---

### ORCH-008: Gate Rung Mapping Duplication

**Location**: `roko-runtime/src/effect_driver.rs` `rung_for_gate_name()`
(lines 645-656)

**Problem**: The EffectDriver duplicates the gate rung mapping from
GateService. The source code contains a TODO acknowledging this:
```rust
/// TODO: expose this mapping from roko-gate as a public function so this
/// duplicate is not needed.
```

**Impact**: If roko-gate adds new gate types or changes rung assignments,
the EffectDriver will silently use stale mappings. The affect policy will
receive incorrect rung values, potentially misclassifying gate results.

**Fix path**: Export `rung_for_gate_name` as a public function from roko-gate.
Import it in EffectDriver instead of duplicating.

---

### ORCH-009: Incomplete TaskScheduler Resume

**Location**: `roko-runtime/src/task_scheduler.rs`, `workflow_engine.rs`

**Problem**: WorkflowEngine checkpoints PipelineStateV2 state (phase, iteration,
findings) but does not checkpoint TaskScheduler state (task statuses, which
tasks are running/completed/failed). A crash during multi-task execution loses
all task-level progress.

**Impact**: Resume after crash restarts all tasks from the beginning, even
those that completed successfully. In a 20-task plan, a crash at task 15
loses the work from tasks 1-14.

**Fix path**: Extend the checkpoint to include TaskScheduler state. The
`TaskStatus` enum is not Serialize/Deserialize -- add those derives. Include
task statuses in the checkpoint JSON alongside PipelineStateV2 state.

---

### ORCH-010: No Cost-Aware Task Ordering

**Location**: `roko-runtime/src/task_scheduler.rs` `next_batch()`

**Problem**: TaskScheduler picks ready tasks based on insertion order and file
exclusion constraints. It does not consider:
- Estimated token cost (from model_hint and description length)
- Task tier (mechanical tasks are cheaper than architectural tasks)
- Downstream dependency count (tasks that unblock more work should run first)
- Critical path membership (zero-slack tasks are time-critical)

**Impact**: Expensive tasks may run before cheap ones, consuming budget before
quick wins are secured. Tasks with many downstream dependents may be delayed
behind tasks that unblock nothing.

**Fix path**: Score ready tasks before selection:
```
priority = critical_path_bonus * 2.0
        + downstream_dependents * 0.5
        + (1.0 / estimated_cost) * 0.3
```
Pick highest-priority tasks first, subject to file exclusion and parallelism cap.

---

## Medium Issues

### ORCH-011: Affect Policy Not Fully Wired

**Location**: `roko-runtime/src/effect_driver.rs`

**Problem**: EffectDriver supports AffectPolicy via `EffectServices::affect_policy`
but Runner v2 only provides `DaimonPolicy::default()` for routing context.
The full DaimonState affect engine in orchestrate.rs (load_or_new at line 299,
with somatic signals, strategy coordinates, and dispatch modulation) is not
available through WorkflowEngine.

**Impact**: Behavioral modulation (exploration vs exploitation, tier bias,
turn limits) defaults to neutral values. The affect engine cannot learn from
task outcomes because it receives no real state.

---

### ORCH-012: Dream Consolidation Not in WorkflowEngine

**Location**: `roko-cli/src/runner/event_loop.rs` (line 668+)

**Problem**: Dream consolidation is triggered in Runner v2 after plan completion
but is not available in WorkflowEngine. The DreamRunner, DreamLoopConfig, and
DreamAgentConfig imports in orchestrate.rs are not used elsewhere.

**Impact**: Knowledge distillation from completed workflows does not happen
when using the unified WorkflowEngine path.

---

### ORCH-013: Merge Queue Not Connected to WorkflowEngine

**Location**: `roko-orchestrator/src/merge_queue.rs`

**Problem**: MergeQueue (924 LOC) is fully built and active in Runner v2 via
PlanMerger, but WorkflowEngine has no merge step. The EffectDriver commits
directly via `git add -A && git commit` without merge queue coordination.

**Impact**: Concurrent WorkflowEngine runs can conflict on merge. No
file-overlap detection, no serialized merge ordering, no regression gate.

---

### ORCH-014: No Structured Agent-to-Agent Communication

**Location**: N/A (not implemented anywhere)

**Problem**: The v2 UX showcase specifies `AgentChat` (from_role, to_role,
text) for cross-agent messages (e.g., ARC -> reviewer, AUD -> reviewer).
No implementation exists.

**Impact**: Multi-role review rounds cannot communicate findings between
reviewers. The multi-role review in ACP (runner.rs) spawns reviewers
sequentially but does not pass findings between them.

---

### ORCH-015: Hardcoded Failure Recovery

**Location**: `roko-runtime/src/pipeline_state.rs` PipelineStateV2::step()
(lines 662-687)

**Problem**: Gate failure recovery is hardcoded:
```rust
if self.autofix_attempts < self.config.max_autofix_attempts {
    // try autofix
} else if self.iteration < self.config.max_iterations {
    // re-implement with failure context
} else {
    // halt
}
```

No per-gate failure routing (e.g., compile failure -> autofix, test failure ->
re-implement, lint failure -> skip). No escalation to a different model or
strategy on repeated failure.

**Impact**: All gate failures are treated identically. A trivial lint warning
triggers the same recovery path as a fundamental type error.

---

### ORCH-016: No Anti-Pattern Checks in WorkflowEngine

**Location**: N/A

**Problem**: The mega-parity runner uses fast grep-based anti-pattern checks
(AP-1 through AP-10) that catch common LLM code generation mistakes in
milliseconds. These are not integrated into WorkflowEngine or any gate.

Anti-patterns detected:
- AP-1: Stub gates that return pass (silent-pass)
- AP-2: `block_on` in async code
- AP-3: Duplicate trait definitions vs foundation.rs
- AP-5: Raw `Command::new("claude")` shell-outs
- AP-6: Inline prompt strings (`format!("You are a...")`)
- AP-7: std::sync::Mutex held across .await
- AP-8: Empty function bodies
- AP-9: unimplemented!/unreachable! left behind
- AP-10: Hardcoded localhost/port in non-test code

**Impact**: LLM-generated code with structural mistakes passes gates because
compilation and tests do not catch these patterns.

**Fix path**: Add AP checks as a pre-gate step (rung -1) that runs in
milliseconds before any expensive compilation.

---

## Low Issues

### ORCH-017: PipelineStateV2 TOML Steps Not Executed

**Location**: `roko-runtime/src/pipeline_state.rs` parse_workflow_config_toml

**Problem**: The `[[workflow.steps]]` array in TOML is parsed but only used
to infer `has_strategy` and `has_review` boolean flags. Step role, order, and
configuration are discarded. The actual execution ignores the steps array
entirely.

**Impact**: Users who configure custom step sequences via TOML get the same
execution as the preset templates. The steps array gives a false impression
of configurability.

---

### ORCH-018: No Gate Confidence/Rung on GateVerdict

**Location**: `roko-core/src/foundation.rs` GateVerdict struct

**Problem**: GateVerdict carries gate_name, passed, output, and duration_ms
but not rung or confidence. The EffectDriver re-derives rung from the gate
name (ORCH-008) and uses a hardcoded confidence (1.0 for deterministic, 0.5
for heuristic).

The source contains a TODO:
```rust
// TODO: add `rung: u8` and `confidence: f64` to GateVerdict
```

**Impact**: Affect policy receives imprecise gate signals. Gate result
analysis cannot distinguish between deterministic and heuristic gate failures.

---

### ORCH-019: Supervision Strategy Defaults to OneForOne(max_restarts=0)

**Location**: `roko-runtime/src/process.rs` SupervisionStrategy::default()

**Problem**: The default supervision strategy has `max_restarts: 0`, meaning
any process failure is terminal. There is no automatic restart.

**Impact**: Transient failures (e.g., API timeout, network glitch) kill the
agent process permanently. The orchestrator must handle restart logic
externally.

---

### ORCH-020: No Disk Pressure Monitoring

**Location**: N/A

**Problem**: The mega-parity runner learned that disk exhaustion (from cargo
build caches) causes silent failures:

> "macOS runs out of space, processes fail with I/O errors, git operations
> fail silently, and things get weird."

WorkflowEngine has no disk space monitoring. Cargo builds in worktrees can
consume 5-15GB each. With parallel execution, disk usage scales linearly.

**Fix path**: Add a pre-dispatch check that queries available disk space.
If below a threshold (e.g., 5GB), pause dispatch until space is freed. The
mega-parity runner's 2GB threshold was insufficient for Rust workspaces.

---

### ORCH-021: Agent Instruction Non-Compliance

**Problem**: The mega-parity runner found that ~5% of agents ignore explicit
instructions (e.g., "do not run cargo"). Agent compliance is probabilistic.

**Impact**: Agents that run unauthorized builds consume 10-30 minutes instead
of 1-5 minutes, exhaust disk space, and create target directory lock contention.

**Evidence from mega-parity runner**:
- ~5% instruction non-compliance rate
- Non-compliant agents take 5-15x longer
- Can be detected by monitoring batch duration

**Fix path**: Monitor agent execution time. If duration exceeds 3x expected
for the task tier, send SIGTERM and retry with stronger instructions. Consider
sandboxing (fake cargo binary, restricted PATH) as a fallback.

---

## Issue Cross-References

| Issue | Blocks | Blocked By |
|---|---|---|
| ORCH-001 (Serial default) | All parallel execution | ORCH-007 (Worktree integration) |
| ORCH-002 (God file) | Feature portability | Nothing |
| ORCH-003 (State machines) | Unified monitoring | Nothing |
| ORCH-004 (Dispatch duplication) | Consistent behavior | Nothing |
| ORCH-005 (Speculative exec) | Latency optimization | ORCH-001, ORCH-010 |
| ORCH-006 (Cumulative context) | Merge conflict reduction | ORCH-001 |
| ORCH-007 (Worktree integration) | Parallel execution | Nothing |
| ORCH-008 (Gate rung duplication) | Nothing | Nothing |
| ORCH-009 (TaskScheduler resume) | Reliable crash recovery | Nothing |
| ORCH-010 (Cost-aware ordering) | Efficient budget use | Nothing |
| ORCH-015 (Hardcoded recovery) | Custom failure handling | ORCH-017 (Steps) |
| ORCH-016 (Anti-pattern checks) | Code quality gates | Nothing |

---

## Sources

- `crates/roko-cli/src/orchestrate.rs` -- Import analysis (lines 1-210), feature inventory
- `crates/roko-runtime/src/pipeline_state.rs` -- PipelineStateV2 FSM, TOML config, failure recovery
- `crates/roko-runtime/src/effect_driver.rs` -- spawn_agent, rung_for_gate_name, affect modulation
- `crates/roko-runtime/src/task_scheduler.rs` -- next_batch(), file exclusion, failure cascade
- `crates/roko-runtime/src/process.rs` -- SupervisionStrategy defaults
- `crates/roko-orchestrator/src/dag.rs` -- UnifiedTaskDag, speculative execution
- `crates/roko-orchestrator/src/executor/state_machine.rs` -- PlanStateMachine transitions
- `crates/roko-cli/src/runner/event_loop.rs` -- max_concurrent defaults
- `tmp/solutions/runner/LESSONS.md` -- Mega-parity runner lessons (195 batches, 6 hours)
