# Cost State and Budgets

> Cost accounting in microdollars, three-dimensional budget enforcement,
> atomic concurrent updates, BudgetEnforcer graceful degradation,
> serializable checkpoints for restart durability, and schema-v2 snapshot
> budget fields.

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/budget.rs` | BudgetTracker, BudgetEnforcer, BudgetCheckpoint |
| `crates/roko-graph/src/snapshot.rs` | GraphSnapshotV2 budget fields |
| `crates/roko-graph/src/types.rs` | GraphConfig (limit configuration) |

---

## Microdollar Representation

All cost accounting uses **microdollars**: 1 USD = 1,000,000 microdollars.
This avoids floating-point accumulation errors during concurrent updates
from parallel wave execution. The conversion:

```
cost_usd -> microdollars:  (cost_usd * 1_000_000.0) as u64
microdollars -> cost_usd:  microdollars as f64 / 1_000_000.0
```

Both `BudgetTracker` and `GraphSnapshotV2` use `u64` microdollars internally.
Public APIs accept and return `f64` USD for convenience.

---

## BudgetTracker

`BudgetTracker` monitors resource consumption during graph execution and
enforces limits. It tracks three resource dimensions:

| Dimension | Field | Type | Limit Source |
|---|---|---|---|
| Tokens | `tokens_used` | `AtomicU64` | `GraphConfig::max_tokens` |
| Cost | `cost_microdollars` | `AtomicU64` | `GraphConfig::max_cost_usd` |
| Time | `start_time` | `Instant` | `GraphConfig::deadline` |

### Construction

```rust
// From graph configuration
let tracker = BudgetTracker::from_config(&graph_config);

// With explicit limits (testing)
let tracker = BudgetTracker::with_limits(BudgetLimits {
    max_tokens: Some(10_000),
    max_cost_usd: Some(5.0),
    deadline: Some(Duration::from_secs(300)),
});
```

### Budget Check

`check()` is called before each node execution. It returns
`Err(GraphError::BudgetExceeded { reason })` when any limit is breached:

```rust
pub fn check(&self) -> Result<()> {
    // 1. Token limit: tokens_used >= max_tokens
    // 2. Cost limit:  cost_usd() >= max_cost_usd
    // 3. Deadline:    elapsed() >= deadline
}
```

When any limit is exceeded, the engine skips remaining nodes. The error
message identifies which dimension was breached and the actual vs. limit
values.

### Recording Consumption

After each node completes, the engine records its resource consumption:

```rust
tracker.record(
    "task.T1.executor",   // node_id
    4500,                 // tokens
    0.0135,               // cost_usd
    Duration::from_secs(12),  // duration
);
```

`record()` uses `fetch_add` with `Ordering::Relaxed` for lock-free concurrent
updates from parallel wave execution. The per-node cost breakdown is stored
in a `Mutex<Vec<NodeCost>>` for reporting.

### Query Methods

| Method | Returns |
|---|---|
| `tokens_used()` | Total tokens consumed |
| `cost_usd()` | Total cost in USD (converted from microdollars) |
| `elapsed()` | Wall-clock time since `start_time` |
| `remaining_cost_usd()` | `max_cost_usd - cost_usd()`, if limit configured |
| `remaining_time()` | `deadline - elapsed()`, if deadline configured |
| `breakdown()` | `Vec<NodeCost>` with per-node token/cost/duration |

### No Limits

When all limit fields are `None`, `check()` always returns `Ok(())`. This
is the default for graphs that do not set `[graph.config]` limits.

---

## BudgetCheckpoint

Serializable cumulative budget state for durable checkpoints:

```rust
pub struct BudgetCheckpoint {
    pub tokens_used:       u64,
    pub cost_microdollars: u64,      // exact, no floating-point loss
    pub elapsed_ms:        u64,
    pub breakdown:         Vec<NodeCostCheckpoint>,
}

pub struct NodeCostCheckpoint {
    pub node_id:           String,
    pub tokens:            u64,
    pub cost_microdollars: u64,      // exact microdollars, not f64
    pub duration_ms:       u64,
}
```

### Checkpoint / Restore Cycle

```rust
// Capture current state
let checkpoint = tracker.checkpoint();

// ... process crash / restart ...

// Restore from checkpoint
tracker.restore_checkpoint(&checkpoint);
```

`restore_checkpoint` restores all counters:
- `tokens_used` and `cost_microdollars` are stored directly.
- `start_time` is backdated by `elapsed_ms` so that elapsed-time checks
  account for prior execution time.
- Per-node breakdown is rebuilt from `NodeCostCheckpoint` entries.

This ensures cost accounting spans process restarts without double-counting.

---

## BudgetEnforcer

`BudgetEnforcer` wraps `BudgetTracker` with Hot-Graph-specific graceful
degradation. Instead of hard-failing when budget is exhausted, it classifies
nodes as **essential** (always run) or **expensive** (skip when over budget).

### Default Essential Types

```rust
["signal-reader", "event-publisher", "sense", "react", "noop"]
```

These are the cognitive loop endpoints that should always run for observability,
even when the budget is exhausted.

### Decision Logic

```rust
pub fn should_execute(&self, cell_type: &str) -> bool {
    if self.tracker.check().is_ok() {
        return true;       // budget available -> always run
    }
    // Budget exhausted -> only essential types proceed
    self.essential_types.contains(cell_type)
}
```

This enables a graceful wind-down: when the cost limit is hit, the cognitive
loop can still execute Sense and React cells for observability while skipping
Act and Compose cells that cost real money.

### Custom Essential Types

```rust
let enforcer = BudgetEnforcer::new(tracker)
    .with_essential("my-monitor");
```

### Enforcer Methods

| Method | Purpose |
|---|---|
| `should_execute(cell_type)` | Whether a node should be allowed to run |
| `is_exhausted()` | Whether the overall budget is exhausted |
| `remaining_cost_usd()` | Remaining USD budget |
| `record(...)` | Delegate to underlying tracker |
| `tokens_used()` / `cost_usd()` | Query totals |
| `breakdown()` | Per-node cost breakdown |
| `checkpoint()` / `restore_checkpoint()` | Durable checkpoint cycle |

---

## Schema-V2 Snapshot Budget Fields

`GraphSnapshotV2` (in `snapshot.rs`) includes two budget fields that persist
across restarts:

```rust
pub struct GraphSnapshotV2 {
    // ...
    pub budget_spent_micro_usd:    u64,  // 1 USD = 1_000_000
    pub budget_reserved_micro_usd: u64,  // reserved but not yet settled
    // ...
}
```

Both fields have `#[serde(default)]` for backward compatibility with v1
snapshots (which lacked them). A v1 snapshot on disk deserializes into V2
with zero values for both fields.

### Atomic Reservations

The `budget_reserved_micro_usd` field supports **atomic cost reservation**:
before dispatching an expensive node, the engine reserves an estimated cost.
If the node completes below estimate, the excess is released. If the node
fails, the reservation is reclaimed. The reservation ensures that concurrent
waves do not collectively exceed the budget by each checking independently.

### Budget State Flow

```
1. Node starts:     reserve estimated_cost in budget_reserved
2. Node completes:  move actual_cost from reserved to spent
3. Node fails:      release reservation back to available
4. Checkpoint:      persist both spent and reserved to snapshot
5. Resume:          restore spent + reserved, continue enforcement
```

---

## GraphConfig

The graph-level resource limits are configured via `GraphConfig`:

```rust
pub struct GraphConfig {
    pub max_tokens:   Option<u64>,       // maximum total tokens
    pub max_cost_usd: Option<f64>,       // maximum total cost in USD
    pub deadline:     Option<Duration>,  // maximum wall-clock time
}
```

All fields default to `None` (unlimited). The config is parsed from the
`[graph.config]` section of TOML graph definitions.

---

## Concurrency Safety

- `tokens_used` and `cost_microdollars` use `AtomicU64` with
  `Ordering::Relaxed` for lock-free concurrent updates from parallel waves.
- The per-node `breakdown` uses `parking_lot::Mutex<Vec<NodeCost>>` because
  it is append-only and read infrequently (reporting only).
- `BudgetEnforcer.essential_types` is a `HashSet<String>` constructed once
  at creation; it is immutable during execution.

---

## Verification

```bash
cargo test -p roko-graph --lib budget::tests
```

The test suite covers:
- Tracker starts empty (all counters zero).
- Token limit exceeded after cumulative `record()` calls.
- Cost limit exceeded (microdollar precision).
- Deadline exceeded (zero-duration deadline).
- No limits always passes (no configured limits).
- Remaining cost computed correctly.
- Breakdown records all nodes.
- Enforcer allows all when budget available.
- Enforcer skips expensive types when exhausted.
- Enforcer preserves custom essential types.
