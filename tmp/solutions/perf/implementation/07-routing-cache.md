# 07 — Routing Decision Cache (B06)

> Bottleneck: every dispatch in the orchestrator reads the entire
> `efficiency.jsonl` file synchronously, scores all candidate models,
> and queries the neuro store per candidate. After 100+ runs this is a
> 150–300 ms hit per dispatch.
>
> Target savings: 100–200 ms per dispatch (additive across plan tasks).
> Effort: ≈4 h. Risk: medium (stale routing decisions).

---

## Goal & success criteria

After this change:

1. Efficiency signals are loaded **at most once every 10 s** per
   process; the parsed signals live in an `Arc<RwLock<...>>` shared
   across all dispatches.
2. Routing decisions are memoized for 5 minutes per
   `(task_type, complexity_band, recent_quality_decile, healthy_models)`
   key.
3. Cache invalidation triggers automatically on writes to
   `efficiency.jsonl` (or via TTL fallback).
4. The TUI / serve dashboards still see fresh data within ≤10 s.

Done when:

- A unit test confirms two dispatches in the same 10 s window share one
  `load_efficiency_signals_sync` call.
- Macro-benchmark on `roko plan run` (3-task plan) shows ≥150 ms
  improvement vs the plan-06 baseline.
- Cache invalidation happens within 1 s of an `efficiency.jsonl` append
  (verified via integration test).

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B06,
  `OPTIMIZATION-PLAYBOOK.md` §7.
- Live call sites:

  ```text
  crates/roko-cli/src/orchestrate.rs
    198    use roko_learn::cascade::load_efficiency_signals_sync
    6073   load_efficiency_signals_sync(&efficiency_path)
   14808   load_efficiency_signals_sync(...)
  ```

- The cascade router scoring loop at `orchestrate.rs:14705-15041` runs
  per candidate model. With 8-12 models and per-candidate neuro queries,
  this dominates dispatch time on plan executions.
- The `roko_learn::cascade` module exposes
  `load_efficiency_signals_sync` (synchronous read of the entire JSONL
  log + parse). The "sync" name is a clue — it was added because no one
  wanted to pay the await cost in the routing path.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` ~14700-15100 | Routing dispatch hot path. |
| `crates/roko-learn/src/cascade/mod.rs` (or wherever `load_efficiency_signals_sync` is defined) | Loader signature + signal shape. |
| `crates/roko-learn/src/efficiency.rs` | `EfficiencySignal` struct + writer (so you know when invalidation should fire). |
| `crates/roko-learn/src/cascade_router.rs` | `CascadeRouter::observe`, `apply_*`, `explain_route` — what the cache feeds into. |

---

## Code-level plan

### Step 1 — Create the routing cache module

New file: `crates/roko-learn/src/cascade/routing_cache.rs`.

```rust
//! Routing-decision and efficiency-signals memo.
//!
//! Two layers of caching:
//!   1. EfficiencySignals: TTL-based, shared per process.
//!   2. Routing decisions: keyed by task fingerprint, TTL 5 min,
//!      invalidated when EfficiencySignals refresh.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use crate::efficiency::EfficiencySignal;
use crate::cascade::load_efficiency_signals_sync;

const SIGNALS_TTL: Duration = Duration::from_secs(10);
const DECISIONS_TTL: Duration = Duration::from_secs(5 * 60);
const DECISIONS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct CachedSignals {
    signals: Arc<Vec<EfficiencySignal>>,
    loaded_at: Instant,
    /// File mtime at load time; used for fast invalidation.
    file_mtime: Option<std::time::SystemTime>,
}

#[derive(Clone)]
pub struct CachedDecision {
    pub model: String,
    pub explanation_blob: Option<String>,   // optional pre-rendered explanation
    cached_at: Instant,
}

pub struct RoutingCache {
    efficiency_path: PathBuf,
    signals: RwLock<Option<CachedSignals>>,
    decisions: RwLock<lru::LruCache<u64, CachedDecision>>,
}

impl RoutingCache {
    pub fn new(efficiency_path: PathBuf) -> Self {
        Self {
            efficiency_path,
            signals: RwLock::new(None),
            decisions: RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(DECISIONS_CAPACITY).unwrap(),
            )),
        }
    }

    /// Return cached signals if fresh, otherwise reload from disk.
    pub fn signals(&self) -> Arc<Vec<EfficiencySignal>> {
        // Fast path: read lock + freshness check.
        if let Some(ref cached) = *self.signals.read() {
            let mtime = std::fs::metadata(&self.efficiency_path)
                .and_then(|m| m.modified()).ok();
            let unchanged = mtime == cached.file_mtime;
            if unchanged && cached.loaded_at.elapsed() < SIGNALS_TTL {
                return Arc::clone(&cached.signals);
            }
        }

        // Slow path: write lock + reload.
        let mut guard = self.signals.write();
        // Double-check (another writer may have refreshed).
        if let Some(ref cached) = *guard {
            if cached.loaded_at.elapsed() < SIGNALS_TTL {
                return Arc::clone(&cached.signals);
            }
        }
        let mtime = std::fs::metadata(&self.efficiency_path)
            .and_then(|m| m.modified()).ok();
        let signals = Arc::new(load_efficiency_signals_sync(&self.efficiency_path).unwrap_or_default());
        *guard = Some(CachedSignals {
            signals: Arc::clone(&signals),
            loaded_at: Instant::now(),
            file_mtime: mtime,
        });
        // Mtime moved → invalidate decisions because they depended on the old signal set.
        self.decisions.write().clear();
        signals
    }

    pub fn lookup_decision(&self, key: u64) -> Option<CachedDecision> {
        let mut guard = self.decisions.write();   // LruCache::get needs &mut
        guard.get(&key).and_then(|d| {
            (d.cached_at.elapsed() < DECISIONS_TTL).then(|| d.clone())
        })
    }

    pub fn record_decision(&self, key: u64, decision: CachedDecision) {
        self.decisions.write().put(key, decision);
    }
}
```

Add to `crates/roko-learn/src/cascade/mod.rs`:

```rust
pub mod routing_cache;
pub use routing_cache::{RoutingCache, CachedDecision};
```

### Step 2 — Define the cache key

```rust
// In orchestrate.rs (or a small helper module).
fn routing_cache_key(
    task_type: &str,
    complexity: TaskComplexityBand,
    recent_quality_decile: u8,
    healthy_models: &[String],
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    task_type.hash(&mut h);
    (complexity as u8).hash(&mut h);
    recent_quality_decile.hash(&mut h);
    for m in healthy_models {
        m.hash(&mut h);
    }
    h.finish()
}
```

> **Decile, not float.** Quality is an `f64`. Hashing floats directly
> defeats the cache because tiny variations produce new keys. Bucketing
> into 10 deciles (0–9) collapses noise. Adjust the bucket count if it
> proves too coarse (a future-improvement comment is fine).

### Step 3 — Wire the cache into `Orchestrator`

`orchestrate.rs`'s `Orchestrator` struct should own one `RoutingCache`
per workdir:

```rust
pub struct Orchestrator {
    // ... existing fields ...
    routing_cache: Arc<RoutingCache>,
}

impl Orchestrator {
    pub fn new(...) -> Self {
        let efficiency_path = workdir.join(".roko/learn/efficiency.jsonl");
        let routing_cache = Arc::new(RoutingCache::new(efficiency_path));
        Self { /* ..., */ routing_cache }
    }
}
```

### Step 4 — Replace the hot loops

In the `dispatch_with_routing` function (~`orchestrate.rs:14700`):

**Before:**
```rust
if let Ok(efficiency_signals) = load_efficiency_signals_sync(&efficiency_path) {
    // ... score every candidate against signals ...
}
```

**After:**
```rust
let signals = self.routing_cache.signals();
// ... score every candidate against &*signals ...
```

After scoring picks a model:
```rust
let key = routing_cache_key(task_type, complexity, recent_quality_decile, &healthy_models);
self.routing_cache.record_decision(key, CachedDecision {
    model: chosen_model.clone(),
    explanation_blob: explanation_summary,
    cached_at: Instant::now(),
});
```

At the top of `dispatch_with_routing`, short-circuit when the cache
already has a decision for the same key:

```rust
if let Some(cached) = self.routing_cache.lookup_decision(key) {
    tracing::debug!(target: "roko_perf", "routing cache hit: {}", cached.model);
    return Ok(cached.model);    // or compose the same Selection as before
}
```

### Step 5 — Active invalidation on writes

The cascade router's observation methods (`observe`, `record_*`) write
to `efficiency.jsonl` indirectly via the learning runtime. Where the
write happens (`crates/roko-learn/src/runtime_feedback.rs::record_completed_run`
or `crates/roko-learn/src/efficiency.rs`), call:

```rust
routing_cache.invalidate_signals();   // bumps loaded_at into the past
```

If wiring the cache through is too invasive, rely on the mtime check in
`signals()` — the next `signals()` call will refresh because the file
mtime moved. This is the recommended approach (zero coupling between
`roko-learn` writers and the cache).

### Step 6 — Disable cache in `--no-cache` mode

For users debugging routing bugs:

```rust
// config / CLI
roko run --no-routing-cache "..."
```

Plumb a boolean `disabled: bool` into `RoutingCache`; when true, every
call goes through to disk. Tests use this mode by default.

---

## Step-by-step execution

1. `git checkout -b perf/07-routing-cache`.
2. Create `routing_cache.rs` (Step 1). Add `lru` dep to `roko-learn` if
   not present.
3. Add the key helper (Step 2).
4. Wire into `Orchestrator` (Step 3).
5. Replace the hot loops (Step 4); `cargo check -p roko-cli` after each
   call site.
6. Confirm mtime invalidation works (Step 5); add the
   `--no-routing-cache` flag (Step 6).
7. Tests + macro-benchmark.
8. Open PR `perf(orchestrator): cache efficiency signals + routing
   decisions (B06)`.

---

## Anti-patterns / things NOT to do

- **Do NOT cache routing decisions across runs of `roko run`.** Each
  CLI invocation is a fresh process; cross-process caching would need a
  disk persist, which costs more than the routing decision itself.
  In-process only.
- **Do NOT forget mtime invalidation.** A 10 s TTL alone misses the
  case where a *parallel* `roko run` writes a new efficiency signal
  inside the same 10 s window. The mtime check fixes this for free.
- **Do NOT hash the raw `f64` quality score** as a cache key (collisions
  guaranteed across reasonable runs but no actual hits). Decile
  bucketing is the contract.
- **Do NOT cache the `RoutingExplanation` deeply** (it has nested
  vectors of model scores). Cache the chosen model + a short blob; if
  the user wants the full explanation later, re-derive it.
- **Do NOT replace `load_efficiency_signals_sync` with an async
  variant** in this plan. The bottleneck is repeated calls, not call
  duration. Async loaders introduce backpressure semantics that aren't
  needed if calls are amortized.
- **Do NOT extend the TTL to "save a few more reads".** The dashboards
  in `roko serve` rely on the cascade router seeing fresh data within
  10 s for live A/B reports. Longer TTL = stale dashboard.
- **Do NOT build a write-through cache that updates on every `observe`
  call.** That couples write paths to cache state and gets racy fast.
  Mtime + TTL invalidation is the only sane design.

---

## Test plan

```rust
#[tokio::test]
async fn signals_are_cached_within_ttl() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("efficiency.jsonl");
    std::fs::write(&path, "{}\n").unwrap();
    let cache = RoutingCache::new(path.clone());

    let counter = install_load_counter();
    let _ = cache.signals();
    let _ = cache.signals();
    let _ = cache.signals();
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn signals_invalidate_on_mtime_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("efficiency.jsonl");
    std::fs::write(&path, "{}\n").unwrap();
    let cache = RoutingCache::new(path.clone());
    let _ = cache.signals();

    std::thread::sleep(Duration::from_millis(20));
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(&path).unwrap()
        .write_all(b"{}\n").unwrap();

    let counter = install_load_counter();
    let _ = cache.signals();
    assert_eq!(counter.load(Ordering::Relaxed), 1, "expected reload after mtime change");
}

#[tokio::test]
async fn decision_cache_hits_for_same_key() {
    let cache = RoutingCache::new("/nonexistent".into());
    cache.record_decision(42, CachedDecision {
        model: "gpt-4.1-mini".into(),
        explanation_blob: None,
        cached_at: Instant::now(),
    });
    let hit = cache.lookup_decision(42).expect("should hit");
    assert_eq!(hit.model, "gpt-4.1-mini");
}

#[tokio::test]
async fn decision_cache_misses_after_ttl() {
    let cache = RoutingCache::new("/nonexistent".into());
    cache.record_decision(42, CachedDecision {
        model: "x".into(), explanation_blob: None,
        cached_at: Instant::now() - Duration::from_secs(310),
    });
    assert!(cache.lookup_decision(42).is_none());
}
```

Macro-benchmark: `roko plan run plans/test-3-tasks/`. Compare wall time
before/after. Target ≥150 ms total improvement (≈50 ms × 3 tasks).

---

## Rollback plan

- `--no-routing-cache` flag disables the cache; users can opt out
  immediately if a stale-decision bug is reported.
- `git revert` the orchestrator changes; the standalone
  `routing_cache.rs` file becomes dead code but compiles fine.
- Hard rollback: delete `routing_cache.rs` and revert the cargo deps.

---

## Status check (acceptance)

- [ ] `RoutingCache` exists in `roko-learn` with both signals and
      decisions caches.
- [ ] All `load_efficiency_signals_sync` call sites in `orchestrate.rs`
      go through the cache.
- [ ] Mtime invalidation works (verified by test).
- [ ] `--no-routing-cache` flag exists and bypasses the cache.
- [ ] Macro-benchmark improvement of ≥150 ms recorded for `plan run`.
