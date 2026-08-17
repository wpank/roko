# 09 — WarmDispatchPool wired into EffectDriver (B15 + B04 partial)

> Bottleneck: every `EffectDriver::spawn_agent` constructs a fresh
> `ModelCallService` via `create_agent_for_model`, paying 20–50 ms per
> dispatch (API path) or 200–500 ms (Claude CLI path).
>
> Target savings: 20–50 ms per warm hit; 100–500 ms saved across a
> standard workflow (2 dispatches) or a multi-task plan (10+ dispatches).
> Effort: 10–12 h. Risk: medium.

---

## Goal & success criteria

After this change:

1. A new `WarmDispatchPool` lives in `crates/roko-runtime/src/`. It
   manages `Arc<dyn ModelCaller>` instances keyed by `(provider,
   model)`.
2. `EffectServices` carries an optional `Arc<WarmDispatchPool>`. When
   present, `spawn_agent` acquires from the pool instead of calling
   `create_agent_for_model` afresh.
3. `roko serve` pre-warms the pool on startup with the providers/models
   declared in `roko.toml`. `roko run` constructs the pool empty (cold
   start tier 3 is fine for a single dispatch; the pool still helps on
   the second/third dispatch in standard/full workflows).
4. The pool exposes metrics (`warm_hits`, `cold_misses`,
   `avg_acquire_us`) consumed by the existing learning subsystem.
5. The Claude CLI path is **not** changed in this plan. CLI subprocess
   warming has independent constraints; see "Out of scope" below.

Done when:

- New unit tests cover acquire, release, eviction, pre-warm, and metric
  recording.
- Macro-benchmark on standard workflow shows ≥30 ms improvement vs
  plan-08 baseline (single warm hit on the reviewer dispatch).
- Macro-benchmark on `roko plan run` (3-task plan, same provider) shows
  ≥80 ms improvement (2 warm hits).
- `roko serve` pre-warms two slots on startup (verified via
  `/v1/perf/warm-pool` route).

---

## Background

- Source: `WARM-POOL-DESIGN.md` (full design),
  `BOTTLENECK-ANALYSIS.md` §B15, `OPTIMIZATION-PLAYBOOK.md` §11.
- `MultiAgentPool` exists at `crates/roko-agent/src/multi_pool.rs` but
  is **not wired** into `EffectDriver`. It targets `Arc<dyn Agent>`
  (whole-agent caching), not `Arc<dyn ModelCaller>` (per-call caching).
- The new `WarmDispatchPool` is the lighter abstraction. We wrap the
  `ModelCaller` rather than the `Agent` because dispatch goes through
  `ModelCallService::call`, not the legacy `Agent::run` path.
- The `SHARED_HTTP_CLIENT` (`crates/roko-agent/src/provider/mod.rs:93`)
  already pools TCP+TLS at 90 s; the warm pool layers on top by caching
  the more expensive **logical agent** state (router config, contract,
  prompt assembler).

---

## Files to read first

| File | Why |
|---|---|
| `WARM-POOL-DESIGN.md` (in this folder) | Full architecture rationale; do **not** skip. |
| `crates/roko-runtime/src/effect_driver.rs` | Where `ModelCaller::call` is invoked. Edit site. |
| `crates/roko-runtime/src/workflow_engine.rs` | Pool lifecycle (pre-warm, evict). |
| `crates/roko-agent/src/multi_pool.rs` | Existing pool primitives — reuse `WarmReusePolicy`. |
| `crates/roko-agent/src/model_call_service.rs` | The thing we cache. |
| `crates/roko-agent/src/provider/mod.rs` | `create_agent_for_model`, the cold-construction path. |
| `crates/roko-cli/src/run.rs::build_workflow_effect_services` | Where to construct & pass the pool. |
| `crates/roko-serve/src/runtime.rs` (or `lib.rs`) | Where serve's startup happens. |

---

## Code-level plan

The plan implements `WarmDispatchPool` step by step, then wires it.

### Step 1 — New module `warm_dispatch_pool.rs`

Create `crates/roko-runtime/src/warm_dispatch_pool.rs`. The full
implementation is spelled out in `WARM-POOL-DESIGN.md` §"Step 1". Copy
that into the file with the following adjustments:

1. Replace `unsafe { buf.as_mut_vec() }` patterns (none in the design
   doc — clean).
2. Use `tokio::sync::Mutex` for the slots vec; the acquire/release
   crosses awaits.
3. Import `roko_core::foundation::ModelCaller` (the trait we cache).
4. The `factory` closure type:

   ```rust
   pub type ModelCallerFactory =
       Arc<dyn Fn(&str, &str) -> Option<Arc<dyn ModelCaller>> + Send + Sync>;
   ```

   Producing one is the caller's job — usually constructed by
   capturing a `RokoConfig` clone.

5. Add a `metrics()` accessor returning `WarmPoolMetrics` by value
   (cloned snapshot).

Add to `crates/roko-runtime/src/lib.rs`:

```rust
pub mod warm_dispatch_pool;
pub use warm_dispatch_pool::{
    WarmDispatchPool, WarmPoolConfig, WarmPoolMetrics, WarmSlotGuard,
};
```

### Step 2 — Extend `EffectServices`

```rust
// crates/roko-runtime/src/effect_driver.rs
use crate::warm_dispatch_pool::WarmDispatchPool;

pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
    /// NEW: optional warm pool for fast dispatch.
    pub warm_pool: Option<Arc<WarmDispatchPool>>,
}
```

Update existing `EffectServices` constructors to default `warm_pool:
None`. The trait surface stays backward-compatible.

### Step 3 — Use the pool in `spawn_agent`

In `EffectDriver::spawn_agent` (`effect_driver.rs:88`), replace the
direct `model_caller.call(request)` invocation with:

```rust
let provider = infer_provider_from_model(&request.model);   // helper
let caller: Arc<dyn ModelCaller> = if let Some(ref pool) = self.services.warm_pool {
    if let Some(guard) = pool.acquire(&provider, &request.model).await {
        // RAII: guard drops at end of this block, returning slot to pool.
        let result = guard.call(request.clone()).await?;
        // ... existing post-call handling ...
        return PipelineInput::AgentCompleted { /* ... */ };
    }
    // Pool exhausted or no factory match — fall back.
    Arc::clone(&self.services.model_caller)
} else {
    Arc::clone(&self.services.model_caller)
};

let result = caller.call(request).await;
// ... existing post-call handling ...
```

> **`infer_provider_from_model`.** Either (a) duplicate the logic from
> `roko-cli`'s `infer_provider`, or (b) add a public helper to
> `roko_agent::provider` that maps a model slug to a provider key. (b)
> is preferred — single source of truth.

### Step 4 — Construct the pool in `build_workflow_effect_services`

```rust
// crates/roko-cli/src/run.rs
fn build_workflow_effect_services(
    workdir: &Path,
    config: &Config,
    mut model_config: RokoConfig,
    selection: &EffectiveModelSelection,
) -> anyhow::Result<EffectServices> {
    // ... existing build ...
    let mut services = services.effect_services();

    if config.conductor.warm_pool.enabled {
        let pool_config = WarmPoolConfig {
            max_warm_slots: config.conductor.warm_pool.max_warm_slots,
            max_active: config.conductor.warm_pool.max_active,
            idle_timeout: Duration::from_secs(config.conductor.warm_pool.idle_timeout_secs),
            pre_warm: false,                  // CLI never pre-warms
            pre_warm_targets: vec![],
        };
        let factory: ModelCallerFactory = build_caller_factory(&model_config)?;
        services.warm_pool = Some(Arc::new(WarmDispatchPool::new(pool_config, factory)));
    }

    Ok(services)
}
```

`build_caller_factory` captures the `RokoConfig` and returns a closure
that constructs a `ModelCallService` for any `(provider, model)`:

```rust
fn build_caller_factory(model_config: &RokoConfig) -> anyhow::Result<ModelCallerFactory> {
    let cfg = model_config.clone();
    Ok(Arc::new(move |provider: &str, model: &str| -> Option<Arc<dyn ModelCaller>> {
        let svc = ModelCallService::new(model.to_string())
            .with_config(cfg.clone());
        Some(Arc::new(svc) as Arc<dyn ModelCaller>)
    }))
}
```

> **Anti-pattern alert.** The factory clones the entire `RokoConfig`
> on every miss. If `RokoConfig` becomes large (it currently isn't,
> but watch this), refactor to `Arc<RokoConfig>` and clone the Arc.

### Step 5 — Add config schema

`crates/roko-core/src/config/mod.rs` (or `schema.rs`):

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WarmPoolConfigSchema {
    pub enabled: bool,
    pub max_warm_slots: usize,
    pub max_active: usize,
    pub idle_timeout_secs: u64,
    pub pre_warm_on_serve: bool,
    pub pre_warm_providers: Vec<String>,
    pub pre_warm_models: Vec<String>,
}

impl Default for WarmPoolConfigSchema {
    fn default() -> Self {
        Self {
            enabled: true,
            max_warm_slots: 4,
            max_active: 8,
            idle_timeout_secs: 300,
            pre_warm_on_serve: true,
            pre_warm_providers: vec![],
            pre_warm_models: vec![],
        }
    }
}
```

Add to the conductor section so users can override:

```toml
# roko.toml
[conductor.warm_pool]
enabled = true
max_warm_slots = 4
idle_timeout_secs = 300
pre_warm_on_serve = true
pre_warm_providers = ["openai", "anthropic"]
pre_warm_models = ["gpt-4.1-mini", "claude-sonnet-4"]
```

### Step 6 — `roko serve` pre-warm + eviction

```rust
// crates/roko-serve/src/runtime.rs (or lib.rs init)
let pool_targets: Vec<(String, String)> = config.conductor.warm_pool.pre_warm_providers
    .iter().cloned()
    .zip(config.conductor.warm_pool.pre_warm_models.iter().cloned())
    .collect();
let pool = Arc::new(WarmDispatchPool::new(
    WarmPoolConfig {
        pre_warm: true,
        pre_warm_targets: pool_targets,
        // ...other fields from config...
    },
    build_caller_factory(&model_config)?,
));
pool.pre_warm().await;

// Periodic eviction.
let evict_pool = Arc::clone(&pool);
tokio::spawn(async move {
    let mut t = tokio::time::interval(Duration::from_secs(60));
    loop {
        t.tick().await;
        evict_pool.evict_idle().await;
    }
});

// Inject into EffectServices used per request.
state.warm_pool = Some(pool);
```

Add a `/v1/perf/warm-pool` HTTP route returning `WarmPoolMetrics` as
JSON for debugging:

```rust
async fn warm_pool_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Some(pool) = &state.warm_pool {
        Json(pool.metrics().await)
    } else {
        Json(WarmPoolMetrics::default())
    }
}
```

### Step 7 — Wire metrics into the learning loop

`crates/roko-learn` already records `EfficiencySignal` with latency.
After each dispatch, augment the signal with a `warm_hit: bool` field.
The cascade router can then prefer providers with high warm-hit rates.

```rust
// In the post-call handling of spawn_agent:
let warm_hit = matches!(acquire_outcome, AcquireOutcome::WarmHit);
feedback_sink.record(FeedbackEvent::ModelCall {
    // ... existing fields ...
    warm_hit,
}).await;
```

If `FeedbackEvent::ModelCall` doesn't have a `warm_hit` field yet, add
one (non-breaking: serde default false). This is small; do it in the
same PR.

---

## Step-by-step execution

1. `git checkout -b perf/09-warm-dispatch-pool`.
2. Create `warm_dispatch_pool.rs` (Step 1). `cargo build -p roko-runtime`.
3. Add tests for `acquire`, `release`, `evict_idle`, `pre_warm`, and
   `metrics` (skeletons in `WARM-POOL-DESIGN.md` §Implementation Plan).
4. Extend `EffectServices` (Step 2). `cargo build -p roko-runtime`.
5. Wire into `spawn_agent` (Step 3). `cargo build -p roko-runtime` and
   `-p roko-cli`.
6. Construct in `build_workflow_effect_services` (Step 4) behind the
   config flag.
7. Add config schema (Step 5). Update default `roko.toml`.
8. Pre-warm in `roko serve` (Step 6). Add the metrics route.
9. Wire `warm_hit` into feedback (Step 7).
10. Macro-benchmark.
11. PR `perf(runtime): WarmDispatchPool wired into EffectDriver (B15)`.

---

## Anti-patterns / things NOT to do

- **Do NOT pre-warm in `roko run` (CLI one-shot).** Pre-warming costs
  the first dispatch latency (≈100 ms) without the win for one-shot
  invocations. `roko run` benefits only from the 2nd dispatch onward
  (reviewer in standard workflow). Construct empty; let the first
  dispatch populate the pool.
- **Do NOT cache `Arc<dyn Agent>` in this plan** — that is what
  `MultiAgentPool` does and it carries Claude-CLI complications. The
  scope here is `Arc<dyn ModelCaller>` only (API path).
- **Do NOT release the slot via Drop.** `Drop` cannot be async; the
  pool's release must be explicit (called at the end of `spawn_agent`)
  OR via a background task. The design doc uses an explicit
  `pool.release(idx)` call; mirror that.
- **Do NOT key warm slots only by provider.** Two requests to the same
  provider with different models can share TLS but should not share a
  caller (different routing/temperature/etc. config). Key by `(provider,
  model)` as designed.
- **Do NOT make `WarmDispatchPool` `Send + 'static` only.** Use
  `Arc<WarmDispatchPool>` everywhere; the pool itself owns no thread.
- **Do NOT bypass the pool when `--no-cache` is set** without also
  bypassing `SHARED_HTTP_CLIENT` (the bigger win). This flag is for
  debugging routing, not for "make everything slow"; document it
  clearly in the help text.
- **Do NOT pre-warm more than 2-3 slots in `roko serve`.** Each warm
  slot holds a `ModelCallService`, which holds config clones. Memory
  cost is small (KBs) but TLS handshakes for every pre-warm target on
  startup add up; the user does not want a 5 s startup pause.
- **Do NOT extend the pool to claude CLI in this plan.** CLI warming
  requires keeping subprocesses alive, handling stdin/stdout streams,
  and CLI-specific session state. That is plan-09b territory.
- **Do NOT measure pool benefit by warm-hit count alone.** Hit count
  matters only if hits replace expensive cold misses. The metric of
  record is `avg_acquire_us` and end-to-end macro-benchmark wall time.

---

## Test plan

Skeleton (full versions in `WARM-POOL-DESIGN.md`):

```rust
#[tokio::test]
async fn acquire_warm_slot_returns_caller() { /* ... */ }

#[tokio::test]
async fn cold_miss_constructs_via_factory() { /* ... */ }

#[tokio::test]
async fn pre_warm_creates_idle_slots() { /* ... */ }

#[tokio::test]
async fn evict_idle_removes_old_slots() {
    // create slot, mark idle, advance time, evict_idle, expect empty
}

#[tokio::test]
async fn metrics_record_hit_and_miss() { /* ... */ }

#[tokio::test]
async fn second_dispatch_in_standard_workflow_uses_warm_slot() {
    let pool = WarmDispatchPool::new(...);
    let services = EffectServices { warm_pool: Some(Arc::new(pool)), ... };
    let driver = EffectDriver::new(services, "run-1".into(), tempdir.path().into());
    // Drive a 2-step workflow.
    let _ = driver.spawn_agent("implementer", "task", None).await;
    let _ = driver.spawn_agent("reviewer", "review", None).await;
    let metrics = driver.services.warm_pool.as_ref().unwrap().metrics().await;
    assert!(metrics.warm_hits >= 1);
}
```

Macro-benchmark: `roko run --workflow standard --gates none "Reply
hello"` before/after; expect ≥30 ms improvement.

For `roko serve`: hit `/v1/perf/warm-pool` and confirm
`warm_hits / total_dispatches > 0.5` after a few requests.

---

## Rollback plan

- The pool is opt-in via `[conductor.warm_pool] enabled = false`.
  Disabling it restores the cold-construct path immediately.
- `git revert` the wiring commits while keeping `warm_dispatch_pool.rs`
  as dead code — the module still compiles and is unit-tested.
- Hard rollback: delete the module + revert config schema.

---

## Status check (acceptance)

- [ ] `WarmDispatchPool` exists, compiles, and is unit-tested.
- [ ] `EffectServices.warm_pool` exists; `spawn_agent` uses it when
      present.
- [ ] `roko serve` pre-warms on startup with config-driven targets.
- [ ] `/v1/perf/warm-pool` returns metrics.
- [ ] Macro-benchmark improvement ≥30 ms for standard workflow.
- [ ] No regressions in existing `effect_driver` and `workflow_engine`
      tests.
