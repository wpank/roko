# M098 — HDC Operations as Cell Implementations

## Objective
Wrap the four core HDC operations (Bind/XOR, Bundle/majority-vote, Permute/rotate, Similarity/Hamming) as composable pipeline-stage structs in `roko-primitives`. Each operation becomes a stage that takes `HdcVector` inputs and produces `HdcVector` (or `f32`) outputs, enabling HDC to be composed within processing pipelines rather than called ad-hoc.

**Important**: `roko-primitives` does NOT depend on `roko-core`, so these wrappers cannot implement the `Cell` trait directly. Instead, define a local `HdcOp` trait or use plain structs with a common method signature. A thin adapter in `roko-neuro` (which depends on both crates) can bridge to the Cell protocol if needed.

## Scope
- Crates: `roko-primitives`, optionally `roko-neuro`
- Files: `crates/roko-primitives/src/hdc.rs` (existing ops), new file `crates/roko-primitives/src/hdc_cells.rs`
- Phase ref: depth doc 11-memory/02-hdc-algebra-and-retrieval.md
- Depth doc: `tmp/unified-depth/11-memory/02-hdc-algebra-and-retrieval.md`

## Steps
1. Discover existing HDC operations and their signatures:
   ```bash
   grep -n 'pub fn\|pub const' crates/roko-primitives/src/hdc.rs | head -30
   grep -n 'pub struct.*Hdc\|pub struct.*Bundle\|pub struct.*Item' crates/roko-primitives/src/hdc.rs | head -10
   grep -rn 'roko-core' crates/roko-primitives/Cargo.toml
   ```

2. **Existing HdcVector methods** (in `crates/roko-primitives/src/hdc.rs`):
   ```rust
   impl HdcVector {
       pub fn bind(&self, other: &Self) -> Self;      // XOR
       pub fn xor(&self, other: &Self) -> Self;        // alias for bind
       pub fn bundle(vectors: &[&Self]) -> Self;       // majority vote
       pub fn permute(&self, n: usize) -> Self;        // cyclic rotate
       pub fn similarity(&self, other: &Self) -> f32;  // normalized Hamming
       pub fn hamming_similarity(&self, other: &Self) -> f32; // alias
   }
   // Also: BundleAccumulator, DecayingBundleAccumulator, ItemMemory
   ```

3. Create `crates/roko-primitives/src/hdc_cells.rs` with four operation wrappers:
   ```rust
   use crate::hdc::{HdcVector, BundleAccumulator};

   /// HDC Bind Cell: XOR two fingerprints to encode a relationship.
   /// Input: two HdcVectors. Output: one XOR'd HdcVector.
   pub struct HdcBindCell;

   impl HdcBindCell {
       /// Bind two vectors (XOR). Result encodes the relationship between them.
       pub fn process(&self, a: &HdcVector, b: &HdcVector) -> HdcVector {
           a.bind(b)
       }
   }

   /// HDC Bundle Cell: majority-vote aggregate N fingerprints into one.
   /// Input: slice of HdcVectors. Output: one bundled HdcVector.
   pub struct HdcBundleCell;

   impl HdcBundleCell {
       /// Bundle multiple vectors via majority vote.
       pub fn process(&self, vectors: &[&HdcVector]) -> HdcVector {
           HdcVector::bundle(vectors)
       }

       /// Incremental bundling via accumulator (for streaming).
       pub fn accumulator(&self) -> BundleAccumulator {
           BundleAccumulator::new()
       }
   }

   /// HDC Permute Cell: cyclic-rotate a fingerprint by k bit positions.
   pub struct HdcPermuteCell {
       pub rotation: usize,
   }

   impl HdcPermuteCell {
       pub fn new(rotation: usize) -> Self {
           Self { rotation }
       }

       /// Permute the vector by the configured rotation amount.
       pub fn process(&self, v: &HdcVector) -> HdcVector {
           v.permute(self.rotation)
       }
   }

   /// HDC Similarity Cell: compute normalized Hamming similarity between two vectors.
   /// Output: f32 in [0.0, 1.0] where 1.0 = identical, 0.5 = random.
   pub struct HdcSimilarityCell;

   impl HdcSimilarityCell {
       /// Compute similarity between two vectors.
       pub fn process(&self, a: &HdcVector, b: &HdcVector) -> f32 {
           a.similarity(b)
       }
   }
   ```

4. Register in `crates/roko-primitives/src/lib.rs`:
   ```rust
   pub mod hdc_cells;
   pub use hdc_cells::{HdcBindCell, HdcBundleCell, HdcPermuteCell, HdcSimilarityCell};
   ```

5. Write tests in `crates/roko-primitives/src/hdc_cells.rs`:
   - Bind Cell: XOR is self-inverse (`bind(a, bind(a, b)) == b`)
   - Bundle Cell: majority of 3 identical vectors returns the same vector
   - Permute Cell: permute(k) then permute(N-k) is identity (where N = HDC_BITS = 10240)
   - Similarity Cell: identical vectors yield 1.0, random vectors yield ~0.5

## Verification
```bash
cargo check -p roko-primitives
cargo clippy -p roko-primitives --no-deps -- -D warnings
cargo test -p roko-primitives -- hdc_cells
```

## What NOT to do
- Do NOT modify existing `HdcVector` methods -- wrap them
- Do NOT add roko-core as a dependency to roko-primitives (would create a cycle) -- use plain structs, not the Cell trait
- Do NOT implement the three-tier search pipeline -- that is M099
- Do NOT change the HDC vector size (10,240 bits = `[u64; 160]`) or encoding format
