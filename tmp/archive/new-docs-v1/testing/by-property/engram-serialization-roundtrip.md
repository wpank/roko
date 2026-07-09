# Engram Serialization Round-Trip

> Serializing an Engram and deserializing it produces an identical Engram.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `serde` implementation on `Engram`
**Last reviewed**: 2026-04-19

---

## Statement

For all valid Engrams E:
`deserialize(serialize(E)) == Ok(E)`

And the converse: for all valid serialized byte strings B:
`serialize(deserialize(B)) == B` (canonical round-trip)

---

## Property Test

```rust
proptest! {
    #[test]
    fn engram_serde_roundtrip(engram in arb_engram()) {
        let serialized = serde_json::to_string(&engram).expect("serialize must not fail");
        let deserialized: Engram = serde_json::from_str(&serialized)
            .expect("deserialize must not fail on valid JSON");
        prop_assert_eq!(engram, deserialized, "Round-trip must preserve all fields");
    }
}
```

---

## Related Properties

- [content-addressing-determinism.md](content-addressing-determinism.md)

## See also

- [../tiers/06-fuzz-tests.md](../tiers/06-fuzz-tests.md) — fuzz target for malformed deserialization input
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
