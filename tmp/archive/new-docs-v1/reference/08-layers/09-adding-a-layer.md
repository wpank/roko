# Adding a Layer

> Guidance for extending the five-layer taxonomy, and the bar for doing so.

**Status**: Specified
**Last reviewed**: 2026-04-19

---

## TL;DR

The five layers are sufficient for the current system. Adding a sixth layer is a
significant architectural decision — it affects every crate in the workspace and every
contributor's mental model. The bar for adding a layer is high: the new layer must
represent a genuinely new kind of crate that cannot be assigned to any existing layer
without distorting its semantics.

---

## When a New Layer Is Appropriate

A new layer is appropriate if all of the following are true:

1. **There is a new class of crate** — not just a new crate of an existing class.
2. **The downward dependency rule creates a genuine problem** — the new crate
   legitimately needs to depend on some existing layer AND be depended upon by another.
3. **The new layer represents a stable long-term boundary** — not a transient
   organizational convenience.

Examples of things that are **not** new layers:
- A new Substrate backend (it's L2).
- A new agent type (it's L2).
- A new CLI command (it's L4).
- A new cross-cut subsystem (it's L2, injected by L3).

---

## The Process

1. Open a discussion in the architecture forum with the proposed layer's definition,
   its crates, and why it cannot fit in an existing layer.
2. Get review from at least two other contributors familiar with the layer system.
3. Update `deny.toml`, all `Cargo.toml` layer assignments, and the `layer-check` binary.
4. Update this file and [Crate–Layer Map](08-crate-layer-map.md).
5. Update all documentation that references "five layers."

---

## Open Questions

- Should there be a "L2b" tier for cross-cut crates? Currently they are at L2, but
  their lifecycle (injected by L3 into L2's `TickContext`) is slightly different from
  pure L2 crates. See [Open Question in Cross-Cuts](../09-cross-cuts/07-open-questions.md).

---

## See also

- [Overview](00-overview.md) — the five-layer structure
- [Dependency Rules](06-dependency-rules.md) — the rule any new layer must comply with
- [Rationale](10-rationale.md) — why five was the right number
