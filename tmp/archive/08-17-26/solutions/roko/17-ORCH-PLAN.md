# Orchestration: Implementation Plan

Phased plan to unify the three runtimes into a single WorkflowEngine with
parallel DAG execution, worktree isolation, cumulative context handoff, and
the orchestrate.rs features that have never been ported.

Each phase is independently shippable. Later phases depend on earlier ones
but the system is usable after each phase.

---

## Phase 1: Parallel Execution (ORCH-001, ORCH-007)

**Objective**: Enable parallel task dispatch in WorkflowEngine with worktree
isolation.

**Duration estimate**: 3-4 days

### 1.1 Wire WorktreeManager into EffectServices

Add optional worktree support to the WorkflowEngine:

```rust
pub struct EffectServices {
    // existing fields...
    pub worktree_manager: Option<Arc<WorktreeManager>>,
    pub merge_queue: Option<Arc<MergeQueue>>,
}
```

When `worktree_manager` is Some, each parallel task gets its own worktree.
When None, all tasks share the single workdir (backward compatible).

**Files to modify**:
- `crates/roko-runtime/src/effect_driver.rs` -- add worktree allocation
- `crates/roko-runtime/src/workflow_engine.rs` -- pass worktree path to
  spawn_agent
- `crates/roko-runtime/Cargo.toml` -- add roko-orchestrator dependency

### 1.2 Concurrent Task Dispatch

Modify WorkflowEngine's run loop to dispatch multiple tasks from
TaskScheduler::next_batch() concurrently:

```rust
// Current: serial
for task_id in scheduler.next_batch() {
    let result = driver.spawn_agent(role, prompt, context).await;
    // handle result
}

// Target: concurrent
let batch = scheduler.next_batch();
let mut join_set = JoinSet::new();
for task_id in &batch {
    scheduler.mark_running(task_id);
    let driver = driver.clone();
    join_set.spawn(async move {
        driver.spawn_agent(role, prompt, context).await
    });
}
while let Some(result) = join_set.join_next().await {
    // handle result, mark completed/failed
}
```

### 1.3 Post-Task Merge

After each task completes in its worktree:
1. Run gate checks in the worktree
2. If gates pass, enqueue merge via MergeQueue
3. Wait for MergeQueue slot (file-overlap-aware serialization)
4. Merge worktree branch into integration branch
5. Run post-merge regression gate via PostMergeRunner
6. Report merge result back to TaskScheduler

### 1.4 Configuration

Add `max_parallel_tasks` to WorkflowConfig:

```toml
[workflow]
template = "standard"
max_parallel_tasks = 4    # 1 = serial (default)
worktree_isolation = true # false = shared workdir
```

### 1.5 Verification

- Unit test: TaskScheduler dispatches correct batches with file exclusion
- Integration test: 3 tasks in parallel with worktree isolation
- Regression: serial execution unchanged when max_parallel_tasks=1

---

## Phase 2: Context Handoff (ORCH-006)

**Objective**: Build cumulative context so each agent knows what prior agents
changed.

**Duration estimate**: 2-3 days

### 2.1 Cumulative Context Buffer

After each task completes:
1. Compute `git diff --stat` in the task's worktree
2. Extract function signatures added/modified (using roko-index if available)
3. Append to a per-plan cumulative context buffer

```rust
pub struct CumulativeContext {
    /// Files changed by completed tasks, with change summaries
    pub changes: Vec<TaskChangeSummary>,
    /// Total tokens used for context rendering
    pub token_count: usize,
}

pub struct TaskChangeSummary {
    pub task_id: String,
    pub files_changed: Vec<String>,
    pub diff_stat: String,            // e.g., "+45 -12"
    pub functions_added: Vec<String>,  // signature only
    pub functions_modified: Vec<String>,
}
```

### 2.2 Context Injection

Before dispatching each task, render the cumulative context as a prompt section:

```markdown
## What Changed Before You

Tasks completed in this plan before your task:

### T1: Wire compile gate
- `src/gate/compile.rs` (+45 -12)
  - Added: `fn run_compile_gate(workdir: &Path) -> GateResult`
  - Modified: `fn gate_pipeline(config: &GateConfig) -> Vec<Gate>`
- `src/lib.rs` (+3 -0)
  - Added: `pub mod gate;`

### T2: Add test fixtures
- `tests/fixtures/sample.toml` (+20 -0) [new file]
```

Token budget: cap cumulative context at 4000 tokens. Use signature-only views
for large files. Truncate oldest task summaries when exceeding budget.

### 2.3 Gate Failure Context

When a task fails gating and is retried, include structured gate feedback:

```markdown
## Previous Attempt Failed

Your previous attempt failed the `compile` gate (rung 0):

```
error[E0308]: mismatched types
  --> src/gate/compile.rs:47:12
```

Additionally, the `clippy` gate flagged:
- warning: unused variable `result` in `run_compile_gate`
```

This extends the existing `last_gate_failure` field in PipelineStateV2 with
structured per-gate breakdowns.

### 2.4 Verification

- Unit test: cumulative context renders correctly for 3 completed tasks
- Unit test: token budget truncation works
- Integration test: retry agent receives gate failure context

---

## Phase 3: Feature Extraction from orchestrate.rs (ORCH-002)

**Objective**: Extract the most valuable orchestrate.rs features into service
traits that WorkflowEngine can use.

**Duration estimate**: 5-7 days

### 3.1 Priority Feature Extraction Order

Based on impact and dependency analysis:

1. **Knowledge routing** -- `build_knowledge_routing_advice()` queries neuro
   store for context relevant to the current task. High impact: reduces
   agent confusion by providing prior knowledge.

2. **Episode logging** -- `EpisodeLogger` records agent turns, gate results,
   and outcomes. Required for the learning subsystem to function.

3. **Playbook queries** -- `PlaybookStore` provides replay context from
   similar past tasks. Reduces errors by 15-20% per the learning subsystem
   data.

4. **Error pattern queries** -- `ErrorPatternStore` matches current failures
   against known patterns. Enables targeted fix suggestions.

5. **Section effectiveness** -- `SectionEffectivenessRegistry` tracks which
   prompt sections improve outcomes. Enables progressive prompt optimization.

6. **Predictive calibration** -- `CalibrationTracker` provides accuracy and
   bias metrics per model/task-category. Informs model routing decisions.

7. **C-factor computation** -- `CFactorSummary` measures collective
   intelligence metrics. Useful for monitoring but not blocking.

8. **Custody audit chain** -- `CustodyLogger` tracks provenance. Required
   for compliance but not for functionality.

9. **Skill extraction** -- `SkillLibrary` extracts reusable patterns from
   successful tasks. Long-term value but not blocking.

10. **Dream consolidation** -- `DreamRunner` runs offline knowledge
    distillation. Already partially ported to Runner v2.

### 3.2 Service Trait Pattern

Each extracted feature becomes a trait in roko-core/src/foundation.rs:

```rust
/// Knowledge routing service.
pub trait KnowledgeRouter: Send + Sync {
    fn route(&self, task: &TaskSpec) -> Vec<KnowledgeEntry>;
}

/// Episode recording service.
pub trait EpisodeRecorder: Send + Sync {
    fn record_turn(&self, turn: &AgentTurn) -> Result<()>;
    fn record_gate(&self, result: &GateResult) -> Result<()>;
    fn finalize_episode(&self, outcome: &TaskOutcome) -> Result<()>;
}
```

Add these as optional fields in EffectServices:

```rust
pub struct EffectServices {
    // existing fields...
    pub knowledge_router: Option<Arc<dyn KnowledgeRouter>>,
    pub episode_recorder: Option<Arc<dyn EpisodeRecorder>>,
    pub playbook_store: Option<Arc<dyn PlaybookQuery>>,
    pub error_patterns: Option<Arc<dyn ErrorPatternQuery>>,
}
```

### 3.3 Implementation Strategy

For each feature:
1. Define the trait in roko-core/src/foundation.rs
2. Implement the trait using the existing orchestrate.rs code
3. Wire the implementation into EffectServices
4. Call from EffectDriver at the appropriate point
5. Test with the same inputs orchestrate.rs uses

Do NOT attempt to port all features at once. Extract one, verify it works
via WorkflowEngine, then extract the next.

---

## Phase 4: Configurable Failure Recovery (ORCH-015)

**Objective**: Replace hardcoded failure recovery with per-step, per-gate
configurable routing.

**Duration estimate**: 2-3 days

### 4.1 Failure Policy Config

```toml
[workflow]
template = "standard"

[workflow.failure]
# Default policy for all gates
default = { action = "autofix", max_attempts = 2 }

# Per-gate overrides
[workflow.failure.compile]
action = "autofix"
max_attempts = 3

[workflow.failure.test]
action = "reimplement"
max_attempts = 2

[workflow.failure.clippy]
action = "autofix"
max_attempts = 1

[workflow.failure.review]
action = "revise"
max_iterations = 2
fallback = "commit"  # commit anyway after max iterations
```

### 4.2 Failure Actions

| Action | Behavior |
|---|---|
| `autofix` | Spawn autofix agent with gate output |
| `reimplement` | Re-run implementation with failure context |
| `revise` | Re-run implementation with review findings |
| `escalate` | Switch to a stronger model and retry |
| `skip` | Mark gate as skipped, continue pipeline |
| `halt` | Stop pipeline, report failure |
| `replan` | Generate new tasks based on failure analysis |

### 4.3 Implementation

Replace the hardcoded match arms in PipelineStateV2::step() with a lookup
against the failure policy. The state machine remains pure -- it just reads
the policy from config instead of hardcoding the decision tree.

```rust
pub fn step(&mut self, input: PipelineInput) -> PipelineOutput {
    match (&self.phase, input) {
        (Phase::Gating, PipelineInput::GateFailed { gate, output }) => {
            let policy = self.config.failure_policy_for(&gate);
            match policy.action {
                FailureAction::AutoFix if self.autofix_attempts < policy.max_attempts => {
                    self.autofix_attempts += 1;
                    self.phase = Phase::AutoFixing;
                    PipelineOutput::SpawnAutoFixer { error_output: output }
                }
                FailureAction::Reimplement if self.iteration < policy.max_iterations => {
                    self.iteration += 1;
                    self.phase = Phase::Implementing;
                    PipelineOutput::SpawnImplementer { /* ... */ }
                }
                FailureAction::Skip => {
                    // Continue to next phase
                    self.advance_past_gates()
                }
                FailureAction::Escalate => {
                    PipelineOutput::EscalateModel { gate, output }
                }
                _ => {
                    self.phase = Phase::Halted { reason: format!("Gate '{gate}' exhausted") };
                    PipelineOutput::Halt { reason }
                }
            }
        }
        // ...
    }
}
```

---

## Phase 5: Speculative Execution (ORCH-005)

**Objective**: Wire the existing speculative execution infrastructure to
reduce critical path latency.

**Duration estimate**: 2 days

### 5.1 Trigger Condition

Speculatively dispatch a task when:
1. It is on the critical path (`dag.slack(task).is_zero()`)
2. Its dependencies are 80%+ complete (by count)
3. Current speculative cost is below `speculative_threshold_multiplier * budget`
4. The task is not already running or completed

### 5.2 Implementation

```rust
// In the event loop, after each task completion:
for task_id in dag.ready_tasks() {
    if dag.slack(&task_id).is_zero() && !speculative_running.contains(&task_id) {
        let deps = dag.deps_of(&task_id);
        let completed = deps.iter().filter(|d| dag.status(d).is_completed()).count();
        let total = deps.len();
        if total > 0 && (completed as f64 / total as f64) >= 0.8 {
            speculative_running.insert(task_id.clone());
            spawn_speculative(task_id, driver, worktree_manager).await;
        }
    }
}
```

### 5.3 Cancellation

If a dependency fails, cancel the speculative agent:
1. Send SIGTERM to the process (via ProcessSupervisor)
2. Clean up the worktree
3. Mark the speculative execution as cancelled in the snapshot
4. Track wasted cost in the learning subsystem

### 5.4 Cost Tracking

Record speculative execution outcomes:
- `speculative_hit`: dependency completed, speculative work was useful
- `speculative_miss`: dependency failed, speculative work was wasted
- `speculative_cost_usd`: total cost of speculative executions

Use hit/miss ratio to tune the 80% threshold adaptively.

---

## Phase 6: Adaptive Parallelism

**Objective**: Dynamically adjust concurrency based on runtime signals.

**Duration estimate**: 2 days

### 6.1 Signals

| Signal | Source | Effect |
|---|---|---|
| Error rate > 30% | TaskScheduler failure count | Reduce max_parallel by 50% |
| Error rate < 10% | TaskScheduler completion count | Increase max_parallel by 1 |
| Disk < 5GB | `statvfs` check | Pause dispatch until space freed |
| API rate limit (429) | Model caller response | Back off, reduce max_parallel |
| All gates passing | Wave gate results | Safe to increase parallelism |
| Merge conflicts > 20% | MergeQueue metrics | Reduce max_parallel, tighten overlap |

### 6.2 Implementation

```rust
pub struct AdaptiveParallelism {
    base_max_parallel: usize,
    current_max_parallel: usize,
    error_window: VecDeque<bool>,  // recent task outcomes
    window_size: usize,
}

impl AdaptiveParallelism {
    pub fn adjust(&mut self, task_succeeded: bool) -> usize {
        self.error_window.push_back(task_succeeded);
        if self.error_window.len() > self.window_size {
            self.error_window.pop_front();
        }
        let error_rate = self.error_window.iter()
            .filter(|&&ok| !ok).count() as f64
            / self.error_window.len() as f64;

        if error_rate > 0.3 {
            self.current_max_parallel = (self.current_max_parallel / 2).max(1);
        } else if error_rate < 0.1 && self.current_max_parallel < self.base_max_parallel {
            self.current_max_parallel += 1;
        }
        self.current_max_parallel
    }
}
```

Integrate into WorkflowEngine's run loop: pass current_max_parallel to
TaskScheduler before each `next_batch()` call.

---

## Phase 7: Anti-Pattern Pre-Gate (ORCH-016)

**Objective**: Add fast grep-based anti-pattern checks as a pre-gate step.

**Duration estimate**: 1-2 days

### 7.1 Anti-Pattern Registry

Port the mega-parity runner's AP checks to a structured registry:

```rust
pub struct AntiPatternCheck {
    pub id: String,           // "AP-1"
    pub name: String,         // "Silent pass gate"
    pub pattern: Regex,       // compiled regex
    pub file_glob: String,    // "*.rs"
    pub exclude_test: bool,   // skip test files
    pub severity: Severity,   // Error | Warning
    pub message: String,      // human-readable explanation
}
```

### 7.2 Pre-Gate Integration

Run AP checks before any compilation gate. They execute in milliseconds
(grep-based, no cargo involved) and catch structural LLM mistakes:

```rust
impl GateRunner for AntiPatternGate {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
        let mut verdicts = Vec::new();
        for check in &self.checks {
            let matches = search_files(&config.workdir, &check.pattern, &check.file_glob);
            if !matches.is_empty() {
                verdicts.push(GateVerdict {
                    gate_name: format!("ap:{}", check.id),
                    passed: check.severity != Severity::Error,
                    output: format_matches(&matches, &check.message),
                    duration_ms: 0,
                });
            }
        }
        Ok(GateReport { verdicts })
    }
}
```

### 7.3 Per-Task Exemptions

Allow batches/tasks to exempt specific AP checks:

```toml
[[task]]
id = "T5"
title = "Add default server config"
ap_exempt = ["AP-10"]  # legitimately contains localhost
```

---

## Phase 8: Cost-Aware Scheduling (ORCH-010)

**Objective**: Prioritize task dispatch by expected cost efficiency and
dependency impact.

**Duration estimate**: 1-2 days

### 8.1 Priority Score

```rust
fn priority_score(task: &SchedulableTask, dag: &UnifiedTaskDag) -> f64 {
    let critical = if dag.slack(&task.id).is_zero() { 2.0 } else { 0.0 };
    let dependents = dag.dependents_of(&task.id).len() as f64 * 0.5;
    let cost_inverse = 1.0 / estimated_cost(task).max(0.01) * 0.3;
    critical + dependents + cost_inverse
}
```

### 8.2 Integration

Replace TaskScheduler's insertion-order traversal with priority-sorted:

```rust
pub fn next_batch(&self) -> Vec<&str> {
    let mut candidates: Vec<_> = self.status.iter()
        .filter(|(_, s)| **s == TaskStatus::Ready)
        .map(|(id, _)| (id.as_str(), self.priority_score(id)))
        .collect();
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    // then apply file exclusion and parallelism cap
}
```

---

## Phase 9: orchestrate.rs Retirement

**Objective**: Remove the legacy orchestrator after all features are ported.

**Duration estimate**: 1-2 days (after all features ported)

### 9.1 Prerequisites

All features in the AUDIT.md "Features Only in Dead Code" table must be
either ported to WorkflowEngine or explicitly marked as deprecated.

### 9.2 Steps

1. Remove `orchestrate.rs` from `crates/roko-cli/src/`
2. Remove legacy `roko plan run` entry point that calls orchestrate
3. Remove legacy dispatch helpers that are only used by orchestrate.rs
4. Remove orphaned imports and utility functions
5. Run `cargo test --workspace` to verify nothing breaks
6. Update CLAUDE.md to remove references to orchestrate.rs

---

## Phase Summary

| Phase | Effort | Blocks | Delivers |
|---|---|---|---|
| 1. Parallel execution | 3-4 days | Nothing | 4-10x faster plan execution |
| 2. Context handoff | 2-3 days | Phase 1 | 30-50% fewer merge conflicts |
| 3. Feature extraction | 5-7 days | Nothing | Knowledge routing, episodes, playbooks |
| 4. Failure recovery | 2-3 days | Nothing | Per-gate failure policy |
| 5. Speculative execution | 2 days | Phase 1 | Reduced critical path latency |
| 6. Adaptive parallelism | 2 days | Phase 1 | Self-tuning concurrency |
| 7. Anti-pattern pre-gate | 1-2 days | Nothing | Fast structural quality checks |
| 8. Cost-aware scheduling | 1-2 days | Phase 1 | Budget-efficient task ordering |
| 9. orchestrate.rs retirement | 1-2 days | Phases 1-8 | 22K LOC removed |

**Critical path**: Phase 1 -> Phase 2 -> Phase 5 (7-9 days for the core
parallel execution with context handoff and speculative dispatch).

**Independent work**: Phases 3, 4, 7, 8 can proceed in parallel with the
critical path.

---

## Verification Plan

### Per-Phase Tests

Each phase includes unit tests and integration tests as described in its
section. Key integration tests:

1. **Parallel execution**: 3 tasks with file-overlap serialization complete
   without data corruption
2. **Context handoff**: Agent receives cumulative diff from prior tasks
3. **Feature extraction**: Knowledge routing provides relevant context
4. **Failure recovery**: Compile failure triggers autofix, test failure
   triggers reimplementation
5. **Speculative execution**: Speculative task is cancelled when dependency fails
6. **Adaptive parallelism**: Error rate spike reduces concurrency
7. **Anti-pattern pre-gate**: AP-7 (mutex across await) is detected
8. **Cost-aware scheduling**: High-dependency tasks run before isolated tasks

### End-to-End Validation

After all phases:
```bash
# Create a plan with 10 tasks, 3 waves, file overlaps
cargo run -p roko-cli -- plan create test-parallel

# Run with parallel execution
cargo run -p roko-cli -- plan run plans/test-parallel --parallel 4

# Verify:
# - Tasks in wave 0 ran in parallel
# - File-overlapping tasks serialized
# - Cumulative context injected
# - Gates ran at wave boundaries
# - Post-merge regression passed
# - Total time < 4x sequential (with 4 parallel slots)
```

---

## Sources

- `crates/roko-runtime/src/workflow_engine.rs` -- WorkflowEngine architecture
- `crates/roko-runtime/src/pipeline_state.rs` -- PipelineStateV2 state machine
- `crates/roko-runtime/src/effect_driver.rs` -- EffectDriver side effects
- `crates/roko-runtime/src/task_scheduler.rs` -- TaskScheduler DAG resolver
- `crates/roko-runtime/src/process.rs` -- ProcessSupervisor
- `crates/roko-orchestrator/src/dag.rs` -- UnifiedTaskDag, waves, CPM
- `crates/roko-orchestrator/src/worktree.rs` -- WorktreeManager
- `crates/roko-orchestrator/src/merge_queue.rs` -- MergeQueue
- `crates/roko-orchestrator/src/replan.rs` -- ReplanStrategy
- `crates/roko-orchestrator/src/repair.rs` -- RepairEngine
- `crates/roko-cli/src/orchestrate.rs` -- Legacy features to extract
- `tmp/solutions/runner/LESSONS.md` -- Mega-parity runner patterns and lessons
- `ISSUES.md` -- Issue cross-references (ORCH-001 through ORCH-021)
