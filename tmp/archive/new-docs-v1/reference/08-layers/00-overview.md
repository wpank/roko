# Five-Layer Taxonomy — Overview

> Every Roko crate belongs to exactly one layer. Dependencies only flow downward.
> No crate at layer N may import from layer N+1 or higher.

**Status**: Shipping
**Crate**: All crates
**Depends on**: Nothing (this is the root architectural rule)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's crates are organized into five layers (L0–L4). The single architectural rule
is: **dependencies are strictly downward**. A crate at layer N may depend on crates
at layers N-1, N-2, …, 0 — but never on layers N+1, N+2, …, 4. This rule is
enforced by CI. Violating it is a build error, not a code review comment.

---

## The Five Layers

```
L4: Orchestration     roko-cli, roko-serve
                            │
L3: Harness           roko-orchestrator, roko-gate
                            │
L2: Scaffold          roko-std, roko-agent, roko-compose
                            │
L1: Framework         roko-core (traits and types)
                            │
L0: Runtime           roko-runtime (async executor, allocator, platform)
```

Each layer has a clear purpose:

| Layer | Name | Purpose | Example crates |
|---|---|---|---|
| L0 | Runtime | Platform abstraction; async executor; allocator | `roko-runtime` |
| L1 | Framework | Universal trait vocabulary; core data types | `roko-core` |
| L2 | Scaffold | Default implementations of L1 traits; `loop_tick()` | `roko-std`, `roko-agent`, `roko-compose` |
| L3 | Harness | Wiring: inject L2 implementations into a running agent | `roko-orchestrator`, `roko-gate` |
| L4 | Orchestration | User-facing entry points: CLI, HTTP API | `roko-cli`, `roko-serve` |

---

## The Strictly Downward Rule

**The rule**: `crate at layer N` may only `use`/`import` from crates at layers ≤ N-1.

Written as a Cargo check:
```toml
# Enforced via cargo-deny deny.toml + a custom layer-linter CI job
[layer-rules]
L4 = ["roko-cli", "roko-serve"]
L3 = ["roko-orchestrator", "roko-gate"]
L2 = ["roko-std", "roko-agent", "roko-compose"]
L1 = ["roko-core"]
L0 = ["roko-runtime"]
# Violation: any edge N→M where M > N
```

The enforcement mechanism is described in detail in [Dependency Rules](06-dependency-rules.md).

---

## Why This Shape?

### Substitutability

L1 defines the traits. L2 provides default implementations. If you don't like the
default `Scorer` in `roko-std`, you replace it at L2 (or inject a custom one from L3)
without touching L0 or L1. The trait boundary is stable; the implementation is not.

### Testability

Every layer can be tested in isolation. L1 tests use mock types. L2 tests use mock
L0/L1 dependencies. L3 tests build full agent configurations but do not need a real
CLI or HTTP server. L4 tests are integration tests against a real (but minimal) stack.

### Deployability

The "run-anywhere" vision requires portability. `roko-runtime` (L0) abstracts the
platform (native / WASM / embedded). Every layer above it is platform-agnostic.
Porting Roko to a new platform requires only a new `roko-runtime` implementation.

### Auditability

The layer structure makes the dependency graph auditable. `cargo tree` for any L4
crate shows the complete transitive dependency graph in layer order. A security audit
can start at L0 and verify layer by layer.

---

## What is NOT a Layer

Some things look like layers but are not:

- **Subsystems** (Neuro, Daimon, Dreams, Oracles) are not a separate layer — they
  span L1–L3. They are [cross-cuts](../09-cross-cuts/README.md).
- **Substrate implementations** (sled, Postgres, in-memory) are L0/L1 — not a
  separate layer.
- **Agent types** (research, coding, chain) are L2 specializations — not a separate
  layer.
- **Test helpers** are defined alongside the layer they test — not a layer.

---

## See also

- [L0 Runtime](01-L0-runtime.md) through [L4 Orchestration](05-L4-orchestration.md)
- [Dependency Rules](06-dependency-rules.md) — the enforcement mechanism
- [Crate–Layer Map](08-crate-layer-map.md) — every crate mapped to its layer
- [Cross-Cuts](../09-cross-cuts/README.md) — structures that span layers
