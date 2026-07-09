# Five-Layer Taxonomy

> The strictly downward dependency hierarchy that organizes every Roko crate.

**Status**: Shipping
**Depends on**: [Engram](../01-engram/README.md), [Substrate](../03-substrate/README.md),
[Operators](../05-operators/README.md)
**Used by**: Everything in Roko

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | Layering rule: strictly downward dependencies | Shipping |
| 01 | [L0 Runtime](01-L0-runtime.md) | `roko-runtime` — the bare-metal execution substrate | Shipping |
| 02 | [L1 Framework](02-L1-framework.md) | `roko-core` traits — the universal vocabulary | Shipping |
| 03 | [L2 Scaffold](03-L2-scaffold.md) | `roko-std`, `roko-agent`, `roko-compose` | Shipping |
| 04 | [L3 Harness](04-L3-harness.md) | `roko-orchestrator`, `roko-gate` | Shipping |
| 05 | [L4 Orchestration](05-L4-orchestration.md) | `roko-cli`, `roko-serve` | Shipping |
| 06 | [Dependency Rules](06-dependency-rules.md) | The "strictly downward" rule, enforcement | Shipping |
| 07 | [Cross-Layer Protocols](07-cross-layer-protocols.md) | How layers communicate across boundaries | Shipping |
| 08 | [Crate–Layer Map](08-crate-layer-map.md) | Which crate sits at which layer | Shipping |
| 09 | [Adding a Layer](09-adding-a-layer.md) | Extending the taxonomy | Specified |
| 10 | [Rationale](10-rationale.md) | Why five, alternatives considered | — |

---

## Suggested reading order

**First-time reader**: 00 → 01–05 (one pass each) → 06

**New contributor (adding a crate)**: 08 → 06 → the layer you're adding to

**Debugger (unexpected dependency)**: 06 → 08 → the two crates involved

---

## See also

- [Crate Map](../11-crate-map.md) — the full crate inventory
- [Cognitive Loop](../06-loop/README.md) — `loop_tick()` lives at L2
- [Cross-Cuts](../09-cross-cuts/README.md) — cross-cuts span L1–L3
