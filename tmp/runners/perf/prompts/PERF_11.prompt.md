# PERF_11: Warm pool config schema + serve startup + metrics route (B15 part 3)

## Task

Add the `[conductor.warm_pool]` config schema, pre-warm the pool on
`roko serve` startup, spawn a periodic eviction task, and expose a
`/v1/perf/warm-pool` route returning `WarmPoolMetrics`. Also wire the
PERF_04 deferred periodic JSONL logger flush.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_11](../ISSUE-TRACKER.md#perf_11)
- Plan: `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md` (steps 5-7)
- Bottleneck: B15 (final wiring)
- Performance contract: **C-10** (serve pre-warms on startup)
- Priority: P1
- Effort: ≈3 h
- Depends on: **PERF_10**
- Wave: 3

## Problem

After PERF_10, the pool is constructed empty for CLI. `roko serve`
startup never populates targets, so the first user dispatch always
pays a cold construction. We need:

1. A config schema so users can declare `[conductor.warm_pool]`.
2. `roko serve` startup that pre-warms based on the schema.
3. A periodic eviction task so idle slots don't hold sockets forever.
4. A debug HTTP route to inspect the pool state.
5. The deferred periodic flush of `JsonlLogger` (from PERF_04).

## Exact Changes

### Step 1 — Config schema

In `crates/roko-core/src/config/schema.rs` (or wherever
`ConductorConfig` lives — search `rg -n 'pub struct ConductorConfig'
crates/roko-core/`):

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WarmPoolConfigSchema {
    /// Master toggle. When false, no pool is constructed.
    pub enabled: bool,
    /// Maximum idle slots across all (provider, model) pairs.
    pub max_warm_slots: usize,
    /// Maximum concurrent active dispatches.
    pub max_active: usize,
    /// Idle slot TTL in seconds.
    pub idle_timeout_secs: u64,
    /// `roko serve`-only: pre-warm slots on startup.
    pub pre_warm_on_serve: bool,
    /// Provider keys to pre-warm (parallel arrays with `pre_warm_models`).
    pub pre_warm_providers: Vec<String>,
    /// Model slugs paired 1:1 with `pre_warm_providers`.
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
            pre_warm_providers: Vec::new(),
            pre_warm_models: Vec::new(),
        }
    }
}
```

Add to `ConductorConfig`:

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct ConductorConfig {
    // ... existing fields ...
    pub warm_pool: WarmPoolConfigSchema,
}
```

> **Anti-pattern check.** `#[serde(default)]` is mandatory on every new
> field so existing `roko.toml` files continue to load. Without it,
> users see a confusing parse error after upgrading.

### Step 2 — Document in default `roko.toml`

If the repo ships an example `roko.toml` (commonly under `examples/` or
the workspace root), add the new section at the bottom:

```toml
[conductor.warm_pool]
# Cache pre-built ModelCaller instances so dispatch skips the
# 20-50ms construction cost. See perf contract C-9.
enabled = true
max_warm_slots = 4
max_active = 8
idle_timeout_secs = 300

# `roko serve` only: pre-warm one slot per (provider, model) pair on
# startup so the first user request hits a warm slot.
pre_warm_on_serve = true
pre_warm_providers = ["openai", "anthropic"]
pre_warm_models = ["gpt-4.1-mini", "claude-sonnet-4"]
```

### Step 3 — `roko serve` startup pre-warm + periodic eviction

`crates/roko-serve/src/lib.rs` (or wherever the server bootstraps —
search `rg -n 'fn serve\|fn start_serve' crates/roko-serve/src/`):

```rust
use std::sync::Arc;
use std::time::Duration;
use roko_runtime::warm_dispatch_pool::{WarmDispatchPool, WarmPoolConfig};

// inside the serve startup function, after model_config is loaded:

let pool = if config.conductor.warm_pool.enabled {
    let cfg = WarmPoolConfig {
        max_warm_slots: config.conductor.warm_pool.max_warm_slots,
        max_active: config.conductor.warm_pool.max_active,
        idle_timeout: Duration::from_secs(config.conductor.warm_pool.idle_timeout_secs),
        pre_warm: config.conductor.warm_pool.pre_warm_on_serve,
        pre_warm_targets: config.conductor.warm_pool.pre_warm_providers
            .iter().cloned()
            .zip(config.conductor.warm_pool.pre_warm_models.iter().cloned())
            .collect(),
    };
    let factory = roko_cli::run::build_caller_factory(&model_config)
        .map_err(|e| anyhow::anyhow!("build warm-pool factory: {e}"))?;
    let pool = Arc::new(WarmDispatchPool::new(cfg, factory));

    if config.conductor.warm_pool.pre_warm_on_serve {
        pool.pre_warm().await;
    }

    // Periodic eviction (60 s tick).
    let evict_pool = Arc::clone(&pool);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await;   // fire-immediate is fine; we just constructed the pool
        loop {
            interval.tick().await;
            evict_pool.evict_idle().await;
        }
    });

    Some(pool)
} else {
    None
};

// Stash on AppState (or whatever the server's shared state struct is):
state.warm_pool = pool;
```

> If `build_caller_factory` lives in `roko-cli` (PERF_10) and is
> private, either re-export it as `pub` or move it into a shared
> location (`roko-agent/src/provider/factory.rs` is a sensible new
> home). Pick whichever has the smaller blast radius; document the
> choice in the commit body.

### Step 4 — Inject the pool into per-request `EffectServices`

The serve dispatch path constructs `EffectServices` per request (search
`rg -n 'EffectServices {' crates/roko-serve/src/`). Set the field:

```rust
let services = EffectServices {
    // ... existing init from per-request context ...
    warm_pool: state.warm_pool.clone(),    // Option<Arc<...>> clones cheaply
};
```

### Step 5 — Metrics HTTP route

`crates/roko-serve/src/routes/mod.rs` (or wherever routes are wired —
search `rg -n 'Router::new()\|axum::Router' crates/roko-serve/src/`):

```rust
async fn warm_pool_metrics(
    axum::extract::State(state): axum::extract::State<Arc<crate::AppState>>,
) -> impl axum::response::IntoResponse {
    use roko_runtime::warm_dispatch_pool::WarmPoolMetrics;
    let metrics = if let Some(pool) = state.warm_pool.as_ref() {
        pool.metrics().await
    } else {
        WarmPoolMetrics::default()
    };
    axum::Json(metrics)
}
```

Register in the router:

```rust
.route("/v1/perf/warm-pool", axum::routing::get(warm_pool_metrics))
```

If the existing serve uses `actix-web` or `tower` or a different
framework, adapt the handler signature accordingly. The API surface is
just "GET → JSON of `WarmPoolMetrics`".

### Step 6 — Periodic JsonlLogger flush (deferred from PERF_04)

In the same `roko serve` startup function (after the pool eviction
spawn), add:

```rust
if let Some(ref logger) = state.runtime_jsonl_logger {
    let logger = Arc::clone(logger);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(err) = logger.flush() {
                tracing::warn!(error = %err, "periodic jsonl flush failed");
            }
        }
    });
}
```

If `state.runtime_jsonl_logger` doesn't exist yet, this batch can
either:

- **Option A** (preferred): wire the logger into `AppState` here. The
  ownership pattern is `AppState { runtime_jsonl_logger: Option<Arc<JsonlLogger>>, ... }`.
- **Option B**: skip and document as deferred. The `Drop`-based flush
  in PERF_04 still ensures durability on serve shutdown; the periodic
  task is purely a "don't lose 8 KiB on a kill -9" insurance.

Pick A if it's a 5-line change, B otherwise — and document which.

### Step 7 — `Cargo.toml` deps

Likely already present, but verify in `crates/roko-serve/Cargo.toml`:

```toml
roko-runtime = { workspace = true, features = ["..."] }   # for WarmDispatchPool
roko-agent = { workspace = true }                          # if calling build_caller_factory
```

If `build_caller_factory` was promoted to `roko-agent`,
`crates/roko-cli/Cargo.toml` and `crates/roko-serve/Cargo.toml` may
need adjustment.

## Write Scope

- `crates/roko-core/src/config/schema.rs`
- `crates/roko-core/src/config/mod.rs`
- `crates/roko-serve/src/lib.rs`
- `crates/roko-serve/src/routes/mod.rs`

(Optional, only if Step 6 picks Option A or Step 3 promotes the
factory: add `crates/roko-serve/src/state.rs`,
`crates/roko-cli/src/run.rs`, `crates/roko-agent/src/provider/factory.rs`.
Update `batches.toml` `scope` BEFORE editing if you go this route.)

## Read-Only Context

- `crates/roko-runtime/src/warm_dispatch_pool.rs` (PERF_09)
- `crates/roko-runtime/src/effect_driver.rs` (PERF_10)
- `crates/roko-cli/src/run.rs::build_caller_factory` (PERF_10)
- `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md`

## Acceptance Criteria

- [ ] `WarmPoolConfigSchema` added to `roko-core::config::schema` with sane defaults.
- [ ] All fields use `#[serde(default)]` so existing `roko.toml` files keep working.
- [ ] Default `roko.toml` (or example) documents `[conductor.warm_pool]`.
- [ ] `roko serve` startup constructs the pool when `enabled = true`.
- [ ] Pre-warm runs on startup when `pre_warm_on_serve = true`.
- [ ] Periodic eviction task spawned with 60 s tick.
- [ ] `/v1/perf/warm-pool` route returns `WarmPoolMetrics` JSON.
- [ ] Pool injected into per-request `EffectServices`.
- [ ] Periodic JsonlLogger flush task spawned with 5 s tick (Option A) OR documented as deferred (Option B).

## Verify

```bash
# Schema added:
rg -n 'WarmPoolConfigSchema|warm_pool: WarmPool' crates/roko-core/src/

# Route registered:
rg -n '/v1/perf/warm-pool' crates/roko-serve/src/

# Manual smoke (post-merge):
cargo run --release -p roko-cli -- serve --port 8088 &
sleep 2
curl http://localhost:8088/v1/perf/warm-pool
# Expected: {"total_dispatches":0,"warm_hits":0,"cold_misses":N,"evictions":0,...}
# where N matches the count of pre_warm targets.
kill %1
```

## Do NOT

- Do NOT pre-warm more than 2-3 slots in `roko serve`. Each warm slot
  holds a `ModelCallService` clone; pre-warming 10 providers on
  startup causes a 5 s startup pause.
- Do NOT remove the periodic eviction task. Long-idle slots otherwise
  hold sockets forever.
- Do NOT enable the route on production without auth. `/v1/perf/*`
  routes should follow the same auth middleware as the rest of `/v1/`.
  If auth doesn't apply, that's a separate fix (file follow-up).
- Do NOT extend the schema with `pre_warm_concurrency` / "fancy"
  knobs in this batch. Keep config minimal; tune later based on data.
- Do NOT skip the `#[serde(default)]` on schema fields. Backwards
  compatibility for existing `roko.toml` is non-negotiable.
- Do NOT couple PERF_11 to the speculative-pre-warm in PERF_16. They
  layer on each other but live in separate batches.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_11 done <commit-sha>
```
