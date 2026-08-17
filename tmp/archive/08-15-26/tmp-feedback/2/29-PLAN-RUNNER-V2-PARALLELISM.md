# Plan Runner v2: max_parallel Ignored, Single Agent at a Time

## Problem

`tasks.toml` supports `max_parallel` in the `[meta]` section, but runner v2 ignores it.
Only one agent runs per plan at a time, regardless of the parallelism setting. The DAG
executor in `roko-orchestrator` has parallel execution support but the runner v2 wrapper
serializes everything.

## Root Cause

**File:** `crates/roko-orchestrator/src/runner.rs`

```rust
impl PlanRunner {
    pub async fn run_all(&mut self) -> Result<()> {
        for task in &self.tasks {
            // Sequential: waits for each task to complete before starting next
            let result = self.dispatch_task(task).await?;
            self.gate_check(task, &result).await?;
        }
        Ok(())
    }
}
```

The runner iterates through tasks sequentially. The `meta.max_parallel` field is parsed
by `task_parser.rs` but never read by the runner.

### What exists but isn't connected:

**File:** `crates/roko-orchestrator/src/dag_executor.rs`

```rust
pub struct DagExecutor {
    pub max_parallel: usize,  // ← supports parallelism
    // ...
}

impl DagExecutor {
    pub async fn execute_all(&mut self) -> Result<()> {
        // Uses tokio::JoinSet to run up to max_parallel tasks concurrently
        // Respects DAG dependencies (only starts tasks whose deps are done)
    }
}
```

The DAG executor supports parallel execution with dependency awareness. But runner v2
doesn't use it — it has its own sequential loop.

## Fix

### Option A: Wire runner v2 through DagExecutor (~20 min)

**File:** `crates/roko-orchestrator/src/runner.rs`

Replace the sequential loop with DagExecutor:
```rust
impl PlanRunner {
    pub async fn run_all(&mut self) -> Result<()> {
        let max_parallel = self.meta.max_parallel.unwrap_or(1);
        let mut executor = DagExecutor::new(
            self.tasks.clone(),
            max_parallel,
            |task| self.dispatch_task(task),
        );
        executor.execute_all().await
    }
}
```

### Option B: Add parallelism directly to runner v2 (~15 min)

```rust
impl PlanRunner {
    pub async fn run_all(&mut self) -> Result<()> {
        let max_parallel = self.meta.max_parallel.unwrap_or(1);
        let semaphore = Arc::new(Semaphore::new(max_parallel));
        let mut join_set = JoinSet::new();

        for task in &self.ready_tasks() {
            let permit = semaphore.clone().acquire_owned().await?;
            let task = task.clone();
            join_set.spawn(async move {
                let result = dispatch_task(&task).await;
                drop(permit);
                (task, result)
            });

            // Check for completed tasks and run gate checks
            while let Some(completed) = join_set.try_join_next() {
                let (task, result) = completed?;
                self.gate_check(&task, &result?).await?;
                self.mark_complete(&task);
            }
        }

        // Wait for remaining
        while let Some(completed) = join_set.join_next().await {
            let (task, result) = completed?;
            self.gate_check(&task, &result?).await?;
        }
        Ok(())
    }
}
```

## Impact

Currently a 10-task plan takes 10x the time it should because tasks run sequentially even
when they're independent. A `max_parallel = 3` setting would let 3 independent tasks run
simultaneously, cutting execution time significantly.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-orchestrator/src/runner.rs` | Add parallel execution using max_parallel |

## Priority

**P1** — Plan execution speed is directly visible to users. A plan with 10 independent tasks
currently takes 10x longer than necessary.
