# PERF_10: Wire warm pool into EffectDriver + run.rs (B15 part 2)

## Task

Add `warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices`,
make `EffectDriver::spawn_agent` acquire from the pool when available,
construct the pool empty in `build_workflow_effect_services`, expose a
`infer_provider_from_model` helper, and add `warm_hit: bool` to
`FeedbackEvent::ModelCall`.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_10](../ISSUE-TRACKER.md#perf_10)
- Plan: `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md` (steps 3-4)
- Bottleneck: B15 (BOTTLENECK-ANALYSIS.md §B15)
- Performance contract: **C-9** (warm hit on second standard-workflow dispatch)
- Priority: P1
- Effort: ≈3 h
- Depends on: **PERF_09** (must land first)
- Wave: 2

## Problem

PERF_09 created the pool but did not wire it. `EffectDriver::spawn_agent`
still calls `services.model_caller.call(request)` directly, bypassing
the pool. `EffectServices` does not yet have a `warm_pool` field.

## Exact Changes

### Step 1 — Extend `EffectServices`

`crates/roko-runtime/src/effect_driver.rs:38`. Add the field:

```rust
use crate::warm_dispatch_pool::WarmDispatchPool;

pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
    /// Optional warm pool for fast model-caller dispatch. When `None`,
    /// every spawn_agent constructs cold via `model_caller`.
    pub warm_pool: Option<Arc<WarmDispatchPool>>,
}
```

Find every existing constructor of `EffectServices` (`rg -n
'EffectServices {' crates/`) and default `warm_pool: None`. Most live
in tests / mock services.

### Step 2 — Add `infer_provider_from_model` helper

In `crates/roko-agent/src/provider/mod.rs`, add a public helper near
`shared_http_client`:

```rust
/// Infer the provider key for a model slug.
///
/// Maps prefixes / common families to provider keys. This is the
/// single source of truth — the orchestrator's existing private
/// `infer_provider` should be migrated to call this.
pub fn infer_provider_from_model(model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    match m.as_str() {
        m if m.starts_with("gpt") || m.starts_with("o3") || m.starts_with("o4")
            || m.starts_with("o1") => "openai",
        m if m.starts_with("claude") => "anthropic",
        m if m.starts_with("gemini") || m.starts_with("models/gemini") => "gemini",
        m if m.starts_with("llama") || m.starts_with("scout")
            || m.starts_with("cerebras-") => "cerebras",
        m if m.starts_with("kimi") || m.starts_with("moonshot") => "moonshot",
        m if m.starts_with("glm") || m.starts_with("zai") => "zhipu",
        m if m.starts_with("qwen") || m.starts_with("perplexity") => "perplexity",
        m if m.starts_with("ollama") || m.starts_with("local-") => "ollama",
        _ => "openai",   // safe default; consumers can override via config
    }
}
```

> Migrate `crates/roko-cli/src/run.rs::infer_provider` (search via
> `rg -n 'fn infer_provider' crates/`) to delegate to the new helper.
> Both must agree.

### Step 3 — Use the pool in `spawn_agent`

`crates/roko-runtime/src/effect_driver.rs::spawn_agent`. Today the
relevant lines are around 149-186. Find:

```rust
let request = model_call_request(ModelCallRequestParts { ... });
// ... emit AgentSpawned event ...
let start = Instant::now();
let result = self.services.model_caller.call(request).await;
```

Change to:

```rust
let request = model_call_request(ModelCallRequestParts { ... });
// ... emit AgentSpawned event ...

let provider = roko_agent::provider::infer_provider_from_model(&request.model);
let start = Instant::now();

let (result, warm_hit) = if let Some(ref pool) = self.services.warm_pool {
    if let Some(guard) = pool.acquire(provider, &request.model).await {
        let res = guard.call(request.clone()).await;
        guard.release().await;
        (res, true)
    } else {
        // Pool exhausted or no factory match — fall back to default
        // caller. This is a cold path; record metrics accordingly.
        let res = self.services.model_caller.call(request).await;
        (res, false)
    }
} else {
    let res = self.services.model_caller.call(request).await;
    (res, false)
};

let latency_ms = duration_millis(start);
```

Pass `warm_hit` into the `FeedbackEvent::ModelCall` construction
further down.

### Step 4 — Add `warm_hit: bool` to `FeedbackEvent::ModelCall`

`crates/roko-core/src/foundation.rs` (or wherever `FeedbackEvent` is
defined — search `rg -n 'enum FeedbackEvent' crates/`). Find the
`ModelCall` variant, add the field with serde defaults:

```rust
ModelCall {
    // ... existing fields ...
    /// True when this dispatch acquired from a WarmDispatchPool slot.
    /// Defaults to false for backward compatibility with old logs.
    #[serde(default)]
    warm_hit: bool,
},
```

Pass `warm_hit` in every `FeedbackEvent::ModelCall { ... }`
construction site (`rg -n 'FeedbackEvent::ModelCall' crates/`). Most
will get `warm_hit: false`; only the new `spawn_agent` path passes the
real value.

### Step 5 — Construct the pool in `build_workflow_effect_services`

`crates/roko-cli/src/run.rs:426`. Today:

```rust
fn build_workflow_effect_services(
    workdir: &std::path::Path,
    config: &Config,
    mut model_config: RokoConfig,
    selection: &EffectiveModelSelection,
) -> anyhow::Result<EffectServices> {
    model_config.agent.default_model = selection.effective_model_key.clone();
    let services = ServiceFactory::build(ServiceConfig { ... })?;
    Ok(services.effect_services())
}
```

After `services.effect_services()`, augment with the pool:

```rust
let mut services = services.effect_services();

// Construct an empty WarmDispatchPool. CLI never pre-warms (one-shot
// runs benefit only from the 2nd dispatch onward; pre-warming would
// cost the first dispatch latency). Serve startup populates targets in
// PERF_11.
let pool_config = WarmPoolConfig {
    max_warm_slots: 4,
    max_active: 8,
    idle_timeout: Duration::from_secs(300),
    pre_warm: false,
    pre_warm_targets: vec![],
};
let factory = build_caller_factory(&model_config)?;
services.warm_pool = Some(Arc::new(WarmDispatchPool::new(pool_config, factory)));

Ok(services)
```

Add the helper at the bottom of the same file:

```rust
fn build_caller_factory(model_config: &RokoConfig) -> anyhow::Result<ModelCallerFactory> {
    let cfg = Arc::new(model_config.clone());
    Ok(Arc::new(move |_provider: &str, model: &str| -> Option<Arc<dyn ModelCaller>> {
        let svc = ModelCallService::new(model.to_string())
            .with_config((*cfg).clone());
        Some(Arc::new(svc) as Arc<dyn ModelCaller>)
    }))
}
```

> **Anti-pattern note.** The factory clones `RokoConfig` per miss. If
> `RokoConfig` becomes large (it currently isn't), bump to
> `Arc<RokoConfig>` and clone the Arc only. The wrapper above already
> uses `Arc<RokoConfig>` internally to pre-empt this.

### Step 6 — Workflow engine evict-idle hook (optional, recommended)

`crates/roko-runtime/src/workflow_engine.rs::WorkflowEngine::run`. After
the workflow exits, call:

```rust
if let Some(ref pool) = self.driver.services.warm_pool {
    pool.evict_idle().await;
}
```

For one-shot CLI runs this is a no-op (process exits immediately). It
matters for `roko serve` (PERF_11 will add a periodic eviction task as
well; this is a belt-and-suspenders measure).

### Step 7 — Test

Add to `crates/roko-runtime/src/effect_driver.rs` test module:

```rust
#[tokio::test]
async fn second_dispatch_in_standard_workflow_uses_warm_slot() {
    use crate::warm_dispatch_pool::{WarmDispatchPool, WarmPoolConfig};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let factory: ModelCallerFactory = Arc::new(move |_p, model| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        Some(Arc::new(MockCaller::new(model.to_string())) as Arc<dyn ModelCaller>)
    });
    let pool = Arc::new(WarmDispatchPool::new(WarmPoolConfig::default(), factory));
    let mut services = mock_effect_services();
    services.warm_pool = Some(Arc::clone(&pool));

    let driver = EffectDriver::new(services, "run-1".into(), tempdir().path().into());
    let _ = driver.spawn_agent("implementer", "task", None).await;
    let _ = driver.spawn_agent("reviewer", "review", None).await;

    let metrics = pool.metrics().await;
    assert!(metrics.warm_hits >= 1, "expected ≥1 warm hit on second dispatch");
    assert_eq!(counter.load(Ordering::Relaxed), 1, "factory should be invoked once");
}
```

(Use whatever `mock_effect_services`/`MockCaller` shim already exists
in this file's tests.)

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-agent/src/provider/mod.rs`

## Read-Only Context

- `crates/roko-runtime/src/warm_dispatch_pool.rs` (created by PERF_09)
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-agent/src/model_call_service.rs`
- `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md`

## Acceptance Criteria

- [ ] `EffectServices.warm_pool: Option<Arc<WarmDispatchPool>>` field added (default `None`).
- [ ] `EffectDriver::spawn_agent` consults the pool when present, falls back to `services.model_caller` otherwise.
- [ ] Released slot via explicit `pool.release(idx)` (NOT `Drop`).
- [ ] `infer_provider_from_model` helper added to `roko_agent::provider` (single source of truth).
- [ ] `crates/roko-cli/src/run.rs::infer_provider` migrated to delegate.
- [ ] `build_workflow_effect_services` constructs the pool empty (CLI does not pre-warm).
- [ ] `build_caller_factory(&model_config)` returns the `ModelCallerFactory`.
- [ ] `FeedbackEvent::ModelCall` carries `warm_hit: bool` (with `#[serde(default)]`).
- [ ] Test `second_dispatch_in_standard_workflow_uses_warm_slot` passes.
- [ ] Workflow engine `evict_idle()` hook added at run exit.

## Verify

```bash
# Pool field present:
rg -n 'pub warm_pool:' crates/roko-runtime/src/effect_driver.rs

# All EffectServices constructors initialise it:
rg -n 'EffectServices {' crates/ --type rust

# No Drop-based release introduced:
rg -n 'impl Drop for WarmSlotGuard' crates/

# Provider inference single source of truth:
rg -n 'fn infer_provider' crates/
# Expected: one in roko-agent::provider, one in roko-cli that delegates.
```

## Do NOT

- Do NOT pre-warm in CLI (AP-DISPATCH-4). The first dispatch's TLS
  handshake is the same with or without; pre-warming costs latency
  without a win in one-shot mode.
- Do NOT release the slot via `Drop` (AP-DISPATCH-5). `Drop` cannot be
  async; release explicitly with `guard.release().await`.
- Do NOT key warm slots only by provider (AP-DISPATCH-3). Two requests
  with different models must not share a caller.
- Do NOT remove the fallback to `services.model_caller`. Tests that
  don't construct a pool rely on it.
- Do NOT skip the `#[serde(default)]` on the new `warm_hit` field.
  Old `efficiency.jsonl` files must still parse.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).
- Do NOT extend `Cargo.toml` deps. Everything needed already exists in
  `roko-runtime` / `roko-agent`.

## Tracker update

```
tracker: PERF_10 done <commit-sha>
```
