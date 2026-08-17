# M091 — Merkle-CRDT Sync

## Objective
Implement Merkle-CRDT synchronization between brain states. A Merkle tree over brain state (knowledge + learning) enables efficient diff computation. CRDT operations (GCounter for citation counts, LWW-Register for calibration state, Add-only set for Signals) enable deterministic conflict-free merging. Two instances with divergent learning can compute a Merkle diff, exchange missing entries, and merge via CRDT rules to converge to identical state.

## Scope
- Crates: `roko-neuro`
- Files: `crates/roko-neuro/src/brain/merkle.rs` (new), `crates/roko-neuro/src/brain/crdt.rs` (new), `crates/roko-neuro/src/brain/sync.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.6
- Spec ref: `tmp/unified/20-DEPLOYMENT.md` SS4.3

## Steps
1. Implement the Merkle tree in `crates/roko-neuro/src/brain/merkle.rs`:
   ```rust
   pub struct MerkleTree {
       root: MerkleNode,
       leaf_count: usize,
   }

   pub struct MerkleNode {
       hash: [u8; 32],
       left: Option<Box<MerkleNode>>,
       right: Option<Box<MerkleNode>>,
   }

   impl MerkleTree {
       pub fn from_entries(entries: &[([u8; 32], Vec<u8>)]) -> Self;
       pub fn root_hash(&self) -> [u8; 32];
       pub fn diff(&self, other: &MerkleTree) -> Vec<[u8; 32]>;  // hashes present in self but not other
       pub fn proof(&self, entry_hash: &[u8; 32]) -> Option<MerkleProof>;
   }
   ```

2. Implement CRDTs in `crates/roko-neuro/src/brain/crdt.rs`:
   ```rust
   /// GCounter: grow-only counter (for citation counts, retrieval counts)
   pub struct GCounter {
       counts: HashMap<String, u64>,  // node_id -> count
   }

   impl GCounter {
       pub fn increment(&mut self, node_id: &str);
       pub fn value(&self) -> u64;
       pub fn merge(&mut self, other: &GCounter);
   }

   /// LWW-Register: last-writer-wins register (for calibration state)
   pub struct LwwRegister<T> {
       value: T,
       timestamp: DateTime<Utc>,
       node_id: String,
   }

   impl<T: Clone> LwwRegister<T> {
       pub fn set(&mut self, value: T, timestamp: DateTime<Utc>, node_id: &str);
       pub fn merge(&mut self, other: &LwwRegister<T>);
   }

   /// Add-only set (for Signals -- never delete, only add)
   pub struct AddOnlySet<T: Hash + Eq> {
       elements: HashSet<T>,
   }

   impl<T: Hash + Eq + Clone> AddOnlySet<T> {
       pub fn add(&mut self, element: T);
       pub fn merge(&mut self, other: &AddOnlySet<T>);
   }
   ```

3. Implement sync protocol in `crates/roko-neuro/src/brain/sync.rs`:
   ```rust
   pub struct BrainSync {
       local_tree: MerkleTree,
       local_state: BrainState,
   }

   impl BrainSync {
       /// Compute what the remote is missing.
       pub fn compute_diff(&self, remote_root: [u8; 32]) -> SyncDiff;
       /// Apply received entries from remote.
       pub fn apply_remote(&mut self, entries: Vec<SyncEntry>) -> Result<SyncSummary>;
       /// Full sync protocol: exchange roots -> diff -> exchange entries -> merge.
       pub async fn sync_with(&mut self, peer: &mut impl SyncPeer) -> Result<SyncSummary>;
   }
   ```

4. Write tests:
   - Two instances diverge for 100 operations, sync, converge to identical state
   - GCounter merge produces correct sum
   - LWW-Register merge keeps the latest value
   - Merkle diff correctly identifies missing entries

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- brain::merkle
cargo test -p roko-neuro -- brain::crdt
cargo test -p roko-neuro -- brain::sync
```

## What NOT to do
- Do NOT implement a custom hash function -- use SHA-256
- Do NOT add network transport here -- sync operates on abstract SyncPeer trait
- Do NOT implement tombstones -- knowledge uses add-only semantics (demurrage handles decay)
- Do NOT add vector clocks -- LWW with wall-clock timestamps is sufficient for this use case
