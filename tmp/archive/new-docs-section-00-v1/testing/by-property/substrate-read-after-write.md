# Substrate Read-After-Write Consistency

> An Engram read immediately after being written is identical to the written Engram.

**Crate**: `roko-fs`
**Test type**: Property-based (proptest)
**Enforcement**: `Substrate::write`, `Substrate::read`
**Last reviewed**: 2026-04-19

---

## Statement

For all substrates S and all valid Engrams E:

`S.write(E); S.read(E.id()) == Some(E)`

The read immediately follows the write within the same process and thread (no concurrent access assumed).

---

## Property Test

```rust
proptest! {
    #[test]
    fn substrate_read_after_write_consistent(engram in arb_engram()) {
        let ctx = TestContext::new();
        let substrate = ctx.file_substrate();

        substrate.write(&engram).unwrap();
        let retrieved = substrate.read(engram.id()).unwrap();

        prop_assert_eq!(Some(&engram), retrieved.as_ref(),
            "Read-after-write must return the written engram");
    }
}
```

---

## Related Properties

- [substrate-idempotence.md](substrate-idempotence.md)
- [content-addressing-determinism.md](content-addressing-determinism.md)

## See also

- [../by-subsystem/subsystem-fs.md](../by-subsystem/subsystem-fs.md)
