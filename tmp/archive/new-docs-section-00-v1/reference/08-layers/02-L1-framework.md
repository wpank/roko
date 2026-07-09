# L1 — Framework Layer

> The universal trait vocabulary and core data types that all higher layers speak.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [L0 Runtime](01-L0-runtime.md)
**Used by**: L2, L3, L4, all subsystems, all cross-cuts
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko-core` defines the traits and types that constitute Roko's "language." Every other
crate that wants to be part of the Roko ecosystem imports from `roko-core`. No concrete
implementations live here — only the contracts that implementations must satisfy.
This is the layer that makes Roko's components substitutable without rewriting anything.

---

## What Lives at L1

### Core Data Types

These are the universal primitives. Every other layer can use them:

| Type | What it is |
|---|---|
| `Engram` | The durable knowledge unit ([full spec](../01-engram/README.md)) |
| `Pulse` | The ephemeral event ([full spec](../02-pulse/README.md)) |
| `Score` | Seven-axis appraisal ([full spec](../10-types/score.md)) |
| `HdcFingerprint` | 10 240-bit hyperdimensional vector |
| `Provenance` | Causal chain record |
| `Kind` | Engram kind enum |
| `Body` | Engram body enum |
| `Decay` | Half-life variant |

### Core Traits

These are the contracts for pluggable behavior:

| Trait | What it defines |
|---|---|
| `Substrate` | Storage backend |
| `Scorer` | Scoring logic |
| `Router` | Routing logic |
| `Composer` | Context assembly |
| `Gate` | Verification check |
| `Policy` | Pre/post-execution rules |
| `Bus` | Pulse transport |
| `Scheduler` | Tick scheduling |

### Operator Traits

The six composable traits that make up the "synapse":
`Substrate`, `Scorer`, `Router`, `Composer`, `Gate`, `Policy`.

---

## Design Principles of L1

**No concrete implementations.** `roko-core` compiles to a library with no binary.
The only code it contains is data type definitions, trait definitions, and simple
derivable functionality (serialization, equality, hashing).

**No async in traits (where avoidable).** Async trait methods require boxing today
(without `async-trait` macro in stable Rust). Where possible, L1 traits are
synchronous; async variants are at L2.

**Minimal dependencies.** `roko-core` depends on `roko-runtime` and
`serde` / `bincode` for serialization. Nothing else. This keeps compile times fast
and the attack surface small.

---

## Crate Structure

```
roko-core/
  src/
    engram.rs       Engram struct + builders
    pulse.rs        Pulse struct + builders
    score.rs        Score + scoring axis types
    fingerprint.rs  HdcFingerprint + operations
    provenance.rs   Provenance record
    kind.rs         Kind enum
    body.rs         Body enum
    decay.rs        Decay variants
    traits/
      substrate.rs
      scorer.rs
      router.rs
      composer.rs
      gate.rs
      policy.rs
      bus.rs
      scheduler.rs
    error.rs        Common error types
```

---

## See also

- [L0 Runtime](01-L0-runtime.md) — the layer L1 depends on
- [L2 Scaffold](03-L2-scaffold.md) — implements the L1 traits
- [Operators](../05-operators/README.md) — the six operator traits defined here
- [Crate–Layer Map](08-crate-layer-map.md) — full crate inventory
