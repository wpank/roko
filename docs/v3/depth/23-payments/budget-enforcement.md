# Budget Enforcement

> Depth file for [23-PAYMENTS](../../23-PAYMENTS.md) -- the six cost ceilings,
> tier multipliers, and how the runner enforces spend limits.

---

## 1. Overview

Budget enforcement prevents runaway spending by applying six independent cost
ceilings and tier-based multipliers. Ceilings are configured in `roko.toml`
under the `[budget]` section. A ceiling of `0.0` means unlimited -- the runner
will not enforce any cap for that dimension.

**Configuration location:** `crates/roko-core/src/config/budget.rs`

---

## 2. The Six Ceilings

```rust
pub struct BudgetConfig {
    pub max_plan_usd: f32,           // per-plan ceiling
    pub max_task_usd: f32,           // base per-task ceiling
    pub max_turn_usd: f32,           // per-turn ceiling
    pub max_task_retry_usd: f32,     // cumulative across retries (default: $5)
    pub max_daily_usd: f32,          // per-calendar-day ceiling
    pub max_agent_lifetime_usd: f32, // per-agent cumulative lifetime
    pub prompt_token_budget: usize,  // token budget for composition (default: 10,000)
    pub tier_multipliers: TaskBudgetMultipliers,
}
```

### 2.1 Ceiling Semantics

| Ceiling | Scope | Default | Enforcement |
|---|---|---|---|
| `max_plan_usd` | Entire plan run | 0.0 (unlimited) | Blocks new dispatches when cumulative plan cost exceeds |
| `max_task_usd` | Single task attempt | 0.0 (unlimited) | Base ceiling, scaled by tier multiplier |
| `max_turn_usd` | Single provider turn | 0.0 (unlimited) | Rejects response if turn cost exceeds |
| `max_task_retry_usd` | All retries for one task | 5.0 | Suppresses retry when cumulative retry cost exceeds |
| `max_daily_usd` | Calendar day across all runs | 0.0 (unlimited) | Reads costs log to check day total |
| `max_agent_lifetime_usd` | Agent's entire lifetime | 0.0 (unlimited) | Emits `AgentBudgetExhausted` and drains agent |

### 2.2 Validation

Pre-flight validation (`validate_budget_ceilings`) rejects:
- Negative values
- `NaN` values
- `Inf` values

Previous defaults of 25.0/3.0 were removed because they silently capped every
run even when the user never configured a budget.

---

## 3. Tier Multipliers

The `TaskBudgetMultipliers` struct scales `max_task_usd` based on task
complexity:

```rust
pub struct TaskBudgetMultipliers {
    pub mechanical: f32,   // default: 0.2x
    pub standard: f32,     // default: 1.0x (aliases: "focused")
    pub complex: f32,      // default: 3.0x (aliases: "integrative")
    pub expert: f32,       // default: 5.0x (aliases: "architectural")
}
```

### 3.1 Effective Task Budget

```
effective_task_budget = max_task_usd * tier_multiplier
```

| Complexity | Multiplier | Example (base $2) |
|---|---|---|
| Mechanical | 0.2x | $0.40 |
| Standard | 1.0x | $2.00 |
| Complex | 3.0x | $6.00 |
| Expert | 5.0x | $10.00 |

A mechanical task (simple formatting, config change) gets 1/5 of the base
budget. An expert task (architecture, safety review) gets 5x. This prevents
cheap tasks from consuming expensive model tiers and ensures complex tasks have
sufficient budget.

---

## 4. Configuration

```toml
# roko.toml
[budget]
max_plan_usd = 50.0
max_task_usd = 5.0
max_turn_usd = 2.0
max_task_retry_usd = 10.0
max_daily_usd = 100.0
max_agent_lifetime_usd = 500.0
prompt_token_budget = 10000

[budget.tier_multipliers]
mechanical = 0.2
standard = 1.0
complex = 3.0
expert = 5.0
```

---

## 5. Enforcement Points

### 5.1 Runner Event Loop

The runner checks ceilings at multiple points:

1. **Pre-dispatch** -- check plan and daily ceilings before spawning a task
2. **Per-turn** -- check turn ceiling after each provider response
3. **Post-task** -- update cumulative cost and check retry ceiling
4. **Post-plan** -- final cost accounting

### 5.2 Graph Execution

The Graph engine (sole engine since #260) enforces paid-failure-aware cost
ceilings with atomic reservations and resume-durable schema-v2 cost state.
Cost accounting survives process restarts through graph checkpoints.

### 5.3 ACP Sessions

ACP sessions carry persisted/enforced USD budgets per agent. The budget is
checked at dispatch time and updated after each provider call.

### 5.4 Daily Ceiling

The daily ceiling reads from the costs log (`CostsLog.load_since()`) to
calculate today's total spend across all plan runs. When the total reaches
the ceiling, new dispatches are blocked.

---

## 6. Budget Exhaustion Behavior

When a ceiling is hit:

| Ceiling | Behavior |
|---|---|
| Plan | New task dispatches blocked; running tasks allowed to complete |
| Task | Task marked failed with "budget exceeded" reason |
| Turn | Provider response rejected; task gets one retry chance |
| Retry | Task marked failed with "cumulative cost cap exceeded" |
| Daily | All new dispatches blocked until midnight UTC |
| Agent lifetime | `AgentBudgetExhausted` event emitted; agent drained |

---

## 7. Cost Model by Provider Tier

| Tier | Per-Request Cost | Typical Daily Calls | Daily Cost |
|---|---|---|---|
| T0 (zero-LLM probes) | $0.00 | 100+ | $0.00 |
| T1 (haiku-tier) | $0.005-0.02 | 50 | $0.25-1.00 |
| T2 (sonnet-tier) | $0.03-0.10 | 30 | $0.90-3.00 |
| T3 (opus-tier) | $0.10-0.50 | 10 | $1.00-5.00 |

The CascadeRouter optimizes model selection, suppressing ~80% of requests to
T0 probes that require no inference cost.

---

## 8. CLI Inspection

```bash
roko learn inspect budget     # Read-only budget inspection
roko config preset budget     # Apply validated budget presets
roko show costs               # Current cost summary
```

---

## 9. Source Locations

| Component | Path |
|---|---|
| BudgetConfig | `crates/roko-core/src/config/budget.rs` |
| Cost enforcement guards | `crates/roko-execution/src/guards.rs` |
| Runner budget checks | `crates/roko-cli/src/runner/` |
| Graph cost state | `crates/roko-graph/src/` |
| ACP budget enforcement | `crates/roko-acp/src/runner.rs` |
| RuntimeServices builder | `crates/roko-execution/src/builder.rs` |

---

*New file for v3. Content derived from actual `BudgetConfig` implementation in
`roko-core`. The six ceilings and tier multipliers are all live and enforced.*
