# Plan Generation: Graph Engine Stub, Double-Generate, No Validation

## Problem

Three bugs in the plan generation and execution path:

1. **Graph Engine is the default but is a no-op stub** — `roko plan run` without feature flags
   routes to `TaskExecutorCell` which does nothing
2. **`roko develop` double-generates the plan** — generates once in `develop.rs`, then again
   when `do_cmd` detects an existing plan
3. **`roko plan generate` has no TOML validation** — agent output that isn't valid tasks.toml
   is written to disk silently

## Root Cause

### A. Graph Engine stub is default

**File:** `crates/roko-orchestrator/src/graph_engine.rs`

`TaskExecutorCell::execute()` is a stub:
```rust
impl TaskExecutorCell {
    pub async fn execute(&self, _task: &Task) -> Result<TaskResult> {
        Ok(TaskResult::success("stub"))  // ← always succeeds, does nothing
    }
}
```

**File:** `crates/roko-cli/src/commands/plan.rs`

```rust
// Default path (no feature flag):
let engine = GraphEngine::new(tasks);
engine.execute_all().await?;  // ← runs stubs, prints "success" for every task

// Runner v2 path (requires `legacy-runner-v2` feature flag):
#[cfg(feature = "legacy-runner-v2")]
{
    let runner = PlanRunner::new(tasks, config);
    runner.run_all().await?;  // ← actually dispatches agents
}
```

The feature flag name is misleading: `legacy-runner-v2` is actually the **working** path,
while the "modern" graph engine is the stub.

### B. `roko develop` double-generates

**File:** `crates/roko-cli/src/commands/develop.rs`

```rust
pub async fn run(prompt: &str, opts: &DevelopOpts) -> Result<()> {
    // Step 1: Generate plan
    let plan_dir = plan_generate(prompt).await?;  // ← generates tasks.toml

    // Step 2: Hand off to do_cmd
    do_cmd::run(&DoOpts {
        prompt: prompt.to_string(),
        plan: Some(plan_dir.clone()),  // ← passes existing plan
        ..
    }).await?;
}
```

But `do_cmd::run()` calls `run_complex_path()` which also generates a plan if one doesn't
match the prompt exactly, resulting in a second plan generation. The first plan is ignored.

### C. No TOML validation on generated plans

**File:** `crates/roko-cli/src/commands/plan.rs`

```rust
pub async fn cmd_generate(prompt: &str) -> Result<PathBuf> {
    let agent_output = agent_exec::run(&options).await?;
    let tasks_toml = extract_toml_block(&agent_output.text);
    std::fs::write(&toml_path, &tasks_toml)?;  // ← written without parsing
    Ok(plan_dir)
}
```

If the agent output contains invalid TOML (wrong field names, missing required fields,
bad syntax), it's written to disk. The next `plan run` will fail with a confusing parse error
far from the generation step.

## Fix

### Fix 1: Make runner v2 the default (~5 min)

**File:** `crates/roko-cli/src/commands/plan.rs`

Remove the feature gate. Make `PlanRunner` the default path. Remove or deprecate `GraphEngine`.

```rust
// Before:
#[cfg(feature = "legacy-runner-v2")]
let runner = PlanRunner::new(tasks, config);
#[cfg(not(feature = "legacy-runner-v2"))]
let engine = GraphEngine::new(tasks);

// After:
let runner = PlanRunner::new(tasks, config);
runner.run_all().await?;
```

### Fix 2: Fix develop double-generation (~10 min)

**File:** `crates/roko-cli/src/commands/develop.rs`

Pass a flag to `do_cmd::run()` indicating the plan already exists and should not be regenerated.
Or: have `develop.rs` call `plan::cmd_run()` directly instead of going through `do_cmd`.

### Fix 3: Validate generated TOML before writing (~10 min)

**File:** `crates/roko-cli/src/commands/plan.rs`

```rust
let tasks_toml = extract_toml_block(&agent_output.text);
// Validate before writing
match toml::from_str::<TasksPlan>(&tasks_toml) {
    Ok(_) => std::fs::write(&toml_path, &tasks_toml)?,
    Err(e) => {
        eprintln!("Agent generated invalid TOML: {e}");
        eprintln!("Retrying with error context...");
        // Retry with the error message in context
    }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/plan.rs` | Remove feature gate, make runner v2 default |
| `crates/roko-cli/src/commands/develop.rs` | Don't double-generate plans |
| `crates/roko-cli/src/commands/plan.rs` | Validate TOML before writing |

## Priority

**P0** — The Graph Engine stub means `roko plan run` does nothing by default. Users must know
to compile with `--features legacy-runner-v2` to get actual execution. This is the most
confusing bug in the system.
