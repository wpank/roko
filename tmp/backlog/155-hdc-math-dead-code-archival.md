# 155 — HDC Advanced Math Dead Code Archival

**Priority**: P3 — dead code removal; no runtime impact
**Size**: XS (under 2 hours)
**Crates**: `crates/roko-primitives/src/`
**Depends on**: None
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` S5.3a, `tmp/mori-old/11-CYBERNETIC-FEATURES-AUDIT.md`

---

## Background

The `roko-primitives` crate contains hyperdimensional computing (HDC) vector operations used
for episode fingerprinting, knowledge similarity, and cascade routing. The core HDC types
(`HdcVector`, binding/bundling operations, cosine similarity) are actively used throughout the
codebase, as are `InferenceTier`, `TierRouter`, `PadVector`, and the `Codebook` subsystem.

However, the crate also contains four advanced mathematical modules that were implemented
speculatively (task codes TA-06, TA-09, TA-10, TA-13, TA-14) and have zero callers outside
their own files. They contribute 3,055 lines of code (roughly half of the entire crate) and
add compile time and code surface area without providing any runtime value.

The four modules are:

1. **Topological Data Analysis (`tda.rs`, 575 lines)**: Takens delay embedding,
   Vietoris-Rips persistence diagrams (H0/H1) via Union-Find, persistence landscapes,
   bottleneck distance. Intended for analysing the topological structure of HDC vector spaces.

2. **Tropical Algebra (`tropical.rs`, 698 lines)**: Max-plus semiring scalar
   (`TropicalF64`), tropical polynomials (max over affine functions), tropical matrices,
   tropical attention (`max_j(q . k_j + v_j)`), and adversarial distance computation.
   Intended for piecewise-linear decision geometry.

3. **Cellular Sheaf Laplacian (`sheaf.rs`, 785 lines)**: Cellular sheaf on a graph with
   per-vertex stalks, per-edge restriction maps (identity, projection, arbitrary linear),
   coboundary operator, sheaf Laplacian (`L_F = delta^T delta`), inconsistency scoring,
   most-inconsistent-vertex identification, and eigenvalue computation via power iteration.
   Intended for oracle consistency checking.

4. **Riemannian Manifold Geometry (`manifold.rs`, 828 lines)**: 4D metric tensors,
   Gauss-Jordan matrix inversion, Christoffel symbols via finite differences, RK4 geodesic
   solver, approximate geodesic distance, Ricci scalar curvature, and Frechet mean. Intended
   for execution cost landscape modelling.

A fifth module, **Robust Statistics (`robust_stats.rs`, 169 lines)**, is also unused
(trimmed mean, MAD, Hodges-Lehmann estimator). It should be included in this archival.

These modules should be gated behind a disabled-by-default feature flag rather than deleted,
preserving them for potential future research use.

## Current State

All five modules are unconditionally compiled and publicly exported from
`crates/roko-primitives/src/lib.rs`:

```rust
pub mod manifold;
pub mod robust_stats;
pub mod sheaf;
pub mod tda;
pub mod tropical;
```

None of these modules appear in `pub use` re-exports in `lib.rs` (only `hdc`, `codebook`,
`pad`, and `tier` have re-exports).

A workspace-wide caller audit confirms **zero production or cross-crate callers**:

| Module | Lines | Public types/fns | Callers outside own file | Test-only |
|---|---|---|---|---|
| `tda.rs` | 575 | `PersistencePoint`, `PersistenceDiagram`, `takens_embedding`, `vietoris_rips`, `persistence_landscape`, `landscape_distance` | 0 | 14 tests (self) |
| `tropical.rs` | 698 | `TropicalF64`, `TropicalTerm`, `TropicalPolynomial`, `TropicalMatrix`, `tropical_attention`, `tropical_attention_batch`, `adversarial_distance` | 0 | 21 tests (self) |
| `sheaf.rs` | 785 | `NodeId`, `EdgeId`, `RestrictionMap`, `CellularSheaf` | 0 | 14 tests (self) |
| `manifold.rs` | 828 | `MetricTensor`, `GeodesicPoint`, `christoffel`, `geodesic_rk4`, `approx_geodesic_distance`, `ricci_scalar`, `frechet_mean`, matrix utils | 0 | 16 tests (self) |
| `robust_stats.rs` | 169 | `trimmed_mean`, `mad`, `hodges_lehmann` | 0 | 8 tests (self) |
| **Total** | **3,055** | | **0** | **73 tests** |

The crate's `Cargo.toml` already has a `[features]` section with one dormant feature (`rkyv`),
so adding a new feature flag is consistent with existing patterns.

## Implementation Plan

1. **Audit callers (done above)**: Confirm that `tda`, `tropical`, `sheaf`, `manifold`, and
   `robust_stats` have zero production callers. Verified: no crate in the workspace imports
   any type or function from these five modules.

2. **Add feature flag to `Cargo.toml`**: In `crates/roko-primitives/Cargo.toml`, add:
   ```toml
   [features]
   default = []
   rkyv = ["dep:rkyv"]
   hdc-advanced-math = []   # TDA, tropical algebra, sheaf Laplacian, manifold geometry, robust stats
   ```

3. **Gate modules in `lib.rs`**: Replace the unconditional `pub mod` declarations with:
   ```rust
   #[cfg(feature = "hdc-advanced-math")]
   pub mod manifold;
   #[cfg(feature = "hdc-advanced-math")]
   pub mod robust_stats;
   #[cfg(feature = "hdc-advanced-math")]
   pub mod sheaf;
   #[cfg(feature = "hdc-advanced-math")]
   pub mod tda;
   #[cfg(feature = "hdc-advanced-math")]
   pub mod tropical;
   ```

4. **Verify clean build**: Run `cargo build --workspace` and `cargo test --workspace`
   without the feature enabled. All five modules and their 73 tests should be excluded.

5. **Verify gated build**: Run
   `cargo test -p roko-primitives --features hdc-advanced-math` to confirm the modules
   and their tests still compile and pass when the feature is enabled.

6. **Remove `#![allow(...)]` lint overrides if possible**: The large `#![allow(...)]` block
   at the top of `lib.rs` exists partly to accommodate these math-heavy modules. After
   gating them, check whether any of the suppressed lints (e.g., `clippy::suboptimal_flops`,
   `clippy::needless_range_loop`, `clippy::cast_precision_loss`) can be narrowed to only
   apply under the feature flag. If the remaining non-gated code still triggers them, leave
   the allows in place.

## Acceptance Criteria

1. The five advanced math modules (`tda`, `tropical`, `sheaf`, `manifold`, `robust_stats`)
   are behind a disabled-by-default `hdc-advanced-math` feature flag.
2. `cargo build --workspace` succeeds without the feature.
3. `cargo test --workspace` succeeds without the feature.
4. `cargo clippy --workspace --no-deps -- -D warnings` succeeds without the feature.
5. `cargo test -p roko-primitives --features hdc-advanced-math` passes all 73 tests in
   the gated modules.
6. No production code paths are broken.

## Verification Checklist

- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test -p roko-primitives --features hdc-advanced-math` passes (73 tests)
- [ ] `grep -rn 'use roko_primitives::\(tda\|tropical\|sheaf\|manifold\|robust_stats\)' crates/ --include='*.rs' | grep -v target/` returns zero hits (confirming no external callers)

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-primitives/Cargo.toml` | Add `hdc-advanced-math = []` feature |
| `crates/roko-primitives/src/lib.rs` | Gate `tda`, `tropical`, `sheaf`, `manifold`, `robust_stats` behind `#[cfg(feature = "hdc-advanced-math")]` |
