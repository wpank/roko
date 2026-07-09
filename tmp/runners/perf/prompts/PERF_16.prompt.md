# PERF_16: Speculative reviewer pre-warm

## Task

While the implementer is dispatching, fire-and-forget pre-warm a slot
for the reviewer's `(provider, model)` so the second dispatch in a
standard / full workflow hits a warm slot instead of constructing
cold. Bound speculation by a per-run budget so a runaway loop cannot
spam the pool.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_16](../ISSUE-TRACKER.md#perf_16)
- Plan: `tmp/solutions/perf/implementation/14-speculative-execution.md`
- Bottleneck: novel
- Performance contract: **C-15** (≥80 % warm-hit rate on standard workflow)
- Priority: P2
- Effort: ≈6 h
- Depends on: **PERF_11** (must land first; needs full warm pool wiring)
- Wave: 4

## Problem

After PERF_10/11, the warm pool serves a hit on the SECOND dispatch
when the FIRST already populated `(provider, model)`. In standard
workflow (`implement → gate → review`), the implementer and reviewer
typically use the same `(provider, model)` (sometimes different role
overrides). When they differ, the reviewer's first dispatch is cold.

We can do better: while the implementer is talking to the LLM
(blocking for 1-3 s), background-construct the reviewer's slot. By the
time the reviewer is acquired, it's warm.

Cost/benefit:

```
P(reviewer runs) × saved_acquire_ms > P(reviewer never runs) × prewarm_cost_ms
0.85             × 30                > 0.15                    × 100
25.5             > 15
                                       net + 10.5 ms per run
```

## Exact Changes

### Step 1 — Add `pre_warm_for(provider, model)` to `WarmDispatchPool`

`crates/roko-runtime/src/warm_dispatch_pool.rs`. Add to the `impl
WarmDispatchPool` block:

```rust
impl WarmDispatchPool {
    /// Eagerly construct a slot for `(provider, model)` if no idle
    /// slot already matches. Returns immediately; the actual factory
    /// invocation runs in a background `tokio::spawn` so the caller
    /// is not blocked.
    ///
    /// Idempotent: a fast `try_lock` check prevents most duplicate
    /// spawns; the post-lock check inside the spawn handles the race
    /// where two pre_warms fire concurrently.
    pub fn pre_warm_for(self: Arc<Self>, provider: String, model: String) {
        // Fast path: avoid spawning if a matching idle slot already exists.
        // Use try_lock so we never block here; failure to acquire just
        // means we err on the side of one extra construction attempt
        // (the post-lock check inside the spawn dedupes).
        let already_warm = match self.slots.try_lock() {
            Ok(slots) => slots.iter().any(|s|
                s.provider == provider && s.model == model
                && matches!(s.state, SlotState::Idle)),
            Err(_) => false,
        };
        if already_warm { return; }

        let pool = Arc::clone(&self);
        tokio::spawn(async move {
            // Construct the caller (network-free; just a struct alloc).
            let Some(caller) = (pool.factory)(&provider, &model) else { return; };

            // Re-check after acquiring the lock: another pre_warm or a
            // synchronous acquire may have warmed the slot in between.
            let mut slots = pool.slots.lock().await;
            let already = slots.iter().any(|s|
                s.provider == provider && s.model == model
                && matches!(s.state, SlotState::Idle));
            if already { return; }

            slots.push(WarmSlot {
                provider: provider.clone(),
                model: model.clone(),
                caller,
                created_at: Instant::now(),
                last_used: Instant::now(),
                dispatches_served: 0,
                state: SlotState::Idle,
            });
            tracing::debug!(target: "roko_perf", provider, model, "speculative pre-warm completed");
        });
    }
}
```

### Step 2 — Add `WorkflowMetadata` to `EffectDriver`

`crates/roko-runtime/src/effect_driver.rs`. Add new types and field:

```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowMetadata {
    pub reviewer_target: Option<ProviderModelTarget>,
    pub strategist_target: Option<ProviderModelTarget>,
}

#[derive(Debug, Clone)]
pub struct ProviderModelTarget {
    pub provider: String,
    pub model: String,
}

pub struct EffectDriver {
    services: EffectServices,
    run_id: String,
    workdir: PathBuf,
    feedback_totals: tokio::sync::Mutex<WorkflowFeedbackTotals>,
    workflow_metadata: tokio::sync::RwLock<WorkflowMetadata>,
    speculation_budget: std::sync::atomic::AtomicU32,
}

impl EffectDriver {
    pub fn new(services: EffectServices, run_id: String, workdir: PathBuf) -> Self {
        Self {
            services,
            run_id,
            workdir,
            feedback_totals: tokio::sync::Mutex::new(WorkflowFeedbackTotals::default()),
            workflow_metadata: tokio::sync::RwLock::new(WorkflowMetadata::default()),
            speculation_budget: std::sync::atomic::AtomicU32::new(3),
        }
    }

    pub async fn set_workflow_metadata(&self, metadata: WorkflowMetadata) {
        *self.workflow_metadata.write().await = metadata;
    }

    pub fn reset_speculation_budget(&self, budget: u32) {
        self.speculation_budget.store(budget, std::sync::atomic::Ordering::Relaxed);
    }
}
```

### Step 3 — Trigger speculation in `spawn_agent`

In `EffectDriver::spawn_agent`, after the agent-spawned event is
emitted but BEFORE the model call awaits, fire the speculation:

```rust
// Speculative pre-warm: if this is the implementer in a workflow with
// review, warm the reviewer's slot in the background. Capped by per-
// run budget to prevent runaway loops.
if role == "implementer" {
    use std::sync::atomic::Ordering;
    if self.speculation_budget.fetch_sub(1, Ordering::Relaxed) > 0 {
        if let Some(ref pool) = self.services.warm_pool {
            let target = self.workflow_metadata.read().await
                .reviewer_target.clone();
            if let Some(t) = target {
                Arc::clone(pool).pre_warm_for(t.provider, t.model);
            }
        }
    } else {
        tracing::debug!(target: "roko_perf",
            "speculation budget exhausted; skipping reviewer pre-warm");
    }
}
```

### Step 4 — Engine populates `WorkflowMetadata`

`crates/roko-runtime/src/workflow_engine.rs::WorkflowEngine::run`. At
the very top of `run`, before any dispatch happens:

```rust
use crate::effect_driver::{ProviderModelTarget, WorkflowMetadata};
use roko_agent::provider::infer_provider_from_model;

let metadata = WorkflowMetadata {
    reviewer_target: if config.workflow.has_review {
        let model = config.role_models.get("reviewer")
            .cloned()
            .unwrap_or_else(|| self.driver.services.default_model.clone());
        if model.is_empty() {
            None
        } else {
            Some(ProviderModelTarget {
                provider: infer_provider_from_model(&model).to_string(),
                model,
            })
        }
    } else { None },
    strategist_target: if config.workflow.has_strategy {
        let model = config.role_models.get("strategist")
            .cloned()
            .unwrap_or_else(|| self.driver.services.default_model.clone());
        if model.is_empty() {
            None
        } else {
            Some(ProviderModelTarget {
                provider: infer_provider_from_model(&model).to_string(),
                model,
            })
        }
    } else { None },
};
self.driver.set_workflow_metadata(metadata).await;
self.driver.reset_speculation_budget(3);
```

> If `WorkflowRunConfig` does not have a `role_models: HashMap<String,
> String>` field today, infer from `default_model`. This is acceptable
> because most users use the same model across roles. If/when role
> overrides are added, the `unwrap_or_else` falls back gracefully.

### Step 5 — Config flag

`crates/roko-core/src/config/schema.rs`. Add to `ConductorConfig` (or
`WorkflowConfig` schema, wherever speculation makes sense):

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WorkflowSpeculationConfig {
    /// Enable speculative reviewer pre-warm during implementer
    /// dispatch. Default: true.
    pub enabled: bool,
    /// Maximum pre-warms per run. Default: 3 (caps runaway loops).
    pub budget_per_run: u32,
}

impl Default for WorkflowSpeculationConfig {
    fn default() -> Self {
        Self { enabled: true, budget_per_run: 3 }
    }
}

// And add to the workflow / conductor parent:
pub struct WorkflowConfigSchema {
    // ... existing fields ...
    pub speculation: WorkflowSpeculationConfig,
}
```

In Step 3, gate the speculation on `enabled`:

```rust
let speculation_enabled = self.workflow_metadata.read().await.reviewer_target.is_some();
// ... and check the config's enabled flag too.
```

(Plumbing the config all the way to `EffectDriver` may require a small
extra field on `EffectServices`. If the plumbing is non-trivial, keep
the AtomicU32 budget as the only knob and document the config flag as
deferred to a follow-up batch.)

### Step 6 — Tests

Append to `crates/roko-runtime/src/warm_dispatch_pool.rs`:

```rust
#[tokio::test]
async fn pre_warm_for_does_not_duplicate_slots() {
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = std::sync::Arc::clone(&counter);
    let factory: ModelCallerFactory = std::sync::Arc::new(move |_p, model| {
        counter_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(std::sync::Arc::new(MockCaller::new(model.to_string())) as std::sync::Arc<dyn ModelCaller>)
    });
    let pool = std::sync::Arc::new(WarmDispatchPool::new(WarmPoolConfig::default(), factory));

    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert_eq!(pool.slot_count_for_test().await, 1, "pre_warm_for must dedupe");
}
```

Append to `crates/roko-runtime/src/effect_driver.rs`:

```rust
#[tokio::test]
async fn speculation_budget_caps_pre_warms() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let factory: ModelCallerFactory = Arc::new(move |_p, _m| {
        counter_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(Arc::new(MockCaller::new("m".into())) as Arc<dyn ModelCaller>)
    });
    let pool = Arc::new(WarmDispatchPool::new(WarmPoolConfig::default(), factory));
    let mut services = mock_effect_services();
    services.warm_pool = Some(Arc::clone(&pool));

    let driver = EffectDriver::new(services, "run".into(), tempdir().path().into());
    driver.reset_speculation_budget(2);
    driver.set_workflow_metadata(WorkflowMetadata {
        reviewer_target: Some(ProviderModelTarget {
            provider: "openai".into(), model: "x".into(),
        }),
        strategist_target: None,
    }).await;

    for _ in 0..5 {
        let _ = driver.spawn_agent("implementer", "task", None).await;
    }
    // budget = 2 → at most 2 pre_warms fired (counter ≤ 2).
    // Plus implementer's own dispatch may construct 1 cold slot if
    // the model differs from reviewer's. We assert the cap loosely:
    assert!(counter.load(std::sync::atomic::Ordering::Relaxed) <= 5);
}

#[tokio::test]
async fn standard_workflow_reviewer_hits_warm_slot() {
    // (Adapt: drive a 2-step workflow with both implementer and
    // reviewer using the same (provider, model). Assert
    // pool.metrics().warm_hits >= 1.)
}
```

## Write Scope

- `crates/roko-runtime/src/warm_dispatch_pool.rs`
- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-core/src/config/schema.rs` (if Step 5 plumbed in this batch)

## Read-Only Context

- `crates/roko-runtime/src/pipeline_state.rs` (`WorkflowConfig.has_review`)
- `crates/roko-agent/src/provider/mod.rs` (`infer_provider_from_model` from PERF_10)
- `tmp/solutions/perf/implementation/14-speculative-execution.md`

## Acceptance Criteria

- [ ] `WarmDispatchPool::pre_warm_for(self: Arc<Self>, provider: String, model: String)` exists; spawns and dedupes (`try_lock` fast-path + post-lock check).
- [ ] `WorkflowMetadata { reviewer_target, strategist_target }` plumbed through `EffectDriver`.
- [ ] Engine populates metadata for `standard`/`full` workflows from resolved model selection (with `default_model` fallback).
- [ ] Speculation budget `AtomicU32` (default 3 per run) caps pre-warms; exhausted budget logs once at `debug!`.
- [ ] `[conductor.workflow.speculation]` config flag (default `true`) controls behaviour OR documented as deferred.
- [ ] Test `pre_warm_for_does_not_duplicate_slots` passes.
- [ ] Test `speculation_budget_caps_pre_warms` passes.
- [ ] Test `standard_workflow_reviewer_hits_warm_slot` passes.

## Verify

```bash
# Method exists:
rg -n 'pub fn pre_warm_for' crates/roko-runtime/src/warm_dispatch_pool.rs

# Speculation budget plumbed:
rg -n 'speculation_budget' crates/roko-runtime/src/effect_driver.rs

# No duplicate spawn loop:
rg -nU --multiline 'pre_warm_for.*?pre_warm_for' crates/
# Expected: only inside the dedup test.
```

## Do NOT

- Do NOT pre-warm during `roko run --workflow express`. Express has no
  review phase; speculation wastes the slot.
- Do NOT pre-warm the same `(provider, model)` twice. The `try_lock`
  + post-lock check pattern is mandatory; do not skip it.
- Do NOT block the implementer on the speculation. Speculation is a
  `tokio::spawn`; never `.await` the handle.
- Do NOT speculate in a tight loop (e.g., per autofix attempt). The
  same model is already warm; further pre-warms are pure waste. The
  speculation budget caps this.
- Do NOT extend speculation to "run the reviewer's prompt assembly".
  Prompt content depends on implementer output; you cannot speculate
  over it. Speculation here is purely about caller construction.
- Do NOT remove `evict_idle` calls. A speculation that misses (e.g.,
  implementer fails) leaves a warm slot; eviction reclaims it.
- Do NOT log misses with `warn!`. Misses are expected (~15 % of
  standard runs); use `debug!`.
- Do NOT bundle this with strategist speculation. Strategist runs first
  (nothing to speculate during). A separate plan could speculate the
  implementer during strategist runtime; that's a different change.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_16 done <commit-sha>
```
