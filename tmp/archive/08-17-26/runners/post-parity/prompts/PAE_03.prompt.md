# PAE_03: Add post-merge regression gate

## Task
After PlanMerger completes a git merge, run a lightweight regression gate (compile + test) to catch merge-induced regressions before proceeding.

## Runner Context
Runner PAE (Gate Pipeline Completeness), batch 3 of 4. Depends on PAE_01 (GateService).

## Problem
GP-3 anti-pattern: "Merge without verify." The merge queue accepts completed tasks and merges their branches, but there is no post-merge verification. A task that passed gates on its branch may introduce regressions when merged (conflicts, import collisions, dependency issues).

## Current Code

**PlanMerger** — `crates/roko-cli/src/runner/merge.rs:48-51`:
```rust
pub struct PlanMerger { /* wraps MergeQueue */ }
```
Methods: `submit()`, `drain_next()`. After drain_next, the merge is done but not verified.

## Exact Changes

### Step 1: Add post-merge gate call in event loop

After `merger.drain_next()` completes a merge:

```rust
// After successful merge:
if let Some(merged_task) = merger.drain_next().await? {
    debug!(task = %merged_task.id, "merge complete — running regression gate");

    // Lightweight post-merge gate: compile + test only (not full pipeline)
    let gate_service = GateService::new(&workdir)
        .with_gates(vec!["compile", "test"]);

    match gate_service.run_all().await {
        Ok(verdicts) if verdicts.iter().all(|v| v.passed) => {
            debug!(task = %merged_task.id, "post-merge regression gate passed");
        }
        Ok(verdicts) => {
            let failed: Vec<_> = verdicts.iter()
                .filter(|v| !v.passed)
                .map(|v| v.gate_name.as_str())
                .collect();
            warn!(
                task = %merged_task.id,
                failed_gates = ?failed,
                "post-merge regression — reverting merge"
            );
            // Revert the merge
            revert_merge(&workdir, &merged_task).await?;
            // Re-queue the task for retry
            task_dag.requeue(&merged_task.id);
        }
        Err(err) => {
            warn!(%err, task = %merged_task.id, "post-merge gate failed to run — proceeding");
        }
    }
}
```

### Step 2: Add revert_merge helper

```rust
async fn revert_merge(workdir: &Path, task: &TaskDef) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(workdir)
        .output()
        .await?;

    if !output.status.success() {
        // merge --abort may fail if we're past the merge commit
        // In that case, reset to pre-merge HEAD
        let output = tokio::process::Command::new("git")
            .args(["reset", "--hard", "HEAD~1"])
            .current_dir(workdir)
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!("failed to revert merge for task {}", task.id));
        }
    }
    Ok(())
}
```

### Step 3: Make post-merge gate configurable

```rust
// In roko.toml or RunConfig:
pub post_merge_gate: Option<bool>,  // default true
pub post_merge_gates: Option<Vec<String>>,  // default ["compile", "test"]
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (post-merge gate call, revert on failure)
- `crates/roko-cli/src/runner/merge.rs` (expose merge revert capability if needed)

## Read-Only Context
- `crates/roko-gate/src/gate_service.rs` (GateService from PAE_01)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Post-merge regression gate runs after every successful merge
- Failed regression → merge reverted, task re-queued
- Only compile + test gates (lightweight, not full pipeline)
- Configurable via `post_merge_gate` config
- Gate failure doesn't crash the pipeline (warn + revert + continue)

## Do NOT
- Run the full 7-rung pipeline post-merge (too expensive)
- Skip regression on first merge (every merge needs verification)
- Change PlanMerger internals
