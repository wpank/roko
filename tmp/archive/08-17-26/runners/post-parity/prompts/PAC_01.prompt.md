# PAC_01: Wire cumulative per-turn spend tracking into SafetyBudgetTracker

## Task
Make `SafetyBudgetTracker` instantiated in production and track cumulative per-turn spend (not just per-call estimates).

## Runner Context
Runner PAC (Safety Completeness), batch 1 of 4. No dependencies.

## Problem
ISS-4 safety gap: `GovernanceRule::MaxCostPerTurn` (contract.rs:302-303) checks estimated cost per individual tool call, but there's no cumulative tracking across a turn. A task making 20 cheap tool calls could exceed the per-turn budget without triggering the guard.

Additionally, `SafetyBudgetTracker` (risk.rs:390-481) is defined and has `check()`, `consume()`, `check_and_consume()`, `is_exhausted()` methods — but it's only instantiated in tests. Zero production call sites.

## Current Code

**SafetyBudgetTracker** — `crates/roko-agent/src/safety/risk.rs:390-481`:
```rust
pub struct SafetyBudgetTracker {
    budget: SafetyBudget,
    usage: SafetyUsage,
    uncertainty_threshold: f64,
}
```

**SafetyLayer field** — `crates/roko-agent/src/safety/mod.rs:198`:
```rust
pub safety_budget: Option<Arc<Mutex<SafetyBudgetTracker>>>,
```

**Builder methods** — `safety/mod.rs:297,304`:
```rust
pub fn with_safety_budget(mut self, budget: SafetyBudget) -> Self { ... }
pub fn with_shared_safety_budget(mut self, budget: Arc<Mutex<SafetyBudgetTracker>>) -> Self { ... }
```

**MaxCostPerTurn TODO** — `crates/roko-agent/src/safety/contract.rs:442-443`:
```rust
// TODO(UX26): enforce cumulative per-turn spend once tool-cost accounting is threaded into ToolContext.
```

**SpendingLimiter** — `crates/roko-agent/src/safety/spending.rs:1-80`:
Separate hook wrapping `BudgetTracker`. Checks per-tool cost estimates. Different from `SafetyBudgetTracker`.

## Exact Changes

### Step 1: Instantiate SafetyBudgetTracker in SafetyLayer::with_defaults()

```rust
// In safety/mod.rs, SafetyLayer::with_defaults():
// BEFORE:
safety_budget: None,

// AFTER:
safety_budget: Some(Arc::new(Mutex::new(SafetyBudgetTracker::new(
    SafetyBudget {
        max_cost_per_turn: 5.0,      // $5 per turn default
        max_total_cost: 100.0,       // $100 total default
        max_tool_calls_per_turn: 50, // 50 calls per turn
    },
)))),
```

### Step 2: Wire cumulative spend tracking into tool dispatch

In the tool dispatch path (`safety/mod.rs`, around the pre-check or post-check flow):

```rust
// After each tool call completes with a cost:
if let Some(budget) = &self.safety_budget {
    let mut tracker = budget.lock().unwrap();
    let cost = tool_result.estimated_cost_usd.unwrap_or(0.0);
    if let Err(err) = tracker.consume(cost) {
        warn!(%err, "safety budget exhausted — blocking further tool calls this turn");
        return Err(SafetyError::BudgetExhausted);
    }
}
```

### Step 3: Reset per-turn tracking at turn boundaries

At the start of each new turn (before tool calls begin):

```rust
// In the tool loop, at turn start:
if let Some(budget) = &safety_layer.safety_budget {
    let mut tracker = budget.lock().unwrap();
    tracker.reset_turn();  // reset per-turn counters, keep total
}
```

If `reset_turn()` doesn't exist on `SafetyBudgetTracker`, add it:
```rust
pub fn reset_turn(&mut self) {
    self.usage.turn_cost = 0.0;
    self.usage.turn_tool_calls = 0;
}
```

### Step 4: Make budget configurable from roko.toml

Read budget limits from config if available:
```rust
if let Some(budget_config) = &config.safety.budget {
    safety_layer = safety_layer.with_safety_budget(SafetyBudget {
        max_cost_per_turn: budget_config.max_cost_per_turn.unwrap_or(5.0),
        max_total_cost: budget_config.max_total_cost.unwrap_or(100.0),
        max_tool_calls_per_turn: budget_config.max_tool_calls_per_turn.unwrap_or(50),
    });
}
```

## Write Scope
- `crates/roko-agent/src/safety/mod.rs` (instantiate budget tracker in with_defaults)
- `crates/roko-agent/src/safety/risk.rs` (add reset_turn if missing)
- `crates/roko-agent/src/tool_loop/mod.rs` (consume cost after calls, reset at turn start)

## Read-Only Context
- `crates/roko-agent/src/safety/contract.rs` (MaxCostPerTurn governance rule, TODO at 442-443)
- `crates/roko-agent/src/safety/spending.rs` (SpendingLimiter — separate mechanism, don't duplicate)


## Verify
```bash
cargo build -p roko-agent 2>&1 | head -30
cargo test -p roko-agent 2>&1 | tail -20
```
## Acceptance Criteria
- `SafetyBudgetTracker` instantiated in production (not just tests)
- Cumulative per-turn spend tracked across tool calls
- Budget exhaustion blocks further tool calls (not crash)
- Per-turn counters reset at turn boundaries
- Budget limits configurable from roko.toml

## Do NOT
- Remove or replace `SpendingLimiter` (it's a separate per-tool-call mechanism)
- Change the `SafetyBudget` struct
- Add budget tracking for non-tool operations (keep scope to tool dispatch)
