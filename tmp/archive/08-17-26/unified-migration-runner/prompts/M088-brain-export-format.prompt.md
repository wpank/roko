# M088 — Brain Export Format

## Objective
Define the brain export format: manifest + knowledge Signals (filtered by tier/date) + learning state (calibration, route posteriors, section effects) + episodes (optional). Target size: 100KB-1MB. Format: CBOR with Merkle tree over entries. The brain export is a portable snapshot of an Agent's accumulated knowledge and learning, suitable for transfer between workspaces.

## Scope
- Crates: `roko-neuro`
- Files: `crates/roko-neuro/src/brain/format.rs` (new), `crates/roko-neuro/src/brain/mod.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.6
- Spec ref: `tmp/unified/20-DEPLOYMENT.md` SS4 (Brain Export)

## Steps
1. Check for existing brain or export code:
   ```bash
   grep -rn 'brain\|Brain\|export\|Export\|backup' crates/roko-neuro/src/ --include='*.rs' | head -15
   ```

2. Add CBOR dependency to roko-neuro/Cargo.toml:
   ```toml
   ciborium = "0.2"
   ```

3. Define the brain export format in `crates/roko-neuro/src/brain/format.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BrainExport {
       pub manifest: BrainManifest,
       pub signals: Vec<ExportedSignal>,
       pub learning_state: LearningState,
       pub episodes: Option<Vec<ExportedEpisode>>,
       pub merkle_root: [u8; 32],
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BrainManifest {
       pub version: String,
       pub agent_id: String,
       pub workspace: String,
       pub created_at: DateTime<Utc>,
       pub signal_count: usize,
       pub min_tier: String,
       pub date_range: (DateTime<Utc>, DateTime<Utc>),
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ExportedSignal {
       pub id: String,
       pub kind: String,
       pub tier: String,
       pub balance: f64,
       pub content_hash: [u8; 32],
       pub hdc_fingerprint: Vec<f32>,
       pub metadata: Value,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct LearningState {
       pub calibration: Value,
       pub route_posteriors: Value,
       pub section_effects: Value,
       pub gate_thresholds: Value,
   }
   ```

4. Implement Merkle tree over entries:
   ```rust
   fn compute_merkle_root(signals: &[ExportedSignal]) -> [u8; 32] {
       // SHA-256 Merkle tree: leaf = hash(signal), internal = hash(left || right)
   }
   ```

5. Implement serialization/deserialization:
   ```rust
   impl BrainExport {
       pub fn to_cbor(&self) -> Result<Vec<u8>>;
       pub fn from_cbor(data: &[u8]) -> Result<Self>;
       pub fn verify_merkle(&self) -> bool;
   }
   ```

6. Write tests:
   - Export produces valid CBOR under 1MB for 10K Signals
   - Merkle root verifies correctly
   - Round-trip: serialize -> deserialize -> compare
   - Manifest fields are populated correctly

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- brain::format
```

## What NOT to do
- Do NOT implement export filtering -- that is M089
- Do NOT implement import -- that is M090
- Do NOT use JSON for the export format -- CBOR is more compact
- Do NOT include raw Signal content in exports -- only metadata, fingerprints, and hashes
