# 06 — TaskScheduler: Wire DAG Execution Into the Engine

> Phase 1.2 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Sibling plan to 05 and 07.

---

## Status (2026-05-01)

**PARTIAL.** Library exists, fully tested. Not used by any live caller.

**What's done:**

- `roko_runtime::task_scheduler::TaskScheduler` — `crates/roko-runtime/src/task_scheduler.rs`
- DAG dependency tracking via `depends_on`
- File-overlap serialization (tasks touching same files cannot run concurrently)
- Skip propagation: `mark_failed` → `skip_dependents` BFS
- Public API:
  - `TaskScheduler::new(tasks: Vec<SchedulableTask>, max_parallel: usize) -> Self`
  - `ready_tasks(&self) -> Vec<&str>`
  - `next_batch(&self) -> Vec<&str>` (filtered by file overlap + max_parallel)
  - `mark_running(&mut self, task_id: &str)`
  - `mark_completed(&mut self, task_id: &str)`
  - `mark_failed(&mut self, task_id: &str, error: String)`
  - `is_done(&self) -> bool`
  - `task_status(&self, task_id: &str) -> Option<&TaskStatus>`
  - `summary(&self) -> (pending, ready, running, completed, failed, skipped)`

**What's not:**

- `WorkflowEngine` does not use `TaskScheduler` (single-prompt only today)
- Wave computation is implicit — no `compute_waves()` API for BFS layering
- Retry cooldown / exponential backoff per failed task — not implemented
- DAG cycle detection happens at `task_parser` level, not on scheduler construction
- Plan-level dependencies (cross-plan `depends_on: ["other-plan:t3"]`) not modeled — flat task graph only
- No equivalent of `roko-orchestrator/src/dag.rs::UnifiedTaskDag` features (priority, hints)

---

## Goal

`TaskScheduler` is the canonical DAG executor consumed by `WorkflowEngine` for `WorkflowTemplate::PlanExecution` runs. Every plan run (`roko plan run`, HTTP `/api/plans/{id}/run`, ACP plan-mode prompts) goes through this scheduler. Cross-plan dependencies are first-class. Retry cooldown is implemented. Wave computation is exposed.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#3 Build Another Runtime** — today `runner/event_loop.rs` and `roko-orchestrator/src/dag.rs::UnifiedTaskDag` and `roko-orchestrator/src/executor/mod.rs::ParallelExecutor` all do versions of DAG dispatch. Pick one.
- **#7 Copy-Paste Between Runtimes** — three DAG schedulers means three places to fix the same bug.

---

## Existing Code — Read These First

- `crates/roko-runtime/src/task_scheduler.rs` (current canonical)
- `crates/roko-orchestrator/src/dag.rs` (legacy `UnifiedTaskDag`, ~92K LOC) — reference for richer features
- `crates/roko-orchestrator/src/executor/mod.rs` (legacy `ParallelExecutor`) — reference for execution loop
- `crates/roko-cli/src/runner/task_dag.rs` (legacy runner DAG)
- `crates/roko-cli/src/task_parser.rs` — `TasksFile`, `TaskDef` parsing + cycle detection

```rust
// crates/roko-runtime/src/task_scheduler.rs
pub struct TaskScheduler {
    tasks: HashMap<String, SchedulableTask>,
    statuses: HashMap<String, TaskStatus>,
    max_parallel: usize,
}

pub struct SchedulableTask {
    pub id: String,
    pub depends_on: Vec<String>,
    pub files: Vec<PathBuf>,             // for file-overlap serialization
    pub estimated_minutes: Option<u32>,
}

pub enum TaskStatus {
    Pending, Ready, Running, Completed, Failed { error: String }, Skipped,
}
```

---

## Implementation Steps

### Step 1 — Add wave computation API

```rust
impl TaskScheduler {
    /// BFS-layer the DAG: wave 0 = no deps, wave 1 = depends only on wave 0, etc.
    pub fn compute_waves(&self) -> Vec<Vec<String>> {
        let mut waves: Vec<Vec<String>> = Vec::new();
        let mut placed: HashSet<String> = HashSet::new();

        loop {
            let next_wave: Vec<String> = self.tasks.values()
                .filter(|t| !placed.contains(&t.id))
                .filter(|t| t.depends_on.iter().all(|d| placed.contains(d)))
                .map(|t| t.id.clone())
                .collect();
            if next_wave.is_empty() { break; }
            placed.extend(next_wave.iter().cloned());
            waves.push(next_wave);
        }
        waves
    }

    pub fn current_wave(&self) -> u32 {
        // Lowest wave index containing a non-completed task
        for (i, wave) in self.compute_waves().iter().enumerate() {
            if wave.iter().any(|id| !matches!(self.statuses[id], TaskStatus::Completed)) {
                return i as u32;
            }
        }
        u32::MAX
    }
}
```

### Step 2 — Add retry cooldown

```rust
pub struct TaskRetryState {
    pub attempts: u32,
    pub last_failure_ms: u64,
    pub backoff_ms: u64,                    // exponential: 1000, 2000, 4000, ...
}

impl TaskScheduler {
    pub fn ready_tasks_at(&self, now_ms: u64) -> Vec<&str> {
        self.tasks.values()
            .filter(|t| matches!(self.statuses[&t.id], TaskStatus::Ready | TaskStatus::Failed { .. }))
            .filter(|t| t.depends_on.iter().all(|d| matches!(self.statuses[d], TaskStatus::Completed)))
            .filter(|t| {
                if let Some(retry) = self.retries.get(&t.id) {
                    now_ms - retry.last_failure_ms >= retry.backoff_ms
                } else { true }
            })
            .map(|t| t.id.as_str())
            .collect()
    }

    pub fn mark_failed(&mut self, task_id: &str, error: String) {
        let retry = self.retries.entry(task_id.into()).or_insert_with(|| TaskRetryState {
            attempts: 0, last_failure_ms: 0, backoff_ms: 1000,
        });
        retry.attempts += 1;
        retry.last_failure_ms = now_ms();
        retry.backoff_ms = (retry.backoff_ms * 2).min(60_000);   // cap at 60s

        if retry.attempts >= self.config.max_retries {
            self.statuses.insert(task_id.into(), TaskStatus::Failed { error });
            self.skip_dependents(task_id);
        } else {
            self.statuses.insert(task_id.into(), TaskStatus::Ready);   // re-queue
        }
    }
}
```

### Step 3 — Add cross-plan dependencies

Today `depends_on` is just `Vec<String>` of task IDs within the same plan. Mori-style cross-plan deps look like `"00-foundation:t3"`. Support this:

```rust
pub struct DependencyRef {
    pub plan_id: Option<String>,            // None = same plan
    pub task_id: String,
}

impl DependencyRef {
    pub fn parse(s: &str) -> Self {
        if let Some((plan, task)) = s.split_once(':') {
            DependencyRef { plan_id: Some(plan.into()), task_id: task.into() }
        } else {
            DependencyRef { plan_id: None, task_id: s.into() }
        }
    }
}

pub struct SchedulableTask {
    pub plan_id: String,            // NEW
    pub id: String,
    pub depends_on: Vec<DependencyRef>,
    pub files: Vec<PathBuf>,
    pub estimated_minutes: Option<u32>,
}
```

For multi-plan execution, `TaskScheduler` accepts tasks from multiple plans; `DependencyRef.plan_id` resolves cross-plan edges.

### Step 4 — Wire `TaskScheduler` into `WorkflowEngine`

**File:** `crates/roko-runtime/src/workflow_engine.rs`

When `WorkflowConfig::template == WorkflowTemplate::PlanExecution(_)`:

```rust
// in run_with_cancel for PlanExecution branch
let scheduler = TaskScheduler::new(load_tasks(&config)?, config.max_concurrent_tasks);
let mut pipeline = PipelineStateV2::new_for_plan(config.workflow.clone(), scheduler);
let mut input = PipelineInput::Start;

loop {
    let actions = pipeline.step_actions(input);
    let outcomes = futures::future::join_all(
        actions.into_iter().map(|action| driver.execute(action))
    ).await;

    // Convert outcomes → next inputs
    for outcome in outcomes {
        match outcome {
            EffectOutcome::AgentDone { agent_id, .. } => {
                pipeline.scheduler_mut().mark_completed(&agent_id);
                input = PipelineInput::TaskCompleted { task_id: agent_id, output: ... };
                pipeline.step(input);
            }
            EffectOutcome::Failed { error, agent_id, .. } => {
                pipeline.scheduler_mut().mark_failed(&agent_id, error.clone());
                input = PipelineInput::TaskFailed { task_id: agent_id, error: ... };
                pipeline.step(input);
            }
            // ...
        }
    }

    persistence.checkpoint(&snapshot_from(&pipeline)).await?;

    if pipeline.is_terminal() { break; }
}
```

The pipeline state machine **owns** the scheduler — accessed via `pipeline.scheduler_mut()`. This matches the FSM-owns-state principle (anti-pattern #4: don't put the scheduler in the driver).

### Step 5 — Migrate `roko plan run` default path

**File:** `crates/roko-cli/src/commands/plan.rs`

Today this calls `runner::event_loop::run`. After this plan, route to `WorkflowEngine`:

```rust
// crates/roko-cli/src/commands/plan.rs
pub async fn run_plan(opts: PlanRunOpts) -> Result<()> {
    let services = ServiceFactory::for_plan(&opts.workdir, &opts.config).await?;
    let engine = WorkflowEngine::new(services);

    let plan_dirs = discover_plans(&opts.plans_dir)?;
    let tasks = load_all_tasks(&plan_dirs)?;

    let cfg = WorkflowRunConfig {
        prompt: format!("plan_run::{}", opts.plans_dir.display()),
        workdir: opts.workdir,
        workflow: WorkflowConfig {
            template: WorkflowTemplate::PlanExecution(PlanExecutionConfig {
                max_concurrent_tasks: opts.max_concurrent.unwrap_or(1),
                task_timeout_secs: 600,
                gate_template: GateTemplate::Standard,
                merge_strategy: opts.merge_strategy,
                doc_revision: opts.docs,
                replan_max_per_plan: opts.replan_max.unwrap_or(3),
            }),
            ..Default::default()
        },
        enabled_gates: opts.gates,
        shell_gates: opts.shell_gates,
        commit_prefix: opts.commit_prefix,
        plan_tasks: Some(tasks),         // NEW field
    };

    let report = engine.run(cfg).await?;
    print_run_report(&report);
    Ok(())
}
```

Behind a transition flag `--use-event-loop` to keep the legacy path callable for one release.

### Step 6 — Delete `runner/event_loop.rs`, `runner/task_dag.rs`, `roko-orchestrator/src/dag.rs::UnifiedTaskDag` (deferred to plan 12)

This plan does NOT delete those — plan 12 (Retirement) handles deletion after the migration soaks. But this plan must ensure the new path covers all features so deletion is safe:

| Legacy Feature | New Location |
|---|---|
| Wave dispatch | `TaskScheduler::compute_waves` (Step 1) |
| File-overlap serialization | already in `TaskScheduler::next_batch` |
| Retry cooldown | `TaskRetryState` (Step 2) |
| Cross-plan deps | `DependencyRef` (Step 3) |
| Speculative execution | **NOT migrated** — was never load-tested in production. Document as removed. |
| Worktree isolation | `MergeStrategy::Worktree` (Step 4) drives `WorktreeManager` from `EffectDriver` |
| Snapshot resume | via `PersistenceService` (plan 04) |

Anything not covered → file an issue + add to plan 12 as "blocker for deletion".

### Step 7 — Tests

```rust
#[tokio::test]
async fn diamond_dag_executes_in_correct_order() {
    let scheduler = TaskScheduler::new(vec![
        task("A", &[], &[]),
        task("B", &["A"], &["src/b.rs"]),
        task("C", &["A"], &["src/c.rs"]),
        task("D", &["B", "C"], &[]),
    ], 4);
    assert_eq!(scheduler.compute_waves().len(), 3);
    assert_eq!(scheduler.compute_waves()[0], vec!["A"]);
    assert_eq!(scheduler.compute_waves()[1].iter().collect::<HashSet<_>>(),
               ["B", "C"].iter().collect());
    assert_eq!(scheduler.compute_waves()[2], vec!["D"]);
}

#[tokio::test]
async fn file_overlap_serializes() {
    let scheduler = TaskScheduler::new(vec![
        task("A", &[], &["src/x.rs"]),
        task("B", &[], &["src/x.rs"]),       // overlap
        task("C", &[], &["src/y.rs"]),
    ], 4);
    let batch = scheduler.next_batch();
    assert_eq!(batch.len(), 2);              // A or B (one), and C
}

#[tokio::test]
async fn retry_backoff_exponential() {
    let mut scheduler = TaskScheduler::new(vec![task("A", &[], &[])], 1)
        .with_max_retries(5);
    let now = 0;
    scheduler.mark_failed("A", "boom".into());
    assert!(scheduler.ready_tasks_at(now + 999).is_empty());     // backoff
    assert!(!scheduler.ready_tasks_at(now + 1001).is_empty());   // ready after 1s
    scheduler.mark_failed("A", "boom".into());
    assert!(scheduler.ready_tasks_at(now + 1500).is_empty());    // 2s now
}

#[tokio::test]
async fn cross_plan_dependency() {
    let s = TaskScheduler::new(vec![
        SchedulableTask { plan_id: "p1".into(), id: "t1".into(), depends_on: vec![],
                          files: vec![], estimated_minutes: None },
        SchedulableTask { plan_id: "p2".into(), id: "t2".into(),
                          depends_on: vec![DependencyRef::parse("p1:t1")],
                          files: vec![], estimated_minutes: None },
    ], 2);
    assert_eq!(s.ready_tasks(), vec!["t1"]);
    s.mark_completed("t1");
    assert_eq!(s.ready_tasks(), vec!["t2"]);
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding a "fast path" scheduler for HTTP plans | One scheduler |
| #7 Copy-paste | Duplicating wave logic in `WorkflowEngine` instead of calling `compute_waves()` | All callers go through `TaskScheduler` |
| #4 Wrong layer | Putting retry cooldown in the driver | Cooldown is scheduling concern; lives in `TaskScheduler` |

---

## Things NOT To Do

1. **Don't make `TaskScheduler` async.** It's pure state management. The driver awaits effects; the scheduler is sync.
2. **Don't store `Arc<dyn ModelCaller>` inside scheduler.** It must remain a pure data structure.
3. **Don't preserve speculative execution.** It's complex, never load-tested, and the audit (doc 15 § 7) says no live caller uses it. Document removal.
4. **Don't auto-detect file overlaps from edit history.** The `files: Vec<PathBuf>` field is **declarative** — task authors specify; the scheduler trusts.
5. **Don't lower `max_concurrent_tasks` default below 1.** Default 1 = serial = matches today's behavior; opting into > 1 is explicit.
6. **Don't merge `task_dag.rs` and `task_scheduler.rs` files.** Keep parsing (in `roko-cli/src/task_parser.rs`) separate from scheduling.
7. **Don't allow cyclic graphs.** `TaskScheduler::new` returns `Result` — fail with a typed `Cycle(Vec<String>)` error if `depends_on` forms a cycle.

---

## Tests / Proof Criteria

```bash
# 1. Single canonical scheduler
rg 'pub struct (TaskScheduler|UnifiedTaskDag|ParallelExecutor)' crates/ --type rust
# expected: only TaskScheduler in roko-runtime
# (UnifiedTaskDag + ParallelExecutor exist until plan 12 — flag if still used by live paths)

# 2. WorkflowEngine uses scheduler
rg 'TaskScheduler' crates/roko-runtime/src/workflow_engine.rs
# expected: 1+ usage

# 3. Plan run command goes through WorkflowEngine
rg 'WorkflowEngine|TaskScheduler' crates/roko-cli/src/commands/plan.rs
# expected: WorkflowEngine usage; no direct event_loop::run unless behind --use-event-loop flag
```

Functional proofs:

- [ ] All 4 unit tests above pass
- [ ] `roko plan run examples/diamond-plan` executes A → {B, C} → D in correct order
- [ ] `roko plan run examples/cross-plan-deps` (with two plans) honors `depends_on: ["00-foo:t3"]`
- [ ] Setting `max_parallel = 4` in tasks.toml runs 4 independent tasks concurrently
- [ ] Failed task with `max_retries = 3` retries 3x with backoff, then marks dependents Skipped
- [ ] Existing `roko plan run` integration tests (in `crates/roko-cli/tests/`) pass against the new path

---

## Dependencies

- **Plan 05 (PipelineState multi-task)** — must land together; the FSM owns the scheduler
- **Plan 04 (PersistenceService)** — needed for crash-safe multi-task runs
- **Plan 07 (EffectDriver completion)** — needs to handle multi-task action variants

---

## Estimated Effort

**M.** ~1-1.5 weeks.

- Step 1 (waves) — S (1 day)
- Step 2 (retry cooldown) — S (1 day)
- Step 3 (cross-plan deps) — M (2 days, parser updates)
- Step 4 (engine wiring) — M (2-3 days)
- Step 5 (plan command migration) — S (1 day)
- Step 6 (audit feature parity) — S (half day)
- Step 7 (tests) — S (1 day)
