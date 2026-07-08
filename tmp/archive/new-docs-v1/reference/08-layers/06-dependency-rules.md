# Dependency Rules

> The "strictly downward" rule, enforcement mechanism, anti-patterns, and enforcement CI.

**Status**: Shipping
**Crate**: All crates (architectural rule)
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Roko crate belongs to exactly one layer. Crates may only depend on crates at
strictly lower layers. This rule is enforced by a CI linter (`layer-check`) that runs
on every pull request and fails the build on violations.

---

## The Rule

```
crate at layer N may depend on crates at layers 0, 1, …, N-1
crate at layer N MUST NOT depend on crates at layers N, N+1, …, 4
```

Layer membership is declared in `Cargo.toml` via a workspace metadata key:

```toml
# crates/roko-agent/Cargo.toml
[package.metadata.roko]
layer = 2
```

The `layer-check` CI job reads all `Cargo.toml` files, resolves the full dependency
graph, and fails if any edge violates the rule.

---

## Anti-Patterns

### Anti-pattern 1: Upward dependency

```
roko-std (L2) imports roko-orchestrator (L3)   ← ILLEGAL
```

`roko-orchestrator` builds the `TickContext` that `roko-std` components are injected
into. If `roko-std` imported `roko-orchestrator`, there would be a circular dependency.

**Correct approach**: define the trait in `roko-core` (L1); implement it in `roko-std`
(L2); wire it in `roko-orchestrator` (L3).

### Anti-pattern 2: Layer skipping for convenience

```
roko-cli (L4) imports roko-agent (L2) directly, bypassing roko-orchestrator (L3)
```

This bypasses health monitoring, budget enforcement, and multi-agent coordination.

**Correct approach**: always go through L3. `roko-cli` calls
`Orchestrator::spawn_agent()`, which calls `TickContextBuilder`, which wires L2
components.

### Anti-pattern 3: Cross-layer globals

```
// In roko-core (L1):
static GLOBAL_BUS: once_cell::Lazy<Arc<dyn Bus>> = once_cell::Lazy::new(|| ...);
```

Global state in L1 would mean every implementation shares the same singleton, breaking
substitutability and testability.

**Correct approach**: pass trait objects via dependency injection. No global state
in L0 or L1.

### Anti-pattern 4: Type-erased "any crate" imports

```
// In roko-std (L2):
use roko_any::get_component::<dyn Scorer>();  // magic registry pattern
```

A runtime component registry bypasses the static layer check. If a component can be
fetched from a global registry at runtime, the dependency graph is opaque.

**Correct approach**: explicit DI at L3.

---

## Enforcement

### CI layer-check

```bash
# .github/workflows/layer-check.yml
cargo run --bin layer-check -- --workspace
```

The `layer-check` binary:
1. Reads `Cargo.toml` for all crates in the workspace.
2. Reads `[package.metadata.roko].layer` for each crate.
3. Resolves transitive dependencies with `cargo metadata`.
4. Reports any edge `(crate A at layer N) → (crate B at layer M)` where M ≥ N.

The check runs on every PR and on every push to `main`. Violations block merging.

### cargo-deny

`cargo-deny` is also configured to reject workspace-internal circular dependencies and
to warn on unexpected external dependencies.

---

## Adding a New Crate

When adding a new crate to the workspace:

1. Declare its layer in `Cargo.toml`:
   ```toml
   [package.metadata.roko]
   layer = 2  # must be 0, 1, 2, 3, or 4
   ```
2. Add it to the layer definition table in `deny.toml`.
3. Ensure its `[dependencies]` only reference crates at lower layers.
4. Run `cargo run --bin layer-check` locally before pushing.

---

## See also

- [Overview](00-overview.md) — the conceptual motivation for the rule
- [Crate–Layer Map](08-crate-layer-map.md) — the authoritative layer assignment for every crate
- [Adding a Layer](09-adding-a-layer.md) — guidance if the five tiers are insufficient
- [Cross-Layer Protocols](07-cross-layer-protocols.md) — the approved communication patterns
