# Runner 08 — CascadeRouter Integration

> **Give this entire file to a fresh agent.**

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. Goal: make `CascadeRouter` (LinUCB bandit) consulted before every model call, with live multi-objective observations from every call.

**Read first:**

1. `tmp/workflow/implementation-plans/08-cascade-router-integration.md`
2. `crates/roko-learn/src/cascade_router.rs` — existing router
3. `crates/roko-agent/src/model_call_service.rs` — where routing would be consulted
4. `crates/roko-learn/src/feedback_service.rs` — where observations are recorded

---

## Work Items

### 1. Define `RoutingContext` (6 features)

Create `crates/roko-learn/src/routing_context.rs`:

```rust
pub struct RoutingContext {
    pub task_tier: Tier,          // Mechanical/Focused/Integrative/Architectural
    pub role: String,
    pub attempt: u32,
    pub budget_pressure: f64,     // 0.0 = flush, 1.0 = broke
    pub prior_failure: bool,
    pub task_category: TaskCategory,
}
pub enum Tier { Mechanical, Focused, Integrative, Architectural }
pub enum TaskCategory { CodeChange, Refactor, Docs, Research, Test, Debug, Architecture, Other }
```

### 2. Define `TaskRequirements` + `filter_candidates`

Create `crates/roko-learn/src/task_requirements.rs`:

```rust
pub struct TaskRequirements {
    pub needs_web_search: bool,
    pub needs_code_execution: bool,
    pub needs_thinking: bool,
    pub min_context_window: Option<u32>,
    pub max_cost_usd: Option<f64>,
}

pub fn filter_candidates<'a>(candidates: &'a [ModelDescriptor], req: &TaskRequirements) -> Vec<&'a ModelDescriptor> { /* filter by capabilities */ }
```

### 3. Wire into `ModelCallService::resolve`

**File:** `crates/roko-agent/src/model_call_service.rs`

In the `resolve` method, after static model resolution, consult `CascadeRouter::select_for_frequency_among` if `self.router` is `Some`. Filter candidates via `filter_candidates` first.

### 4. `force_backend` override

Read `[runtime.routing].force_backend` from config. If set, bypass router entirely. Log with `tracing::info!`.

### 5. Tier-based fallback

When router has < 5 observations per arm, fall back: `Mechanical→haiku`, `Focused→sonnet`, `Integrative→sonnet`, `Architectural→opus`. Configurable via `[runtime.routing.tier_defaults]`.

### 6. Tests

- 5 failures → router avoids failing model
- Budget pressure high → cheaper model selected
- Task with `needs_web_search` → Ollama filtered out

---

## Verification

```bash
rg 'pub struct RoutingContext' crates/roko-learn/src/ --type rust
# struct with 6 fields

rg 'observe_multi_objective' crates/roko-learn/src/feedback_service.rs
# is the live path

rg 'force_backend' crates/roko-agent/src/model_call_service.rs
# present

cargo test --workspace
```
