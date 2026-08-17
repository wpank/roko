# M102 — Federation Spaces and Confidence Functor

## Objective
Implement knowledge federation boundaries using Space-scoped Store partitions and a confidence decay functor that attenuates knowledge confidence as it crosses domain boundaries. This enables safe cross-agent knowledge transfer where remote knowledge enters at reduced confidence and must earn its way up through local verification.

## Scope
- Crates: `roko-neuro`, `roko-core`
- Files: `crates/roko-neuro/src/knowledge_store.rs`, new file `crates/roko-neuro/src/federation.rs`
- Phase ref: depth doc 11-memory/05-cross-domain-transfer.md
- Depth doc: `tmp/unified-depth/11-memory/05-cross-domain-transfer.md`

## Steps
1. Discover existing federation/sync code and KnowledgeStore interface:
   ```bash
   grep -rn 'federation\|sync\|mesh\|transfer\|remote' crates/roko-neuro/src/ --include='*.rs' | head -15
   grep -rn 'Space\|space\|domain\|partition' crates/roko-neuro/src/ --include='*.rs' | head -15
   grep -n 'pub fn ingest' crates/roko-neuro/src/knowledge_store.rs | head -5
   grep -n 'pub tier:' crates/roko-neuro/src/lib.rs | head -5
   grep -n 'pub tags:' crates/roko-neuro/src/lib.rs | head -5
   ```
   **Note**: `KnowledgeStore::ingest` takes `&self` (uses file-level locking), not `&mut self`.

2. Create `crates/roko-neuro/src/federation.rs`:
   ```rust
   /// A federation space defines a trust boundary for knowledge transfer.
   /// Knowledge crossing a space boundary has its confidence attenuated.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FederationSpace {
       /// Unique identifier for this space (e.g., agent ID, workspace ID)
       pub space_id: String,
       /// Human-readable name
       pub name: String,
       /// Trust level for knowledge from this space (0.0 = untrusted, 1.0 = fully trusted)
       pub trust_level: f64,
       /// Tags that define the domain scope of this space
       pub domain_tags: Vec<String>,
   }

   /// Confidence functor that attenuates knowledge as it crosses boundaries.
   #[derive(Debug, Clone)]
   pub struct ConfidenceFunctor {
       /// Base decay factor per boundary crossing (default: 0.5)
       pub boundary_decay: f64,
       /// Minimum confidence after transfer (floor)
       pub min_confidence: f64,
       /// Whether to factor in HDC similarity to local knowledge
       pub use_resonance_boost: bool,
   }
   ```

3. Implement the confidence functor:
   ```rust
   impl ConfidenceFunctor {
       /// Apply confidence decay when transferring knowledge across a boundary.
       pub fn transfer(
           &self,
           entry: &KnowledgeEntry,
           source_space: &FederationSpace,
           target_space: &FederationSpace,
           local_store: &KnowledgeStore,
       ) -> KnowledgeEntry {
           let mut transferred = entry.clone();
           // Base decay
           transferred.confidence *= self.boundary_decay * source_space.trust_level;
           // Resonance boost: if similar knowledge exists locally, boost confidence
           if self.use_resonance_boost {
               let resonance = self.compute_resonance(&transferred, local_store);
               transferred.confidence *= 1.0 + resonance;
           }
           // Floor
           transferred.confidence = transferred.confidence.max(self.min_confidence);
           // Tag with source provenance
           transferred.tags.push(format!("federation:{}", source_space.space_id));
           // Reset tier to Transient (must earn its way up locally)
           transferred.tier = KnowledgeTier::Transient;
           transferred
       }

       fn compute_resonance(&self, entry: &KnowledgeEntry, store: &KnowledgeStore) -> f64 { ... }
   }
   ```

4. Implement an ingestion function that applies the functor:
   ```rust
   /// Ingest knowledge from a remote federation space.
   /// Note: KnowledgeStore::ingest takes &self (file-level locking), not &mut self.
   pub fn ingest_federated(
       entry: &KnowledgeEntry,
       source: &FederationSpace,
       local: &FederationSpace,
       functor: &ConfidenceFunctor,
       store: &KnowledgeStore,
   ) -> Result<IngestResult> {
       let transferred = functor.transfer(entry, source, local, store);
       store.ingest(vec![transferred])?;
       Ok(IngestResult { ... })
   }
   ```

5. Register in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod federation;
   ```

6. Write tests:
   - Transferred knowledge has reduced confidence
   - Trust level 1.0 still applies boundary_decay
   - Knowledge with local resonance gets boosted
   - Transferred entries always start at Transient tier
   - Provenance tag is added

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- federation
```

## What NOT to do
- Do NOT implement network transport for federation -- this is the data model only
- Do NOT modify the brain export/import format (M088-M090) -- federation uses its own ingestion path
- Do NOT implement multi-hop federation chains -- single boundary crossing only
- Do NOT add external dependencies for distributed consensus
