# PERF_09: WarmDispatchPool module (B15 part 1)

## Task

Create `crates/roko-runtime/src/warm_dispatch_pool.rs` containing the
`WarmDispatchPool` type, its config, metrics, slot guard, and full unit
tests. **No wiring** to `EffectDriver` or `roko serve` — those land in
PERF_10 and PERF_11. This batch is the standalone module.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_09](../ISSUE-TRACKER.md#perf_09)
- Plan: `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md` (steps 1-2)
- Bottleneck: B15 (BOTTLENECK-ANALYSIS.md §B15)
- Performance contract: **C-9** (warm acquire <5 µs)
- Priority: P1
- Effort: ≈4 h
- Depends on: none
- Wave: 1

## Problem

`MultiAgentPool` exists at `crates/roko-agent/src/multi_pool.rs` but
caches whole `Arc<dyn Agent>` (heavyweight, claude-CLI-aware). The
warm path the workflow engine wants is lighter:
`Arc<dyn ModelCaller>` keyed by `(provider, model)`.

This batch creates the new abstraction in `roko-runtime` (next to
`effect_driver.rs`) so the next batch (PERF_10) can wire it through
`EffectServices` without touching agent internals.

The full design is in `tmp/solutions/perf/WARM-POOL-DESIGN.md` —
**read it before starting**.

## Exact Changes

### Step 1 — Create `crates/roko-runtime/src/warm_dispatch_pool.rs`

```rust
//! WarmDispatchPool — process-wide pool of pre-built `ModelCaller`
//! instances.
//!
//! Layered on top of `SHARED_HTTP_CLIENT` (which already pools TLS
//! connections at 90 s). The warm pool caches the more expensive
//! *logical agent* state: provider resolution, ModelCallService
//! construction, safety contract reference, prompt assembler glue.
//!
//! See `tmp/solutions/perf/WARM-POOL-DESIGN.md` for the full design.
//! See perf contract C-9 for the acquire-latency invariant.

use std::sync::Arc;
use std::time::{Duration, Instant};

use roko_core::foundation::{ModelCaller, ModelCallRequest, ModelCallResponse};
use tokio::sync::Mutex;

/// Configuration knobs for a WarmDispatchPool.
#[derive(Debug, Clone)]
pub struct WarmPoolConfig {
    /// Maximum number of warm (idle) slots across all providers.
    pub max_warm_slots: usize,
    /// Maximum concurrent active dispatches.
    pub max_active: usize,
    /// How long a warm slot stays alive without use.
    pub idle_timeout: Duration,
    /// Whether to pre-warm slots on pool creation (use for `roko serve`).
    pub pre_warm: bool,
    /// Provider/model pairs to pre-warm. Only honoured when `pre_warm`
    /// is true.
    pub pre_warm_targets: Vec<(String, String)>,
}

impl Default for WarmPoolConfig {
    fn default() -> Self {
        Self {
            max_warm_slots: 4,
            max_active: 8,
            idle_timeout: Duration::from_secs(300),
            pre_warm: false,
            pre_warm_targets: Vec::new(),
        }
    }
}

/// Snapshot of pool runtime metrics. Cloned out of the pool's internal
/// counters; safe to pass across threads / serialize as JSON.
#[derive(Debug, Default, Clone)]
pub struct WarmPoolMetrics {
    pub total_dispatches: u64,
    /// Acquires that found a matching idle slot.
    pub warm_hits: u64,
    /// Acquires that had to construct a fresh slot.
    pub cold_misses: u64,
    /// Slots evicted by `evict_idle`.
    pub evictions: u64,
    /// Highest concurrent slot count observed.
    pub peak_active: u64,
    /// Microseconds, average across all dispatches.
    pub avg_acquire_us: f64,
    /// Milliseconds, average across all dispatches.
    pub avg_dispatch_ms: f64,
}

/// Factory: given `(provider, model)`, construct a `ModelCaller` (or
/// return None if no factory match exists).
pub type ModelCallerFactory =
    Arc<dyn Fn(&str, &str) -> Option<Arc<dyn ModelCaller>> + Send + Sync>;

#[derive(Debug)]
enum SlotState {
    Idle,
    Active { run_id: String, since: Instant },
    Draining,
}

struct WarmSlot {
    provider: String,
    model: String,
    caller: Arc<dyn ModelCaller>,
    created_at: Instant,
    last_used: Instant,
    dispatches_served: u64,
    state: SlotState,
}

/// Pool that manages warm `ModelCaller`s for fast dispatch.
pub struct WarmDispatchPool {
    config: WarmPoolConfig,
    slots: Mutex<Vec<WarmSlot>>,
    metrics: Mutex<WarmPoolMetrics>,
    factory: ModelCallerFactory,
}

impl WarmDispatchPool {
    pub fn new(config: WarmPoolConfig, factory: ModelCallerFactory) -> Self {
        Self {
            config,
            slots: Mutex::new(Vec::new()),
            metrics: Mutex::new(WarmPoolMetrics::default()),
            factory,
        }
    }

    /// Acquire a model caller for the given provider and model.
    ///
    /// Returns `None` only when the factory cannot construct a caller
    /// (unknown provider) AND no idle slot exists. The caller MUST
    /// invoke `release(idx)` after dispatch (Drop cannot be async).
    pub async fn acquire(
        self: &Arc<Self>,
        provider: &str,
        model: &str,
    ) -> Option<WarmSlotGuard> {
        let start = Instant::now();
        let mut slots = self.slots.lock().await;

        // 1. Exact match (same provider + same model).
        if let Some(idx) = slots.iter().position(|s|
            matches!(s.state, SlotState::Idle)
            && s.provider == provider
            && s.model == model
        ) {
            slots[idx].state = SlotState::Active {
                run_id: String::new(),
                since: Instant::now(),
            };
            let caller = Arc::clone(&slots[idx].caller);
            self.update_peak(slots.len()).await;
            drop(slots);
            self.record_hit(start).await;
            tracing::debug!(target: "roko_perf", provider, model, "warm slot hit");
            return Some(WarmSlotGuard { pool: Arc::clone(self), slot_idx: idx, caller });
        }

        // 2. Same provider, different model. Re-target the slot
        //    (model is a request param, not baked into the caller for
        //    most providers; this trades a small re-config cost for a
        //    huge HTTP/TLS reuse benefit).
        if let Some(idx) = slots.iter().position(|s|
            matches!(s.state, SlotState::Idle) && s.provider == provider
        ) {
            slots[idx].model = model.to_string();
            slots[idx].state = SlotState::Active {
                run_id: String::new(),
                since: Instant::now(),
            };
            let caller = Arc::clone(&slots[idx].caller);
            self.update_peak(slots.len()).await;
            drop(slots);
            self.record_hit(start).await;
            tracing::debug!(target: "roko_perf", provider, model, "warm slot hit (model retarget)");
            return Some(WarmSlotGuard { pool: Arc::clone(self), slot_idx: idx, caller });
        }

        // 3. Construct cold (if under max).
        let total_cap = self.config.max_warm_slots + self.config.max_active;
        if slots.len() < total_cap {
            if let Some(caller) = (self.factory)(provider, model) {
                let slot = WarmSlot {
                    provider: provider.to_string(),
                    model: model.to_string(),
                    caller: Arc::clone(&caller),
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    dispatches_served: 0,
                    state: SlotState::Active {
                        run_id: String::new(),
                        since: Instant::now(),
                    },
                };
                let idx = slots.len();
                slots.push(slot);
                self.update_peak(slots.len()).await;
                drop(slots);
                self.record_miss(start).await;
                tracing::debug!(target: "roko_perf", provider, model, "warm slot cold-construct");
                return Some(WarmSlotGuard { pool: Arc::clone(self), slot_idx: idx, caller });
            }
        }

        drop(slots);
        self.record_miss(start).await;
        None
    }

    /// Pre-warm slots for configured targets. Idempotent (will not
    /// duplicate slots).
    pub async fn pre_warm(self: &Arc<Self>) {
        for (provider, model) in self.config.pre_warm_targets.clone() {
            // Re-use acquire's no-duplicate logic by checking slots
            // directly here.
            let mut slots = self.slots.lock().await;
            let already = slots.iter().any(|s|
                s.provider == provider
                && s.model == model
                && matches!(s.state, SlotState::Idle));
            if already { continue; }

            if let Some(caller) = (self.factory)(&provider, &model) {
                slots.push(WarmSlot {
                    provider,
                    model,
                    caller,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    dispatches_served: 0,
                    state: SlotState::Idle,
                });
            }
        }
    }

    /// Evict idle slots past their timeout.
    pub async fn evict_idle(&self) {
        let mut slots = self.slots.lock().await;
        let deadline = Instant::now() - self.config.idle_timeout;
        let before = slots.len();
        slots.retain(|s|
            !matches!(s.state, SlotState::Idle) || s.last_used > deadline
        );
        let evicted = (before - slots.len()) as u64;
        drop(slots);
        if evicted > 0 {
            self.metrics.lock().await.evictions += evicted;
        }
    }

    /// Release a slot back to idle. Called by `WarmSlotGuard::release`
    /// (which the consumer of `acquire()` MUST invoke after dispatch).
    async fn release(&self, idx: usize) {
        let mut slots = self.slots.lock().await;
        if idx < slots.len() {
            slots[idx].state = SlotState::Idle;
            slots[idx].last_used = Instant::now();
            slots[idx].dispatches_served += 1;
        }
    }

    pub async fn metrics(&self) -> WarmPoolMetrics {
        self.metrics.lock().await.clone()
    }

    async fn record_hit(&self, start: Instant) {
        let mut m = self.metrics.lock().await;
        m.total_dispatches += 1;
        m.warm_hits += 1;
        let us = start.elapsed().as_micros() as f64;
        m.avg_acquire_us = (m.avg_acquire_us * (m.total_dispatches - 1) as f64 + us)
            / m.total_dispatches as f64;
    }

    async fn record_miss(&self, start: Instant) {
        let mut m = self.metrics.lock().await;
        m.total_dispatches += 1;
        m.cold_misses += 1;
        let us = start.elapsed().as_micros() as f64;
        m.avg_acquire_us = (m.avg_acquire_us * (m.total_dispatches - 1) as f64 + us)
            / m.total_dispatches as f64;
    }

    async fn update_peak(&self, current: usize) {
        let mut m = self.metrics.lock().await;
        if (current as u64) > m.peak_active {
            m.peak_active = current as u64;
        }
    }

    #[cfg(test)]
    pub async fn slot_count_for_test(&self) -> usize {
        self.slots.lock().await.len()
    }
}

/// Guard returned by `acquire()`. Drop is sync so release is explicit.
/// The consumer pattern in the workflow engine is:
///
/// ```ignore
/// let guard = pool.acquire(provider, model).await.unwrap();
/// let response = guard.call(request).await;
/// guard.release().await;
/// ```
pub struct WarmSlotGuard {
    pool: Arc<WarmDispatchPool>,
    slot_idx: usize,
    pub caller: Arc<dyn ModelCaller>,
}

impl WarmSlotGuard {
    pub async fn call(&self, request: ModelCallRequest) -> roko_core::error::Result<ModelCallResponse> {
        self.caller.call(request).await
    }

    /// Return the slot to the warm pool. Call exactly once after the
    /// dispatch completes (success or failure).
    pub async fn release(self) {
        self.pool.release(self.slot_idx).await;
    }
}
```

> **Why explicit `release` and not `Drop`?** `Drop` cannot be async,
> and we need to grab the pool's tokio mutex to flip `Idle`. The
> alternative — spawning a release task in `Drop` — leaks tasks during
> shutdown and obscures debug traces. Explicit release matches the
> pattern documented in `WARM-POOL-DESIGN.md`.

### Step 2 — Re-export from `crates/roko-runtime/src/lib.rs`

Add to the module list (alphabetical order if maintained, otherwise
near `effect_driver`):

```rust
pub mod warm_dispatch_pool;
pub use warm_dispatch_pool::{
    WarmDispatchPool, WarmPoolConfig, WarmPoolMetrics, WarmSlotGuard,
    ModelCallerFactory,
};
```

### Step 3 — Add unit tests

Append to `crates/roko-runtime/src/warm_dispatch_pool.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock ModelCaller that just echoes its model name.
    struct MockCaller {
        model: String,
        constructions: Arc<AtomicUsize>,
    }

    impl MockCaller {
        fn new(model: &str, counter: Arc<AtomicUsize>) -> Self {
            counter.fetch_add(1, Ordering::Relaxed);
            Self { model: model.to_string(), constructions: counter }
        }
    }

    #[async_trait]
    impl ModelCaller for MockCaller {
        async fn call(&self, _req: ModelCallRequest) -> roko_core::error::Result<ModelCallResponse> {
            Ok(ModelCallResponse {
                content: format!("hello from {}", self.model),
                model: self.model.clone(),
                usage: Default::default(),
                cost_usd: 0.0,
                latency_ms: 1,
            })
        }
    }

    fn mock_factory(counter: Arc<AtomicUsize>) -> ModelCallerFactory {
        Arc::new(move |_provider, model| {
            Some(Arc::new(MockCaller::new(model, Arc::clone(&counter))) as Arc<dyn ModelCaller>)
        })
    }

    fn cfg() -> WarmPoolConfig {
        WarmPoolConfig {
            max_warm_slots: 2,
            max_active: 4,
            idle_timeout: Duration::from_secs(60),
            pre_warm: false,
            pre_warm_targets: vec![],
        }
    }

    #[tokio::test]
    async fn acquire_warm_slot_returns_caller() {
        let counter = Arc::new(AtomicUsize::new(0));
        let pool = Arc::new(WarmDispatchPool::new(cfg(), mock_factory(counter.clone())));

        let g1 = pool.acquire("openai", "gpt-4.1-mini").await.expect("acquire 1");
        g1.release().await;

        let g2 = pool.acquire("openai", "gpt-4.1-mini").await.expect("acquire 2");
        // Same caller (warm hit) — counter only incremented once.
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        g2.release().await;

        let m = pool.metrics().await;
        assert_eq!(m.total_dispatches, 2);
        assert_eq!(m.warm_hits, 1);
        assert_eq!(m.cold_misses, 1);
    }

    #[tokio::test]
    async fn cold_miss_constructs_via_factory() {
        let counter = Arc::new(AtomicUsize::new(0));
        let pool = Arc::new(WarmDispatchPool::new(cfg(), mock_factory(counter.clone())));
        let g1 = pool.acquire("openai", "gpt-4.1-mini").await.expect("a");
        g1.release().await;
        let g2 = pool.acquire("anthropic", "claude-sonnet-4").await.expect("b");
        g2.release().await;
        // Different (provider, model) ⇒ two constructions.
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn pre_warm_creates_idle_slots() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut config = cfg();
        config.pre_warm = true;
        config.pre_warm_targets = vec![
            ("openai".into(), "gpt-4.1-mini".into()),
            ("anthropic".into(), "claude-sonnet-4".into()),
        ];
        let pool = Arc::new(WarmDispatchPool::new(config, mock_factory(counter.clone())));
        pool.pre_warm().await;
        assert_eq!(pool.slot_count_for_test().await, 2);
        assert_eq!(counter.load(Ordering::Relaxed), 2);

        // pre_warm is idempotent: running again does not add more.
        pool.pre_warm().await;
        assert_eq!(pool.slot_count_for_test().await, 2);
    }

    #[tokio::test]
    async fn evict_idle_removes_old_slots() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut config = cfg();
        config.idle_timeout = Duration::from_millis(20);
        let pool = Arc::new(WarmDispatchPool::new(config, mock_factory(counter.clone())));
        let g = pool.acquire("openai", "gpt-4.1-mini").await.expect("a");
        g.release().await;
        assert_eq!(pool.slot_count_for_test().await, 1);

        tokio::time::sleep(Duration::from_millis(40)).await;
        pool.evict_idle().await;
        assert_eq!(pool.slot_count_for_test().await, 0);

        let m = pool.metrics().await;
        assert_eq!(m.evictions, 1);
    }

    #[tokio::test]
    async fn metrics_record_hit_and_miss() {
        let counter = Arc::new(AtomicUsize::new(0));
        let pool = Arc::new(WarmDispatchPool::new(cfg(), mock_factory(counter.clone())));
        let g1 = pool.acquire("openai", "x").await.expect("a");
        g1.release().await;
        let g2 = pool.acquire("openai", "x").await.expect("b");
        g2.release().await;
        let g3 = pool.acquire("anthropic", "y").await.expect("c");
        g3.release().await;
        let m = pool.metrics().await;
        assert_eq!(m.total_dispatches, 3);
        assert_eq!(m.warm_hits, 1);
        assert_eq!(m.cold_misses, 2);
        assert!(m.avg_acquire_us > 0.0);
    }
}
```

> Adjust the `ModelCallResponse` literal at the top of the mock to
> match the actual struct fields in `crates/roko-core/src/foundation.rs`.
> The fields shown (`content`, `model`, `usage`, `cost_usd`,
> `latency_ms`) match the design doc but you should verify against the
> live code.

### Step 4 — `Cargo.toml` (no new deps expected)

`tokio` (with `sync` feature) and `roko-core` are already deps of
`roko-runtime`. `async-trait` is needed for the mock test; check
`crates/roko-runtime/Cargo.toml`:

```toml
[dev-dependencies]
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt", "time", "sync"] }
```

If `async-trait` is missing in `[dev-dependencies]`, add it.

## Write Scope

- `crates/roko-runtime/src/warm_dispatch_pool.rs`
- `crates/roko-runtime/src/lib.rs`
- `crates/roko-runtime/Cargo.toml`

## Read-Only Context

- `crates/roko-agent/src/multi_pool.rs` (existing pool primitives — context only)
- `crates/roko-agent/src/session.rs` (`WarmReusePolicy`)
- `crates/roko-core/src/foundation.rs` (`ModelCaller`, `ModelCallRequest`, `ModelCallResponse`)
- `tmp/solutions/perf/WARM-POOL-DESIGN.md` (architecture reference)
- `tmp/solutions/perf/implementation/09-warm-dispatch-pool.md` (full plan)

## Acceptance Criteria

- [ ] New file `crates/roko-runtime/src/warm_dispatch_pool.rs` exists.
- [ ] `WarmDispatchPool`, `WarmPoolConfig`, `WarmPoolMetrics`, `WarmSlotGuard`, `ModelCallerFactory` are exported from `crates/roko-runtime/src/lib.rs`.
- [ ] `acquire(provider, model) -> Option<WarmSlotGuard>` records hit/miss metrics correctly.
- [ ] `pre_warm()` populates configured targets idempotently.
- [ ] `evict_idle()` removes slots past `idle_timeout`.
- [ ] Pool uses `tokio::sync::Mutex` (acquire crosses awaits).
- [ ] `metrics()` returns a snapshot by value.
- [ ] Unit tests cover acquire/release/eviction/pre_warm/metrics.
- [ ] No `Drop` impl on `WarmSlotGuard` (release is explicit).

## Verify

```bash
# Module exports:
rg -n 'WarmDispatchPool|WarmPoolConfig' crates/roko-runtime/src/lib.rs
# Expected: at least one re-export line.

# No accidental Drop on guard:
rg -n 'impl Drop for WarmSlotGuard' crates/roko-runtime/src/warm_dispatch_pool.rs
# Expected: 0 matches.

# Tests:
cargo test -p roko-runtime --release warm_dispatch_pool
```

## Do NOT

- Do NOT wire the pool into `EffectServices` or `EffectDriver` in this
  batch. That is **PERF_10**'s job. This batch is the standalone
  module so PERF_10 can plug it in cleanly.
- Do NOT cache `Arc<dyn Agent>` (that's `MultiAgentPool`'s territory and
  carries Claude-CLI complications). Cache `Arc<dyn ModelCaller>` only.
- Do NOT release the slot via `Drop` (AP-DISPATCH-5). Use explicit
  `WarmSlotGuard::release(self).await`.
- Do NOT key warm slots only by provider (AP-DISPATCH-3). Two requests
  to the same provider with different models must not share a caller's
  routing/temperature config.
- Do NOT make `WarmDispatchPool` `Send + 'static` only. Use
  `Arc<WarmDispatchPool>` everywhere.
- Do NOT bypass the pool when `--no-cache` is set without also
  bypassing `SHARED_HTTP_CLIENT`. This batch does not add a `--no-pool`
  flag (PERF_11 will, via config).
- Do NOT pre-warm more than 2-3 slots in `roko serve`. Each warm slot
  holds a `ModelCallService` clone; pre-warming 10 providers on startup
  causes a 5 s startup pause.
- Do NOT extend the pool to claude CLI in this plan. CLI warming is a
  different abstraction; defer to a future batch (plan-09b).
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_09 done <commit-sha>
```
