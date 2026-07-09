# M123 — PAD as Signal metadata and PadContext type

## Objective
Define `PadContext` as a first-class metadata type that Signals can carry. Add `stamp_affect()` and `read_affect()` helpers. Extend the existing `PadVector` (defined in `roko-primitives`, re-exported by `roko-core`) with a `confidence` field and `octant()` classifier. Align the existing `AffectOctant` in `roko-daimon/src/phase2_stubs.rs` with the Mehrabian naming convention. This is the foundation for all affect-as-functor work.

## Scope
- Crates: `roko-primitives`, `roko-core`, `roko-daimon`
- Files:
  - `crates/roko-primitives/src/pad.rs` (canonical `PadVector` struct — **this is where PadVector is defined**)
  - `crates/roko-primitives/src/lib.rs` (re-exports `pub use pad::PadVector`)
  - `crates/roko-core/src/affect.rs` (re-exports PadVector via `pub use roko_primitives::PadVector`, defines `BehavioralState`, `EmotionalTag`)
  - `crates/roko-daimon/src/phase2_stubs.rs` (existing `AffectOctant` enum with 8 variants)
  - `crates/roko-daimon/src/lib.rs` (DaimonState, SomaticMarker, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/18-affect-as-functor.md`

## Steps
1. Discover the actual codebase layout before making changes:
   ```bash
   # PadVector is defined in roko-primitives, NOT roko-core
   grep -rn 'pub struct PadVector' crates/ --include='*.rs' | grep -v target/
   # Re-export chain: primitives -> core -> other crates
   grep -rn 'pub use.*PadVector' crates/ --include='*.rs' | grep -v target/
   # Existing AffectOctant (already has 8 variants: Excited, Surprised, Confident, Relaxed, Angry, Anxious, Bored, Depressed)
   grep -rn 'AffectOctant' crates/roko-daimon/src/phase2_stubs.rs | head -20
   # Existing PadVector methods
   grep -rn 'pub fn\|pub const fn' crates/roko-primitives/src/pad.rs | head -20
   ```

2. In `crates/roko-primitives/src/pad.rs`, add a `confidence: f64` field to `PadVector`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
   pub struct PadVector {
       pub pleasure: f64,
       pub arousal: f64,
       pub dominance: f64,
       /// Motivational confidence in [0.0, 1.0], orthogonal to PAD dimensions.
       #[serde(default = "default_confidence")]
       pub confidence: f64,
   }

   fn default_confidence() -> f64 { 0.5 }
   ```
   Update `PadVector::new()` to accept a 4th parameter, add `PadVector::new3(p, a, d)` for backward compat that defaults confidence to 0.5. Update `neutral()`, `clamped()`, `apply_delta()`, `decay_by_factor()`, `magnitude()`, `cosine_similarity()` accordingly. The Default derive will need a manual impl that sets confidence to 0.5.

   **IMPORTANT**: Changing PadVector fields will break call sites across the workspace. Add `new3()` first, then grep for all `PadVector::new(` calls and update them:
   ```bash
   grep -rn 'PadVector::new(' crates/ --include='*.rs' | grep -v target/ | wc -l
   ```

3. The `AffectOctant` enum already exists in `crates/roko-daimon/src/phase2_stubs.rs` with these variants: `Excited, Surprised, Confident, Relaxed, Angry, Anxious, Bored, Depressed`. It already has `from_pad()` and `behavior_modulation()` methods. Do NOT replace it with Mehrabian names — use the existing names. If you need Mehrabian aliases, add a doc comment mapping:
   - Exuberant = Excited, Dependent = Surprised, Relaxed = Relaxed, Docile = Confident
   - Hostile = Angry, Anxious = Anxious, Disdainful = Bored, Depressed = Depressed

4. Add `octant()` method to `PadVector` in `crates/roko-primitives/src/pad.rs`. Since `AffectOctant` lives in `roko-daimon`, add the octant method to PadVector as a tuple return instead:
   ```rust
   /// Returns the sign-triple (pleasure >= 0, arousal >= 0, dominance >= 0)
   /// for octant classification by downstream modules.
   pub fn octant_signs(&self) -> (bool, bool, bool) {
       (self.pleasure >= 0.0, self.arousal >= 0.0, self.dominance >= 0.0)
   }
   ```
   Then update `AffectOctant::from_pad` in phase2_stubs.rs to use `pad.octant_signs()`.

5. Add `distance()` method to `PadVector` in `crates/roko-primitives/src/pad.rs`:
   ```rust
   /// Euclidean distance between two PAD vectors (3D, ignores confidence).
   pub fn distance(&self, other: &PadVector) -> f64 {
       let dp = self.pleasure - other.pleasure;
       let da = self.arousal - other.arousal;
       let dd = self.dominance - other.dominance;
       (dp * dp + da * da + dd * dd).sqrt()
   }
   ```

6. Add `PadContext` as a newtype in `crates/roko-core/src/affect.rs`:
   ```rust
   /// PAD context for Signal metadata enrichment.
   /// Wraps PadVector with the confidence dimension for functor use.
   #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
   pub struct PadContext {
       pub pad: PadVector,
   }
   ```

7. Add Signal metadata helpers in `crates/roko-daimon/src/lib.rs` (or a new `affect_metadata.rs` submodule):
   ```rust
   use std::collections::HashMap;
   use roko_primitives::PadVector;

   pub fn stamp_affect(metadata: &mut HashMap<String, serde_json::Value>, pad: &PadVector) {
       metadata.insert("affect_pleasure".into(), serde_json::json!(pad.pleasure));
       metadata.insert("affect_arousal".into(), serde_json::json!(pad.arousal));
       metadata.insert("affect_dominance".into(), serde_json::json!(pad.dominance));
       metadata.insert("affect_confidence".into(), serde_json::json!(pad.confidence));
   }

   pub fn read_affect(metadata: &HashMap<String, serde_json::Value>) -> Option<PadVector> {
       let p = metadata.get("affect_pleasure")?.as_f64()?;
       let a = metadata.get("affect_arousal")?.as_f64()?;
       let d = metadata.get("affect_dominance")?.as_f64()?;
       let c = metadata.get("affect_confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
       Some(PadVector { pleasure: p, arousal: a, dominance: d, confidence: c })
   }
   ```

8. Add a lightweight `PulseAffect` struct for Pulse annotations in `crates/roko-daimon/src/phase2_stubs.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
   pub struct PulseAffect {
       pub octant: AffectOctant,
       pub confidence: f64,
   }
   ```

9. Update existing DaimonState methods that construct PadVector to include the confidence field. Grep for all sites:
   ```bash
   grep -rn 'PadVector::new\|PadVector {' crates/roko-daimon/src/ --include='*.rs' | grep -v target/ | head -30
   ```

10. Add tests:
    - `octant_signs()` classification for all 8 combinations
    - `distance()` is symmetric and zero for identical PAD
    - `stamp_affect`/`read_affect` round-trips correctly
    - PadVector serialization includes confidence (deserialization defaults to 0.5 for old data)
    - Existing tests still pass with new confidence field (serde default)

## Verification
```bash
cargo check -p roko-primitives -p roko-core -p roko-daimon
cargo clippy -p roko-primitives -p roko-core -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-primitives -- pad
cargo test -p roko-core -- pad
cargo test -p roko-daimon -- pad
# Verify no breakage across workspace from PadVector field addition
cargo check --workspace
```

## What NOT to do
- Do NOT modify PadVector in roko-core — it is only a re-export. The canonical definition is `crates/roko-primitives/src/pad.rs`
- Do NOT change the existing PadVector field names (pleasure, arousal, dominance) — only add confidence
- Do NOT replace the existing AffectOctant variant names (Excited, Surprised, etc.) — they are already used across the codebase
- Do NOT remove any existing methods on PadVector — extend only
- Do NOT wire into orchestrate.rs yet — that is M126
- Do NOT add ALMA temporal model here — that is M124
