# Examples

> End-to-end usage patterns for `Substrate`.

**Status**: Shipping
**Crate**: `roko-core`, `roko-fs`, `roko-runtime`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## Example 1: Open a File Backend and Store a Fact

```rust
// source: crates/roko-fs/src/lib.rs
use roko_fs::FileSubstrate;
use roko_core::{Substrate, Engram, EngramBuilder, Kind, Body};
use std::path::Path;

fn store_a_fact() -> Result<(), Box<dyn std::error::Error>> {
    let mut substrate = FileSubstrate::open(Path::new("./agent.jsonl"))?;

    let engram = EngramBuilder::new()
        .kind(Kind::Fact)
        .body(Body::Text("The capital of France is Paris.".into()))
        .build()?;

    substrate.put(engram)?;
    println!("stored {} records", substrate.len());
    Ok(())
}
```
<!-- source: crates/roko-fs/src/lib.rs -->

---

## Example 2: Query by Kind and Time Window

```rust
// source: crates/roko-core/src/substrate.rs
use roko_core::{SubstrateQuery, Kind};

let yesterday = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)?
    .as_secs() - 86_400;

let recent_facts = substrate.query(&SubstrateQuery {
    kind: Some(Kind::Fact),
    created_after: Some(yesterday),
    min_confidence: Some(0.7),
    limit: 50,
    ..Default::default()
})?;

println!("found {} recent facts", recent_facts.len());
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Example 3: Associative Recall (query_similar)

```rust
// source: crates/roko-core/src/substrate.rs
// Given a current-context Engram, find the 16 most similar memories.
use roko_core::Substrate;

let context_fp = current_engram
    .fingerprint
    .as_ref()
    .expect("context engram must have a fingerprint");

let similar_memories = substrate.query_similar(context_fp, 16)?;
for memory in &similar_memories {
    println!("  recall: {:?}", memory.body);
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Example 4: Using MemorySubstrate in Tests

```rust
// source: crates/roko-runtime/src/memory_substrate.rs
#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Substrate, EngramBuilder, Kind, Body};

    #[test]
    fn put_and_get_roundtrip() {
        let mut s = MemorySubstrate::new();

        let e = EngramBuilder::new()
            .kind(Kind::Episode)
            .body(Body::Text("test memory".into()))
            .build()
            .unwrap();

        let hash = e.hash.clone();
        s.put(e).unwrap();

        let retrieved = s.get(&hash).unwrap().expect("record must exist");
        assert_eq!(retrieved.kind, Kind::Episode);
    }
}
```
<!-- source: crates/roko-runtime/src/memory_substrate.rs -->

---

## Example 5: Scheduled Prune

```rust
// source: crates/roko-runtime/src/agent.rs
use std::time::Duration;
use tokio::time;

async fn prune_loop(substrate: Arc<Mutex<Box<dyn Substrate>>>) {
    let mut interval = time::interval(Duration::from_secs(300)); // every 5 min
    loop {
        interval.tick().await;
        let removed = substrate.lock().unwrap().prune().unwrap_or(0);
        tracing::info!(removed, "substrate pruned");
    }
}
```
<!-- source: crates/roko-runtime/src/agent.rs -->

---

## See Also

- [Put, Get, Query](./02-put-get-query.md)
- [Query Similar](./03-query-similar.md)
- [Pruning](./06-pruning.md)
- [Backend: JSONL File](./08-backend-file-jsonl.md)
