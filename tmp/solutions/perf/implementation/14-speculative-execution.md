# 14 — Speculative Reviewer Pre-warm (novel)

> Bottleneck: in `standard` and `full` workflow templates, the reviewer
> agent is always invoked after gates pass. Today the reviewer's model
> caller is constructed cold (or warmed lazily) at the moment we need
> it. With the warm pool from Plan 09 in place, we can speculate ahead:
> while the implementer is running, pre-warm a reviewer slot.
>
> Target savings: 20–50 ms / standard run.
> Effort: ≈6 h. Risk: medium (wasted work if implementation fails or
> review is short-circuited).

---

## Goal & success criteria

After this change:

1. When the workflow engine spawns the implementer in a workflow that
   has `has_review = true`, it concurrently kicks off
   `WarmDispatchPool::pre_warm_for(provider, model)` for the reviewer's
   target model.
2. Pre-warming is **fire-and-forget**: it never blocks the implementer
   and never causes the workflow to fail.
3. The reviewer's `acquire` call (when implementation finishes) hits
   the pre-warmed slot in ≥90 % of runs.

Done when:

- A unit test runs a mock standard workflow and asserts the reviewer
  acquired a warm slot (not a cold construction).
- Macro-benchmark: standard workflow improvement ≥20 ms vs plan-09
  baseline.
- Pool metrics show `warm_hits` ≥ `total_dispatches - 1` for standard
  runs.

---

## Background

- Source: `WARM-POOL-DESIGN.md` §"Speculative Execution Extension".
- Speculative execution is a classic CPU pipelining trick adapted to
  agent dispatch. The cost is "wasted" pre-warm if the implementer
  never produces a passing result; the benefit is removing the
  reviewer's TLS handshake from the critical path.
- Net positive when:
  ```
  P(reviewer runs)  *  saved_acquire_ms  >  P(reviewer never runs)  *  prewarm_cost_ms
  ```
  In practice, `P(reviewer runs)` ≈ 0.85 in standard workflow and
  `saved_acquire_ms` ≈ 30, so even at `prewarm_cost_ms` = 100, net is
  +25 ms.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-runtime/src/workflow_engine.rs` | The state machine output that produces "spawn implementer" events. |
| `crates/roko-runtime/src/effect_driver.rs` | Where to add the speculation hook. |
| `crates/roko-runtime/src/warm_dispatch_pool.rs` (created by Plan 09) | Add `pre_warm_for(provider, model)` if not present. |
| `crates/roko-runtime/src/pipeline_state.rs` | `WorkflowConfig.has_review`, `has_strategy` flags. |

---

## Code-level plan

### Step 1 — Add `pre_warm_for(provider, model)` to the pool

If Plan 09 already shipped a `pre_warm()` that uses the configured
targets, add a more targeted method:

```rust
// crates/roko-runtime/src/warm_dispatch_pool.rs
impl WarmDispatchPool {
    /// Eagerly construct a slot for `(provider, model)` if not already
    /// warm. Returns immediately; the construction runs to completion
    /// in the background.
    pub fn pre_warm_for(self: Arc<Self>, provider: String, model: String) {
        let already_warm = {
            let slots = self.slots.try_lock();    // best-effort, no contention waits
            match slots {
                Ok(s) => s.iter().any(|slot|
                    slot.provider == provider && slot.model == model && matches!(slot.state, SlotState::Idle)),
                Err(_) => false,
            }
        };
        if already_warm { return; }

        let pool = Arc::clone(&self);
        tokio::spawn(async move {
            let Some(caller) = (pool.factory)(&provider, &model) else { return; };
            let mut slots = pool.slots.lock().await;
            // Re-check after acquiring lock (may have warmed concurrently).
            if !slots.iter().any(|s|
                s.provider == provider && s.model == model && matches!(s.state, SlotState::Idle))
            {
                slots.push(WarmSlot {
                    provider: provider.clone(),
                    model: model.clone(),
                    caller,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    dispatches_served: 0,
                    state: SlotState::Idle,
                });
            }
        });
    }
}
```

> **Anti-pattern alert.** Using `try_lock` for the fast path is safe
> here because failure to acquire just means we err on the side of
> spawning an extra construction (worst case: a duplicate slot, which
> the post-lock check eliminates). Do not use `try_lock` in the
> `acquire` path — there it would falsely return cold-miss.

### Step 2 — Trigger speculation in `EffectDriver::spawn_agent`

```rust
pub async fn spawn_agent(&self, role: &str, ...) -> PipelineInput {
    // Existing setup ...

    // Speculative pre-warm: if this is the implementer in a workflow
    // that has review, warm the reviewer's slot in the background.
    if role == "implementer" {
        if let (Some(ref pool), Some(ref reviewer_target)) =
            (self.services.warm_pool.as_ref(), self.workflow_metadata.reviewer_target.as_ref())
        {
            Arc::clone(pool).pre_warm_for(
                reviewer_target.provider.clone(),
                reviewer_target.model.clone(),
            );
        }
    }

    // Continue with the existing acquire+call ...
}
```

This requires plumbing a `workflow_metadata: WorkflowMetadata` field
into `EffectDriver`:

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
```

The metadata is set at workflow construction time by the engine, based
on `WorkflowConfig` + the resolved model selection.

### Step 3 — Workflow engine populates metadata

```rust
// crates/roko-runtime/src/workflow_engine.rs
impl WorkflowEngine {
    pub async fn run(&self, config: WorkflowRunConfig) -> Result<WorkflowRunReport> {
        let metadata = WorkflowMetadata {
            reviewer_target: if config.workflow.has_review {
                Some(resolve_reviewer_target(&config))
            } else { None },
            strategist_target: if config.workflow.has_strategy {
                Some(resolve_strategist_target(&config))
            } else { None },
        };
        self.driver.set_workflow_metadata(metadata);
        // ... existing run logic ...
    }
}
```

`resolve_reviewer_target` looks up `config.role_models.get("reviewer")`
or falls back to `config.default_model` and infers the provider via the
helper from Plan 09 (`infer_provider_from_model`).

### Step 4 — Add a speculation budget

To avoid runaway speculation (e.g., a misconfigured workflow that loops
spawn-implementer endlessly), add a per-run counter:

```rust
struct EffectDriver {
    speculation_budget: AtomicU32,    // default 3 per run
    // ...
}

if role == "implementer"
    && self.speculation_budget.fetch_sub(1, Ordering::Relaxed) > 0
{
    // pre_warm_for as above
}
```

If the budget hits 0, log once and stop speculating.

---

## Step-by-step execution

1. `git checkout -b perf/14-speculative-reviewer-prewarm`.
2. Add `pre_warm_for` to `WarmDispatchPool` (Step 1).
3. Define `WorkflowMetadata` and plumb through `EffectDriver` (Step 2).
4. Engine populates metadata (Step 3).
5. Speculation budget (Step 4).
6. Tests + macro-benchmark.
7. PR `perf(runtime): speculative reviewer pre-warm in standard
   workflow (novel)`.

---

## Anti-patterns / things NOT to do

- **Do NOT pre-warm during `roko run --workflow express`.** Express
  has no review phase; speculation wastes the slot.
- **Do NOT pre-warm the same `(provider, model)` twice.** The
  `try_lock` + post-lock check pattern in Step 1 dedupes; do not skip
  it.
- **Do NOT block the implementer on the speculation.** Speculation is
  a `tokio::spawn`. If you `.await` it, you've defeated the purpose.
- **Do NOT speculate in a tight loop** (e.g., per autofix attempt).
  The same model is already warm; further pre-warms are pure waste.
  The speculation budget caps this.
- **Do NOT extend speculation to "run the reviewer's prompt
  assembly".** Prompt content depends on implementer output; you
  cannot speculate over it. Speculation here is purely about the
  caller's TLS / model-call-service init.
- **Do NOT remove `evict_idle` calls.** A speculation that misses
  (e.g., implementer fails) leaves a warm slot. Eviction reclaims it
  after `idle_timeout`.
- **Do NOT log a `warn!` when speculation misses.** Misses are
  expected (~15 % of standard runs). Log at `debug!` only; `info!` is
  noise.
- **Do NOT bundle this with strategist speculation.** Strategist runs
  *first*, so there is nothing to speculate "during". A separate plan
  could speculate the implementer during strategist runtime, but
  that's a different change.

---

## Test plan

```rust
#[tokio::test]
async fn standard_workflow_reviewer_hits_warm_slot() {
    let factory = mock_factory_with_latency_ms(50);
    let pool = Arc::new(WarmDispatchPool::new(default_pool_cfg(), factory));
    let mut services = mock_effect_services();
    services.warm_pool = Some(Arc::clone(&pool));

    let engine = WorkflowEngine::new(services);
    let _ = engine.run(WorkflowRunConfig {
        workflow: WorkflowConfig::standard(),
        // ...
    }).await.unwrap();

    let metrics = pool.metrics().await;
    assert!(metrics.warm_hits >= 1, "expected at least one warm hit");
}

#[tokio::test]
async fn pre_warm_does_not_duplicate_slots() {
    let factory = mock_factory();
    let pool = Arc::new(WarmDispatchPool::new(default_pool_cfg(), factory));
    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    pool.clone().pre_warm_for("openai".into(), "gpt-4.1-mini".into());
    tokio::time::sleep(Duration::from_millis(50)).await;
    let metrics = pool.metrics().await;
    let slots = pool.slot_count_for_test().await;
    assert_eq!(slots, 1, "pre_warm_for must dedupe");
}

#[tokio::test]
async fn speculation_budget_caps_pre_warms() {
    let pool = Arc::new(WarmDispatchPool::new(default_pool_cfg(), mock_factory()));
    let mut services = mock_effect_services();
    services.warm_pool = Some(Arc::clone(&pool));
    let driver = EffectDriver::new_with_speculation_budget(services, "run".into(), tempdir(), 2);
    for _ in 0..5 {
        driver.spawn_agent("implementer", "task", None).await;
    }
    let pre_warms = pool.metrics().await.pre_warm_count_for_test;
    assert!(pre_warms <= 2);
}
```

---

## Rollback plan

- `[conductor.workflow.speculation]` config flag (default true)
  disables the speculative path.
- `git revert` the wiring; pool changes (Step 1) become dead code.

---

## Status check (acceptance)

- [ ] `WarmDispatchPool::pre_warm_for` exists.
- [ ] `WorkflowMetadata` plumbed through `EffectDriver`.
- [ ] Engine populates metadata for standard/full workflows.
- [ ] Speculation budget caps per-run pre-warms.
- [ ] Macro-benchmark improvement ≥20 ms for standard workflow.
