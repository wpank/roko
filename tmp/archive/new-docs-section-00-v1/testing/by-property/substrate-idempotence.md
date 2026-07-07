# Substrate Write Idempotence

> Writing the same Engram twice leaves the substrate in the same state as writing it once.

**Crate**: `roko-fs`
**Test type**: Property-based (proptest)
**Enforcement**: `Substrate::write`, content-hash deduplication
**Last reviewed**: 2026-04-19

---

## Statement

For all substrates S and all valid Engrams E:

`S.write(E); S.write(E); S` has the same observable state as `S.write(E); S`.

Observable state means: `S.read(E.id())` returns the same result, and `S.list_all()` returns the same collection.

---

## Why It Matters

Idempotent writes are critical for:
- **Crash recovery**: if the orchestrator crashes after writing E but before recording the completion, it re-sends E on resume. The substrate must not duplicate it.
- **Multi-agent**: two agents that independently derive the same Engram (same content → same hash) should not create two substrate entries.
- **Retry logic**: gate-approved Engrams are written with at-least-once semantics. Idempotence makes at-least-once equivalent to exactly-once.

---

## Implementation

The `FileSubstrate` deduplicates by content hash at write time. The JSONL file is indexed by content hash; a write checks whether the hash already exists before appending.

```rust
impl Substrate for FileSubstrate {
    fn write(&self, engram: &Engram) -> Result<(), SubstrateError> {
        let hash = engram.id().content_hash();
        if self.index.contains(&hash) {
            return Ok(()); // already present — idempotent success
        }
        self.append_to_file(engram)?;
        self.index.insert(hash);
        Ok(())
    }
}
```

<!-- source: crates/roko-fs/src/file_substrate.rs -->

---

## Property Test

```rust
proptest! {
    #[test]
    fn substrate_write_idempotent(engram in arb_engram()) {
        let ctx = TestContext::new();
        let substrate = ctx.file_substrate();

        substrate.write(&engram).unwrap();
        substrate.write(&engram).unwrap(); // second write

        // Count must be 1
        let all = substrate.list_all().unwrap();
        let matching: Vec<_> = all.iter().filter(|e| e.id() == engram.id()).collect();
        prop_assert_eq!(
            matching.len(), 1,
            "Writing the same engram twice must produce exactly one entry"
        );
    }
}
```

**File**: `crates/roko-fs/src/tests/idempotence_tests.rs`

---

## Related Properties

- [substrate-read-after-write.md](substrate-read-after-write.md) — read-after-write consistency
- [substrate-gc-preserves-living.md](substrate-gc-preserves-living.md) — GC respects idempotent writes
- [content-addressing-determinism.md](content-addressing-determinism.md) — idempotence depends on deterministic IDs

## See also

- [../by-subsystem/subsystem-fs.md](../by-subsystem/subsystem-fs.md)
