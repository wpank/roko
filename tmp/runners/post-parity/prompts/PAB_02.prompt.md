# PAB_02: Thread max_concurrent_tasks and max_concurrent_plans from RunConfig

## Task
Read `max_concurrent_tasks` and `max_concurrent_plans` from `RunConfig` / `roko.toml` instead of hardcoding.

## Runner Context
Runner PAB, batch 2 of 4. No dependencies.

## Problem
`event_loop.rs:113-116`:
```rust
let exec_config = ExecutorConfig {
    max_concurrent_plans: 4,  // hardcoded
    max_concurrent_tasks: 1,  // hardcoded — negates the DAG
};
```

## Exact Changes

### Step 1: Add fields to RunConfig

In `crates/roko-cli/src/runner/mod.rs` or wherever `RunConfig` is defined:
```rust
pub struct RunConfig {
    // ... existing fields ...
    pub max_concurrent_plans: usize,
    pub max_concurrent_tasks: usize,
}
```

### Step 2: Read from roko.toml

In `RunConfig::from_roko_config`:
```rust
max_concurrent_plans: config.runner.as_ref()
    .and_then(|r| r.max_concurrent_plans)
    .unwrap_or(4),
max_concurrent_tasks: config.runner.as_ref()
    .and_then(|r| r.max_concurrent_tasks)
    .unwrap_or(1),
```

### Step 3: Thread into ExecutorConfig

```rust
let exec_config = ExecutorConfig {
    max_concurrent_plans: run_config.max_concurrent_plans,
    max_concurrent_tasks: run_config.max_concurrent_tasks,
};
```

### Step 4: Add roko.toml schema

```toml
[runner]
max_concurrent_tasks = 4
max_concurrent_plans = 4
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context
- `crates/roko-core/src/config/mod.rs` (RokoConfig structure)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- `max_concurrent_tasks` configurable via `roko.toml` `[runner]` section
- `max_concurrent_plans` configurable via `roko.toml` `[runner]` section
- Default values match current behavior (4 plans, 1 task)
- `roko plan run` with `max_concurrent_tasks = 4` runs 4 independent tasks simultaneously

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
