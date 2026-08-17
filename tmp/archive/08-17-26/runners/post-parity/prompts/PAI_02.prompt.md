# PAI_02: Add conductor per-watcher configuration from roko.toml

## Task
Make conductor watcher thresholds configurable from roko.toml instead of hardcoded constructor parameters.

## Runner Context
Runner PAI (Config & Infrastructure), batch 2 of 4. No dependencies.

## Problem
CI-2 anti-pattern: "10 watchers, all hardcoded thresholds." The conductor has 10 watchers (roko-conductor/src/watchers/), each with inline hardcoded thresholds passed via constructor. No external configuration. Users can't tune sensitivity without recompiling.

## Current Watchers (VERIFIED)

| Watcher | Constructor | Hardcoded |
|---------|-----------|-----------|
| TimeOverrunWatcher | `::new()` (no params) | Internal default |
| CostOverrunWatcher | `::new(default_budget: f64)` | Budget value |
| StuckPatternWatcher | `::new(max_actions: usize)` | Action limit |
| SpecDriftWatcher | `::new(max_drift: f64)` | Drift threshold |
| TestFailureBudgetWatcher | `::new(min_failure_increase: u32)` | Failure delta |
| IterationLoopWatcher | `::new(max_attempts: usize)` | Retry limit |
| CompileFailRepeatWatcher | `::new(max_failures: usize)` | Failure count |
| GhostTurnWatcher | `::new(max_ghost_turns: usize)` | Ghost count |
| ReviewLoopWatcher | `::new(max_cycles: usize)` | Cycle limit |
| ContextWindowPressureWatcher | `::new(max_ratio: f64)` | Pressure ratio |

## Exact Changes

### Step 1: Add ConductorWatcherConfig to roko.toml schema

```rust
// In roko-core/src/config/schema.rs, under [conductor]:
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConductorConfig {
    pub enabled: Option<bool>,
    pub watchers: Option<WatcherConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatcherConfig {
    pub cost_budget: Option<f64>,           // default: 10.0
    pub max_stuck_actions: Option<usize>,    // default: 5
    pub max_spec_drift: Option<f64>,         // default: 0.3
    pub min_test_failure_increase: Option<u32>, // default: 2
    pub max_iteration_attempts: Option<usize>,  // default: 3
    pub max_compile_failures: Option<usize>,    // default: 3
    pub max_ghost_turns: Option<usize>,         // default: 3
    pub max_review_cycles: Option<usize>,       // default: 5
    pub max_context_pressure: Option<f64>,      // default: 0.9
}
```

### Step 2: Read config when constructing watchers

```rust
fn build_watchers(config: &WatcherConfig) -> Vec<Box<dyn Watcher>> {
    vec![
        Box::new(TimeOverrunWatcher::new()),
        Box::new(CostOverrunWatcher::new(config.cost_budget.unwrap_or(10.0))),
        Box::new(StuckPatternWatcher::new(config.max_stuck_actions.unwrap_or(5))),
        Box::new(SpecDriftWatcher::new(config.max_spec_drift.unwrap_or(0.3))),
        Box::new(TestFailureBudgetWatcher::new(config.min_test_failure_increase.unwrap_or(2))),
        Box::new(IterationLoopWatcher::new(config.max_iteration_attempts.unwrap_or(3))),
        Box::new(CompileFailRepeatWatcher::new(config.max_compile_failures.unwrap_or(3))),
        Box::new(GhostTurnWatcher::new(config.max_ghost_turns.unwrap_or(3))),
        Box::new(ReviewLoopWatcher::new(config.max_review_cycles.unwrap_or(5))),
        Box::new(ContextWindowPressureWatcher::new(config.max_context_pressure.unwrap_or(0.9))),
    ]
}
```

### Step 3: Example roko.toml section

```toml
[conductor]
enabled = true

[conductor.watchers]
cost_budget = 15.0
max_iteration_attempts = 5
max_compile_failures = 5
```

## Write Scope
- `crates/roko-core/src/config/schema.rs` (WatcherConfig struct)
- `crates/roko-conductor/src/lib.rs` or wherever watchers are constructed (use config)

## Read-Only Context
- `crates/roko-conductor/src/watchers/` (each watcher's constructor)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- All 10 watcher thresholds configurable from roko.toml
- Default values match current hardcoded values (backward compatible)
- Missing config section → all defaults (no change)
- Config values validated at parse time (no negative thresholds)

## Do NOT
- Change watcher behavior or algorithms
- Add new watchers
- Make watcher selection configurable (all watchers always run)
