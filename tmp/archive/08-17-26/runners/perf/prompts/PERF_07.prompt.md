# PERF_07: Routing decision cache (B06)

## Task

Build a per-process routing-decision cache that memoizes (a) the parsed
`efficiency.jsonl` signals (TTL 10 s + mtime invalidation) and (b)
routing decisions keyed by task profile (TTL 5 min, LRU 1024). Wire it
into `Orchestrator` so dispatch hot-path stops re-reading the entire
JSONL log.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_07](../ISSUE-TRACKER.md#perf_07)
- Plan: `tmp/solutions/perf/implementation/07-routing-cache.md`
- Bottleneck: B06 (BOTTLENECK-ANALYSIS.md §B06)
- Performance contract: **C-7** (cached signals + decisions)
- Priority: P1
- Effort: ≈4 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-cli/src/orchestrate.rs` calls
`load_efficiency_signals_sync(...)` from the dispatch hot path
(≈line 6073 and ≈line 14808). Each call:

1. Reads the entire `.roko/learn/efficiency.jsonl` (50 KB after 100
   runs, 500 KB after 1000).
2. Parses every line into `EfficiencySignal`.
3. The orchestrator then loops over 8-12 candidate models, scoring each
   against signals and querying the neuro store per candidate.

Total cost: 150-300 ms per dispatch. Multiplied across plan tasks, this
is the single largest non-network cost in the dispatch path.

## Exact Changes

### Step 1 — Create the cache module

New file: `crates/roko-learn/src/cascade/routing_cache.rs`.

```rust
//! Routing-decision and efficiency-signals memo (perf contract C-7).
//!
//! Two layers of caching:
//!   1. EfficiencySignals: TTL-based (10 s) + mtime invalidation,
//!      shared across all dispatches in a process.
//!   2. Routing decisions: keyed by task fingerprint, TTL 5 min,
//!      bounded by LRU capacity 1024. Invalidated en bloc when
//!      EfficiencySignals refresh (because the inputs to scoring
//!      changed).
//!
//! **Cache key inputs (decisions):** `(task_type, complexity_band,
//! recent_quality_decile, healthy_models)`. Quality is bucketed into
//! deciles to avoid float-hash key explosion (see AP-CACHE-5).
//!
//! **Anti-patterns avoided:** AP-CACHE-2/4/5/7, AP-ROUTE-1/2/3/4.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use lru::LruCache;
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
    file_mtime: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub struct CachedDecision {
    pub model: String,
    pub explanation_blob: Option<String>,
    cached_at: Instant,
}

impl CachedDecision {
    pub fn new(model: impl Into<String>, explanation: Option<String>) -> Self {
        Self {
            model: model.into(),
            explanation_blob: explanation,
            cached_at: Instant::now(),
        }
    }
}

pub struct RoutingCache {
    efficiency_path: PathBuf,
    signals: RwLock<Option<CachedSignals>>,
    decisions: RwLock<LruCache<u64, CachedDecision>>,
    /// When true, all caches bypass and go straight to disk/scoring.
    /// Set via the `--no-routing-cache` CLI flag.
    disabled: bool,
}

impl RoutingCache {
    pub fn new(efficiency_path: PathBuf) -> Self {
        Self {
            efficiency_path,
            signals: RwLock::new(None),
            decisions: RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(DECISIONS_CAPACITY).unwrap(),
            )),
            disabled: false,
        }
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Return the cached signals. On TTL expiry or mtime change, reload
    /// from disk and clear the decisions cache.
    pub fn signals(&self) -> Arc<Vec<EfficiencySignal>> {
        if self.disabled {
            return Arc::new(load_efficiency_signals_sync(&self.efficiency_path).unwrap_or_default());
        }

        // Fast path: read lock, freshness check.
        {
            let cached = self.signals.read();
            if let Some(ref cached) = *cached {
                let mtime = std::fs::metadata(&self.efficiency_path)
                    .and_then(|m| m.modified()).ok();
                if mtime == cached.file_mtime
                    && cached.loaded_at.elapsed() < SIGNALS_TTL
                {
                    return Arc::clone(&cached.signals);
                }
            }
        }

        // Slow path: write lock + reload.
        let mtime = std::fs::metadata(&self.efficiency_path)
            .and_then(|m| m.modified()).ok();
        let signals = Arc::new(
            load_efficiency_signals_sync(&self.efficiency_path).unwrap_or_default()
        );

        // Double-check pattern: another writer may have refreshed.
        let mut guard = self.signals.write();
        if let Some(ref existing) = *guard {
            if existing.loaded_at.elapsed() < SIGNALS_TTL
                && existing.file_mtime == mtime
            {
                return Arc::clone(&existing.signals);
            }
        }
        *guard = Some(CachedSignals {
            signals: Arc::clone(&signals),
            loaded_at: Instant::now(),
            file_mtime: mtime,
        });
        // Drop signals lock before grabbing decisions lock to avoid
        // potential ordering issues if both ever come up together.
        drop(guard);
        // Mtime moved → invalidate decisions; their inputs changed.
        self.decisions.write().clear();

        tracing::debug!(target: "roko_perf", "routing signals reloaded");
        signals
    }

    pub fn lookup_decision(&self, key: u64) -> Option<CachedDecision> {
        if self.disabled { return None; }
        // LruCache::get bumps recency, so it needs &mut.
        let mut guard = self.decisions.write();
        guard.get(&key).and_then(|d| {
            if d.cached_at.elapsed() < DECISIONS_TTL {
                tracing::debug!(target: "roko_perf", model = %d.model, "routing cache hit");
                Some(d.clone())
            } else {
                None
            }
        })
    }

    pub fn record_decision(&self, key: u64, decision: CachedDecision) {
        if self.disabled { return; }
        self.decisions.write().put(key, decision);
    }

    /// Test-only accessor.
    #[cfg(test)]
    pub fn signals_len_for_test(&self) -> usize {
        self.signals.read().as_ref().map_or(0, |c| c.signals.len())
    }
}
```

### Step 2 — Add to `crates/roko-learn/src/cascade/mod.rs`

```rust
pub mod routing_cache;
pub use routing_cache::{RoutingCache, CachedSignals, CachedDecision};
```

If `crates/roko-learn/src/cascade/` does not exist as a module
directory yet, check `crates/roko-learn/src/cascade.rs` (single-file
module) or wherever `load_efficiency_signals_sync` lives. Add the
`routing_cache.rs` next to it; use a `mod` declaration that matches
the existing layout.

### Step 3 — Add `lru` to `crates/roko-learn/Cargo.toml` if absent

```toml
lru = "0.12"
parking_lot = "0.12"   # if not already present
```

### Step 4 — Cache key helper

Decide where this lives. Two options:

- Beside `routing_cache.rs` as `pub fn routing_cache_key(...)`.
- In `crates/roko-cli/src/orchestrate.rs` as a private helper.

Pick the orchestrator location (the inputs are orchestrator-side):

```rust
// In crates/roko-cli/src/orchestrate.rs near the dispatch hot path:
fn routing_cache_key(
    task_type: &str,
    complexity: TaskComplexityBand,
    recent_quality_decile: u8,        // 0-9
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

fn quality_to_decile(q: f64) -> u8 {
    let clamped = q.clamp(0.0, 1.0);
    ((clamped * 10.0).floor() as u8).min(9)
}
```

### Step 5 — Wire `RoutingCache` into `Orchestrator`

`crates/roko-cli/src/orchestrate.rs::Orchestrator` (find the struct
declaration near the top of the file). Add a field:

```rust
pub struct Orchestrator {
    // ... existing fields ...
    routing_cache: Arc<RoutingCache>,
}

impl Orchestrator {
    pub fn new(...) -> Self {
        // ... existing init ...
        let efficiency_path = workdir.join(".roko/learn/efficiency.jsonl");
        let routing_cache = Arc::new(RoutingCache::new(efficiency_path));
        Self {
            // ... existing fields ...
            routing_cache,
        }
    }

    pub fn with_routing_cache_disabled(mut self) -> Self {
        // Take + replace because Arc<RoutingCache> isn't mutable.
        let path = self.routing_cache.efficiency_path.clone();
        self.routing_cache = Arc::new(RoutingCache::new(path).with_disabled(true));
        self
    }
}
```

(If `efficiency_path` is private, add a `pub(crate) fn path(&self) ->
&Path` accessor on `RoutingCache`.)

### Step 6 — Replace the hot loops

In `crates/roko-cli/src/orchestrate.rs::dispatch_with_routing`
(≈line 14705+), find the existing pattern:

```rust
if let Ok(efficiency_signals) = load_efficiency_signals_sync(&efficiency_path) {
    // ... use efficiency_signals to score candidates ...
}
```

Replace with:

```rust
let signals = self.routing_cache.signals();
// `signals` is Arc<Vec<EfficiencySignal>>; use &*signals where the
// old code used &efficiency_signals.
```

Add the decision-cache check **before** the scoring loop:

```rust
let decile = quality_to_decile(recent_quality_score);
let key = routing_cache_key(task_type, complexity, decile, &healthy_models);

if let Some(cached) = self.routing_cache.lookup_decision(key) {
    return Ok(cached.model);    // OR construct the existing Selection
                                // type from `cached.model`.
}
```

After scoring picks `chosen_model`:

```rust
let summary = explanation
    .as_ref()
    .map(|e| serde_json::to_string(e).unwrap_or_default());
self.routing_cache.record_decision(
    key,
    CachedDecision::new(chosen_model.clone(), summary),
);
```

Repeat for the other call site at ≈line 6073.

### Step 7 — CLI flag

In `crates/roko-cli/src/main.rs`, add to the global `--` flags:

```rust
/// Disable the in-process routing decision cache. Use only when
/// debugging routing bugs; this restores the per-dispatch
/// load_efficiency_signals_sync behaviour.
#[clap(long, global = true)]
pub no_routing_cache: bool,
```

In the orchestrator construction site (`run_once` or a sibling), pass
the flag:

```rust
let mut orchestrator = Orchestrator::new(...);
if cli.no_routing_cache {
    orchestrator = orchestrator.with_routing_cache_disabled();
}
```

### Step 8 — Tests

In `crates/roko-learn/src/cascade/routing_cache.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn signals_are_cached_within_ttl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("efficiency.jsonl");
        std::fs::write(&path, "").unwrap();
        let cache = RoutingCache::new(path.clone());
        let _ = cache.signals();
        let _ = cache.signals();
        let _ = cache.signals();
        // We cannot directly assert "1 disk read" without instrumenting
        // load_efficiency_signals_sync. Instead, assert the cache slot
        // is populated:
        assert!(cache.signals.read().is_some());
    }

    #[tokio::test]
    async fn signals_invalidate_on_mtime_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("efficiency.jsonl");
        std::fs::write(&path, "").unwrap();
        let cache = RoutingCache::new(path.clone());

        let _ = cache.signals();
        let mtime_before = cache.signals.read().as_ref().unwrap().file_mtime;

        std::thread::sleep(Duration::from_millis(20));
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();

        let _ = cache.signals();
        let mtime_after = cache.signals.read().as_ref().unwrap().file_mtime;
        assert_ne!(mtime_before, mtime_after);
    }

    #[test]
    fn decision_cache_hits_for_same_key() {
        let cache = RoutingCache::new(std::path::PathBuf::from("/nonexistent"));
        cache.record_decision(42, CachedDecision::new("gpt-4.1-mini", None));
        let hit = cache.lookup_decision(42).expect("should hit");
        assert_eq!(hit.model, "gpt-4.1-mini");
    }

    #[test]
    fn decision_cache_misses_after_ttl() {
        let cache = RoutingCache::new(std::path::PathBuf::from("/nonexistent"));
        cache.decisions.write().put(42, CachedDecision {
            model: "x".into(),
            explanation_blob: None,
            cached_at: Instant::now() - Duration::from_secs(310),
        });
        assert!(cache.lookup_decision(42).is_none());
    }

    #[test]
    fn disabled_cache_returns_no_decision_hits() {
        let cache = RoutingCache::new(std::path::PathBuf::from("/nonexistent")).with_disabled(true);
        cache.record_decision(42, CachedDecision::new("x", None));
        assert!(cache.lookup_decision(42).is_none());
    }
}
```

## Write Scope

- `crates/roko-learn/src/cascade/mod.rs`
- `crates/roko-learn/src/cascade/routing_cache.rs`
- `crates/roko-learn/Cargo.toml`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/main.rs`

## Read-Only Context

- `crates/roko-learn/src/efficiency.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `tmp/solutions/perf/implementation/07-routing-cache.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-CACHE-*, AP-ROUTE-*)

## Acceptance Criteria

- [ ] New module `crates/roko-learn/src/cascade/routing_cache.rs` exists.
- [ ] `lru` (and `parking_lot` if absent) added to `roko-learn`'s `Cargo.toml`.
- [ ] `RoutingCache::signals()` honours both 10 s TTL and `efficiency.jsonl` mtime invalidation.
- [ ] `RoutingCache::lookup_decision`/`record_decision` honour 5 min TTL with LRU cap 1024.
- [ ] `routing_cache_key` buckets quality into deciles (no float hashing).
- [ ] `Orchestrator` owns one `Arc<RoutingCache>` per workdir; replaces all `load_efficiency_signals_sync` call sites in the dispatch hot path.
- [ ] CLI `--no-routing-cache` flag exists and disables both signal and decision caches.
- [ ] Tests `signals_are_cached_within_ttl`, `signals_invalidate_on_mtime_change`, `decision_cache_hits_for_same_key`, `decision_cache_misses_after_ttl`, `disabled_cache_returns_no_decision_hits` pass.

## Verify

```bash
# Hot-path call sites of load_efficiency_signals_sync:
rg -n 'load_efficiency_signals_sync' crates/roko-cli/src/orchestrate.rs
# Expected: 0 (now goes through self.routing_cache.signals()).

# Lock-across-await audit:
rg -nU --multiline 'lock\(\).*?\.await' crates/roko-learn/src/cascade/routing_cache.rs
# Expected: empty.

# Tests:
cargo test -p roko-learn --release routing_cache
```

## Do NOT

- Do NOT cache routing decisions across `roko run` invocations
  (AP-ROUTE-1). Each CLI invocation is a fresh process; cross-process
  caching needs disk persistence that costs more than the routing
  decision itself.
- Do NOT skip the mtime check (AP-ROUTE-2). A 10 s TTL alone misses
  parallel writers.
- Do NOT hash raw `f64` quality scores (AP-CACHE-5). Use deciles.
- Do NOT cache the full `RoutingExplanation` deeply (AP-ROUTE-4). Cache
  the chosen model + a short blob; re-derive the full explanation on
  demand.
- Do NOT extend the TTLs beyond the values in this prompt. Dashboards
  in `roko serve` rely on freshness within 10 s.
- Do NOT build a write-through cache that updates on every observe call
  (AP-ROUTE-3). Mtime + TTL invalidation is the only sane design.
- Do NOT couple the cache to `LearningRuntime`. The cache is read-only
  with respect to the runtime; updating it on every learn-write would
  re-introduce the contention you are removing.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_07 done <commit-sha>
```
