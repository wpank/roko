# M108 — Staging Store Partition and SHY Renormalization

## Objective
Implement the staging buffer as a Store partition rather than a separate struct, and add Synaptic Homeostasis (SHY) renormalization that periodically scales down all knowledge confidence values to prevent runaway confidence inflation. The staging partition uses metadata tags to separate staged knowledge from the main store, sharing the same persistence and query infrastructure.

## Scope
- Crates: `roko-dreams`, `roko-neuro`
- Files: `crates/roko-dreams/src/staging.rs` (existing), `crates/roko-neuro/src/knowledge_store.rs`
- Phase ref: depth doc 11-memory/09-consolidation-and-staging.md
- Depth doc: `tmp/unified-depth/11-memory/09-consolidation-and-staging.md`

## Steps
1. Discover existing staging buffer API and knowledge store interface:
   ```bash
   grep -n 'pub fn\|pub struct\|pub enum' crates/roko-dreams/src/staging.rs | head -20
   wc -l crates/roko-dreams/src/staging.rs
   grep -n 'pub fn ingest\|pub fn query\|pub fn read_all' crates/roko-neuro/src/knowledge_store.rs | head -10
   ```
   **Existing StagingBuffer API** (in `crates/roko-dreams/src/staging.rs`):
   - `StagingBuffer::new()`, `load_or_new(path)`, `save(path)`
   - `add_candidate(entry, episode_id)`, `advance_stage(idx)`, `promote_validated(...)`
   - `gc()`, `gc_at(now)`, `remove_promoted()`
   - `ConfidenceStage` enum: `Raw -> Replayed -> Validated -> Promoted`

   **Note**: `KnowledgeStore::ingest` takes `&self` (file-level write lock), not `&mut self`.

2. Audit the knowledge store for existing partition support:
   ```bash
   grep -rn 'partition\|staging\|tag\|metadata' crates/roko-neuro/src/knowledge_store.rs | head -15
   ```

3. Add partition support to `KnowledgeStore` in `crates/roko-neuro/src/knowledge_store.rs`:
   ```rust
   /// Partition tag for staging buffer entries.
   pub const STAGING_PARTITION: &str = "partition:staging";
   /// Partition tag for cold/archived entries.
   pub const COLD_PARTITION: &str = "partition:cold";

   impl KnowledgeStore {
       /// Query entries from a specific partition.
       pub fn query_partition(
           &self,
           partition: &str,
           query: &str,
           limit: usize,
       ) -> Result<Vec<KnowledgeEntry>> { ... }

       /// Write an entry to a specific partition (adds partition tag).
       pub fn write_to_partition(
           &self,
           entry: &mut KnowledgeEntry,
           partition: &str,
       ) -> Result<()> { ... }

       /// Move an entry between partitions (e.g., staging -> main).
       pub fn promote_from_partition(
           &self,
           entry_id: &str,
           from_partition: &str,
       ) -> Result<()> { ... }
   }
   ```

4. Implement SHY renormalization:
   ```rust
   /// Synaptic Homeostasis (SHY) renormalization.
   /// Periodically scales down all confidence values to prevent inflation.
   pub struct ShyRenormalizer {
       /// Scale factor per renormalization pass (e.g., 0.95 = 5% reduction)
       pub scale_factor: f64,
       /// Minimum confidence floor (entries below this are candidates for GC)
       pub confidence_floor: f64,
       /// How often to renormalize (number of cycles between passes)
       pub period_cycles: usize,
   }

   impl ShyRenormalizer {
       /// Apply SHY renormalization to all entries in the store.
       /// Returns the number of entries affected.
       /// Note: KnowledgeStore uses `&self` with internal file locking.
       pub fn renormalize(&self, store: &KnowledgeStore) -> Result<usize> {
           // Scale all confidence values by scale_factor
           // Entries below confidence_floor are flagged for GC
           // AntiKnowledge entries are scaled differently (slower decay)
       }
   }
   ```

5. Create a `StagingPartitionAdapter` in `crates/roko-dreams/src/staging.rs` that wraps the partition methods:
   ```rust
   /// Adapter for the staging partition that wraps KnowledgeStore partition methods.
   /// Note: KnowledgeStore uses `&self` with internal file-level locking for write ops.
   pub struct StagingPartitionAdapter<'a> {
       store: &'a KnowledgeStore,
   }

   impl<'a> StagingPartitionAdapter<'a> {
       pub fn new(store: &'a KnowledgeStore) -> Self {
           Self { store }
       }

       pub fn stage(&self, entry: &mut KnowledgeEntry) -> Result<()> {
           self.store.write_to_partition(entry, STAGING_PARTITION)
       }

       pub fn promote(&self, entry_id: &str) -> Result<()> {
           self.store.promote_from_partition(entry_id, STAGING_PARTITION)
       }

       pub fn query_staged(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
           self.store.query_partition(STAGING_PARTITION, query, limit)
       }
   }
   ```

6. Write tests:
   - Entry written to staging partition has correct tag
   - Default query excludes staging partition entries
   - Partition query only returns entries from that partition
   - Promote removes partition tag from entry
   - SHY renormalization scales confidence correctly
   - SHY respects confidence floor
   - AntiKnowledge entries decay more slowly under SHY

## Verification
```bash
cargo check -p roko-neuro -p roko-dreams
cargo clippy -p roko-neuro -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-neuro -- knowledge_store
cargo test -p roko-dreams -- staging
```

## What NOT to do
- Do NOT delete the existing `StagingBuffer` struct -- the adapter wraps the new partition methods
- Do NOT change the JSONL persistence format -- partitions are tag-based metadata
- Do NOT implement dream evolution (iterative staging) -- keep it to basic stage/promote/query
- Do NOT add a separate file for staged entries -- they live in the same JSONL
