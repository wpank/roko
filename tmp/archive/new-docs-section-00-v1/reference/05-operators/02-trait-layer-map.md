# Trait × Layer Map

> Which of the five Roko layers each operator trait belongs to, and why.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Overview](./00-overview.md), [Five-Layer Taxonomy](../08-layers/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko has five layers: Runtime, Framework, Scaffold, Harness, Orchestration. Each operator
trait is defined at exactly one layer; implementations may exist at the same or a lower
layer. Dependencies always flow downward — no lower layer imports from a higher one.

---

## The Five Layers (Summary)

| Layer | # | Role |
|---|---|---|
| Runtime | 1 (lowest) | Core types: `Engram`, `Score`, `Decay`, `Provenance`, `ContentHash` |
| Framework | 2 | Operator traits: `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy` |
| Scaffold | 3 | Concrete operator implementations (`FileSubstrate`, etc.) |
| Harness | 4 | The cognitive loop (`loop_tick`), cross-cuts (Neuro, Daimon, Dreams) |
| Orchestration | 5 (highest) | Agent assembly, configuration, CLI |

---

## Trait Locations

| Trait | Layer | Crate | Notes |
|---|---|---|---|
| `Substrate` | Framework (2) | `roko-core` | Trait defined here; implementations in layer 3+ |
| `Scorer` | Framework (2) | `roko-core` | — |
| `Gate` | Framework (2) | `roko-core` | — |
| `Router` | Framework (2) | `roko-core` | — |
| `Composer` | Framework (2) | `roko-core` | — |
| `Policy` | Framework (2) | `roko-core` | — |
| `Bus` (target) | Framework (2) | `roko-core` | Not yet shipped |

All traits are defined at the Framework layer. Implementations live in Scaffold or above.

---

## Implementation Locations

| Implementation | Trait | Layer | Crate |
|---|---|---|---|
| `FileSubstrate` | `Substrate` | Scaffold (3) | `roko-fs` |
| `MemorySubstrate` | `Substrate` | Scaffold (3) | `roko-runtime` |
| `EventBus<E>` | `Bus` (future) | Scaffold (3) | `roko-runtime` |
| Default `Scorer` | `Scorer` | Scaffold (3) | `roko-core` |
| Safety `Gate` | `Gate` | Scaffold (3) | `roko-gate` |
| `CascadeRouter` | `Router` | Scaffold (3) | `roko-agent` |
| `SystemPromptBuilder` | `Composer` | Scaffold (3) | `roko-compose` |
| `CircuitBreakerPolicy` | `Policy` | Scaffold (3) | `roko-core` |

---

## Why Traits at the Framework Layer

The Framework layer is the "seam" layer — it is where the cognitive loop's interface is
defined without any implementation. This means:

1. **The loop (Harness, layer 4) imports only the Framework.** It calls `scorer.score(...)`,
   not `FileSubstrate::open(...)`. The loop is independent of concrete backends.
2. **Implementations (Scaffold, layer 3) can import the Runtime but not the Harness.** They
   don't know they're called from a loop.
3. **Orchestration (layer 5) wires everything together.** It imports all lower layers and
   assembles the concrete runtime.

---

## Dependency Diagram

```
Orchestration (5) ─── imports ──→ Harness, Scaffold, Framework, Runtime
     │
  Harness (4) ─── imports ──→ Framework, Runtime
     │
  Scaffold (3) ─── imports ──→ Framework, Runtime
     │
  Framework (2) ─── imports ──→ Runtime
     │
  Runtime (1) ─── imports ──→ std only
```

No upward dependencies. `roko-core` (Runtime + Framework) imports only `std` and a small
set of pure-data crates (`blake3`, `serde`, `uuid`).

---

## See Also

- [Five-Layer Taxonomy](../08-layers/README.md)
- [Overview](./00-overview.md)
- [Crate Map](../11-crate-map.md)
