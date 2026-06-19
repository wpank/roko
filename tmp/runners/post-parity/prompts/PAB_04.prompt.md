# PAB_04: Make cascade router normalization denominators configurable

## Task
Replace hardcoded cost ($1) and latency (300s) normalization values with config-driven references.

## Runner Context
Runner PAB, batch 4 of 4. No dependencies.

## Problem
`event_loop.rs:2415-2417`:
```rust
let normalized_cost = (state.cost_usd / 1.0).clamp(0.0, 1.0);    // $1 reference
let normalized_latency = (wall_secs / 300.0).clamp(0.0, 1.0);     // 5min reference
```

Real tasks can cost $5–50 and take 20+ minutes. This clamps nearly all observations to 1.0, making the multi-objective reward function useless (everything is at maximum).

## Exact Changes

### Step 1: Add config fields

In `RunConfig` or the cascade router config section:
```rust
pub struct RunnerConfig {
    // ... existing ...
    pub reward_cost_reference_usd: f64,    // default: 5.0
    pub reward_latency_reference_secs: f64, // default: 600.0
}
```

### Step 2: Use config values

```rust
let cost_ref = run_config.reward_cost_reference_usd.max(0.01);  // prevent div by zero
let latency_ref = run_config.reward_latency_reference_secs.max(1.0);
let normalized_cost = (state.cost_usd / cost_ref).clamp(0.0, 1.0);
let normalized_latency = (wall_secs / latency_ref).clamp(0.0, 1.0);
```

### Step 3: Document in roko.toml schema

```toml
[runner]
# Reference values for cascade router reward normalization
reward_cost_reference_usd = 5.0     # tasks costing > $5 get normalized_cost = 1.0
reward_latency_reference_secs = 600  # tasks taking > 10min get normalized_latency = 1.0
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context
- `crates/roko-core/src/config/mod.rs` (config structure)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Normalization uses config values, not hardcoded $1/300s
- Default references are reasonable for typical plan tasks ($5, 600s)
- Cascade router sees a meaningful spread of normalized values (not all 1.0)
- Div-by-zero prevented with `.max()` guard

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
