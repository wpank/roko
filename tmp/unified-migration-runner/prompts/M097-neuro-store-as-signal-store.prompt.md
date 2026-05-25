# M097 — Neuro Store as Engram Store Adapter

## Objective
Refactor `KnowledgeStore` in `roko-neuro` to implement the `Store` protocol from `roko-core`, making knowledge persistence use the same `put`/`get`/`query` interface as every other Store in the system. Knowledge entries are written as Engrams with knowledge-specific Kind variants. The existing JSONL persistence format is preserved but accessed through the Store trait.

## Scope
- Crates: `roko-neuro`, `roko-core`, `roko-fs`
- Files: `crates/roko-neuro/src/knowledge_store.rs`, `crates/roko-core/src/traits.rs` (Store trait), `crates/roko-fs/src/file_substrate.rs`
- Phase ref: depth doc 11-memory/01-knowledge-as-signal.md
- Depth doc: `tmp/unified-depth/11-memory/01-knowledge-as-signal.md`

## Steps
1. Discover actual types and interfaces:
   ```bash
   grep -A 15 'pub trait Store' crates/roko-core/src/traits.rs
   grep -rn 'impl.*KnowledgeStore' crates/roko-neuro/src/ --include='*.rs' | head -10
   grep -n 'pub fn\|pub async fn' crates/roko-neuro/src/knowledge_store.rs | head -20
   grep -rn 'struct Engram' crates/roko-core/src/engram.rs | head -5
   grep -rn 'struct ContentHash' crates/roko-core/src/ --include='*.rs' | head -5
   ```

2. **Current Store trait** (in `crates/roko-core/src/traits.rs`):
   ```rust
   pub trait Store: Send + Sync {
       async fn put(&self, engram: Engram) -> Result<ContentHash>;
       async fn get(&self, id: &ContentHash) -> Result<Option<Engram>>;
       async fn query(&self, q: &Query, ctx: &Context) -> Result<Vec<Engram>>;
   }
   ```

3. **Current KnowledgeStore** (in `crates/roko-neuro/src/knowledge_store.rs`):
   ```rust
   pub struct KnowledgeStore {
       path: PathBuf,
       confirmations_path: PathBuf,
       write_gate: Arc<Mutex<()>>,
   }
   ```
   Note: KnowledgeStore uses synchronous file I/O and returns `KnowledgeEntry` values.

4. Create an adapter module `crates/roko-neuro/src/store_adapter.rs` that wraps `KnowledgeStore` with the Store protocol:
   ```rust
   use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeStore};
   use roko_core::{Engram, EngramBuilder, ContentHash, Kind, Body, Query, Context};

   /// Adapter that wraps KnowledgeStore to implement the Store protocol.
   /// Since Store is async and KnowledgeStore is sync, this uses
   /// tokio::task::spawn_blocking for I/O operations.
   pub struct NeuroStoreAdapter {
       inner: KnowledgeStore,
   }

   impl NeuroStoreAdapter {
       pub fn new(store: KnowledgeStore) -> Self {
           Self { inner: store }
       }

       /// Convert a KnowledgeEntry to an Engram for storage.
       /// Uses M096's `to_engram_kind()` for Kind mapping.
       pub fn entry_to_engram(entry: &KnowledgeEntry) -> Engram {
           EngramBuilder::new(entry.kind.to_engram_kind())
               .body(Body::text(&entry.content))
               .tag("knowledge.id", &entry.id)
               .tag("knowledge.source", entry.source.as_deref().unwrap_or(""))
               .tag("knowledge.tier", &format!("{:?}", entry.tier))
               .build()
       }

       /// Convert an Engram back to a KnowledgeEntry for legacy code.
       pub fn engram_to_entry(engram: &Engram) -> Option<KnowledgeEntry> {
           let kind = KnowledgeKind::from_engram_kind(&engram.kind)?;
           // Reconstruct KnowledgeEntry from Engram tags and body
           // Note: Body::as_text() returns Result<&str>, not Option
           let content = engram.body.as_text().ok().unwrap_or("").to_string();
           Some(KnowledgeEntry {
               id: engram.tags.get("knowledge.id").cloned().unwrap_or_default(),
               kind,
               content,
               ..Default::default()
           })
       }
   }
   ```

5. Implement `Store` for `NeuroStoreAdapter`:
   ```rust
   #[async_trait::async_trait]
   impl roko_core::Store for NeuroStoreAdapter {
       async fn put(&self, engram: Engram) -> anyhow::Result<ContentHash> {
           // Convert Engram -> KnowledgeEntry, delegate to inner.ingest()
       }

       async fn get(&self, id: &ContentHash) -> anyhow::Result<Option<Engram>> {
           // Query inner store by content hash
       }

       async fn query(&self, q: &Query, _ctx: &Context) -> anyhow::Result<Vec<Engram>> {
           // Convert Query to knowledge query, map results to Engrams
       }
   }
   ```

6. Register the module in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod store_adapter;
   pub use store_adapter::NeuroStoreAdapter;
   ```

7. Write tests:
   - Round-trip: `KnowledgeEntry` -> `Engram` -> `KnowledgeEntry` preserves kind and content
   - `entry_to_engram` sets the correct Kind variant
   - `engram_to_entry` returns `None` for non-knowledge Engram kinds
   - Adapter `put` delegates to `KnowledgeStore::ingest`

## Verification
```bash
cargo check -p roko-neuro -p roko-core -p roko-fs
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro --lib
```

## What NOT to do
- Do NOT change the JSONL file format -- existing `.roko/neuro/knowledge.jsonl` files must remain readable
- Do NOT remove `KnowledgeStore` -- the adapter wraps it, does not replace it
- Do NOT implement Store trait directly on KnowledgeStore -- use the adapter pattern since Store is async
- Do NOT touch admission.rs or context.rs -- those are separate batches
