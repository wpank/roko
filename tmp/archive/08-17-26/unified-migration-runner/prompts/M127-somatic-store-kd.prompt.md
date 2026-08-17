# M127 — Somatic Store with dual k-d tree and contrarian Functor

## Objective
Extend the somatic system with a dual-tree `SomaticStore` architecture, a `ContrarianRetrieval` functor, and consolidation logic. The existing `SomaticLandscape` in `lib.rs` already uses `kiddo::KdTree<f64, 8>` for spatial indexing; this batch adds a separate `SomaticStore` with dual-tree (live + consolidated), proper contrarian retrieval, and merge/depotentiation for dream cycles.

## Scope
- Crates: `roko-daimon`
- Files:
  - New: `crates/roko-daimon/src/somatic_store.rs`
  - `crates/roko-daimon/src/somatic_ta.rs` (reference — existing somatic technical analysis)
  - `crates/roko-daimon/src/phase2_stubs.rs` (existing `ContrarianTracker`, `ResourcePressureCompressor`)
  - `crates/roko-daimon/src/lib.rs` (module decl, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/20-somatic-landscape.md`

## Existing types reference

The somatic system already exists (`crates/roko-daimon/src/lib.rs`):
```rust
const STRATEGY_DIMENSIONS: usize = 8;
type SomaticTree = KdTree<f64, STRATEGY_DIMENSIONS>;  // kiddo::KdTree, 8 dimensions

pub struct StrategyCoordinates {
    pub complexity: f64, pub risk: f64, pub novelty: f64, pub confidence: f64,
    pub time_pressure: f64, pub scope: f64, pub reversibility: f64, pub dependency_depth: f64,
}
impl StrategyCoordinates {
    pub const fn as_array(self) -> [f64; STRATEGY_DIMENSIONS] { ... }
}

pub struct SomaticMarker {
    pub strategy_coords: StrategyCoordinates,
    pub valence: f64,           // [-1.0, 1.0]
    pub intensity: f64,         // [0.0, 1.0]
    pub episodes: Vec<ContentHash>,
    pub updated_at: DateTime<Utc>,
}

pub struct SomaticSignal {
    pub valence: f64, pub intensity: f64,
    pub neighbor_count: usize, pub contrarian_count: usize,
    pub source_episodes: Vec<ContentHash>,
}

pub struct SomaticLandscape {
    pub markers: Vec<SomaticMarker>,
    tree: SomaticTree,  // kiddo KdTree
}
// Already has: insert(), query_nearest(), summary(), etc.
```

The `ContrarianTracker` already exists in `phase2_stubs.rs` (DaimonState has `pub contrarian_tracker: ContrarianTracker`).
The `ResourcePressure` struct already exists in `phase2_stubs.rs` (note: not "ResourcePressureCompressor").

The crate depends on `kiddo = "5.3.0"`. Note: kiddo v5 does NOT have `ImmutableKdTree` — it only has `KdTree`. If you need a separate immutable tree, use a second `KdTree` that is only rebuilt during consolidation.

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-daimon/src/lib.rs | grep -i somatic | head -15
   grep -rn 'ContrarianTracker' crates/roko-daimon/src/phase2_stubs.rs | head -10
   grep -rn 'ResourcePressure\|compress' crates/roko-daimon/src/phase2_stubs.rs | head -10
   grep -rn 'kiddo' crates/roko-daimon/Cargo.toml
   ```

2. Create `crates/roko-daimon/src/somatic_store.rs` using `kiddo::KdTree` (NOT `ImmutableKdTree`):
   ```rust
   use kiddo::{KdTree, SquaredEuclidean};
   use super::{STRATEGY_DIMENSIONS, SomaticMarker, SomaticSignal, StrategyCoordinates};
   use roko_core::ContentHash;
   use chrono::{DateTime, Utc};

   /// Dual-tree somatic index: consolidated (read-heavy) + live (write-heavy).
   pub struct SomaticIndex {
       consolidated: KdTree<f64, STRATEGY_DIMENSIONS>,  // rebuilt during consolidation
       live: KdTree<f64, STRATEGY_DIMENSIONS>,           // accumulates new markers
       markers: Vec<SomaticMarker>,
   }
   impl SomaticIndex {
       pub fn query_nearest(&self, coords: &[f64; STRATEGY_DIMENSIONS], k: usize) -> Vec<(f64, &SomaticMarker)> { ... }
       pub fn insert(&mut self, marker: SomaticMarker) { ... }
       pub fn rebuild_consolidated(&mut self) { /* rebuild consolidated from all markers, clear live */ }
   }
   ```

3. Add `SomaticStore` wrapping the index:
   ```rust
   pub struct SomaticStore {
       index: SomaticIndex,
   }
   impl SomaticStore {
       pub fn record_outcome(&mut self, coords: [f64; STRATEGY_DIMENSIONS], valence: f64, intensity: f64, episode: ContentHash) { ... }
       pub fn query(&self, coords: &[f64; STRATEGY_DIMENSIONS], k: usize) -> Vec<(f64, SomaticMarker)> { ... }
       pub fn count(&self) -> usize { self.index.markers.len() }
   }
   ```

4. Implement `ContrarianRetrieval`:
   ```rust
   pub struct ContrarianRetrieval {
       pub contrarian_fraction: f64,  // default 0.15 (matches CONTRARIAN_FRACTION constant)
   }
   impl ContrarianRetrieval {
       pub fn query_with_contrarian(&self, store: &SomaticStore, coords: &[f64; STRATEGY_DIMENSIONS], k: usize) -> SomaticSignal {
           // Query k nearest, separate by valence sign
           // Blend: (1 - fraction) * congruent_avg + fraction * contrarian_avg
       }
   }
   ```

5. Implement resource pressure coordinate compression:
   ```rust
   pub fn compress_coords(raw: &[f64; STRATEGY_DIMENSIONS], vitality: f64) -> [f64; STRATEGY_DIMENSIONS] {
       let pressure_scalar = vitality.sqrt();
       let midpoint = 0.5;
       raw.map(|c| midpoint + pressure_scalar * (c - midpoint))
   }
   ```

6. Implement consolidation logic:
   ```rust
   pub fn merge_nearby(markers: &mut Vec<SomaticMarker>, distance_sq_threshold: f64) { ... }
   pub fn depotentiate(markers: &mut [SomaticMarker], now: DateTime<Utc>, age_hours_threshold: f64) { ... }
   ```

7. Add module to `lib.rs`:
   ```rust
   pub mod somatic_store;
   pub use somatic_store::{SomaticStore, SomaticIndex, ContrarianRetrieval, compress_coords};
   ```

8. Add tests:
   - Dual-tree query returns results from both consolidated and live trees
   - Contrarian blending produces opposite-valence component
   - Coordinate compression moves values toward midpoint
   - `merge_nearby` merges markers within distance threshold
   - `depotentiate` reduces intensity of old extreme markers

## Verification
```bash
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- somatic_store
cargo test -p roko-daimon -- contrarian
cargo test -p roko-daimon -- consolidation
cargo test -p roko-daimon -- compress
```

## What NOT to do
- Do NOT replace the existing `SomaticLandscape` — add `SomaticStore` as a parallel implementation
- Do NOT use `ImmutableKdTree` — kiddo v5.3 does not expose it; use two `KdTree` instances
- Do NOT add `Store` trait impl — just internal data structures
- Do NOT wire into dream cycle — that is a separate integration
- Do NOT add Bus publication — just data structures and algorithms
