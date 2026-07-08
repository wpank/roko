# Rationale

> Why `Substrate` is a trait rather than a concrete type, and what alternatives were
> considered and rejected.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Overview](./00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A trait was chosen over a concrete type because: (1) backends must be swappable in tests,
(2) deployment contexts differ (laptop vs. server vs. chain), and (3) cognitive-loop code
should not contain storage-layer knowledge. The rejected alternatives were a generic type
parameter, an enum of backends, and no abstraction at all.

---

## The Problem Without a Trait

Suppose `Substrate` were a concrete type (e.g., `FileSubstrate` is the storage layer, full
stop). The consequences:

1. **Test friction** — every agent test writes to disk. Tests are slow, stateful, and require
   cleanup. The common workaround — mocking — requires either a trait or an injectable
   function pointer anyway.

2. **Deployment coupling** — code written for a file backend cannot be redeployed against a
   chain backend without modifying the agent. The storage decision propagates into agent logic.

3. **Benchmark noise** — performance tests of the cognitive loop mix agent logic costs with
   I/O costs. Separating them is impossible without an injectable abstraction.

4. **Extensibility friction** — adding a new backend (SQLite, Redis, on-chain) requires
   forking or patching the core crate.

---

## Why a Trait (Not a Type Parameter)

One alternative to a `dyn Substrate` trait is a generic `Agent<S: Substrate>`. This avoids
dynamic dispatch.

The trade-offs:

| | `dyn Substrate` (chosen) | `Agent<S: Substrate>` |
|---|---|---|
| Runtime swappability | Yes | No (monomorphised at compile time) |
| Dynamic dispatch cost | ~ns per call | Zero (inlined) |
| Object safety required | Yes | No |
| Test ergonomics | `Box<dyn Substrate>` | `Agent<MemorySubstrate>` |
| Code bloat | None | Monomorphised copies of Agent |

For Roko, **runtime swappability** matters more than the nanoseconds saved by static
dispatch. The substrate call overhead (a virtual dispatch for `put`/`get`) is dwarfed by
the actual storage I/O cost. Dynamic dispatch is the right trade.

---

## Rejected: Enum of Backends

A `SubstrateKind` enum:

```rust
// NOT implemented — shown for contrast
enum SubstrateKind {
    File(FileSubstrate),
    Memory(MemorySubstrate),
}
```

This is exhaustive — every new backend requires adding a variant and modifying every match
arm. It also makes third-party backends impossible without forking. Rejected.

---

## Rejected: No Abstraction

Early prototypes of Roko wired `FileSubstrate` directly. The test suite was painful (10 ms+
per test, filesystem cleanup in every test teardown). The trait was introduced at the first
non-trivial refactor. There is no practical benefit to removing it.

---

## Why Not a Repository Pattern with a Separate Query Object?

Some storage frameworks separate reads from writes into distinct traits (`ReadSubstrate` /
`WriteSubstrate`). This allows read-only handles.

Roko does not do this today because:
- `Agent` always needs both reads and writes.
- The added type complexity was not justified by the benefit.

If multi-reader / write-isolated access becomes important, the split can be added without
breaking the current trait (the current `Substrate` would become `WriteSubstrate = Substrate`,
and a `ReadSubstrate` supertrait extracted).

---

## See Also

- [Overview](./00-overview.md)
- [Concurrency Model](./05-concurrency-model.md)
- [Backends Overview](./07-backends-overview.md)

## Open Questions

- Should `Substrate` be split into `ReadSubstrate` and `WriteSubstrate` as agents scale to
  multi-reader/write-once patterns?
